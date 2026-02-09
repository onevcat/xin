use assert_cmd::Command;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::json;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[derive(Debug, Clone, serde::Deserialize)]
struct PageToken {
    position: i32,
    limit: usize,
    #[serde(rename = "collapseThreads")]
    collapse_threads: bool,
    #[serde(rename = "isAscending")]
    is_ascending: bool,
    #[allow(dead_code)]
    filter: serde_json::Value,
}

#[tokio::test]
async fn search_emits_next_page_even_when_total_is_missing() {
    let server = MockServer::start().await;

    let session = json!({
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
    });

    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(session))
        .mount(&server)
        .await;

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

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["search", "--max", "2", "--filter-json", "{}"])
        .output()
        .expect("run");

    assert!(output.status.success());

    let v: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    let tok = v
        .get("meta")
        .and_then(|m| m.get("nextPage"))
        .and_then(|s| s.as_str())
        .expect("nextPage should exist");

    let bytes = URL_SAFE_NO_PAD.decode(tok).expect("b64");
    let t: PageToken = serde_json::from_slice(&bytes).expect("token json");
    assert_eq!(t.position, 2);
    assert_eq!(t.limit, 2);
    assert_eq!(t.collapse_threads, true);
    assert_eq!(t.is_ascending, false);
}
