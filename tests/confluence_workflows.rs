//! CLI-to-HTTP contracts for Confluence commands. The local server exercises the
//! built binary without touching customer content or relying on Atlassian state.
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    net::TcpListener,
    process::{Command, Output},
    thread,
    time::Duration,
};

fn server(replies: Vec<Value>) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let mut requests = Vec::new();
        for reply in replies {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut raw = Vec::new();
            loop {
                let mut chunk = [0; 4096];
                let count = stream.read(&mut chunk).unwrap();
                if count == 0 {
                    break;
                }
                raw.extend_from_slice(&chunk[..count]);
                let text = String::from_utf8_lossy(&raw);
                if let Some(end) = text.find("\r\n\r\n") {
                    let length = text[..end]
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length: ")
                                .and_then(|v| v.parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if raw.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            requests.push(String::from_utf8_lossy(&raw).into_owned());
            let body = reply.to_string();
            let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            stream.write_all(response.as_bytes()).unwrap();
        }
        requests
    });
    (format!("http://{address}"), handle)
}

fn run(site: &str, args: &[&str]) -> Output {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.toml");
    std::fs::write(&config, format!("[profiles.test]\nprovider = \"confluence\"\nauth_type = \"none\"\nsite_url = \"{site}\"\n")).unwrap();
    Command::new(env!("CARGO_BIN_EXE_aai-cli"))
        .args([
            "--config",
            config.to_str().unwrap(),
            "--profile",
            "test",
            "confluence",
        ])
        .args(args)
        .output()
        .unwrap()
}

fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn search_encodes_cql_and_collects_results() {
    let (site, handle) = server(vec![json!({"results":[{"id":"1"}],"_links":{}})]);
    let result = success(run(
        &site,
        &["search", "--query", "release notes", "--limit", "2"],
    ));
    assert_eq!(result["results"][0]["id"], "1");
    let request = handle.join().unwrap().remove(0);
    assert!(
        request.starts_with(
            "GET /wiki/rest/api/content/search?cql=text%20~%20%22release%20notes%22&limit=2 "
        ),
        "{request}"
    );
}

#[test]
fn database_entries_send_filter_and_sort_and_preserve_values() {
    let (site, handle) = server(vec![
        json!({"results":[{"id":"row-1","values":{"field-1":"Ready"}}],"_links":{}}),
    ]);
    let result = success(run(
        &site,
        &[
            "databases",
            "entries",
            "12",
            "--filter",
            "field-1[eq]=Ready",
            "--sort",
            "field-1:asc",
            "--limit",
            "2",
        ],
    ));
    assert_eq!(result["results"][0]["values"]["field-1"], "Ready");
    let request = handle.join().unwrap().remove(0);
    assert!(request.starts_with("GET /wiki/api/v2/databases/12/entries?limit=2&sort=field-1%3Aasc&filter[field-1][eq]=Ready "), "{request}");
}

#[test]
fn comment_update_uses_next_version_and_storage_body() {
    let (site, handle) = server(vec![
        json!({"version":{"number":3}}),
        json!({"id":"7","version":{"number":4}}),
    ]);
    let result = success(run(
        &site,
        &[
            "pages",
            "comments",
            "update",
            "7",
            "--kind",
            "inline",
            "--body",
            "<p>edited</p>",
        ],
    ));
    assert_eq!(result["version"]["number"], 4);
    let requests = handle.join().unwrap();
    assert!(requests[0].starts_with("GET /wiki/api/v2/inline-comments/7 "));
    assert!(requests[1].starts_with("PUT /wiki/api/v2/inline-comments/7 "));
    assert!(requests[1].contains("\"version\":{\"number\":4}"));
    assert!(requests[1].contains("\"value\":\"<p>edited</p>\""));
}

#[test]
fn attachment_replacement_posts_multipart_to_existing_attachment() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("replacement.txt");
    std::fs::write(&file, "replacement bytes").unwrap();
    let (site, handle) = server(vec![json!({"id":"55"})]);
    let result = success(run(
        &site,
        &[
            "pages",
            "attachments",
            "replace",
            "12",
            "55",
            "--file",
            file.to_str().unwrap(),
        ],
    ));
    assert_eq!(result["id"], "55");
    let request = handle.join().unwrap().remove(0);
    assert!(request.starts_with("POST /wiki/rest/api/content/12/child/attachment/55/data "));
    assert!(request
        .to_ascii_lowercase()
        .contains("content-type: multipart/form-data;"));
    assert!(request.contains("replacement bytes"));
}

#[test]
fn permission_delete_targets_one_principal() {
    let (site, handle) = server(vec![json!({})]);
    success(run(
        &site,
        &[
            "pages",
            "permissions",
            "delete",
            "12",
            "--principal-type",
            "user",
            "--principal-id",
            "abc",
            "--operation",
            "read",
        ],
    ));
    let request = handle.join().unwrap().remove(0);
    assert!(request.starts_with("DELETE /wiki/api/v2/pages/12/permissions?principal-type=user&principal-id=abc&operation=read "), "{request}");
}

