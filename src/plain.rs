use serde_json::Value;

use crate::output::Envelope;

fn sanitize_field(s: &str) -> String {
    s.replace(['\t', '\n', '\r'], " ")
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i >= max.saturating_sub(1) {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}

fn tsv(fields: &[String]) -> String {
    fields.join("\t")
}

fn get_str<'a>(v: &'a Value, ptr: &str) -> Option<&'a str> {
    v.pointer(ptr).and_then(|x| x.as_str())
}

fn get_bool(v: &Value, ptr: &str) -> Option<bool> {
    v.pointer(ptr).and_then(|x| x.as_bool())
}

fn first_email(v: &Value, ptr: &str) -> Option<String> {
    v.pointer(ptr)
        .and_then(|x| x.as_array())
        .and_then(|a| a.first())
        .and_then(|o| o.get("email"))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
}

fn plain_err(env: &Envelope<Value>) -> String {
    let kind = env
        .error
        .as_ref()
        .map(|e| e.kind.as_str())
        .unwrap_or("error");
    let msg = env
        .error
        .as_ref()
        .map(|e| e.message.as_str())
        .unwrap_or("");

    tsv(&[
        "ERR".to_string(),
        env.command.clone(),
        kind.to_string(),
        sanitize_field(msg),
    ])
}

fn plain_ok_fallback(env: &Envelope<Value>) -> String {
    tsv(&["OK".to_string(), env.command.clone()])
}

fn render_search_items(items: &[Value]) -> String {
    let mut lines: Vec<String> = Vec::new();

    for it in items {
        let received_at = it
            .get("receivedAt")
            .and_then(|x| x.as_str())
            .unwrap_or("");
        let from = first_email(it, "/from").unwrap_or_default();
        let subject = it.get("subject").and_then(|x| x.as_str()).unwrap_or("");
        let unread = it.get("unread").and_then(|x| x.as_bool()).unwrap_or(false);
        let has_att = it
            .get("hasAttachment")
            .and_then(|x| x.as_bool())
            .unwrap_or(false);
        let thread_id = it.get("threadId").and_then(|x| x.as_str()).unwrap_or("");
        let email_id = it.get("emailId").and_then(|x| x.as_str()).unwrap_or("");

        lines.push(tsv(&[
            received_at.to_string(),
            sanitize_field(&from),
            truncate(&sanitize_field(subject), 120),
            (if unread { "unread" } else { "read" }).to_string(),
            (if has_att { "att" } else { "" }).to_string(),
            thread_id.to_string(),
            email_id.to_string(),
        ]));
    }

    lines.join("\n")
}

fn render_labels_list(env: &Envelope<Value>) -> String {
    let data = match env.data.as_ref() {
        Some(d) => d,
        None => return plain_ok_fallback(env),
    };

    let mbs = match data.pointer("/mailboxes").and_then(|x| x.as_array()) {
        Some(a) => a,
        None => return plain_ok_fallback(env),
    };

    let mut lines: Vec<String> = Vec::new();
    for m in mbs {
        let id = m.get("id").and_then(|x| x.as_str()).unwrap_or("");
        let role = m.get("role").and_then(|x| x.as_str()).unwrap_or("");
        let name = m.get("name").and_then(|x| x.as_str()).unwrap_or("");
        lines.push(tsv(&[
            id.to_string(),
            role.to_string(),
            sanitize_field(name),
        ]));
    }

    lines.join("\n")
}

fn render_thread_attachments(env: &Envelope<Value>) -> String {
    let data = match env.data.as_ref() {
        Some(d) => d,
        None => return plain_ok_fallback(env),
    };
    let atts = match data.pointer("/attachments").and_then(|x| x.as_array()) {
        Some(a) => a,
        None => return plain_ok_fallback(env),
    };

    let mut lines: Vec<String> = Vec::new();
    for a in atts {
        let email_id = a.get("emailId").and_then(|x| x.as_str()).unwrap_or("");
        let blob_id = a.get("blobId").and_then(|x| x.as_str()).unwrap_or("");
        let name = a.get("name").and_then(|x| x.as_str()).unwrap_or("");
        let ty = a.get("type").and_then(|x| x.as_str()).unwrap_or("");
        let size = a.get("size").and_then(|x| x.as_i64()).unwrap_or(0);

        lines.push(tsv(&[
            email_id.to_string(),
            blob_id.to_string(),
            sanitize_field(name),
            ty.to_string(),
            size.to_string(),
        ]));
    }

    lines.join("\n")
}

fn render_attachment(env: &Envelope<Value>) -> String {
    let data = match env.data.as_ref() {
        Some(d) => d,
        None => return plain_ok_fallback(env),
    };

    let out = get_str(data, "/out").unwrap_or("");
    let bytes = data.pointer("/bytes").and_then(|x| x.as_i64()).unwrap_or(0);

    tsv(&[
        out.to_string(),
        bytes.to_string(),
        get_str(data, "/blobId").unwrap_or("").to_string(),
    ])
}

