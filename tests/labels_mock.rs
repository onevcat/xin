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
async fn labels_list_works_against_mock_jmap() {
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
                    "id": "mb1",
                    "name": "Inbox",
                    "role": "inbox",
                    "parentId": null,
                    "sortOrder": 0,
                    "totalEmails": 1,
                    "unreadEmails": 1,
                    "totalThreads": 1,
                    "unreadThreads": 1,
                    "isSubscribed": true
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
        .args(["labels", "list"])
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
        Some("labels.list")
    );

    let items = v
        .get("data")
        .and_then(|d| d.get("mailboxes"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].get("id").and_then(|v| v.as_str()), Some("mb1"));
    assert_eq!(
        items[0].get("role").and_then(|v| v.as_str()),
        Some("inbox")
    );
}

#[tokio::test]
async fn labels_create_returns_created_summary() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_session(&server)))
        .mount(&server)
        .await;

    let mailbox_set_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Mailbox/set", {
                "accountId": "A",
                "oldState": "s",
                "newState": "s",
                "created": {"c0": {"id": "mbNew", "name": "A"}}
            }, "s0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Mailbox/set"))
        .and(body_string_contains("\"name\":\"A\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(mailbox_set_response))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["labels", "create", "A"])
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
        Some("labels.create")
    );

    let created = v
        .get("data")
        .and_then(|d| d.get("created"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    assert_eq!(created.len(), 1);
    assert_eq!(created[0].get("id").and_then(|v| v.as_str()), Some("mbNew"));
}

#[tokio::test]
async fn labels_delete_passes_remove_emails_argument() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_session(&server)))
        .mount(&server)
        .await;

    let mailbox_set_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Mailbox/set", {
                "accountId": "A",
                "oldState": "s",
                "newState": "s",
                "destroyed": ["mb1"]
            }, "s0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Mailbox/set"))
        .and(body_string_contains("onDestroyRemoveEmails"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mailbox_set_response))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["labels", "delete", "mb1", "--remove-emails"])
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
        v.get("data")
            .and_then(|d| d.get("destroyed"))
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())
            .and_then(|v| v.as_str()),
        Some("mb1")
    );
}
