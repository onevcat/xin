use assert_cmd::Command;
use serde_json::json;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Fastmail is strict about JMAP capabilities: methods under the submission capability
/// (e.g. Identity/get) must include `urn:ietf:params:jmap:submission` in the request `using` list.
#[tokio::test]
async fn identities_list_includes_submission_capability_in_using() {
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
            ["Identity/get", {
                "accountId": "A",
                "state": "s",
                "list": [{
                    "id": "I1",
                    "name": "Me",
                    "email": "me@example.com"
                }],
                "notFound": []
            }, "s0"]
        ]
    });

    // Strict matcher: Identity/get must include submission capability in `using`.
    Mock::given(method("POST"))
        .and(path("/jmap"))
        .and(body_string_contains("Identity/get"))
        .and(body_string_contains("urn:ietf:params:jmap:submission"))
        .respond_with(ResponseTemplate::new(200).set_body_json(jmap_response))
        .mount(&server)
        .await;

    let output = Command::new(assert_cmd::cargo::cargo_bin!("xin"))
        .env("XIN_BASE_URL", server.uri())
        .env("XIN_TOKEN", "test-token")
        .args(["identities", "list"])
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

    let n = v
        .get("data")
        .and_then(|d| d.get("identities"))
        .and_then(|x| x.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    assert_eq!(n, 1);
}
