use chrono::{DateTime, Utc};
use jmap_client::email::Email;
use serde_json::{Value, json};

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

#[derive(Debug, Clone, serde::Serialize)]
struct BodyMeta {
    #[serde(rename = "isTruncated")]
    is_truncated: bool,
    #[serde(rename = "isEncodingProblem")]
    is_encoding_problem: bool,
}

fn extract_body_value(email: &Email, part_id: &str) -> (Option<String>, Option<BodyMeta>) {
    let bv = match email.body_value(part_id) {
        Some(v) => v,
        None => return (None, None),
    };

    let meta = BodyMeta {
        is_truncated: bv.is_truncated(),
        is_encoding_problem: bv.is_encoding_problem(),
    };

    (Some(bv.value().to_string()), Some(meta))
}

pub fn extract_full_body(email: &Email, max_body_value_bytes: usize) -> (Value, Vec<String>) {
    let mut warnings: Vec<String> = Vec::new();

    let (text, text_meta) = email
        .text_body()
        .and_then(|parts| parts.first())
        .and_then(|p| p.part_id())
        .map(|pid| extract_body_value(email, pid))
        .unwrap_or((None, None));

    let (html, html_meta) = email
        .html_body()
        .and_then(|parts| parts.first())
        .and_then(|p| p.part_id())
        .map(|pid| extract_body_value(email, pid))
        .unwrap_or((None, None));

    if let Some(meta) = &text_meta {
        if meta.is_truncated {
            warnings.push(format!(
                "body.text truncated (maxBodyValueBytes={})",
                max_body_value_bytes
            ));
        }
    }
    if let Some(meta) = &html_meta {
        if meta.is_truncated {
            warnings.push(format!(
                "body.html truncated (maxBodyValueBytes={})",
                max_body_value_bytes
            ));
        }
    }

    let body = json!({
        "text": text,
        "html": html,
        "textMeta": text_meta,
        "htmlMeta": html_meta
    });

    (body, warnings)
}

pub fn extract_attachments(email: &Email) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();

    if let Some(parts) = email.attachments() {
        for p in parts {
            out.push(json!({
                "emailId": email.id(),
                "blobId": p.blob_id(),
                "name": p.name(),
                "type": p.content_type(),
                "size": p.size(),
                "disposition": p.content_disposition()
            }));
        }
    }

    out
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

pub fn get_email_full_data(
    email: &Email,
    raw: Option<Value>,
    max_body_value_bytes: usize,
) -> (Value, Vec<String>) {
    let (body, warnings) = extract_full_body(email, max_body_value_bytes);
    let attachments = extract_attachments(email);

    (
        json!({
            "email": email_metadata_object(email),
            "body": body,
            "attachments": attachments,
            "raw": raw
        }),
        warnings,
    )
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
