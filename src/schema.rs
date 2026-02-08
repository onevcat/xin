use chrono::{DateTime, Utc};
use jmap_client::email::Email;
use serde_json::{json, Value};

fn received_at_rfc3339(e: &Email) -> Option<String> {
    e.received_at()
        .and_then(|ts| DateTime::<Utc>::from_timestamp(ts, 0))
        .map(|dt| dt.to_rfc3339())
}

pub fn email_summary_item(e: &Email) -> Value {
    let keywords = e
        .keywords()
        .iter()
        .map(|k| (k.to_string(), Value::Bool(true)))
        .collect::<serde_json::Map<String, Value>>();

    let mailbox_ids = e
        .mailbox_ids()
        .iter()
        .map(|id| (id.to_string(), Value::Bool(true)))
        .collect::<serde_json::Map<String, Value>>();

    let unread = !e.keywords().iter().any(|k| *k == "$seen");

    json!({
        "threadId": e.thread_id(),
        "emailId": e.id(),
        "receivedAt": received_at_rfc3339(e),
        "subject": e.subject(),
        "from": e.from(),
        "to": e.to(),
        "snippet": e.preview(),
        "hasAttachment": e.has_attachment(),
        "mailboxIds": mailbox_ids,
        "keywords": keywords,
        "unread": unread
    })
}

pub fn email_summary_items(emails: &[Email]) -> Vec<Value> {
    emails.iter().map(email_summary_item).collect()
}
