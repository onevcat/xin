use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct XinErrorOut {
    pub kind: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jmap: Option<serde_json::Value>,
}

impl XinErrorOut {
    pub fn not_implemented(message: impl Into<String>) -> Self {
        Self {
            kind: "xinNotImplemented".to_string(),
            message: message.into(),
            http: None,
            jmap: None,
        }
    }

    pub fn usage(message: impl Into<String>) -> Self {
        Self {
            kind: "xinUsageError".to_string(),
            message: message.into(),
            http: None,
            jmap: None,
        }
    }
}
