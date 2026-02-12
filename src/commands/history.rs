use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde_json::{Value, json};

use crate::backend::Backend;
use crate::cli::HistoryArgs;
use crate::error::XinErrorOut;
use crate::output::{Envelope, Meta};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
struct PageToken {
    #[serde(rename = "sinceState")]
    since_state: String,

    #[serde(rename = "maxChanges")]
    max_changes: usize,
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

pub async fn history(account: Option<String>, args: &HistoryArgs) -> Envelope<Value> {
    let command_name = "history";

    let backend = match Backend::connect().await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let max_changes = args.max.unwrap_or(100);

    // Bootstrap: return current state and no changes.
    if args.since.is_none() && args.page.is_none() {
        let state = match backend.email_state().await {
            Ok(s) => s,
            Err(e) => return Envelope::err(command_name, account, e),
        };

        return Envelope::ok(
            command_name,
            account,
            json!({
                "sinceState": state,
                "newState": state,
                "hasMoreChanges": false,
                "changes": {"created": [], "updated": [], "destroyed": []}
            }),
            Meta::default(),
        );
    }

    // Normal: Email/changes.
    let (since_state, used_max) = match &args.page {
        Some(token) => match decode_page_token(token) {
            Ok(t) => {
                if t.max_changes != max_changes {
                    return Envelope::err(
                        command_name,
                        account,
                        XinErrorOut::usage("page token does not match args".to_string()),
                    );
                }
                if let Some(since) = &args.since {
                    if since != &t.since_state {
                        return Envelope::err(
                            command_name,
                            account,
                            XinErrorOut::usage("page token does not match args".to_string()),
                        );
                    }
                }
                (t.since_state, t.max_changes)
            }
            Err(e) => return Envelope::err(command_name, account, e),
        },
        None => (args.since.clone().unwrap_or_default(), max_changes),
    };

    if since_state.trim().is_empty() {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage("missing --since (or --page)".to_string()),
        );
    }

    let mut resp = match backend.email_changes(&since_state, Some(used_max)).await {
        Ok(r) => r,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let has_more = resp.has_more_changes();
    let new_state = resp.take_new_state();

    let created = resp.take_created();
    let updated = resp.take_updated();
    let destroyed = resp.take_destroyed();

    let mut meta = Meta::default();
    if has_more {
        meta.next_page = Some(encode_page_token(&PageToken {
            since_state: since_state.clone(),
            max_changes: used_max,
        }));
    }

    Envelope::ok(
        command_name,
        account,
        json!({
            "sinceState": since_state,
            "newState": new_state,
            "hasMoreChanges": has_more,
            "changes": {
                "created": created,
                "updated": updated,
                "destroyed": destroyed
            }
        }),
        meta,
    )
}
