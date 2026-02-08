use std::collections::HashMap;

use reqwest::header;
use serde::Deserialize;

use crate::error::XinErrorOut;

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Session {
    #[serde(rename = "apiUrl")]
    pub api_url: String,

    #[serde(rename = "downloadUrl")]
    pub download_url: String,

    #[serde(rename = "uploadUrl")]
    pub upload_url: String,

    #[serde(rename = "primaryAccounts")]
    pub primary_accounts: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct JmapClient {
    http: reqwest::Client,
    pub session: Session,
    pub mail_account_id: String,
}

impl JmapClient {
    pub async fn connect(session_url: &str, token: &str) -> Result<Self, XinErrorOut> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|_| XinErrorOut::usage("invalid bearer token".to_string()))?,
        );
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static("xin/0.1"),
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| XinErrorOut::usage(format!("failed to build http client: {e}")))?;

        let resp = http
            .get(session_url)
            .send()
            .await
            .map_err(|e| XinErrorOut::usage(format!("failed to fetch session: {e}")))?;

        if !resp.status().is_success() {
            return Err(XinErrorOut {
                kind: "httpError".to_string(),
                message: format!("session http error: {}", resp.status()),
                http: Some(serde_json::json!({"status": resp.status().as_u16()})),
                jmap: None,
            });
        }

        let session: Session = resp
            .json()
            .await
            .map_err(|e| XinErrorOut::usage(format!("invalid session json: {e}")))?;

        let mail_account_id = session
            .primary_accounts
            .get("urn:ietf:params:jmap:mail")
            .cloned()
            .ok_or_else(|| {
                XinErrorOut::usage("session missing primaryAccounts for jmap:mail".to_string())
            })?;

        Ok(Self {
            http,
            session,
            mail_account_id,
        })
    }

    pub async fn call(&self, request: serde_json::Value) -> Result<serde_json::Value, XinErrorOut> {
        let resp = self
            .http
            .post(&self.session.api_url)
            .json(&request)
            .send()
            .await
            .map_err(|e| XinErrorOut::usage(format!("failed to call apiUrl: {e}")))?;

        if !resp.status().is_success() {
            return Err(XinErrorOut {
                kind: "httpError".to_string(),
                message: format!("api http error: {}", resp.status()),
                http: Some(serde_json::json!({"status": resp.status().as_u16()})),
                jmap: None,
            });
        }

        resp.json()
            .await
            .map_err(|e| XinErrorOut::usage(format!("invalid api json: {e}")))
    }

    #[allow(dead_code)]
    pub fn expand_download_url(
        &self,
        blob_id: &str,
        name: &str,
        content_type: &str,
    ) -> String {
        let mut url = self.session.download_url.clone();
        url = url.replace("{accountId}", &self.mail_account_id);
        url = url.replace("{blobId}", blob_id);
        url = url.replace("{name}", &urlencoding::encode(name));
        url = url.replace("{type}", &urlencoding::encode(content_type));
        url
    }

    #[allow(dead_code)]
    pub async fn download_bytes(&self, blob_id: &str, name: &str, content_type: &str) -> Result<Vec<u8>, XinErrorOut> {
        let url = self.expand_download_url(blob_id, name, content_type);
        let resp = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| XinErrorOut::usage(format!("download failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(XinErrorOut {
                kind: "httpError".to_string(),
                message: format!("download http error: {}", resp.status()),
                http: Some(serde_json::json!({"status": resp.status().as_u16()})),
                jmap: None,
            });
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| XinErrorOut::usage(format!("download read failed: {e}")))?;
        Ok(bytes.to_vec())
    }
}
