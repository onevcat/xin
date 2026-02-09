use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jmap_client::core::query::{Filter as CoreFilter, Operator};
use jmap_client::email;
use serde_json::{Value, json};

use crate::backend::Backend;
use crate::cli::{
    AttachmentArgs, GetArgs, GetFormat, MessagesSearchArgs, SearchArgs, ThreadAttachmentsArgs,
    ThreadGetArgs,
};
use crate::sugar;
use crate::config::read_json_arg;
use crate::error::XinErrorOut;
use crate::output::{Envelope, Meta};
use crate::schema;
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

fn parse_filter_value(v: &Value) -> Result<Option<CoreFilter<email::query::Filter>>, XinErrorOut> {
    let obj = match v.as_object() {
        Some(o) => o,
        None => {
            return Err(XinErrorOut::usage(
                "filter-json must be an object".to_string(),
            ));
        }
    };

    if obj.is_empty() {
        return Ok(None);
    }

    Ok(Some(parse_filter(v)?))
}

fn parse_filter(v: &Value) -> Result<CoreFilter<email::query::Filter>, XinErrorOut> {
    if let Some(op) = v.get("operator").and_then(|v| v.as_str()) {
        let op = match op {
            "AND" => Operator::And,
            "OR" => Operator::Or,
            "NOT" => Operator::Not,
            other => return Err(XinErrorOut::usage(format!("unsupported operator: {other}"))),
        };
        let conditions = v
            .get("conditions")
            .and_then(|v| v.as_array())
            .ok_or_else(|| XinErrorOut::usage("operator filter missing conditions[]".to_string()))?
            .iter()
            .map(parse_filter)
            .collect::<Result<Vec<_>, _>>()?;

        return Ok(CoreFilter::operator(op, conditions));
    }

    let obj = v
        .as_object()
        .ok_or_else(|| XinErrorOut::usage("filter condition must be an object".to_string()))?;

    let mut parts: Vec<CoreFilter<email::query::Filter>> = Vec::new();

    for (k, vv) in obj {
        let cond: email::query::Filter = match k.as_str() {
            "inMailbox" => email::query::Filter::InMailbox {
                value: vv
                    .as_str()
                    .ok_or_else(|| XinErrorOut::usage("inMailbox must be string".to_string()))?
                    .to_string(),
            },
            "hasAttachment" => email::query::Filter::HasAttachment {
                value: vv
                    .as_bool()
                    .ok_or_else(|| XinErrorOut::usage("hasAttachment must be bool".to_string()))?,
            },
            "from" => email::query::Filter::From {
                value: vv
                    .as_str()
                    .ok_or_else(|| XinErrorOut::usage("from must be string".to_string()))?
                    .to_string(),
            },
            "to" => email::query::Filter::To {
                value: vv
                    .as_str()
                    .ok_or_else(|| XinErrorOut::usage("to must be string".to_string()))?
                    .to_string(),
            },
            "cc" => email::query::Filter::Cc {
                value: vv
                    .as_str()
                    .ok_or_else(|| XinErrorOut::usage("cc must be string".to_string()))?
                    .to_string(),
            },
            "bcc" => email::query::Filter::Bcc {
                value: vv
                    .as_str()
                    .ok_or_else(|| XinErrorOut::usage("bcc must be string".to_string()))?
                    .to_string(),
            },
            "subject" => email::query::Filter::Subject {
                value: vv
                    .as_str()
                    .ok_or_else(|| XinErrorOut::usage("subject must be string".to_string()))?
                    .to_string(),
            },
            "text" => email::query::Filter::Text {
                value: vv
                    .as_str()
                    .ok_or_else(|| XinErrorOut::usage("text must be string".to_string()))?
                    .to_string(),
            },
            "body" => email::query::Filter::Body {
                value: vv
                    .as_str()
                    .ok_or_else(|| XinErrorOut::usage("body must be string".to_string()))?
                    .to_string(),
            },
            "hasKeyword" => email::query::Filter::HasKeyword {
                value: vv
                    .as_str()
                    .ok_or_else(|| XinErrorOut::usage("hasKeyword must be string".to_string()))?
                    .to_string(),
            },
            "notKeyword" => email::query::Filter::NotKeyword {
                value: vv
                    .as_str()
                    .ok_or_else(|| XinErrorOut::usage("notKeyword must be string".to_string()))?
                    .to_string(),
            },
            "after" => {
                let s = vv
                    .as_str()
                    .ok_or_else(|| XinErrorOut::usage("after must be RFC3339 string".to_string()))?;
                let dt = chrono::DateTime::parse_from_rfc3339(s)
                    .map_err(|e| XinErrorOut::usage(format!("invalid after date: {e}")))?
                    .with_timezone(&chrono::Utc);
                email::query::Filter::After { value: dt }
            }
            "before" => {
                let s = vv
                    .as_str()
                    .ok_or_else(|| XinErrorOut::usage("before must be RFC3339 string".to_string()))?;
                let dt = chrono::DateTime::parse_from_rfc3339(s)
                    .map_err(|e| XinErrorOut::usage(format!("invalid before date: {e}")))?
                    .with_timezone(&chrono::Utc);
                email::query::Filter::Before { value: dt }
            }
            "minSize" => email::query::Filter::MinSize {
                value: vv
                    .as_u64()
                    .ok_or_else(|| XinErrorOut::usage("minSize must be number".to_string()))?
                    as u32,
            },
            "maxSize" => email::query::Filter::MaxSize {
                value: vv
                    .as_u64()
                    .ok_or_else(|| XinErrorOut::usage("maxSize must be number".to_string()))?
                    as u32,
            },
            other => {
                return Err(XinErrorOut::usage(format!(
                    "unsupported filter-json key: {other}"
                )));
            }
        };

        parts.push(cond.into());
    }

    if parts.len() == 1 {
        Ok(parts.pop().unwrap())
    } else {
        Ok(CoreFilter::and(parts))
    }
}

pub async fn search(
    command_name: &str,
    account: Option<String>,
    args: &SearchArgs,
) -> Envelope<Value> {
    let backend = match Backend::connect().await {
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
            Some(q) if !q.trim().is_empty() => match sugar::compile_search_filter(q, &backend).await {
                Ok(v) => v,
                Err(e) => return Envelope::err(command_name, account, e),
            },
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

    let filter = match parse_filter_value(&filter_json) {
        Ok(f) => f,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let result = match backend
        .search(filter, position, limit, collapse_threads, is_ascending)
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

pub async fn get(args: &GetArgs) -> Envelope<Value> {
    let command_name = "get";
    let account = None;

    let backend = match Backend::connect().await {
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
            .get_email_full(&args.email_id, max_body_value_bytes, extra_header_props.clone())
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
            if let Some(email_obj) = data
                .get_mut("email")
                .and_then(|v| v.as_object_mut())
            {
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
            if let Some(email_obj) = data
                .get_mut("email")
                .and_then(|v| v.as_object_mut())
            {
                email_obj.insert("headers".to_string(), Value::Object(h));
            }
        }

        Envelope::ok(command_name, account, data, Meta::default())
    }
}

pub async fn thread_get(args: &ThreadGetArgs) -> Envelope<Value> {
    let command_name = "thread.get";
    let account = None;

    let backend = match Backend::connect().await {
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

pub async fn thread_attachments(args: &ThreadAttachmentsArgs) -> Envelope<Value> {
    let command_name = "thread.attachments";
    let account = None;

    let backend = match Backend::connect().await {
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

pub async fn attachment_download(args: &AttachmentArgs) -> Envelope<Value> {
    let command_name = "attachment";
    let account = None;

    let backend = match Backend::connect().await {
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
