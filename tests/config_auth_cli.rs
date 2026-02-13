use assert_cmd::Command;

fn xin() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("xin"))
}

fn json_from_stdout(out: &std::process::Output) -> serde_json::Value {
    let s = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str(&s).expect("stdout json")
}

#[test]
fn config_init_creates_config_at_overridden_path_and_is_idempotent() {
    let tmp = tempfile::tempdir().expect("tmp");
    let cfg_path = tmp.path().join("xin-config.json");

    // 1st init: created=true
    let out1 = xin()
        .env("XIN_CONFIG_PATH", &cfg_path)
        .args(["config", "init"])
        .output()
        .expect("run");
    assert!(out1.status.success());

    let v1 = json_from_stdout(&out1);
    assert_eq!(v1.pointer("/ok").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        v1.pointer("/command").and_then(|v| v.as_str()),
        Some("config.init")
    );
    assert_eq!(
        v1.pointer("/data/created").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert!(cfg_path.exists());

    // 2nd init: created=false
    let out2 = xin()
        .env("XIN_CONFIG_PATH", &cfg_path)
        .args(["config", "init"])
        .output()
        .expect("run2");
    assert!(out2.status.success());

    let v2 = json_from_stdout(&out2);
    assert_eq!(
        v2.pointer("/data/created").and_then(|v| v.as_bool()),
        Some(false)
    );
}

#[test]
fn auth_set_token_writes_token_file_and_updates_config() {
    let tmp = tempfile::tempdir().expect("tmp");
    let cfg_path = tmp.path().join("xin-config.json");

    // init config
    xin()
        .env("XIN_CONFIG_PATH", &cfg_path)
        .args(["config", "init"])
        .assert()
        .success();

    // set token
    let token = "tok_123";
    let out = xin()
        .env("XIN_CONFIG_PATH", &cfg_path)
        .args(["auth", "set-token", token])
        .output()
        .expect("auth");

    assert!(out.status.success());
    let v = json_from_stdout(&out);

    assert_eq!(
        v.pointer("/command").and_then(|v| v.as_str()),
        Some("auth.set_token")
    );

    let token_file = v
        .pointer("/data/tokenFile")
        .and_then(|v| v.as_str())
        .expect("tokenFile");

    let token_file_path = std::path::PathBuf::from(token_file);
    assert!(token_file_path.exists());

    let stored = std::fs::read_to_string(&token_file_path).expect("read token");
    assert_eq!(stored.trim(), token);

    // config should point to tokenFile
    let cfg_text = std::fs::read_to_string(&cfg_path).expect("read cfg");
    assert!(cfg_text.contains(token_file));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let cfg_mode = std::fs::metadata(&cfg_path)
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(cfg_mode, 0o600);

        let tok_mode = std::fs::metadata(&token_file_path)
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(tok_mode, 0o600);
    }
}

#[test]
fn config_set_default_unknown_account_is_usage_error() {
    let tmp = tempfile::tempdir().expect("tmp");
    let cfg_path = tmp.path().join("xin-config.json");

    xin()
        .env("XIN_CONFIG_PATH", &cfg_path)
        .args(["config", "init"])
        .assert()
        .success();

    let out = xin()
        .env("XIN_CONFIG_PATH", &cfg_path)
        .args(["config", "set-default", "does-not-exist"])
        .output()
        .expect("run");

    assert!(!out.status.success());

    let v = json_from_stdout(&out);
    assert_eq!(v.pointer("/ok").and_then(|v| v.as_bool()), Some(false));
    assert_eq!(
        v.pointer("/error/kind").and_then(|v| v.as_str()),
        Some("xinUsageError")
    );
}

#[test]
fn config_show_effective_prefers_env_over_config() {
    let tmp = tempfile::tempdir().expect("tmp");
    let cfg_path = tmp.path().join("xin-config.json");

    xin()
        .env("XIN_CONFIG_PATH", &cfg_path)
        .args(["config", "init"])
        .assert()
        .success();

    let out = xin()
        .env("XIN_CONFIG_PATH", &cfg_path)
        .env("XIN_BASE_URL", "https://example.invalid")
        .env("XIN_TOKEN", "dummy")
        .args(["config", "show", "--effective"])
        .output()
        .expect("run");

    assert!(out.status.success());

    let v = json_from_stdout(&out);
    assert_eq!(
        v.pointer("/data/runtime/baseUrl").and_then(|v| v.as_str()),
        Some("https://example.invalid")
    );
    assert_eq!(
        v.pointer("/data/runtime/auth/type").and_then(|v| v.as_str()),
        Some("bearer")
    );
}
