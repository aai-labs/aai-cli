#[path = "http/custom_api.rs"]
pub(crate) mod custom_api;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose, Engine as _};
use reqwest::{Client, Method, RequestBuilder};
use serde_json::Value;

use crate::gateway_catalog;
use crate::gateway_protocol::{
    ExecuteMetadata, GatewayErrorBody, EXECUTE_PATH, PROTOCOL_VERSION, RESPONSE_SOURCE_HEADER,
};
use crate::{config::Profile, error::AppError};

pub(crate) fn multipart_boundary() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("----AaiCliBoundary{nanos:x}{n:x}")
}

pub struct ApiClient {
    client: Client,
    no_redirect_client: Client,
    server_grant: Option<crate::gateway_server::Grant>,
}

/// A request whose body is already-encoded bytes rather than a JSON value.
///
/// Providers that take pre-encoded payloads — multipart/related upload bodies, raw
/// media bytes — need a content type and occasionally extra headers alongside the
/// body, so they travel together instead of as five positional arguments.
pub struct BytesRequest {
    pub method: Method,
    pub url: String,
    pub content_type: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// A parsed response plus the `Location` header.
///
/// Upload protocols that hand back a session URI put it in `Location` and leave the
/// body empty, so the header cannot be dropped the way `request` drops it.
pub struct RawResponse {
    pub body: Value,
    pub location: Option<String>,
}

impl ApiClient {
    pub fn new() -> Result<Self, AppError> {
        let client = Client::builder()
            .user_agent("aai-cli/0.1")
            .build()
            .map_err(|err| AppError::internal("http", "client", err.to_string()))?;
        let no_redirect_client = Client::builder()
            .user_agent("aai-cli/0.1")
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|err| AppError::internal("http", "client", err.to_string()))?;
        Ok(Self {
            client,
            no_redirect_client,
            server_grant: None,
        })
    }

