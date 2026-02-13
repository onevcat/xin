use assert_cmd::Command;
use serde_json::json;
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use std::io::Write;
use tempfile::NamedTempFile;

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
async fn drafts_rewrite_subject_works_against_mock_jmap() {
    let server = MockServer::start().await;
    mount_session(&server).await;

    // Existing draft (full) to preserve fields.
    let full_get_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/get", {
                "accountId": "A",
                "state": "s",
                "list": [{
                    "id": "m1",
                    "threadId": "t1",
                    "subject": "Old",
                    "from": [{"name": "Me", "email": "me@example.com"}],
                    "to": [{"name": null, "email": "you@example.com"}],
                    "bodyStructure": {"type": "text/plain", "partId": "text"},
                    "bodyValues": {"text": {"value": "old"}},
                    "attachments": []
                }],
                "notFound": []
            }, "g0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/get"))
        .and(body_string_contains("bodyStructure"))
        .respond_with(ResponseTemplate::new(200).set_body_json(full_get_response))
        .expect(1)
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
        .expect(1)
        .mount(&server)
        .await;

    let create_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/set", {
                "accountId": "A",
                "oldState": "s",
                "newState": "s",
                "created": {
                    "c0": { "id": "m2", "threadId": "t2" }
                }
            }, "e0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/set"))
        .and(body_string_contains("\"create\""))
        .and(body_string_contains("\"subject\":\"Updated\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(create_response))
        .expect(1)
        .mount(&server)
        .await;

    let remove_response = json!({
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
        // update request removes Drafts mailbox membership
        .respond_with(ResponseTemplate::new(200).set_body_json(remove_response))
        .expect(1)
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["drafts", "rewrite", "m1", "--subject", "Updated"])
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
        Some("m2")
    );
    assert_eq!(
        v.get("data")
            .and_then(|d| d.get("replacedFrom"))
            .and_then(|x| x.as_str()),
        Some("m1")
    );
}

#[tokio::test]
async fn drafts_rewrite_keeps_existing_attachments_and_appends_new_ones() {
    let server = MockServer::start().await;
    mount_session(&server).await;

    // 1) Email/get(full) to preserve existing body + attachments.
    let full_get_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/get", {
                "accountId": "A",
                "state": "s",
                "list": [{
                    "id": "m1",
                    "threadId": "t1",
                    "from": [{"name": "Me", "email": "me@example.com"}],
                    "to": [{"name": null, "email": "you@example.com"}],
                    "bodyStructure": {
                        "type": "multipart/mixed",
                        "subParts": [
                            {"type": "text/plain", "partId": "text"},
                            {"type": "application/pdf", "blobId": "b_old", "name": "old.pdf", "disposition": "attachment"}
                        ]
                    },
                    "bodyValues": {"text": {"value": "old body"}},
                    "attachments": [
                        {"type": "application/pdf", "blobId": "b_old", "name": "old.pdf", "size": 3, "disposition": "attachment"}
                    ]
                }],
                "notFound": []
            }, "g0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/get"))
        .and(body_string_contains("bodyStructure"))
        .respond_with(ResponseTemplate::new(200).set_body_json(full_get_response))
        .expect(1)
        .mount(&server)
        .await;

    // 2) Upload new attachment.
    Mock::given(method("POST"))
        .and(path("/upload/A"))
        .and(header("content-type", "application/pdf"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "accountId": "A",
            "blobId": "b_new",
            "type": "application/pdf",
            "size": 3
        })))
        .expect(1)
        .mount(&server)
        .await;

    // 3) Mailbox/get to resolve Drafts mailbox id.
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
        .expect(1)
        .mount(&server)
        .await;

    // 4) Email/set(create) must include both old + new attachment blobIds.
    let create_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/set", {
                "accountId": "A",
                "oldState": "s",
                "newState": "s",
                "created": {"c0": {"id": "m2", "threadId": "t2"}}
            }, "e0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/set"))
        .and(body_string_contains("\"create\""))
        .and(body_string_contains("\"blobId\":\"b_old\""))
        .and(body_string_contains("\"blobId\":\"b_new\""))
        .and(body_string_contains("\"disposition\":\"attachment\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(create_response))
        .expect(1)
        .mount(&server)
        .await;

    // 5) Email/set(destroy) old draft.
    let destroy_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/set", {
                "accountId": "A",
                "oldState": "s",
                "newState": "s",
                "updated": {"m1": null}
            }, "d0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/set"))
        .and(body_string_contains("\"update\""))
        .and(body_string_contains("m1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(destroy_response))
        .expect(1)
        .mount(&server)
        .await;

    // 4) Email/get minimal to fetch threadId for output stability.
    let get_min_response = json!({
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
        .and(body_string_contains("\"properties\":[\"id\",\"threadId\"]"))
        .respond_with(ResponseTemplate::new(200).set_body_json(get_min_response))
        // rewrite no longer does a post-create Email/get; keep this mock as a guardrail.
        .expect(0)
        .mount(&server)
        .await;

    let mut f = NamedTempFile::new().expect("tmp");
    f.write_all(b"PDF").expect("write");
    let p = f.path().with_extension("pdf");
    std::fs::rename(f.path(), &p).expect("rename");

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["drafts", "rewrite", "m1", "--attach", p.to_str().unwrap()])
        .output()
        .expect("run");

    if !output.status.success() {
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        let requests = server.received_requests().await.expect("requests");
        for r in &requests {
            eprintln!(
                "request: {} {}\n{}",
                r.method,
                r.url,
                String::from_utf8_lossy(&r.body)
            );
        }
    }

    assert!(output.status.success());
}

#[tokio::test]
async fn drafts_rewrite_replace_attachments_drops_existing() {
    let server = MockServer::start().await;
    mount_session(&server).await;

    // Email/get(full) to discover existing attachments.
    let full_get_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/get", {
                "accountId": "A",
                "state": "s",
                "list": [{
                    "id": "m1",
                    "threadId": "t1",
                    "from": [{"name": "Me", "email": "me@example.com"}],
                    "to": [{"name": null, "email": "you@example.com"}],
                    "bodyStructure": {
                        "type": "multipart/mixed",
                        "subParts": [
                            {"type": "text/plain", "partId": "text"},
                            {"type": "application/pdf", "blobId": "b_old", "name": "old.pdf", "disposition": "attachment"}
                        ]
                    },
                    "bodyValues": {"text": {"value": "old body"}},
                    "attachments": [
                        {"type": "application/pdf", "blobId": "b_old", "name": "old.pdf", "size": 3, "disposition": "attachment"}
                    ]
                }],
                "notFound": []
            }, "g0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/get"))
        .and(body_string_contains("bodyStructure"))
        .respond_with(ResponseTemplate::new(200).set_body_json(full_get_response))
        .expect(1)
        .mount(&server)
        .await;

    // Upload new attachment.
    Mock::given(method("POST"))
        .and(path("/upload/A"))
        .and(header("content-type", "application/pdf"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "accountId": "A",
            "blobId": "b_new",
            "type": "application/pdf",
            "size": 3
        })))
        .expect(1)
        .mount(&server)
        .await;

    // 3) Mailbox/get to resolve Drafts mailbox id.
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
        .expect(1)
        .mount(&server)
        .await;

    // Guardrail: create must NOT include old attachment blobId.
    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/set"))
        .and(body_string_contains("\"create\""))
        .and(body_string_contains("\"blobId\":\"b_old\""))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let create_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/set", {
                "accountId": "A",
                "oldState": "s",
                "newState": "s",
                "created": {"c0": {"id": "m2", "threadId": "t2"}}
            }, "e0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/set"))
        .and(body_string_contains("\"create\""))
        .and(body_string_contains("\"blobId\":\"b_new\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(create_response))
        .expect(1)
        .mount(&server)
        .await;

    let destroy_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/set", {
                "accountId": "A",
                "oldState": "s",
                "newState": "s",
                "updated": {"m1": null}
            }, "d0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/set"))
        .and(body_string_contains("\"update\""))
        .and(body_string_contains("m1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(destroy_response))
        .expect(1)
        .mount(&server)
        .await;

    let mut f = NamedTempFile::new().expect("tmp");
    f.write_all(b"PDF").expect("write");
    let p = f.path().with_extension("pdf");
    std::fs::rename(f.path(), &p).expect("rename");

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args([
            "drafts",
            "rewrite",
            "m1",
            "--replace-attachments",
            "--attach",
            p.to_str().unwrap(),
        ])
        .output()
        .expect("run");

    assert!(output.status.success());
}

#[tokio::test]
async fn drafts_rewrite_clear_attachments_removes_all() {
    let server = MockServer::start().await;
    mount_session(&server).await;

    let full_get_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/get", {
                "accountId": "A",
                "state": "s",
                "list": [{
                    "id": "m1",
                    "threadId": "t1",
                    "from": [{"name": "Me", "email": "me@example.com"}],
                    "to": [{"name": null, "email": "you@example.com"}],
                    "bodyStructure": {
                        "type": "multipart/mixed",
                        "subParts": [
                            {"type": "text/plain", "partId": "text"},
                            {"type": "application/pdf", "blobId": "b_old", "name": "old.pdf", "disposition": "attachment"}
                        ]
                    },
                    "bodyValues": {"text": {"value": "old body"}},
                    "attachments": [
                        {"type": "application/pdf", "blobId": "b_old", "name": "old.pdf", "size": 3, "disposition": "attachment"}
                    ]
                }],
                "notFound": []
            }, "g0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/get"))
        .and(body_string_contains("bodyStructure"))
        .respond_with(ResponseTemplate::new(200).set_body_json(full_get_response))
        .expect(1)
        .mount(&server)
        .await;

    // No uploads should happen.
    Mock::given(method("POST"))
        .and(path("/upload/A"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    // 3) Mailbox/get to resolve Drafts mailbox id.
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
        .expect(1)
        .mount(&server)
        .await;

    // Guardrail: create must NOT include old attachment blobId.
    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/set"))
        .and(body_string_contains("\"create\""))
        .and(body_string_contains("\"blobId\":\"b_old\""))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let create_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/set", {
                "accountId": "A",
                "oldState": "s",
                "newState": "s",
                "created": {"c0": {"id": "m2", "threadId": "t2"}}
            }, "e0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/set"))
        .and(body_string_contains("\"create\""))
        .and(body_string_contains("text/plain"))
        .respond_with(ResponseTemplate::new(200).set_body_json(create_response))
        .expect(1)
        .mount(&server)
        .await;

    let destroy_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/set", {
                "accountId": "A",
                "oldState": "s",
                "newState": "s",
                "updated": {"m1": null}
            }, "d0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/set"))
        .and(body_string_contains("\"update\""))
        .and(body_string_contains("m1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(destroy_response))
        .expect(1)
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["drafts", "rewrite", "m1", "--clear-attachments"])
        .output()
        .expect("run");

    assert!(output.status.success());
}

