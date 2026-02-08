use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use jmap_client::core::query::{Filter as CoreFilter, Operator};
use jmap_client::email;
use serde_json::{json, Value};

use crate::backend::Backend;
use crate::cli::{GetArgs, GetFormat, MessagesSearchArgs, SearchArgs};
use crate::config::read_json_arg;
use crate::error::XinErrorOut;
use crate::output::{Envelope, Meta};
use crate::schema;

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
            ))
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
            other => {
                return Err(XinErrorOut::usage(format!(
                    "unsupported operator: {other}"
                )))
            }
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
            other => {
                return Err(XinErrorOut::usage(format!(
                    "unsupported filter-json key: {other}"
                )))
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

pub async fn search(command_name: &str, account: Option<String>, args: &SearchArgs) -> Envelope<Value> {
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
        None => json!({}),
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
    if let Some(total) = result.query.total() {
        let next_position = position + items.len() as i32;
        if next_position < total as i32 {
            meta.next_page = Some(encode_page_token(&PageToken {
                position: next_position,
                limit,
                collapse_threads,
                is_ascending,
                filter: stable_filter_json,
            }));
        }
    }

    Envelope::ok(command_name, account, json!({"items": items}), meta)
}

pub async fn messages_search(account: Option<String>, args: &MessagesSearchArgs) -> Envelope<Value> {
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

    let props = match args.format {
        GetFormat::Metadata => Some(vec![
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
        GetFormat::Raw => None,
        GetFormat::Full => {
            return Envelope::err(
                command_name,
                account,
                XinErrorOut::not_implemented("get --format full not implemented yet"),
            );
        }
    };

    let email = match backend.get_email(&args.email_id, props).await {
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
            )
        }
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let raw = match args.format {
        GetFormat::Raw => serde_json::to_value(&email).ok(),
        _ => None,
    };

    Envelope::ok(command_name, account, schema::get_email_data(&email, raw), Meta::default())
}

pub async fn thread_get(_args: &crate::cli::ThreadGetArgs) -> Envelope<Value> {
    Envelope::err(
        "thread.get",
        None,
        XinErrorOut::not_implemented("thread get not implemented yet"),
    )
}

pub async fn thread_attachments(_args: &crate::cli::ThreadAttachmentsArgs) -> Envelope<Value> {
    Envelope::err(
        "thread.attachments",
        None,
        XinErrorOut::not_implemented("thread attachments not implemented yet"),
    )
}

pub async fn attachment_download(_args: &crate::cli::AttachmentArgs) -> Envelope<Value> {
    Envelope::err(
        "attachment",
        None,
        XinErrorOut::not_implemented("attachment download not implemented yet"),
    )
}
