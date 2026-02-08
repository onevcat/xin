use crate::config::RuntimeConfig;
use crate::error::XinErrorOut;

pub struct XinJmap {
    client: jmap_client::client::Client,
}

impl XinJmap {
    pub async fn connect(cfg: &RuntimeConfig) -> Result<Self, XinErrorOut> {
        // Redirect policy: trust base host by default; allow extra via env.
        let mut trusted_hosts = Vec::new();
        if let Ok(u) = url::Url::parse(&cfg.base_url) {
            if let Some(h) = u.host_str() {
                trusted_hosts.push(h.to_string());
            }
        }
        trusted_hosts.extend(cfg.follow_redirect_hosts.iter().cloned());

        let client = jmap_client::client::Client::new()
            .credentials(cfg.token.clone())
            .follow_redirects(trusted_hosts)
            .connect(&cfg.base_url)
            .await
            .map_err(|e| XinErrorOut {
                kind: "httpError".to_string(),
                message: format!("connect failed: {e}"),
                http: None,
                jmap: None,
            })?;

        Ok(Self { client })
    }

    pub fn client(&self) -> &jmap_client::client::Client {
        &self.client
    }

    #[allow(dead_code)]
    pub fn mail_account_id(&self) -> String {
        self.client.default_account_id().to_string()
    }
}
