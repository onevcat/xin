use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

use crate::backend::{Backend, UploadedBlob};
use crate::cli::{
    DraftsCreateArgs, DraftsDeleteArgs, DraftsGetArgs, DraftsListArgs, DraftsSendArgs,
    DraftsUpdateArgs, IdentitiesGetArgs, SendArgs,
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

fn find_drafts_mailbox_id(
    mailboxes: &[jmap_client::mailbox::Mailbox],
) -> Result<String, XinErrorOut> {
    mailboxes
        .iter()
        .find(|m| m.role() == jmap_client::mailbox::Role::Drafts)
        .and_then(|m| m.id())
        .map(|id| id.to_string())
        .ok_or_else(|| XinErrorOut::config("drafts mailbox not found".to_string()))
}

fn resolve_identity(
    identities: &[jmap_client::identity::Identity],
    selector: Option<&str>,
) -> Result<(String, Option<String>, String), XinErrorOut> {
    let idt = match selector {
        None => identities.first().ok_or_else(|| {
            XinErrorOut::config("no identities available".to_string())
        })?,
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
        uploaded.push(
            backend
                .upload_blob(bytes, Some(content_type), name)
                .await?,
        );
    }

    Ok(uploaded)
}

pub async fn identities_list(account: Option<String>) -> Envelope<Value> {
    let backend = match Backend::connect().await {
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

    Envelope::ok("identities.list", account, json!({"identities": out}), Meta::default())
}

pub async fn identities_get(account: Option<String>, args: &IdentitiesGetArgs) -> Envelope<Value> {
    let backend = match Backend::connect().await {
        Ok(b) => b,
        Err(e) => return Envelope::err("identities.get", account, e),
    };

    let identities = match backend.list_identities().await {
        Ok(i) => i,
        Err(e) => return Envelope::err("identities.get", account, e),
    };

    let sel = args.id.as_str();
    let found = identities.iter().find(|i| {
        i.id().map(|id| id == sel).unwrap_or(false)
            || i.email().map(|e| e == sel).unwrap_or(false)
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
    let backend = match Backend::connect().await {
        Ok(b) => b,
        Err(e) => return Envelope::err("send", account, e),
    };

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

    let (identity_id, from_name, from_email) = match resolve_identity(
        &identities,
        args.identity.as_deref(),
    ) {
        Ok(v) => v,
        Err(e) => return Envelope::err("send", account, e),
    };

    let uploaded = match upload_attachments(&backend, &args.attach).await {
        Ok(v) => v,
        Err(e) => return Envelope::err("send", account, e),
    };

    let email = match backend
        .create_draft_email(
            &drafts_id,
            from_name,
            from_email,
            &args.to,
            &args.cc,
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

pub async fn drafts_list(account: Option<String>, args: &DraftsListArgs) -> Envelope<Value> {
    // Reuse READ search machinery for query+hydrate+page tokens.
    let backend = match Backend::connect().await {
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

    let backend = match Backend::connect().await {
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
        _ => match backend.get_email(&args.draft_email_id, requested_props).await {
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

    let backend = match Backend::connect().await {
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

    let (_identity_id, from_name, from_email) = match resolve_identity(
        &identities,
        args.identity.as_deref(),
    ) {
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

    let backend = match Backend::connect().await {
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
            XinErrorOut::usage(
                "--replace-attachments requires at least one --attach".to_string(),
            ),
        );
    }

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

    let wants_body_or_attachments = body_text.is_some()
        || body_html.is_some()
        || !args.attach.is_empty()
        || args.clear_attachments
        || args.replace_attachments;

    let mut existing_text: Option<String> = None;
    let mut existing_html: Option<String> = None;
    let mut existing_attachments: Vec<UploadedBlob> = Vec::new();

    if wants_body_or_attachments {
        let max_body_value_bytes = 1_048_576;
        let email = match backend
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

        let (b, _warnings) = crate::schema::extract_full_body(&email, max_body_value_bytes);
        existing_text = b
            .get("text")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        existing_html = b
            .get("html")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if let Some(parts) = email.attachments() {
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
    }

    let final_text: Option<String> = if body_text.is_some() {
        body_text
    } else {
        existing_text
    };

    let final_html: Option<String> = if body_html.is_some() {
        body_html
    } else {
        existing_html
    };

    let (attachments_for_update, uploaded_new) = if wants_body_or_attachments {
        if args.clear_attachments {
            (Some(Vec::<UploadedBlob>::new()), Vec::<UploadedBlob>::new())
        } else {
            let mut atts: Vec<UploadedBlob> = Vec::new();
            if !args.replace_attachments {
                atts.extend(existing_attachments);
            }

            let uploaded = match upload_attachments(&backend, &args.attach).await {
                Ok(v) => v,
                Err(e) => return Envelope::err(command_name, account, e),
            };
            atts.extend(uploaded.clone());

            (Some(atts), uploaded)
        }
    } else {
        (None, Vec::<UploadedBlob>::new())
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

        (name, Some(email))
    } else {
        (None, None)
    };

    if args.to.is_none()
        && args.cc.is_none()
        && args.bcc.is_none()
        && args.subject.is_none()
        && !wants_body_or_attachments
        && args.identity.is_none()
    {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage("no changes specified".to_string()),
        );
    }

    let attachments_slice: Option<&[UploadedBlob]> =
        attachments_for_update.as_ref().map(|v| v.as_slice());

    if let Err(e) = backend
        .update_draft_email(
            &args.draft_email_id,
            from_name,
            from_email,
            args.to.as_deref(),
            args.cc.as_deref(),
            args.bcc.as_deref(),
            args.subject.as_deref(),
            final_text.as_deref(),
            final_html.as_deref(),
            attachments_slice,
        )
        .await
    {
        return Envelope::err(command_name, account, e);
    }

    // Fetch threadId for output stability.
    let updated = match backend
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
                XinErrorOut::config("updated draft not found after update".to_string()),
            );
        }
        Err(e) => return Envelope::err(command_name, account, e),
    };

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
            "draft": { "emailId": args.draft_email_id, "threadId": updated.thread_id() },
            "uploaded": uploaded_out
        }),
        Meta::default(),
    )
}

pub async fn drafts_send(account: Option<String>, args: &DraftsSendArgs) -> Envelope<Value> {
    let command_name = "drafts.send";

    let backend = match Backend::connect().await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let identities = match backend.list_identities().await {
        Ok(i) => i,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let (identity_id, _from_name, _from_email) = match resolve_identity(
        &identities,
        args.identity.as_deref(),
    ) {
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

    let backend = match Backend::connect().await {
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
        json!({"deleted": args.draft_email_ids}),
        Meta::default(),
    )
}
