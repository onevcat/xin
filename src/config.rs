use std::fs;

use crate::error::XinErrorOut;

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub session_url: String,
    pub token: String,
}

impl RuntimeConfig {
    pub fn from_env() -> Result<Self, XinErrorOut> {
        let session_url = std::env::var("XIN_SESSION_URL")
            .or_else(|_| {
                std::env::var("XIN_BASE_URL")
                    .map(|base| format!("{}/.well-known/jmap", base.trim_end_matches('/')))
            })
            .map_err(|_| {
                XinErrorOut::usage(
                    "missing XIN_SESSION_URL (or XIN_BASE_URL) environment variable".to_string(),
                )
            })?;

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

        Ok(Self { session_url, token })
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
