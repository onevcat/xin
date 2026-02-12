use chrono::{NaiveDate, TimeZone, Utc};
use serde_json::{Value, json};

use crate::backend::Backend;
use crate::error::XinErrorOut;

/// Compile xin's v0 sugar DSL into a JMAP Email/query filter JSON object.
///
/// Notes (v0):
/// - AND is implicit by whitespace.
/// - OR is only supported via `or:(a | b | ...)`.
/// - NOT is supported via `-term` prefix.
/// - Parentheses grouping is NOT supported.
/// - If a token has no `key:` prefix, it is treated as `text:<token>`.
///
/// Mailbox resolution:
/// - `in:<mailbox>` resolves by id, role, then (case-sensitive) name, then case-insensitive name.
/// - Aliases: `spam` -> role `junk`, `bin` -> role `trash`.
pub async fn compile_search_filter(query: &str, backend: &Backend) -> Result<Value, XinErrorOut> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(json!({}));
    }

    let tokens = lex_tokens(query)?;

    // Only fetch mailboxes if we see any in:<...> term.
    let needs_mailboxes = tokens.iter().any(|t| match t {
        Token::Term { key: Some(k), .. } => k == "in",
        Token::OrGroup(terms) => terms
            .iter()
            .any(|tt| matches!(tt.key.as_deref(), Some("in"))),
        _ => false,
    });

    let mailboxes = if needs_mailboxes {
        Some(backend.list_mailboxes().await?)
    } else {
        None
    };

    let mut compiled: Vec<Value> = Vec::new();

    for t in tokens {
        match t {
            Token::Term {
                negated,
                key,
                value,
            } => {
                let cond = compile_one_term(&key, &value, mailboxes.as_deref())?;
                compiled.push(if negated { not(cond) } else { cond });
            }
            Token::OrGroup(terms) => {
                let mut ors: Vec<Value> = Vec::new();
                for tt in terms {
                    let cond = compile_one_term(&tt.key, &tt.value, mailboxes.as_deref())?;
                    ors.push(if tt.negated { not(cond) } else { cond });
                }
                compiled.push(op("OR", ors));
            }
        }
    }

    Ok(and_all(compiled))
}

#[derive(Debug, Clone)]
enum Token {
    Term {
        negated: bool,
        key: Option<String>,
        value: String,
    },
    OrGroup(Vec<TermToken>),
}

#[derive(Debug, Clone)]
struct TermToken {
    negated: bool,
    key: Option<String>,
    value: String,
}

fn lex_tokens(input: &str) -> Result<Vec<Token>, XinErrorOut> {
    // Split on whitespace, but keep quoted segments intact.
    let raw = split_ws_quoted(input)?;

    let mut out: Vec<Token> = Vec::new();

    for tok in raw {
        if tok.starts_with("or:(") {
            // v0: or:(a | b | c)
            if !tok.ends_with(')') {
                return Err(XinErrorOut::usage(
                    "or:(...) group must be a single token; quote the full query".to_string(),
                ));
            }
            let inner = tok
                .strip_prefix("or:(")
                .and_then(|s| s.strip_suffix(')'))
                .unwrap_or("");

            if inner.contains("or:(") {
                return Err(XinErrorOut::usage(
                    "nested or:(...) is not supported in v0; use `--filter-json` for nested boolean logic".to_string(),
                ));
            }

            let parts = split_or_terms(inner)?;
            let mut terms: Vec<TermToken> = Vec::new();
            for p in parts {
                let (negated, key, value) = parse_simple_term(&p)?;
                terms.push(TermToken {
                    negated,
                    key,
                    value,
                });
            }
            if terms.is_empty() {
                return Err(XinErrorOut::usage("or:(...) group is empty".to_string()));
            }
            out.push(Token::OrGroup(terms));
            continue;
        }

        let (negated, key, value) = parse_simple_term(&tok)?;
        out.push(Token::Term {
            negated,
            key,
            value,
        });
    }

    Ok(out)
}

fn parse_simple_term(token: &str) -> Result<(bool, Option<String>, String), XinErrorOut> {
    let mut s = token.trim();
    if s.is_empty() {
        return Err(XinErrorOut::usage("empty token".to_string()));
    }

    let mut negated = false;
    if let Some(rest) = s.strip_prefix('-') {
        negated = true;
        s = rest;
    }

    // No parentheses grouping in v0.
    if token.trim_start().starts_with("-(") {
        return Err(XinErrorOut::usage(
            "group negation `-(...)` is not supported in v0; negate individual terms (e.g. `-from:alice -subject:foo`) or use `--filter-json` (inline JSON or @file). Example: --filter-json '{\"operator\":\"NOT\",\"conditions\":[{\"from\":\"alice\"}]}'".to_string(),
        ));
    }

    if s.starts_with('(') || s.ends_with(')') {
        return Err(XinErrorOut::usage(
            "parentheses grouping is not supported in v0; use `or:(a|b|...)`, `-term`, or `--filter-json` (inline JSON or @file) for complex filters. Example: --filter-json '{\"operator\":\"AND\",\"conditions\":[{\"from\":\"alice\"},{\"subject\":\"foo\"}]}'".to_string(),
        ));
    }

    if let Some((k, v)) = s.split_once(':') {
        let key = k.trim().to_lowercase();
        let value = unquote(v.trim());
        Ok((negated, Some(key), value))
    } else {
        // Bare term: map to text search.
        Ok((negated, Some("text".to_string()), unquote(s)))
    }
}

