use assert_cmd::Command;
use serde_json::json;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn search_works_against_mock_jmap() {
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
                "ids": ["m1"],
                "total": 1
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
                    "preview": "preview",
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
        .args(["search", "--max", "10", "--filter-json", "{}"])
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

    let items = v
        .get("data")
        .and_then(|d| d.get("items"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].get("emailId").and_then(|v| v.as_str()), Some("m1"));
    assert_eq!(
        items[0].get("threadId").and_then(|v| v.as_str()),
        Some("t1")
    );
    assert_eq!(
        items[0].get("unread").and_then(|v| v.as_bool()),
        Some(false)
    );
}

#[tokio::test]
async fn search_sugar_compiles_to_filter_and_is_sent_to_server() {
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
                "ids": ["m1"],
                "total": 1
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
                    "preview": "preview",
                    "hasAttachment": false,
                    "mailboxIds": {"inbox": true},
                    "keywords": {"$seen": true}
                }],
                "notFound": []
            }, "s1"]
        ]
    });

    // Ensure the request body includes compiled filter fragments.
    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("\"from\":\"alice\""))
        .and(body_string_contains("\"notKeyword\":\"$seen\""))
        .and(body_string_contains("\"hasAttachment\":true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(jmap_response))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args([
            "search",
            "from:alice seen:false has:attachment",
            "--max",
            "10",
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
}

#[tokio::test]
async fn search_sugar_in_mailbox_triggers_mailbox_get_and_resolves_to_in_mailbox_id() {
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

    // First POST: Mailbox/get
    let mailbox_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Mailbox/get", {
                "accountId": "A",
                "state": "s",
                "list": [
                    {"id": "mbx_inbox", "name": "Inbox", "role": "inbox"}
                ],
                "notFound": []
            }, "m0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("\"Mailbox/get\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(mailbox_response))
        .mount(&server)
        .await;

    // Second POST: Email/query + Email/get
    let jmap_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/query", {
                "accountId": "A",
                "queryState": "s",
                "canCalculateChanges": false,
                "position": 0,
                "ids": ["m1"],
                "total": 1
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
                    "preview": "preview",
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
        .and(body_string_contains("\"Email/query\""))
        .and(body_string_contains("\"inMailbox\":\"mbx_inbox\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(jmap_response))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["search", "in:Inbox", "--max", "10"])
        .output()
        .expect("run");

    assert!(output.status.success());
}

#[tokio::test]
async fn search_sugar_or_group_and_seen_false_are_sent_to_server() {
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
                "ids": ["m1"],
                "total": 1
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
                    "preview": "preview",
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
        .and(body_string_contains("\"operator\":\"OR\""))
        .and(body_string_contains("\"from\":\"alice\""))
        .and(body_string_contains("\"from\":\"bob\""))
        .and(body_string_contains("\"notKeyword\":\"$seen\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(jmap_response))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args([
            "search",
            "or:(from:alice|from:bob) seen:false",
            "--max",
            "10",
        ])
        .output()
        .expect("run");

    assert!(output.status.success());
}

#[tokio::test]
async fn search_sugar_bare_term_maps_to_text_filter() {
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
                "ids": ["m1"],
                "total": 1
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
                    "preview": "preview",
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
        .and(body_string_contains("\"text\":\"meeting\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(jmap_response))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["search", "meeting", "--max", "10"])
        .output()
        .expect("run");

    assert!(output.status.success());
}

#[tokio::test]
async fn search_sugar_negated_in_trash_resolves_mailbox_and_wraps_not_operator() {
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

    // First POST: Mailbox/get
    let mailbox_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Mailbox/get", {
                "accountId": "A",
                "state": "s",
                "list": [
                    {"id": "mbx_trash", "name": "Trash", "role": "trash"}
                ],
                "notFound": []
            }, "m0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("\"Mailbox/get\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(mailbox_response))
        .mount(&server)
        .await;

    // Second POST: Email/query + Email/get
    let jmap_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/query", {
                "accountId": "A",
                "queryState": "s",
                "canCalculateChanges": false,
                "position": 0,
                "ids": ["m1"],
                "total": 1
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
                    "preview": "preview",
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
        .and(body_string_contains("\"operator\":\"NOT\""))
        .and(body_string_contains("\"inMailbox\":\"mbx_trash\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(jmap_response))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["search", "-in:trash", "--max", "10"])
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "xin failed. status={:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

async fn mount_minimal_jmap_session(server: &MockServer) {
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
        "downloadUrl": format!(
            "{}/download/{{accountId}}/{{blobId}}/{{name}}?type={{type}}",
            server.uri()
        ),
        "uploadUrl": format!("{}/upload/{{accountId}}", server.uri()),
        "eventSourceUrl": format!("{}/events", server.uri()),
        "state": "s"
    });

    Mock::given(method("GET"))
        .and(path("/.well-known/jmap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(session))
        .mount(server)
        .await;
}

#[tokio::test]
async fn search_sugar_parentheses_are_rejected_in_v0() {
    let server = MockServer::start().await;

    mount_minimal_jmap_session(&server).await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["search", "(from:alice)", "--max", "10"])
        .output()
        .expect("run");

    assert!(!output.status.success());

    let v: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(v.get("ok").and_then(|v| v.as_bool()), Some(false));
    assert_eq!(
        v.get("error")
            .and_then(|e| e.get("kind"))
            .and_then(|k| k.as_str()),
        Some("xinUsageError")
    );

    let msg = v
        .get("error")
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .unwrap_or("");
    assert!(msg.contains("parentheses"), "message={msg}");
    assert!(msg.contains("--filter-json"), "message={msg}");
}

#[tokio::test]
async fn search_sugar_group_negation_is_rejected_in_v0() {
    let server = MockServer::start().await;

    mount_minimal_jmap_session(&server).await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["search", "-(from:alice)", "--max", "10"])
        .output()
        .expect("run");

    assert!(!output.status.success());

    let v: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(v.get("ok").and_then(|v| v.as_bool()), Some(false));
    assert_eq!(
        v.get("error")
            .and_then(|e| e.get("kind"))
            .and_then(|k| k.as_str()),
        Some("xinUsageError")
    );

    let msg = v
        .get("error")
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .unwrap_or("");
    assert!(msg.contains("group negation"), "message={msg}");
    assert!(msg.contains("--filter-json"), "message={msg}");
}

#[tokio::test]
async fn search_sugar_nested_or_group_is_rejected_in_v0() {
    let server = MockServer::start().await;

    mount_minimal_jmap_session(&server).await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args([
            "search",
            "or:(or:(from:alice|from:bob)|from:carol)",
            "--max",
            "10",
        ])
        .output()
        .expect("run");

    assert!(!output.status.success());

    let v: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(v.get("ok").and_then(|v| v.as_bool()), Some(false));

    let msg = v
        .get("error")
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .unwrap_or("");
    assert!(msg.contains("nested or"), "message={msg}");
}
