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

pub async fn watch(account: Option<String>, args: &WatchArgs) -> Envelope<Value> {
    let command_name = "watch";

    let backend = match Backend::connect().await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
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
                Err(e) => return Envelope::err(command_name, account, e),
            };

            if let Some(max) = args.max {
                if max != t.max_changes {
                    return Envelope::err(
                        command_name,
                        account,
                        XinErrorOut::usage("page token does not match args".to_string()),
                    );
                }
            }

            if let Some(since) = &args.since {
                if since != &t.since_state {
                    return Envelope::err(
                        command_name,
                        account,
                        XinErrorOut::usage("page token does not match args".to_string()),
                    );
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
                    Err(e) => return Envelope::err(command_name, account, e),
                }
            };

            (since, args.max.unwrap_or(max_changes_default))
        }
    };

    if since_state.trim().is_empty() {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage("missing --since (or --page/--checkpoint)".to_string()),
        );
    }

    let emit = if args.pretty { pretty_line } else { json_line };

    // Initial ready event (useful for agents).
    emit(&json!({
        "type": "ready",
        "sinceState": since_state,
        "maxChanges": used_max,
    }));

    // (no RNG dependency; jitter derived from system time)

    loop {
        let (mut resp, hydrated_created, hydrated_updated) = if args.hydrate {
            match backend
                .email_changes_hydrate(&since_state, Some(used_max))
                .await
            {
                Ok((r, created, updated)) => (r, Some(created), Some(updated)),
                Err(e) => return Envelope::err(command_name, account, e),
            }
        } else {
            match backend.email_changes(&since_state, Some(used_max)).await {
                Ok(r) => (r, None, None),
                Err(e) => return Envelope::err(command_name, account, e),
            }
        };

        let has_more = resp.has_more_changes();
        let new_state = resp.take_new_state();

        let created = resp.take_created();
        let updated = resp.take_updated();
        let destroyed = resp.take_destroyed();

        // Emit events.
        if !created.is_empty() || !updated.is_empty() || !destroyed.is_empty() {
            emit(&json!({
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
                emit(&json!({
                    "type": "email.change",
                    "changeType": "created",
                    "id": id,
                    "newState": new_state
                }));
            }
            for id in updated {
                emit(&json!({
                    "type": "email.change",
                    "changeType": "updated",
                    "id": id,
                    "newState": new_state
                }));
            }
            for id in destroyed {
                emit(&json!({
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
                emit(&json!({
                    "type": "email.hydrated",
                    "newState": new_state,
                    "hydrated": {
                        "created": crate::schema::email_summary_items(&created_emails),
                        "updated": crate::schema::email_summary_items(&updated_emails)
                    }
                }));
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
        let jitter = if args.jitter_ms == 0 {
            0
        } else {
            // Derive a pseudo-random jitter without extra deps.
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos() as u64)
                .unwrap_or(0);
            nanos % (args.jitter_ms + 1)
        };
        let wait = args.interval_ms.saturating_add(jitter);

        // Allow Ctrl-C to exit quickly.
        let sleep_fut = tokio::time::sleep(std::time::Duration::from_millis(wait));
        tokio::select! {
            _ = sleep_fut => {},
            _ = tokio::signal::ctrl_c() => {
                emit(&json!({"type":"stopped","reason":"ctrl_c"}));
                break;
            }
        }
    }

    Envelope::ok(command_name, account, json!({"ok": true}), Meta::default())
}
