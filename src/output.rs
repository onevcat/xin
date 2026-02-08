use serde::Serialize;

use crate::error::XinErrorOut;

#[derive(Debug, Default, Clone, Serialize)]
pub struct Meta {
    #[serde(rename = "requestId", skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,

    #[serde(rename = "nextPage", skip_serializing_if = "Option::is_none")]
    pub next_page: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Envelope<T>
where
    T: Serialize,
{
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,

    pub ok: bool,
    pub command: String,
    pub account: Option<String>,
    pub data: Option<T>,
    pub error: Option<XinErrorOut>,
    pub meta: Meta,
}

impl<T> Envelope<T>
where
    T: Serialize,
{
    #[allow(dead_code)]
    pub fn ok(command: impl Into<String>, account: Option<String>, data: T, meta: Meta) -> Self {
        Self {
            schema_version: "0.1".to_string(),
            ok: true,
            command: command.into(),
            account,
            data: Some(data),
            error: None,
            meta,
        }
    }

    pub fn err(command: impl Into<String>, account: Option<String>, error: XinErrorOut) -> Self {
        Self {
            schema_version: "0.1".to_string(),
            ok: false,
            command: command.into(),
            account,
            data: None,
            error: Some(error),
            meta: Meta::default(),
        }
    }
}

pub fn print_envelope<T>(env: &Envelope<T>)
where
    T: Serialize,
{
    // Phase 1: always JSON.
    println!("{}", serde_json::to_string_pretty(env).expect("serialize"));
}
