use std::fs;

use crate::error::XinErrorOut;
use jmap_client::client::Credentials;
use std::fmt;

use crate::app_config;

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

    #[allow(dead_code)]
    pub fn from_env() -> Result<Self, XinErrorOut> {
        resolve_runtime_config(None).map(|r| r.config)
    }

}

#[derive(Debug, Clone)]
pub struct ResolvedRuntimeConfig {
    pub config: RuntimeConfig,
    pub account: Option<String>,
}

fn parse_origin_from_session_url(session_url: &str) -> Result<String, XinErrorOut> {
    let u = url::Url::parse(session_url)
        .map_err(|e| XinErrorOut::config(format!("invalid session url: {e}")))?;
    Ok(format!(
        "{}://{}{}",
        u.scheme(),
        u.host_str().unwrap_or(""),
        u.port().map(|p| format!(":{p}")).unwrap_or_default()
    ))
}

fn parse_follow_redirect_hosts_from_env() -> Vec<String> {
    std::env::var("XIN_TRUST_REDIRECT_HOSTS")
        .ok()
        .map(|s| {
            s.split(',')
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn parse_auth_from_global_env() -> Result<Option<AuthConfig>, XinErrorOut> {
    let token_env = std::env::var("XIN_TOKEN").ok();
    let token_file_env = std::env::var("XIN_TOKEN_FILE").ok();
    let basic_user_env = std::env::var("XIN_BASIC_USER").ok();
    let basic_pass_env = std::env::var("XIN_BASIC_PASS").ok();
    let basic_pass_file_env = std::env::var("XIN_BASIC_PASS_FILE").ok();

    let any_bearer = token_env.is_some() || token_file_env.is_some();
    let any_basic = basic_user_env.is_some() || basic_pass_env.is_some() || basic_pass_file_env.is_some();

    if any_bearer && any_basic {
        return Err(XinErrorOut::config(
            "both bearer (XIN_TOKEN/XIN_TOKEN_FILE) and basic (XIN_BASIC_USER/XIN_BASIC_PASS[_FILE]) credentials are set; choose one".to_string(),
        ));
    }

    if any_bearer {
        if let Some(token) = token_env {
            return Ok(Some(AuthConfig::Bearer(token)));
        }
        if let Some(path) = token_file_env {
            let token = fs::read_to_string(path)
                .map_err(|e| XinErrorOut::config(format!("failed to read token file: {e}")))?;
            return Ok(Some(AuthConfig::Bearer(token.trim().to_string())));
        }
    }

    if any_basic {
        let user = basic_user_env.ok_or_else(|| {
            XinErrorOut::config("missing XIN_BASIC_USER environment variable".to_string())
        })?;
        let pass = match basic_pass_env {
            Some(v) => v,
            None => {
                let path = basic_pass_file_env.ok_or_else(|| {
                    XinErrorOut::config(
                        "missing XIN_BASIC_PASS (or XIN_BASIC_PASS_FILE) environment variable".to_string(),
                    )
                })?;
                fs::read_to_string(path)
                    .map_err(|e| XinErrorOut::config(format!("failed to read basic pass file: {e}")))?
            }
        };
        return Ok(Some(AuthConfig::Basic {
            user,
            pass: pass.trim().to_string(),
        }));
    }

    Ok(None)
}

fn read_text_file_trimmed(path: &str, what: &str) -> Result<String, XinErrorOut> {
    let text = fs::read_to_string(path)
        .map_err(|e| XinErrorOut::config(format!("failed to read {what} file: {e}")))?;
    Ok(text.trim().to_string())
}

fn resolve_account_from_config(
    cfg: &app_config::AppConfig,
    selected: Option<&str>,
) -> Result<String, XinErrorOut> {
    if let Some(a) = selected {
        return Ok(a.to_string());
    }
    if let Some(a) = &cfg.defaults.account {
        return Ok(a.clone());
    }
    if cfg.accounts.len() == 1 {
        return Ok(cfg.accounts.keys().next().unwrap().to_string());
    }
    Err(XinErrorOut::config(
        "missing --account and no default account is set; run `xin config set-default <name>`".to_string(),
    ))
}

fn resolve_auth_from_account(acct: &app_config::AccountConfig) -> Result<AuthConfig, XinErrorOut> {
    use app_config::AuthConfigFile;
    match &acct.auth {
        AuthConfigFile::Bearer {
            token,
            token_env,
            token_file,
        } => {
            if let Some(t) = token.as_ref().filter(|s| !s.trim().is_empty()) {
                return Ok(AuthConfig::Bearer(t.trim().to_string()));
            }
            if let Some(env_key) = token_env {
                if let Ok(v) = std::env::var(env_key) {
                    return Ok(AuthConfig::Bearer(v.trim().to_string()));
                }
            }
            if let Some(path) = token_file {
                let p = app_config::expand_user_path(path)?;
                let t = read_text_file_trimmed(&p.to_string_lossy(), "token")?;
                return Ok(AuthConfig::Bearer(t));
            }
            Err(XinErrorOut::config(
                "missing bearer token; set XIN_TOKEN (or run `xin auth set-token <TOKEN>`)".to_string(),
            ))
        }
        AuthConfigFile::Basic {
            user,
            pass,
            pass_env,
            pass_file,
        } => {
            let u = user
                .as_ref()
                .filter(|s| !s.trim().is_empty())
                .cloned()
                .or_else(|| std::env::var("XIN_BASIC_USER").ok())
                .ok_or_else(|| XinErrorOut::config("missing basic user".to_string()))?;

            if let Some(p) = pass.as_ref().filter(|s| !s.trim().is_empty()) {
                return Ok(AuthConfig::Basic {
                    user: u,
                    pass: p.trim().to_string(),
                });
            }

            if let Some(env_key) = pass_env {
                if let Ok(v) = std::env::var(env_key) {
                    return Ok(AuthConfig::Basic {
                        user: u,
                        pass: v.trim().to_string(),
                    });
                }
            }

            if let Some(path) = pass_file {
                let p = app_config::expand_user_path(path)?;
                let v = read_text_file_trimmed(&p.to_string_lossy(), "basic pass")?;
                return Ok(AuthConfig::Basic { user: u, pass: v });
            }

            Err(XinErrorOut::config(
                "missing basic password; set XIN_BASIC_PASS or configure passFile".to_string(),
            ))
        }
    }
}

pub fn resolve_runtime_config(selected_account: Option<&str>) -> Result<ResolvedRuntimeConfig, XinErrorOut> {
    // Field-by-field precedence: CLI-selected account -> env -> config.

    let env_base_url = std::env::var("XIN_BASE_URL").ok().map(|s| s.trim_end_matches('/').to_string());
    let env_session_url = std::env::var("XIN_SESSION_URL").ok();

    let base_url_from_env = match (env_base_url, env_session_url) {
        (Some(b), _) => Some(b),
        (None, Some(su)) => Some(parse_origin_from_session_url(&su)?),
        (None, None) => None,
    };

    let auth_from_env = parse_auth_from_global_env()?;

    // If env already provides everything, we can skip config.
    // Otherwise, we may need config to supply missing fields.
    let cfg_opt = app_config::read_config().ok();

    let (cfg, _cfg_path) = match cfg_opt {
        Some((cfg, path)) => (Some(cfg), Some(path)),
        None => (None, None),
    };

    let (account_name, acct_cfg) = if let Some(cfg) = cfg.as_ref() {
        let name = resolve_account_from_config(cfg, selected_account)?;
        let acct = cfg.accounts.get(&name).ok_or_else(|| {
            XinErrorOut::config(format!("unknown account '{name}' in config"))
        })?;
        (Some(name), Some(acct.clone()))
    } else {
        (selected_account.map(|s| s.to_string()), None)
    };

    // base_url
    let base_url = if let Some(b) = base_url_from_env {
        b
    } else if let Some(acct) = acct_cfg.as_ref() {
        if let Some(session_url) = &acct.session_url {
            parse_origin_from_session_url(session_url)?
        } else if let Some(base) = &acct.base_url {
            base.trim_end_matches('/').to_string()
        } else {
            return Err(XinErrorOut::config(
                "missing baseUrl/sessionUrl in account config".to_string(),
            ));
        }
    } else {
        return Err(XinErrorOut::config(
            "missing XIN_BASE_URL (or XIN_SESSION_URL) environment variable, and no config file found".to_string(),
        ));
    };

    // auth
    let auth = if let Some(a) = auth_from_env {
        a
    } else if let Some(acct) = acct_cfg.as_ref() {
        resolve_auth_from_account(acct)?
    } else {
        return Err(XinErrorOut::config(
            "missing XIN_TOKEN (or XIN_TOKEN_FILE) or XIN_BASIC_USER/XIN_BASIC_PASS environment variable".to_string(),
        ));
    };

    let follow_redirect_hosts = {
        let env_hosts = parse_follow_redirect_hosts_from_env();
        if !env_hosts.is_empty() {
            env_hosts
        } else if let Some(acct) = acct_cfg.as_ref() {
            acct.trust_redirect_hosts.clone()
        } else {
            vec![]
        }
    };

    Ok(ResolvedRuntimeConfig {
        config: RuntimeConfig {
            base_url,
            auth,
            follow_redirect_hosts,
        },
        account: account_name,
    })
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
