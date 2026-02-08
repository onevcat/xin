use std::fs;

use crate::error::XinErrorOut;

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Base URL that the JMAP client library will connect to (it will append `/.well-known/jmap`).
    pub base_url: String,

    pub token: String,

    /// Optional comma-separated redirect hosts allowlist (Fastmail may redirect session URL).
    pub follow_redirect_hosts: Vec<String>,
}

impl RuntimeConfig {
    pub fn from_env() -> Result<Self, XinErrorOut> {
        let base_url = if let Ok(base) = std::env::var("XIN_BASE_URL") {
            base.trim_end_matches('/').to_string()
        } else {
            let session_url = std::env::var("XIN_SESSION_URL").map_err(|_| {
                XinErrorOut::usage(
                    "missing XIN_BASE_URL (or XIN_SESSION_URL) environment variable".to_string(),
                )
            })?;
            let u = url::Url::parse(&session_url)
                .map_err(|e| XinErrorOut::usage(format!("invalid XIN_SESSION_URL: {e}")))?;
            format!(
                "{}://{}{}",
                u.scheme(),
                u.host_str().unwrap_or(""),
                u.port().map(|p| format!(":{p}")).unwrap_or_default()
            )
        };

        let token = match std::env::var("XIN_TOKEN") {
            Ok(v) => v,
            Err(_) => {
                let path = std::env::var("XIN_TOKEN_FILE").map_err(|_| {
                    XinErrorOut::usage(
                        "missing XIN_TOKEN (or XIN_TOKEN_FILE) environment variable".to_string(),
                    )
                })?;
                fs::read_to_string(path)
                    .map_err(|e| XinErrorOut::usage(format!("failed to read token file: {e}")))?
                    .trim()
                    .to_string()
            }
        };

        let follow_redirect_hosts = std::env::var("XIN_TRUST_REDIRECT_HOSTS")
            .ok()
            .map(|s| {
                s.split(',')
                    .map(|x| x.trim().to_string())
                    .filter(|x| !x.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(Self {
            base_url,
            token,
            follow_redirect_hosts,
        })
    }
}

pub fn read_json_arg(value: &str) -> Result<serde_json::Value, XinErrorOut> {
    // Support @/path/to/file.json
    if let Some(path) = value.strip_prefix('@') {
        let text = fs::read_to_string(path)
            .map_err(|e| XinErrorOut::usage(format!("failed to read json file {path}: {e}")))?;
        serde_json::from_str(&text)
            .map_err(|e| XinErrorOut::usage(format!("invalid json in {path}: {e}")))
    } else {
        serde_json::from_str(value)
            .map_err(|e| XinErrorOut::usage(format!("invalid json: {e}")))
    }
}
