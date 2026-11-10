//! CLI-level contract for `gitlab`, run against the built binary. A local server stands
//! in for GitLab, so these assert the raw request line: GitLab addresses projects and
//! files by URL-encoded paths, and a decoded `/` there is a 404 rather than a parse error.

use std::{
    io::{Read, Write},
    net::TcpListener,
    path::Path,
    process::{Command, Output},
    thread,
};

use serde_json::{json, Value};

/// Serves `replies` in order and yields each raw request.
fn serve(replies: Vec<String>) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        replies
            .into_iter()
            .map(|reply| {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .unwrap();
                let mut raw = Vec::new();
                let mut bytes = [0; 4096];
                loop {
                    let count = stream.read(&mut bytes).unwrap();
                    if count == 0 {
                        break;
                    }
                    raw.extend_from_slice(&bytes[..count]);
                    let text = String::from_utf8_lossy(&raw);
                    let Some(end) = text.find("\r\n\r\n") else {
                        continue;
                    };
                    let length = text[..end]
                        .lines()
                        .find_map(|line| {
                            line.to_lowercase()
                                .strip_prefix("content-length: ")?
                                .parse()
                                .ok()
                        })
                        .unwrap_or(0);
                    if raw.len() >= end + 4 + length {
                        break;
                    }
                }
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                    reply.len()
                );
                stream.write_all(response.as_bytes()).unwrap();
                String::from_utf8(raw).unwrap()
            })
            .collect()
    });
    (format!("http://{address}"), handle)
}

fn config(dir: &Path, base_url: &str, extra: &str) -> String {
    let path = dir.join("config.toml");
    let escape = |p: &Path| p.display().to_string().replace('\\', "\\\\");
    std::fs::write(
        &path,
        format!(
            "secrets_file = \"{}\"\nkey_file = \"{}\"\n\n[profiles.gl]\nprovider = \"gitlab\"\nauth_type = \"bearer_token\"\nbase_url = \"{base_url}/api/v4\"\ntoken_env = \"GITLAB_TEST_TOKEN\"\n{extra}",
            escape(&dir.join("secrets.enc.json")),
            escape(&dir.join("key"))
        ),
    )
    .unwrap();
    path.display().to_string()
}

fn run(config: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aai-cli"))
        .args(["--config", config, "--profile", "gl"])
        .args(args)
        .env("GITLAB_TEST_TOKEN", "glpat-test")
        .output()
        .unwrap()
}

