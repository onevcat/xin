use serde_json::{Value, json};

use crate::app_config;
use crate::cli::AuthSetTokenArgs;
use crate::error::XinErrorOut;
use crate::output::{Envelope, Meta};

fn default_token_file_for_account(account: &str) -> Result<std::path::PathBuf, XinErrorOut> {
    let dir = app_config::default_tokens_dir()
        .map_err(|e| XinErrorOut::config(format!("token dir error: {:?}", e)))?;
    Ok(dir.join(format!("{account}.token")))
}

pub async fn set_token(cli_account: Option<&str>, args: &AuthSetTokenArgs) -> Envelope<Value> {
    let command_name = "auth.set_token";

    // Ensure we have at least a minimal config.
    let (mut cfg, _path) = match app_config::ensure_minimal_fastmail_config() {
        Ok(v) => v,
        Err(e) => return Envelope::err(command_name, None, e),
    };

    // Select account.
    let account = if let Some(a) = cli_account {
        a.to_string()
    } else if let Some(a) = &cfg.defaults.account {
        a.clone()
    } else {
        "fastmail".to_string()
    };

    // Ensure account exists.
    if !cfg.accounts.contains_key(&account) {
        if account == "fastmail" {
            // Shouldn't happen (ensure_minimal creates it), but keep it robust.
            cfg = app_config::ensure_minimal_fastmail_config()
                .map(|v| v.0)
                .unwrap_or(cfg);
        } else {
            return Envelope::err(
                command_name,
                None,
                XinErrorOut::usage(format!(
                    "unknown account '{account}'; run `xin config init` and edit config first"
                )),
            );
        }
    }

    let acct = cfg.accounts.get_mut(&account).expect("account exists");

    // Decide token file path.
    let token_file_path = match &acct.auth {
        app_config::AuthConfigFile::Bearer { token_file, .. } => {
            if let Some(s) = token_file {
                match app_config::expand_user_path(s) {
                    Ok(p) => p,
                    Err(e) => return Envelope::err(command_name, None, e),
                }
            } else {
                match default_token_file_for_account(&account) {
                    Ok(p) => p,
                    Err(e) => return Envelope::err(command_name, None, e),
                }
            }
        }
        _ => match default_token_file_for_account(&account) {
            Ok(p) => p,
            Err(e) => return Envelope::err(command_name, None, e),
        },
    };

    // Write token file.
    if let Err(e) = app_config::write_token_file(&token_file_path, &args.token) {
        return Envelope::err(command_name, None, e);
    }

    // Update config to point to this token file.
    acct.auth = app_config::AuthConfigFile::Bearer {
        token: None,
        token_env: None,
        token_file: Some(token_file_path.to_string_lossy().to_string()),
    };

    if cfg.defaults.account.is_none() {
        cfg.defaults.account = Some(account.clone());
    }

    let config_path = match app_config::write_config(&cfg) {
        Ok(p) => p,
        Err(e) => return Envelope::err(command_name, None, e),
    };

    Envelope::ok(
        command_name,
        None,
        json!({
            "account": account,
            "configPath": config_path.to_string_lossy(),
            "tokenFile": token_file_path.to_string_lossy(),
        }),
        Meta::default(),
    )
}
