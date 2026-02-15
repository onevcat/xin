use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

use crate::backend::{Backend, ModifyPlan, UploadedBlob};
use crate::cli::{
    DraftsCreateArgs, DraftsDeleteArgs, DraftsDestroyArgs, DraftsGetArgs, DraftsListArgs,
    DraftsRewriteArgs, DraftsSendArgs, DraftsUpdateArgs, IdentitiesGetArgs, ReplyArgs, SendArgs,
};
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

fn resolve_mailbox_id(s: &str, mailboxes: &[jmap_client::mailbox::Mailbox]) -> Option<String> {
    let needle = s.trim();
    if needle.is_empty() {
        return None;
    }

    // 0) direct id match
    for m in mailboxes {
        if let Some(id) = m.id() {
            if id == needle {
                return Some(id.to_string());
            }
        }
    }

    // 1) role match
    let needle_lower = needle.to_lowercase();
    let role = match needle_lower.as_str() {
        "spam" => "junk",
        "bin" => "trash",
        other => other,
    };

    let role_match = mailboxes.iter().find(|m| {
        use jmap_client::mailbox::Role;
        match (role, m.role()) {
            ("inbox", Role::Inbox)
            | ("trash", Role::Trash)
            | ("junk", Role::Junk)
            | ("sent", Role::Sent)
            | ("drafts", Role::Drafts)
            | ("archive", Role::Archive)
            | ("important", Role::Important) => true,
            (other, Role::Other(s)) => other == s.to_lowercase(),
            _ => false,
        }
    });

    if let Some(m) = role_match {
        return m.id().map(|id| id.to_string());
    }

    // 2) exact name match
    if let Some(m) = mailboxes.iter().find(|m| m.name() == Some(needle)) {
        return m.id().map(|id| id.to_string());
    }

    // 3) case-insensitive name match
    let lower = needle.to_lowercase();
    if let Some(m) = mailboxes
        .iter()
        .find(|m| m.name().map(|n| n.to_lowercase()) == Some(lower.clone()))
    {
        return m.id().map(|id| id.to_string());
    }

    None
}

/// Build reply headers (In-Reply-To + References) from the original email.
fn build_reply_headers(
    original: &jmap_client::email::Email,
) -> Result<Vec<(String, String)>, XinErrorOut> {
    let orig_msg_id = original
        .message_id()
        .and_then(|ids| ids.first())
        .cloned()
        .ok_or_else(|| XinErrorOut::usage("original email missing Message-ID".to_string()))?;

    let mut headers: Vec<(String, String)> = Vec::new();
    headers.push(("In-Reply-To".to_string(), orig_msg_id.clone()));

    let mut refs: Vec<String> = Vec::new();
    if let Some(existing_refs) = original.references() {
        refs.extend(existing_refs.iter().cloned());
    }
    refs.push(orig_msg_id);
    headers.push(("References".to_string(), refs.join(" ")));
    Ok(headers)
}

fn default_reply_subject(original_subject: Option<&str>) -> String {
    let s = original_subject.unwrap_or("");
    let trimmed = s.trim();
    if trimmed.to_lowercase().starts_with("re:") {
        trimmed.to_string()
    } else if trimmed.is_empty() {
        "Re:".to_string()
    } else {
        format!("Re: {trimmed}")
    }
}

/// Infer reply recipients based on original email and reply args.
///
/// Rules:
/// - If args.to is provided (override), use it as-is.
/// - Otherwise infer `To` from original `Reply-To` (preferred) or `From`.
/// - If reply_all is set, include original To + Cc in CC (excluding self email).
fn infer_reply_recipients(
    original: &jmap_client::email::Email,
    reply_all: bool,
    override_to: &[String],
    override_cc: &[String],
    self_email: &str,
) -> (Vec<String>, Vec<String>) {
    use std::collections::HashSet;

    fn extract_email(s: &str) -> String {
        if let (Some(l), Some(r)) = (s.rfind('<'), s.rfind('>')) {
            if l < r {
                return s[l + 1..r].trim().to_string();
            }
        }
        s.trim().to_string()
    }

    fn key(s: &str) -> String {
        extract_email(s).to_lowercase()
    }

    let self_key = self_email.to_lowercase();

    let mut to: Vec<String> = override_to.to_vec();
    let mut cc: Vec<String> = override_cc.to_vec();

    let mut seen: HashSet<String> = HashSet::new();
    for r in to.iter().chain(cc.iter()) {
        seen.insert(key(r));
    }

    let mut push_unique = |list: &mut Vec<String>, raw: String| {
        let k = key(&raw);
        if !seen.contains(&k) {
            seen.insert(k);
            list.push(raw);
        }
    };

    let fmt_addr = |a: &jmap_client::email::EmailAddress| -> String { a.email().to_string() };

    if to.is_empty() {
        // Prefer Reply-To when present (mailing lists / automated senders often set it).
        if let Some(reply_to_addrs) = original.reply_to() {
            for addr in reply_to_addrs {
                push_unique(&mut to, fmt_addr(addr));
            }
        } else if let Some(from_addrs) = original.from() {
            for addr in from_addrs {
                push_unique(&mut to, fmt_addr(addr));
            }
        }
    }

    if reply_all {
        let mut push_cc = |raw: String| {
            let k = key(&raw);
            if k == self_key {
                return;
            }
            if !seen.contains(&k) {
                seen.insert(k);
                cc.push(raw);
            }
        };

        if let Some(to_addrs) = original.to() {
            for addr in to_addrs {
                push_cc(fmt_addr(addr));
            }
        }
        if let Some(cc_addrs) = original.cc() {
            for addr in cc_addrs {
                push_cc(fmt_addr(addr));
            }
        }
    }

    (to, cc)
}