fn stdout_json(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn request_line(raw: &str) -> &str {
    raw.lines().next().unwrap()
}

fn body(raw: &str) -> Value {
    serde_json::from_str(raw.split_once("\r\n\r\n").unwrap().1).unwrap()
}

#[test]
fn nested_project_path_stays_encoded_on_the_wire_with_bearer_auth() {
    let dir = tempfile::tempdir().unwrap();
    let (url, server) = serve(vec![json!({"iid": 7}).to_string()]);
    let config = config(dir.path(), &url, "");
    let output = run(
        &config,
        &["gitlab", "mrs", "get", "7", "--project", "group/sub/app"],
    );
    assert_eq!(stdout_json(&output)["iid"], 7);
    let requests = server.join().unwrap();
    assert_eq!(
        request_line(&requests[0]),
        "GET /api/v4/projects/group%2Fsub%2Fapp/merge_requests/7 HTTP/1.1"
    );
    assert!(requests[0]
        .to_lowercase()
        .contains("authorization: bearer glpat-test"));
}

#[test]
fn project_comes_from_the_profile_when_the_flag_is_omitted() {
    let dir = tempfile::tempdir().unwrap();
    let (url, server) = serve(vec![json!({"id": 1}).to_string()]);
    let config = config(dir.path(), &url, "repo = \"group/app\"\n");
    stdout_json(&run(&config, &["gitlab", "projects", "get"]));
    assert_eq!(
        request_line(&server.join().unwrap()[0]),
        "GET /api/v4/projects/group%2Fapp HTTP/1.1"
    );
}

#[test]
fn numeric_project_ids_pass_through() {
    let dir = tempfile::tempdir().unwrap();
    let (url, server) = serve(vec![json!({"id": 42}).to_string()]);
    let config = config(dir.path(), &url, "");
    stdout_json(&run(&config, &["gitlab", "projects", "get", "42"]));
    assert_eq!(
        request_line(&server.join().unwrap()[0]),
        "GET /api/v4/projects/42 HTTP/1.1"
    );
}

#[test]
fn missing_project_is_a_structured_error_before_any_request() {
    let dir = tempfile::tempdir().unwrap();
    let config = config(dir.path(), "http://127.0.0.1:9", "");
    let output = run(&config, &["gitlab", "mrs", "list"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--project"));
}

#[test]
fn file_path_and_ref_are_fully_encoded_for_raw_source() {
    let dir = tempfile::tempdir().unwrap();
    let (url, server) = serve(vec!["fn main() {}".to_string()]);
    let config = config(dir.path(), &url, "");
    let output = run(
        &config,
        &[
            "gitlab",
            "source",
            "get",
            "feature/x",
            "src/my file.rs",
            "--project",
            "group/app",
        ],
    );
    assert!(output.status.success());
    assert_eq!(
        request_line(&server.join().unwrap()[0]),
        "GET /api/v4/projects/group%2Fapp/repository/files/src%2Fmy%20file.rs/raw?ref=feature%2Fx HTTP/1.1"
    );
}

#[test]
fn lists_walk_page_numbers_to_the_limit_and_report_truncation() {
    let dir = tempfile::tempdir().unwrap();
    let page = |ids: std::ops::Range<u32>| {
        Value::Array(ids.map(|id| json!({"iid": id})).collect()).to_string()
    };
    let (url, server) = serve(vec![page(0..100), page(100..200)]);
    let config = config(dir.path(), &url, "");
    let value = stdout_json(&run(
        &config,
        &[
            "gitlab",
            "mrs",
            "list",
            "--project",
            "group/app",
            "--state",
            "opened",
            "--limit",
            "150",
        ],
    ));
    assert_eq!(value["size"], 150);
    assert_eq!(value["truncated"], true);
    let requests = server.join().unwrap();
    assert_eq!(
        request_line(&requests[0]),
        "GET /api/v4/projects/group%2Fapp/merge_requests?state=opened&per_page=100&page=1 HTTP/1.1"
    );
    assert_eq!(
        request_line(&requests[1]),
        "GET /api/v4/projects/group%2Fapp/merge_requests?state=opened&per_page=100&page=2 HTTP/1.1"
    );
}

#[test]
fn short_page_ends_a_list_without_truncation() {
    let dir = tempfile::tempdir().unwrap();
    let (url, _server) = serve(vec![json!([{"id": 1}]).to_string()]);
    let config = config(dir.path(), &url, "");
    let value = stdout_json(&run(&config, &["gitlab", "projects", "list"]));
    assert_eq!(value["size"], 1);
    assert_eq!(value["truncated"], false);
}

#[test]
fn mr_create_merges_typed_flags_over_json() {
    let dir = tempfile::tempdir().unwrap();
    let (url, server) = serve(vec![json!({"iid": 1}).to_string()]);
    let config = config(dir.path(), &url, "");
    stdout_json(&run(
        &config,
        &[
            "gitlab",
            "mrs",
            "create",
            "--project",
            "group/app",
            "--source-branch",
            "feature",
            "--target-branch",
            "main",
            "--title",
            "typed",
            "--json",
            r#"{"title":"json","squash":true}"#,
        ],
    ));
    let requests = server.join().unwrap();
    assert_eq!(
        request_line(&requests[0]),
        "POST /api/v4/projects/group%2Fapp/merge_requests HTTP/1.1"
    );
    assert_eq!(
        body(&requests[0]),
        json!({"source_branch":"feature","target_branch":"main","title":"typed","squash":true})
    );
}

#[test]
fn mr_close_is_a_state_event_update() {
    let dir = tempfile::tempdir().unwrap();
    let (url, server) = serve(vec![json!({"iid": 3}).to_string()]);
    let config = config(dir.path(), &url, "");
    stdout_json(&run(
        &config,
        &["gitlab", "mrs", "close", "3", "--project", "group/app"],
    ));
    let requests = server.join().unwrap();
    assert_eq!(
        request_line(&requests[0]),
        "PUT /api/v4/projects/group%2Fapp/merge_requests/3 HTTP/1.1"
    );
    assert_eq!(body(&requests[0]), json!({"state_event": "close"}));
}

#[test]
fn mr_note_create_posts_the_body() {
    let dir = tempfile::tempdir().unwrap();
    let (url, server) = serve(vec![json!({"id": 9}).to_string()]);
    let config = config(dir.path(), &url, "");
    stdout_json(&run(
        &config,
        &[
            "gitlab",
            "mrs",
            "notes",
            "create",
            "3",
            "--project",
            "group/app",
            "--body",
            "lgtm",
        ],
    ));
    let requests = server.join().unwrap();
    assert_eq!(
        request_line(&requests[0]),
        "POST /api/v4/projects/group%2Fapp/merge_requests/3/notes HTTP/1.1"
    );
    assert_eq!(body(&requests[0]), json!({"body": "lgtm"}));
}

#[test]
fn branch_names_are_encoded_as_one_segment() {
    let dir = tempfile::tempdir().unwrap();
    let (url, server) = serve(vec![json!({"name": "feature/x"}).to_string()]);
    let config = config(dir.path(), &url, "");
    stdout_json(&run(
        &config,
        &[
            "gitlab",
            "branches",
            "get",
            "feature/x",
            "--project",
            "group/app",
        ],
    ));
    assert_eq!(
        request_line(&server.join().unwrap()[0]),
        "GET /api/v4/projects/group%2Fapp/repository/branches/feature%2Fx HTTP/1.1"
    );
}

#[test]
fn generic_request_keeps_an_encoded_path_and_requires_allow_write() {
    let dir = tempfile::tempdir().unwrap();
    let (url, server) = serve(vec![json!({"id": 1}).to_string()]);
    let config = config(dir.path(), &url, "");
    stdout_json(&run(
        &config,
        &["gitlab", "request", "get", "projects/group%2Fapp/labels"],
    ));
    assert_eq!(
        request_line(&server.join().unwrap()[0]),
        "GET /api/v4/projects/group%2Fapp/labels HTTP/1.1"
    );
    let refused = run(&config, &["gitlab", "request", "post", "projects/1/labels"]);
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("--allow-write"));
}
