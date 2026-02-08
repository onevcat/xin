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
async fn send_text_works_against_mock_jmap() {
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
        .respond_with(ResponseTemplate::new(200).set_body_json(email_set_response))
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

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args([
            "send",
            "--to",
            "to@example.com",
            "--subject",
            "Hi",
            "--text",
            "Hello",
        ])
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
            .and_then(|d| d.get("draft"))
            .and_then(|e| e.get("emailId"))
            .and_then(|x| x.as_str()),
        Some("m1")
    );
    assert_eq!(
        v.get("data")
            .and_then(|d| d.get("submission"))
            .and_then(|s| s.get("id"))
            .and_then(|x| x.as_str()),
        Some("s1")
    );
}
