use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::XinErrorOut;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub defaults: Defaults,

    #[serde(default)]
    pub accounts: BTreeMap<String, AccountConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Defaults {
    #[serde(default)]
    pub account: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccountConfig {
    /// Origin, e.g. https://api.fastmail.com (xin will call /.well-known/jmap)
    #[serde(rename = "baseUrl", default)]
    pub base_url: Option<String>,

    /// Full Session URL, e.g. https://api.fastmail.com/.well-known/jmap
    #[serde(rename = "sessionUrl", default)]
    pub session_url: Option<String>,

    pub auth: AuthConfigFile,

    #[serde(rename = "trustRedirectHosts", default)]
    pub trust_redirect_hosts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AuthConfigFile {
    Bearer {
        #[serde(default)]
        token: Option<String>,
        #[serde(rename = "tokenEnv", default)]
        token_env: Option<String>,
        #[serde(rename = "tokenFile", default)]
        token_file: Option<String>,
    },
    Basic {
        #[serde(default)]
        user: Option<String>,
        #[serde(default)]
        pass: Option<String>,
        #[serde(rename = "passEnv", default)]
        pass_env: Option<String>,
        #[serde(rename = "passFile", default)]
        pass_file: Option<String>,
    },
}

impl Default for AuthConfigFile {
    fn default() -> Self {
        Self::Bearer {
            token: None,
            token_env: None,
            token_file: None,
        }
    }
}

fn home_dir() -> Result<PathBuf, XinErrorOut> {
    let home = std::env::var("HOME")
        .map_err(|_| XinErrorOut::config("missing HOME environment variable".to_string()))?;
    Ok(PathBuf::from(home))
}

pub fn default_config_path() -> Result<PathBuf, XinErrorOut> {
    if let Ok(p) = std::env::var("XIN_CONFIG_PATH") {
        return Ok(PathBuf::from(p));
    }

    let home = home_dir()?;

    // Unified default location across platforms: ~/.config/xin/config.json
    // (can be overridden by XDG_CONFIG_HOME or XIN_CONFIG_PATH).
    let base = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    Ok(base.join("xin/config.json"))
}

pub fn default_tokens_dir() -> Result<PathBuf, XinErrorOut> {
    let cfg_path = default_config_path()?;
    Ok(cfg_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("tokens"))
}

pub fn expand_user_path(s: &str) -> Result<PathBuf, XinErrorOut> {
    if let Some(rest) = s.strip_prefix("~/") {
        Ok(home_dir()?.join(rest))
    } else {
        Ok(PathBuf::from(s))
    }
}

pub fn read_config() -> Result<(AppConfig, PathBuf), XinErrorOut> {
    let path = default_config_path()?;
    let text = fs::read_to_string(&path).map_err(|e| {
        XinErrorOut::config(format!("failed to read config {}: {e}", path.display()))
    })?;
    let cfg: AppConfig = serde_json::from_str(&text)
        .map_err(|e| XinErrorOut::config(format!("invalid config json: {e}")))?;
    Ok((cfg, path))
}

pub fn write_config(cfg: &AppConfig) -> Result<PathBuf, XinErrorOut> {
    let path = default_config_path()?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| XinErrorOut::config(format!("config mkdir failed: {e}")))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
        }
    }

    let text = serde_json::to_string_pretty(cfg)
        .map_err(|e| XinErrorOut::config(format!("config serialize failed: {e}")))?;

    // Best-effort atomic write.
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, format!("{}\n", text))
        .map_err(|e| XinErrorOut::config(format!("config write failed: {e}")))?;
    fs::rename(&tmp, &path)
        .map_err(|e| XinErrorOut::config(format!("config rename failed: {e}")))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }

    Ok(path)
}

pub fn write_token_file(path: &Path, token: &str) -> Result<(), XinErrorOut> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| XinErrorOut::config(format!("token dir mkdir failed: {e}")))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
        }
    }

    // Best-effort atomic write.
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, format!("{}\n", token.trim()))
        .map_err(|e| XinErrorOut::config(format!("token write failed: {e}")))?;
    fs::rename(&tmp, path).map_err(|e| XinErrorOut::config(format!("token rename failed: {e}")))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }

    Ok(())
}

pub fn ensure_minimal_fastmail_config() -> Result<(AppConfig, PathBuf), XinErrorOut> {
    // If config exists, return it.
    if let Ok((cfg, path)) = read_config() {
        return Ok((cfg, path));
    }

    let tokens_dir = default_tokens_dir()?;
    let token_file = tokens_dir.join("fastmail.token");

    let mut cfg = AppConfig::default();
    cfg.defaults.account = Some("fastmail".to_string());

    cfg.accounts.insert(
        "fastmail".to_string(),
        AccountConfig {
            base_url: Some("https://api.fastmail.com".to_string()),
            session_url: None,
            auth: AuthConfigFile::Bearer {
                token: None,
                token_env: None,
                token_file: Some(token_file.to_string_lossy().to_string()),
            },
            trust_redirect_hosts: vec![
                "api.fastmail.com".to_string(),
                "jmap.fastmail.com".to_string(),
                "fastmail.com".to_string(),
            ],
        },
    );

    let path = write_config(&cfg)?;
    Ok((cfg, path))
}
