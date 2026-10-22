use super::*;
use clap::Parser;
use std::collections::HashSet;

#[test]
fn every_upstream_operation_is_mapped_or_explicitly_excluded() {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/agent_barn/openapi.json"
    ))
    .unwrap();
    let coverage: Vec<Value> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/agent_barn/coverage.json"
    ))
    .unwrap();
    let actual: HashSet<_> = schema["paths"]
        .as_object()
        .unwrap()
        .iter()
        .flat_map(|(p, ms)| {
            ms.as_object().unwrap().iter().map(move |(m, o)| {
                (
                    m.to_uppercase(),
                    p.clone(),
                    o["operationId"].as_str().unwrap().to_owned(),
                )
            })
        })
        .collect();
    let declared: HashSet<_> = coverage
        .iter()
        .map(|e| {
            (
                e["method"].as_str().unwrap().to_owned(),
                e["path"].as_str().unwrap().to_owned(),
                e["operation_id"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    assert_eq!(actual, declared);
    for row in &coverage {
        if let Some(command) = row.get("command").and_then(Value::as_str) {
            let e = registry::endpoints()
                .iter()
                .find(|e| e.command == command)
                .unwrap();
            assert_eq!(e.path, row["path"]);
            assert_eq!(e.method, row["method"]);
            assert_eq!(e.operation_id, row["operation_id"]);
            let op = &schema["paths"][&e.path][e.method.to_lowercase()];
            let expected: HashSet<_> = op["parameters"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|p| p["in"] == "query")
                .map(|p| p["name"].as_str().unwrap())
                .collect();
            assert_eq!(
                e.query
                    .iter()
                    .map(|p| p.name.as_str())
                    .collect::<HashSet<_>>(),
                expected
            );
        } else {
            assert!(row["excluded"].as_str().is_some_and(|s| !s.is_empty()));
        }
    }
}
#[test]
fn every_endpoint_parses_and_builds_its_pinned_path() {
    for e in registry::endpoints() {
        let mut argv = vec!["aai-cli".to_string(), "agent-barn".into()];
        argv.extend(e.command.split_whitespace().map(str::to_owned));
        argv.extend(["--organization-id".into(), "org-123".into()]);
        let mut expected = e.path.replace("{organization_id}", "org-123");
        for p in &e.path_fields {
            if p.name == "organization_id" {
                continue;
            }
            let text = if let Some(choice) = p.choices.first() {
                choice.as_str()
            } else if p.kind == "integer" {
                "1"
            } else {
                "resource-123"
            };
            argv.extend([format!("--{}", p.name.replace('_', "-")), text.into()]);
            expected = expected.replace(&format!("{{{}}}", p.name), text);
        }
        for p in &e.query {
            if p.required {
                argv.extend([
                    format!("--{}", p.name.replace('_', "-")),
                    p.choices.first().cloned().unwrap_or_else(|| "1".into()),
                ]);
            }
        }
        if e.body_required {
            argv.extend(["--json".into(), "{}".into()]);
        }
        if e.mode != "json" {
            argv.extend(["--output".into(), "unused.bin".into()]);
        }
        let parsed = crate::cli::Cli::try_parse_from(&argv)
            .unwrap_or_else(|err| panic!("{}: {err}", e.command));
        let crate::cli::Command::AgentBarn(command) = parsed.command else {
            panic!()
        };
        let cli::Invocation::Endpoint(found, args) = command.invocation else {
            panic!()
        };
        assert_eq!(found.command, e.command);
        // Seed body only for required schemas; construction must not depend on a live server.
        {
            let (url, _) = prepare(
                "https://barn.test/api/v1",
                e,
                &args,
                command.organization.as_deref(),
            )
            .unwrap();
            assert_eq!(url.path(), format!("/api/v1{expected}"));
        }
    }
}
#[test]
fn base_url_rejects_unsafe_or_wrong_roots() {
    for base in [
        "https://barn.test",
        "ftp://barn.test/api/v1",
        "https://user:pass@barn.test/api/v1",
        "https://barn.test/api/v1?q=1",
        "https://barn.test/api/v1#x",
    ] {
        assert!(base_url(Some(base)).is_err(), "{base}");
    }
    assert_eq!(
        base_url(Some("https://barn.test/prefix/api/v1/")).unwrap(),
        "https://barn.test/prefix/api/v1"
    );
}

fn context(address: &str) -> Context {
    Context {
        profile: crate::config::Profile {
            base_url: Some(format!("http://{address}/api/v1")),
            token: Some("fake-personal-key".into()),
            auth_type: Some("bearer_token".into()),
            org: Some("profile-org".into()),
            // These must never initiate OAuth for Agent Barn.
            refresh_token: Some("not-used".into()),
            client_id: Some("not-used".into()),
            client_secret: Some("not-used".into()),
            ..Default::default()
        },
        secrets_file: "unused".into(),
        key_file: "unused".into(),
    }
}
async fn run(ctx: &Context, args: &[&str]) -> Result<Value, AppError> {
    let argv = [vec!["aai-cli", "agent-barn"], args.to_vec()].concat();
    let parsed = crate::cli::Cli::try_parse_from(&argv).unwrap();
    let crate::cli::Command::AgentBarn(command) = parsed.command else {
        panic!()
    };
    let response = dispatch(&ApiClient::new().unwrap(), ctx, command).await?;
    Ok(crate::pagination::annotate(
        response,
        &argv.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
    ))
}
#[tokio::test]
async fn existing_key_body_merge_org_override_and_encoded_ids_reach_http() {
    use crate::test_support::{json_response, request_body, serve};
    let (address, server) = serve(vec![json_response(
        r#"{"id":"agent-1","extra":"preserved"}"#,
    )]);
    let result = run(
        &context(&address),
        &[
            "agents",
            "update",
            "--organization-id",
            "override",
            "--agent-id",
            "id/with space",
            "--json",
            r#"{"name":"old","model":null}"#,
            "--name",
            "new",
        ],
    )
    .await
    .unwrap();
    assert_eq!(result["extra"], "preserved");
    let requests = server.join().unwrap();
    assert!(
        requests[0].starts_with("patch /api/v1/organizations/override/agents/id%2fwith%20space ")
    );
    assert!(requests[0].contains("authorization: bearer fake-personal-key"));
    assert_eq!(
        serde_json::from_str::<Value>(request_body(&requests[0])).unwrap(),
        json!({"name":"new","model":null})
    );
}
#[tokio::test]
async fn page_aggregation_and_within_page_resume_preserve_records_and_metadata() {
    use crate::test_support::{json_response, serve};
    let first = r#"{"page":1,"page_size":2,"total":4,"items":[{"id":"1"},{"id":"2"}],"cap":5}"#;
    let second = r#"{"page":2,"page_size":2,"total":4,"items":[{"id":"3"},{"id":"4"}],"cap":5}"#;
    let (address, server) = serve(vec![
        json_response(first),
        json_response(second),
        json_response(second),
    ]);
    let ctx = context(&address);
    let result = run(
        &ctx,
        &["agents", "list", "--page-size", "2", "--limit", "3"],
    )
    .await
    .unwrap();
    assert_eq!(result["items"], json!([{"id":"1"},{"id":"2"},{"id":"3"}]));
    assert_eq!(result["cap"], 5);
    assert!(result.get("_aai_collection").is_none());
    assert_eq!(result["_aai"]["pagination"]["status"], "more_available");
    let next = result["_aai"]["pagination"]["next_command"]
        .as_str()
        .unwrap();
    assert!(next.contains("--page 2"), "{next}");
    assert!(next.contains("--page-skip 1"), "{next}");
    let result = run(
        &ctx,
        &[
            "agents",
            "list",
            "--page-size",
            "2",
            "--limit",
            "3",
            "--page",
            "2",
            "--page-skip",
            "1",
        ],
    )
    .await
    .unwrap();
    assert_eq!(result["items"], json!([{"id":"4"}]));
    assert_eq!(result["_aai"]["pagination"]["status"], "complete");
    let requests = server.join().unwrap();
    assert!(requests[1].contains("page=2"));
}
#[tokio::test]
async fn conversation_cursor_is_composite_and_resets_page_skip() {
    use crate::test_support::{json_response, serve};
    let page = r#"{"threads":[{"root":{"id":"1"}},{"root":{"id":"2"}}],"has_more":true,"next_cursor":{"before_occurred_at":"2026-01-01T00:00:00Z","before_id":"next-id"}}"#;
    let (address, server) = serve(vec![json_response(page)]);
    let result = run(
        &context(&address),
        &[
            "agents",
            "conversations",
            "connections",
            "channels",
            "messages",
            "list",
            "--agent-id",
            "a",
            "--connection-id",
            "c",
            "--channel-id",
            "room",
            "--limit",
            "1",
            "--page-skip",
            "1",
        ],
    )
    .await
    .unwrap();
    let next = result["_aai"]["pagination"]["next_command"]
        .as_str()
        .unwrap();
    assert!(next.contains("--before-id next-id"), "{next}");
    assert!(next.contains("--before-occurred-at"));
    assert!(next.contains("--page-skip 0"));
    server.join().unwrap();
}
#[tokio::test]
async fn repeated_cursor_and_empty_page_with_continuation_fail() {
    use crate::test_support::{json_response, serve};
    for threads in [json!([{"root":{"id":"1"}}]), json!([])] {
        let body=json!({"threads":threads,"has_more":true,"next_cursor":{"before_occurred_at":"same","before_id":"same"}}).to_string();
        let repeats = if threads.as_array().unwrap().is_empty() {
            1
        } else {
            2
        };
        let (address, server) = serve((0..repeats).map(|_| json_response(&body)).collect());
        let error = run(
            &context(&address),
            &[
                "agents",
                "conversations",
                "connections",
                "channels",
                "messages",
                "list",
                "--agent-id",
                "a",
                "--connection-id",
                "c",
                "--channel-id",
                "room",
            ],
        )
        .await
        .unwrap_err();
        assert!(
            error.message.contains(if repeats == 1 {
                "empty page"
            } else {
                "repeated"
            }),
            "{error}"
        );
        server.join().unwrap();
    }
}
#[tokio::test]
async fn empty_zero_limit_capped_lists_and_snapshot_history() {
    use crate::test_support::{json_response, serve};
    let result = run(&context("127.0.0.1:1"), &["agents", "list", "--limit", "0"])
        .await
        .unwrap();
    assert_eq!(result["_aai"]["pagination"]["status"], "unknown");
    let (address, server) = serve(vec![
        json_response(r#"{"page":1,"page_size":100,"total":0,"items":[]}"#),
        json_response(r#"{"lines":["old"],"has_more":true,"next_snapshot_id":"older"}"#),
        json_response(r#"{"lines":["older"],"has_more":false,"next_snapshot_id":null}"#),
        json_response(&json!(vec![json!({"thread_id":"t"}); 100]).to_string()),
    ]);
    let ctx = context(&address);
    let empty = run(&ctx, &["agents", "list"]).await.unwrap();
    assert_eq!(empty["_aai"]["pagination"]["status"], "complete");
    let history = run(
        &ctx,
        &["agents", "logs", "history", "list", "--agent-id", "a"],
    )
    .await
    .unwrap();
    assert_eq!(history["lines"], json!(["old", "older"]));
    let threads = run(
        &ctx,
        &["agents", "web-chat", "threads", "list", "--agent-id", "a"],
    )
    .await
    .unwrap();
    assert_eq!(threads["_aai"]["pagination"]["status"], "unknown");
    let requests = server.join().unwrap();
    assert!(requests[2].contains("snapshot_id=older"));
}
#[tokio::test]
async fn provider_errors_remain_structured_and_writes_are_not_retried() {
    use crate::test_support::serve;
    for status in [401, 403, 404, 409, 422, 503] {
        let body = r#"{"detail":"provider explanation"}"#;
        let response=format!("HTTP/1.1 {status} Error\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).into_bytes();
        let (address, server) = serve(vec![response]);
        let error = run(&context(&address), &["agents", "start", "--agent-id", "a"])
            .await
            .unwrap_err();
        assert_eq!(error.status, Some(status));
        assert_eq!(error.details.unwrap()["detail"], "provider explanation");
        assert_eq!(server.join().unwrap().len(), 1);
    }
}
#[tokio::test]
async fn missing_org_invalid_auth_and_generic_escapes_fail_before_request() {
    let mut ctx = context("127.0.0.1:1");
    ctx.profile.org = None;
    assert!(run(&ctx, &["agents", "list"])
        .await
        .unwrap_err()
        .message
        .contains("organization-id"));
    for path in [
        "https://evil.test/x",
        "../outside",
        "%2e%2e/outside",
        "//evil.test/x",
    ] {
        assert!(run(&ctx, &["request", "get", path]).await.unwrap_err().code == "invalid_input");
    }
    assert_eq!(
        run(&ctx, &["request", "post", "/health"])
            .await
            .unwrap_err()
            .code,
        "invalid_input"
    );
    ctx.profile.auth_type = Some("token_url".into());
    assert_eq!(
        run(&ctx, &["auth", "context"]).await.unwrap_err().code,
        "auth_error"
    );
}

#[tokio::test]
async fn download_preserves_bytes_refuses_overwrite_and_reports_interruption() {
    use crate::test_support::serve;
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("package.zip");
    let output = output.to_str().unwrap();
    let mut response=b"HTTP/1.1 200 OK\r\nContent-Type: application/zip\r\nContent-Length: 5\r\nConnection: close\r\n\r\n".to_vec();
    response.extend_from_slice(&[0, 255, 1, 2, 3]);
    let (address, server) = serve(vec![response]);
    let args = [
        "agents",
        "connections",
        "app-package",
        "--agent-id",
        "a",
        "--connection-id",
        "c",
        "--output",
        output,
    ];
    let result = run(&context(&address), &args).await.unwrap();
    assert_eq!(result["bytes"], 5);
    assert_eq!(result["content_type"], "application/zip");
    assert_eq!(std::fs::read(output).unwrap(), [0, 255, 1, 2, 3]);
    server.join().unwrap();
    let error = run(&context("127.0.0.1:1"), &args).await.unwrap_err();
    assert_eq!(error.code, "invalid_input");
    assert_eq!(std::fs::read(output).unwrap(), [0, 255, 1, 2, 3]);
    let partial = dir.path().join("partial.zip");
    let (address,server)=serve(vec![b"HTTP/1.1 200 OK\r\nContent-Type: application/zip\r\nContent-Length: 100\r\nConnection: close\r\n\r\npartial".to_vec()]);
    let error = run(
        &context(&address),
        &[
            "agents",
            "connections",
            "app-package",
            "--agent-id",
            "a",
            "--connection-id",
            "c",
            "--output",
            partial.to_str().unwrap(),
        ],
    )
    .await
    .unwrap_err();
    assert_eq!(
        error.details.unwrap()["partial_file"],
        partial.to_str().unwrap()
    );
    assert!(partial.exists());
    server.join().unwrap();
}
#[tokio::test]
async fn capture_rejects_redirects_without_forwarding_credentials() {
    use crate::test_support::serve;
    let dir = tempfile::tempdir().unwrap();
    for stream in [false, true] {
        let output = dir
            .path()
            .join(if stream { "stream.sse" } else { "file.zip" });
        let (address,server)=serve(vec![b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/leak\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec()]);
        let args = if stream {
            vec![
                "agents",
                "logs",
                "stream",
                "--agent-id",
                "a",
                "--output",
                output.to_str().unwrap(),
            ]
        } else {
            vec![
                "agents",
                "connections",
                "app-package",
                "--agent-id",
                "a",
                "--connection-id",
                "c",
                "--output",
                output.to_str().unwrap(),
            ]
        };
        let error = run(&context(&address), &args).await.unwrap_err();
        assert_eq!(error.status, Some(302));
        assert!(!output.exists());
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].contains("authorization: bearer fake-personal-key"));
    }
}
#[tokio::test]
async fn stream_captures_split_frames_and_stops_at_deadline() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
        time::Duration,
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut req = [0; 8192];
        let n = stream.read(&mut req).unwrap();
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream; charset=utf-8\r\nTransfer-Encoding: chunked\r\n\r\n").unwrap();
        for chunk in [b"data: hel".as_slice(), b"lo\n\n", b": ping\n\n"] {
            write!(stream, "{:x}\r\n", chunk.len()).unwrap();
            stream.write_all(chunk).unwrap();
            stream.write_all(b"\r\n").unwrap();
            thread::sleep(Duration::from_millis(10));
        }
        thread::sleep(Duration::from_millis(1200));
        String::from_utf8_lossy(&req[..n]).to_string()
    });
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("events.sse");
    let result = run(
        &context(&address),
        &[
            "agents",
            "logs",
            "stream",
            "--agent-id",
            "a",
            "--output",
            output.to_str().unwrap(),
            "--duration-seconds",
            "1",
        ],
    )
    .await
    .unwrap();
    assert_eq!(result["termination_reason"], "duration_elapsed");
    assert_eq!(
        std::fs::read(&output).unwrap(),
        b"data: hello\n\n: ping\n\n"
    );
    assert_eq!(result["bytes"], 21);
    assert!(server.join().unwrap().contains("accept: text/event-stream"));
}
#[tokio::test]
async fn stream_disconnect_retains_partial_file_and_structured_error() {
    use crate::test_support::serve;
    let body = b"data: ended\n\n";
    let mut response=format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len()).into_bytes();
    response.extend_from_slice(body);
    let (address, server) = serve(vec![response]);
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("events.sse");
    let error = run(
        &context(&address),
        &[
            "agents",
            "web-chat",
            "stream",
            "--agent-id",
            "a",
            "--output",
            output.to_str().unwrap(),
        ],
    )
    .await
    .unwrap_err();
    assert!(error.message.contains("disconnected"));
    assert_eq!(
        error.details.unwrap()["partial_file"],
        output.to_str().unwrap()
    );
    assert_eq!(std::fs::read(&output).unwrap(), body);
    server.join().unwrap();
}

#[tokio::test]
async fn capped_members_and_initial_chat_do_not_claim_complete_history() {
    use crate::test_support::{json_response, serve};
    let members = json!((0..200)
        .map(|n| json!({"user_id":n.to_string()}))
        .collect::<Vec<_>>());
    let messages = json!((0..500)
        .map(|n| json!({"id":n.to_string()}))
        .collect::<Vec<_>>());
    let (address, server) = serve(vec![
        json_response(&members.to_string()),
        json_response(&messages.to_string()),
        json_response(&messages.to_string()),
        json_response("[]"),
    ]);
    let ctx = context(&address);
    let result = run(
        &ctx,
        &["organizations", "members", "list", "--limit", "500"],
    )
    .await
    .unwrap();
    assert_eq!(result["_aai"]["pagination"]["status"], "unknown");
    let result = run(
        &ctx,
        &[
            "agents",
            "web-chat",
            "messages",
            "list",
            "--agent-id",
            "a",
            "--limit",
            "500",
        ],
    )
    .await
    .unwrap();
    assert_eq!(result["_aai"]["pagination"]["status"], "unknown");
    let result = run(
        &ctx,
        &[
            "agents",
            "web-chat",
            "messages",
            "list",
            "--agent-id",
            "a",
            "--after-id",
            "seed",
            "--limit",
            "501",
        ],
    )
    .await
    .unwrap();
    assert_eq!(result["results"].as_array().unwrap().len(), 500);
    assert_eq!(result["_aai"]["pagination"]["status"], "complete");
    let requests = server.join().unwrap();
    assert!(requests[0].contains("limit=200"));
    assert!(requests[3].contains("after_id=499"));
}
