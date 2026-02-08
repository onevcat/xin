use assert_cmd::Command;
use predicates::prelude::*;

fn run(args: &[&str]) -> (std::process::ExitStatus, serde_json::Value) {
    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .args(args)
        .output()
        .expect("run xin");

    let v: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    (output.status, v)
}

#[test]
fn help_mentions_key_commands() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("xin"));
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
fn default_output_is_json_envelope() {
    let (status, v) = run(&["labels", "list"]);

    // main() forces exit code 1 when ok=false.
    assert_eq!(status.code(), Some(1));

    assert_eq!(
        v.get("schemaVersion").and_then(|v| v.as_str()),
        Some("0.1")
    );
    assert_eq!(v.get("ok").and_then(|v| v.as_bool()), Some(false));
    assert_eq!(v.get("command").and_then(|v| v.as_str()), Some("labels.list"));
    assert_eq!(
        v.get("error")
            .and_then(|e| e.get("kind"))
            .and_then(|k| k.as_str()),
        Some("xinNotImplemented")
    );
}

#[test]
fn mailboxes_alias_exists_and_has_its_own_command_name() {
    let (_status, v) = run(&["mailboxes", "list"]);
    assert_eq!(
        v.get("command").and_then(|v| v.as_str()),
        Some("mailboxes.list")
    );
}

#[test]
fn account_flag_is_reflected_in_envelope() {
    let (_status, v) = run(&["--account", "fastmail", "labels", "list"]);
    assert_eq!(v.get("account").and_then(|v| v.as_str()), Some("fastmail"));
}

#[test]
fn command_names_for_nested_subcommands_are_stable() {
    let (_status, v) = run(&["messages", "search"]);
    assert_eq!(
        v.get("command").and_then(|v| v.as_str()),
        Some("messages.search")
    );

    let (_status, v) = run(&["thread", "get", "T123"]);
    assert_eq!(v.get("command").and_then(|v| v.as_str()), Some("thread.get"));
}
