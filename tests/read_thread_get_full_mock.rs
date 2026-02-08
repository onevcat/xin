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
        "downloadUrl": format!(
            "{}/download/{{accountId}}/{{blobId}}/{{name}}?type={{type}}",
            server.uri()
        ),
        "uploadUrl": format!("{}/upload/{{accountId}}", server.uri()),
        "eventSourceUrl": format!("{}/events", server.uri()),
        "state": "s"
    })
}

#[tokio::test]
async fn thread_get_full_returns_per_email_full_items_and_warnings_are_prefixed() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_session(&server)))
        .mount(&server)
        .await;

    let jmap_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Thread/get", {
                "accountId": "A",
                "state": "s",
                "list": [{
                    "id": "t1",
                    "emailIds": ["m1", "m2"]
                }],
                "notFound": []
            }, "s0"],
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
                    "hasAttachment": true,
                    "mailboxIds": {"inbox": true},
                    "keywords": {"$seen": true},
                    "htmlBody": [{"partId": "h1", "type": "text/html", "size": 10}],
                    "bodyValues": {
                        "h1": {"value": "<p>hello</p>", "isTruncated": true, "isEncodingProblem": false}
                    },
                    "attachments": [{
                        "partId": "a1",
                        "blobId": "B1",
                        "size": 12,
                        "name": "a.txt",
                        "type": "text/plain",
                        "disposition": "attachment"
                    }]
                },{
                    "id": "m2",
                    "threadId": "t1",
                    "receivedAt": "2026-02-08T01:00:00Z",
                    "subject": "Re: Hi",
                    "from": [{"name": "Me", "email": "me@example.com"}],
                    "to": [{"name": "Alice", "email": "alice@example.com"}],
                    "cc": [],
                    "bcc": [],
                    "preview": "reply",
                    "hasAttachment": false,
                    "mailboxIds": {"inbox": true},
                    "keywords": {"$seen": true}
                }],
                "notFound": []
            }, "s1"]
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
        .args(["thread", "get", "t1", "--full"])
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
        v.pointer("/data/threadId").and_then(|x| x.as_str()),
        Some("t1")
    );

    // Full items are objects with {email, body, attachments, raw}
    assert_eq!(
        v.pointer("/data/emails/0/email/emailId")
            .and_then(|x| x.as_str()),
        Some("m1")
    );
    assert_eq!(
        v.pointer("/data/emails/0/body/htmlMeta/isTruncated")
            .and_then(|x| x.as_bool()),
        Some(true)
    );
    assert_eq!(
        v.pointer("/data/emails/0/attachments/0/blobId")
            .and_then(|x| x.as_str()),
        Some("B1")
    );

    let warnings = v
        .pointer("/meta/warnings")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();

    assert_eq!(warnings.len(), 1);
    let w0 = warnings[0].as_str().unwrap_or("");
    assert!(w0.contains("emailId=m1:"), "warning should be prefixed: {w0}");
    assert!(w0.contains("body.html truncated"), "warning should mention truncation: {w0}");
}
