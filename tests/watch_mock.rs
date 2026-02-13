use assert_cmd::Command;
use serde_json::json;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn mock_session(server: &MockServer) -> serde_json::Value {
    json!({
        "capabilities": {
            "urn:ietf:params:jmap:core": {
                "maxSizeUpload": 1000000,
                "maxConcurrentUpload": 4,
                "maxSizeRequest": 1000000,
                "maxConcurrentRequests": 4,
                "maxCallsInRequest": 16,
                "maxObjectsInGet": 256,
                "maxObjectsInSet": 256,
                "collationAlgorithms": ["i;unicode-casemap"]
            },
            "urn:ietf:params:jmap:mail": {},
            "urn:ietf:params:jmap:submission": {}
        },
        "accounts": {
            "A": {
                "name": "mock",
                "isPersonal": true,
                "isReadOnly": false,
                "accountCapabilities": {
                    "urn:ietf:params:jmap:mail": {},
                    "urn:ietf:params:jmap:core": {},
                    "urn:ietf:params:jmap:submission": {}
                }
            }
        },
        "primaryAccounts": {
            "urn:ietf:params:jmap:mail": "A",
            "urn:ietf:params:jmap:core": "A",
            "urn:ietf:params:jmap:submission": "A"
        },
        "username": "me",
        "apiUrl": format!("{}/jmap", server.uri()),
        "downloadUrl": format!("{}/download/{{accountId}}/{{blobId}}/{{name}}?type={{type}}", server.uri()),
        "uploadUrl": format!("{}/upload/{{accountId}}", server.uri()),
        "eventSourceUrl": format!("{}/events", server.uri()),
        "state": "s"
    })
}

#[tokio::test]
async fn watch_once_emits_ready_and_change_events_and_writes_checkpoint() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_session(&server)))
        .mount(&server)
        .await;

    // Email/changes response (single page).
    let changes_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/changes", {
                "accountId": "A",
                "oldState": "S0",
                "newState": "S1",
                "hasMoreChanges": false,
                "created": ["m_new"],
                "updated": ["m_upd"],
                "destroyed": ["m_del"]
            }, "c0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/changes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(changes_response))
        .mount(&server)
        .await;

    let tmp = tempfile::tempdir().expect("tmp");
    let checkpoint = tmp.path().join("watch.token");

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args([
            "watch",
            "--since",
            "S0",
            "--max",
            "100",
            "--once",
            "--checkpoint",
        ])
        .arg(&checkpoint)
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "xin failed. status={:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // First line should be a JSON event.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines.len() >= 2,
        "expected multiple NDJSON lines, got: {stdout}"
    );

    let ready: serde_json::Value = serde_json::from_str(lines[0]).expect("ready json");
    assert_eq!(ready.get("type").and_then(|v| v.as_str()), Some("ready"));

    // Should include email.change events.
    assert!(stdout.contains("\"type\":\"email.change\""));

    // Checkpoint should be written.
    let token = std::fs::read_to_string(&checkpoint).expect("checkpoint");
    assert!(!token.trim().is_empty());
}

#[tokio::test]
async fn watch_no_envelope_success_is_ndjson_only() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_session(&server)))
        .mount(&server)
        .await;

    // Email/changes response (single page).
    let changes_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/changes", {
                "accountId": "A",
                "oldState": "S0",
                "newState": "S1",
                "hasMoreChanges": false,
                "created": ["m_new"],
                "updated": [],
                "destroyed": []
            }, "c0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/changes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(changes_response))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["watch", "--no-envelope", "--since", "S0", "--once"])
        .output()
        .expect("run");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("\"schemaVersion\""),
        "expected no final envelope when --no-envelope is set. stdout:\n{stdout}"
    );

    let lines: Vec<&str> = stdout.lines().collect();
    assert!(!lines.is_empty());

    let ready: serde_json::Value = serde_json::from_str(lines[0]).expect("ready json");
    assert_eq!(ready.get("type").and_then(|v| v.as_str()), Some("ready"));
}

#[tokio::test]
async fn watch_no_envelope_emits_error_event_on_invalid_page_token() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_session(&server)))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["watch", "--no-envelope", "--page", "not-a-token", "--once"])
        .output()
        .expect("run");

    assert!(
        !output.status.success(),
        "expected failure exit code. stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("\"schemaVersion\""),
        "expected no final envelope when --no-envelope is set. stdout:\n{stdout}"
    );

    let lines: Vec<&str> = stdout.lines().collect();
    assert!(!lines.is_empty());

    let v: serde_json::Value = serde_json::from_str(lines[0]).expect("error json");
    assert_eq!(v.get("type").and_then(|v| v.as_str()), Some("error"));
    assert_eq!(
        v.pointer("/error/kind").and_then(|v| v.as_str()),
        Some("xinUsageError")
    );
}

#[tokio::test]
async fn watch_checkpoint_resumes_and_ignores_since_when_not_provided() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_session(&server)))
        .mount(&server)
        .await;

    // First run: S0->S1
    let changes_1 = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/changes", {
                "accountId": "A",
                "oldState": "S0",
                "newState": "S1",
                "hasMoreChanges": false,
                "created": [],
                "updated": [],
                "destroyed": []
            }, "c0"]
        ]
    });

    // Second run: should call oldState=S1 (from checkpoint) and return S2.
    let changes_2 = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/changes", {
                "accountId": "A",
                "oldState": "S1",
                "newState": "S2",
                "hasMoreChanges": false,
                "created": ["m2"],
                "updated": [],
                "destroyed": []
            }, "c0"]
        ]
    });

    // We mount two responses in order.
    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/changes"))
        .and(body_string_contains("\"sinceState\":\"S0\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(changes_1))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/changes"))
        .and(body_string_contains("\"sinceState\":\"S1\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(changes_2))
        .expect(1)
        .mount(&server)
        .await;

    let tmp = tempfile::tempdir().expect("tmp");
    let checkpoint = tmp.path().join("watch.token");

    // run 1 (writes checkpoint)
    Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["watch", "--since", "S0", "--once", "--checkpoint"])
        .arg(&checkpoint)
        .assert()
        .success();

    // run 2 (resume from checkpoint)
    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["watch", "--once", "--checkpoint"])
        .arg(&checkpoint)
        .output()
        .expect("run2");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"id\":\"m2\""), "stdout: {stdout}");
}
