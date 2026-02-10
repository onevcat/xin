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

fn find_single_method_args(
    body: &serde_json::Value,
    method_name: &str,
) -> Option<serde_json::Value> {
    let calls = body.get("methodCalls")?.as_array()?;
    for c in calls {
        let arr = c.as_array()?;
        if arr.len() < 2 {
            continue;
        }
        if arr[0].as_str()? == method_name {
            return Some(arr[1].clone());
        }
    }
    None
}

#[tokio::test]
async fn batch_modify_emits_patch_keys_for_mailbox_and_keyword() {
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
                    "id": "mb_inbox",
                    "name": "Inbox",
                    "role": "inbox"
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

    let email_set_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/set", {
                "accountId": "A",
                "oldState": "s",
                "newState": "s",
                "updated": {"m1": {}}
            }, "e0"]
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
        .args([
            "batch", "modify", "m1", "--add", "inbox", "--add", "$flagged",
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
        v.get("command").and_then(|v| v.as_str()),
        Some("batch.modify")
    );

    let requests = server.received_requests().await.expect("requests");
    let email_set = requests
        .iter()
        .find(|r| String::from_utf8_lossy(&r.body).contains("Email/set"))
        .expect("Email/set request");

    let body: serde_json::Value = serde_json::from_slice(&email_set.body).expect("email/set json");
    let args = find_single_method_args(&body, "Email/set").expect("Email/set args");
    let update = args
        .get("update")
        .and_then(|v| v.as_object())
        .expect("update object");

    let u = update
        .get("m1")
        .and_then(|v| v.as_object())
        .expect("m1 patch");

    // For non-replace mailbox updates, we expect patch keys like "mailboxIds/<id>" and
    // "keywords/<kw>".
    assert!(
        u.keys().any(|k| k == "mailboxIds/mb_inbox"),
        "expected mailboxIds/mb_inbox patch key, got keys: {:?}",
        u.keys().collect::<Vec<_>>()
    );
    assert!(
        u.keys().any(|k| k == "keywords/$flagged"),
        "expected keywords/$flagged patch key, got keys: {:?}",
        u.keys().collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn trash_whole_thread_uses_mailbox_ids_replacement_object() {
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
                    "id": "mb_trash",
                    "name": "Trash",
                    "role": "trash"
                }, {
                    "id": "mb_inbox",
                    "name": "Inbox",
                    "role": "inbox"
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

    let email_get_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/get", {
                "accountId": "A",
                "state": "s",
                "list": [{"id": "m1", "threadId": "t1"}],
                "notFound": []
            }, "g0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Email/get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(email_get_response))
        .mount(&server)
        .await;

    let thread_get_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Thread/get", {
                "accountId": "A",
                "state": "s",
                "list": [{"id": "t1", "emailIds": ["m1", "m2"]}],
                "notFound": []
            }, "t0"]
        ]
    });

    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Thread/get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(thread_get_response))
        .mount(&server)
        .await;

    let email_set_response = json!({
        "sessionState": "s",
        "methodResponses": [
            ["Email/set", {
                "accountId": "A",
                "oldState": "s",
                "newState": "s",
                "updated": {"m1": {}, "m2": {}}
            }, "e0"]
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
        .args(["trash", "--whole-thread", "m1"])
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
    assert_eq!(v.get("command").and_then(|v| v.as_str()), Some("trash"));

    let requests = server.received_requests().await.expect("requests");
    let email_set = requests
        .iter()
        .find(|r| String::from_utf8_lossy(&r.body).contains("Email/set"))
        .expect("Email/set request");

    let body: serde_json::Value = serde_json::from_slice(&email_set.body).expect("email/set json");
    let args = find_single_method_args(&body, "Email/set").expect("Email/set args");
    let update = args
        .get("update")
        .and_then(|v| v.as_object())
        .expect("update object");

    for id in ["m1", "m2"] {
        let u = update
            .get(id)
            .and_then(|v| v.as_object())
            .unwrap_or_else(|| panic!("missing patch for {id}"));

        assert!(
            u.contains_key("mailboxIds"),
            "expected mailboxIds replacement object for {id}, got keys: {:?}",
            u.keys().collect::<Vec<_>>()
        );

        // And it should not use patch keys mailboxIds/<id> when doing replacement.
        assert!(
            !u.keys().any(|k| k.starts_with("mailboxIds/")),
            "expected no mailboxIds/<id> patch keys for {id}, got keys: {:?}",
            u.keys().collect::<Vec<_>>()
        );

        let mbs = u
            .get("mailboxIds")
            .and_then(|v| v.as_object())
            .expect("mailboxIds object");
        assert_eq!(mbs.get("mb_trash").and_then(|v| v.as_bool()), Some(true));
    }
}
