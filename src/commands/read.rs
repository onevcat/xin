use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde_json::{json, Value};

use crate::cli::{AttachmentArgs, GetArgs, MessagesSearchArgs, SearchArgs, ThreadAttachmentsArgs, ThreadGetArgs};
use crate::config::{read_json_arg, RuntimeConfig};
use crate::error::XinErrorOut;
use crate::jmap::JmapClient;
use crate::output::{Envelope, Meta};

fn method_response<'a>(resp: &'a Value, name: &str, tag: &str) -> Option<&'a Value> {
    let arr = resp.get("methodResponses")?.as_array()?;
    for item in arr {
        let parts = item.as_array()?;
        if parts.len() != 3 {
            continue;
        }
        if parts[0].as_str()? == name && parts[2].as_str()? == tag {
            return Some(&parts[1]);
        }
    }
    None
}

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

fn email_to_item(e: &Value) -> Value {
    let keywords = e.get("keywords").cloned().unwrap_or_else(|| json!({}));
    let unread = !keywords
        .as_object()
        .and_then(|m| m.get("$seen"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    json!({
        "threadId": e.get("threadId"),
        "emailId": e.get("id"),
        "receivedAt": e.get("receivedAt"),
        "subject": e.get("subject"),
        "from": e.get("from"),
        "to": e.get("to"),
        "snippet": e.get("preview"),
        "hasAttachment": e.get("hasAttachment").unwrap_or(&json!(false)),
        "mailboxIds": e.get("mailboxIds").cloned().unwrap_or_else(|| json!({})),
        "keywords": keywords,
        "unread": unread
    })
}

async fn connect() -> Result<JmapClient, XinErrorOut> {
    let cfg = RuntimeConfig::from_env()?;
    JmapClient::connect(&cfg.session_url, &cfg.token).await
}

pub async fn search(command_name: &str, account: Option<String>, args: &SearchArgs) -> Envelope<Value> {

    let client = match connect().await {
        Ok(c) => c,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let limit = args.max.unwrap_or(20);
    let collapse_threads = args.collapse_threads.unwrap_or(true);
    let is_ascending = args.oldest;

    let filter = match &args.filter_json {
        Some(s) => match read_json_arg(s) {
            Ok(v) => v,
            Err(e) => return Envelope::err(command_name, account, e),
        },
        None => json!({}),
    };

    let (position, next_meta_filter) = match &args.page {
        Some(token) => match decode_page_token(token) {
            Ok(t) => {
                if t.limit != limit
                    || t.collapse_threads != collapse_threads
                    || t.is_ascending != is_ascending
                    || t.filter != filter
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
        None => (0, filter.clone()),
    };

    let req = json!({
        "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
        "methodCalls": [
            ["Email/query", {
                "accountId": client.mail_account_id,
                "filter": filter,
                "sort": [{"property": "receivedAt", "isAscending": is_ascending}],
                "collapseThreads": collapse_threads,
                "position": position,
                "limit": limit
            }, "q1"],
            ["Email/get", {
                "accountId": client.mail_account_id,
                "ids": "#q1/ids",
                "properties": ["id","threadId","receivedAt","subject","from","to","preview","hasAttachment","mailboxIds","keywords"]
            }, "g1"]
        ]
    });

    let resp = match client.call(req).await {
        Ok(v) => v,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let q = method_response(&resp, "Email/query", "q1").cloned().unwrap_or_else(|| json!({}));
    let g = method_response(&resp, "Email/get", "g1").cloned().unwrap_or_else(|| json!({}));

    let list = g
        .get("list")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let items: Vec<Value> = list.iter().map(email_to_item).collect();

    let mut meta = Meta::default();
    let returned = items.len() as i32;
    let total = q.get("total").and_then(|v| v.as_u64()).unwrap_or(0) as i32;

    if total > 0 && position + returned < total {
        let token = PageToken {
            position: position + returned,
            limit,
            collapse_threads,
            is_ascending,
            filter: next_meta_filter,
        };
        meta.next_page = Some(encode_page_token(&token));
    }

    Envelope::ok(command_name, account, json!({"items": items}), meta)
}

pub async fn messages_search(account: Option<String>, args: &MessagesSearchArgs) -> Envelope<Value> {
    // v0: same as search, but collapseThreads=false.
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

pub async fn get(_args: &GetArgs) -> Envelope<Value> {
    Envelope::err("get", None, XinErrorOut::not_implemented("get not implemented yet"))
}

pub async fn thread_get(_args: &ThreadGetArgs) -> Envelope<Value> {
    Envelope::err(
        "thread.get",
        None,
        XinErrorOut::not_implemented("thread get not implemented yet"),
    )
}

pub async fn thread_attachments(_args: &ThreadAttachmentsArgs) -> Envelope<Value> {
    Envelope::err(
        "thread.attachments",
        None,
        XinErrorOut::not_implemented("thread attachments not implemented yet"),
    )
}

pub async fn attachment_download(_args: &AttachmentArgs) -> Envelope<Value> {
    Envelope::err(
        "attachment",
        None,
        XinErrorOut::not_implemented("attachment download not implemented yet"),
    )
}
