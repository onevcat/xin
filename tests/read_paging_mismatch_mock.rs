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
async fn page_token_mismatch_is_a_usage_error_and_does_not_hit_query_endpoint() {
    let server = MockServer::start().await;

    // Both runs need session discovery.
    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_session(&server)))
        .mount(&server)
        .await;

    // First run: returns exactly `limit` items so nextPage is produced.
    let jmap_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/query", {
                "accountId": "A",
                "queryState": "s",
                "canCalculateChanges": false,
                "position": 0,
                "ids": ["m1", "m2"]
            }, "q0"],
            ["Email/get", {
                "accountId": "A",
                "state": "s",
                "list": [
                    {
                        "id": "m1",
                        "threadId": "t1",
                        "receivedAt": "2026-02-08T00:00:00Z",
                        "subject": "Hi1",
                        "from": [{"name": "Alice", "email": "alice@example.com"}],
                        "to": [{"name": null, "email": "me@example.com"}],
                        "preview": "preview",
                        "hasAttachment": false,
                        "mailboxIds": {"inbox": true},
                        "keywords": {"$seen": true}
                    },
                    {
                        "id": "m2",
                        "threadId": "t2",
                        "receivedAt": "2026-02-08T00:00:00Z",
                        "subject": "Hi2",
                        "from": [{"name": "Bob", "email": "bob@example.com"}],
                        "to": [{"name": null, "email": "me@example.com"}],
                        "preview": "preview",
                        "hasAttachment": false,
                        "mailboxIds": {"inbox": true},
                        "keywords": {"$seen": true}
                    }
                ],
                "notFound": []
            }, "g0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("\"Email/query\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(jmap_response))
        .mount(&server)
        .await;

    let out1 = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["search", "--max", "2", "--filter-json", "{}"])
        .output()
        .expect("run");

    assert!(out1.status.success());

    let v1: serde_json::Value = serde_json::from_slice(&out1.stdout).expect("json");
    let tok = v1
        .get("meta")
        .and_then(|m| m.get("nextPage"))
        .and_then(|s| s.as_str())
        .expect("nextPage should exist")
        .to_string();

    // Second run: change args (limit) while reusing token => usage error.
    let out2 = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["search", "--max", "3", "--page", &tok, "--filter-json", "{}"])
        .output()
        .expect("run");

    assert!(!out2.status.success());

    let v2: serde_json::Value = serde_json::from_slice(&out2.stdout).expect("json");
    assert_eq!(v2.get("ok").and_then(|v| v.as_bool()), Some(false));
    assert_eq!(
        v2.get("error")
            .and_then(|e| e.get("kind"))
            .and_then(|k| k.as_str()),
        Some("xinUsageError")
    );

    let msg = v2
        .get("error")
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .unwrap_or("");
    assert!(msg.contains("page token does not match args"), "message={msg}");

    // Assert we only hit Email/query once total.
    let requests = server.received_requests().await.expect("requests");
    let query_posts = requests
        .iter()
        .filter(|r| String::from_utf8_lossy(&r.body).contains("Email/query"))
        .count();
    assert_eq!(query_posts, 1);
}