fn unquote(s: &str) -> String {
    if let Some(inner) = s.strip_prefix('"').and_then(|x| x.strip_suffix('"')) {
        inner.to_string()
    } else {
        s.to_string()
    }
}

fn split_ws_quoted(input: &str) -> Result<Vec<String>, XinErrorOut> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;

    for ch in input.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                cur.push(ch);
            }
            c if c.is_whitespace() && !in_quotes => {
                if !cur.is_empty() {
                    out.push(cur);
                    cur = String::new();
                }
            }
            _ => cur.push(ch),
        }
    }

    if in_quotes {
        return Err(XinErrorOut::usage("unterminated quote".to_string()));
    }

    if !cur.is_empty() {
        out.push(cur);
    }

    Ok(out)
}

fn split_or_terms(inner: &str) -> Result<Vec<String>, XinErrorOut> {
    // Split on `|`, but allow quoted strings.
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;

    for ch in inner.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                cur.push(ch);
            }
            '|' if !in_quotes => {
                let s = cur.trim();
                if !s.is_empty() {
                    out.push(s.to_string());
                }
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }

    if in_quotes {
        return Err(XinErrorOut::usage(
            "unterminated quote in or:(...)".to_string(),
        ));
    }

    let s = cur.trim();
    if !s.is_empty() {
        out.push(s.to_string());
    }

    Ok(out)
}

fn compile_one_term(
    key: &Option<String>,
    value: &str,
    mailboxes: Option<&[jmap_client::mailbox::Mailbox]>,
) -> Result<Value, XinErrorOut> {
    let key = key.as_deref().unwrap_or("text");

    match key {
        // Addressing
        "from" | "to" | "cc" | "bcc" => Ok(json!({ key: value })),

        // Content
        "subject" | "text" | "body" => Ok(json!({ key: value })),

        // Mailbox
        "in" => {
            let Some(mbxs) = mailboxes else {
                return Err(XinErrorOut::usage(
                    "in:<mailbox> requires mailbox listing (internal error)".to_string(),
                ));
            };
            let id = resolve_mailbox_id(value, mbxs)
                .ok_or_else(|| XinErrorOut::usage(format!("unknown mailbox: {value}")))?;
            Ok(json!({"inMailbox": id }))
        }

        // Attachments
        "has" if value == "attachment" => Ok(json!({"hasAttachment": true})),
        "hasattachment" => Ok(json!({"hasAttachment": parse_bool(value, "hasAttachment")?})),

        // Keywords/state
        "seen" => {
            let b = parse_bool(value, "seen")?;
            if b {
                Ok(json!({"hasKeyword": "$seen"}))
            } else {
                Ok(json!({"notKeyword": "$seen"}))
            }
        }
        "flagged" => {
            let b = parse_bool(value, "flagged")?;
            if b {
                Ok(json!({"hasKeyword": "$flagged"}))
            } else {
                Ok(json!({"notKeyword": "$flagged"}))
            }
        }

        // Time (receivedAt)
        "after" => Ok(json!({"after": parse_date(value, "after")?})),
        "before" => Ok(json!({"before": parse_date(value, "before")?})),

        other => Err(XinErrorOut::usage(format!("unsupported term: {other}"))),
    }
}

fn parse_bool(value: &str, label: &str) -> Result<bool, XinErrorOut> {
    match value.to_lowercase().as_str() {
        "true" | "1" | "yes" => Ok(true),
        "false" | "0" | "no" => Ok(false),
        _ => Err(XinErrorOut::usage(format!("{label} must be true|false"))),
    }
}

fn parse_date(value: &str, label: &str) -> Result<String, XinErrorOut> {
    // v0: accept YYYY-MM-DD (interpreted as 00:00:00Z) or RFC3339.
    if let Ok(d) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        let dt = Utc.from_utc_datetime(
            &d.and_hms_opt(0, 0, 0)
                .ok_or_else(|| XinErrorOut::usage(format!("invalid date for {label}")))?,
        );
        return Ok(dt.to_rfc3339());
    }

    chrono::DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc).to_rfc3339())
        .map_err(|e| XinErrorOut::usage(format!("invalid {label} date: {e}")))
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

fn op(kind: &str, conditions: Vec<Value>) -> Value {
    json!({"operator": kind, "conditions": conditions})
}

fn not(cond: Value) -> Value {
    op("NOT", vec![cond])
}

fn and_all(mut conditions: Vec<Value>) -> Value {
    if conditions.is_empty() {
        json!({})
    } else if conditions.len() == 1 {
        conditions.pop().unwrap()
    } else {
        op("AND", conditions)
    }
}
