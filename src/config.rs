use std::fs;

use crate::error::XinErrorOut;
use jmap_client::client::Credentials;
use std::fmt;

#[derive(Clone)]
pub enum AuthConfig {
    Bearer(String),
    Basic { user: String, pass: String },
}

impl AuthConfig {
    pub fn credentials(&self) -> Credentials {
        match self {
            AuthConfig::Bearer(token) => Credentials::bearer(token.clone()),
            AuthConfig::Basic { user, pass } => Credentials::basic(user, pass),
        }
    }
}

impl fmt::Debug for AuthConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthConfig::Bearer(_) => f.debug_struct("AuthConfig::Bearer").finish(),
            AuthConfig::Basic { .. } => f.debug_struct("AuthConfig::Basic").finish(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Base URL that the JMAP client library will connect to (it will append `/.well-known/jmap`).
    pub base_url: String,

    pub auth: AuthConfig,

    /// Optional comma-separated redirect hosts allowlist (Fastmail may redirect session URL).
    pub follow_redirect_hosts: Vec<String>,
}

impl RuntimeConfig {
    pub fn credentials(&self) -> Credentials {
        self.auth.credentials()
    }

    pub fn from_env() -> Result<Self, XinErrorOut> {
        let base_url = if let Ok(base) = std::env::var("XIN_BASE_URL") {
            base.trim_end_matches('/').to_string()
        } else {
            let session_url = std::env::var("XIN_SESSION_URL").map_err(|_| {
                XinErrorOut::config(
                    "missing XIN_BASE_URL (or XIN_SESSION_URL) environment variable".to_string(),
                )
            })?;
            let u = url::Url::parse(&session_url)
                .map_err(|e| XinErrorOut::config(format!("invalid XIN_SESSION_URL: {e}")))?;
            format!(
                "{}://{}{}",
                u.scheme(),
                u.host_str().unwrap_or(""),
                u.port().map(|p| format!(":{p}")).unwrap_or_default()
            )
        };

        let token_env = std::env::var("XIN_TOKEN").ok();
        let token_file_env = std::env::var("XIN_TOKEN_FILE").ok();
        let basic_user_env = std::env::var("XIN_BASIC_USER").ok();
        let basic_pass_env = std::env::var("XIN_BASIC_PASS").ok();
        let basic_pass_file_env = std::env::var("XIN_BASIC_PASS_FILE").ok();

        if (token_env.is_some() || token_file_env.is_some())
            && (basic_user_env.is_some()
                || basic_pass_env.is_some()
                || basic_pass_file_env.is_some())
        {
            return Err(XinErrorOut::config(
                "both bearer (XIN_TOKEN/XIN_TOKEN_FILE) and basic (XIN_BASIC_USER/XIN_BASIC_PASS[_FILE]) credentials are set; choose one".to_string(),
            ));
        }

        let auth = if let Some(token) = token_env {
            AuthConfig::Bearer(token)
        } else if let Some(path) = token_file_env {
            let token = fs::read_to_string(path)
                .map_err(|e| XinErrorOut::config(format!("failed to read token file: {e}")))?;
            AuthConfig::Bearer(token.trim().to_string())
        } else if basic_user_env.is_some()
            || basic_pass_env.is_some()
            || basic_pass_file_env.is_some()
        {
            let user = basic_user_env.ok_or_else(|| {
                XinErrorOut::config("missing XIN_BASIC_USER environment variable".to_string())
            })?;
            let pass = match basic_pass_env {
                Some(v) => v,
                None => {
                    let path = basic_pass_file_env.ok_or_else(|| {
                        XinErrorOut::config(
                            "missing XIN_BASIC_PASS (or XIN_BASIC_PASS_FILE) environment variable"
                                .to_string(),
                        )
                    })?;
                    fs::read_to_string(path).map_err(|e| {
                        XinErrorOut::config(format!("failed to read basic pass file: {e}"))
                    })?
                }
            };
            AuthConfig::Basic {
                user,
                pass: pass.trim().to_string(),
            }
        } else {
            return Err(XinErrorOut::config(
                "missing XIN_TOKEN (or XIN_TOKEN_FILE) or XIN_BASIC_USER/XIN_BASIC_PASS environment variable".to_string(),
            ));
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
            auth,
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
        serde_json::from_str(value).map_err(|e| XinErrorOut::usage(format!("invalid json: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env<T>(vars: &[(&str, Option<&str>)], f: impl FnOnce() -> T) -> T {
        let _guard = ENV_LOCK.lock().unwrap();
        let mut saved = Vec::with_capacity(vars.len());

        for (key, value) in vars {
            saved.push((key.to_string(), std::env::var(key).ok()));
            match value {
                Some(v) => unsafe { std::env::set_var(key, v) },
                None => unsafe { std::env::remove_var(key) },
            }
        }

        let result = f();

        for (key, value) in saved {
            match value {
                Some(v) => unsafe { std::env::set_var(&key, v) },
                None => unsafe { std::env::remove_var(&key) },
            }
        }

        result
    }

    #[test]
    fn basic_auth_selected_from_env() {
        with_env(
            &[
                ("XIN_BASE_URL", Some("https://example.com")),
                ("XIN_BASIC_USER", Some("user@example.com")),
                ("XIN_BASIC_PASS", Some("secret")),
                ("XIN_BASIC_PASS_FILE", None),
                ("XIN_TOKEN", None),
                ("XIN_TOKEN_FILE", None),
            ],
            || {
                let cfg = RuntimeConfig::from_env().expect("config");
                match cfg.auth {
                    AuthConfig::Basic { user, pass } => {
                        assert_eq!(user, "user@example.com");
                        assert_eq!(pass, "secret");
                    }
                    _ => panic!("expected basic auth"),
                }
            },
        );
    }

    #[test]
    fn auth_conflict_returns_config_error() {
        with_env(
            &[
                ("XIN_BASE_URL", Some("https://example.com")),
                ("XIN_BASIC_USER", Some("user@example.com")),
                ("XIN_BASIC_PASS", Some("pass-123")),
                ("XIN_BASIC_PASS_FILE", None),
                ("XIN_TOKEN", Some("token-123")),
                ("XIN_TOKEN_FILE", None),
            ],
            || {
                let err = RuntimeConfig::from_env().expect_err("expected conflict");
                assert_eq!(err.kind, "xinConfigError");
                assert!(!err.message.contains("token-123"));
                assert!(!err.message.contains("pass-123"));
            },
        );
    }
}
