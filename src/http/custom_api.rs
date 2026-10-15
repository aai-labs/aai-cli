//! Scope, credentials and bounded execution for user-configured HTTP APIs.
use crate::{
    config::{CustomHeader, Profile},
    error::AppError,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client, Method, Url,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::{
    collections::HashSet,
    net::{IpAddr, ToSocketAddrs},
    time::Duration,
};
const LIMIT: usize = 1024 * 1024;
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    #[serde(default = "default_method")]
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub query: Map<String, Value>,
    #[serde(default)]
    pub body: Value,
    #[serde(default)]
    pub headers: Vec<CustomHeader>,
}
fn default_method() -> String {
    "GET".into()
}
pub fn refused(message: &str) -> AppError {
    AppError::invalid_input("custom_api", "request", message)
}
fn unavailable() -> AppError {
    AppError::internal("custom_api", "request", "Custom API upstream unavailable")
}
fn parse_url(value: &str, base: bool) -> Result<Url, AppError> {
    if value.bytes().any(|c| c <= 32 || c == 127 || c == b'\\') {
        return Err(refused("Invalid absolute HTTPS URL"));
    }
    // Validate the original path before Url normalizes dot segments.
    let authority_end = value
        .find("://")
        .ok_or_else(|| refused("Absolute HTTPS URL required"))?
        + 3;
    let path = value[authority_end..]
        .find('/')
        .map(|index| &value[authority_end + index..])
        .unwrap_or("");
    let path = path.split(['?', '#']).next().unwrap_or("");
    let decoded = urlencoding::decode(path).map_err(|_| refused("Invalid encoded path"))?;
    if decoded.contains(['%', '\\'])
        || decoded.contains("//")
        || decoded.split('/').any(|s| s == "." || s == "..")
        || decoded.split('/').count() != path.split('/').count()
        || decoded.chars().any(char::is_control)
    {
        return Err(refused("Unsafe encoded or dot-segment path"));
    }
    let url = Url::parse(value).map_err(|_| refused("Invalid absolute HTTPS URL"))?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || (base && url.query().is_some())
        || url.port() == Some(0)
        || url.host_str().is_some_and(|host| host.ends_with('.'))
    {
        return Err(refused(
            "Absolute HTTPS URL without credentials or fragments required",
        ));
    }
    Ok(url)
}
fn scoped(base: &str, target: &str) -> Result<Url, AppError> {
    let base = parse_url(base, true)?;
    let target = parse_url(target, false)?;
    let root = urlencoding::decode(base.path()).map_err(|_| refused("Invalid base path"))?;
    let root = root.trim_end_matches('/');
    let path = urlencoding::decode(target.path()).map_err(|_| refused("Invalid target path"))?;
    if base.origin() != target.origin()
        || (!root.is_empty() && path != root && !path.starts_with(&format!("{root}/")))
    {
        return Err(refused("URL is outside configured origin and base path"));
    }
    Ok(target)
}
fn public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !(ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_multicast()
                || ip.is_unspecified()
                || a == 0
                || a >= 240
                || (a == 100 && (64..=127).contains(&b))
                || (a == 192 && ((b == 0 && (c == 0 || c == 2)) || (b == 88 && c == 99)))
                || (a == 198 && (b == 18 || b == 19 || (b == 51 && c == 100)))
                || (a == 203 && b == 0 && c == 113))
        }
        IpAddr::V6(ip) => {
            let segments = ip.segments();
            segments[0] & 0xe000 == 0x2000
                && !(segments[0] == 0x2001 && (segments[1] < 0x200 || segments[1] == 0xdb8))
                && segments[0] != 0x2002
                && segments[0] != 0x3fff
        }
    }
}
fn reserved(name: &str) -> bool {
    matches!(
        name,
        "authorization"
            | "proxy-authorization"
            | "host"
            | "connection"
            | "content-length"
            | "transfer-encoding"
            | "upgrade"
            | "te"
            | "trailer"
            | "keep-alive"
            | "proxy-connection"
    )
}
fn header(name: &str, value: &str, auth: bool) -> Result<(HeaderName, HeaderValue), AppError> {
    let name =
        HeaderName::from_bytes(name.as_bytes()).map_err(|_| refused("Invalid header name"))?;
    if reserved(name.as_str()) && !(auth && name == "authorization") {
        return Err(refused("Reserved header name"));
    }
    if value.bytes().any(|b| !(32..=126).contains(&b)) {
        return Err(refused("Header value must be printable ASCII"));
    }
    let value = HeaderValue::from_str(value).map_err(|_| refused("Invalid header value"))?;
    Ok((name, value))
}
fn credentials(profile: &Profile) -> Result<(HeaderMap, Vec<String>), AppError> {
    let mut headers = HeaderMap::new();
    let mut secrets = Vec::new();
    let required = |value: &Option<String>| {
        value
            .clone()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| refused("Missing configured credential"))
    };
    let auth = match profile.auth_type.as_deref().unwrap_or("none") {
        "none" => None,
        "bearer_token" => {
            let token = required(&profile.token)?;
            secrets.push(token.clone());
            Some(("authorization".into(), format!("Bearer {token}")))
        }
        "api_key_header" => {
            let token = required(&profile.api_token)?;
            secrets.push(token.clone());
            Some((
                profile
                    .api_key_header
                    .clone()
                    .ok_or_else(|| refused("Missing API key header name"))?,
                token,
            ))
        }
        "basic" => {
            let user = required(&profile.username)?;
            if user.contains(':') {
                return Err(refused("Basic username cannot contain colon"));
            }
            header("x-basic-user", &user, false)?;
            secrets.push(user.clone());
            let password = required(&profile.password)?;
            header("x-basic-password", &password, false)?;
            secrets.push(password.clone());
            let encoded = STANDARD.encode(format!("{user}:{password}"));
            secrets.push(encoded.clone());
            Some(("authorization".into(), format!("Basic {encoded}")))
        }
        _ => return Err(refused("Unsupported Custom API authentication")),
    };
    if let Some((name, value)) = auth {
        secrets.push(value.clone());
        let (name, value) = header(&name, &value, true)?;
        headers.insert(name, value);
    }
    for item in &profile.headers {
        let value = item
            .value
            .clone()
            .ok_or_else(|| refused("Missing configured header value"))?;
        if item.secret && value.is_empty() {
            return Err(refused("Secret header value must not be empty"));
        }
        let (name, parsed) = header(&item.name, &value, false)?;
        if headers.contains_key(&name) {
            return Err(refused("Duplicate configured header"));
        }
        if item.secret || item.value_env.is_some() || item.value_secret.is_some() {
            secrets.push(value);
        }
        headers.insert(name, parsed);
    }
    Ok((headers, secrets))
}
fn client_builder() -> reqwest::ClientBuilder {
    Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(30))
}
pub async fn execute(profile: &Profile, request: Request) -> Result<Value, AppError> {
    if profile.provider.as_deref() != Some("custom_api") {
        return Err(refused("Select a custom_api profile"));
    }
    let url = scoped(
        profile
            .base_url
            .as_deref()
            .ok_or_else(|| refused("Missing base_url"))?,
        &request.url,
    )?;
    let method = Method::from_bytes(request.method.to_uppercase().as_bytes())
        .map_err(|_| refused("Invalid HTTP method"))?;
    if !matches!(
        method,
        Method::GET
            | Method::HEAD
            | Method::POST
            | Method::PUT
            | Method::PATCH
            | Method::DELETE
            | Method::OPTIONS
    ) {
        return Err(refused("Unsupported HTTP method"));
    }
    let body_size = serde_json::to_vec(&request.body)
        .map_err(|_| refused("Invalid body"))?
        .len();
    if body_size > LIMIT || request.headers.len() > 32 {
        return Err(refused("Request exceeds limits"));
    }
    let mut seen = HashSet::new();
    for item in &request.headers {
        let (name, _) = header(
            &item.name,
            item.value
                .as_deref()
                .ok_or_else(|| refused("Missing header value"))?,
            false,
        )?;
        if !seen.insert(name) {
            return Err(refused("Duplicate request header"));
        }
    }
    if let Some(gateway) = &profile.gateway_url {
        if profile.token.is_some()
            || profile.api_token.is_some()
            || profile.password.is_some()
            || !profile.headers.is_empty()
            || profile.auth_type.as_deref().is_some_and(|s| s != "none")
        {
            return Err(refused(
                "Isolated profiles cannot contain upstream credentials or headers",
            ));
        }
        let gateway = Url::parse(gateway).map_err(|_| refused("Invalid gateway URL"))?;
        if !matches!(gateway.scheme(), "http" | "https")
            || !gateway.username().is_empty()
            || gateway.password().is_some()
            || gateway.fragment().is_some()
        {
            return Err(refused("Invalid gateway URL"));
        }
        let token = profile
            .gateway_token
            .as_deref()
            .ok_or_else(|| refused("Missing gateway token"))?;
        let response = client_builder()
            .build()
            .map_err(|_| unavailable())?
            .post(gateway)
            .bearer_auth(token)
            .json(&request)
            .send()
            .await
            .map_err(|_| unavailable())?;
        let status = response.status();
        let body = read_response(response, &[token.into()]).await?;
        if !status.is_success() {
            return Err(AppError::api(
                "custom_api",
                "request",
                status,
                "Custom API gateway refused request",
                None,
            ));
        }
        if body.get("status").and_then(Value::as_u64).is_none() || body.get("body").is_none() {
            return Err(unavailable());
        }
        return Ok(body);
    }
    if profile.gateway_token.is_some() {
        return Err(refused("gateway_token requires gateway_url"));
    }
    let host = url.host_str().ok_or_else(|| refused("Missing hostname"))?;
    let addresses = (
        host.trim_start_matches('[').trim_end_matches(']'),
        url.port_or_known_default().unwrap_or(443),
    )
        .to_socket_addrs()
        .map_err(|_| unavailable())?
        .collect::<Vec<_>>();
    if addresses.is_empty() || addresses.iter().any(|address| !public(address.ip())) {
        return Err(refused(
            "Private, internal and special network destinations are unsupported",
        ));
    }
    let client = client_builder()
        .resolve_to_addrs(host, &addresses)
        .build()
        .map_err(|_| unavailable())?;
    send_direct(&client, profile, request, url, method).await
}
async fn send_direct(
    client: &Client,
    profile: &Profile,
    request: Request,
    mut url: Url,
    method: Method,
) -> Result<Value, AppError> {
    let (mut headers, secrets) = credentials(profile)?;
    for item in &request.headers {
        let (name, value) = header(&item.name, item.value.as_deref().unwrap_or(""), false)?;
        if headers.contains_key(&name) {
            return Err(refused("Configured headers cannot be overridden"));
        }
        headers.insert(name, value);
    }
    for (name, value) in &request.query {
        for value in value
            .as_array()
            .map(|a| a.as_slice())
            .unwrap_or(std::slice::from_ref(value))
        {
            let value = value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| value.to_string());
            url.query_pairs_mut().append_pair(name, &value);
        }
    }
    let mut outbound = client.request(method, url).headers(headers);
    if !request.body.is_null() {
        outbound = if let Some(text) = request.body.as_str() {
            outbound.body(text.to_owned())
        } else {
            outbound.json(&request.body)
        };
    }
    let response = outbound.send().await.map_err(|_| unavailable())?;
    let status = response.status();
    if status.is_redirection() {
        return Err(refused("Redirects are not followed"));
    }
    let body = read_response(response, &secrets).await?;
    Ok(json!({"status":status.as_u16(),"body":body}))
}
async fn read_response(
    mut response: reqwest::Response,
    secrets: &[String],
) -> Result<Value, AppError> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| unavailable())? {
        if bytes.len() + chunk.len() > LIMIT {
            return Err(refused("Response exceeds 1 MiB"));
        }
        bytes.extend_from_slice(&chunk);
    }
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let mut secrets = secrets.to_vec();
    secrets.sort_by_key(|s| std::cmp::Reverse(s.len()));
    Ok(redact(
        serde_json::from_str(&text).unwrap_or(Value::String(text)),
        &secrets,
    ))
}
fn redact(value: Value, secrets: &[String]) -> Value {
    let clean = |mut text: String| {
        for secret in secrets {
            if !secret.is_empty() {
                text = text.replace(secret, "[REDACTED]");
            }
        }
        text
    };
    match value {
        Value::String(text) => Value::String(clean(text)),
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|item| redact(item, secrets))
                .collect(),
        ),
        Value::Object(items) => Value::Object(
            items
                .into_iter()
                .map(|(key, value)| (clean(key), redact(value, secrets)))
                .collect(),
        ),
        other => {
            if secrets
                .iter()
                .any(|secret| !secret.is_empty() && other.to_string().contains(secret))
            {
                Value::String("[REDACTED]".into())
            } else {
                other
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };
    fn profile() -> Profile {
        Profile {
            provider: Some("custom_api".into()),
            base_url: Some("https://api.example.com/v1".into()),
            ..Default::default()
        }
    }
    fn request() -> Request {
        serde_json::from_value(json!({"url":"https://api.example.com/v1/items"})).unwrap()
    }
    fn serve(status: &str, body: &str) -> (String, thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let reply = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut raw = Vec::new();
            let mut bytes = [0; 4096];
            loop {
                let count = stream.read(&mut bytes).unwrap();
                raw.extend_from_slice(&bytes[..count]);
                let text = String::from_utf8_lossy(&raw);
                if let Some(end) = text.find("\r\n\r\n") {
                    let length = text[..end]
                        .lines()
                        .find_map(|line| {
                            line.to_lowercase()
                                .strip_prefix("content-length: ")
                                .and_then(|s| s.parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if raw.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            stream.write_all(reply.as_bytes()).unwrap();
            String::from_utf8(raw).unwrap()
        });
        (format!("http://{address}"), handle)
    }
    #[test]
    fn scope_rejects_ambiguous_and_escaped_boundaries() {
        for url in [
            "/v1/items",
            "https://evil.example/v1",
            "https://api.example.com:444/v1",
            "https://api.example.com/v10",
            "https://api.example.com/v1/../admin",
            "https://api.example.com/v1/%2e%2e/admin",
            "https://api.example.com/v1/%2fadmin",
            "https://api.example.com/v1/%252e",
            "https://api.example.com/v1//items",
            "https://user@api.example.com/v1",
            "https://api.example.com/v1#fragment",
        ] {
            assert!(scoped("https://api.example.com/v1", url).is_err(), "{url}");
        }
        assert!(scoped(
            "https://api.example.com:443/v1",
            "https://api.example.com/v1/hello%20world?q=yes"
        )
        .is_ok());
    }
    #[test]
    fn special_addresses_are_refused() {
        for ip in [
            "127.0.0.1",
            "10.0.0.1",
            "169.254.169.254",
            "100.64.0.1",
            "192.0.2.1",
            "198.18.0.1",
            "224.0.0.1",
            "::1",
            "::ffff:8.8.8.8",
            "fc00::1",
            "2001:db8::1",
            "2002:0808:0808::1",
        ] {
            assert!(!public(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["8.8.8.8", "2606:4700:4700::1111"] {
            assert!(public(ip.parse().unwrap()));
        }
    }
    #[test]
    fn auth_and_secret_headers_are_validated_and_redacted() {
        let mut profile = profile();
        profile.auth_type = Some("api_key_header".into());
        profile.api_key_header = Some("Authorization".into());
        profile.api_token = Some("Token fixture-credential".into());
        profile.headers.push(CustomHeader {
            name: "x-secret".into(),
            value: Some("fixture-header".into()),
            secret: true,
            ..Default::default()
        });
        let (headers, secrets) = credentials(&profile).unwrap();
        assert_eq!(headers["authorization"], "Token fixture-credential");
        assert!(secrets.contains(&"fixture-header".into()));
        profile.headers.push(CustomHeader {
            name: "X-Secret".into(),
            value: Some("duplicate".into()),
            ..Default::default()
        });
        assert!(credentials(&profile).is_err());
        profile.headers.clear();
        profile.auth_type = Some("basic".into());
        profile.username = Some("user".into());
        profile.password = Some("fixture-password".into());
        let (headers, _) = credentials(&profile).unwrap();
        assert_eq!(
            headers["authorization"],
            format!("Basic {}", STANDARD.encode("user:fixture-password"))
        );
    }
    #[tokio::test]
    async fn gateway_forwards_complete_request_without_upstream_credentials() {
        let (url, server) = serve("200 OK", r#"{"status":201,"body":{"ok":true}}"#);
        let mut profile = profile();
        profile.gateway_url = Some(format!("{url}/gateway/v1/custom-api/request"));
        profile.gateway_token = Some("fixture-gateway".into());
        let mut request = request();
        request.method = "POST".into();
        request.query.insert("limit".into(), json!(10));
        request.body = json!({"name":"sample"});
        let result = execute(&profile, request).await.unwrap();
        assert_eq!(result, json!({"status":201,"body":{"ok":true}}));
        let raw = server.join().unwrap();
        assert!(raw.starts_with("POST /gateway/v1/custom-api/request"));
        assert!(raw.contains("authorization: Bearer fixture-gateway"));
        let wire: Value = serde_json::from_str(raw.split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(wire["query"]["limit"], 10);
        assert_eq!(wire["body"]["name"], "sample");
    }
    #[tokio::test]
    async fn gateway_failures_and_redirects_never_expose_response_or_token() {
        for status in ["403 Forbidden", "302 Found"] {
            let (url, server) = serve(status, "fixture-gateway");
            let mut profile = profile();
            profile.gateway_url = Some(url);
            profile.gateway_token = Some("fixture-gateway".into());
            let error = execute(&profile, request()).await.unwrap_err();
            server.join().unwrap();
            assert!(!error.to_json_line().contains("fixture-gateway"));
        }
    }
    #[tokio::test]
    async fn direct_response_is_bounded_parsed_and_sanitized() {
        let (url, server) = serve("200 OK", r#"{"echo":"fixture-secret"}"#);
        let response = client_builder()
            .build()
            .unwrap()
            .get(url)
            .send()
            .await
            .unwrap();
        let body = read_response(response, &["fixture-secret".into()])
            .await
            .unwrap();
        server.join().unwrap();
        assert_eq!(body, json!({"echo":"[REDACTED]"}));
        let (url, server) = serve("200 OK", "plain text");
        let response = client_builder()
            .build()
            .unwrap()
            .get(url)
            .send()
            .await
            .unwrap();
        assert_eq!(
            read_response(response, &[]).await.unwrap(),
            json!("plain text")
        );
        server.join().unwrap();
    }
    #[tokio::test]
    async fn direct_private_destinations_and_mixed_isolated_profiles_fail_closed() {
        let mut profile = profile();
        profile.base_url = Some("https://127.0.0.1".into());
        let mut target = request();
        target.url = "https://127.0.0.1/".into();
        assert!(execute(&profile, target)
            .await
            .unwrap_err()
            .message
            .contains("Private"));
        profile = Profile {
            provider: Some("custom_api".into()),
            base_url: Some("https://api.example.com/v1".into()),
            ..Default::default()
        };
        profile.gateway_url = Some("http://127.0.0.1:9".into());
        profile.token = Some("never-send".into());
        assert!(execute(&profile, request())
            .await
            .unwrap_err()
            .message
            .contains("Isolated"));
    }
}

#[cfg(test)]
mod direct_transport_tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };
    #[tokio::test]
    async fn direct_transport_attaches_credentials_query_and_body_and_refuses_redirects() {
        for status in ["201 Created", "302 Found"] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let url = Url::parse(&format!(
                "http://{}/v1/items?existing=yes",
                listener.local_addr().unwrap()
            ))
            .unwrap();
            let reply =
                format!("HTTP/1.1 {status}\r\nContent-Length: 4\r\nConnection: close\r\n\r\ntrue");
            let server = thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                let mut bytes = [0; 8192];
                let count = stream.read(&mut bytes).unwrap();
                let raw = String::from_utf8_lossy(&bytes[..count]).into_owned();
                stream.write_all(reply.as_bytes()).unwrap();
                raw
            });
            let profile = Profile {
                auth_type: Some("bearer_token".into()),
                token: Some("fixture-token".into()),
                ..Default::default()
            };
            let request:Request=serde_json::from_value(json!({"url":url.as_str(),"method":"POST","query":{"q":"hello world"},"body":{"ok":true}})).unwrap();
            let result = send_direct(
                &client_builder().build().unwrap(),
                &profile,
                request,
                url,
                Method::POST,
            )
            .await;
            let raw = server.join().unwrap();
            assert!(raw.contains("authorization: Bearer fixture-token"));
            assert!(raw.contains("existing=yes&q=hello+world"));
            if status.starts_with("201") {
                assert_eq!(result.unwrap(), json!({"status":201,"body":true}));
            } else {
                assert!(result.unwrap_err().message.contains("Redirects"));
            }
        }
    }
}

#[cfg(test)]
mod regression_tests {
    use super::*;
    #[test]
    fn documentation_prefixes_do_not_block_neighboring_public_networks() {
        for ip in ["192.0.2.8", "198.51.100.8"] {
            assert!(!public(ip.parse().unwrap()));
        }
        for ip in ["192.2.1.8", "198.51.1.8"] {
            assert!(public(ip.parse().unwrap()));
        }
    }
    #[test]
    fn redaction_covers_escaped_strings_keys_and_json_primitives() {
        let value = json!({"123456789":123456789,"nested":[true,null,"quote\"secret"]});
        let clean = redact(
            value,
            &[
                "123456789".into(),
                "true".into(),
                "null".into(),
                "quote\"secret".into(),
            ],
        );
        assert_eq!(
            clean,
            json!({"[REDACTED]":"[REDACTED]","nested":["[REDACTED]","[REDACTED]","[REDACTED]"]})
        );
    }
    #[test]
    fn invalid_percent_escapes_and_double_encoding_are_refused() {
        for path in [
            "%", "%GG", "%2", "%252f", "%00", "%7f", "%5c", "%2F", "%2E%2E",
        ] {
            assert!(
                scoped(
                    "https://example.com/v1",
                    &format!("https://example.com/v1/{path}")
                )
                .is_err(),
                "{path}"
            );
        }
    }
}
