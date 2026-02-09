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

async fn mount_session(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_session(server)))
        .mount(server)
        .await;
}

#[tokio::test]
async fn drafts_list_works_against_mock_jmap() {
    let server = MockServer::start().await;
    mount_session(&server).await;

    let mailbox_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Mailbox/get", {
                "accountId": "A",
                "state": "s",
                "list": [{
                    "id": "mb1",
                    "name": "Drafts",
                    "role": "drafts"
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

    let list_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/query", {
                "accountId": "A",
                "queryState": "s",
                "canCalculateChanges": false,
                "position": 0,
                "ids": ["m1"],
                "total": 1
            }, "q0"],
            ["Email/get", {
                "accountId": "A",
                "state": "s",
                "list": [{
                    "id": "m1",
                    "threadId": "t1",
                    "receivedAt": "2026-02-08T00:00:00Z",
                    "subject": "Draft",
                    "from": [{"name": "Me", "email": "me@example.com"}],
                    "to": [{"name": null, "email": "you@example.com"}],
                    "preview": "preview",
                    "hasAttachment": false,
                    "mailboxIds": {"mb1": true},
                    "keywords": {"$draft": true}
                }],
                "notFound": []
            }, "q1"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/query"))
        .and(body_string_contains("\"inMailbox\":\"mb1\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(list_response))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["drafts", "list", "--max", "5"])
        .output()
        .expect("run");

    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(v.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        v.get("data")
            .and_then(|d| d.get("items"))
            .and_then(|v| v.as_array())
            .map(|v| v.len()),
        Some(1)
    );
}

#[tokio::test]
async fn drafts_get_works_against_mock_jmap() {
    let server = MockServer::start().await;
    mount_session(&server).await;

    let get_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/get", {
                "accountId": "A",
                "state": "s",
                "list": [{
                    "id": "m1",
                    "threadId": "t1",
                    "receivedAt": "2026-02-08T00:00:00Z",
                    "subject": "Draft",
                    "from": [{"name": "Me", "email": "me@example.com"}],
                    "to": [{"name": null, "email": "you@example.com"}],
                    "preview": "preview",
                    "hasAttachment": false,
                    "mailboxIds": {"mb1": true},
                    "keywords": {"$draft": true}
                }],
                "notFound": []
            }, "g0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(get_response))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["drafts", "get", "m1"])
        .output()
        .expect("run");

    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(v.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        v.get("data")
            .and_then(|d| d.get("draft"))
            .and_then(|e| e.get("emailId"))
            .and_then(|x| x.as_str()),
        Some("m1")
    );
}

#[tokio::test]
async fn drafts_create_works_against_mock_jmap() {
    let server = MockServer::start().await;
    mount_session(&server).await;

    let mailbox_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Mailbox/get", {
                "accountId": "A",
                "state": "s",
                "list": [{
                    "id": "mb1",
                    "name": "Drafts",
                    "role": "drafts"
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

    let identity_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Identity/get", {
                "accountId": "A",
                "state": "s",
                "list": [{
                    "id": "i1",
                    "name": "Me",
                    "email": "me@example.com"
                }],
                "notFound": []
            }, "i0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Identity/get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(identity_response))
        .mount(&server)
        .await;

    let email_set_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/set", {
                "accountId": "A",
                "oldState": "s",
                "newState": "s",
                "created": {
                    "c0": { "id": "m1", "threadId": "t1" }
                }
            }, "e0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/set"))
        .and(body_string_contains("multipart/alternative"))
        .respond_with(ResponseTemplate::new(200).set_body_json(email_set_response))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args([
            "drafts",
            "create",
            "--to",
            "you@example.com",
            "--subject",
            "Hi",
            "--body",
            "Hello",
            "--body-html",
            "<b>Hello</b>",
        ])
        .output()
        .expect("run");

    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(v.get("ok").and_then(|v| v.as_bool()), Some(true));
}

#[tokio::test]
async fn drafts_update_subject_works_against_mock_jmap() {
    let server = MockServer::start().await;
    mount_session(&server).await;

    let update_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/set", {
                "accountId": "A",
                "oldState": "s",
                "newState": "s",
                "updated": {"m1": null}
            }, "u0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/set"))
        .and(body_string_contains("\"update\""))
        .and(body_string_contains("\"subject\":\"Updated\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(update_response))
        .mount(&server)
        .await;

    let get_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/get", {
                "accountId": "A",
                "state": "s",
                "list": [{
                    "id": "m1",
                    "threadId": "t1"
                }],
                "notFound": []
            }, "g0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(get_response))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["drafts", "update", "m1", "--subject", "Updated"])
        .output()
        .expect("run");

    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(v.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        v.get("data")
            .and_then(|d| d.get("draft"))
            .and_then(|e| e.get("emailId"))
            .and_then(|x| x.as_str()),
        Some("m1")
    );
}

#[tokio::test]
async fn drafts_send_works_against_mock_jmap() {
    let server = MockServer::start().await;
    mount_session(&server).await;

    let identity_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Identity/get", {
                "accountId": "A",
                "state": "s",
                "list": [{
                    "id": "i1",
                    "name": "Me",
                    "email": "me@example.com"
                }],
                "notFound": []
            }, "i0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Identity/get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(identity_response))
        .mount(&server)
        .await;

    let submission_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["EmailSubmission/set", {
                "accountId": "A",
                "oldState": "s",
                "newState": "s",
                "created": {
                    "c0": { "id": "s1", "emailId": "m1" }
                }
            }, "s0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("EmailSubmission/set"))
        .respond_with(ResponseTemplate::new(200).set_body_json(submission_response))
        .mount(&server)
        .await;

    let get_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/get", {
                "accountId": "A",
                "state": "s",
                "list": [{
                    "id": "m1",
                    "threadId": "t1"
                }],
                "notFound": []
            }, "g0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(get_response))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["drafts", "send", "m1"])
        .output()
        .expect("run");

    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(v.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        v.get("data")
            .and_then(|d| d.get("submission"))
            .and_then(|s| s.get("id"))
            .and_then(|x| x.as_str()),
        Some("s1")
    );
}

#[tokio::test]
async fn drafts_delete_works_against_mock_jmap() {
    let server = MockServer::start().await;
    mount_session(&server).await;

    let delete_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/set", {
                "accountId": "A",
                "oldState": "s",
                "newState": "s",
                "destroyed": ["m1"]
            }, "d0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/set"))
        .and(body_string_contains("\"destroy\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(delete_response))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["drafts", "delete", "m1"])
        .output()
        .expect("run");

    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(v.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        v.get("data")
            .and_then(|d| d.get("deleted"))
            .and_then(|v| v.as_array())
            .map(|v| v.len()),
        Some(1)
    );
}
