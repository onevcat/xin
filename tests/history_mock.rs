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

#[tokio::test]
async fn history_page_token_is_source_of_truth_when_max_not_specified() {
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
        .and(body_string_contains("\"maxChanges\":2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(changes_page1))
        .mount(&server)
        .await;

    let output1 = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["history", "--since", "S0", "--max", "2"])
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
        .expect("meta.nextPage")
        .to_string();

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

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/changes"))
        .and(body_string_contains("\"sinceState\":\"S1\""))
        .and(body_string_contains("\"maxChanges\":2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(changes_page2))
        .mount(&server)
        .await;

    // NOTE: Intentionally omit --max here. Token should be the source of truth.
    let output2 = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["history", "--page", &next_page])
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
}

#[tokio::test]
async fn history_hydrate_fetches_created_and_updated_summaries_in_one_request() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_session(&server)))
        .mount(&server)
        .await;

    let jmap_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/changes", {
                "accountId": "A",
                "oldState": "S0",
                "newState": "S1",
                "hasMoreChanges": false,
                "created": ["m1"],
                "updated": ["m2"],
                "destroyed": ["m3"]
            }, "c0"],
            ["Email/get", {
                "accountId": "A",
                "state": "S1",
                "list": [
                    {
                        "id": "m1",
                        "threadId": "t1",
                        "receivedAt": "2026-02-08T00:00:00Z",
                        "subject": "Created",
                        "from": [{"name": "Alice", "email": "alice@example.com"}],
                        "to": [{"name": null, "email": "me@example.com"}],
                        "preview": "p1",
                        "hasAttachment": false,
                        "mailboxIds": {"inbox": true},
                        "keywords": {"$seen": true}
                    }
                ],
                "notFound": []
            }, "g1"],
            ["Email/get", {
                "accountId": "A",
                "state": "S1",
                "list": [
                    {
                        "id": "m2",
                        "threadId": "t2",
                        "receivedAt": "2026-02-08T00:00:01Z",
                        "subject": "Updated",
                        "from": [{"name": "Bob", "email": "bob@example.com"}],
                        "to": [{"name": null, "email": "me@example.com"}],
                        "preview": "p2",
                        "hasAttachment": true,
                        "mailboxIds": {"inbox": true},
                        "keywords": {}
                    }
                ],
                "notFound": []
            }, "g2"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("\"Email/changes\""))
        .and(body_string_contains("\"Email/get\""))
        .and(body_string_contains("\"#ids\""))
        .and(body_string_contains("\"path\":\"/created\""))
        .and(body_string_contains("\"path\":\"/updated\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(jmap_response))
        .mount(&server)
        .await;

    let out = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["history", "--since", "S0", "--max", "100", "--hydrate"])
        .output()
        .expect("run");

    assert!(
        out.status.success(),
        "xin failed. status={:?}\nstdout:\n{}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(v.pointer("/ok").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        v.pointer("/data/changes/created/0")
            .and_then(|v| v.as_str()),
        Some("m1")
    );
    assert_eq!(
        v.pointer("/data/changes/updated/0")
            .and_then(|v| v.as_str()),
        Some("m2")
    );
    assert_eq!(
        v.pointer("/data/changes/destroyed/0")
            .and_then(|v| v.as_str()),
        Some("m3")
    );

    // Hydrated summaries present.
    assert_eq!(
        v.pointer("/data/hydrated/created/0/emailId")
            .and_then(|v| v.as_str()),
        Some("m1")
    );
    assert_eq!(
        v.pointer("/data/hydrated/updated/0/emailId")
            .and_then(|v| v.as_str()),
        Some("m2")
    );

    // Ensure only one POST occurred.
    let requests = server.received_requests().await.expect("requests");
    let posts = requests
        .iter()
        .filter(|r| r.method.as_str() == "POST")
        .count();
    assert_eq!(posts, 1);
}

#[tokio::test]
async fn history_page_token_max_mismatch_is_usage_error_and_does_not_hit_changes_endpoint() {
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
                "created": [],
                "updated": [],
                "destroyed": []
            }, "c0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/changes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(changes_page1))
        .mount(&server)
        .await;

    let out1 = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["history", "--since", "S0", "--max", "2"])
        .output()
        .expect("run");

    assert!(out1.status.success());

    let v1: serde_json::Value = serde_json::from_slice(&out1.stdout).expect("json");
    let tok = v1
        .pointer("/meta/nextPage")
        .and_then(|v| v.as_str())
        .expect("meta.nextPage")
        .to_string();

    // Second run: change args (max) while reusing token => usage error.
    let out2 = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["history", "--page", &tok, "--max", "3"])
        .output()
        .expect("run");

    assert!(!out2.status.success());

    let v2: serde_json::Value = serde_json::from_slice(&out2.stdout).expect("json");
    assert_eq!(v2.get("ok").and_then(|v| v.as_bool()), Some(false));
    assert_eq!(
        v2.pointer("/error/kind").and_then(|v| v.as_str()),
        Some("xinUsageError")
    );

    // Assert we only hit Email/changes once total.
    let requests = server.received_requests().await.expect("requests");
    let changes_posts = requests
        .iter()
        .filter(|r| String::from_utf8_lossy(&r.body).contains("Email/changes"))
        .count();
    assert_eq!(changes_posts, 1);
}

#[tokio::test]
async fn history_page_token_since_mismatch_is_usage_error_and_does_not_hit_changes_endpoint() {
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
                "created": [],
                "updated": [],
                "destroyed": []
            }, "c0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/changes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(changes_page1))
        .mount(&server)
        .await;

    let out1 = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["history", "--since", "S0", "--max", "2"])
        .output()
        .expect("run");

    assert!(out1.status.success());

    let v1: serde_json::Value = serde_json::from_slice(&out1.stdout).expect("json");
    let tok = v1
        .pointer("/meta/nextPage")
        .and_then(|v| v.as_str())
        .expect("meta.nextPage")
        .to_string();

    // Second run: change args (since) while reusing token => usage error.
    let out2 = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["history", "--page", &tok, "--since", "S999"])
        .output()
        .expect("run");

    assert!(!out2.status.success());

    let v2: serde_json::Value = serde_json::from_slice(&out2.stdout).expect("json");
    assert_eq!(v2.get("ok").and_then(|v| v.as_bool()), Some(false));
    assert_eq!(
        v2.pointer("/error/kind").and_then(|v| v.as_str()),
        Some("xinUsageError")
    );

    // Assert we only hit Email/changes once total.
    let requests = server.received_requests().await.expect("requests");
    let changes_posts = requests
        .iter()
        .filter(|r| String::from_utf8_lossy(&r.body).contains("Email/changes"))
        .count();
    assert_eq!(changes_posts, 1);
}
