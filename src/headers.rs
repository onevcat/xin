use chrono::{DateTime, Utc};
use jmap_client::email;
use jmap_client::email::{Email, HeaderValue, Property};
use serde_json::Value;

/// Parse `--headers a,b,c` into a normalized, de-duplicated list of header keys.
///
/// Normalization rules (v0):
/// - trim whitespace
/// - lowercase
/// - collapse internal spaces
/// - keep hyphens as-is
pub fn parse_headers_arg(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();

    for part in s.split(',') {
        let k = normalize_header_key(part);
        if k.is_empty() {
            continue;
        }
        if out.iter().any(|x| x == &k) {
            continue;
        }
        out.push(k);
    }

    out
}

fn normalize_header_key(s: &str) -> String {
    s.trim()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
        .replace(' ', "")
}

fn canonical_header_name(key: &str) -> String {
    // A tiny handful of common special-cases, mostly for aesthetics.
    match key {
        "message-id" => "Message-ID".to_string(),
        "in-reply-to" => "In-Reply-To".to_string(),
        "reply-to" => "Reply-To".to_string(),
        "dkim-signature" => "DKIM-Signature".to_string(),
        "mime-version" => "MIME-Version".to_string(),
        "list-id" => "List-Id".to_string(),
        "list-unsubscribe" => "List-Unsubscribe".to_string(),
        "authentication-results" => "Authentication-Results".to_string(),
        "content-type" => "Content-Type".to_string(),
        "content-transfer-encoding" => "Content-Transfer-Encoding".to_string(),
        "content-disposition" => "Content-Disposition".to_string(),
        "content-id" => "Content-ID".to_string(),
        "return-path" => "Return-Path".to_string(),
        "received" => "Received".to_string(),
        _ => {
            // Best-effort title-case.
            key.split('-')
                .filter(|p| !p.is_empty())
                .map(|p| {
                    let mut cs = p.chars();
                    match cs.next() {
                        None => "".to_string(),
                        Some(first) => first.to_uppercase().collect::<String>() + cs.as_str(),
                    }
                })
                .collect::<Vec<_>>()
                .join("-")
        }
    }
}

fn is_repeatable_header(key: &str) -> bool {
    key == "received"
        || key == "authentication-results"
        || key == "dkim-signature"
        || key.starts_with("resent-")
}

fn builtin_property_for_header_key(key: &str) -> Option<Property> {
    match key {
        // These are already part of our default `metadata`/`full` property sets.
        "from" | "to" | "cc" | "bcc" | "subject" => None,

        // Built-ins that we only fetch when explicitly requested.
        "sender" => Some(Property::Sender),
        "reply-to" => Some(Property::ReplyTo),
        "date" => Some(Property::SentAt),
        "message-id" => Some(Property::MessageId),
        "in-reply-to" => Some(Property::InReplyTo),
        "references" => Some(Property::References),

        _ => None,
    }
}

/// Build the extra Email/get properties needed to satisfy `--headers`.
///
/// This returns *additive* properties to merge into the existing property list.
pub fn extra_email_properties_for_headers(requested: &[String]) -> Vec<Property> {
    let mut out: Vec<Property> = Vec::new();

    for key in requested {
        if let Some(p) = builtin_property_for_header_key(key) {
            if !out.contains(&p) {
                out.push(p);
            }
            continue;
        }

        // Custom header -> request computed header property.
        if is_builtin_header_key(key) {
            continue;
        }

        let name = canonical_header_name(key);
        let all = is_repeatable_header(key);
        let p = Property::Header(email::Header::as_text(name, all));
        if !out.contains(&p) {
            out.push(p);
        }
    }

    out
}

fn is_builtin_header_key(key: &str) -> bool {
    matches!(
        key,
        "from"
            | "to"
            | "cc"
            | "bcc"
            | "subject"
            | "sender"
            | "reply-to"
            | "date"
            | "message-id"
            | "in-reply-to"
            | "references"
    )
}