fn render_history(env: &Envelope<Value>) -> String {
    let data = match env.data.as_ref() {
        Some(d) => d,
        None => return plain_ok_fallback(env),
    };

    let since = get_str(data, "/sinceState").unwrap_or("");
    let new = get_str(data, "/newState").unwrap_or("");

    let created = data
        .pointer("/changes/created")
        .and_then(|x| x.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let updated = data
        .pointer("/changes/updated")
        .and_then(|x| x.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let destroyed = data
        .pointer("/changes/destroyed")
        .and_then(|x| x.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    let has_more = get_bool(data, "/hasMoreChanges").unwrap_or(false);
    let next_page = env.meta.next_page.clone().unwrap_or_default();

    tsv(&[
        format!("{since} -> {new}"),
        format!("created={created}"),
        format!("updated={updated}"),
        format!("destroyed={destroyed}"),
        format!("hasMore={has_more}"),
        (if next_page.is_empty() {
            "".to_string()
        } else {
            format!("nextPage={next_page}")
        }),
    ])
}

fn render_get(env: &Envelope<Value>) -> String {
    let data = match env.data.as_ref() {
        Some(d) => d,
        None => return plain_ok_fallback(env),
    };

    let email = match data.pointer("/email").and_then(|x| x.as_object()) {
        Some(o) => o,
        None => return plain_ok_fallback(env),
    };

    let subject = email.get("subject").and_then(|x| x.as_str()).unwrap_or("");
    let from = data
        .pointer("/email/from")
        .and_then(|x| x.as_array())
        .and_then(|a| a.first())
        .and_then(|o| o.get("email"))
        .and_then(|x| x.as_str())
        .unwrap_or("");
    let to = data
        .pointer("/email/to")
        .and_then(|x| x.as_array())
        .and_then(|a| a.first())
        .and_then(|o| o.get("email"))
        .and_then(|x| x.as_str())
        .unwrap_or("");

    let received_at = email
        .get("receivedAt")
        .and_then(|x| x.as_str())
        .unwrap_or("");
    let thread_id = email.get("threadId").and_then(|x| x.as_str()).unwrap_or("");
    let email_id = email.get("emailId").and_then(|x| x.as_str()).unwrap_or("");

    let unread = !data
        .pointer("/email/keywords/$seen")
        .and_then(|x| x.as_bool())
        .unwrap_or(false);

    let has_att = email
        .get("hasAttachment")
        .and_then(|x| x.as_bool())
        .unwrap_or(false);

    let preview = email.get("preview").and_then(|x| x.as_str()).unwrap_or("");

    let mut out = String::new();
    out.push_str(&format!("Subject: {}\n", subject));
    out.push_str(&format!("From:    {}\n", from));
    out.push_str(&format!("To:      {}\n", to));
    out.push_str(&format!("Date:    {}\n", received_at));
    out.push_str(&format!("Thread:  {}\n", thread_id));
    out.push_str(&format!("Email:   {}\n", email_id));
    out.push_str(&format!("Unread:  {}\n", if unread { "true" } else { "false" }));
    out.push_str(&format!("Attach:  {}\n", if has_att { "true" } else { "false" }));

    if !preview.is_empty() {
        out.push_str("\nPreview:\n");
        out.push_str(&truncate(preview, 400));
        out.push('\n');
    }

    // body (full)
    let body_text = get_str(data, "/body/text").unwrap_or("");
    if !body_text.is_empty() {
        out.push_str("\nBody (text):\n");
        out.push_str(body_text);
        if !body_text.ends_with('\n') {
            out.push('\n');
        }
    }

    // attachments
    if let Some(atts) = data.pointer("/attachments").and_then(|x| x.as_array()) {
        if !atts.is_empty() {
            out.push_str("\nAttachments:\n");
            for a in atts {
                let name = a.get("name").and_then(|x| x.as_str()).unwrap_or("");
                let ty = a.get("type").and_then(|x| x.as_str()).unwrap_or("");
                let size = a.get("size").and_then(|x| x.as_i64()).unwrap_or(0);
                let blob = a.get("blobId").and_then(|x| x.as_str()).unwrap_or("");
                out.push_str(&format!("- {}\t{}\t{}\t{}\n", sanitize_field(name), ty, size, blob));
            }
        }
    }

    out.trim_end_matches('\n').to_string()
}

pub fn render(env: &Envelope<Value>) -> String {
    if !env.ok {
        return plain_err(env);
    }

    match env.command.as_str() {
        "search" | "messages.search" | "drafts.list" => {
            if let Some(data) = env.data.as_ref() {
                if let Some(items) = data.pointer("/items").and_then(|x| x.as_array()) {
                    return render_search_items(items);
                }
            }
            plain_ok_fallback(env)
        }

        "labels.list" | "mailboxes.list" => render_labels_list(env),

        "thread.attachments" => render_thread_attachments(env),

        "attachment" => render_attachment(env),

        "history" => render_history(env),

        "get" => render_get(env),

        _ => plain_ok_fallback(env),
    }
}