fn find_drafts_mailbox_id(
    mailboxes: &[jmap_client::mailbox::Mailbox],
) -> Result<String, XinErrorOut> {
    // Prefer RFC role-based resolution, but some servers may omit roles.
    if let Some(id) = mailboxes
        .iter()
        .find(|m| m.role() == jmap_client::mailbox::Role::Drafts)
        .and_then(|m| m.id())
    {
        return Ok(id.to_string());
    }

    // Fallback: name match.
    if let Some(id) = mailboxes
        .iter()
        .find(|m| m.name() == Some("Drafts"))
        .and_then(|m| m.id())
    {
        return Ok(id.to_string());
    }

    let want = "drafts";
    if let Some(id) = mailboxes
        .iter()
        .find(|m| m.name().map(|n| n.to_lowercase()) == Some(want.to_string()))
        .and_then(|m| m.id())
    {
        return Ok(id.to_string());
    }

    // Some servers/users may use singular naming.
    if let Some(id) = mailboxes
        .iter()
        .find(|m| m.name().map(|n| n.to_lowercase()) == Some("draft".to_string()))
        .and_then(|m| m.id())
    {
        return Ok(id.to_string());
    }

    Err(XinErrorOut::config("drafts mailbox not found".to_string()))
}

fn resolve_identity(
    identities: &[jmap_client::identity::Identity],
    selector: Option<&str>,
) -> Result<(String, Option<String>, String), XinErrorOut> {
    let idt = match selector {
        None => identities
            .first()
            .ok_or_else(|| XinErrorOut::config("no identities available".to_string()))?,
        Some(sel) => {
            let sel_lower = sel.to_lowercase();
            identities
                .iter()
                .find(|i| i.id().map(|id| id == sel).unwrap_or(false))
                .or_else(|| {
                    identities
                        .iter()
                        .find(|i| i.email().map(|e| e.to_lowercase() == sel_lower).unwrap_or(false))
                })
                .ok_or_else(|| {
                    XinErrorOut::usage(format!(
                        "unknown identity: {sel} (use `xin identities list` to see available identities)"
                    ))
                })?
        }
    };

    let identity_id = idt
        .id()
        .map(|id| id.to_string())
        .ok_or_else(|| XinErrorOut::config("identity missing id".to_string()))?;

    let from_email = idt
        .email()
        .map(|email| email.to_string())
        .ok_or_else(|| XinErrorOut::config("identity missing email".to_string()))?;

    let from_name = idt.name().map(|n| n.to_string());

    Ok((identity_id, from_name, from_email))
}

fn infer_filename(path: &str) -> Option<String> {
    Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}

fn guess_content_type(path: &str) -> Option<&'static str> {
    // Minimal heuristics; unknown types fall back to application/octet-stream.
    let ext = Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "txt" => Some("text/plain"),
        "md" => Some("text/markdown"),
        "html" | "htm" => Some("text/html"),
        "json" => Some("application/json"),
        "pdf" => Some("application/pdf"),
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        _ => None,
    }
}

async fn upload_attachments(
    backend: &Backend,
    paths: &[String],
) -> Result<Vec<UploadedBlob>, XinErrorOut> {
    let mut uploaded: Vec<UploadedBlob> = Vec::new();

    for p in paths {
        let bytes = fs::read(p)
            .map_err(|e| XinErrorOut::usage(format!("failed to read attachment {p}: {e}")))?;

        let content_type = guess_content_type(p).unwrap_or("application/octet-stream");
        let name = infer_filename(p);
        uploaded.push(backend.upload_blob(bytes, Some(content_type), name).await?);
    }

    Ok(uploaded)
}