    pub fn with_server_grant(grant: crate::gateway_server::Grant) -> Result<Self, AppError> {
        let mut client = Self::new()?;
        client.client = Client::builder()
            .user_agent("aai-gateway/0.1")
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|err| AppError::internal("http", "client", err.to_string()))?;
        client.server_grant = Some(grant);
        Ok(client)
    }

    fn check_server_grant(
        &self,
        service: &'static str,
        operation: &'static str,
        method: &Method,
    ) -> Result<(), AppError> {
        if let Some(grant) = &self.server_grant {
            if operation == "request" {
                return Err(AppError::gateway_protocol(
                    service,
                    operation,
                    "generic requests are not available through the gateway",
                ));
            }
            if !grant.permits(operation, method) {
                return Err(AppError::gateway_denied(
                    service,
                    operation,
                    "gateway token is not granted this operation and method",
                ));
            }
        }
        Ok(())
    }

    fn check_server_target(
        &self,
        service: &'static str,
        operation: &'static str,
        profile: &Profile,
        url: &str,
    ) -> Result<(), AppError> {
        if self.server_grant.is_some()
            && !crate::gateway_server::target_allowed(service, url, profile)
        {
            return Err(AppError::gateway_protocol(
                service,
                operation,
                "gateway request target is not allowed",
            ));
        }
        Ok(())
    }

    /// Capture downloads or bounded SSE without buffering the complete response.
    /// Uses the shared authentication boundary and refuses all redirects.
    pub async fn capture_to_file(
        &self,
        service: &'static str,
        operation: &'static str,
        profile: &Profile,
        url: String,
        output: &str,
        duration_seconds: Option<u64>,
    ) -> Result<Value, AppError> {
        self.check_server_grant(service, operation, &Method::GET)?;
        self.check_server_target(service, operation, profile, &url)?;
        use std::{fs::OpenOptions, io::Write, path::Path};
        use tokio::time::{timeout, timeout_at, Duration, Instant};
        let path = std::path::absolute(Path::new(output))
            .map_err(|e| AppError::invalid_input(service, operation, e.to_string()))?;
        if profile.credential_source.as_deref() == Some("gateway") {
            if duration_seconds.is_some() {
                return Err(AppError::invalid_input(
                    service,
                    operation,
                    "stream capture is not available for gateway profiles",
                ));
            }
            let bytes = self
                .download_with_accept(service, operation, profile, url, "application/octet-stream")
                .await?;
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(|e| {
                    AppError::invalid_input(
                        service,
                        operation,
                        format!("cannot create output file: {e}"),
                    )
                })?;
            file.write_all(&bytes).map_err(|e| {
                AppError::internal(service, operation, format!("failed writing output: {e}"))
            })?;
            file.flush().map_err(|e| {
                AppError::internal(service, operation, format!("failed flushing output: {e}"))
            })?;
            return Ok(serde_json::json!({
                "path": path,
                "content_type": "application/octet-stream",
                "bytes": bytes.len(),
                "termination_reason": "complete",
            }));
        }
        let token = self.bearer(service, operation, profile).await?;
        let effective = Profile {
            token: Some(token),
            ..profile.clone()
        };
        let request = apply_auth(
            self.no_redirect_client.get(url),
            service,
            operation,
            &effective,
        )?
        .header(
            "Accept",
            if duration_seconds.is_some() {
                "text/event-stream"
            } else {
                "application/octet-stream"
            },
        );
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| {
                AppError::invalid_input(
                    service,
                    operation,
                    format!("cannot create output file: {e}"),
                )
            })?;
        let partial = |message: String| {
            let mut error = AppError::internal(service, operation, message);
            error.details = Some(serde_json::json!({"partial_file": path}));
            error
        };
        let mut response = timeout(Duration::from_secs(30), request.send())
            .await
            .map_err(|_| partial("timed out waiting for response headers".into()))?
            .map_err(|e| partial(format!("capture request failed: {e}")))?;
        let status = response.status();
        if !status.is_success() {
            let body = timeout(Duration::from_secs(30), response.text())
                .await
                .map_err(|_| partial("timed out reading provider error".into()))?
                .map_err(|e| partial(format!("failed reading provider error: {e}")))?;
            let details = serde_json::from_str::<Value>(&body)
                .unwrap_or_else(|_| Value::String(body.chars().take(4096).collect()));
            let error = AppError::api(
                service,
                operation,
                status,
                format!("provider returned HTTP {}", status.as_u16()),
                Some(details),
            );
            // No response bytes were saved; do not leave an empty artifact after an HTTP error.
            drop(file);
            let _ = std::fs::remove_file(&path);
            return Err(error);
        }
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_owned();
        if duration_seconds.is_some()
            && content_type
                .split(';')
                .next()
                .is_none_or(|s| s.trim() != "text/event-stream")
        {
            return Err(partial("expected text/event-stream response".into()));
        }
        let deadline =
            duration_seconds.map(|seconds| Instant::now() + Duration::from_secs(seconds));
        let mut bytes = 0_u64;
        let reason = loop {
            let next = if let Some(deadline) = deadline {
                match timeout_at(deadline, response.chunk()).await {
                    Ok(chunk) => chunk,
                    Err(_) => break "duration_elapsed",
                }
            } else {
                timeout(Duration::from_secs(30), response.chunk())
                    .await
                    .map_err(|_| partial("download stalled; partial file retained".into()))?
            };
            match next.map_err(|e| partial(format!("transfer interrupted: {e}")))? {
                Some(chunk) => {
                    file.write_all(&chunk)
                        .map_err(|e| partial(format!("failed writing output: {e}")))?;
                    bytes += chunk.len() as u64;
                }
                None if duration_seconds.is_some() => {
                    return Err(partial(
                        "stream disconnected before requested duration; partial file retained"
                            .into(),
                    ))
                }
                None => break "complete",
            }
        };
        file.flush()
            .map_err(|e| partial(format!("failed flushing output: {e}")))?;
        Ok(
            serde_json::json!({"path":path,"content_type":content_type,"bytes":bytes,"termination_reason":reason}),
        )
    }

    /// Resolve the profile's bearer token without making a provider request.
    pub async fn bearer(
        &self,
        service: &'static str,
        operation: &'static str,
        profile: &crate::config::Profile,
    ) -> Result<String, AppError> {
        crate::oauth::resolve_token(profile, &self.client, service, operation).await
    }

    pub async fn request(
        &self,
        service: &'static str,
        operation: &'static str,
        profile: &Profile,
        method: Method,
        url: String,
        body: Option<Value>,
    ) -> Result<Value, AppError> {
        self.check_server_grant(service, operation, &method)?;
        self.check_server_target(service, operation, profile, &url)?;
        if profile.credential_source.as_deref() == Some("gateway") {
            let body = body
                .map(|value| serde_json::to_vec(&value))
                .transpose()
                .map_err(|err| AppError::internal(service, operation, err.to_string()))?;
            let response = self
                .gateway_execute(GatewayExecuteRequest {
                    service,
                    operation,
                    profile,
                    method,
                    url,
                    body,
                    content_type: Some("application/json"),
                    accept: Some("application/json"),
                    headers: Vec::new(),
                })
                .await?;
            return parse_json_response(service, operation, response);
        }
        Self::request_with(&self.client, service, operation, profile, method, url, body)
            .await
            .map(|(value, _)| value)
    }

    pub async fn request_no_redirect(
        &self,
        service: &'static str,
        operation: &'static str,
        profile: &Profile,
        method: Method,
        url: String,
        body: Option<Value>,
    ) -> Result<Value, AppError> {
        self.check_server_grant(service, operation, &method)?;
        self.check_server_target(service, operation, profile, &url)?;
        if profile.credential_source.as_deref() == Some("gateway") {
            let body = body
                .map(|value| serde_json::to_vec(&value))
                .transpose()
                .map_err(|err| AppError::internal(service, operation, err.to_string()))?;
            let response = self
                .gateway_execute(GatewayExecuteRequest {
                    service,
                    operation,
                    profile,
                    method,
                    url,
                    body,
                    content_type: Some("application/json"),
                    accept: Some("application/json"),
                    headers: Vec::new(),
                })
                .await?;
            return parse_json_response(service, operation, response);
        }
        let (value, next_url) = Self::request_with(
            &self.no_redirect_client,
            service,
            operation,
            profile,
            method,
            url,
            body,
        )
        .await?;
        Ok(attach_provider_next_url(value, next_url))
    }

    async fn request_with(
        client: &Client,
        service: &'static str,
        operation: &'static str,
        profile: &Profile,
        method: Method,
        url: String,
        body: Option<Value>,
    ) -> Result<(Value, Option<String>), AppError> {
        let token = crate::oauth::resolve_token(profile, client, service, operation).await?;
        let effective = crate::config::Profile {
            token: Some(token),
            ..profile.clone()
        };
        let mut request = client.request(method, &url);
        request = apply_auth(request, service, operation, &effective)?;
        request = request.header("Accept", "application/json");
        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await.map_err(|err| {
            AppError::internal(service, operation, format!("request failed: {err}"))
        })?;
        let status = response.status();
        let next_url = response
            .headers()
            .get(reqwest::header::LINK)
            .and_then(|value| value.to_str().ok())
            .and_then(link_next_url);
        let text = response.text().await.map_err(|err| {
            AppError::internal(
                service,
                operation,
                format!("failed to read response: {err}"),
            )
        })?;
        let parsed = if text.trim().is_empty() {
            Value::Object(serde_json::Map::new())
        } else {
            serde_json::from_str(&text).unwrap_or_else(|_| Value::String(text.clone()))
        };

        if status.is_success() {
            Ok((parsed, next_url))
        } else {
            Err(AppError::api(
                service,
                operation,
                status,
                format!("provider returned HTTP {}", status.as_u16()),
                Some(parsed),
            ))
        }
    }

    pub async fn download(
        &self,
        service: &'static str,
        operation: &'static str,
        profile: &Profile,
        url: String,
    ) -> Result<Vec<u8>, AppError> {
        let accept = match service {
            "github" => "application/json",
            "bitbucket" => "*/*",
            _ => "*/*",
        };
        self.download_with_accept(service, operation, profile, url, accept)
            .await
    }

    pub async fn download_with_accept(
        &self,
        service: &'static str,
        operation: &'static str,
        profile: &Profile,
        url: String,
        accept: &str,
    ) -> Result<Vec<u8>, AppError> {
        self.check_server_grant(service, operation, &Method::GET)?;
        self.check_server_target(service, operation, profile, &url)?;
        if profile.credential_source.as_deref() == Some("gateway") {
            let response = self
                .gateway_execute(GatewayExecuteRequest {
                    service,
                    operation,
                    profile,
                    method: Method::GET,
                    url,
                    body: None,
                    content_type: None,
                    accept: Some(accept),
                    headers: Vec::new(),
                })
                .await?;
            return response_bytes(service, operation, response);
        }
        let token = crate::oauth::resolve_token(profile, &self.client, service, operation).await?;
        let effective = crate::config::Profile {
            token: Some(token),
            ..profile.clone()
        };
        let mut target = parse_url(service, operation, &url)?;
        // Origin includes the scheme, so an http -> https hop on the same host also drops
        // credentials. reqwest compared only host and port; this is the stricter choice.
        let credentialed_origin = if self.server_grant.is_some() {
            profile
                .site_url
                .as_deref()
                .or(profile.base_url.as_deref())
                .and_then(|base| reqwest::Url::parse(base).ok())
                .map(|base| base.origin())
                .or_else(|| {
                    (!target.host_str().is_some_and(|host| {
                        crate::gateway_server::file_host_allowed(service, host)
                    }))
                    .then(|| target.origin())
                })
        } else {
            Some(target.origin())
        };
        // Redirects are followed here rather than by reqwest, which strips only the
        // standard auth headers on a cross-origin hop: a provider-specific header such as
        // Pipedrive's x-api-token would otherwise reach a signed-storage host.
        for _ in 0..=MAX_DOWNLOAD_REDIRECTS {
            self.check_server_target(service, operation, profile, target.as_str())?;
            let mut request = self
                .no_redirect_client
                .request(Method::GET, target.clone())
                .header("Accept", accept);
            if credentialed_origin.as_ref() == Some(&target.origin()) {
                request = apply_auth(request, service, operation, &effective)?;
            }
            let response = request.send().await.map_err(|err| {
                AppError::internal(service, operation, format!("request failed: {err}"))
            })?;
            if !response.status().is_redirection() {
                return read_download(service, operation, response).await;
            }
            let Some(location) = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
            else {
                return read_download(service, operation, response).await;
            };
            target = target.join(location).map_err(|err| {
                AppError::internal(
                    service,
                    operation,
                    format!("invalid redirect location {location:?}: {err}"),
                )
            })?;
        }
        Err(AppError::internal(
            service,
            operation,
            format!("download exceeded {MAX_DOWNLOAD_REDIRECTS} redirects"),
        ))
    }

    /// Send an already-encoded byte body and parse the JSON reply.
    ///
    /// `upload` below builds one specific multipart/form-data shape for Atlassian.
    /// This is the general form: the caller owns the encoding, this owns auth,
    /// execution, and error mapping.
    pub async fn request_bytes(
        &self,
        service: &'static str,
        operation: &'static str,
        profile: &Profile,
        request: BytesRequest,
    ) -> Result<RawResponse, AppError> {
        self.check_server_grant(service, operation, &request.method)?;
        self.check_server_target(service, operation, profile, &request.url)?;
        if profile.credential_source.as_deref() == Some("gateway") {
            let BytesRequest {
                method,
                url,
                content_type,
                headers,
                body,
            } = request;
            let response = self
                .gateway_execute(GatewayExecuteRequest {
                    service,
                    operation,
                    profile,
                    method,
                    url,
                    body: Some(body),
                    content_type: Some(&content_type),
                    accept: Some("application/json"),
                    headers,
                })
                .await?;
            let body = response_bytes(service, operation, response)?;
            return Ok(RawResponse {
                body: parse_body_value(&body),
                location: None,
            });
        }
        let token = crate::oauth::resolve_token(profile, &self.client, service, operation).await?;
        let effective = crate::config::Profile {
            token: Some(token),
            ..profile.clone()
        };
        let mut builder = self
            .client
            .request(request.method, &request.url)
            .header("Content-Type", request.content_type)
            .header("Accept", "application/json")
            .body(request.body);
        for (name, value) in request.headers {
            builder = builder.header(name, value);
        }
        builder = apply_auth(builder, service, operation, &effective)?;

        let response = builder.send().await.map_err(|err| {
            AppError::internal(service, operation, format!("request failed: {err}"))
        })?;
        let status = response.status();
        let location = response
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let text = response.text().await.map_err(|err| {
            AppError::internal(
                service,
                operation,
                format!("failed to read response: {err}"),
            )
        })?;
        let parsed = if text.trim().is_empty() {
            Value::Object(serde_json::Map::new())
        } else {
            serde_json::from_str(&text).unwrap_or_else(|_| Value::String(text.clone()))
        };

        if status.is_success() {
            Ok(RawResponse {
                body: parsed,
                location,
            })
        } else {
            Err(AppError::api(
                service,
                operation,
                status,
                format!("provider returned HTTP {}", status.as_u16()),
                Some(parsed),
            ))
        }
    }

    pub async fn upload(
        &self,
        service: &'static str,
        operation: &'static str,
        profile: &Profile,
        url: String,
        file_path: &str,
        comment: Option<&str>,
    ) -> Result<Value, AppError> {
        self.check_server_grant(service, operation, &Method::POST)?;
        self.check_server_target(service, operation, profile, &url)?;
        let file_bytes = std::fs::read(file_path).map_err(|e| {
            AppError::internal(service, operation, format!("failed to read file: {e}"))
        })?;
        let filename = std::path::Path::new(file_path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "file".to_string());

        let boundary = multipart_boundary();
        let mut body: Vec<u8> = Vec::new();

        // file part
        let file_header = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
        );
        body.extend_from_slice(file_header.as_bytes());
        body.extend_from_slice(&file_bytes);
        body.extend_from_slice(b"\r\n");

        // optional comment part
        if let Some(c) = comment {
            let comment_header =
                format!("--{boundary}\r\nContent-Disposition: form-data; name=\"comment\"\r\n\r\n");
            body.extend_from_slice(comment_header.as_bytes());
            body.extend_from_slice(c.as_bytes());
            body.extend_from_slice(b"\r\n");
        }

        // closing boundary
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

        let content_type = format!("multipart/form-data; boundary={boundary}");
        if profile.credential_source.as_deref() == Some("gateway") {
            let response = self
                .gateway_execute(GatewayExecuteRequest {
                    service,
                    operation,
                    profile,
                    method: Method::POST,
                    url,
                    body: Some(body),
                    content_type: Some(&content_type),
                    accept: Some("application/json"),
                    headers: vec![("X-Atlassian-Token".to_string(), "no-check".to_string())],
                })
                .await?;
            return parse_json_response(service, operation, response);
        }
        let token = crate::oauth::resolve_token(profile, &self.client, service, operation).await?;
        let effective = crate::config::Profile {
            token: Some(token),
            ..profile.clone()
        };
        let mut request = self
            .client
            .post(&url)
            .body(body)
            .header("Content-Type", content_type);
        request = apply_auth(request, service, operation, &effective)?;
        request = request.header("X-Atlassian-Token", "no-check");

        let response = request
            .send()
            .await
            .map_err(|e| AppError::internal(service, operation, format!("request failed: {e}")))?;
        let status = response.status();
        let text = response.text().await.map_err(|e| {
            AppError::internal(service, operation, format!("failed to read response: {e}"))
        })?;
        let parsed = if text.trim().is_empty() {
            Value::Object(serde_json::Map::new())
        } else {
            serde_json::from_str(&text).unwrap_or(Value::String(text))
        };
        if status.is_success() {
            Ok(parsed)
        } else {
            Err(AppError::api(
                service,
                operation,
                status,
                format!("provider returned HTTP {}", status.as_u16()),
                Some(parsed),
            ))
        }
    }

    async fn gateway_execute(
        &self,
        request: GatewayExecuteRequest<'_>,
    ) -> Result<GatewayResponse, AppError> {
        let GatewayExecuteRequest {
            service,
            operation,
            profile,
            method,
            url,
            body,
            content_type,
            accept,
            headers,
        } = request;
        if !gateway_catalog::allows(service, operation, &method) {
            return Err(AppError::invalid_input(
                service,
                operation,
                "gateway mode only supports valid typed operations",
            ));
        }
        let gateway_url = profile.gateway_url.as_deref().ok_or_else(|| {
            AppError::auth(service, operation, "gateway profile is missing gateway_url")
        })?;
        let profile_id = profile.gateway_profile_id.as_deref().ok_or_else(|| {
            AppError::auth(
                service,
                operation,
                "gateway profile is missing gateway_profile_id",
            )
        })?;
        let token = profile.gateway_token.as_deref().ok_or_else(|| {
            AppError::auth(
                service,
                operation,
                "gateway profile is missing gateway token",
            )
        })?;
        let metadata = ExecuteMetadata {
            protocol_version: PROTOCOL_VERSION,
            profile_id: profile_id.to_string(),
            service: service.to_string(),
            operation: operation.to_string(),
            method: method.to_string(),
            url,
            content_type: content_type.map(str::to_string),
            accept: accept.map(str::to_string),
            headers: headers.clone(),
        };
        let metadata_json = serde_json::to_string(&metadata).map_err(|err| {
            AppError::internal(
                service,
                operation,
                format!("failed to encode gateway request: {err}"),
            )
        })?;
        let metadata_part = reqwest::multipart::Part::text(metadata_json)
            .mime_str("application/json")
            .map_err(|err| AppError::internal(service, operation, err.to_string()))?;
        let mut form = reqwest::multipart::Form::new().part("metadata", metadata_part);
        if let Some(body) = body {
            form = form.part("body", reqwest::multipart::Part::bytes(body));
        }
        let endpoint = format!("{}{}", gateway_url.trim_end_matches('/'), EXECUTE_PATH);
        let response = self
            .no_redirect_client
            .post(endpoint)
            .bearer_auth(token)
            .multipart(form)
            .send()
            .await
            .map_err(|err| {
                AppError::internal(service, operation, format!("gateway request failed: {err}"))
            })?;
        let status = response.status();
        let source = response
            .headers()
            .get(RESPONSE_SOURCE_HEADER)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("gateway")
            .to_string();
        let body = response.bytes().await.map_err(|err| {
            AppError::internal(
                service,
                operation,
                format!("failed to read gateway response: {err}"),
            )
        })?;
        if !status.is_success() {
            if source == "provider" {
                let details = serde_json::from_slice(&body).ok().or_else(|| {
                    Some(Value::String(
                        String::from_utf8_lossy(&body).chars().take(4096).collect(),
                    ))
                });
                return Err(AppError::api(
                    service,
                    operation,
                    status,
                    format!("provider returned HTTP {}", status.as_u16()),
                    details,
                ));
            }
            let details = serde_json::from_slice::<GatewayErrorBody>(&body)
                .ok()
                .and_then(|error| serde_json::to_value(error).ok());
            return Err(AppError::gateway(
                service,
                operation,
                status,
                format!("gateway rejected request (HTTP {})", status.as_u16()),
                details,
            ));
        }
        Ok(GatewayResponse {
            status,
            body: body.to_vec(),
        })
    }
}

