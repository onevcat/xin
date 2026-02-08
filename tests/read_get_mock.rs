use assert_cmd::Command;
use serde_json::json;
use wiremock::matchers::{method, path};
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
async fn get_metadata_works_against_mock_jmap() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_session(&server)))
        .mount(&server)
        .await;

    let jmap_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/get", {
                "accountId": "A",
                "state": "s",
                "list": [{
                    "id": "m1",
                    "threadId": "t1",
                    "receivedAt": "2026-02-08T00:00:00Z",
                    "subject": "Hi",
                    "from": [{"name": "Alice", "email": "alice@example.com"}],
                    "to": [{"name": null, "email": "me@example.com"}],
                    "cc": [],
                    "bcc": [],
                    "preview": "preview",
                    "hasAttachment": false,
                    "mailboxIds": {"inbox": true},
                    "keywords": {"$seen": true}
                }],
                "notFound": []
            }, "s0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(jmap_response))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["get", "m1", "--format", "metadata"])
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
            .and_then(|d| d.get("email"))
            .and_then(|e| e.get("emailId"))
            .and_then(|x| x.as_str()),
        Some("m1")
    );
}

#[tokio::test]
async fn get_metadata_headers_dict_works_against_mock_jmap() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_session(&server)))
        .mount(&server)
        .await;

    let jmap_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/get", {
                "accountId": "A",
                "state": "s",
                "list": [{
                    "id": "m1",
                    "threadId": "t1",
                    "receivedAt": "2026-02-08T00:00:00Z",
                    "subject": "Hi",
                    "from": [{"name": "Alice", "email": "alice@example.com"}],
                    "to": [{"name": null, "email": "me@example.com"}],
                    "cc": [],
                    "bcc": [],
                    "preview": "preview",
                    "hasAttachment": false,
                    "mailboxIds": {"inbox": true},
                    "keywords": {"$seen": true},

                    "messageId": ["<m1@example.com>"],
                    "sentAt": "2026-02-08T00:00:00Z",
                    "replyTo": [{"name": null, "email": "reply@example.com"}],

                    "header:Received:asText:all": ["from A", "from B"],
                    "header:DKIM-Signature:asText:all": ["v=1; a=rsa-sha256; ..."],
                    "header:X-Custom:asText": "hello"
                }],
                "notFound": []
            }, "s0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(jmap_response))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args([
            "get",
            "m1",
            "--format",
            "metadata",
            "--headers",
            "received,dkim-signature,message-id,date,reply-to,x-custom",
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

    let headers = v
        .get("data")
        .and_then(|d| d.get("email"))
        .and_then(|e| e.get("headers"))
        .expect("headers dict");

    assert_eq!(headers.get("message-id").and_then(|x| x.as_str()), Some("<m1@example.com>"));
    assert_eq!(
        headers.get("date").and_then(|x| x.as_str()),
        Some("2026-02-08T00:00:00+00:00")
    );
    assert_eq!(
        headers.get("received").and_then(|x| x.as_array()).map(|a| a.len()),
        Some(2)
    );
    assert_eq!(headers.get("x-custom").and_then(|x| x.as_str()), Some("hello"));
}