pub async fn identities_list(account: Option<String>) -> Envelope<Value> {
    let backend = match Backend::connect(account.as_deref()).await {
        Ok(b) => b,
        Err(e) => return Envelope::err("identities.list", account, e),
    };

    let identities = match backend.list_identities().await {
        Ok(i) => i,
        Err(e) => return Envelope::err("identities.list", account, e),
    };

    let out = identities
        .iter()
        .map(|i| {
            json!({
                "id": i.id(),
                "name": i.name(),
                "email": i.email()
            })
        })
        .collect::<Vec<_>>();

    Envelope::ok(
        "identities.list",
        account,
        json!({"identities": out}),
        Meta::default(),
    )
}

pub async fn identities_get(account: Option<String>, args: &IdentitiesGetArgs) -> Envelope<Value> {
    let backend = match Backend::connect(account.as_deref()).await {
        Ok(b) => b,
        Err(e) => return Envelope::err("identities.get", account, e),
    };

    let identities = match backend.list_identities().await {
        Ok(i) => i,
        Err(e) => return Envelope::err("identities.get", account, e),
    };

    let sel = args.id.as_str();
    let found = identities.iter().find(|i| {
        i.id().map(|id| id == sel).unwrap_or(false) || i.email().map(|e| e == sel).unwrap_or(false)
    });

    let Some(i) = found else {
        return Envelope::err(
            "identities.get",
            account,
            XinErrorOut::usage(format!(
                "identity not found: {sel} (use `xin identities list`)"
            )),
        );
    };

    Envelope::ok(
        "identities.get",
        account,
        json!({
            "identity": {"id": i.id(), "name": i.name(), "email": i.email()}
        }),
        Meta::default(),
    )
}

