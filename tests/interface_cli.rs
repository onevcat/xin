use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_mentions_key_commands() {
    let mut cmd = Command::cargo_bin("xin").unwrap();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("search"))
        .stdout(predicate::str::contains("messages"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("thread"))
        .stdout(predicate::str::contains("attachment"))
        .stdout(predicate::str::contains("labels"))
        .stdout(predicate::str::contains("mailboxes"))
        .stdout(predicate::str::contains("drafts"))
        .stdout(predicate::str::contains("send"))
        .stdout(predicate::str::contains("identities"))
        .stdout(predicate::str::contains("history"))
        .stdout(predicate::str::contains("watch"));
}

#[test]
fn unimplemented_command_returns_structured_error() {
    let output = Command::cargo_bin("xin")
        .unwrap()
        .args(["labels", "list", "--json"])
        .output()
        .expect("run xin");

    assert!(!output.status.success());

    let v: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(v.get("ok").and_then(|v| v.as_bool()), Some(false));
    assert_eq!(
        v.get("error")
            .and_then(|e| e.get("kind"))
            .and_then(|k| k.as_str()),
        Some("xinNotImplemented")
    );
}
