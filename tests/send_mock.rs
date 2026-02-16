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

#[tokio::test]
async fn send_reply_infers_recipients_and_sets_threading_headers() {
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
        .expect(1)
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
        .expect(1)
        .mount(&server)
        .await;

    // Fetch original via Email/get
    // Simulate `header:*:asMessageIds` parsed tokens (no surrounding angle brackets).
    // This matches what Fastmail returns and is the tricky case for `xin reply`.
    let get_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/get", {
                "accountId": "A",
                "state": "s",
                "list": [{
                    "id": "orig1",
                    "threadId": "t1",
                    "messageId": ["orig@example.com"],
                    "references": ["r0@example.com"],
                    "from": [{"name": "Alice", "email": "alice@example.com"}],
                    "to": [{"name": null, "email": "me@example.com"}],
                    "cc": [{"name": null, "email": "cc1@example.com"}]
                }],
                "notFound": []
            }, "g0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(get_response))
        .expect(1)
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

    // Ensure reply threading is set using JMAP parsed header forms (not raw header text).
    // This avoids RFC 5322 formatting pitfalls (e.g. missing angle brackets) and matches Fastmail behavior.
    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(|req: &wiremock::Request| {
            // Match only the Email/set call.
            let Ok(body) = std::str::from_utf8(&req.body) else {
                return false;
            };
            let Ok(v) = serde_json::from_str::<serde_json::Value>(body) else {
                return false;
            };

            // JMAP request body should have methodCalls.
            let Some(method_calls) = v.get("methodCalls").and_then(|x| x.as_array()) else {
                return false;
            };

            // Find Email/set call.
            let email_set = method_calls.iter().find(|call| {
                call.as_array()
                    .and_then(|a| a.get(0))
                    .and_then(|m| m.as_str())
                    == Some("Email/set")
            });
            let Some(email_set) = email_set else {
                return false;
            };

            let Some(args) = email_set.as_array().and_then(|a| a.get(1)) else {
                return false;
            };
            let Some(create) = args.get("create").and_then(|c| c.as_object()) else {
                return false;
            };
            // Pick the first created Email object.
            let Some((_cid, obj)) = create.iter().next() else {
                return false;
            };
            let Some(obj) = obj.as_object() else {
                return false;
            };

            // We must set parsed header properties, not raw header strings.
            // header:In-Reply-To:asMessageIds -> ["orig@example.com"]
            // header:References:asMessageIds -> ["r0@example.com", "orig@example.com"]
            match (
                obj.get("header:In-Reply-To:asMessageIds"),
                obj.get("header:References:asMessageIds"),
            ) {
                (Some(in_reply_to), Some(references)) => {
                    in_reply_to == &serde_json::json!(["orig@example.com"])
                        && references
                            == &serde_json::json!(["r0@example.com", "orig@example.com"])
                        // Ensure we did NOT try to write raw header text with angle brackets.
                        && !body.contains("<orig@example.com>")
                }
                _ => false,
            }
        })
        .respond_with(ResponseTemplate::new(200).set_body_json(email_set_response))
        .expect(1)
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
        .expect(1)
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args([
            "reply",
            "orig1",
            "--subject",
            "Re: Hi",
            "--text",
            "Hello reply",
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
async fn send_unknown_identity_is_rejected_before_any_side_effects() {
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
        .expect(1)
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
        .expect(1)
        .mount(&server)
        .await;

    // Guardrail: when identity selection fails, xin must not send Email/set or EmailSubmission/set.
    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/set"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("EmailSubmission/set"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
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
            "--identity",
            "does-not-exist@example.com",
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

    let msg = v
        .get("error")
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .unwrap_or("");
    assert!(msg.contains("unknown identity"), "message={msg}");
    assert!(msg.contains("identities list"), "message={msg}");
}

#[tokio::test]
async fn send_html_and_attachment_uploads_and_sets_body_structure() {
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
                }, {
                    "id": "i2",
                    "name": "Other",
                    "email": "other@example.com"
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

    Mock::given(method("POST"))
        .and(path("/upload/A"))
        .and(header("content-type", "application/pdf"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "accountId": "A",
            "blobId": "b1",
            "type": "application/pdf",
            "size": 3
        })))
        .expect(1)
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
        .and(body_string_contains("multipart/mixed"))
        .and(body_string_contains("multipart/alternative"))
        .and(body_string_contains("\"blobId\":\"b1\""))
        .and(body_string_contains("other@example.com"))
        .and(body_string_contains("\"disposition\":\"attachment\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(email_set_response))
        .expect(1)
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

    let mut f = NamedTempFile::new().expect("tmp");
    f.write_all(b"PDF").expect("write");
    let p = f.path().with_extension("pdf");
    std::fs::rename(f.path(), &p).expect("rename");

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args([
            "send",
            "--to",
            "to@example.com",
            "--cc",
            "cc@example.com",
            "--bcc",
            "bcc@example.com",
            "--subject",
            "Hi",
            "--text",
            "Hello",
            "--body-html",
            "<b>Hello</b>",
            "--attach",
            p.to_str().unwrap(),
            "--identity",
            "other@example.com",
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
            .and_then(|d| d.get("uploaded"))
            .and_then(|u| u.as_array())
            .map(|a| a.len()),
        Some(1)
    );
}
