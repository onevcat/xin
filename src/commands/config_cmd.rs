use serde_json::{Value, json};

use crate::app_config;
use crate::cli::{ConfigSetDefaultArgs, ConfigShowArgs};
use crate::error::XinErrorOut;
use crate::output::{Envelope, Meta};

fn sanitize_config(mut cfg: app_config::AppConfig) -> app_config::AppConfig {
    use app_config::AuthConfigFile;

    for (_name, acct) in cfg.accounts.iter_mut() {
        match &mut acct.auth {
            AuthConfigFile::Bearer { token, .. } => {
                *token = None;
            }
            AuthConfigFile::Basic { pass, .. } => {
                *pass = None;
            }
        }
    }

    cfg
}

pub async fn init() -> Envelope<Value> {
    let command_name = "config.init";

    let (cfg, path, created) = match app_config::read_config() {
        Ok((cfg, path)) => (cfg, path, false),
        Err(_) => {
            let (cfg, path) = match app_config::ensure_minimal_fastmail_config() {
                Ok(v) => v,
                Err(e) => return Envelope::err(command_name, None, e),
            };
            (cfg, path, true)
        }
    };

    Envelope::ok(
        command_name,
        None,
        json!({
            "path": path.to_string_lossy(),
            "created": created,
            "defaults": {"account": cfg.defaults.account},
            "accounts": cfg.accounts.keys().collect::<Vec<_>>()
        }),
        Meta::default(),
    )
}

pub async fn list() -> Envelope<Value> {
    let command_name = "config.list";

    let (cfg, path) = match app_config::read_config() {
        Ok(v) => v,
        Err(e) => return Envelope::err(command_name, None, e),
    };

    let mut accounts: Vec<Value> = Vec::new();
    for (name, acct) in &cfg.accounts {
        let auth = match &acct.auth {
            app_config::AuthConfigFile::Bearer {
                token_env,
                token_file,
                ..
            } => json!({
                "type": "bearer",
                "tokenEnv": token_env,
                "tokenFile": token_file,
            }),
            app_config::AuthConfigFile::Basic {
                user,
                pass_env,
                pass_file,
                ..
            } => json!({
                "type": "basic",
                "user": user,
                "passEnv": pass_env,
                "passFile": pass_file,
            }),
        };

        accounts.push(json!({
            "name": name,
            "baseUrl": acct.base_url,
            "sessionUrl": acct.session_url,
            "auth": auth,
            "trustRedirectHosts": acct.trust_redirect_hosts,
        }));
    }

    Envelope::ok(
        command_name,
        None,
        json!({
            "path": path.to_string_lossy(),
            "defaultAccount": cfg.defaults.account,
            "accounts": accounts
        }),
        Meta::default(),
    )
}

pub async fn set_default(args: &ConfigSetDefaultArgs) -> Envelope<Value> {
    let command_name = "config.set_default";

    let (mut cfg, _path) = match app_config::read_config() {
        Ok(v) => v,
        Err(e) => return Envelope::err(command_name, None, e),
    };

    if !cfg.accounts.contains_key(&args.account) {
        return Envelope::err(
            command_name,
            None,
            XinErrorOut::usage(format!("unknown account '{}'", args.account)),
        );
    }

    cfg.defaults.account = Some(args.account.clone());

    let path = match app_config::write_config(&cfg) {
        Ok(p) => p,
        Err(e) => return Envelope::err(command_name, None, e),
    };

    Envelope::ok(
        command_name,
        None,
        json!({
            "path": path.to_string_lossy(),
            "defaultAccount": cfg.defaults.account,
        }),
        Meta::default(),
    )
}

pub async fn show(cli_account: Option<&str>, args: &ConfigShowArgs) -> Envelope<Value> {
    let command_name = "config.show";

    if args.effective {
        let resolved = match crate::config::resolve_runtime_config(cli_account) {
            Ok(r) => r,
            Err(e) => return Envelope::err(command_name, None, e),
        };

        // Never include secret values.
        let auth = match &resolved.config.auth {
            crate::config::AuthConfig::Bearer(_) => json!({"type":"bearer"}),
            crate::config::AuthConfig::Basic { .. } => json!({"type":"basic"}),
        };

        return Envelope::ok(
            command_name,
            None,
            json!({
                "selectedAccount": resolved.account,
                "runtime": {
                    "baseUrl": resolved.config.base_url,
                    "auth": auth,
                    "trustRedirectHosts": resolved.config.follow_redirect_hosts
                }
            }),
            Meta::default(),
        );
    }

    let (cfg, path) = match app_config::read_config() {
        Ok(v) => v,
        Err(e) => return Envelope::err(command_name, None, e),
    };

    Envelope::ok(
        command_name,
        None,
        json!({
            "path": path.to_string_lossy(),
            "config": sanitize_config(cfg)
        }),
        Meta::default(),
    )
}
