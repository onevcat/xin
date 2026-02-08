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

fn email_metadata_object(email: &Email) -> Value {
    json!({
        "emailId": email.id(),
        "threadId": email.thread_id(),
        "receivedAt": received_at_rfc3339(email),
        "subject": email.subject(),
        "from": email.from(),
        "to": email.to(),
        "cc": email.cc(),
        "bcc": email.bcc(),
        "mailboxIds": email
            .mailbox_ids()
            .iter()
            .map(|id| (id.to_string(), Value::Bool(true)))
            .collect::<serde_json::Map<String, Value>>(),
        "keywords": email
            .keywords()
            .iter()
            .map(|k| (k.to_string(), Value::Bool(true)))
            .collect::<serde_json::Map<String, Value>>(),
        "hasAttachment": email.has_attachment(),
        "preview": email.preview(),
    })
}

pub fn get_email_data(email: &Email, raw: Option<Value>) -> Value {
    // v0 metadata-first: keep a stable shape; fill more fields as READ expands.
    json!({
        "email": email_metadata_object(email),
        "body": {"text": null, "html": null},
        "attachments": [],
        "raw": raw
    })
}

pub fn thread_get_data(thread_id: &str, email_ids: &[String], emails: &[Email]) -> Value {
    json!({
        "threadId": thread_id,
        "emailIds": email_ids,
        "emails": emails.iter().map(email_metadata_object).collect::<Vec<_>>()
    })
}

pub fn thread_attachments_data(thread_id: &str, emails: &[Email]) -> Value {
    let mut out: Vec<Value> = Vec::new();

    for e in emails {
        let email_id = e.id();
        if let Some(parts) = e.attachments() {
            for p in parts {
                out.push(json!({
                    "emailId": email_id,
                    "blobId": p.blob_id(),
                    "name": p.name(),
                    "type": p.content_type(),
                    "size": p.size(),
                    "disposition": p.content_disposition()
                }));
            }
        }
    }

    json!({
        "threadId": thread_id,
        "attachments": out
    })
}
