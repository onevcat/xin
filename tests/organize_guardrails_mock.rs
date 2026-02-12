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

fn assert_usage_error(output: &std::process::Output, expected_message_contains: &str) {
    assert!(
        !output.status.success(),
        "expected failure exit status\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let v: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(v.get("ok").and_then(|v| v.as_bool()), Some(false));
    assert_eq!(
        v.get("error")
            .and_then(|e| e.get("kind"))
            .and_then(|s| s.as_str()),
        Some("xinUsageError")
    );

    let msg = v
        .get("error")
        .and_then(|e| e.get("message"))
        .and_then(|s| s.as_str())
        .unwrap_or("");
    assert!(
        msg.contains(expected_message_contains),
        "expected error message to contain {expected_message_contains:?}, got {msg:?}"
    );
}

#[tokio::test]
async fn batch_delete_requires_force_and_makes_no_requests() {
    let server = MockServer::start().await;

    // If xin tries to connect, it will call this. For this guardrail we expect 0 requests.
    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_session(&server)))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["batch", "delete", "m1"])
        .output()
        .expect("run");

    assert_usage_error(&output, "pass --force");

    let requests = server.received_requests().await.expect("requests");
    assert!(requests.is_empty(), "expected no HTTP requests");
}

#[tokio::test]
async fn thread_delete_requires_force_and_makes_no_requests() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_session(&server)))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["thread", "delete", "t1"])
        .output()
        .expect("run");

    assert_usage_error(&output, "pass --force");

    let requests = server.received_requests().await.expect("requests");
    assert!(requests.is_empty(), "expected no HTTP requests");
}

#[tokio::test]
async fn whole_thread_requires_exactly_one_email_id() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_session(&server)))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["trash", "--whole-thread", "m1", "m2"])
        .output()
        .expect("run");

    assert_usage_error(&output, "requires exactly one emailId");

    let requests = server.received_requests().await.expect("requests");
    assert!(requests.is_empty(), "expected no HTTP requests");
}

#[tokio::test]
async fn batch_modify_with_no_changes_errors_and_does_not_call_email_set() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_session(&server)))
        .mount(&server)
        .await;

    // batch.modify calls Mailbox/get to resolve roles/names.
    let mailbox_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Mailbox/get", {
                "accountId": "A",
                "state": "s",
                "list": [{
                    "id": "mb_inbox",
                    "name": "Inbox",
                    "role": "inbox"
                }],
                "notFound": []
            }, "m0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Mailbox/get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mailbox_response))
        .mount(&server)
        .await;

    // Intentionally do NOT mount Email/set.

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["batch", "modify", "m1"])
        .output()
        .expect("run");

    assert_usage_error(&output, "no changes specified");

    let requests = server.received_requests().await.expect("requests");
    assert!(
        requests
            .iter()
            .any(|r| String::from_utf8_lossy(&r.body).contains("Mailbox/get")),
        "expected Mailbox/get request"
    );
    assert!(
        !requests
            .iter()
            .any(|r| String::from_utf8_lossy(&r.body).contains("Email/set")),
        "unexpected Email/set request"
    );
}

#[tokio::test]
async fn thread_modify_with_no_changes_errors_and_does_not_call_thread_get_or_email_set() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_session(&server)))
        .mount(&server)
        .await;

    let mailbox_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Mailbox/get", {
                "accountId": "A",
                "state": "s",
                "list": [{
                    "id": "mb_inbox",
                    "name": "Inbox",
                    "role": "inbox"
                }],
                "notFound": []
            }, "m0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Mailbox/get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mailbox_response))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["thread", "modify", "t1"])
        .output()
        .expect("run");

    assert_usage_error(&output, "no changes specified");

    let requests = server.received_requests().await.expect("requests");
    assert!(
        requests
            .iter()
            .any(|r| String::from_utf8_lossy(&r.body).contains("Mailbox/get")),
        "expected Mailbox/get request"
    );
    assert!(
        !requests
            .iter()
            .any(|r| String::from_utf8_lossy(&r.body).contains("Thread/get")),
        "unexpected Thread/get request"
    );
    assert!(
        !requests
            .iter()
            .any(|r| String::from_utf8_lossy(&r.body).contains("Email/set")),
        "unexpected Email/set request"
    );
}

#[tokio::test]
async fn thread_delete_thread_not_found_is_usage_error_and_does_not_destroy() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_session(&server)))
        .mount(&server)
        .await;

    // Thread/get returns empty list => None => "thread not found".
    let thread_get_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Thread/get", {
                "accountId": "A",
                "state": "s",
                "list": [],
                "notFound": ["t_missing"]
            }, "t0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Thread/get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(thread_get_response))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["--dry-run", "thread", "delete", "--force", "t_missing"])
        .output()
        .expect("run");

    assert_usage_error(&output, "thread not found");

    let requests = server.received_requests().await.expect("requests");
    assert!(
        requests
            .iter()
            .any(|r| String::from_utf8_lossy(&r.body).contains("Thread/get")),
        "expected Thread/get request"
    );
    assert!(
        !requests
            .iter()
            .any(|r| String::from_utf8_lossy(&r.body).contains("Email/set")),
        "unexpected Email/set request"
    );
}

#[tokio::test]
async fn trash_whole_thread_email_not_found_is_usage_error_and_does_not_call_thread_get() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_session(&server)))
        .mount(&server)
        .await;

    let mailbox_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Mailbox/get", {
                "accountId": "A",
                "state": "s",
                "list": [{
                    "id": "mb_trash",
                    "name": "Trash",
                    "role": "trash"
                }],
                "notFound": []
            }, "m0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Mailbox/get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mailbox_response))
        .mount(&server)
        .await;

    let email_get_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/get", {
                "accountId": "A",
                "state": "s",
                "list": [],
                "notFound": ["m_missing"]
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
        .args(["--dry-run", "trash", "--whole-thread", "m_missing"])
        .output()
        .expect("run");

    assert_usage_error(&output, "email not found");

    let requests = server.received_requests().await.expect("requests");
    assert!(
        requests
            .iter()
            .any(|r| String::from_utf8_lossy(&r.body).contains("Mailbox/get")),
        "expected Mailbox/get request"
    );
    assert!(
        requests
            .iter()
            .any(|r| String::from_utf8_lossy(&r.body).contains("Email/get")),
        "expected Email/get request"
    );
    assert!(
        !requests
            .iter()
            .any(|r| String::from_utf8_lossy(&r.body).contains("Thread/get")),
        "unexpected Thread/get request"
    );
    assert!(
        !requests
            .iter()
            .any(|r| String::from_utf8_lossy(&r.body).contains("Email/set")),
        "unexpected Email/set request"
    );
}
