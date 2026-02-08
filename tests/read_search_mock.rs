use assert_cmd::Command;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn search_works_against_mock_jmap() {
    let server = MockServer::start().await;

    let session = json!({
        "apiUrl": format!("{}/jmap", server.uri()),
        "downloadUrl": format!("{}/download/{{accountId}}/{{blobId}}/{{name}}?type={{type}}", server.uri()),
        "uploadUrl": format!("{}/upload/{{accountId}}", server.uri()),
        "primaryAccounts": {
            "urn:ietf:params:jmap:mail": "A"
        }
    });

    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(session))
        .mount(&server)
        .await;

    let jmap_response = json!({
        "methodResponses": [
            ["Email/query", {
                "accountId": "A",
                "queryState": "s",
                "canCalculateChanges": false,
                "position": 0,
                "ids": ["m1"],
                "total": 1
            }, "q1"],
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
                    "preview": "preview",
                    "hasAttachment": false,
                    "mailboxIds": {"inbox": true},
                    "keywords": {"$seen": true}
                }],
                "notFound": []
            }, "g1"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(jmap_response))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_SESSION_URL", format!("{}/.well-known/jmap", server.uri()))
        .env("XIN_TOKEN", "test-token")
        .args(["search", "--max", "10", "--filter-json", "{}"])
        .output()
        .expect("run");

    assert!(output.status.success());

    let v: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(v.get("ok").and_then(|v| v.as_bool()), Some(true));

    let items = v
        .get("data")
        .and_then(|d| d.get("items"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].get("emailId").and_then(|v| v.as_str()), Some("m1"));
    assert_eq!(items[0].get("threadId").and_then(|v| v.as_str()), Some("t1"));
    assert_eq!(items[0].get("unread").and_then(|v| v.as_bool()), Some(false));
}
