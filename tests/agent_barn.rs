//! CLI/config/HTTP/output integration using only a local peer and fake credentials.
use serde_json::Value;
use std::{
    io::{Read, Write},
    net::TcpListener,
    process::Command,
    thread,
};

#[test]
fn agent_barn_profiles_and_json_output_work_through_the_binary() {
    for (status, body) in [
        (200, r#"{"user_id":"test-user","organizations":[]}"#),
        (403, r#"{"detail":"This API key is read-only"}"#),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let worker = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut bytes = [0; 8192];
            let read = socket.read(&mut bytes).unwrap();
            write!(socket,"HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
            String::from_utf8_lossy(&bytes[..read]).into_owned()
        });
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config.toml");
        std::fs::write(
            &config,
            format!(
                r#"
[profiles.barn]
provider = "agent-barn"
auth_type = "bearer_token"
base_url = "http://{address}/api/v1"
token_env = "AAI_AGENT_BARN_OFFLINE_TEST_TOKEN"
"#
            ),
        )
        .unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_aai-cli"))
            .env(
                "AAI_AGENT_BARN_OFFLINE_TEST_TOKEN",
                "fake-environment-token",
            )
            .args([
                "--config",
                config.to_str().unwrap(),
                "--profile",
                "barn",
                "agent-barn",
                "auth",
                "context",
            ])
            .output()
            .unwrap();
        if status == 200 {
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(output.stderr.is_empty());
            let response: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(response["user_id"], "test-user");
        } else {
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            let error: Value = serde_json::from_slice(&output.stderr).unwrap();
            assert_eq!(error["code"], "auth_error");
            assert_eq!(error["status"], 403);
            assert_eq!(error["details"]["detail"], "This API key is read-only");
        }
        assert!(!String::from_utf8_lossy(&output.stdout).contains("fake-environment-token"));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("fake-environment-token"));
        let request = worker.join().unwrap().to_lowercase();
        assert!(request.starts_with("get /api/v1/auth/context "));
        assert!(request.contains("authorization: bearer fake-environment-token"));
    }
}

#[test]
fn agent_barn_skill_is_packaged_and_discoverable() {
    let output = Command::new(env!("CARGO_BIN_EXE_aai-cli"))
        .args(["skills", "discover"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(response.to_string().contains("aai-agent-barn"));
}
