//! CLI-level contract for `custom-api request`, run against the built binary. A local
//! gateway stands in for the network so these tests stay deterministic.

use std::{
    io::{Read, Write},
    net::TcpListener,
    path::Path,
    process::{Command, Output},
    thread,
};

use serde_json::{json, Value};

fn serve(reply: &'static str) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
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
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
            reply.len()
        );
        stream.write_all(response.as_bytes()).unwrap();
        String::from_utf8(raw).unwrap()
    });
    (format!("http://{address}"), handle)
}

fn write_config(dir: &Path, profiles: &str) -> String {
    let path = dir.join("config.toml");
    let escape = |p: &Path| p.display().to_string().replace('\\', "\\\\");
    std::fs::write(
        &path,
        format!(
            "secrets_file = \"{}\"\nkey_file = \"{}\"\n\n{profiles}",
            escape(&dir.join("secrets.enc.json")),
            escape(&dir.join("key"))
        ),
    )
    .unwrap();
    path.display().to_string()
}

fn run(config: &str, profile: &str, args: &[&str], envs: &[(&str, &str)]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aai-cli"))
        .args([
            "--config",
            config,
            "--profile",
            profile,
            "custom-api",
            "request",
        ])
        .args(args)
        .envs(envs.iter().copied())
        .output()
        .expect("execute aai-cli")
}

fn error_of(output: &Output) -> Value {
    assert!(output.stdout.is_empty(), "stdout must be empty on error");
    serde_json::from_slice(&output.stderr).expect("structured JSON error on stderr")
}

#[test]
fn isolated_request_prints_only_the_status_body_envelope() {
    let (gateway, server) = serve(r#"{"status":200,"body":{"items":[1,2]}}"#);
    let temp = tempfile::tempdir().unwrap();
    let config = write_config(
        temp.path(),
        &format!(
            r#"[profiles.iso]
provider = "custom_api"
base_url = "https://api.example.com/v1"
gateway_url = "{gateway}/request"
gateway_token_env = "AAI_TEST_CUSTOM_GATEWAY_TOKEN"
"#
        ),
    );
    let output = run(
        &config,
        "iso",
        &[
            "--url",
            "https://api.example.com/v1/items",
            "--method",
            "POST",
            "--query",
            "limit=2",
            "--header",
            "X-Trace=abc",
            "--json",
            r#"{"name":"sample"}"#,
        ],
        &[
            ("AAI_TEST_CUSTOM_GATEWAY_TOKEN", "fixture-gateway"),
            // Ambient fallbacks must not turn an isolated profile into a credentialed one.
            ("AAI_ISO_TOKEN", "ambient-token"),
            ("AAI_TOKEN", "ambient-token"),
            ("AAI_API_TOKEN", "ambient-token"),
            ("AAI_PASSWORD", "ambient-token"),
        ],
    );
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(stdout, json!({"status":200,"body":{"items":[1,2]}}));

    let raw = server.join().unwrap();
    assert!(raw.starts_with("POST /request "));
    assert!(raw.contains("authorization: Bearer fixture-gateway"));
    assert!(!raw.contains("ambient-token"));
    let wire: Value = serde_json::from_str(raw.split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(
        wire,
        json!({
            "method":"POST",
            "url":"https://api.example.com/v1/items",
            "query":{"limit":"2"},
            "body":{"name":"sample"},
            "headers":[{"name":"X-Trace","value":"abc"}]
        })
    );
}

#[test]
fn validation_failures_are_structured_stderr_with_exit_code_two() {
    let temp = tempfile::tempdir().unwrap();
    let config = write_config(
        temp.path(),
        r#"[profiles.direct]
provider = "custom_api"
base_url = "https://api.example.com/v1"
auth_type = "bearer_token"
token = "fixture-token"

[profiles.other]
provider = "github"
base_url = "https://api.example.com/v1"
"#,
    );
    for (profile, url, message) in [
        (
            "direct",
            "https://evil.example/v1",
            "URL is outside configured origin and base path",
        ),
        (
            "direct",
            "https://api.example.com/v1/%2e%2e/admin",
            "Unsafe encoded or dot-segment path",
        ),
        (
            "other",
            "https://api.example.com/v1",
            "Select a custom_api profile",
        ),
    ] {
        let output = run(&config, profile, &["--url", url], &[]);
        assert_eq!(output.status.code(), Some(2), "{url}");
        let error = error_of(&output);
        assert_eq!(error["code"], "invalid_input");
        assert_eq!(error["service"], "custom_api");
        assert_eq!(error["message"], message);
        assert!(!error.to_string().contains("fixture-token"));
    }
}

#[test]
fn missing_url_is_a_usage_error() {
    let temp = tempfile::tempdir().unwrap();
    let config = write_config(temp.path(), "[profiles.p]\nprovider = \"custom_api\"\n");
    let output = run(&config, "p", &[], &[]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--url"));
}
