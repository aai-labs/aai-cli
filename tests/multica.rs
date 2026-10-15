//! Offline HTTP contract checks for Multica application API revision
//! 83527872634824012dd26bb183aa216fab631c05. No real credentials or provider calls.
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    net::TcpListener,
    process::{Command, Output},
    thread,
    time::{Duration, Instant},
};

struct Reply {
    method: String,
    path: String,
    response: Value,
    status: u16,
    headers: Vec<(String, String)>,
    query: Vec<(String, String)>,
    body: Option<Value>,
    workspace: String,
}
impl Reply {
    fn new(method: &str, path: &str, response: Value) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            response,
            status: 200,
            headers: Vec::new(),
            query: Vec::new(),
            body: None,
            workspace: "demo".into(),
        }
    }
    fn body(mut self, body: Value) -> Self {
        self.body = Some(body);
        self
    }
    fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
    fn query(mut self, key: &str, value: &str) -> Self {
        self.query.push((key.into(), value.into()));
        self
    }
}
struct Server {
    dir: tempfile::TempDir,
    worker: thread::JoinHandle<()>,
}
impl Server {
    fn new(replies: Vec<Reply>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();
        let worker = thread::spawn(move || {
            for reply in replies {
                let deadline = Instant::now() + Duration::from_secs(20);
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(e)
                            if e.kind() == std::io::ErrorKind::WouldBlock
                                && Instant::now() < deadline =>
                        {
                            thread::sleep(Duration::from_millis(5))
                        }
                        Err(e) => panic!("no request for {} {}: {e}", reply.method, reply.path),
                    }
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut bytes = Vec::new();
                let (head, body) = loop {
                    let mut buf = [0; 8192];
                    let n = stream.read(&mut buf).unwrap();
                    assert_ne!(n, 0, "unexpected EOF");
                    bytes.extend_from_slice(&buf[..n]);
                    if let Some(pos) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        let head = String::from_utf8(bytes[..pos].to_vec()).unwrap();
                        let length = head
                            .lines()
                            .find_map(|l| {
                                l.to_ascii_lowercase()
                                    .strip_prefix("content-length: ")
                                    .map(|s| s.parse::<usize>().unwrap())
                            })
                            .unwrap_or(0);
                        if bytes.len() >= pos + 4 + length {
                            break (head, bytes[pos + 4..pos + 4 + length].to_vec());
                        }
                    }
                };
                let mut line = head.lines().next().unwrap().split_whitespace();
                assert_eq!(line.next(), Some(reply.method.as_str()));
                let url =
                    reqwest::Url::parse(&format!("http://local{}", line.next().unwrap())).unwrap();
                assert_eq!(url.path(), reply.path);
                let lower = head.to_ascii_lowercase();
                assert!(
                    lower.contains("authorization: bearer fake-multica-token\r\n"),
                    "{head}"
                );
                let header = if reply.workspace.len() == 36 {
                    "x-workspace-id"
                } else {
                    "x-workspace-slug"
                };
                assert!(
                    lower.contains(&format!("{header}: {}", reply.workspace)),
                    "{head}"
                );
                for (k, v) in reply.query {
                    assert!(
                        url.query_pairs().any(|(key, value)| key == k && value == v),
                        "{url} missing {k}={v}"
                    );
                }
                if let Some(expected) = reply.body {
                    assert_eq!(serde_json::from_slice::<Value>(&body).unwrap(), expected);
                }
                let body = reply.response.to_string();
                let headers = reply
                    .headers
                    .iter()
                    .map(|(k, v)| format!("{k}: {v}\r\n"))
                    .collect::<String>();
                write!(stream,"HTTP/1.1 {} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{}\r\n{}",reply.status,body.len(),headers,body).unwrap();
            }
        });
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("config.toml"),format!("[profiles.multica]\nprovider = \"multica\"\nauth_type = \"bearer_token\"\nbase_url = \"http://{address}/api\"\nworkspace = \"demo\"\ntoken = \"fake-multica-token\"\n")).unwrap();
        Self { dir, worker }
    }
    fn output(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_aai-cli"))
            .env_clear()
            .args([
                "--config",
                self.dir.path().join("config.toml").to_str().unwrap(),
                "--profile",
                "multica",
                "multica",
            ])
            .args(args)
            .output()
            .unwrap()
    }
    fn run(&self, args: &[&str]) -> Value {
        let out = self.output(args);
        assert!(
            out.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap()
    }
    fn finish(self) {
        self.worker.join().unwrap();
    }
}

