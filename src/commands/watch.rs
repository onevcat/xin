use std::io::Write;
use std::path::PathBuf;

use serde_json::{Value, json};

use crate::backend::Backend;
use crate::cli::WatchArgs;
use crate::error::XinErrorOut;
use crate::output::{Envelope, Meta};

use super::history::{PageToken, decode_page_token, encode_page_token};

fn read_checkpoint(path: &PathBuf) -> Option<String> {
    let s = std::fs::read_to_string(path).ok()?;
    let s = s.trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn write_checkpoint(path: &PathBuf, token: &str) -> Result<(), XinErrorOut> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| XinErrorOut::config(format!("checkpoint mkdir failed: {e}")))?;
    }

    // Best-effort atomic write.
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, format!("{}\n", token))
        .map_err(|e| XinErrorOut::config(format!("checkpoint write failed: {e}")))?;
    std::fs::rename(&tmp, path)
        .map_err(|e| XinErrorOut::config(format!("checkpoint rename failed: {e}")))?;
    Ok(())
}

fn json_line(v: &Value) {
    let mut out = std::io::stdout().lock();
    // If serialization fails, we intentionally panic: this is programmer error.
    let s = serde_json::to_string(v).expect("json serialize");
    let _ = writeln!(out, "{}", s);
    let _ = out.flush();
}

fn pretty_line(v: &Value) {
    let mut out = std::io::stdout().lock();
    let s = serde_json::to_string_pretty(v).expect("json serialize");
    let _ = writeln!(out, "{}", s);
    let _ = out.flush();
}

fn plain_line(s: &str) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{}", s);
    let _ = out.flush();
}

fn now_nanos() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0)
}

fn compute_wait_ms(interval_ms: u64, jitter_ms: u64, now_nanos: u64) -> u64 {
    let jitter = if jitter_ms == 0 {
        0
    } else {
        now_nanos % (jitter_ms + 1)
    };
    interval_ms.saturating_add(jitter)
}

async fn sleep_with<F, Fut>(dur: std::time::Duration, sleeper: F)
where
    F: FnOnce(std::time::Duration) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    sleeper(dur).await
}