#[test]
fn database_read_routes_cover_metadata_fields_views_and_entry() {
    let cases: Vec<(Vec<&str>, &str, Value)> = vec![
        (
            vec!["databases", "get", "12"],
            "GET /wiki/api/v2/databases/12 ",
            json!({"id":"12"}),
        ),
        (
            vec!["databases", "fields", "12", "--limit", "2"],
            "GET /wiki/api/v2/databases/12/fields?limit=2 ",
            json!({"results":[],"_links":{}}),
        ),
        (
            vec!["databases", "views", "12", "--limit", "2"],
            "GET /wiki/api/v2/databases/12/views?limit=2 ",
            json!({"results":[],"_links":{}}),
        ),
        (
            vec!["databases", "entry", "12", "7"],
            "GET /wiki/api/v2/databases/12/entries/7 ",
            json!({"id":"7"}),
        ),
    ];
    for (args, expected, reply) in cases {
        let (site, handle) = server(vec![reply]);
        success(run(&site, &args));
        let request = handle.join().unwrap().remove(0);
        assert!(request.starts_with(expected), "{request}");
    }
}

#[test]
fn comment_create_get_and_delete_route_to_correct_kinds() {
    let cases: Vec<(Vec<&str>, &str)> = vec![
        (
            vec![
                "pages",
                "comments",
                "create",
                "12",
                "--body",
                "<p>new</p>",
                "--inline-marker-ref",
                "mark-1",
            ],
            "POST /wiki/api/v2/inline-comments ",
        ),
        (
            vec!["pages", "comments", "get", "7", "--kind", "footer"],
            "GET /wiki/api/v2/footer-comments/7 ",
        ),
        (
            vec!["pages", "comments", "delete", "7", "--kind", "inline"],
            "DELETE /wiki/api/v2/inline-comments/7 ",
        ),
    ];
    for (args, expected) in cases {
        let (site, handle) = server(vec![json!({"id":"7"})]);
        success(run(&site, &args));
        let request = handle.join().unwrap().remove(0);
        assert!(request.starts_with(expected), "{request}");
        if args.contains(&"--inline-marker-ref") {
            assert!(
                request.contains("\"inlineCommentProperties\":{\"inlineMarkerRef\":\"mark-1\"}")
            );
        }
    }
}

#[test]
fn attachment_delete_and_page_versions_route_to_provider_endpoints() {
    let cases: Vec<(Vec<&str>, &str, Value)> = vec![
        (
            vec!["pages", "attachments", "delete", "55"],
            "DELETE /wiki/api/v2/attachments/55 ",
            json!({}),
        ),
        (
            vec!["pages", "versions", "list", "12", "--limit", "2"],
            "GET /wiki/api/v2/pages/12/versions?limit=2 ",
            json!({"results":[],"_links":{}}),
        ),
        (
            vec!["pages", "versions", "get", "12", "3"],
            "GET /wiki/api/v2/pages/12/versions/3 ",
            json!({"number":3}),
        ),
    ];
    for (args, expected, reply) in cases {
        let (site, handle) = server(vec![reply]);
        success(run(&site, &args));
        let request = handle.join().unwrap().remove(0);
        assert!(request.starts_with(expected), "{request}");
    }
}

#[test]
fn page_permissions_read_and_write_use_expected_methods_and_bodies() {
    let cases: Vec<(Vec<&str>, &str, Value)> = vec![
        (
            vec!["pages", "permissions", "list", "12", "--limit", "2"],
            "GET /wiki/api/v2/pages/12/permissions?limit=2 ",
            json!({"results":[],"_links":{}}),
        ),
        (
            vec!["pages", "permissions", "restriction-state", "12"],
            "GET /wiki/api/v2/pages/12/restriction-state ",
            json!({"state":"OPEN"}),
        ),
        (
            vec![
                "pages",
                "permissions",
                "set-restriction-state",
                "12",
                "--json",
                "{\"state\":\"OPEN\"}",
            ],
            "PUT /wiki/api/v2/pages/12/restriction-state ",
            json!({"state":"OPEN"}),
        ),
        (
            vec![
                "pages",
                "permissions",
                "add",
                "12",
                "--json",
                "{\"permissions\":[]}",
            ],
            "POST /wiki/api/v2/pages/12/permissions ",
            json!({"results":[]}),
        ),
        (
            vec![
                "pages",
                "permissions",
                "set",
                "12",
                "--json",
                "{\"permissions\":[]}",
            ],
            "PUT /wiki/api/v2/pages/12/permissions ",
            json!({"results":[]}),
        ),
    ];
    for (args, expected, reply) in cases {
        let (site, handle) = server(vec![reply]);
        success(run(&site, &args));
        let request = handle.join().unwrap().remove(0);
        assert!(request.starts_with(expected), "{request}");
        if args.contains(&"--json") {
            assert!(request.contains("{\""), "{request}");
        }
    }
}