#[tokio::test]
async fn drafts_rewrite_replace_attachments_requires_attach() {
    let server = MockServer::start().await;
    mount_session(&server).await;

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["drafts", "rewrite", "m1", "--replace-attachments"])
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
}

#[tokio::test]
async fn drafts_rewrite_clear_attachments_cannot_combine_with_attach() {
    let server = MockServer::start().await;
    mount_session(&server).await;

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args([
            "drafts",
            "rewrite",
            "m1",
            "--clear-attachments",
            "--attach",
            "a.pdf",
        ])
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
async fn drafts_delete_removes_membership_from_drafts_mailbox() {
    let server = MockServer::start().await;
    mount_session(&server).await;

    let mailbox_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Mailbox/get", {
                "accountId": "A",
                "state": "s",
                "list": [
                    {
                        "id": "mb1",
                        "name": "Drafts",
                        "role": "drafts"
                    },
                    {
                        "id": "mbTrash",
                        "name": "Trash",
                        "role": "trash"
                    }
                ],
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

    let email_set_response = json!({
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
        .respond_with(ResponseTemplate::new(200).set_body_json(email_set_response))
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

    // Ensure we didn't send a destroy.
    let requests = server.received_requests().await.expect("requests");
    let email_set = requests
        .iter()
        .find(|r| String::from_utf8_lossy(&r.body).contains("Email/set"))
        .expect("Email/set request");
    let body_str = String::from_utf8_lossy(&email_set.body);
    assert!(
        !body_str.contains("\"destroy\""),
        "expected no destroy in Email/set: {body_str}"
    );
    assert!(
        body_str.contains("\"mailboxIds/mb1\":null"),
        "expected mailboxIds/mb1 removal (null) in Email/set: {body_str}"
    );
    assert!(
        body_str.contains("mailboxIds/mbTrash") && body_str.contains("true"),
        "expected mailboxIds/mbTrash add in Email/set: {body_str}"
    );
    assert!(
        body_str.contains("\"keywords/$draft\":null"),
        "expected keywords/$draft removal (null) in Email/set: {body_str}"
    );
}

#[tokio::test]
async fn drafts_destroy_requires_force() {
    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .args(["drafts", "destroy", "m1"])
        .output()
        .expect("run");

    assert!(!output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(v.get("ok").and_then(|v| v.as_bool()), Some(false));
    assert_eq!(
        v.get("command").and_then(|v| v.as_str()),
        Some("drafts.destroy")
    );
    assert_eq!(
        v.get("error")
            .and_then(|e| e.get("kind"))
            .and_then(|k| k.as_str()),
        Some("xinUsageError")
    );
}

#[tokio::test]
async fn drafts_destroy_sends_email_set_destroy_when_forced() {
    let server = MockServer::start().await;
    mount_session(&server).await;

    let delete_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/set", {
                "accountId": "A",
                "oldState": "s",
                "newState": "s",
                "updated": {"m1": null}
            }, "d0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/set"))
        .and(body_string_contains("\"destroy\":[\"m1\"]"))
        .respond_with(ResponseTemplate::new(200).set_body_json(delete_response))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["--force", "drafts", "destroy", "m1"])
        .output()
        .expect("run");

    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(v.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        v.get("data")
            .and_then(|d| d.get("destroyed"))
            .and_then(|v| v.as_array())
            .and_then(|a| a.get(0))
            .and_then(|x| x.as_str()),
        Some("m1")
    );
}

#[tokio::test]
async fn drafts_list_works_with_drafts_name_fallback_when_role_missing() {
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
                    "name": "Drafts"
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
}

#[tokio::test]
async fn drafts_create_works_with_drafts_name_fallback_when_role_missing() {
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
                    "name": "Drafts"
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
