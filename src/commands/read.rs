use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};

use crate::backend::Backend;
use crate::cli::{
    AttachmentArgs, GetArgs, GetFormat, MessagesSearchArgs, SearchArgs, ThreadAttachmentsArgs,
    ThreadGetArgs,
};
use crate::config::read_json_arg;
use crate::error::XinErrorOut;
use crate::output::{Envelope, Meta};
use crate::schema;
use crate::sugar;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
struct PageToken {
    position: i32,
    limit: usize,
    #[serde(rename = "collapseThreads")]
    collapse_threads: bool,
    #[serde(rename = "isAscending")]
    is_ascending: bool,
    filter: Value,
}

fn encode_page_token(token: &PageToken) -> String {
    let bytes = serde_json::to_vec(token).expect("token json");
    URL_SAFE_NO_PAD.encode(bytes)
}

fn decode_page_token(s: &str) -> Result<PageToken, XinErrorOut> {
    let bytes = URL_SAFE_NO_PAD
        .decode(s)
        .map_err(|e| XinErrorOut::usage(format!("invalid page token: {e}")))?;
    serde_json::from_slice(&bytes)
        .map_err(|e| XinErrorOut::usage(format!("invalid page token json: {e}")))
}

pub async fn search(
    command_name: &str,
    account: Option<String>,
    args: &SearchArgs,
) -> Envelope<Value> {
    let backend = match Backend::connect(account.as_deref()).await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let limit = args.max.unwrap_or(20);
    let collapse_threads = args.collapse_threads.unwrap_or(true);
    let is_ascending = args.oldest;

    let filter_json = match &args.filter_json {
        Some(s) => match read_json_arg(s) {
            Ok(v) => v,
            Err(e) => return Envelope::err(command_name, account, e),
        },
        None => match &args.query {
            Some(q) if !q.trim().is_empty() => {
                match sugar::compile_search_filter(q, &backend).await {
                    Ok(v) => v,
                    Err(e) => return Envelope::err(command_name, account, e),
                }
            }
            _ => json!({}),
        },
    };

    let (position, stable_filter_json) = match &args.page {
        Some(token) => match decode_page_token(token) {
            Ok(t) => {
                if t.limit != limit
                    || t.collapse_threads != collapse_threads
                    || t.is_ascending != is_ascending
                    || t.filter != filter_json
                {
                    return Envelope::err(
                        command_name,
                        account,
                        XinErrorOut::usage("page token does not match args".to_string()),
                    );
                }
                (t.position, t.filter)
            }
            Err(e) => return Envelope::err(command_name, account, e),
        },
        None => (0, filter_json.clone()),
    };

    let result = match backend
        .search_raw_filter_json(
            stable_filter_json.clone(),
            position,
            limit,
            collapse_threads,
            is_ascending,
        )
        .await
    {
        Ok(r) => r,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let items = schema::email_summary_items(&result.emails);

    let mut meta = Meta::default();
    let next_position = position + items.len() as i32;

    // Prefer `total` when available, but don't require it.
    // Some servers omit `total` even though they support paging via position/limit.
    let has_more = match result.query.total() {
        Some(total) => next_position < total as i32,
        None => items.len() == limit,
    };

    if has_more {
        meta.next_page = Some(encode_page_token(&PageToken {
            position: next_position,
            limit,
            collapse_threads,
            is_ascending,
            filter: stable_filter_json,
        }));
    }

    Envelope::ok(command_name, account, json!({"items": items}), meta)
}

pub async fn messages_search(
    account: Option<String>,
    args: &MessagesSearchArgs,
) -> Envelope<Value> {
    let search_args = SearchArgs {
        query: args.query.clone(),
        max: args.max,
        page: args.page.clone(),
        oldest: false,
        filter_json: args.filter_json.clone(),
        collapse_threads: Some(false),
        sort: crate::cli::SortKey::ReceivedAt,
    };

    search("messages.search", account, &search_args).await
}

// READ: get

pub async fn get(account: Option<String>, args: &GetArgs) -> Envelope<Value> {
    let command_name = "get";

    let backend = match Backend::connect(account.as_deref()).await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let requested_headers = args
        .headers
        .as_deref()
        .map(crate::headers::parse_headers_arg)
        .filter(|v| !v.is_empty());

    let extra_header_props = requested_headers
        .as_ref()
        .map(|h| crate::headers::extra_email_properties_for_headers(h))
        .unwrap_or_default();

    let props = match args.format {
        GetFormat::Metadata => {
            let mut p = vec![
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
            ];
            for ep in &extra_header_props {
                if !p.contains(ep) {
                    p.push(ep.clone());
                }
            }
            Some(p)
        }
        GetFormat::Raw => None,
        GetFormat::Full => None,
    };

    let max_body_value_bytes = args.max_body_bytes.unwrap_or(262_144);

    let email = match args.format {
        GetFormat::Full => match backend
            .get_email_full(
                &args.email_id,
                max_body_value_bytes,
                extra_header_props.clone(),
            )
            .await
        {
            Ok(Some(e)) => e,
            Ok(None) => {
                return Envelope::err(
                    command_name,
                    account,
                    XinErrorOut {
                        kind: "jmapMethodError".to_string(),
                        message: "email not found".to_string(),
                        http: None,
                        jmap: Some(json!({"type": "notFound"})),
                    },
                );
            }
            Err(e) => return Envelope::err(command_name, account, e),
        },
        _ => match backend.get_email(&args.email_id, props).await {
            Ok(Some(e)) => e,
            Ok(None) => {
                return Envelope::err(
                    command_name,
                    account,
                    XinErrorOut {
                        kind: "jmapMethodError".to_string(),
                        message: "email not found".to_string(),
                        http: None,
                        jmap: Some(json!({"type": "notFound"})),
                    },
                );
            }
            Err(e) => return Envelope::err(command_name, account, e),
        },
    };

    // For `--format raw`, we fetch properties=null (all standard properties), but computed
    // `header:*` properties are only returned when explicitly requested in `properties`.
    let custom_header_email = if args.format == GetFormat::Raw
        && requested_headers.is_some()
        && extra_header_props
            .iter()
            .any(|p| matches!(p, jmap_client::email::Property::Header(_)))
    {
        match backend
            .get_email(&args.email_id, Some(extra_header_props.clone()))
            .await
        {
            Ok(Some(e)) => Some(e),
            Ok(None) => None,
            Err(e) => return Envelope::err(command_name, account, e),
        }
    } else {
        None
    };

    let raw = match args.format {
        GetFormat::Raw => serde_json::to_value(&email).ok(),
        _ => None,
    };

    let headers_dict = requested_headers.as_ref().map(|h| {
        crate::headers::extract_headers_dict_dual(&email, custom_header_email.as_ref(), h)
    });

    if args.format == GetFormat::Full {
        let (mut data, warnings) = schema::get_email_full_data(&email, raw, max_body_value_bytes);

        if let Some(h) = headers_dict {
            if let Some(email_obj) = data.get_mut("email").and_then(|v| v.as_object_mut()) {
                email_obj.insert("headers".to_string(), Value::Object(h));
            }
        }

        let mut meta = Meta::default();
        if !warnings.is_empty() {
            meta.warnings = Some(warnings);
        }
        Envelope::ok(command_name, account, data, meta)
    } else {
        let mut data = schema::get_email_data(&email, raw);

        if let Some(h) = headers_dict {
            if let Some(email_obj) = data.get_mut("email").and_then(|v| v.as_object_mut()) {
                email_obj.insert("headers".to_string(), Value::Object(h));
            }
        }

        Envelope::ok(command_name, account, data, Meta::default())
    }
}

pub async fn thread_get(account: Option<String>, args: &ThreadGetArgs) -> Envelope<Value> {
    let command_name = "thread.get";

    let backend = match Backend::connect(account.as_deref()).await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let max_body_value_bytes = 262144;

    let result = match backend
        .thread_get(&args.thread_id, false, args.full, max_body_value_bytes)
        .await
    {
        Ok(Some(r)) => r,
        Ok(None) => {
            return Envelope::err(
                command_name,
                account,
                XinErrorOut {
                    kind: "jmapMethodError".to_string(),
                    message: "thread not found".to_string(),
                    http: None,
                    jmap: Some(json!({"type": "notFound"})),
                },
            );
        }
        Err(e) => return Envelope::err(command_name, account, e),
    };

    if args.full {
        let (data, warnings) = schema::thread_get_full_data(
            &result.thread_id,
            &result.email_ids,
            &result.emails,
            max_body_value_bytes,
        );
        let mut meta = Meta::default();
        if !warnings.is_empty() {
            meta.warnings = Some(warnings);
        }
        Envelope::ok(command_name, account, data, meta)
    } else {
        Envelope::ok(
            command_name,
            account,
            schema::thread_get_data(&result.thread_id, &result.email_ids, &result.emails),
            Meta::default(),
        )
    }
}

pub async fn thread_attachments(
    account: Option<String>,
    args: &ThreadAttachmentsArgs,
) -> Envelope<Value> {
    let command_name = "thread.attachments";

    let backend = match Backend::connect(account.as_deref()).await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let result = match backend
        .thread_get(&args.thread_id, true, false, 262144)
        .await
    {
        Ok(Some(r)) => r,
        Ok(None) => {
            return Envelope::err(
                command_name,
                account,
                XinErrorOut {
                    kind: "jmapMethodError".to_string(),
                    message: "thread not found".to_string(),
                    http: None,
                    jmap: Some(json!({"type": "notFound"})),
                },
            );
        }
        Err(e) => return Envelope::err(command_name, account, e),
    };

    Envelope::ok(
        command_name,
        account,
        schema::thread_attachments_data(&result.thread_id, &result.emails),
        Meta::default(),
    )
}

pub async fn attachment_download(account: Option<String>, args: &AttachmentArgs) -> Envelope<Value> {
    let command_name = "attachment";

    let backend = match Backend::connect(account.as_deref()).await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let bytes = match backend.download_blob(&args.blob_id).await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let out_path: PathBuf = if let Some(out) = &args.out {
        out.into()
    } else if let Some(name) = &args.name {
        name.into()
    } else {
        args.blob_id.clone().into()
    };

    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = fs::create_dir_all(parent) {
                return Envelope::err(
                    command_name,
                    account,
                    XinErrorOut::usage(format!("failed to create output dir: {e}")),
                );
            }
        }
    }

    if let Err(e) = fs::write(&out_path, &bytes) {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage(format!("failed to write output file: {e}")),
        );
    }

    Envelope::ok(
        command_name,
        account,
        json!({
            "emailId": args.email_id,
            "blobId": args.blob_id,
            "out": out_path.to_string_lossy(),
            "bytes": bytes.len(),
        }),
        Meta::default(),
    )
}