struct GatewayExecuteRequest<'a> {
    service: &'static str,
    operation: &'static str,
    profile: &'a Profile,
    method: Method,
    url: String,
    body: Option<Vec<u8>>,
    content_type: Option<&'a str>,
    accept: Option<&'a str>,
    headers: Vec<(String, String)>,
}

struct GatewayResponse {
    status: reqwest::StatusCode,
    body: Vec<u8>,
}

fn parse_body_value(body: &[u8]) -> Value {
    if body.is_empty() {
        Value::Object(serde_json::Map::new())
    } else {
        serde_json::from_slice(body)
            .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(body).into_owned()))
    }
}

fn parse_json_response(
    service: &'static str,
    operation: &'static str,
    response: GatewayResponse,
) -> Result<Value, AppError> {
    if !response.status.is_success() {
        return Err(AppError::api(
            service,
            operation,
            response.status,
            format!("provider returned HTTP {}", response.status.as_u16()),
            Some(parse_body_value(&response.body)),
        ));
    }
    Ok(parse_body_value(&response.body))
}

fn response_bytes(
    service: &'static str,
    operation: &'static str,
    response: GatewayResponse,
) -> Result<Vec<u8>, AppError> {
    if response.status.is_success() {
        Ok(response.body)
    } else {
        Err(AppError::api(
            service,
            operation,
            response.status,
            format!("provider returned HTTP {}", response.status.as_u16()),
            Some(parse_body_value(&response.body)),
        ))
    }
}

