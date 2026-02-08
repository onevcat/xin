use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use std::fs;

use crate::backend::Backend;
use crate::cli::SendArgs;
use crate::error::XinErrorOut;
use crate::output::{Envelope, Meta};

fn read_text_arg(value: &str) -> Result<String, XinErrorOut> {
    if let Some(path) = value.strip_prefix('@') {
        fs::read_to_string(path)
            .map_err(|e| XinErrorOut::usage(format!("failed to read text file {path}: {e}")))
    } else {
        Ok(value.to_string())
    }
}

fn to_rfc3339(ts: Option<i64>) -> Option<String> {
    ts.and_then(|v| DateTime::<Utc>::from_timestamp(v, 0))
        .map(|dt| dt.to_rfc3339())
}

pub async fn send(account: Option<String>, args: &SendArgs) -> Envelope<Value> {
    let backend = match Backend::connect().await {
        Ok(b) => b,
        Err(e) => return Envelope::err("send", account, e),
    };

    let text = match read_text_arg(&args.text) {
        Ok(t) => t,
        Err(e) => return Envelope::err("send", account, e),
    };

    let mailboxes = match backend.list_mailboxes().await {
        Ok(m) => m,
        Err(e) => return Envelope::err("send", account, e),
    };

    let drafts_id = mailboxes
        .iter()
        .find(|m| m.role() == jmap_client::mailbox::Role::Drafts)
        .and_then(|m| m.id())
        .map(|id| id.to_string());

    let drafts_id = match drafts_id {
        Some(id) => id,
        None => {
            return Envelope::err(
                "send",
                account,
                XinErrorOut::config("drafts mailbox not found".to_string()),
            );
        }
    };

    let identities = match backend.list_identities().await {
        Ok(i) => i,
        Err(e) => return Envelope::err("send", account, e),
    };

    let identity = match identities.first() {
        Some(i) => i,
        None => {
            return Envelope::err(
                "send",
                account,
                XinErrorOut::config("no identities available".to_string()),
            );
        }
    };

    let identity_id = match identity.id() {
        Some(id) => id.to_string(),
        None => {
            return Envelope::err(
                "send",
                account,
                XinErrorOut::config("identity missing id".to_string()),
            );
        }
    };

    let from_email = match identity.email() {
        Some(email) => email.to_string(),
        None => {
            return Envelope::err(
                "send",
                account,
                XinErrorOut::config("identity missing email".to_string()),
            );
        }
    };

    let from_name = identity.name().map(|n| n.to_string());

    let email = match backend
        .create_text_email(
            &drafts_id,
            from_name,
            from_email,
            &args.to,
            &args.subject,
            &text,
        )
        .await
    {
        Ok(e) => e,
        Err(e) => return Envelope::err("send", account, e),
    };

    let email_id = match email.id() {
        Some(id) => id.to_string(),
        None => {
            return Envelope::err(
                "send",
                account,
                XinErrorOut::config("Email/set did not return email id".to_string()),
            );
        }
    };

    let submission = match backend.submit_email(&email_id, &identity_id).await {
        Ok(s) => s,
        Err(e) => return Envelope::err("send", account, e),
    };

    let data = json!({
        "draft": {
            "emailId": email_id,
            "threadId": email.thread_id()
        },
        "submission": {
            "id": submission.id(),
            "sendAt": to_rfc3339(submission.send_at())
        },
        "uploaded": []
    });

    Envelope::ok("send", account, data, Meta::default())
}