pub async fn watch(account: Option<String>, args: &WatchArgs, plain: bool) -> Envelope<Value> {
    let command_name = "watch";

    // Emitter: JSON (default) or plain text.
    let emit_json = if args.pretty { pretty_line } else { json_line };
    let emit_plain = |line: &str| plain_line(line);

    let emit_error = |e: &XinErrorOut| {
        if plain {
            emit_plain(&format!("ERROR\t{}\t{}", e.kind, e.message));
            return;
        }

        if args.no_envelope {
            let v = serde_json::to_value(e).unwrap_or_else(
                |_| json!({"kind":"unknown","message":"failed to serialize error"}),
            );
            emit_json(&json!({"type":"error","error": v}));
        }
    };

    let backend = match Backend::connect(account.as_deref()).await {
        Ok(b) => b,
        Err(e) => {
            emit_error(&e);
            return Envelope::err(command_name, account, e);
        }
    };

    // Resolve start token priority:
    // 1) --page
    // 2) --checkpoint (if exists)
    // 3) --since
    // 4) bootstrap current state
    let mut page_token = args.page.clone();

    if page_token.is_none() {
        if let Some(path) = &args.checkpoint {
            if let Some(s) = read_checkpoint(path) {
                page_token = Some(s);
            }
        }
    }

    let max_changes_default = 100;

    let (mut since_state, used_max) = match &page_token {
        Some(token) => {
            let t = match decode_page_token(token) {
                Ok(t) => t,
                Err(e) => {
                    emit_error(&e);
                    return Envelope::err(command_name, account, e);
                }
            };

            if let Some(max) = args.max {
                if max != t.max_changes {
                    let e = XinErrorOut::usage("page token does not match args".to_string());
                    emit_error(&e);
                    return Envelope::err(command_name, account, e);
                }
            }

            if let Some(since) = &args.since {
                if since != &t.since_state {
                    let e = XinErrorOut::usage("page token does not match args".to_string());
                    emit_error(&e);
                    return Envelope::err(command_name, account, e);
                }
            }

            (t.since_state, t.max_changes)
        }
        None => {
            let since = if let Some(s) = &args.since {
                s.clone()
            } else {
                match backend.email_state().await {
                    Ok(s) => s,
                    Err(e) => {
                        emit_error(&e);
                        return Envelope::err(command_name, account, e);
                    }
                }
            };

            (since, args.max.unwrap_or(max_changes_default))
        }
    };

    if since_state.trim().is_empty() {
        let e = XinErrorOut::usage("missing --since (or --page/--checkpoint)".to_string());
        emit_error(&e);
        return Envelope::err(command_name, account, e);
    }

    // Initial ready event (useful for agents).
    if plain {
        emit_plain(&format!(
            "READY\tsinceState={}\tmaxChanges={}",
            since_state, used_max
        ));
    } else {
        emit_json(&json!({
            "type": "ready",
            "sinceState": since_state,
            "maxChanges": used_max,
        }));
    }

    // (no RNG dependency; jitter derived from system time)

    loop {
        let (mut resp, hydrated_created, hydrated_updated) = if args.hydrate {
            match backend
                .email_changes_hydrate(&since_state, Some(used_max))
                .await
            {
                Ok((r, created, updated)) => (r, Some(created), Some(updated)),
                Err(e) => {
                    emit_error(&e);
                    return Envelope::err(command_name, account, e);
                }
            }
        } else {
            match backend.email_changes(&since_state, Some(used_max)).await {
                Ok(r) => (r, None, None),
                Err(e) => {
                    emit_error(&e);
                    return Envelope::err(command_name, account, e);
                }
            }
        };

        let has_more = resp.has_more_changes();
        let new_state = resp.take_new_state();

        let created = resp.take_created();
        let updated = resp.take_updated();
        let destroyed = resp.take_destroyed();

        // Emit events.
        if !created.is_empty() || !updated.is_empty() || !destroyed.is_empty() {
            if plain {
                emit_plain(&format!(
                    "TICK\tsinceState={}\tnewState={}\tcreated={}\tupdated={}\tdestroyed={}\thasMoreChanges={}",
                    since_state,
                    new_state,
                    created.len(),
                    updated.len(),
                    destroyed.len(),
                    has_more
                ));

                for id in &created {
                    emit_plain(&format!("CREATED\t{}\tnewState={}", id, new_state));
                }
                for id in &updated {
                    emit_plain(&format!("UPDATED\t{}\tnewState={}", id, new_state));
                }
                for id in &destroyed {
                    emit_plain(&format!("DESTROYED\t{}\tnewState={}", id, new_state));
                }

                if let (Some(created_emails), Some(updated_emails)) =
                    (hydrated_created.as_ref(), hydrated_updated.as_ref())
                {
                    emit_plain(&format!(
                        "HYDRATED\tcreated={}\tupdated={}",
                        created_emails.len(),
                        updated_emails.len()
                    ));
                }
            } else {
                emit_json(&json!({
                    "type": "tick",
                    "sinceState": since_state,
                    "newState": new_state,
                    "hasMoreChanges": has_more,
                    "counts": {
                        "created": created.len(),
                        "updated": updated.len(),
                        "destroyed": destroyed.len()
                    }
                }));

                for id in created {
                    emit_json(&json!({
                        "type": "email.change",
                        "changeType": "created",
                        "id": id,
                        "newState": new_state
                    }));
                }
                for id in updated {
                    emit_json(&json!({
                        "type": "email.change",
                        "changeType": "updated",
                        "id": id,
                        "newState": new_state
                    }));
                }
                for id in destroyed {
                    emit_json(&json!({
                        "type": "email.change",
                        "changeType": "destroyed",
                        "id": id,
                        "newState": new_state
                    }));
                }

                if let (Some(created_emails), Some(updated_emails)) =
                    (hydrated_created, hydrated_updated)
                {
                    // Optional hydrated summaries (agent can ignore).
                    emit_json(&json!({
                        "type": "email.hydrated",
                        "newState": new_state,
                        "hydrated": {
                            "created": crate::schema::email_summary_items(&created_emails),
                            "updated": crate::schema::email_summary_items(&updated_emails)
                        }
                    }));
                }
            }
        }

        // Advance checkpoint.
        since_state = new_state.clone();
        let next_token = encode_page_token(&PageToken {
            since_state: since_state.clone(),
            max_changes: used_max,
        });

        if let Some(path) = &args.checkpoint {
            if let Err(e) = write_checkpoint(path, &next_token) {
                emit_error(&e);
                return Envelope::err(command_name, account, e);
            }
        }

        if args.once && !has_more {
            break;
        }

        if has_more {
            // Drain remaining pages immediately.
            continue;
        }

        // Sleep before next poll.
        let wait = compute_wait_ms(args.interval_ms, args.jitter_ms, now_nanos());

        // Allow Ctrl-C to exit quickly.
        let dur = std::time::Duration::from_millis(wait);
        tokio::select! {
            _ = sleep_with(dur, tokio::time::sleep) => {},
            _ = tokio::signal::ctrl_c() => {
                if plain {
                    emit_plain("STOPPED\treason=ctrl_c");
                } else {
                    emit_json(&json!({"type":"stopped","reason":"ctrl_c"}));
                }
                break;
            }
        }
    }

    Envelope::ok(command_name, account, json!({"ok": true}), Meta::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_wait_ms_respects_interval_and_jitter_bounds() {
        let interval = 8000;
        let jitter = 600;

        // now_nanos=0 => jitter=0
        assert_eq!(compute_wait_ms(interval, jitter, 0), 8000);

        // now_nanos=jitter => jitter=jitter
        assert_eq!(compute_wait_ms(interval, jitter, jitter), 8600);

        // always within bounds
        for n in [1, 123, 999_999, 42_424_242] {
            let w = compute_wait_ms(interval, jitter, n);
            assert!(w >= interval);
            assert!(w <= interval + jitter);
        }
    }

    #[tokio::test]
    async fn sleep_with_allows_injection() {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };

        let called = Arc::new(AtomicBool::new(false));
        let called2 = called.clone();

        sleep_with(std::time::Duration::from_millis(12), move |dur| {
            let called3 = called2.clone();
            async move {
                called3.store(true, Ordering::SeqCst);
                assert_eq!(dur, std::time::Duration::from_millis(12));
            }
        })
        .await;

        assert!(called.load(Ordering::SeqCst));
    }
}