fn link_next_url(link: &str) -> Option<String> {
    link.split(',').find_map(|part| {
        let part = part.trim();
        if !part.contains("rel=\"next\"") && !part.contains("rel=next") {
            return None;
        }
        let start = part.find('<')? + 1;
        let end = part[start..].find('>')? + start;
        Some(part[start..end].to_string())
    })
}

fn attach_provider_next_url(value: Value, next_url: Option<String>) -> Value {
    let Some(next_url) = next_url else {
        return value;
    };
    match value {
        Value::Object(mut object) => {
            object.insert(
                "_aai_provider_next_url".to_string(),
                Value::String(next_url),
            );
            Value::Object(object)
        }
        Value::Array(results) => serde_json::json!({
            "results": results,
            "_aai_provider_next_url": next_url,
        }),
        other => serde_json::json!({
            "result": other,
            "_aai_provider_next_url": next_url,
        }),
    }
}

const MAX_DOWNLOAD_REDIRECTS: usize = 10;

fn parse_url(
    service: &'static str,
    operation: &'static str,
    url: &str,
) -> Result<reqwest::Url, AppError> {
    reqwest::Url::parse(url).map_err(|err| {
        AppError::internal(service, operation, format!("invalid URL {url:?}: {err}"))
    })
}

async fn read_download(
    service: &'static str,
    operation: &'static str,
    response: reqwest::Response,
) -> Result<Vec<u8>, AppError> {
    let status = response.status();
    let bytes = response.bytes().await.map_err(|err| {
        AppError::internal(
            service,
            operation,
            format!("failed to read response: {err}"),
        )
    })?;

    if status.is_success() {
        Ok(bytes.to_vec())
    } else {
        let details = std::str::from_utf8(&bytes)
            .ok()
            .and_then(|text| serde_json::from_str(text).ok())
            .or_else(|| {
                Some(Value::String(
                    String::from_utf8_lossy(&bytes).chars().take(4096).collect(),
                ))
            });
        Err(AppError::api(
            service,
            operation,
            status,
            format!("provider returned HTTP {}", status.as_u16()),
            details,
        ))
    }
}