fn ts_rfc3339(ts: i64) -> Option<String> {
    DateTime::<Utc>::from_timestamp(ts, 0).map(|dt| dt.to_rfc3339())
}

fn header_value_to_json(v: &HeaderValue) -> Value {
    match v {
        HeaderValue::AsText(s) => Value::String(s.clone()),
        HeaderValue::AsTextAll(vs) => Value::Array(vs.iter().cloned().map(Value::String).collect()),
        HeaderValue::AsDate(dt) => Value::String(dt.to_rfc3339()),
        HeaderValue::AsDateAll(vs) => {
            Value::Array(vs.iter().map(|dt| Value::String(dt.to_rfc3339())).collect())
        }
        HeaderValue::AsAddresses(addrs) => serde_json::to_value(addrs).unwrap_or(Value::Null),
        HeaderValue::AsAddressesAll(groups) => {
            // `:all` for addresses yields Vec<Vec<EmailAddress>>.
            serde_json::to_value(groups).unwrap_or(Value::Null)
        }
        HeaderValue::AsGroupedAddresses(groups) => {
            serde_json::to_value(groups).unwrap_or(Value::Null)
        }
        HeaderValue::AsGroupedAddressesAll(groups) => {
            serde_json::to_value(groups).unwrap_or(Value::Null)
        }
        HeaderValue::AsTextListAll(list) => serde_json::to_value(list).unwrap_or(Value::Null),
    }
}

/// Extract a parsed headers dictionary according to the v0 `--headers` contract.
///
/// Output keys are normalized (lowercase).
/// Values are:
/// - `null` if the header is missing
/// - scalar for singleton headers
/// - array for repeatable headers (per RFC / common practice)
#[allow(dead_code)]
pub fn extract_headers_dict(email: &Email, requested: &[String]) -> serde_json::Map<String, Value> {
    extract_headers_dict_dual(email, None, requested)
}

/// Same as `extract_headers_dict`, but allows sourcing *custom* header:* values from a different
/// Email object.
///
/// This is useful for `--format raw`, where we may want the full raw Email (properties=null), but
/// still need a second Email/get call to retrieve computed header:* properties.
pub fn extract_headers_dict_dual(
    primary: &Email,
    custom: Option<&Email>,
    requested: &[String],
) -> serde_json::Map<String, Value> {
    let mut out = serde_json::Map::new();

    for key in requested {
        let v = match key.as_str() {
            // Address-ish (already parsed by jmap-client)
            "from" => json_opt(primary.from()),
            "to" => json_opt(primary.to()),
            "cc" => json_opt(primary.cc()),
            "bcc" => json_opt(primary.bcc()),
            "sender" => json_opt(primary.sender()),
            "reply-to" => json_opt(primary.reply_to()),

            // Scalars
            "subject" => json_opt(primary.subject()),
            "date" => primary
                .sent_at()
                .and_then(ts_rfc3339)
                .map(Value::String)
                .unwrap_or(Value::Null),

            // Threading / ids
            "message-id" => primary
                .message_id()
                .and_then(|ids| ids.first().cloned())
                .map(Value::String)
                .unwrap_or(Value::Null),
            "in-reply-to" => json_opt(primary.in_reply_to()),
            "references" => json_opt(primary.references()),

            // Custom headers: computed header:* properties.
            _ => {
                let src = custom.unwrap_or(primary);
                let name = canonical_header_name(key);
                let all = is_repeatable_header(key);
                let h = email::Header::as_text(name, all);
                match src.header(&h) {
                    Some(hv) => header_value_to_json(hv),
                    None => Value::Null,
                }
            }
        };

        out.insert(key.clone(), v);
    }

    out
}

fn json_opt<T: serde::Serialize>(v: Option<T>) -> Value {
    serde_json::to_value(v).unwrap_or(Value::Null)
}
