//! Scope, credentials and bounded execution for user-configured HTTP APIs.
use crate::{config::Profile, error::AppError};
use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client, Method, Url,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::{collections::HashSet, net::IpAddr, time::Duration};
const LIMIT: usize = 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(30);
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
    pub headers: Vec<RequestHeader>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequestHeader {
    pub name: String,
    pub value: String,
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
/// Config-time check for `base_url`, shared with `config profiles validate`.
pub(crate) fn check_base_url(value: &str) -> Result<(), AppError> {
    parse_url(value, true).map(drop)
}
pub(crate) fn parse_gateway_url(value: &str) -> Result<Url, AppError> {
    let gateway = Url::parse(value).map_err(|_| refused("Invalid gateway URL"))?;
    if !matches!(gateway.scheme(), "http" | "https")
        || !gateway.username().is_empty()
        || gateway.password().is_some()
        || gateway.fragment().is_some()
    {
        return Err(refused("Invalid gateway URL"));
    }
    Ok(gateway)
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
fn printable_ascii(value: &str) -> Result<(), AppError> {
    if value.bytes().any(|b| !(32..=126).contains(&b)) {
        return Err(refused("Header value must be printable ASCII"));
    }
    Ok(())
}
fn header(name: &str, value: &str, auth: bool) -> Result<(HeaderName, HeaderValue), AppError> {
    let name =
        HeaderName::from_bytes(name.as_bytes()).map_err(|_| refused("Invalid header name"))?;
    if reserved(name.as_str()) && !(auth && name == "authorization") {
        return Err(refused("Reserved header name"));
    }
    printable_ascii(value)?;
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
            // The username is an identifier, not a secret; redacting it would mangle
            // any response that mentions it.
            printable_ascii(&user)?;
            let password = required(&profile.password)?;
            printable_ascii(&password)?;
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
}
pub async fn execute(profile: &Profile, request: Request) -> Result<Value, AppError> {
    with_deadline(TIMEOUT, run(profile, request)).await
}
/// One deadline over DNS, sending and reading. ponytail: a stalled system resolver thread
/// can still delay process exit after the error is reported; tokio cannot cancel it.
async fn with_deadline(
    limit: Duration,
    work: impl std::future::Future<Output = Result<Value, AppError>>,
) -> Result<Value, AppError> {
    tokio::time::timeout(limit, work)
        .await
        .map_err(|_| AppError::internal("custom_api", "request", "Custom API request timed out"))?
}
async fn run(profile: &Profile, request: Request) -> Result<Value, AppError> {
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
        let (name, _) = header(&item.name, &item.value, false)?;
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
        let gateway = parse_gateway_url(gateway)?;
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
    let addresses = tokio::net::lookup_host((
        host.trim_start_matches('[').trim_end_matches(']'),
        url.port_or_known_default().unwrap_or(443),
    ))
    .await
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
        let (name, value) = header(&item.name, &item.value, false)?;
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
    // Only navigation statuses; 304 and other 3xx are ordinary results.
    if matches!(status.as_u16(), 301 | 302 | 303 | 307 | 308) {
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
    use crate::config::CustomHeader;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    const BASE: &str = "https://api.example.com/v1";

    fn profile() -> Profile {
        Profile {
            provider: Some("custom_api".into()),
            base_url: Some(BASE.into()),
            ..Default::default()
        }
    }

    /// A profile whose base is a loopback literal: requests that pass validation stop at the
    /// private-destination check, so reaching that message proves earlier checks passed.
    fn private_profile() -> Profile {
        Profile {
            base_url: Some("https://127.0.0.1/v1".into()),
            ..profile()
        }
    }

    fn request(value: Value) -> Request {
        serde_json::from_value(value).unwrap()
    }

    fn items() -> Request {
        request(json!({"url": format!("{BASE}/items")}))
    }

    fn header_item(name: &str, value: &str) -> CustomHeader {
        CustomHeader {
            name: name.into(),
            value: Some(value.into()),
            ..Default::default()
        }
    }

    fn reply(status: &str, body: &[u8]) -> Vec<u8> {
        let mut reply = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .into_bytes();
        reply.extend_from_slice(body);
        reply
    }

    /// Accepts one connection, reads the complete request, writes `reply` and returns the
    /// raw request. Write errors are ignored so clients may abort oversized responses.
    fn serve(reply: Vec<u8>) -> (String, thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
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
            let _ = stream.write_all(&reply);
            String::from_utf8(raw).unwrap()
        });
        (format!("http://{address}"), handle)
    }

    fn body_of(raw: &str) -> &str {
        raw.split_once("\r\n\r\n").map_or("", |(_, body)| body)
    }

    async fn fetch(reply: Vec<u8>, secrets: &[String]) -> Result<Value, AppError> {
        let (url, server) = serve(reply);
        let response = client_builder()
            .build()
            .unwrap()
            .get(url)
            .send()
            .await
            .unwrap();
        let result = read_response(response, secrets).await;
        server.join().unwrap();
        result
    }

    async fn direct(
        profile: &Profile,
        request: Request,
        status: &str,
        body: &str,
    ) -> (Result<Value, AppError>, String) {
        let (base, server) = serve(reply(status, body.as_bytes()));
        let url = Url::parse(&format!("{base}/v1/items?existing=yes")).unwrap();
        let method = Method::from_bytes(request.method.to_uppercase().as_bytes()).unwrap();
        let client = client_builder().build().unwrap();
        let result = send_direct(&client, profile, request, url, method).await;
        (result, server.join().unwrap())
    }

    fn gateway_profile(url: &str) -> Profile {
        Profile {
            gateway_url: Some(url.into()),
            gateway_token: Some("fixture-gateway".into()),
            ..profile()
        }
    }

    // URL scope

    #[test]
    fn scope_rejects_ambiguous_escaped_and_foreign_targets() {
        for url in [
            "/v1/items",
            "api.example.com/v1/items",
            "http://api.example.com/v1/items",
            "https://evil.example/v1",
            "https://api.example.com.evil.example/v1",
            "https://api.example.com:444/v1",
            "https://api.example.com:0/v1",
            "https://api.example.com./v1",
            "https://api.example.com/v10",
            "https://api.example.com/v",
            "https://api.example.com/",
            "https://api.example.com/v1/../admin",
            "https://api.example.com/v1/./items",
            "https://api.example.com/v1/%2e%2e/admin",
            "https://api.example.com/v1/%2fadmin",
            "https://api.example.com/v1/%252e",
            "https://api.example.com/v1//items",
            "https://api.example.com/v1/a b",
            "https://api.example.com/v1/a\tb",
            "https://api.example.com/v1\\items",
            "https://user@api.example.com/v1",
            "https://:pw@api.example.com/v1",
            "https://user:pw@api.example.com/v1",
            "https://api.example.com/v1#fragment",
        ] {
            assert!(scoped(BASE, url).is_err(), "{url}");
        }
    }

    #[test]
    fn scope_accepts_the_base_path_and_its_descendants() {
        for (base, url) in [
            (BASE, "https://api.example.com/v1"),
            (BASE, "https://api.example.com/v1/"),
            (BASE, "https://api.example.com/v1/a/b?x=1&y=2"),
            (BASE, "https://API.EXAMPLE.COM/v1/items"),
            (BASE, "https://api.example.com:443/v1/items"),
            (
                "https://api.example.com:443/v1",
                "https://api.example.com/v1/hello%20world?q=yes",
            ),
            ("https://api.example.com/v1/", "https://api.example.com/v1"),
            (
                "https://api.example.com",
                "https://api.example.com/any/path",
            ),
            ("https://api.example.com/", "https://api.example.com/"),
        ] {
            let target = scoped(base, url).unwrap_or_else(|_| panic!("{base} {url}"));
            assert_eq!(target.host_str(), Some("api.example.com"));
        }
    }

    #[test]
    fn base_url_must_be_a_plain_https_url() {
        for base in [
            "http://api.example.com/v1",
            "https://api.example.com/v1?key=1",
            "https://user:pw@api.example.com/v1",
            "https://api.example.com/v1#x",
            "api.example.com/v1",
        ] {
            assert!(
                scoped(base, "https://api.example.com/v1/items").is_err(),
                "{base}"
            );
        }
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

    // Destination addresses

    #[test]
    fn special_addresses_are_refused() {
        for ip in [
            "0.0.0.0",
            "0.1.2.3",
            "127.0.0.1",
            "10.0.0.1",
            "172.16.0.1",
            "192.168.1.1",
            "169.254.169.254",
            "100.64.0.1",
            "100.127.255.255",
            "192.0.0.8",
            "192.0.2.1",
            "192.88.99.1",
            "198.18.0.1",
            "198.19.255.255",
            "198.51.100.8",
            "203.0.113.5",
            "224.0.0.1",
            "240.0.0.1",
            "255.255.255.255",
            "::",
            "::1",
            "::ffff:8.8.8.8",
            "64:ff9b::808:808",
            "fc00::1",
            "fe80::1",
            "ff02::1",
            "2001::1",
            "2001:db8::1",
            "2002:0808:0808::1",
            "3fff::1",
        ] {
            assert!(!public(ip.parse().unwrap()), "{ip}");
        }
    }

    #[test]
    fn public_addresses_next_to_special_ranges_are_allowed() {
        for ip in [
            "8.8.8.8",
            "1.1.1.1",
            "100.63.255.255",
            "100.128.0.1",
            "192.2.1.8",
            "198.20.0.1",
            "198.51.1.8",
            "2606:4700:4700::1111",
            "2001:4860::8888",
        ] {
            assert!(public(ip.parse().unwrap()), "{ip}");
        }
    }

    // Header validation

    #[test]
    fn reserved_and_malformed_headers_are_refused() {
        for name in [
            "Authorization",
            "proxy-authorization",
            "HOST",
            "Connection",
            "content-length",
            "Transfer-Encoding",
            "upgrade",
            "TE",
            "trailer",
            "keep-alive",
            "proxy-connection",
        ] {
            assert_eq!(
                header(name, "x", false).unwrap_err().message,
                "Reserved header name",
                "{name}"
            );
        }
        assert!(header("Authorization", "x", true).is_ok());
        assert!(header("Host", "x", true).is_err());
        assert_eq!(
            header("Bad Name", "x", false).unwrap_err().message,
            "Invalid header name"
        );
        for value in ["caf\u{e9}", "a\u{1}b", "tab\there", "line\nbreak", "\u{7f}"] {
            assert_eq!(
                header("X-Value", value, false).unwrap_err().message,
                "Header value must be printable ASCII",
                "{value:?}"
            );
        }
        let (name, value) = header("X-Trace", "abc 123 ~!", false).unwrap();
        assert_eq!(
            (name.as_str(), value.to_str().unwrap()),
            ("x-trace", "abc 123 ~!")
        );
    }

    // Credentials

    #[test]
    fn no_auth_sends_configured_headers_and_only_secret_ones_are_redacted() {
        let mut profile = profile();
        profile.headers = vec![
            header_item("X-Workspace", "plain-value"),
            CustomHeader {
                secret: true,
                ..header_item("X-Secret", "flagged-secret")
            },
            CustomHeader {
                value_env: Some("UNUSED".into()),
                ..header_item("X-From-Env", "env-secret")
            },
            CustomHeader {
                value_secret: Some("unused.key".into()),
                ..header_item("X-From-Store", "store-secret")
            },
            header_item("X-Empty", ""),
        ];
        let (headers, secrets) = credentials(&profile).unwrap();
        assert_eq!(headers.len(), 5);
        assert!(headers.get("authorization").is_none());
        assert_eq!(headers["x-workspace"], "plain-value");
        assert_eq!(headers["x-empty"], "");
        assert_eq!(secrets, ["flagged-secret", "env-secret", "store-secret"]);
    }

    #[test]
    fn bearer_token_is_attached_and_redacted() {
        let profile = Profile {
            auth_type: Some("bearer_token".into()),
            token: Some("fixture-token".into()),
            ..profile()
        };
        let (headers, secrets) = credentials(&profile).unwrap();
        assert_eq!(headers["authorization"], "Bearer fixture-token");
        assert!(secrets.contains(&"fixture-token".into()));
        assert!(secrets.contains(&"Bearer fixture-token".into()));
    }

    #[test]
    fn api_key_header_uses_the_configured_name_including_authorization() {
        let mut profile = Profile {
            auth_type: Some("api_key_header".into()),
            api_key_header: Some("X-API-Key".into()),
            api_token: Some("fixture-key".into()),
            ..profile()
        };
        let (headers, secrets) = credentials(&profile).unwrap();
        assert_eq!(headers["x-api-key"], "fixture-key");
        assert!(secrets.contains(&"fixture-key".into()));

        profile.api_key_header = Some("Authorization".into());
        profile.api_token = Some("Token fixture-credential".into());
        let (headers, _) = credentials(&profile).unwrap();
        assert_eq!(headers["authorization"], "Token fixture-credential");

        profile.api_key_header = Some("Host".into());
        assert_eq!(
            credentials(&profile).unwrap_err().message,
            "Reserved header name"
        );
    }

    #[test]
    fn basic_auth_encodes_credentials_and_redacts_only_secrets() {
        let profile = Profile {
            auth_type: Some("basic".into()),
            username: Some("user".into()),
            password: Some("fixture-password".into()),
            ..profile()
        };
        let (headers, secrets) = credentials(&profile).unwrap();
        let encoded = STANDARD.encode("user:fixture-password");
        assert_eq!(headers["authorization"], format!("Basic {encoded}"));
        assert!(secrets.contains(&"fixture-password".into()));
        assert!(secrets.contains(&encoded));
        assert!(!secrets.contains(&"user".into()));
    }

    #[test]
    fn invalid_or_missing_credentials_are_refused() {
        let cases = [
            (
                Profile {
                    auth_type: Some("bearer_token".into()),
                    ..profile()
                },
                "Missing configured credential",
            ),
            (
                Profile {
                    auth_type: Some("bearer_token".into()),
                    token: Some(String::new()),
                    ..profile()
                },
                "Missing configured credential",
            ),
            (
                Profile {
                    auth_type: Some("api_key_header".into()),
                    api_key_header: Some("X-Key".into()),
                    ..profile()
                },
                "Missing configured credential",
            ),
            (
                Profile {
                    auth_type: Some("api_key_header".into()),
                    api_token: Some("key".into()),
                    ..profile()
                },
                "Missing API key header name",
            ),
            (
                Profile {
                    auth_type: Some("basic".into()),
                    password: Some("pw".into()),
                    ..profile()
                },
                "Missing configured credential",
            ),
            (
                Profile {
                    auth_type: Some("basic".into()),
                    username: Some("user".into()),
                    ..profile()
                },
                "Missing configured credential",
            ),
            (
                Profile {
                    auth_type: Some("basic".into()),
                    username: Some("a:b".into()),
                    password: Some("pw".into()),
                    ..profile()
                },
                "Basic username cannot contain colon",
            ),
            (
                Profile {
                    auth_type: Some("basic".into()),
                    username: Some("us\u{e9}r".into()),
                    password: Some("pw".into()),
                    ..profile()
                },
                "Header value must be printable ASCII",
            ),
            (
                Profile {
                    auth_type: Some("basic".into()),
                    username: Some("user".into()),
                    password: Some("pw\n".into()),
                    ..profile()
                },
                "Header value must be printable ASCII",
            ),
            (
                Profile {
                    auth_type: Some("bearer_token".into()),
                    token: Some("bad\ntoken".into()),
                    ..profile()
                },
                "Header value must be printable ASCII",
            ),
            (
                Profile {
                    auth_type: Some("oauth2".into()),
                    token: Some("t".into()),
                    ..profile()
                },
                "Unsupported Custom API authentication",
            ),
        ];
        for (profile, message) in cases {
            let error = credentials(&profile).unwrap_err();
            assert_eq!(error.code, "invalid_input");
            assert_eq!(error.message, message, "{:?}", profile.auth_type);
        }
    }

    #[test]
    fn invalid_configured_headers_are_refused() {
        let cases = [
            (
                vec![CustomHeader {
                    name: "X-Missing".into(),
                    ..Default::default()
                }],
                "Missing configured header value",
            ),
            (
                vec![CustomHeader {
                    secret: true,
                    ..header_item("X-Secret", "")
                }],
                "Secret header value must not be empty",
            ),
            (
                vec![header_item("X-Secret", "a"), header_item("x-secret", "b")],
                "Duplicate configured header",
            ),
            (vec![header_item("Host", "a")], "Reserved header name"),
            (
                vec![header_item("Authorization", "a")],
                "Reserved header name",
            ),
            (vec![header_item("Bad Name", "a")], "Invalid header name"),
        ];
        for (headers, message) in cases {
            let profile = Profile {
                headers,
                ..profile()
            };
            assert_eq!(credentials(&profile).unwrap_err().message, message);
        }
        let profile = Profile {
            auth_type: Some("api_key_header".into()),
            api_key_header: Some("X-API-Key".into()),
            api_token: Some("key".into()),
            headers: vec![header_item("x-api-key", "other")],
            ..profile()
        };
        assert_eq!(
            credentials(&profile).unwrap_err().message,
            "Duplicate configured header"
        );
    }

    // Request validation (no network: every case fails before a connection is made)

    #[tokio::test]
    async fn execute_validates_profile_scope_method_and_limits_before_sending() {
        let invalid = |message: &'static str, profile: Profile, request: Request| async move {
            let error = execute(&profile, request).await.unwrap_err();
            assert_eq!(error.code, "invalid_input", "{message}");
            assert_eq!(error.message, message);
        };
        let target = "https://127.0.0.1/v1/items";
        invalid("Select a custom_api profile", Profile::default(), items()).await;
        invalid(
            "Select a custom_api profile",
            Profile {
                provider: Some("github".into()),
                ..profile()
            },
            items(),
        )
        .await;
        invalid(
            "Missing base_url",
            Profile {
                base_url: None,
                ..profile()
            },
            items(),
        )
        .await;
        invalid(
            "URL is outside configured origin and base path",
            profile(),
            request(json!({"url":"https://api.example.com/v2/items"})),
        )
        .await;
        invalid(
            "Invalid HTTP method",
            private_profile(),
            request(json!({"url":target,"method":"GE T"})),
        )
        .await;
        for method in ["TRACE", "CONNECT", "PROPFIND"] {
            invalid(
                "Unsupported HTTP method",
                private_profile(),
                request(json!({"url":target,"method":method})),
            )
            .await;
        }
        invalid(
            "Request exceeds limits",
            private_profile(),
            request(json!({"url":target,"body":"x".repeat(LIMIT)})),
        )
        .await;
        let headers = (0..33)
            .map(|i| json!({"name":format!("X-H{i}"),"value":"v"}))
            .collect::<Vec<_>>();
        invalid(
            "Request exceeds limits",
            private_profile(),
            request(json!({"url":target,"headers":headers})),
        )
        .await;
        invalid(
            "Duplicate request header",
            private_profile(),
            request(json!({"url":target,"headers":[{"name":"X-A","value":"1"},{"name":"x-a","value":"2"}]})),
        )
        .await;
        for name in ["Host", "Authorization", "Transfer-Encoding"] {
            invalid(
                "Reserved header name",
                private_profile(),
                request(json!({"url":target,"headers":[{"name":name,"value":"1"}]})),
            )
            .await;
        }
        invalid(
            "Header value must be printable ASCII",
            private_profile(),
            request(json!({"url":target,"headers":[{"name":"X-A","value":"a\r\nInjected: 1"}]})),
        )
        .await;
        invalid(
            "gateway_token requires gateway_url",
            Profile {
                gateway_token: Some("orphan".into()),
                ..private_profile()
            },
            request(json!({"url":target})),
        )
        .await;
    }

    #[tokio::test]
    async fn requests_within_limits_reach_destination_checks() {
        let headers = (0..32)
            .map(|i| json!({"name":format!("X-H{i}"),"value":"v"}))
            .collect::<Vec<_>>();
        for request in [
            request(json!({"url":"https://127.0.0.1/v1","method":"delete"})),
            request(json!({"url":"https://127.0.0.1/v1","headers":headers})),
            request(json!({"url":"https://127.0.0.1/v1","body":"x".repeat(LIMIT - 2)})),
        ] {
            let error = execute(&private_profile(), request).await.unwrap_err();
            assert!(error.message.starts_with("Private"), "{}", error.message);
        }
    }

    #[tokio::test]
    async fn private_and_special_destinations_are_refused_after_resolution() {
        for base in [
            "https://127.0.0.1",
            "https://[::1]",
            "https://10.1.2.3",
            "https://169.254.169.254",
            "https://localhost",
        ] {
            let profile = Profile {
                base_url: Some(base.into()),
                ..profile()
            };
            let error = execute(&profile, request(json!({"url":format!("{base}/")})))
                .await
                .unwrap_err();
            assert_eq!(error.code, "invalid_input", "{base}");
            assert!(error.message.starts_with("Private"), "{base}");
        }
    }

    // Gateway transport

    #[tokio::test]
    async fn gateway_forwards_the_complete_request_with_only_the_gateway_token() {
        let (url, server) = serve(reply("200 OK", br#"{"status":201,"body":{"ok":true}}"#));
        let profile = gateway_profile(&format!("{url}/gateway/v1/custom-api/request"));
        let request = request(json!({
            "method":"POST",
            "url":"https://api.example.com/v1/items",
            "query":{"limit":10,"tag":["a","b"]},
            "body":{"name":"sample"},
            "headers":[{"name":"X-Trace","value":"1"}]
        }));
        let result = execute(&profile, request).await.unwrap();
        assert_eq!(result, json!({"status":201,"body":{"ok":true}}));
        let raw = server.join().unwrap();
        assert!(raw.starts_with("POST /gateway/v1/custom-api/request HTTP/1.1"));
        assert!(raw.contains("authorization: Bearer fixture-gateway"));
        assert!(raw.contains("content-type: application/json"));
        let wire: Value = serde_json::from_str(body_of(&raw)).unwrap();
        assert_eq!(
            wire,
            json!({
                "method":"POST",
                "url":"https://api.example.com/v1/items",
                "query":{"limit":10,"tag":["a","b"]},
                "body":{"name":"sample"},
                "headers":[{"name":"X-Trace","value":"1"}]
            })
        );
    }

    #[tokio::test]
    async fn gateway_success_body_redacts_the_gateway_token() {
        let (url, server) = serve(reply(
            "200 OK",
            br#"{"status":200,"body":{"echo":"fixture-gateway"}}"#,
        ));
        let result = execute(&gateway_profile(&url), items()).await.unwrap();
        server.join().unwrap();
        assert_eq!(result, json!({"status":200,"body":{"echo":"[REDACTED]"}}));
    }

    #[tokio::test]
    async fn gateway_http_failures_are_structured_and_never_expose_response_or_token() {
        for (status, code, http) in [
            ("401 Unauthorized", "auth_error", 401),
            ("403 Forbidden", "auth_error", 403),
            ("404 Not Found", "not_found", 404),
            ("429 Too Many Requests", "rate_limited", 429),
            ("500 Internal Server Error", "provider_api_error", 500),
            ("302 Found", "provider_api_error", 302),
        ] {
            let (url, server) = serve(reply(status, b"fixture-gateway leaked upstream detail"));
            let error = execute(&gateway_profile(&url), items()).await.unwrap_err();
            server.join().unwrap();
            assert_eq!((error.code, error.status), (code, Some(http)), "{status}");
            assert_eq!(error.message, "Custom API gateway refused request");
            let line = error.to_json_line();
            assert!(!line.contains("fixture-gateway") && !line.contains("leaked"));
        }
    }

    #[tokio::test]
    async fn gateway_success_without_an_envelope_is_unavailable() {
        for body in [
            r#"{"ok":true}"#,
            r#"{"status":"200","body":1}"#,
            r#"{"status":200}"#,
            r#"{"body":1}"#,
            "plain text",
        ] {
            let (url, server) = serve(reply("200 OK", body.as_bytes()));
            let error = execute(&gateway_profile(&url), items()).await.unwrap_err();
            server.join().unwrap();
            assert_eq!(error.code, "internal_error", "{body}");
            assert_eq!(error.message, "Custom API upstream unavailable");
        }
    }

    #[tokio::test]
    async fn unreachable_gateway_is_unavailable_and_never_falls_back_to_direct() {
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let error = execute(
            &gateway_profile(&format!("http://127.0.0.1:{port}")),
            items(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, "internal_error");
        assert_eq!(error.message, "Custom API upstream unavailable");
    }

    #[tokio::test]
    async fn isolated_profiles_refuse_upstream_credentials_and_bad_gateway_config() {
        let gateway = "http://127.0.0.1:9";
        for profile in [
            Profile {
                token: Some("t".into()),
                ..gateway_profile(gateway)
            },
            Profile {
                api_token: Some("t".into()),
                ..gateway_profile(gateway)
            },
            Profile {
                password: Some("t".into()),
                ..gateway_profile(gateway)
            },
            Profile {
                headers: vec![header_item("X-A", "1")],
                ..gateway_profile(gateway)
            },
            Profile {
                auth_type: Some("bearer_token".into()),
                ..gateway_profile(gateway)
            },
        ] {
            assert_eq!(
                execute(&profile, items()).await.unwrap_err().message,
                "Isolated profiles cannot contain upstream credentials or headers"
            );
        }
        let explicit_none = Profile {
            auth_type: Some("none".into()),
            gateway_token: None,
            ..gateway_profile(gateway)
        };
        assert_eq!(
            execute(&explicit_none, items()).await.unwrap_err().message,
            "Missing gateway token"
        );
        for url in [
            "not a url",
            "ftp://gateway.internal/request",
            "http://user:pw@gateway.internal/request",
            "http://gateway.internal/request#frag",
        ] {
            assert_eq!(
                execute(&gateway_profile(url), items())
                    .await
                    .unwrap_err()
                    .message,
                "Invalid gateway URL",
                "{url}"
            );
        }
    }

    #[tokio::test]
    async fn isolated_profiles_still_enforce_local_scope() {
        let error = execute(
            &gateway_profile("http://127.0.0.1:9"),
            request(json!({"url":"https://evil.example/v1"})),
        )
        .await
        .unwrap_err();
        assert_eq!(
            error.message,
            "URL is outside configured origin and base path"
        );
    }

    // Direct transport

    #[tokio::test]
    async fn direct_transport_attaches_credentials_headers_query_and_json_body() {
        let profile = Profile {
            auth_type: Some("bearer_token".into()),
            token: Some("fixture-token".into()),
            headers: vec![header_item("X-Workspace", "team")],
            ..profile()
        };
        let request = request(json!({
            "url":"ignored",
            "method":"POST",
            "query":{"q":"hello world","n":10,"flag":true,"tag":["a","b"],"obj":{"k":1}},
            "body":{"ok":true},
            "headers":[{"name":"X-Trace","value":"abc"}]
        }));
        let (result, raw) = direct(&profile, request, "201 Created", "true").await;
        assert_eq!(result.unwrap(), json!({"status":201,"body":true}));
        let request_line = raw.lines().next().unwrap();
        assert!(request_line.starts_with("POST /v1/items?existing=yes&"));
        for pair in [
            "q=hello+world",
            "n=10",
            "flag=true",
            "tag=a",
            "tag=b",
            "obj=%7B%22k%22%3A1%7D",
        ] {
            assert!(request_line.contains(pair), "{pair} in {request_line}");
        }
        assert!(raw.contains("authorization: Bearer fixture-token"));
        assert!(raw.contains("x-workspace: team"));
        assert!(raw.contains("x-trace: abc"));
        assert!(raw.contains("content-type: application/json"));
        assert_eq!(body_of(&raw), r#"{"ok":true}"#);
    }

    #[tokio::test]
    async fn direct_text_body_is_sent_verbatim_and_null_body_sends_nothing() {
        let text = request(json!({"url":"ignored","method":"PUT","body":"plain <text> \"q\""}));
        let (_, raw) = direct(&profile(), text, "200 OK", "ok").await;
        assert!(raw.starts_with("PUT "));
        assert!(!raw.contains("application/json"));
        assert_eq!(body_of(&raw), "plain <text> \"q\"");

        let (_, raw) = direct(&profile(), items(), "200 OK", "ok").await;
        assert!(raw.starts_with("GET "));
        assert!(!raw.to_lowercase().contains("content-length"));
        assert_eq!(body_of(&raw), "");
    }

    #[tokio::test]
    async fn direct_upstream_errors_are_returned_in_the_envelope() {
        for (status, code) in [("404 Not Found", 404), ("500 Internal Server Error", 500)] {
            let (result, _) = direct(&profile(), items(), status, r#"{"error":"missing"}"#).await;
            assert_eq!(
                result.unwrap(),
                json!({"status":code,"body":{"error":"missing"}})
            );
        }
    }

    #[tokio::test]
    async fn direct_redirects_are_refused_not_followed() {
        for status in [
            "301 Moved Permanently",
            "302 Found",
            "303 See Other",
            "307 Temporary Redirect",
            "308 Permanent Redirect",
        ] {
            let (result, _) = direct(&profile(), items(), status, "").await;
            let error = result.unwrap_err();
            assert_eq!(error.code, "invalid_input", "{status}");
            assert_eq!(error.message, "Redirects are not followed");
        }
    }

    #[tokio::test]
    async fn direct_non_redirect_3xx_statuses_are_returned() {
        for (status, code) in [("304 Not Modified", 304), ("300 Multiple Choices", 300)] {
            let (result, _) = direct(&profile(), items(), status, "").await;
            assert_eq!(
                result.unwrap(),
                json!({"status":code,"body":""}),
                "{status}"
            );
        }
    }

    #[tokio::test]
    async fn work_past_the_deadline_times_out() {
        let error = with_deadline(
            Duration::from_millis(20),
            std::future::pending::<Result<Value, AppError>>(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, "internal_error");
        assert_eq!(error.message, "Custom API request timed out");
        let done = with_deadline(Duration::from_secs(1), async { Ok(json!(1)) }).await;
        assert_eq!(done.unwrap(), json!(1));
    }

    #[tokio::test(start_paused = true)]
    async fn execute_applies_the_overall_deadline_to_a_stalled_upstream() {
        // The listener never accepts, so the request stalls after connecting; paused time
        // jumps straight to the 30 second deadline.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let error = execute(&gateway_profile(&url), items()).await.unwrap_err();
        drop(listener);
        assert_eq!(error.code, "internal_error");
        assert_eq!(error.message, "Custom API request timed out");
    }

    #[tokio::test]
    async fn direct_response_redacts_echoed_credentials() {
        let profile = Profile {
            auth_type: Some("bearer_token".into()),
            token: Some("fixture-token".into()),
            headers: vec![CustomHeader {
                secret: true,
                ..header_item("X-Secret", "fixture-header")
            }],
            ..profile()
        };
        let (result, _) = direct(
            &profile,
            items(),
            "200 OK",
            r#"{"auth":"Bearer fixture-token","token":"fixture-token","h":"fixture-header"}"#,
        )
        .await;
        assert_eq!(
            result.unwrap(),
            json!({"status":200,"body":{"auth":"[REDACTED]","token":"[REDACTED]","h":"[REDACTED]"}})
        );
    }

    #[tokio::test]
    async fn direct_requests_cannot_override_configured_headers() {
        let profile = Profile {
            auth_type: Some("api_key_header".into()),
            api_key_header: Some("X-API-Key".into()),
            api_token: Some("key".into()),
            headers: vec![header_item("X-Workspace", "team")],
            ..profile()
        };
        let url = Url::parse("http://127.0.0.1:9/v1").unwrap();
        for name in ["x-workspace", "X-API-KEY"] {
            let request = request(json!({"url":"ignored","headers":[{"name":name,"value":"x"}]}));
            let error = send_direct(
                &client_builder().build().unwrap(),
                &profile,
                request,
                url.clone(),
                Method::GET,
            )
            .await
            .unwrap_err();
            assert_eq!(
                error.message, "Configured headers cannot be overridden",
                "{name}"
            );
        }
    }

    // Response handling

    #[tokio::test]
    async fn responses_preserve_json_or_text() {
        assert_eq!(
            fetch(reply("200 OK", br#"{"a":[1,null]}"#), &[])
                .await
                .unwrap(),
            json!({"a":[1,null]})
        );
        assert_eq!(
            fetch(reply("200 OK", b"plain text"), &[]).await.unwrap(),
            json!("plain text")
        );
        assert_eq!(fetch(reply("200 OK", b""), &[]).await.unwrap(), json!(""));
        assert_eq!(
            fetch(reply("200 OK", b"\xffok"), &[]).await.unwrap(),
            json!("\u{fffd}ok")
        );
    }

    #[tokio::test]
    async fn responses_are_bounded_to_one_mebibyte() {
        let exact = vec![b'a'; LIMIT];
        assert_eq!(
            fetch(reply("200 OK", &exact), &[])
                .await
                .unwrap()
                .as_str()
                .map(str::len),
            Some(LIMIT)
        );
        let error = fetch(reply("200 OK", &vec![b'a'; LIMIT + 1]), &[])
            .await
            .unwrap_err();
        assert_eq!(error.code, "invalid_input");
        assert_eq!(error.message, "Response exceeds 1 MiB");
    }

    #[tokio::test]
    async fn redaction_replaces_longest_secrets_first_and_ignores_empty_ones() {
        let secrets = ["abc".to_string(), "abcdef".into(), String::new()];
        assert_eq!(
            fetch(reply("200 OK", b"abcdef abc xyz"), &secrets)
                .await
                .unwrap(),
            json!("[REDACTED] [REDACTED] xyz")
        );
    }

    #[test]
    fn redaction_covers_escaped_strings_keys_and_json_primitives() {
        let value = json!({"123456789":123456789,"nested":[true,null,"quote\"secret"],"keep":1});
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
            json!({"[REDACTED]":"[REDACTED]","nested":["[REDACTED]","[REDACTED]","[REDACTED]"],"keep":1})
        );
    }
}