pub(crate) fn apply_auth(
    request: RequestBuilder,
    service: &'static str,
    operation: &'static str,
    profile: &Profile,
) -> Result<RequestBuilder, AppError> {
    let auth_type = profile.auth_type.as_deref().unwrap_or("bearer_token");
    match auth_type {
        "basic_api_token" | "basic" => {
            let username = profile
                .email
                .as_deref()
                .or(profile.username.as_deref())
                .ok_or_else(|| {
                    AppError::auth(service, operation, "profile is missing email or username")
                })?;
            let token = profile
                .api_token
                .as_deref()
                .or(profile.token.as_deref())
                .ok_or_else(|| {
                    AppError::auth(service, operation, "profile is missing api_token or token")
                })?;
            Ok(request.header(
                "Authorization",
                format!(
                    "Basic {}",
                    general_purpose::STANDARD.encode(format!("{username}:{token}"))
                ),
            ))
        }
        "none" => Ok(request),
        "zoho_oauth" | "zoho-oauth" => {
            let token = profile
                .token
                .as_deref()
                .or(profile.api_token.as_deref())
                .ok_or_else(|| AppError::auth(service, operation, "profile is missing token"))?;
            Ok(request.header("Authorization", format!("Zoho-oauthtoken {token}")))
        }
        "pipedrive_personal_token" | "pipedrive-personal-token" => {
            let token = profile
                .api_token
                .as_deref()
                .or(profile.token.as_deref())
                .ok_or_else(|| {
                    AppError::auth(service, operation, "profile is missing api_token or token")
                })?;
            Ok(request.header("x-api-token", token))
        }
        "apollo_api_key" | "apollo-api-key" => {
            let token = profile
                .api_token
                .as_deref()
                .or(profile.token.as_deref())
                .ok_or_else(|| {
                    AppError::auth(service, operation, "profile is missing api_token or token")
                })?;
            Ok(request.header("x-api-key", token))
        }
        "api_key_header" | "api-key-header" => {
            let token = profile
                .api_token
                .as_deref()
                .or(profile.token.as_deref())
                .ok_or_else(|| {
                    AppError::auth(service, operation, "profile is missing api_token or token")
                })?;
            let name = profile.api_key_header.as_deref().unwrap_or("X-API-Key");
            let name = reqwest::header::HeaderName::from_bytes(name.as_bytes()).map_err(|_| {
                AppError::service_config(
                    service,
                    operation,
                    format!("profile.api_key_header {name:?} is not a valid header name"),
                )
            })?;
            Ok(request.header(name, token))
        }
        "hubspot_service_key"
        | "hubspot-service-key"
        | "hubspot_legacy_private_app"
        | "hubspot-legacy-private-app" => {
            let token = profile
                .token
                .as_deref()
                .or(profile.api_token.as_deref())
                .ok_or_else(|| AppError::auth(service, operation, "profile is missing token"))?;
            Ok(request.bearer_auth(token))
        }
        "openpanel_client_credentials" | "openpanel-client-credentials" => {
            let client_id = profile.client_id.as_deref().ok_or_else(|| {
                AppError::auth(service, operation, "profile is missing client_id")
            })?;
            let client_secret = profile
                .api_token
                .as_deref()
                .or(profile.token.as_deref())
                .ok_or_else(|| {
                    AppError::auth(
                        service,
                        operation,
                        "profile is missing api_token or token (the client secret)",
                    )
                })?;
            Ok(request
                .header("openpanel-client-id", client_id)
                .header("openpanel-client-secret", client_secret))
        }
        _ => {
            let token = profile
                .token
                .as_deref()
                .or(profile.api_token.as_deref())
                .ok_or_else(|| AppError::auth(service, operation, "profile is missing token"))?;
            Ok(request.bearer_auth(token))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::serve;
    use reqwest::Method;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{Arc, Mutex},
        thread,
    };

    #[test]
    fn pipedrive_personal_token_uses_x_api_token_header() {
        let client = Client::new();
        let profile = Profile {
            auth_type: Some("pipedrive_personal_token".to_string()),
            api_token: Some("pd-token".to_string()),
            ..Profile::default()
        };
        let request = apply_auth(
            client.request(Method::GET, "https://api.pipedrive.com/api/v2/deals"),
            "pipedrive",
            "deals.list",
            &profile,
        )
        .unwrap()
        .build()
        .unwrap();

        assert_eq!(request.headers()["x-api-token"], "pd-token");
        assert!(!request.headers().contains_key("authorization"));
    }

    #[test]
    fn apollo_api_key_uses_x_api_key_header() {
        let client = Client::new();
        let profile = Profile {
            auth_type: Some("apollo_api_key".to_string()),
            api_token: Some("apollo-token".to_string()),
            ..Profile::default()
        };
        let request = apply_auth(
            client.request(
                Method::GET,
                "https://api.apollo.io/api/v1/users/api_profile",
            ),
            "apollo",
            "users.me",
            &profile,
        )
        .unwrap()
        .build()
        .unwrap();

        assert_eq!(request.headers()["x-api-key"], "apollo-token");
        assert!(!request.headers().contains_key("authorization"));
    }

    #[test]
    fn openpanel_client_credentials_uses_both_headers() {
        let client = Client::new();
        let profile = Profile {
            auth_type: Some("openpanel_client_credentials".to_string()),
            client_id: Some("018f0000-0000-0000-0000-000000000000".to_string()),
            api_token: Some("openpanel-secret".to_string()),
            ..Profile::default()
        };
        let request = apply_auth(
            client.request(Method::GET, "https://api.openpanel.dev/manage/projects"),
            "openpanel",
            "projects.list",
            &profile,
        )
        .unwrap()
        .build()
        .unwrap();

        assert_eq!(
            request.headers()["openpanel-client-id"],
            "018f0000-0000-0000-0000-000000000000"
        );
        assert_eq!(
            request.headers()["openpanel-client-secret"],
            "openpanel-secret"
        );
        assert!(!request.headers().contains_key("authorization"));
    }

    fn api_key_header_request(header: Option<&str>) -> Result<reqwest::Request, AppError> {
        let profile = Profile {
            auth_type: Some("api_key_header".to_string()),
            api_token: Some("key-1".to_string()),
            api_key_header: header.map(str::to_string),
            ..Profile::default()
        };
        apply_auth(
            Client::new().request(Method::GET, "https://example.test/x"),
            "merkys",
            "conversations.list",
            &profile,
        )
        .map(|request| request.build().unwrap())
    }

    #[test]
    fn api_key_header_defaults_to_x_api_key() {
        let request = api_key_header_request(None).unwrap();
        assert_eq!(request.headers()["x-api-key"], "key-1");
        assert!(!request.headers().contains_key("authorization"));
    }

    #[test]
    fn api_key_header_uses_the_configured_header_name() {
        let request = api_key_header_request(Some("X-Custom-Key")).unwrap();
        assert_eq!(request.headers()["x-custom-key"], "key-1");
        assert!(!request.headers().contains_key("x-api-key"));
    }

    #[test]
    fn api_key_header_rejects_an_invalid_header_name() {
        let error = api_key_header_request(Some("bad header")).unwrap_err();
        assert_eq!(error.code, "config_error");
    }

    #[test]
    fn extracts_next_link_header() {
        assert_eq!(
            link_next_url(
                r#"<https://api.github.com/items?page=1>; rel="prev", <https://api.github.com/items?page=3>; rel="next""#
            )
            .as_deref(),
            Some("https://api.github.com/items?page=3")
        );
    }

    #[tokio::test]
    async fn no_redirect_requests_return_redirect_response_without_following() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request).unwrap();
            stream
                .write_all(
                    b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:9/redirected\r\nContent-Length: 0\r\n\r\n",
                )
                .unwrap();
        });
        let client = ApiClient::new().unwrap();
        let profile = Profile {
            auth_type: Some("none".to_string()),
            ..Profile::default()
        };

        let error = client
            .request_no_redirect(
                "test",
                "request",
                &profile,
                Method::GET,
                format!("http://{address}/start"),
                None,
            )
            .await
            .unwrap_err();
        server.join().unwrap();

        assert_eq!(error.code, "provider_api_error");
        assert_eq!(error.status, Some(302));
    }

    #[tokio::test]
    async fn gateway_requests_do_not_follow_gateway_redirects() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request).unwrap();
            stream
                .write_all(
                    b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:9/redirected\r\nContent-Length: 0\r\n\r\n",
                )
                .unwrap();
        });
        let client = ApiClient::new().unwrap();
        let profile = Profile {
            credential_source: Some("gateway".into()),
            gateway_url: Some(format!("http://{address}")),
            gateway_profile_id: Some("profile".into()),
            gateway_token: Some("proxy-token".into()),
            ..Profile::default()
        };

        let error = client
            .request(
                "github",
                "items.list",
                &profile,
                Method::GET,
                "http://provider.test/items".into(),
                None,
            )
            .await
            .unwrap_err();
        server.join().unwrap();

        assert_eq!(error.code, "gateway_error");
        assert_eq!(error.status, Some(302));
    }

    async fn provider_once(
        body: &'static [u8],
        content_type: &'static str,
    ) -> (String, Arc<Mutex<Option<String>>>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let authorization = Arc::new(Mutex::new(None));
        let captured = Arc::clone(&authorization);
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut bytes = vec![0_u8; 8192];
            let count = stream.read(&mut bytes).await.unwrap();
            let request = String::from_utf8_lossy(&bytes[..count]);
            let value = request
                .lines()
                .find_map(|line| {
                    line.strip_prefix("authorization: ")
                        .or_else(|| line.strip_prefix("Authorization: "))
                })
                .map(str::to_string);
            *captured.lock().unwrap() = value;
            let head = format!("HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
            stream.write_all(head.as_bytes()).await.unwrap();
            stream.write_all(body).await.unwrap();
        });
        (format!("http://{address}"), authorization)
    }

    async fn gateway_profile(
        provider_url: &str,
        provider_name: &str,
        operation: &str,
    ) -> (String, String, tempfile::TempDir) {
        use crate::gateway_server::{router, GatewayApp, GatewayStore};
        let temp = tempfile::tempdir().unwrap();
        let store = GatewayStore::open(
            temp.path().join("state.enc.json"),
            temp.path().join("state.key"),
        )
        .unwrap();
        let client = Client::builder().build().unwrap();
        let app = GatewayApp {
            store: Arc::new(tokio::sync::RwLock::new(store)),
            admin_token: "admin".to_string(),
            client: client.clone(),
        };
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, router(app)).await.unwrap();
        });
        let gateway_url = format!("http://{address}");
        let profile = Profile {
            provider: Some(provider_name.into()),
            auth_type: Some("bearer_token".into()),
            base_url: Some(provider_url.into()),
            site_url: Some(provider_url.into()),
            gateway_profile_id: Some("fake".into()),
            token: Some("provider-secret".into()),
            ..Profile::default()
        };
        client
            .post(format!("{gateway_url}/api/v1/admin/profiles"))
            .bearer_auth("admin")
            .json(&serde_json::json!({"profile": profile}))
            .send()
            .await
            .unwrap();
        let token_response: Value = client.post(format!("{gateway_url}/api/v1/admin/tokens")).bearer_auth("admin").json(&serde_json::json!({"grants": {"fake": {"allow_operations": [operation], "allow_methods": ["GET", "POST", "PUT", "PATCH", "DELETE"]}}})).send().await.unwrap().json().await.unwrap();
        (
            gateway_url,
            token_response["token"].as_str().unwrap().to_string(),
            temp,
        )
    }

    #[tokio::test]
    async fn direct_json_request_keeps_provider_auth_local() {
        let (provider, authorization) = provider_once(br#"{"ok":true}"#, "application/json").await;
        let client = ApiClient::new().unwrap();
        let profile = Profile {
            base_url: Some(provider.clone()),
            auth_type: Some("bearer_token".into()),
            token: Some("provider-secret".into()),
            ..Profile::default()
        };
        let value = client
            .request(
                "github",
                "items.list",
                &profile,
                Method::GET,
                format!("{provider}/items"),
                None,
            )
            .await
            .unwrap();
        assert_eq!(value["ok"], true);
        assert_eq!(
            authorization.lock().unwrap().as_deref(),
            Some("Bearer provider-secret")
        );
    }

    #[tokio::test]
    async fn gateway_typed_command_injects_provider_auth() {
        let (provider, authorization) = provider_once(br#"{"ok":true}"#, "application/json").await;
        let (gateway, proxy_token, _temp) =
            gateway_profile(&provider, "github", "repos.list").await;
        let profile = Profile {
            credential_source: Some("gateway".into()),
            gateway_url: Some(gateway),
            gateway_profile_id: Some("fake".into()),
            gateway_token: Some(proxy_token),
            ..Profile::default()
        };
        let value = crate::gateway_command::execute(
            &profile,
            vec![
                "aai-cli".into(),
                "github".into(),
                "repos".into(),
                "list".into(),
            ],
        )
        .await
        .unwrap();
        assert_eq!(value["ok"], true);
        assert_eq!(
            authorization.lock().unwrap().as_deref(),
            Some("Bearer provider-secret")
        );
    }

    #[tokio::test]
    async fn gateway_never_forwards_upstream_authorization_response_header() {
        use axum::{http::StatusCode as AxumStatus, routing::get, Json, Router};
        for upstream_status in [AxumStatus::OK, AxumStatus::UNAUTHORIZED] {
            let provider_app = Router::new().route(
                "/user/repos",
                get(move || async move {
                    (
                        upstream_status,
                        [("authorization", "Basic encoded-provider-credential")],
                        Json(serde_json::json!({"ok": upstream_status.is_success()})),
                    )
                }),
            );
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let provider = format!("http://{}", listener.local_addr().unwrap());
            tokio::spawn(async move { axum::serve(listener, provider_app).await.unwrap() });
            let (gateway, proxy_token, _temp) =
                gateway_profile(&provider, "github", "repos.list").await;
            let metadata = crate::gateway_protocol::CommandMetadata {
                protocol_version: crate::gateway_protocol::COMMAND_PROTOCOL_VERSION,
                profile_id: "fake".into(),
                argv: vec![
                    "aai-cli".into(),
                    "github".into(),
                    "repos".into(),
                    "list".into(),
                ],
                scope: crate::gateway_protocol::CommandScope::default(),
            };
            let response = Client::new()
                .post(format!("{gateway}/api/v2/execute"))
                .bearer_auth(proxy_token)
                .multipart(
                    reqwest::multipart::Form::new()
                        .text("metadata", serde_json::to_string(&metadata).unwrap()),
                )
                .send()
                .await
                .unwrap();
            assert!(response
                .headers()
                .get(reqwest::header::AUTHORIZATION)
                .is_none());
            assert_eq!(response.status().is_success(), upstream_status.is_success());
        }
    }

    #[tokio::test]
    async fn gateway_rejects_generic_and_ungranted_typed_commands() {
        let (provider, _) = provider_once(br#"{"ok":true}"#, "application/json").await;
        let (gateway, proxy_token, _temp) =
            gateway_profile(&provider, "github", "repos.list").await;
        let profile = Profile {
            credential_source: Some("gateway".into()),
            gateway_url: Some(gateway),
            gateway_profile_id: Some("fake".into()),
            gateway_token: Some(proxy_token),
            ..Profile::default()
        };
        for (argv, code) in [
            (
                vec!["aai-cli", "github", "request", "get", "/user"],
                "gateway_protocol_error",
            ),
            (
                vec![
                    "aai-cli", "github", "repos", "get", "--owner", "a", "--repo", "b",
                ],
                "gateway_denied",
            ),
        ] {
            let error = crate::gateway_command::execute(
                &profile,
                argv.into_iter().map(str::to_string).collect(),
            )
            .await
            .unwrap_err();
            assert_eq!(error.code, code);
        }
    }

    #[tokio::test]
    async fn gateway_redacts_known_provider_secret_from_typed_result() {
        use axum::{routing::get, Json, Router};
        let provider_app = Router::new().route(
            "/user/repos",
            get(|| async { Json(serde_json::json!({"echo": "provider-secret"})) }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let provider = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, provider_app).await.unwrap() });
        let direct = Profile {
            base_url: Some(provider.clone()),
            auth_type: Some("bearer_token".into()),
            token: Some("provider-secret".into()),
            ..Profile::default()
        };
        let direct_value = ApiClient::new()
            .unwrap()
            .request(
                "github",
                "repos.list",
                &direct,
                Method::GET,
                format!("{provider}/user/repos"),
                None,
            )
            .await
            .unwrap();
        assert_eq!(direct_value["echo"], "provider-secret");
        let (gateway, proxy_token, _temp) =
            gateway_profile(&provider, "github", "repos.list").await;
        let remote = Profile {
            credential_source: Some("gateway".into()),
            gateway_url: Some(gateway),
            gateway_profile_id: Some("fake".into()),
            gateway_token: Some(proxy_token),
            ..Profile::default()
        };
        let result = crate::gateway_command::execute(
            &remote,
            vec![
                "aai-cli".into(),
                "github".into(),
                "repos".into(),
                "list".into(),
            ],
        )
        .await
        .unwrap();
        assert_eq!(result["echo"], "[REDACTED]");
    }

    #[tokio::test]
    async fn gateway_preserves_provider_error_contract() {
        use axum::{http::StatusCode as AxumStatus, routing::get, Json, Router};
        let provider_app = Router::new().route(
            "/user/repos",
            get(|| async {
                (
                    AxumStatus::UNAUTHORIZED,
                    Json(serde_json::json!({"error": "denied"})),
                )
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let provider = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, provider_app).await.unwrap() });
        let direct_profile = Profile {
            base_url: Some(provider.clone()),
            auth_type: Some("bearer_token".into()),
            token: Some("provider-secret".into()),
            ..Profile::default()
        };
        let direct = ApiClient::new()
            .unwrap()
            .request(
                "github",
                "repos.list",
                &direct_profile,
                Method::GET,
                format!("{provider}/user/repos?per_page=50"),
                None,
            )
            .await
            .unwrap_err();
        let (gateway, proxy_token, _temp) =
            gateway_profile(&provider, "github", "repos.list").await;
        let gateway_profile = Profile {
            credential_source: Some("gateway".into()),
            gateway_url: Some(gateway),
            gateway_profile_id: Some("fake".into()),
            gateway_token: Some(proxy_token),
            ..Profile::default()
        };
        let remote = crate::gateway_command::execute(
            &gateway_profile,
            vec![
                "aai-cli".into(),
                "github".into(),
                "repos".into(),
                "list".into(),
            ],
        )
        .await
        .unwrap_err();
        assert_eq!(remote.code, direct.code);
        assert_eq!(remote.status, direct.status);
        assert_eq!(remote.service, direct.service);
        assert_eq!(remote.operation, direct.operation);
        assert_eq!(remote.details, direct.details);
    }

    #[tokio::test]
    async fn large_attachment_upload_works_direct_and_through_gateway() {
        use axum::{body::Bytes, http::HeaderMap as AxumHeaders, routing::post, Json, Router};
        let received = Arc::new(Mutex::new(Vec::<(usize, String)>::new()));
        let capture = Arc::clone(&received);
        let provider_app = Router::new()
            .route(
                "/rest/api/2/issue/TEST-1/attachments",
                post(move |headers: AxumHeaders, body: Bytes| {
                    let capture = Arc::clone(&capture);
                    async move {
                        let auth = headers
                            .get("authorization")
                            .and_then(|value| value.to_str().ok())
                            .unwrap_or_default()
                            .to_string();
                        capture.lock().unwrap().push((body.len(), auth));
                        Json(serde_json::json!({"ok": true}))
                    }
                }),
            )
            .layer(axum::extract::DefaultBodyLimit::max(64 * 1024 * 1024));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let provider = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, provider_app).await.unwrap() });
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("large.bin");
        std::fs::write(&file, vec![42_u8; 2_200_000]).unwrap();
        let file_path = file.to_str().unwrap();
        let direct = Profile {
            site_url: Some(provider.clone()),
            auth_type: Some("bearer_token".into()),
            token: Some("provider-secret".into()),
            ..Profile::default()
        };
        let direct_value = ApiClient::new()
            .unwrap()
            .upload(
                "jira",
                "issues.attachments.upload",
                &direct,
                format!("{provider}/rest/api/2/issue/TEST-1/attachments"),
                file_path,
                None,
            )
            .await
            .unwrap();
        assert_eq!(direct_value["ok"], true);
        let (gateway, proxy_token, _gateway_state) =
            gateway_profile(&provider, "jira", "issues.attachments.upload").await;
        let gateway_profile = Profile {
            credential_source: Some("gateway".into()),
            gateway_url: Some(gateway),
            gateway_profile_id: Some("fake".into()),
            gateway_token: Some(proxy_token),
            ..Profile::default()
        };
        let gateway_value = crate::gateway_command::execute(
            &gateway_profile,
            vec![
                "aai-cli".into(),
                "jira".into(),
                "issues".into(),
                "attachments".into(),
                "upload".into(),
                "TEST-1".into(),
                "--file".into(),
                file_path.into(),
            ],
        )
        .await
        .unwrap();
        assert_eq!(gateway_value["ok"], true);
        let received = received.lock().unwrap();
        assert_eq!(received.len(), 2);
        assert!(received
            .iter()
            .all(|(bytes, auth)| *bytes > 2_200_000 && auth == "Bearer provider-secret"));
    }

    #[tokio::test]
    async fn resumable_drive_upload_works_direct_and_through_gateway() {
        use axum::{
            body::Bytes,
            http::{HeaderMap as AxumHeaders, HeaderValue as AxumHeaderValue},
            routing::{post, put},
            Json, Router,
        };
        use clap::Parser;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let provider = format!("http://{}", listener.local_addr().unwrap());
        let sessions = Arc::new(Mutex::new(Vec::<String>::new()));
        let uploads = Arc::new(Mutex::new(Vec::<(usize, String)>::new()));
        let session_capture = Arc::clone(&sessions);
        let upload_capture = Arc::clone(&uploads);
        let session_url = format!("{provider}/session/opaque-provider-url");
        let provider_app = Router::new()
            .route(
                "/upload/drive/v3/files",
                post(move |headers: AxumHeaders| {
                    let session_capture = Arc::clone(&session_capture);
                    let session_url = session_url.clone();
                    async move {
                        session_capture.lock().unwrap().push(
                            headers
                                .get("authorization")
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .to_string(),
                        );
                        let mut response_headers = AxumHeaders::new();
                        response_headers
                            .insert("location", AxumHeaderValue::from_str(&session_url).unwrap());
                        (response_headers, Json(serde_json::json!({})))
                    }
                }),
            )
            .route(
                "/session/opaque-provider-url",
                put(move |headers: AxumHeaders, body: Bytes| {
                    let upload_capture = Arc::clone(&upload_capture);
                    async move {
                        upload_capture.lock().unwrap().push((
                            body.len(),
                            headers
                                .get("authorization")
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .to_string(),
                        ));
                        Json(serde_json::json!({"id": "uploaded"}))
                    }
                }),
            )
            .layer(axum::extract::DefaultBodyLimit::max(64 * 1024 * 1024));
        tokio::spawn(async move { axum::serve(listener, provider_app).await.unwrap() });
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("big.bin");
        std::fs::write(&file, vec![7_u8; 5_100_000]).unwrap();
        let file_path = file.to_str().unwrap().to_string();
        let argv = vec![
            "aai-cli".into(),
            "drive".into(),
            "files".into(),
            "upload".into(),
            file_path,
        ];
        let direct_cli = crate::cli::Cli::try_parse_from(argv.clone()).unwrap();
        let direct_context = crate::config::Context {
            profile: Profile {
                provider: Some("drive".into()),
                base_url: Some(provider.clone()),
                auth_type: Some("bearer_token".into()),
                token: Some("provider-secret".into()),
                ..Profile::default()
            },
            secrets_file: temp.path().join("unused-secrets"),
            key_file: temp.path().join("unused-key"),
        };
        let direct = crate::services::dispatch(&direct_context, direct_cli.command)
            .await
            .unwrap();
        assert_eq!(direct["id"], "uploaded");
        let (gateway, token, _gateway_state) =
            gateway_profile(&provider, "drive", "files.upload").await;
        let remote = Profile {
            credential_source: Some("gateway".into()),
            gateway_url: Some(gateway),
            gateway_profile_id: Some("fake".into()),
            gateway_token: Some(token),
            ..Profile::default()
        };
        let remote_result = crate::gateway_command::execute(&remote, argv)
            .await
            .unwrap();
        assert_eq!(remote_result["id"], "uploaded");
        assert_eq!(
            sessions.lock().unwrap().as_slice(),
            ["Bearer provider-secret", "Bearer provider-secret"]
        );
        let uploads = uploads.lock().unwrap();
        assert_eq!(uploads.len(), 2);
        assert!(uploads
            .iter()
            .all(|(bytes, auth)| *bytes == 5_100_000 && auth == "Bearer provider-secret"));
    }

    #[tokio::test]
    async fn drive_download_writes_only_to_the_client_in_gateway_mode() {
        use axum::{http::Uri, routing::get, Json, Router};
        use clap::Parser;
        let provider_app = Router::new().route(
            "/drive/v3/files/item-1",
            get(|uri: Uri| async move {
                if uri.query().unwrap_or_default().contains("alt=media") {
                    axum::response::IntoResponse::into_response("file-bytes")
                } else {
                    axum::response::IntoResponse::into_response(Json(serde_json::json!({
                        "id": "item-1", "name": "note.txt", "mimeType": "text/plain"
                    })))
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let provider = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, provider_app).await.unwrap() });
        let temp = tempfile::tempdir().unwrap();
        let direct_output = temp.path().join("direct.txt");
        let direct_argv = vec![
            "aai-cli".to_string(),
            "drive".into(),
            "files".into(),
            "download".into(),
            "item-1".into(),
            "--output".into(),
            direct_output.to_str().unwrap().into(),
        ];
        let direct_cli = crate::cli::Cli::try_parse_from(direct_argv).unwrap();
        let direct_context = crate::config::Context {
            profile: Profile {
                provider: Some("google".into()),
                base_url: Some(provider.clone()),
                auth_type: Some("bearer_token".into()),
                token: Some("provider-secret".into()),
                ..Profile::default()
            },
            secrets_file: temp.path().join("unused-secrets"),
            key_file: temp.path().join("unused-key"),
        };
        crate::services::dispatch(&direct_context, direct_cli.command)
            .await
            .unwrap();
        assert_eq!(std::fs::read(&direct_output).unwrap(), b"file-bytes");
        let (gateway, token, _gateway_state) =
            gateway_profile(&provider, "google", "files.download").await;
        let gateway_profile = Profile {
            credential_source: Some("gateway".into()),
            gateway_url: Some(gateway),
            gateway_profile_id: Some("fake".into()),
            gateway_token: Some(token),
            ..Profile::default()
        };
        let gateway_output = temp.path().join("gateway.txt");
        let value = crate::gateway_command::execute(
            &gateway_profile,
            vec![
                "aai-cli".into(),
                "drive".into(),
                "files".into(),
                "download".into(),
                "item-1".into(),
                "--output".into(),
                gateway_output.to_str().unwrap().into(),
            ],
        )
        .await
        .unwrap();
        assert_eq!(std::fs::read(&gateway_output).unwrap(), b"file-bytes");
        assert_eq!(value["output"], gateway_output.to_str().unwrap());
    }

    #[tokio::test]
    async fn services_added_after_gateway_v1_keep_working_in_direct_mode() {
        for (service, operation, method) in [
            ("agent-barn", "list_agents_get", Method::GET),
            ("apollo", "people.search", Method::POST),
            ("merkys", "ask", Method::POST),
            ("microsoft", "planner.tasks.update", Method::PATCH),
        ] {
            let (provider, authorization) =
                provider_once(br#"{"ok":true}"#, "application/json").await;
            let profile = Profile {
                base_url: Some(provider.clone()),
                auth_type: Some("bearer_token".into()),
                token: Some("provider-secret".into()),
                ..Profile::default()
            };
            let value = ApiClient::new()
                .unwrap()
                .request(
                    service,
                    operation,
                    &profile,
                    method,
                    format!("{provider}/typed"),
                    None,
                )
                .await
                .unwrap();
            assert_eq!(value["ok"], true, "{service}");
            assert_eq!(
                authorization.lock().unwrap().as_deref(),
                Some("Bearer provider-secret"),
                "{service}"
            );
        }
    }

    #[tokio::test]
    async fn legacy_url_based_gateway_transport_is_rejected() {
        let (provider, _) = provider_once(br#"{"ok":true}"#, "application/json").await;
        let (gateway, proxy_token, _temp) =
            gateway_profile(&provider, "github", "repos.list").await;
        let response = Client::new()
            .post(format!("{gateway}/api/v1/execute"))
            .bearer_auth(proxy_token)
            .multipart(reqwest::multipart::Form::new().text("metadata", "{}"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::GONE);
    }

    #[tokio::test]
    async fn direct_download_returns_provider_bytes() {
        let (provider, authorization) =
            provider_once(b"provider-secret", "application/octet-stream").await;
        let client = ApiClient::new().unwrap();
        let profile = Profile {
            base_url: Some(provider.clone()),
            auth_type: Some("bearer_token".into()),
            token: Some("provider-secret".into()),
            ..Profile::default()
        };
        let bytes = client
            .download(
                "github",
                "items.download",
                &profile,
                format!("{provider}/items/1"),
            )
            .await
            .unwrap();
        assert_eq!(bytes, b"provider-secret");
        assert_eq!(
            authorization.lock().unwrap().as_deref(),
            Some("Bearer provider-secret")
        );
    }

    #[tokio::test]
    async fn direct_bytes_request_uses_provider_auth() {
        let (provider, authorization) =
            provider_once(br#"{"uploaded":true}"#, "application/json").await;
        let client = ApiClient::new().unwrap();
        let profile = Profile {
            base_url: Some(provider.clone()),
            auth_type: Some("bearer_token".into()),
            token: Some("provider-secret".into()),
            ..Profile::default()
        };
        let response = client
            .request_bytes(
                "github",
                "items.upload",
                &profile,
                BytesRequest {
                    method: Method::POST,
                    url: format!("{provider}/items"),
                    content_type: "application/octet-stream".into(),
                    headers: vec![],
                    body: b"data".to_vec(),
                },
            )
            .await
            .unwrap();
        assert_eq!(response.body["uploaded"], true);
        assert_eq!(
            authorization.lock().unwrap().as_deref(),
            Some("Bearer provider-secret")
        );
    }

    /// Answer one request per response, in order, and hand back the raw request texts.
    fn redirect_to(location: &str) -> Vec<u8> {
        format!("HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .into_bytes()
    }

    const FILE_RESPONSE: &[u8] =
        b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\nConnection: close\r\n\r\npdf-bytes";

    fn pipedrive_profile() -> Profile {
        Profile {
            auth_type: Some("pipedrive_personal_token".to_string()),
            api_token: Some("pd-secret".to_string()),
            ..Profile::default()
        }
    }

    #[tokio::test]
    async fn download_does_not_forward_credentials_to_the_redirect_target() {
        let (storage, storage_server) = serve(vec![FILE_RESPONSE.to_vec()]);
        let (api, api_server) = serve(vec![redirect_to(&format!(
            "http://{storage}/signed?sig=abc"
        ))]);

        let bytes = ApiClient::new()
            .unwrap()
            .download(
                "pipedrive",
                "files.download",
                &pipedrive_profile(),
                format!("http://{api}/v1/files/7/download"),
            )
            .await
            .unwrap();
        let api_requests = api_server.join().unwrap();
        let storage_requests = storage_server.join().unwrap();

        assert_eq!(bytes, b"pdf-bytes");
        assert!(api_requests[0].contains("x-api-token: pd-secret"));
        assert!(
            !storage_requests[0].contains("pd-secret"),
            "credentials reached the redirect target:\n{}",
            storage_requests[0]
        );
    }

    #[tokio::test]
    async fn download_keeps_credentials_on_a_same_origin_redirect() {
        let (api, api_server) = serve(vec![
            redirect_to("/v1/files/7/content"),
            FILE_RESPONSE.to_vec(),
        ]);

        let bytes = ApiClient::new()
            .unwrap()
            .download(
                "pipedrive",
                "files.download",
                &pipedrive_profile(),
                format!("http://{api}/v1/files/7/download"),
            )
            .await
            .unwrap();
        let requests = api_server.join().unwrap();

        assert_eq!(bytes, b"pdf-bytes");
        assert!(requests[1].starts_with("get /v1/files/7/content"));
        assert!(requests[1].contains("x-api-token: pd-secret"));
    }

    #[tokio::test]
    async fn download_stops_after_the_redirect_limit() {
        let responses = (0..=MAX_DOWNLOAD_REDIRECTS)
            .map(|hop| redirect_to(&format!("/v1/files/7/hop{hop}")))
            .collect();
        let (api, api_server) = serve(responses);

        let error = ApiClient::new()
            .unwrap()
            .download(
                "pipedrive",
                "files.download",
                &pipedrive_profile(),
                format!("http://{api}/v1/files/7/download"),
            )
            .await
            .unwrap_err();
        let requests = api_server.join().unwrap();

        assert_eq!(requests.len(), MAX_DOWNLOAD_REDIRECTS + 1);
        assert!(error.to_string().contains("redirects"), "{error}");
    }

    #[tokio::test]
    async fn download_reports_a_redirect_without_a_location() {
        let (api, api_server) = serve(vec![
            b"HTTP/1.1 302 Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
        ]);

        let error = ApiClient::new()
            .unwrap()
            .download(
                "pipedrive",
                "files.download",
                &pipedrive_profile(),
                format!("http://{api}/v1/files/7/download"),
            )
            .await
            .unwrap_err();
        let requests = api_server.join().unwrap();

        assert_eq!(requests.len(), 1);
        assert!(error.to_string().contains("302"), "{error}");
    }
}