#[test]
fn multica_all_dedicated_commands_match_upstream_routes() {
    let fixtures: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/multica/routes.json")).unwrap();
    let replies = fixtures
        .iter()
        .map(|f| {
            Reply::new(
                f["method"].as_str().unwrap(),
                f["path"].as_str().unwrap(),
                f["response"].clone(),
            )
        })
        .collect();
    let server = Server::new(replies);
    for fixture in fixtures {
        let args = fixture["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>();
        let result = server.run(&args);
        assert!(result.get("_aai").is_some(), "{args:?}");
    }
    server.finish();
}

#[test]
fn multica_inbox_issue_reply_and_explicit_mark_read_workflow() {
    let item = json!({"id":"notification-1","workspace_id":"workspace-1","recipient_type":"member","recipient_id":"member-1","type":"comment","severity":"info","issue_id":"issue-1","title":"Progress","body":"Ready for review","read":false,"archived":false,"created_at":"2026-10-15T08:00:00Z","details":{"comment_id":"root-1"}});
    let issue = json!({"id":"issue-1","identifier":"DEMO-12","title":"Ship feature","status":"in_progress","assignee_type":"agent","assignee_id":"agent-1","revision":2});
    let comment = json!({"id":"root-1","issue_id":"issue-1","author_type":"agent","author_id":"agent-1","content":"Ready for review","type":"comment","parent_id":null,"created_at":"2026-10-15T08:00:00Z","revision":1});
    let mut read = item.clone();
    read["read"] = json!(true);
    let server = Server::new(vec![
        Reply::new("GET", "/api/inbox", json!([item])),
        Reply::new("GET", "/api/issues/issue-1", issue.clone()),
        Reply::new("GET", "/api/issues/issue-1/comments", json!([comment]))
            .query("thread", "root-1")
            .query("since", "2026-10-15T07:00:00Z"),
        Reply::new("PUT", "/api/issues/issue-1", issue)
            .body(json!({"status":"done","assignee_type":"squad","assignee_id":"squad-1"})),
        Reply::new(
            "POST",
            "/api/issues/issue-1/comments",
            json!({"id":"reply-1","content":"Reviewed"}),
        )
        .body(json!({"content":"Reviewed","parent_id":"root-1"})),
        Reply::new("POST", "/api/inbox/notification-1/read", read).body(json!({})),
    ]);
    assert_eq!(server.run(&["inbox", "list"])["results"][0]["read"], false);
    server.run(&["issues", "get", "--issue-id", "issue-1"]);
    server.run(&[
        "issues",
        "comments",
        "list",
        "--issue-id",
        "issue-1",
        "--thread",
        "root-1",
        "--since",
        "2026-10-15T07:00:00Z",
    ]);
    server.run(&[
        "issues",
        "update",
        "--issue-id",
        "issue-1",
        "--status",
        "done",
        "--assignee-type",
        "squad",
        "--assignee-id",
        "squad-1",
    ]);
    server.run(&[
        "issues",
        "comments",
        "create",
        "--issue-id",
        "issue-1",
        "--content",
        "Reviewed",
        "--parent-id",
        "root-1",
    ]);
    assert_eq!(
        server.run(&["inbox", "read", "--inbox-id", "notification-1"])["read"],
        true
    );
    server.finish();
}

#[test]
fn multica_pagination_preserves_envelopes_and_continuations_without_skipping_rows() {
    let server = Server::new(vec![
        Reply::new(
            "GET",
            "/api/issues",
            json!({"issues":[{"id":"1"},{"id":"2"}],"total":3,"extra":"preserved"}),
        )
        .query("offset", "0")
        .query("limit", "100"),
        Reply::new(
            "GET",
            "/api/issues",
            json!({"issues":[{"id":"3"}],"total":3,"extra":"preserved"}),
        )
        .query("offset", "2"),
        Reply::new(
            "GET",
            "/api/inbox/archived/page",
            json!({"items":[{"id":"n1"},{"id":"n2"}],"next_cursor":"next","has_more":true}),
        ),
        Reply::new(
            "GET",
            "/api/inbox/archived/page",
            json!({"items":[{"id":"n1"},{"id":"n2"}],"next_cursor":"next","has_more":true}),
        ),
    ]);
    let all = server.run(&["issues", "list", "--offset", "0", "--limit", "3"]);
    assert_eq!(all["issues"].as_array().unwrap().len(), 3);
    assert_eq!(all["extra"], "preserved");
    assert_eq!(all["_aai"]["pagination"]["status"], "complete");
    let first = server.run(&["inbox", "archived", "page", "--limit", "1"]);
    let next = first["_aai"]["pagination"]["next_command"]
        .as_str()
        .unwrap();
    assert!(next.contains("--page-skip 1"));
    assert!(!next.contains("--cursor next"));
    let second = server.run(&[
        "inbox",
        "archived",
        "page",
        "--limit",
        "1",
        "--page-skip",
        "1",
    ]);
    assert_eq!(second["items"][0]["id"], "n2");
    assert!(second["_aai"]["pagination"]["next_command"]
        .as_str()
        .unwrap()
        .contains("--cursor next"));
    server.finish();
}

#[test]
fn multica_comment_and_agent_header_cursors_and_truncation_are_visible() {
    let first = json!({"id":"c2","created_at":"2026-10-15T08:00:00Z","content":"new"});
    let older = json!({"id":"c1","created_at":"2026-10-15T07:00:00Z","content":"old"});
    let server = Server::new(vec![
        Reply::new("GET", "/api/issues/i/comments", json!([first]))
            .header("X-Multica-Next-Before", "2026-10-15T08:00:00Z")
            .header("X-Multica-Next-Before-Id", "c2"),
        Reply::new("GET", "/api/issues/i/comments", json!([older])).query("before_id", "c2"),
        Reply::new("GET", "/api/agents/a/tasks", json!([{"id":"t1"}]))
            .header("X-Agent-Tasks-Next-Cursor", "opaque:cursor"),
        Reply::new("GET", "/api/issues/i/timeline", json!([]))
            .header("X-Timeline-Truncated", "activity"),
    ]);
    let comments = server.run(&[
        "issues",
        "comments",
        "list",
        "--issue-id",
        "i",
        "--recent",
        "1",
        "--limit",
        "5",
    ]);
    assert_eq!(comments["results"][0]["id"], "c1");
    assert_eq!(comments["_aai"]["pagination"]["status"], "complete");
    let tasks = server.run(&["agents", "tasks", "--agent-id", "a", "--limit", "1"]);
    assert!(tasks["_aai"]["pagination"]["next_command"]
        .as_str()
        .unwrap()
        .contains("--before opaque:cursor"));
    let timeline = server.run(&["issues", "timeline", "--issue-id", "i"]);
    assert_eq!(timeline["_aai"]["pagination"]["status"], "unknown");
    assert!(timeline["_aai"]["pagination"]["has_more"].is_null());
    server.finish();
}

#[test]
fn multica_write_merge_nulls_assignment_and_workspace_override() {
    let mut reply = Reply::new("PUT", "/api/issues/i", json!({"id":"i"})).body(
        json!({"title":"typed","description":null,"assignee_type":"member","assignee_id":"m"}),
    );
    reply.workspace = "12345678-1234-1234-1234-123456789abc".into();
    let server = Server::new(vec![
        reply,
        Reply::new("DELETE", "/api/squads/s/members", json!({}))
            .body(json!({"member_type":"agent","member_id":"a"})),
        Reply::new("PUT", "/api/agents/a", json!({"id":"a"})).body(
            json!({"instructions":"new instructions","max_concurrent_tasks":2,"thinking_level":""}),
        ),
        Reply::new(
            "PUT",
            "/api/agents/a/env",
            json!({"custom_env":{"KEY":"****"}}),
        )
        .body(json!({"custom_env":{"KEY":"fake-env-value"}})),
    ]);
    server.run(&[
        "issues",
        "update",
        "--issue-id",
        "i",
        "--workspace",
        "12345678-1234-1234-1234-123456789abc",
        "--json",
        r#"{"title":"json","description":null}"#,
        "--title",
        "typed",
        "--assignee-type",
        "member",
        "--assignee-id",
        "m",
    ]);
    server.run(&[
        "squads",
        "members",
        "remove",
        "--squad-id",
        "s",
        "--member-type",
        "agent",
        "--member-id",
        "a",
    ]);
    server.run(&[
        "agents",
        "update",
        "--agent-id",
        "a",
        "--instructions",
        "new instructions",
        "--max-concurrent-tasks",
        "2",
        "--thinking-level",
        "",
    ]);
    server.run(&[
        "agents",
        "env",
        "set",
        "--agent-id",
        "a",
        "--json",
        r#"{"custom_env":{"KEY":"fake-env-value"}}"#,
    ]);
    server.finish();
}

#[test]
fn multica_raw_requests_preserve_auth_scope_headers_and_write_guard() {
    let server = Server::new(vec![
        Reply::new("GET", "/api/issues/i/comments", json!([]))
            .header("X-Multica-Next-Before", "time")
            .header("X-Multica-Next-Before-Id", "id"),
        Reply::new("POST", "/api/issues", json!({"id":"i"})).body(json!({"title":"raw"})),
    ]);
    let invalid = server.output(&["request", "post", "issues"]);
    assert!(!invalid.status.success());
    assert!(invalid.stdout.is_empty());
    assert!(serde_json::from_slice::<Value>(&invalid.stderr).is_ok());
    let result = server.run(&["request", "get", "issues/i/comments", "--query", "recent=1"]);
    let next = result["_aai"]["pagination"]["next_command"]
        .as_str()
        .unwrap();
    assert!(next.contains("before_id=id"));
    assert!(result.get("_aai_response_headers").is_none());
    server.run(&[
        "request",
        "post",
        "issues",
        "--allow-write",
        "--json",
        r#"{"title":"raw"}"#,
    ]);
    server.finish();
}

#[test]
fn multica_errors_are_structured_and_mutations_are_not_retried() {
    let mut denied = Reply::new("POST", "/api/issues", json!({"error":"permission denied"}));
    denied.status = 403;
    let mut missing = Reply::new("GET", "/api/issues/missing", json!({"error":"not found"}));
    missing.status = 404;
    let server = Server::new(vec![denied, missing]);
    for args in [
        vec!["issues", "create", "--title", "No"],
        vec!["issues", "get", "--issue-id", "missing"],
    ] {
        let output = server.output(&args);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert!(error.to_string().contains("provider"));
        assert!(!error.to_string().contains("fake-multica-token"));
    }
    server.finish();
}

#[test]
fn multica_zero_limit_and_invalid_input_do_not_call_server() {
    let server = Server::new(vec![]);
    let empty = server.run(&["issues", "list", "--limit", "0"]);
    assert_eq!(empty["issues"], json!([]));
    for args in [
        vec!["issues", "create", "--json", "[]"],
        vec!["issues", "create"],
        vec!["issues", "get", "--issue-id", ".."],
        vec!["issues", "list", "--offset", "-1"],
    ] {
        assert!(!server.output(&args).status.success());
    }
    server.finish();
}

#[test]
fn multica_comment_resume_does_not_drop_replies_or_repeat_roots_in_one_result() {
    let root = json!({"id":"root","created_at":"2026-10-15T01:00:00Z"});
    let newer = json!({"id":"newer","created_at":"2026-10-15T04:00:00Z"});
    let middle = json!({"id":"middle","created_at":"2026-10-15T03:00:00Z"});
    let older = json!({"id":"older","created_at":"2026-10-15T02:00:00Z"});
    let server = Server::new(vec![
        Reply::new(
            "GET",
            "/api/issues/i/comments",
            json!([root.clone(), newer]),
        )
        .header("X-Multica-Next-Before", "time")
        .header("X-Multica-Next-Before-Id", "newer"),
        Reply::new(
            "GET",
            "/api/issues/i/comments",
            json!([root, older, middle]),
        )
        .query("before", "time"),
    ]);
    let result = server.run(&[
        "issues",
        "comments",
        "list",
        "--issue-id",
        "i",
        "--thread",
        "root",
        "--tail",
        "2",
        "--limit",
        "3",
    ]);
    let rows = result["results"].as_array().unwrap();
    assert_eq!(
        rows.iter()
            .map(|r| r["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["root", "older", "newer"]
    );
    let next = result["_aai"]["pagination"]["next_command"]
        .as_str()
        .unwrap();
    assert!(next.contains("--page-skip 2"));
    assert!(next.contains("--before time"));
    server.finish();
}

#[test]
fn multica_repeated_and_empty_cursor_pages_fail_instead_of_hanging() {
    for empty in [false, true] {
        let page = if empty {
            json!([])
        } else {
            json!([{"id":"c"}])
        };
        let response = || {
            Reply::new("GET", "/api/issues/i/comments", page.clone())
                .header("X-Multica-Next-Before", "time")
                .header("X-Multica-Next-Before-Id", "id")
        };
        let replies = if empty {
            vec![response()]
        } else {
            vec![response(), response()]
        };
        let server = Server::new(replies);
        let output = server.output(&[
            "issues",
            "comments",
            "list",
            "--issue-id",
            "i",
            "--recent",
            "1",
            "--limit",
            "10",
        ]);
        assert!(!output.status.success());
        assert!(serde_json::from_slice::<Value>(&output.stderr).is_ok());
        server.finish();
    }
}

#[test]
fn multica_autopilot_run_total_is_page_local_not_a_global_count() {
    let records: Vec<_> = (0..100).map(|i| json!({"id":format!("r{i}")})).collect();
    let server = Server::new(vec![
        Reply::new(
            "GET",
            "/api/autopilots/a/runs",
            json!({"runs":records,"total":100}),
        ),
        Reply::new(
            "GET",
            "/api/autopilots/a/runs",
            json!({"runs":[{"id":"r100"}],"total":1}),
        )
        .query("offset", "100"),
    ]);
    let result = server.run(&[
        "autopilots",
        "runs",
        "list",
        "--autopilot-id",
        "a",
        "--limit",
        "101",
    ]);
    assert_eq!(result["runs"].as_array().unwrap().len(), 101);
    assert_eq!(result["total"], 1);
    assert_eq!(result["_aai"]["pagination"]["status"], "complete");
    server.finish();
}

#[test]
fn multica_encoded_identifiers_and_query_flags_override_extra_query_parameters() {
    let server = Server::new(vec![
        Reply::new(
            "GET",
            "/api/issues/space%20%2F%3F%23",
            json!({"id":"encoded"}),
        ),
        Reply::new("GET", "/api/issues", json!({"issues":[],"total":0}))
            .query("status", "in_progress")
            .query("q", "a & b"),
    ]);
    server.run(&["issues", "get", "--issue-id", "space /?#"]);
    server.run(&[
        "issues",
        "list",
        "--query",
        "status=done",
        "--status",
        "in_progress",
        "--q",
        "a & b",
    ]);
    server.finish();
}

#[test]
fn multica_credentials_and_scope_fail_locally_and_provider_401_is_preserved() {
    let server = Server::new(vec![]);
    for profile in [
        "base_url = 'http://127.0.0.1:1/api'\nworkspace = 'demo'\n",
        "base_url = 'http://127.0.0.1:1/api'\ntoken = 'fake'\n",
        "base_url = 'http://127.0.0.1:1/api'\ntoken = 'fake'\nworkspace = 'demo'\nauth_type = 'basic'\n",
    ] {
        std::fs::write(server.dir.path().join("config.toml"),format!("[profiles.multica]\n{profile}")).unwrap();
        let output=server.output(&["issues","list"]);
        assert!(!output.status.success());
        assert!(serde_json::from_slice::<Value>(&output.stderr).is_ok());
    }
    server.finish();
    let mut reply = Reply::new("GET", "/api/inbox", json!({"error":"invalid token"}));
    reply.status = 401;
    let server = Server::new(vec![reply]);
    let output = server.output(&["inbox", "list"]);
    assert!(!output.status.success());
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert!(error.to_string().contains("401"));
    assert!(!error.to_string().contains("fake-multica-token"));
    server.finish();
}
