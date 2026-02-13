use serde_json::{Value, json};

use crate::backend::Backend;
use crate::cli::UrlArgs;
use crate::error::XinErrorOut;
use crate::output::{Envelope, Meta};

fn is_fastmail(resolved_account: Option<&str>, base_url: &str) -> bool {
    if resolved_account == Some("fastmail") {
        return true;
    }

    let host = url::Url::parse(base_url)
        .ok()
        .and_then(|u| u.host_str().map(|s| s.to_string()))
        .unwrap_or_default();

    host.ends_with("fastmail.com")
}

fn url_encode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

fn fastmail_web_url_for_message_id(message_id: &str) -> String {
    // Fastmail web UI supports searching by Message-ID.
    // Example: https://app.fastmail.com/mail/search:msgid:%3C...%3E
    format!(
        "https://app.fastmail.com/mail/search:msgid:{}",
        url_encode(message_id)
    )
}

pub async fn url(account: Option<String>, args: &UrlArgs) -> Envelope<Value> {
    let command_name = "url";

    // Keep `xin url` (no args) as a safe, config-free placeholder for interface tests/help.
    if args.ids.is_empty() {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::not_implemented(
                "url is Fastmail-only and requires one or more ids (threadId or emailId)".to_string(),
            ),
        );
    }

    let resolved = match crate::config::resolve_runtime_config(account.as_deref()) {
        Ok(r) => r,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    if !is_fastmail(resolved.account.as_deref(), &resolved.config.base_url) {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::not_implemented("url is only supported for Fastmail accounts".to_string()),
        );
    }

    let backend = match Backend::connect(resolved.account.as_deref()).await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let mut items: Vec<Value> = Vec::new();

    for id in &args.ids {
        // Try thread id first.
        let (kind, email_id) = match backend.thread_email_ids(id).await {
            Ok(Some(email_ids)) if !email_ids.is_empty() => {
                ("thread", email_ids[0].clone())
            }
            _ => ("email", id.clone()),
        };

        // Fetch Message-ID.
        let props = Some(vec![
            jmap_client::email::Property::Id,
            jmap_client::email::Property::MessageId,
        ]);

        let email = match backend.get_email(&email_id, props).await {
            Ok(v) => v,
            Err(e) => {
                items.push(json!({
                    "id": id,
                    "kind": kind,
                    "ok": false,
                    "emailId": email_id,
                    "url": null,
                    "error": e,
                }));
                continue;
            }
        };

        let message_id = email
            .as_ref()
            .and_then(|e| e.message_id())
            .and_then(|ids| ids.first().cloned());

        match message_id {
            Some(mid) => {
                items.push(json!({
                    "id": id,
                    "kind": kind,
                    "ok": true,
                    "emailId": email_id,
                    "messageId": mid,
                    "url": fastmail_web_url_for_message_id(&mid)
                }));
            }
            None => {
                items.push(json!({
                    "id": id,
                    "kind": kind,
                    "ok": false,
                    "emailId": email_id,
                    "url": null,
                    "error": {
                        "kind": "xinNotImplemented",
                        "message": "email missing Message-ID; cannot build Fastmail URL"
                    }
                }));
            }
        }
    }

    Envelope::ok(command_name, account, json!({"items": items}), Meta::default())
}
