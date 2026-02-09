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
async fn batch_delete_sends_email_set_destroy() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_session(&server)))
        .mount(&server)
        .await;

    let email_set_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/set", {
                "accountId": "A",
                "oldState": "s",
                "newState": "s",
                "destroyed": ["m1", "m2"]
            }, "e0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/set"))
        .and(body_string_contains("\"destroy\":[\"m1\",\"m2\"]"))
        .respond_with(ResponseTemplate::new(200).set_body_json(email_set_response))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["--force", "batch", "delete", "m1", "m2"])
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
        v.get("command").and_then(|v| v.as_str()),
        Some("batch.delete")
    );
}

#[tokio::test]
async fn thread_delete_expands_thread_then_destroys_emails() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_session(&server)))
        .mount(&server)
        .await;

    let thread_get_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Thread/get", {
                "accountId": "A",
                "state": "s",
                "list": [{"id": "t1", "emailIds": ["m1", "m2"]}],
                "notFound": []
            }, "t0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Thread/get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(thread_get_response))
        .mount(&server)
        .await;

    let email_set_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/set", {
                "accountId": "A",
                "oldState": "s",
                "newState": "s",
                "destroyed": ["m1", "m2"]
            }, "e0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/set"))
        .and(body_string_contains("\"destroy\":[\"m1\",\"m2\"]"))
        .respond_with(ResponseTemplate::new(200).set_body_json(email_set_response))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["--force", "thread", "delete", "t1"])
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
        v.get("command").and_then(|v| v.as_str()),
        Some("thread.delete")
    );
}