pub async fn send(account: Option<String>, args: &SendArgs) -> Envelope<Value> {
    let backend = match Backend::connect(account.as_deref()).await {
        Ok(b) => b,
        Err(e) => return Envelope::err("send", account, e),
    };

    // Resolve Drafts mailbox.
    let mailboxes = match backend.list_mailboxes().await {
        Ok(m) => m,
        Err(e) => return Envelope::err("send", account, e),
    };

    let drafts_id = match find_drafts_mailbox_id(&mailboxes) {
        Ok(id) => id,
        Err(e) => return Envelope::err("send", account, e),
    };

    let identities = match backend.list_identities().await {
        Ok(i) => i,
        Err(e) => return Envelope::err("send", account, e),
    };

    let (identity_id, from_name, from_email) =
        match resolve_identity(&identities, args.identity.as_deref()) {
            Ok(v) => v,
            Err(e) => return Envelope::err("send", account, e),
        };

    let to = args.to.clone();
    let cc = args.cc.clone();

    let text = match &args.text {
        Some(v) => Some(match read_text_arg(v) {
            Ok(t) => t,
            Err(e) => return Envelope::err("send", account, e),
        }),
        None => None,
    };

    let html = match &args.body_html {
        Some(v) => Some(match read_text_arg(v) {
            Ok(t) => t,
            Err(e) => return Envelope::err("send", account, e),
        }),
        None => None,
    };

    if text.is_none() && html.is_none() && args.attach.is_empty() {
        return Envelope::err(
            "send",
            account,
            XinErrorOut::usage(
                "missing message content: provide --text, --body-html, or --attach".to_string(),
            ),
        );
    }

    let uploaded = match upload_attachments(&backend, &args.attach).await {
        Ok(v) => v,
        Err(e) => return Envelope::err("send", account, e),
    };

    let email = match backend
        .create_draft_email(
            &drafts_id,
            from_name,
            from_email,
            &to,
            &cc,
            &args.bcc,
            Some(&args.subject),
            text.as_deref(),
            html.as_deref(),
            &uploaded,
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

    let uploaded_out = uploaded
        .iter()
        .map(|u| {
            json!({
                "blobId": u.blob_id,
                "type": u.content_type,
                "size": u.size,
            })
        })
        .collect::<Vec<_>>();

    let data = json!({
        "draft": {
            "emailId": email_id,
            "threadId": email.thread_id()
        },
        "submission": {
            "id": submission.id(),
            "sendAt": to_rfc3339(submission.send_at())
        },
        "uploaded": uploaded_out
    });

    Envelope::ok("send", account, data, Meta::default())
}

pub async fn reply(account: Option<String>, args: &ReplyArgs) -> Envelope<Value> {
    let backend = match Backend::connect(account.as_deref()).await {
        Ok(b) => b,
        Err(e) => return Envelope::err("reply", account, e),
    };

    // Resolve Drafts mailbox.
    let mailboxes = match backend.list_mailboxes().await {
        Ok(m) => m,
        Err(e) => return Envelope::err("reply", account, e),
    };

    let drafts_id = match find_drafts_mailbox_id(&mailboxes) {
        Ok(id) => id,
        Err(e) => return Envelope::err("reply", account, e),
    };

    // Resolve sending identity (needed for EmailSubmission and for reply-all self exclusion).
    let identities = match backend.list_identities().await {
        Ok(i) => i,
        Err(e) => return Envelope::err("reply", account, e),
    };

    let (identity_id, from_name, from_email) =
        match resolve_identity(&identities, args.identity.as_deref()) {
            Ok(v) => v,
            Err(e) => return Envelope::err("reply", account, e),
        };

    // Fetch original email by emailId.
    let original = match backend
        .get_email(
            &args.email_id,
            Some(vec![
                jmap_client::email::Property::Id,
                jmap_client::email::Property::ThreadId,
                jmap_client::email::Property::MessageId,
                jmap_client::email::Property::References,
                jmap_client::email::Property::From,
                jmap_client::email::Property::ReplyTo,
                jmap_client::email::Property::To,
                jmap_client::email::Property::Cc,
                jmap_client::email::Property::Subject,
            ]),
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return Envelope::err("reply", account, e),
    };

    let Some(original) = original else {
        return Envelope::err(
            "reply",
            account,
            XinErrorOut::usage(format!("original email not found: {}", args.email_id)),
        );
    };

    let headers = match build_reply_headers(&original) {
        Ok(h) => h,
        Err(e) => return Envelope::err("reply", account, e),
    };

    let (to, cc) =
        infer_reply_recipients(&original, args.reply_all, &args.to, &args.cc, &from_email);

    if to.is_empty() {
        return Envelope::err(
            "reply",
            account,
            XinErrorOut::usage("could not infer reply recipients (provide --to)".to_string()),
        );
    }

    let subject = args
        .subject
        .clone()
        .unwrap_or_else(|| default_reply_subject(original.subject()));

    let text = match &args.text {
        Some(v) => Some(match read_text_arg(v) {
            Ok(t) => t,
            Err(e) => return Envelope::err("reply", account, e),
        }),
        None => None,
    };

    let html = match &args.body_html {
        Some(v) => Some(match read_text_arg(v) {
            Ok(t) => t,
            Err(e) => return Envelope::err("reply", account, e),
        }),
        None => None,
    };

    if text.is_none() && html.is_none() && args.attach.is_empty() {
        return Envelope::err(
            "reply",
            account,
            XinErrorOut::usage(
                "missing message content: provide --text, --body-html, or --attach".to_string(),
            ),
        );
    }

    // Merge explicit BCC.
    let bcc = args.bcc.clone();

    let uploaded = match upload_attachments(&backend, &args.attach).await {
        Ok(v) => v,
        Err(e) => return Envelope::err("reply", account, e),
    };

    let email = match backend
        .create_draft_email_with_headers(
            &drafts_id,
            from_name,
            from_email,
            &to,
            &cc,
            &bcc,
            Some(&subject),
            text.as_deref(),
            html.as_deref(),
            &uploaded,
            Some(&headers),
        )
        .await
    {
        Ok(e) => e,
        Err(e) => return Envelope::err("reply", account, e),
    };

    let email_id = match email.id() {
        Some(id) => id.to_string(),
        None => {
            return Envelope::err(
                "reply",
                account,
                XinErrorOut::config("Email/set did not return email id".to_string()),
            );
        }
    };

    let submission = match backend.submit_email(&email_id, &identity_id).await {
        Ok(s) => s,
        Err(e) => return Envelope::err("reply", account, e),
    };

    let uploaded_out = uploaded
        .iter()
        .map(|u| {
            json!({
                "blobId": u.blob_id,
                "type": u.content_type,
                "size": u.size,
            })
        })
        .collect::<Vec<_>>();

    let data = json!({
        "draft": {
            "emailId": email_id,
            "threadId": email.thread_id()
        },
        "submission": {
            "id": submission.id(),
            "sendAt": to_rfc3339(submission.send_at())
        },
        "uploaded": uploaded_out
    });

    Envelope::ok("reply", account, data, Meta::default())
}

pub async fn drafts_list(account: Option<String>, args: &DraftsListArgs) -> Envelope<Value> {
    // Reuse READ search machinery for query+hydrate+page tokens.
    let backend = match Backend::connect(account.as_deref()).await {
        Ok(b) => b,
        Err(e) => return Envelope::err("drafts.list", account, e),
    };

    let mailboxes = match backend.list_mailboxes().await {
        Ok(m) => m,
        Err(e) => return Envelope::err("drafts.list", account, e),
    };

    let drafts_id = match find_drafts_mailbox_id(&mailboxes) {
        Ok(id) => id,
        Err(e) => return Envelope::err("drafts.list", account, e),
    };

    let filter_json = json!({"inMailbox": drafts_id}).to_string();

    let search_args = crate::cli::SearchArgs {
        query: None,
        max: args.max,
        page: args.page.clone(),
        oldest: false,
        filter_json: Some(filter_json),
        collapse_threads: Some(false),
        sort: crate::cli::SortKey::ReceivedAt,
    };

    crate::commands::read::search("drafts.list", account, &search_args).await
}

pub async fn drafts_get(account: Option<String>, args: &DraftsGetArgs) -> Envelope<Value> {
    let command_name = "drafts.get";

    let backend = match Backend::connect(account.as_deref()).await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let requested_props = match args.format {
        crate::cli::GetFormat::Metadata => Some(vec![
            jmap_client::email::Property::Id,
            jmap_client::email::Property::ThreadId,
            jmap_client::email::Property::ReceivedAt,
            jmap_client::email::Property::Subject,
            jmap_client::email::Property::From,
            jmap_client::email::Property::To,
            jmap_client::email::Property::Cc,
            jmap_client::email::Property::Bcc,
            jmap_client::email::Property::Preview,
            jmap_client::email::Property::HasAttachment,
            jmap_client::email::Property::MailboxIds,
            jmap_client::email::Property::Keywords,
        ]),
        crate::cli::GetFormat::Raw => None,
        crate::cli::GetFormat::Full => None,
    };

    let max_body_value_bytes = 262_144;

    let email = match args.format {
        crate::cli::GetFormat::Full => match backend
            .get_email_full(&args.draft_email_id, max_body_value_bytes, vec![])
            .await
        {
            Ok(Some(e)) => e,
            Ok(None) => {
                return Envelope::err(
                    command_name,
                    account,
                    XinErrorOut::usage("draft not found".to_string()),
                );
            }
            Err(e) => return Envelope::err(command_name, account, e),
        },
        _ => match backend
            .get_email(&args.draft_email_id, requested_props)
            .await
        {
            Ok(Some(e)) => e,
            Ok(None) => {
                return Envelope::err(
                    command_name,
                    account,
                    XinErrorOut::usage("draft not found".to_string()),
                );
            }
            Err(e) => return Envelope::err(command_name, account, e),
        },
    };

    let raw = match args.format {
        crate::cli::GetFormat::Raw => serde_json::to_value(&email).ok(),
        _ => None,
    };

    let (data, warnings) = match args.format {
        crate::cli::GetFormat::Full => {
            let (d, w) = crate::schema::get_email_full_data(&email, raw, max_body_value_bytes);
            (d, w)
        }
        _ => (crate::schema::get_email_data(&email, raw), Vec::new()),
    };

    let mut obj = data.as_object().cloned().unwrap_or_default();
    if let Some(v) = obj.remove("email") {
        obj.insert("draft".to_string(), v);
    }

    let mut meta = Meta::default();
    if !warnings.is_empty() {
        meta.warnings = Some(warnings);
    }

    Envelope::ok(command_name, account, Value::Object(obj), meta)
}

pub async fn drafts_create(account: Option<String>, args: &DraftsCreateArgs) -> Envelope<Value> {
    let command_name = "drafts.create";

    let backend = match Backend::connect(account.as_deref()).await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let body_text = match (&args.body, &args.body_file) {
        (Some(_), Some(_)) => {
            return Envelope::err(
                command_name,
                account,
                XinErrorOut::usage("use either --body or --body-file".to_string()),
            );
        }
        (Some(s), None) => Some(match read_text_arg(s) {
            Ok(t) => t,
            Err(e) => return Envelope::err(command_name, account, e),
        }),
        (None, Some(p)) => Some(match read_text_arg(&format!("@{p}")) {
            Ok(t) => t,
            Err(e) => return Envelope::err(command_name, account, e),
        }),
        (None, None) => None,
    };

    let body_html = match &args.body_html {
        Some(s) => Some(match read_text_arg(s) {
            Ok(t) => t,
            Err(e) => return Envelope::err(command_name, account, e),
        }),
        None => None,
    };

    if body_text.is_none() && body_html.is_none() && args.attach.is_empty() {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage(
                "missing draft content: provide --body/--body-file, --body-html, or --attach"
                    .to_string(),
            ),
        );
    }

    let mailboxes = match backend.list_mailboxes().await {
        Ok(m) => m,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let drafts_id = match find_drafts_mailbox_id(&mailboxes) {
        Ok(id) => id,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let identities = match backend.list_identities().await {
        Ok(i) => i,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let (_identity_id, from_name, from_email) =
        match resolve_identity(&identities, args.identity.as_deref()) {
            Ok(v) => v,
            Err(e) => return Envelope::err(command_name, account, e),
        };

    let uploaded = match upload_attachments(&backend, &args.attach).await {
        Ok(v) => v,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let email = match backend
        .create_draft_email(
            &drafts_id,
            from_name,
            from_email,
            &args.to,
            &args.cc,
            &args.bcc,
            args.subject.as_deref(),
            body_text.as_deref(),
            body_html.as_deref(),
            &uploaded,
        )
        .await
    {
        Ok(e) => e,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let email_id = match email.id() {
        Some(id) => id.to_string(),
        None => {
            return Envelope::err(
                command_name,
                account,
                XinErrorOut::config("Email/set did not return email id".to_string()),
            );
        }
    };

    let uploaded_out = uploaded
        .iter()
        .map(|u| {
            json!({
                "blobId": u.blob_id,
                "type": u.content_type,
                "size": u.size,
            })
        })
        .collect::<Vec<_>>();

    Envelope::ok(
        command_name,
        account,
        json!({
            "draft": { "emailId": email_id, "threadId": email.thread_id() },
            "uploaded": uploaded_out
        }),
        Meta::default(),
    )
}

pub async fn drafts_update(account: Option<String>, args: &DraftsUpdateArgs) -> Envelope<Value> {
    let command_name = "drafts.update";

    let backend = match Backend::connect(account.as_deref()).await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    if args.add.is_empty()
        && args.remove.is_empty()
        && args.add_mailbox.is_empty()
        && args.remove_mailbox.is_empty()
        && args.add_keyword.is_empty()
        && args.remove_keyword.is_empty()
    {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage("no changes specified".to_string()),
        );
    }

    let mailboxes = match backend.list_mailboxes().await {
        Ok(m) => m,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let mut plan = ModifyPlan::default();

    // Explicit mailbox flags.
    for m in &args.add_mailbox {
        let id = resolve_mailbox_id(m, &mailboxes)
            .ok_or_else(|| XinErrorOut::usage(format!("unknown mailbox: {m}")));
        let id = match id {
            Ok(v) => v,
            Err(e) => return Envelope::err(command_name, account, e),
        };
        plan.add_mailboxes.push(id);
    }

    for m in &args.remove_mailbox {
        let id = resolve_mailbox_id(m, &mailboxes)
            .ok_or_else(|| XinErrorOut::usage(format!("unknown mailbox: {m}")));
        let id = match id {
            Ok(v) => v,
            Err(e) => return Envelope::err(command_name, account, e),
        };
        plan.remove_mailboxes.push(id);
    }

    for k in &args.add_keyword {
        plan.add_keywords.push(k.clone());
    }

    for k in &args.remove_keyword {
        plan.remove_keywords.push(k.clone());
    }

    // Auto route: mailbox if resolvable, otherwise keyword.
    for t in &args.add {
        if let Some(id) = resolve_mailbox_id(t, &mailboxes) {
            plan.add_mailboxes.push(id);
        } else {
            plan.add_keywords.push(t.clone());
        }
    }

    for t in &args.remove {
        if let Some(id) = resolve_mailbox_id(t, &mailboxes) {
            plan.remove_mailboxes.push(id);
        } else {
            plan.remove_keywords.push(t.clone());
        }
    }

    if let Err(e) = backend
        .modify_emails(&[args.draft_email_id.clone()], &plan)
        .await
    {
        return Envelope::err(command_name, account, e);
    }

    let email = match backend
        .get_email(
            &args.draft_email_id,
            Some(vec![
                jmap_client::email::Property::Id,
                jmap_client::email::Property::ThreadId,
            ]),
        )
        .await
    {
        Ok(Some(e)) => e,
        Ok(None) => {
            return Envelope::err(
                command_name,
                account,
                XinErrorOut::usage("draft not found".to_string()),
            );
        }
        Err(e) => return Envelope::err(command_name, account, e),
    };

    Envelope::ok(
        command_name,
        account,
        json!({
            "draft": { "emailId": args.draft_email_id, "threadId": email.thread_id() },
        }),
        Meta::default(),
    )
}

pub async fn drafts_rewrite(
    account: Option<String>,
    args: &DraftsRewriteArgs,
    force: bool,
) -> Envelope<Value> {
    let command_name = "drafts.rewrite";

    let backend = match Backend::connect(account.as_deref()).await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    if args.body.is_some() && args.body_file.is_some() {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage("use either --body or --body-file".to_string()),
        );
    }

    if args.clear_attachments && (!args.attach.is_empty() || args.replace_attachments) {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage(
                "--clear-attachments cannot be combined with --attach/--replace-attachments"
                    .to_string(),
            ),
        );
    }

    if args.replace_attachments && args.attach.is_empty() {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage("--replace-attachments requires at least one --attach".to_string()),
        );
    }

    let max_body_value_bytes = 1_048_576;
    let existing = match backend
        .get_email_full(&args.draft_email_id, max_body_value_bytes, vec![])
        .await
    {
        Ok(Some(e)) => e,
        Ok(None) => {
            return Envelope::err(
                command_name,
                account,
                XinErrorOut::usage("draft not found".to_string()),
            );
        }
        Err(e) => return Envelope::err(command_name, account, e),
    };

    // Resolve Drafts mailbox id for the new draft.
    let mailboxes = match backend.list_mailboxes().await {
        Ok(m) => m,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let drafts_id = match find_drafts_mailbox_id(&mailboxes) {
        Ok(id) => id,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    // Existing envelope fields.
    let existing_to = existing
        .to()
        .map(|v| v.iter().map(|a| a.email().to_string()).collect::<Vec<_>>())
        .unwrap_or_default();
    let existing_cc = existing
        .cc()
        .map(|v| v.iter().map(|a| a.email().to_string()).collect::<Vec<_>>())
        .unwrap_or_default();
    let existing_bcc = existing
        .bcc()
        .map(|v| v.iter().map(|a| a.email().to_string()).collect::<Vec<_>>())
        .unwrap_or_default();
    let existing_subject = existing.subject().map(|s| s.to_string());

    // Existing body.
    let (b, _warnings) = crate::schema::extract_full_body(&existing, max_body_value_bytes);
    let existing_text = b
        .get("text")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let existing_html = b
        .get("html")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Existing attachments.
    let mut existing_attachments: Vec<UploadedBlob> = Vec::new();
    if let Some(parts) = existing.attachments() {
        for p in parts {
            if let (Some(blob_id), Some(ct)) = (p.blob_id(), p.content_type()) {
                existing_attachments.push(UploadedBlob {
                    blob_id: blob_id.to_string(),
                    content_type: ct.to_string(),
                    size: p.size(),
                    name: p.name().map(|n| n.to_string()),
                });
            }
        }
    }

    // Existing From.
    let mut existing_from_name: Option<String> = None;
    let mut existing_from_email: Option<String> = None;
    if let Some(froms) = existing.from() {
        if let Some(a) = froms.first() {
            existing_from_name = a.name().map(|n| n.to_string());
            existing_from_email = Some(a.email().to_string());
        }
    }

    // Apply overrides.
    let final_to = args.to.clone().unwrap_or(existing_to);
    let final_cc = args.cc.clone().unwrap_or(existing_cc);
    let final_bcc = args.bcc.clone().unwrap_or(existing_bcc);
    let final_subject = args.subject.as_deref().or(existing_subject.as_deref());

    let body_text = match (&args.body, &args.body_file) {
        (Some(s), None) => Some(match read_text_arg(s) {
            Ok(t) => t,
            Err(e) => return Envelope::err(command_name, account, e),
        }),
        (None, Some(p)) => Some(match read_text_arg(&format!("@{p}")) {
            Ok(t) => t,
            Err(e) => return Envelope::err(command_name, account, e),
        }),
        _ => None,
    };

    let body_html = match &args.body_html {
        Some(s) => Some(match read_text_arg(s) {
            Ok(t) => t,
            Err(e) => return Envelope::err(command_name, account, e),
        }),
        None => None,
    };

    let final_text: Option<String> = body_text.or(existing_text);
    let final_html: Option<String> = body_html.or(existing_html);

    let uploaded_new = match upload_attachments(&backend, &args.attach).await {
        Ok(v) => v,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let attachments_for_new_draft: Vec<UploadedBlob> = if args.clear_attachments {
        Vec::new()
    } else if args.replace_attachments {
        uploaded_new.clone()
    } else {
        let mut out = existing_attachments;
        out.extend(uploaded_new.clone());
        out
    };

    let (from_name, from_email) = if let Some(sel) = args.identity.as_deref() {
        let identities = match backend.list_identities().await {
            Ok(i) => i,
            Err(e) => return Envelope::err(command_name, account, e),
        };
        let (_identity_id, name, email) = match resolve_identity(&identities, Some(sel)) {
            Ok(v) => v,
            Err(e) => return Envelope::err(command_name, account, e),
        };
        (name, email)
    } else if let Some(email) = existing_from_email {
        (existing_from_name, email)
    } else {
        let identities = match backend.list_identities().await {
            Ok(i) => i,
            Err(e) => return Envelope::err(command_name, account, e),
        };
        let (_identity_id, name, email) = match resolve_identity(&identities, None) {
            Ok(v) => v,
            Err(e) => return Envelope::err(command_name, account, e),
        };
        (name, email)
    };

    let new_email = match backend
        .create_draft_email(
            &drafts_id,
            from_name,
            from_email,
            &final_to,
            &final_cc,
            &final_bcc,
            final_subject,
            final_text.as_deref(),
            final_html.as_deref(),
            &attachments_for_new_draft,
        )
        .await
    {
        Ok(e) => e,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let new_email_id = match new_email.id() {
        Some(id) => id.to_string(),
        None => {
            return Envelope::err(
                command_name,
                account,
                XinErrorOut::config("Email/set did not return email id".to_string()),
            );
        }
    };

    // Clean up the old draft.
    // Default: non-destructive (remove Drafts mailbox membership + $draft keyword).
    // If --destroy-old is set, requires global --force.
    let mut warnings: Vec<String> = Vec::new();
    if args.destroy_old {
        if !force {
            return Envelope::err(
                command_name,
                account,
                XinErrorOut::usage("--destroy-old requires global --force".to_string()),
            );
        }

        if let Err(e) = backend.destroy_emails(&[args.draft_email_id.clone()]).await {
            warnings.push(format!(
                "failed to destroy replaced draft {}: {}",
                args.draft_email_id, e.message
            ));
        }
    } else {
        let mut plan = ModifyPlan::default();
        plan.remove_mailboxes.push(drafts_id);
        plan.remove_keywords.push("$draft".to_string());
        if let Err(e2) = backend
            .modify_emails(&[args.draft_email_id.clone()], &plan)
            .await
        {
            warnings.push(format!(
                "failed to remove Drafts membership for replaced draft {}: {}",
                args.draft_email_id, e2.message
            ));
        }
    }

    let uploaded_out = uploaded_new
        .iter()
        .map(|u| {
            json!({
                "blobId": u.blob_id,
                "type": u.content_type,
                "size": u.size,
            })
        })
        .collect::<Vec<_>>();

    Envelope::ok(
        command_name,
        account,
        json!({
            "draft": { "emailId": new_email_id, "threadId": new_email.thread_id() },
            "uploaded": uploaded_out,
            "replacedFrom": args.draft_email_id
        }),
        Meta {
            warnings: if warnings.is_empty() {
                None
            } else {
                Some(warnings)
            },
            ..Meta::default()
        },
    )
}

pub async fn drafts_send(account: Option<String>, args: &DraftsSendArgs) -> Envelope<Value> {
    let command_name = "drafts.send";

    let backend = match Backend::connect(account.as_deref()).await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let identities = match backend.list_identities().await {
        Ok(i) => i,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let (identity_id, _from_name, _from_email) =
        match resolve_identity(&identities, args.identity.as_deref()) {
            Ok(v) => v,
            Err(e) => return Envelope::err(command_name, account, e),
        };

    let submission = match backend
        .submit_email(&args.draft_email_id, &identity_id)
        .await
    {
        Ok(s) => s,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let thread_id = backend
        .get_email(
            &args.draft_email_id,
            Some(vec![
                jmap_client::email::Property::Id,
                jmap_client::email::Property::ThreadId,
            ]),
        )
        .await
        .ok()
        .flatten()
        .and_then(|e| e.thread_id().map(|s| s.to_string()));

    Envelope::ok(
        command_name,
        account,
        json!({
            "draft": { "emailId": args.draft_email_id, "threadId": thread_id },
            "submission": { "id": submission.id(), "sendAt": to_rfc3339(submission.send_at()) }
        }),
        Meta::default(),
    )
}

pub async fn drafts_delete(account: Option<String>, args: &DraftsDeleteArgs) -> Envelope<Value> {
    let command_name = "drafts.delete";

    let backend = match Backend::connect(account.as_deref()).await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    if args.draft_email_ids.is_empty() {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage("missing draft id".to_string()),
        );
    }

    let mailboxes = match backend.list_mailboxes().await {
        Ok(m) => m,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let drafts_id = match find_drafts_mailbox_id(&mailboxes) {
        Ok(id) => id,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    // Non-destructive delete: move draft out of Drafts and into Trash.
    // Rationale: Email.mailboxIds must not become empty; some servers may reject removing the
    // last mailbox membership. Moving to Trash preserves recoverability while removing from Drafts.
    let trash_id = resolve_mailbox_id("trash", &mailboxes)
        .ok_or_else(|| XinErrorOut::config("trash mailbox not found".to_string()));
    let trash_id = match trash_id {
        Ok(v) => v,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let mut plan = ModifyPlan::default();
    plan.remove_mailboxes.push(drafts_id);
    plan.add_mailboxes.push(trash_id);
    plan.remove_keywords.push("$draft".to_string());

    if let Err(e) = backend.modify_emails(&args.draft_email_ids, &plan).await {
        return Envelope::err(command_name, account, e);
    }

    Envelope::ok(
        command_name,
        account,
        json!({"deleted": args.draft_email_ids}),
        Meta::default(),
    )
}

pub async fn drafts_destroy(
    account: Option<String>,
    args: &DraftsDestroyArgs,
    force: bool,
) -> Envelope<Value> {
    let command_name = "drafts.destroy";

    if !force {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage("drafts destroy is destructive; pass --force".to_string()),
        );
    }

    let backend = match Backend::connect(account.as_deref()).await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    if args.draft_email_ids.is_empty() {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage("missing draft id".to_string()),
        );
    }

    if let Err(e) = backend.destroy_emails(&args.draft_email_ids).await {
        return Envelope::err(command_name, account, e);
    }

    Envelope::ok(
        command_name,
        account,
        json!({"destroyed": args.draft_email_ids}),
        Meta::default(),
    )
}
