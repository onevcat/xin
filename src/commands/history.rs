use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};

use crate::backend::Backend;
use crate::cli::HistoryArgs;
use crate::error::XinErrorOut;
use crate::output::{Envelope, Meta};
use crate::schema;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub(crate) struct PageToken {
    #[serde(rename = "sinceState")]
    pub(crate) since_state: String,

    #[serde(rename = "maxChanges")]
    pub(crate) max_changes: usize,
}

pub(crate) fn encode_page_token(token: &PageToken) -> String {
    let bytes = serde_json::to_vec(token).expect("token json");
    URL_SAFE_NO_PAD.encode(bytes)
}

pub(crate) fn decode_page_token(s: &str) -> Result<PageToken, XinErrorOut> {
    let bytes = URL_SAFE_NO_PAD
        .decode(s)
        .map_err(|e| XinErrorOut::usage(format!("invalid page token: {e}")))?;
    serde_json::from_slice(&bytes)
        .map_err(|e| XinErrorOut::usage(format!("invalid page token json: {e}")))
}

pub async fn history(account: Option<String>, args: &HistoryArgs) -> Envelope<Value> {
    let command_name = "history";

    let backend = match Backend::connect(account.as_deref()).await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let max_changes_default = 100;

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
    //
    // Paging rule:
    // - If --page is provided, the token is the source of truth for sinceState/maxChanges.
    // - If the user explicitly also provides --since/--max, they MUST match the token.
    let (since_state, used_max) = match &args.page {
        Some(token) => {
            let t = match decode_page_token(token) {
                Ok(t) => t,
                Err(e) => return Envelope::err(command_name, account, e),
            };

            if let Some(max) = args.max {
                if max != t.max_changes {
                    return Envelope::err(
                        command_name,
                        account,
                        XinErrorOut::usage("page token does not match args".to_string()),
                    );
                }
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
        None => (
            args.since.clone().unwrap_or_default(),
            args.max.unwrap_or(max_changes_default),
        ),
    };

    if since_state.trim().is_empty() {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage("missing --since (or --page)".to_string()),
        );
    }

    let (mut resp, hydrated_created, hydrated_updated) = if args.hydrate {
        match backend
            .email_changes_hydrate(&since_state, Some(used_max))
            .await
        {
            Ok((r, created, updated)) => (r, Some(created), Some(updated)),
            Err(e) => return Envelope::err(command_name, account, e),
        }
    } else {
        match backend.email_changes(&since_state, Some(used_max)).await {
            Ok(r) => (r, None, None),
            Err(e) => return Envelope::err(command_name, account, e),
        }
    };

    let has_more = resp.has_more_changes();
    let new_state = resp.take_new_state();

    let created = resp.take_created();
    let updated = resp.take_updated();
    let destroyed = resp.take_destroyed();

    let mut meta = Meta::default();
    if has_more {
        // JMAP /changes pagination: continue with the `newState` returned by the
        // previous call.
        meta.next_page = Some(encode_page_token(&PageToken {
            since_state: new_state.clone(),
            max_changes: used_max,
        }));
    }

    let mut data = json!({
        "sinceState": since_state,
        "newState": new_state,
        "hasMoreChanges": has_more,
        "changes": {
            "created": created,
            "updated": updated,
            "destroyed": destroyed
        }
    });

    if let (Some(created_emails), Some(updated_emails)) = (hydrated_created, hydrated_updated) {
        data.as_object_mut().expect("data object").insert(
            "hydrated".to_string(),
            json!({
                "created": schema::email_summary_items(&created_emails),
                "updated": schema::email_summary_items(&updated_emails)
            }),
        );
    }

    Envelope::ok(command_name, account, data, meta)
}
