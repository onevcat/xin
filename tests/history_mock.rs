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
async fn history_bootstrap_returns_current_state_and_empty_changes() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_session(&server)))
        .mount(&server)
        .await;

    let email_get_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/get", {
                "accountId": "A",
                "state": "S123",
                "list": [],
                "notFound": []
            }, "g0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(email_get_response))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["history"])
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "xin failed. status={:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let v: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(v.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(v.get("command").and_then(|v| v.as_str()), Some("history"));

    assert_eq!(
        v.pointer("/data/newState").and_then(|v| v.as_str()),
        Some("S123")
    );
    assert_eq!(
        v.pointer("/data/sinceState").and_then(|v| v.as_str()),
        Some("S123")
    );

    assert_eq!(
        v.pointer("/data/changes/created")
            .and_then(|v| v.as_array())
            .map(|a| a.len()),
        Some(0)
    );
    assert_eq!(
        v.pointer("/data/changes/updated")
            .and_then(|v| v.as_array())
            .map(|a| a.len()),
        Some(0)
    );
    assert_eq!(
        v.pointer("/data/changes/destroyed")
            .and_then(|v| v.as_array())
            .map(|a| a.len()),
        Some(0)
    );
}

#[tokio::test]
async fn history_changes_returns_created_updated_destroyed_and_new_state() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_session(&server)))
        .mount(&server)
        .await;

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

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["history", "--since", "S0", "--max", "100"])
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "xin failed. status={:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let v: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(v.get("ok").and_then(|v| v.as_bool()), Some(true));

    assert_eq!(
        v.pointer("/data/sinceState").and_then(|v| v.as_str()),
        Some("S0")
    );
    assert_eq!(
        v.pointer("/data/newState").and_then(|v| v.as_str()),
        Some("S1")
    );

    assert_eq!(
        v.pointer("/data/changes/created/0")
            .and_then(|v| v.as_str()),
        Some("m_new")
    );
    assert_eq!(
        v.pointer("/data/changes/updated/0")
            .and_then(|v| v.as_str()),
        Some("m_upd")
    );
    assert_eq!(
        v.pointer("/data/changes/destroyed/0")
            .and_then(|v| v.as_str()),
        Some("m_del")
    );
}

#[tokio::test]
async fn history_changes_paging_uses_new_state_as_continuation_cursor() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_session(&server)))
        .mount(&server)
        .await;

    let changes_page1 = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/changes", {
                "accountId": "A",
                "oldState": "S0",
                "newState": "S1",
                "hasMoreChanges": true,
                "created": ["m1"],
                "updated": [],
                "destroyed": []
            }, "c0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/changes"))
        .and(body_string_contains("\"sinceState\":\"S0\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(changes_page1))
        .mount(&server)
        .await;

    let output1 = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["history", "--since", "S0", "--max", "100"])
        .output()
        .expect("run");

    assert!(
        output1.status.success(),
        "xin failed. status={:?}\nstdout:\n{}\nstderr:\n{}",
        output1.status.code(),
        String::from_utf8_lossy(&output1.stdout),
        String::from_utf8_lossy(&output1.stderr)
    );

    let v1: serde_json::Value = serde_json::from_slice(&output1.stdout).expect("json");
    let next_page = v1
        .pointer("/meta/nextPage")
        .and_then(|v| v.as_str())
        .expect("meta.nextPage");

    let changes_page2 = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/changes", {
                "accountId": "A",
                "oldState": "S1",
                "newState": "S2",
                "hasMoreChanges": false,
                "created": [],
                "updated": ["m2"],
                "destroyed": []
            }, "c0"]
        ]
    });

    // Expect the second call to continue from S1 (the newState from page 1).
    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/changes"))
        .and(body_string_contains("\"sinceState\":\"S1\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(changes_page2))
        .mount(&server)
        .await;

    let output2 = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["history", "--page", next_page, "--max", "100"])
        .output()
        .expect("run");

    assert!(
        output2.status.success(),
        "xin failed. status={:?}\nstdout:\n{}\nstderr:\n{}",
        output2.status.code(),
        String::from_utf8_lossy(&output2.stdout),
        String::from_utf8_lossy(&output2.stderr)
    );

    let v2: serde_json::Value = serde_json::from_slice(&output2.stdout).expect("json");
    assert_eq!(v2.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        v2.pointer("/data/sinceState").and_then(|v| v.as_str()),
        Some("S1")
    );
    assert_eq!(
        v2.pointer("/data/newState").and_then(|v| v.as_str()),
        Some("S2")
    );
    assert_eq!(
        v2.pointer("/data/changes/updated/0")
            .and_then(|v| v.as_str()),
        Some("m2")
    );
}
