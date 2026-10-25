use std::{
    collections::HashMap,
    fs, io,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    extract::{DefaultBodyLimit, Multipart, Path as AxumPath, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce,
};
use clap::Parser;
use rand::RngCore;
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use tokio::sync::RwLock;
use url::Url;

use crate::{
    cli::{Cli, Command, MicrosoftResource},
    config::Context,
    config::Profile,
    error::AppError,
    gateway_catalog,
    gateway_command::{apply_scope, command_paths, normalize_argv, validate_inline_json},
    gateway_protocol::{
        CommandMetadata, CommandResult, ExecuteMetadata, GatewayErrorBody,
        COMMAND_PROTOCOL_VERSION, PROTOCOL_VERSION, RESPONSE_SOURCE_HEADER,
    },
    http::{apply_auth, ApiClient},
    oauth, services,
};

const MAX_COMMAND_FILE_BYTES: usize = 64 * 1024 * 1024;
const MAX_COMMAND_REQUEST_BYTES: usize = 70 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Grant {
    #[serde(default)]
    pub allow_all_operations: bool,
    #[serde(default)]
    pub allow_operations: Vec<String>,
    #[serde(default)]
    pub deny_operations: Vec<String>,
    #[serde(default)]
    pub allow_methods: Vec<String>,
    #[serde(default)]
    pub deny_methods: Vec<String>,
}

impl Grant {
    pub(crate) fn permits(&self, operation: &str, method: &Method) -> bool {
        let method = method.as_str().to_ascii_uppercase();
        if self.deny_operations.iter().any(|item| item == operation)
            || self
                .deny_methods
                .iter()
                .any(|item| item.eq_ignore_ascii_case(&method))
        {
            return false;
        }
        let operation_allowed =
            self.allow_all_operations || self.allow_operations.iter().any(|item| item == operation);
        operation_allowed
            && self
                .allow_methods
                .iter()
                .any(|item| item.eq_ignore_ascii_case(&method))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToken {
    pub label: Option<String>,
    pub digest: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub expires_at: Option<u64>,
    #[serde(default)]
    pub grants: HashMap<String, Grant>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GatewayDocument {
    pub version: u32,
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
    #[serde(default)]
    pub tokens: HashMap<String, StoredToken>,
}

#[derive(Clone)]
pub struct GatewayApp {
    pub store: Arc<RwLock<GatewayStore>>,
    pub admin_token: String,
    pub client: Client,
}

pub struct GatewayStore {
    document: GatewayDocument,
    state_path: PathBuf,
    key_path: PathBuf,
    key: [u8; 32],
}

impl GatewayStore {
    pub fn open(state_path: PathBuf, key_path: PathBuf) -> Result<Self, AppError> {
        let key = load_or_create_key(&key_path, state_path.exists())?;
        let document = if state_path.exists() {
            decrypt_document(&state_path, &key)?
        } else {
            GatewayDocument {
                version: 1,
                ..GatewayDocument::default()
            }
        };
        Ok(Self {
            document,
            state_path,
            key_path,
            key,
        })
    }

    fn save(&self) -> Result<(), AppError> {
        self.save_document(&self.document)
    }

    fn save_document(&self, document: &GatewayDocument) -> Result<(), AppError> {
        let plaintext = serde_json::to_vec(document)
            .map_err(|err| AppError::internal("gateway", "state.save", err.to_string()))?;
        let cipher = XChaCha20Poly1305::new((&self.key).into());
        let mut nonce = [0_u8; 24];
        rand::rng().fill_bytes(&mut nonce);
        let ciphertext = cipher
            .encrypt(XNonce::from_slice(&nonce), plaintext.as_ref())
            .map_err(|_| AppError::internal("gateway", "state.save", "state encryption failed"))?;
        let envelope = serde_json::json!({
            "version": 1,
            "nonce": B64.encode(nonce),
            "ciphertext": B64.encode(ciphertext),
        });
        let rendered = serde_json::to_vec_pretty(&envelope)
            .map_err(|err| AppError::internal("gateway", "state.save", err.to_string()))?;
        if let Some(parent) = self.state_path.parent() {
            fs::create_dir_all(parent).map_err(|err| io_error("state.save", err))?;
        }
        let temporary = self.state_path.with_extension("tmp");
        fs::write(&temporary, rendered).map_err(|err| io_error("state.save", err))?;
        set_private(&temporary)?;
        fs::rename(&temporary, &self.state_path).map_err(|err| io_error("state.save", err))?;
        let _ = &self.key_path;
        Ok(())
    }
}

fn io_error(operation: &'static str, err: io::Error) -> AppError {
    AppError::internal("gateway", operation, err.to_string())
}

fn load_or_create_key(path: &Path, state_exists: bool) -> Result<[u8; 32], AppError> {
    if path.exists() {
        let bytes = fs::read(path).map_err(|err| io_error("state.key", err))?;
        return bytes
            .try_into()
            .map_err(|_| AppError::config("gateway key file must contain exactly 32 bytes"));
    }
    if state_exists {
        return Err(AppError::config(
            "gateway encrypted state exists but its key file is missing",
        ));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| io_error("state.key", err))?;
    }
    let mut key = [0_u8; 32];
    rand::rng().fill_bytes(&mut key);
    fs::write(path, key).map_err(|err| io_error("state.key", err))?;
    set_private(path)?;
    Ok(key)
}

fn decrypt_document(path: &Path, key: &[u8; 32]) -> Result<GatewayDocument, AppError> {
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(path).map_err(|err| io_error("state.load", err))?)
            .map_err(|err| AppError::config(format!("invalid gateway state: {err}")))?;
    let nonce = B64
        .decode(
            value
                .get("nonce")
                .and_then(|v| v.as_str())
                .unwrap_or_default(),
        )
        .map_err(|_| AppError::config("invalid gateway state nonce"))?;
    let ciphertext = B64
        .decode(
            value
                .get("ciphertext")
                .and_then(|v| v.as_str())
                .unwrap_or_default(),
        )
        .map_err(|_| AppError::config("invalid gateway state ciphertext"))?;
    if nonce.len() != 24 {
        return Err(AppError::config("invalid gateway state nonce length"));
    }
    let cipher = XChaCha20Poly1305::new(key.into());
    let plaintext = cipher
        .decrypt(XNonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|_| AppError::auth("gateway", "state.load", "failed to decrypt gateway state"))?;
    serde_json::from_slice(&plaintext)
        .map_err(|err| AppError::config(format!("invalid gateway state payload: {err}")))
}

#[cfg(unix)]
fn set_private(path: &Path) -> Result<(), AppError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|err| io_error("state.permissions", err))
}

#[cfg(not(unix))]
fn set_private(_path: &Path) -> Result<(), AppError> {
    Ok(())
}

pub fn router(app: GatewayApp) -> Router {
    Router::new()
        .route("/health/live", get(health))
        .route("/health/ready", get(health))
        .route("/api/v1/execute", post(legacy_execute))
        .route("/api/v2/execute", post(execute_command))
        .route(
            "/api/v1/admin/profiles",
            get(list_profiles).post(create_profile),
        )
        .route(
            "/api/v1/admin/profiles/{id}",
            get(get_profile).put(replace_profile).delete(delete_profile),
        )
        .route("/api/v1/admin/tokens", get(list_tokens).post(create_token))
        .route(
            "/api/v1/admin/tokens/{id}",
            patch(update_token).delete(delete_token),
        )
        .with_state(app)
        .layer(DefaultBodyLimit::max(MAX_COMMAND_REQUEST_BYTES))
}

async fn health() -> &'static str {
    "ok"
}

async fn legacy_execute() -> Response {
    gateway_error(
        StatusCode::GONE,
        "gateway_protocol_error",
        "URL-based gateway requests are no longer accepted",
        None,
    )
}

async fn execute_command(
    State(app): State<GatewayApp>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Response {
    let Some(token) = bearer(&headers) else {
        return gateway_error(
            StatusCode::UNAUTHORIZED,
            "gateway_auth_error",
            "missing proxy token",
            None,
        );
    };
    let mut metadata = None;
    let mut input = None;
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(_) => {
                return gateway_error(
                    StatusCode::BAD_REQUEST,
                    "gateway_protocol_error",
                    "invalid multipart request",
                    None,
                )
            }
        };
        match field.name() {
            Some("metadata") if metadata.is_none() => {
                metadata = match field.text().await {
                    Ok(text) if text.len() <= 256 * 1024 => Some(text),
                    _ => {
                        return gateway_error(
                            StatusCode::BAD_REQUEST,
                            "gateway_protocol_error",
                            "invalid command metadata",
                            None,
                        )
                    }
                };
            }
            Some("input") if input.is_none() => {
                input = match field.bytes().await {
                    Ok(bytes) if bytes.len() <= MAX_COMMAND_FILE_BYTES => Some(bytes.to_vec()),
                    _ => {
                        return gateway_error(
                            StatusCode::PAYLOAD_TOO_LARGE,
                            "gateway_protocol_error",
                            "input file exceeds 64 MiB",
                            None,
                        )
                    }
                };
            }
            _ => {
                return gateway_error(
                    StatusCode::BAD_REQUEST,
                    "gateway_protocol_error",
                    "unexpected command part",
                    None,
                )
            }
        }
    }
    let Some(metadata) =
        metadata.and_then(|text| serde_json::from_str::<CommandMetadata>(&text).ok())
    else {
        return gateway_error(
            StatusCode::BAD_REQUEST,
            "gateway_protocol_error",
            "missing or invalid command metadata",
            None,
        );
    };
    if metadata.protocol_version != COMMAND_PROTOCOL_VERSION
        || metadata.argv.is_empty()
        || metadata.argv.len() > 256
    {
        return gateway_error(
            StatusCode::BAD_REQUEST,
            "gateway_protocol_error",
            "unsupported command protocol",
            None,
        );
    }
    let (mut profile, grant, key_path) = {
        let store = app.store.read().await;
        let Some(stored) = find_token(&store.document, &token) else {
            return gateway_error(
                StatusCode::UNAUTHORIZED,
                "gateway_auth_error",
                "invalid proxy token",
                None,
            );
        };
        if !stored.enabled || stored.expires_at.is_some_and(|expiry| expiry <= now()) {
            return gateway_error(
                StatusCode::UNAUTHORIZED,
                "gateway_auth_error",
                "proxy token is disabled or expired",
                None,
            );
        }
        let Some(grant) = stored.grants.get(&metadata.profile_id) else {
            return gateway_error(
                StatusCode::FORBIDDEN,
                "gateway_denied",
                "token is not granted this profile",
                None,
            );
        };
        let Some(profile) = store.document.profiles.get(&metadata.profile_id) else {
            return gateway_error(
                StatusCode::NOT_FOUND,
                "gateway_denied",
                "gateway profile was not found",
                None,
            );
        };
        (profile.clone(), grant.clone(), store.key_path.clone())
    };
    let mut argv = normalize_argv(metadata.argv);
    let paths = match command_paths(&argv) {
        Ok(paths) => paths,
        Err(err) => return app_error_response(StatusCode::BAD_REQUEST, err),
    };
    if let Err(err) = validate_inline_json(&argv) {
        return app_error_response(StatusCode::BAD_REQUEST, err);
    }
    if paths.input.is_some() != input.is_some() {
        return gateway_error(
            StatusCode::BAD_REQUEST,
            "gateway_protocol_error",
            "input attachment does not match command",
            None,
        );
    }
    let temporary = match tempfile::tempdir() {
        Ok(temporary) => temporary,
        Err(err) => {
            return app_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                io_error("execute.temp", err),
            )
        }
    };
    if let (Some(index), Some(input)) = (paths.input, input) {
        let name = Path::new(&argv[index]).file_name().unwrap_or_default();
        let input_dir = temporary.path().join("input");
        if let Err(err) = fs::create_dir(&input_dir) {
            return app_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                io_error("execute.input", err),
            );
        }
        let path = input_dir.join(name);
        if let Err(err) = fs::write(&path, input) {
            return app_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                io_error("execute.input", err),
            );
        }
        argv[index] = path.to_string_lossy().into_owned();
    }
    let output_path = paths.output.map(|index| {
        let path = temporary.path().join("output");
        argv[index] = path.to_string_lossy().into_owned();
        path
    });
    let parsed = match Cli::try_parse_from(argv) {
        Ok(parsed) => parsed,
        Err(_) => {
            return gateway_error(
                StatusCode::BAD_REQUEST,
                "gateway_protocol_error",
                "invalid typed command",
                None,
            )
        }
    };
    let service = match command_service(&parsed.command) {
        Some(service) => service,
        None => {
            return gateway_error(
                StatusCode::BAD_REQUEST,
                "gateway_protocol_error",
                "command is not available through the gateway",
                None,
            )
        }
    };
    if !profile_matches_service(profile.provider.as_deref(), service) {
        return gateway_error(
            StatusCode::FORBIDDEN,
            "gateway_denied",
            "profile does not match the requested service",
            None,
        );
    }
    if (service == "email"
        && (matches!(
            profile.transport.as_deref(),
            Some("smtp_imap" | "imap_smtp")
        ) || matches!(profile.auth_type.as_deref(), Some("app_password"))))
        || (service == "calendar"
            && (matches!(profile.transport.as_deref(), Some("caldav"))
                || matches!(profile.auth_type.as_deref(), Some("caldav_password"))))
    {
        return gateway_error(
            StatusCode::BAD_REQUEST,
            "gateway_protocol_error",
            "non-HTTP transports are not available through the gateway",
            None,
        );
    }
    apply_scope(&mut profile, metadata.scope);
    profile.credential_source = Some("local".to_string());
    profile.runtime_key_file = Some(key_path.clone());
    let ctx = Context {
        profile,
        secrets_file: temporary.path().join("unused-secrets.enc.json"),
        key_file: key_path,
    };
    let client = match ApiClient::with_server_grant(grant) {
        Ok(client) => client,
        Err(err) => return app_error_response(StatusCode::INTERNAL_SERVER_ERROR, err),
    };
    let value = match services::dispatch_with_client(&client, &ctx, parsed.command).await {
        Ok(value) => redact_command_value(value, &ctx.profile),
        Err(mut err) => {
            err.message = redact_text(&err.message, &ctx.profile);
            err.details = err
                .details
                .map(|details| redact_command_value(details, &ctx.profile));
            return app_error_response(StatusCode::BAD_REQUEST, err);
        }
    };
    let (output, server_output_path) = if let Some(path) = output_path {
        let bytes = match fs::read(&path) {
            Ok(bytes) if bytes.len() <= MAX_COMMAND_FILE_BYTES => bytes,
            Ok(_) => {
                return gateway_error(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "gateway_protocol_error",
                    "output file exceeds 64 MiB",
                    None,
                )
            }
            Err(err) => {
                return app_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    io_error("execute.output", err),
                )
            }
        };
        let bytes = if std::str::from_utf8(&bytes).is_ok() {
            redact(
                &bytes,
                &ctx.profile,
                Some(&HeaderValue::from_static("text/plain")),
            )
        } else {
            bytes
        };
        (
            Some(B64.encode(bytes)),
            Some(path.to_string_lossy().into_owned()),
        )
    } else {
        (None, None)
    };
    Json(CommandResult {
        value,
        output,
        server_output_path,
    })
    .into_response()
}

fn redact_command_value(mut value: serde_json::Value, profile: &Profile) -> serde_json::Value {
    match &mut value {
        serde_json::Value::String(text) => {
            *text = redact_text(text, profile);
        }
        serde_json::Value::Array(items) => {
            for item in items {
                *item = redact_command_value(std::mem::take(item), profile);
            }
        }
        serde_json::Value::Object(fields) => {
            for item in fields.values_mut() {
                *item = redact_command_value(std::mem::take(item), profile);
            }
        }
        _ => {}
    }
    value
}

fn redact_text(text: &str, profile: &Profile) -> String {
    String::from_utf8(redact(
        text.as_bytes(),
        profile,
        Some(&HeaderValue::from_static("text/plain")),
    ))
    .unwrap_or_default()
}

fn command_service(command: &Command) -> Option<&'static str> {
    match command {
        Command::AgentBarn(_) => Some("agent-barn"),
        Command::Jira(_) => Some("jira"),
        Command::Confluence(_) => Some("confluence"),
        Command::Bitbucket(_) => Some("bitbucket"),
        Command::Github(_) => Some("github"),
        Command::Hubspot(_) => Some("hubspot"),
        Command::Email(_) => Some("email"),
        Command::Calendar(_) => Some("calendar"),
        Command::Pipedrive(_) => Some("pipedrive"),
        Command::Apollo(_) => Some("apollo"),
        Command::Sheets(_) => Some("sheets"),
        Command::Drive(_) => Some("drive"),
        Command::Slack(_) => Some("slack"),
        Command::Openpanel(_) => Some("openpanel"),
        Command::Posthog(_) => Some("posthog"),
        Command::Microsoft(command) if !matches!(command.resource, MicrosoftResource::Auth(_)) => {
            Some("microsoft")
        }
        Command::Merkys(_) => Some("merkys"),
        _ => None,
    }
}

fn profile_matches_service(provider: Option<&str>, service: &str) -> bool {
    match service {
        "email" => matches!(provider, Some("email" | "gmail" | "google" | "zoho")),
        "calendar" => matches!(provider, Some("calendar" | "google" | "zoho")),
        "drive" | "sheets" => matches!(provider, Some("google" | "drive" | "sheets")),
        _ => provider == Some(service),
    }
}

async fn execute(
    State(app): State<GatewayApp>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Response {
    let Some(token) = bearer(&headers) else {
        return gateway_error(
            StatusCode::UNAUTHORIZED,
            "gateway_auth_error",
            "missing proxy token",
            None,
        );
    };
    let mut metadata = None;
    let mut body = Vec::new();
    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name() {
            Some("metadata") => metadata = field.text().await.ok(),
            Some("body") => {
                if let Ok(bytes) = field.bytes().await {
                    body = bytes.to_vec();
                }
            }
            _ => {}
        }
    }
    let Some(metadata) =
        metadata.and_then(|text| serde_json::from_str::<ExecuteMetadata>(&text).ok())
    else {
        return gateway_error(
            StatusCode::BAD_REQUEST,
            "gateway_protocol_error",
            "missing or invalid metadata",
            None,
        );
    };
    if metadata.protocol_version != PROTOCOL_VERSION || metadata.operation == "request" {
        return gateway_error(
            StatusCode::BAD_REQUEST,
            "gateway_protocol_error",
            "unsupported gateway operation or protocol version",
            None,
        );
    }
    let Ok(method) = metadata.method() else {
        return gateway_error(
            StatusCode::BAD_REQUEST,
            "gateway_protocol_error",
            "invalid HTTP method",
            None,
        );
    };
    if !gateway_catalog::allows(&metadata.service, &metadata.operation, &method) {
        return gateway_error(
            StatusCode::BAD_REQUEST,
            "gateway_protocol_error",
            "operation and HTTP method are not a valid typed operation",
            None,
        );
    }
    let (profile, grant) = {
        let store = app.store.read().await;
        let Some(stored) = find_token(&store.document, &token) else {
            return gateway_error(
                StatusCode::UNAUTHORIZED,
                "gateway_auth_error",
                "invalid proxy token",
                None,
            );
        };
        if !stored.enabled || stored.expires_at.is_some_and(|expiry| expiry <= now()) {
            return gateway_error(
                StatusCode::UNAUTHORIZED,
                "gateway_auth_error",
                "proxy token is disabled or expired",
                None,
            );
        }
        let Some(grant) = stored.grants.get(&metadata.profile_id) else {
            return gateway_error(
                StatusCode::FORBIDDEN,
                "gateway_denied",
                "token is not granted this profile",
                None,
            );
        };
        let Some(profile) = store.document.profiles.get(&metadata.profile_id) else {
            return gateway_error(
                StatusCode::NOT_FOUND,
                "gateway_denied",
                "gateway profile was not found",
                None,
            );
        };
        (profile.clone(), grant.clone())
    };
    if !grant.permits(&metadata.operation, &method) {
        return gateway_error(
            StatusCode::FORBIDDEN,
            "gateway_denied",
            "operation or HTTP method is not permitted",
            None,
        );
    }
    if !target_allowed(&metadata.service, &metadata.url, &profile) {
        return gateway_error(
            StatusCode::BAD_REQUEST,
            "gateway_protocol_error",
            "request target is not allowed for this profile",
            None,
        );
    }
    forward(&app, &metadata, method, body, profile).await
}

fn bearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            let (scheme, credentials) = value.split_once(' ')?;
            scheme.eq_ignore_ascii_case("Bearer").then_some(credentials)
        })
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn find_token<'a>(document: &'a GatewayDocument, token: &str) -> Option<&'a StoredToken> {
    let digest = Sha256::digest(token.as_bytes());
    document.tokens.values().find(|candidate| {
        hex_digest(&digest)
            .as_bytes()
            .ct_eq(candidate.digest.as_bytes())
            .into()
    })
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(crate) fn target_allowed(service: &str, raw_url: &str, profile: &Profile) -> bool {
    let Ok(url) = Url::parse(raw_url) else {
        return false;
    };
    if url.username() != "" || url.password().is_some() || url.fragment().is_some() {
        return false;
    }
    if url.scheme() != "https"
        && !(url.scheme() == "http" && url.host_str().is_some_and(is_loopback))
    {
        return false;
    }
    let expected = profile
        .site_url
        .as_deref()
        .or(profile.base_url.as_deref())
        .and_then(|base| Url::parse(base).ok());
    if expected
        .as_ref()
        .is_some_and(|base| base.origin() == url.origin())
    {
        return true;
    }
    url.host_str().is_some_and(|actual| {
        (url.scheme() == "https"
            && default_hosts(service)
                .iter()
                .any(|host| host.eq_ignore_ascii_case(actual)))
            || (url.scheme() == "https" && file_host_allowed(service, actual))
    })
}

pub(crate) fn file_host_allowed(service: &str, host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    let suffixes: &[&str] = match service {
        "jira" | "confluence" => &[".atlassian.com", ".atlassian.net"],
        "drive" | "sheets" | "email" | "calendar" => &[".googleusercontent.com", ".googleapis.com"],
        "microsoft" => &[".sharepoint.com", ".1drv.com"],
        "github" => &[".githubusercontent.com"],
        "slack" => &[".slack.com"],
        "pipedrive" => &[".amazonaws.com"],
        _ => &[],
    };
    suffixes.iter().any(|suffix| host.ends_with(suffix))
}

fn is_loopback(host: &str) -> bool {
    host == "localhost" || host == "127.0.0.1" || host == "[::1]"
}

fn default_hosts(service: &str) -> &'static [&'static str] {
    match service {
        "github" => &["api.github.com"],
        "bitbucket" => &["api.bitbucket.org"],
        "jira" | "confluence" => &[],
        "agent-barn" => &[],
        "apollo" => &["api.apollo.io"],
        "hubspot" => &["api.hubapi.com"],
        "slack" => &["slack.com"],
        "openpanel" => &["api.openpanel.dev"],
        "posthog" => &["us.i.posthog.com", "eu.i.posthog.com"],
        "pipedrive" => &["api.pipedrive.com"],
        "microsoft" => &["graph.microsoft.com"],
        "drive" | "sheets" | "email" | "calendar" => &[
            "www.googleapis.com",
            "sheets.googleapis.com",
            "gmail.googleapis.com",
            "calendar.googleapis.com",
            "mail.zoho.com",
            "calendar.zoho.com",
        ],
        _ => &[],
    }
}

async fn forward(
    app: &GatewayApp,
    metadata: &ExecuteMetadata,
    method: Method,
    body: Vec<u8>,
    profile: Profile,
) -> Response {
    let token = match oauth::resolve_token(&profile, &app.client, "gateway", "execute").await {
        Ok(token) => token,
        Err(err) => return app_error_response(StatusCode::UNAUTHORIZED, err),
    };
    let effective = Profile {
        token: Some(token),
        ..profile.clone()
    };
    let mut request = app.client.request(method, &metadata.url);
    request = match apply_auth(request, "gateway", "execute", &effective) {
        Ok(request) => request,
        Err(err) => return app_error_response(StatusCode::UNAUTHORIZED, err),
    };
    if let Some(content_type) = &metadata.content_type {
        request = request.header(header::CONTENT_TYPE, content_type);
    }
    if let Some(accept) = &metadata.accept {
        request = request.header(header::ACCEPT, accept);
    }
    for (name, value) in &metadata.headers {
        if !matches!(
            name.to_ascii_lowercase().as_str(),
            "content-range"
                | "x-upload-content-type"
                | "x-upload-content-length"
                | "x-atlassian-token"
                | "if-match"
        ) {
            return gateway_error(
                StatusCode::BAD_REQUEST,
                "gateway_protocol_error",
                "request contains an unsafe header",
                None,
            );
        }
        request = request.header(name, value);
    }
    if !body.is_empty() {
        request = request.body(body);
    }
    let result = request.send().await;
    let response = match result {
        Ok(response) => response,
        Err(err) => {
            return app_error_response(
                StatusCode::BAD_GATEWAY,
                AppError::internal("gateway", "execute", err.to_string()),
            )
        }
    };
    let status = response.status();
    let content_type = response.headers().get(header::CONTENT_TYPE).cloned();
    let bytes = match response.bytes().await {
        Ok(bytes) => redact(&bytes, &effective, content_type.as_ref()),
        Err(err) => {
            return app_error_response(
                StatusCode::BAD_GATEWAY,
                AppError::internal("gateway", "execute", err.to_string()),
            )
        }
    };
    let mut output = Response::new(axum::body::Body::from(bytes));
    *output.status_mut() = status;
    output
        .headers_mut()
        .insert(RESPONSE_SOURCE_HEADER, HeaderValue::from_static("provider"));
    if let Some(content_type) = content_type {
        output
            .headers_mut()
            .insert(header::CONTENT_TYPE, content_type);
    }
    output
}

fn redact(bytes: &[u8], profile: &Profile, content_type: Option<&HeaderValue>) -> Vec<u8> {
    let textual = content_type
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            value.starts_with("text/")
                || value.contains("json")
                || value.contains("xml")
                || value.contains("javascript")
        })
        .unwrap_or(false);
    if !textual {
        return bytes.to_vec();
    }
    let mut text = String::from_utf8_lossy(bytes).into_owned();
    for secret in [
        profile.token.as_deref(),
        profile.refresh_token.as_deref(),
        profile.api_token.as_deref(),
        profile.client_secret.as_deref(),
        profile.password.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        if !secret.is_empty() {
            text = text.replace(secret, "[REDACTED]");
        }
    }
    text.into_bytes()
}

fn gateway_error(
    status: StatusCode,
    code: &str,
    message: &str,
    details: Option<serde_json::Value>,
) -> Response {
    let body = GatewayErrorBody {
        code: code.to_string(),
        message: message.to_string(),
        service: "gateway".to_string(),
        operation: "execute".to_string(),
        status: Some(status.as_u16()),
        details,
    };
    let mut response = (status, Json(body)).into_response();
    response
        .headers_mut()
        .insert(RESPONSE_SOURCE_HEADER, HeaderValue::from_static("gateway"));
    response
}

fn app_error_response(status: StatusCode, error: AppError) -> Response {
    let response_status = error
        .status
        .and_then(|code| StatusCode::from_u16(code).ok())
        .unwrap_or(status);
    let source = if error.status.is_some() && error.service.as_ref() != "gateway" {
        "provider"
    } else {
        "gateway"
    };
    let body = GatewayErrorBody {
        code: error.code.to_string(),
        message: error.message,
        service: error.service.to_string(),
        operation: error.operation.to_string(),
        status: error.status,
        details: error.details,
    };
    let mut response = (response_status, Json(body)).into_response();
    response
        .headers_mut()
        .insert(RESPONSE_SOURCE_HEADER, HeaderValue::from_static(source));
    response
}

fn admin_ok(headers: &HeaderMap, expected: &str) -> bool {
    bearer(headers).is_some_and(|token| token.as_bytes().ct_eq(expected.as_bytes()).into())
}

async fn list_profiles(State(app): State<GatewayApp>, headers: HeaderMap) -> Response {
    if !admin_ok(&headers, &app.admin_token) {
        return gateway_error(
            StatusCode::UNAUTHORIZED,
            "gateway_auth_error",
            "invalid admin token",
            None,
        );
    }
    let store = app.store.read().await;
    let values: Vec<_> = store.document.profiles.iter().map(|(id, profile)| serde_json::json!({"id": id, "provider": profile.provider, "auth_type": profile.auth_type, "has_credential": profile.token.is_some() || profile.api_token.is_some() || profile.password.is_some()})).collect();
    Json(values).into_response()
}

async fn get_profile(
    State(app): State<GatewayApp>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Response {
    if !admin_ok(&headers, &app.admin_token) {
        return gateway_error(
            StatusCode::UNAUTHORIZED,
            "gateway_auth_error",
            "invalid admin token",
            None,
        );
    }
    let store = app.store.read().await;
    match store.document.profiles.get(&id) {
        Some(profile) => {
            Json(serde_json::json!({"id": id, "profile": redact_profile(profile)})).into_response()
        }
        None => gateway_error(
            StatusCode::NOT_FOUND,
            "not_found",
            "profile not found",
            None,
        ),
    }
}

#[derive(Debug, Deserialize)]
struct ProfileRequest {
    id: Option<String>,
    profile: Profile,
}

async fn create_profile(
    State(app): State<GatewayApp>,
    headers: HeaderMap,
    Json(request): Json<ProfileRequest>,
) -> Response {
    if !admin_ok(&headers, &app.admin_token) {
        return gateway_error(
            StatusCode::UNAUTHORIZED,
            "gateway_auth_error",
            "invalid admin token",
            None,
        );
    }
    let id = request
        .id
        .or_else(|| request.profile.gateway_profile_id.clone())
        .or_else(|| request.profile.provider.clone())
        .unwrap_or_else(|| "default".to_string());
    let mut store = app.store.write().await;
    if store.document.profiles.contains_key(&id) {
        return gateway_error(
            StatusCode::CONFLICT,
            "conflict",
            "profile already exists",
            None,
        );
    }
    store.document.profiles.insert(id.clone(), request.profile);
    if let Err(err) = store.save() {
        return app_error_response(StatusCode::INTERNAL_SERVER_ERROR, err);
    }
    Json(serde_json::json!({"id": id})).into_response()
}

async fn replace_profile(
    State(app): State<GatewayApp>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
    Json(request): Json<ProfileRequest>,
) -> Response {
    if !admin_ok(&headers, &app.admin_token) {
        return gateway_error(
            StatusCode::UNAUTHORIZED,
            "gateway_auth_error",
            "invalid admin token",
            None,
        );
    }
    let mut store = app.store.write().await;
    store.document.profiles.insert(id.clone(), request.profile);
    if let Err(err) = store.save() {
        return app_error_response(StatusCode::INTERNAL_SERVER_ERROR, err);
    }
    Json(serde_json::json!({"id": id})).into_response()
}

async fn delete_profile(
    State(app): State<GatewayApp>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Response {
    if !admin_ok(&headers, &app.admin_token) {
        return gateway_error(
            StatusCode::UNAUTHORIZED,
            "gateway_auth_error",
            "invalid admin token",
            None,
        );
    }
    let mut store = app.store.write().await;
    if store
        .document
        .tokens
        .values()
        .any(|token| token.enabled && token.grants.contains_key(&id))
    {
        return gateway_error(
            StatusCode::CONFLICT,
            "conflict",
            "profile is referenced by an enabled token",
            None,
        );
    }
    store.document.profiles.remove(&id);
    if let Err(err) = store.save() {
        return app_error_response(StatusCode::INTERNAL_SERVER_ERROR, err);
    }
    StatusCode::NO_CONTENT.into_response()
}

#[derive(Debug, Serialize)]
struct TokenView {
    id: String,
    label: Option<String>,
    enabled: bool,
    expires_at: Option<u64>,
    profiles: Vec<String>,
}

async fn list_tokens(State(app): State<GatewayApp>, headers: HeaderMap) -> Response {
    if !admin_ok(&headers, &app.admin_token) {
        return gateway_error(
            StatusCode::UNAUTHORIZED,
            "gateway_auth_error",
            "invalid admin token",
            None,
        );
    }
    let store = app.store.read().await;
    let values: Vec<_> = store
        .document
        .tokens
        .iter()
        .map(|(id, token)| TokenView {
            id: id.clone(),
            label: token.label.clone(),
            enabled: token.enabled,
            expires_at: token.expires_at,
            profiles: token.grants.keys().cloned().collect(),
        })
        .collect();
    Json(values).into_response()
}

#[derive(Debug, Deserialize)]
struct TokenRequest {
    label: Option<String>,
    expires_at: Option<u64>,
    grants: HashMap<String, Grant>,
}

async fn create_token(
    State(app): State<GatewayApp>,
    headers: HeaderMap,
    Json(request): Json<TokenRequest>,
) -> Response {
    if !admin_ok(&headers, &app.admin_token) {
        return gateway_error(
            StatusCode::UNAUTHORIZED,
            "gateway_auth_error",
            "invalid admin token",
            None,
        );
    }
    let mut secret = [0_u8; 32];
    rand::rng().fill_bytes(&mut secret);
    let id = format!("gw_{}", &hex_digest(&Sha256::digest(secret))[..16]);
    let token = format!("{id}.{}", B64.encode(secret));
    let digest = hex_digest(&Sha256::digest(token.as_bytes()));
    let stored = StoredToken {
        label: request.label,
        digest,
        enabled: true,
        expires_at: request.expires_at,
        grants: request.grants,
    };
    let mut store = app.store.write().await;
    if stored
        .grants
        .keys()
        .any(|id| !store.document.profiles.contains_key(id))
    {
        return gateway_error(
            StatusCode::BAD_REQUEST,
            "invalid_input",
            "token grant references an unknown profile",
            None,
        );
    }
    store.document.tokens.insert(id.clone(), stored);
    if let Err(err) = store.save() {
        return app_error_response(StatusCode::INTERNAL_SERVER_ERROR, err);
    }
    Json(serde_json::json!({"id": id, "token": token})).into_response()
}

async fn delete_token(
    State(app): State<GatewayApp>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Response {
    if !admin_ok(&headers, &app.admin_token) {
        return gateway_error(
            StatusCode::UNAUTHORIZED,
            "gateway_auth_error",
            "invalid admin token",
            None,
        );
    }
    let mut store = app.store.write().await;
    store.document.tokens.remove(&id);
    if let Err(err) = store.save() {
        return app_error_response(StatusCode::INTERNAL_SERVER_ERROR, err);
    }
    StatusCode::NO_CONTENT.into_response()
}

#[derive(Debug, Deserialize)]
struct TokenUpdate {
    enabled: Option<bool>,
    expires_at: Option<u64>,
    grants: Option<HashMap<String, Grant>>,
}

async fn update_token(
    State(app): State<GatewayApp>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
    Json(update): Json<TokenUpdate>,
) -> Response {
    if !admin_ok(&headers, &app.admin_token) {
        return gateway_error(
            StatusCode::UNAUTHORIZED,
            "gateway_auth_error",
            "invalid admin token",
            None,
        );
    }
    let mut store = app.store.write().await;
    if let Some(grants) = update.grants.as_ref() {
        if grants
            .keys()
            .any(|profile_id| !store.document.profiles.contains_key(profile_id))
        {
            return gateway_error(
                StatusCode::BAD_REQUEST,
                "invalid_input",
                "token grant references an unknown profile",
                None,
            );
        }
    }
    let mut next_document = store.document.clone();
    let Some(token) = next_document.tokens.get_mut(&id) else {
        return gateway_error(StatusCode::NOT_FOUND, "not_found", "token not found", None);
    };
    if let Some(enabled) = update.enabled {
        token.enabled = enabled;
    }
    if update.expires_at.is_some() {
        token.expires_at = update.expires_at;
    }
    if let Some(grants) = update.grants {
        token.grants = grants;
    }
    if let Err(err) = store.save_document(&next_document) {
        return app_error_response(StatusCode::INTERNAL_SERVER_ERROR, err);
    }
    store.document = next_document;
    StatusCode::NO_CONTENT.into_response()
}

fn redact_profile(profile: &Profile) -> serde_json::Value {
    serde_json::json!({ "provider": profile.provider, "auth_type": profile.auth_type, "base_url": profile.base_url, "site_url": profile.site_url, "has_token": profile.token.is_some(), "has_api_token": profile.api_token.is_some(), "has_password": profile.password.is_some(), "has_client_secret": profile.client_secret.is_some() })
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

pub async fn serve(
    bind: SocketAddr,
    state_path: PathBuf,
    key_path: PathBuf,
    admin_token: String,
) -> Result<(), AppError> {
    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|err| AppError::internal("gateway", "serve", err.to_string()))?;
    let store = GatewayStore::open(state_path, key_path)?;
    let app = router(GatewayApp {
        store: Arc::new(RwLock::new(store)),
        admin_token,
        client,
    });
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(|err| io_error("serve.bind", err))?;
    axum::serve(listener, app)
        .await
        .map_err(|err| AppError::internal("gateway", "serve", err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grant_requires_both_operation_and_method() {
        let grant = Grant {
            allow_operations: vec!["items.list".into()],
            allow_methods: vec!["GET".into()],
            ..Grant::default()
        };
        assert!(grant.permits("items.list", &Method::GET));
        assert!(!grant.permits("items.list", &Method::POST));
        assert!(!grant.permits("items.get", &Method::GET));
    }

    #[test]
    fn explicit_deny_wins() {
        let grant = Grant {
            allow_all_operations: true,
            allow_methods: vec!["GET".into()],
            deny_operations: vec!["items.get".into()],
            ..Grant::default()
        };
        assert!(!grant.permits("items.get", &Method::GET));
    }

    #[test]
    fn encrypted_state_round_trip_does_not_store_plaintext_credentials() {
        let temp = tempfile::tempdir().unwrap();
        let state_path = temp.path().join("gateway.enc.json");
        let key_path = temp.path().join("gateway.key");
        let mut store = GatewayStore::open(state_path.clone(), key_path.clone()).unwrap();
        store.document.profiles.insert(
            "github".into(),
            Profile {
                provider: Some("github".into()),
                token: Some("provider-secret".into()),
                ..Profile::default()
            },
        );
        store.save().unwrap();
        let ciphertext = std::fs::read_to_string(&state_path).unwrap();
        assert!(!ciphertext.contains("provider-secret"));
        let reopened = GatewayStore::open(state_path, key_path).unwrap();
        assert_eq!(
            reopened.document.profiles["github"].token.as_deref(),
            Some("provider-secret")
        );
    }

    #[test]
    fn token_digest_lookup_supports_revocation() {
        let mut document = GatewayDocument::default();
        let token = "gw_test.secret";
        document.tokens.insert(
            "test".into(),
            StoredToken {
                label: None,
                digest: hex_digest(&Sha256::digest(token.as_bytes())),
                enabled: true,
                expires_at: None,
                grants: HashMap::new(),
            },
        );
        assert!(find_token(&document, token).is_some());
        document.tokens.get_mut("test").unwrap().enabled = false;
        assert!(find_token(&document, "wrong").is_none());
        assert!(!find_token(&document, token).unwrap().enabled);
    }

    #[tokio::test]
    async fn failed_revocation_save_does_not_change_live_or_disk_state() {
        let temporary = tempfile::tempdir().unwrap();
        let state_path = temporary.path().join("state.enc.json");
        let key_path = temporary.path().join("state.key");
        let mut store = GatewayStore::open(state_path.clone(), key_path.clone()).unwrap();
        store.document.tokens.insert(
            "agent".into(),
            StoredToken {
                label: None,
                digest: hex_digest(&Sha256::digest(b"gw_agent.secret")),
                enabled: true,
                expires_at: None,
                grants: HashMap::new(),
            },
        );
        store.save().unwrap();
        let invalid_parent = temporary.path().join("not-a-directory");
        fs::write(&invalid_parent, b"file").unwrap();
        store.state_path = invalid_parent.join("state.enc.json");
        let app = GatewayApp {
            store: Arc::new(RwLock::new(store)),
            admin_token: "admin".into(),
            client: Client::new(),
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer admin"),
        );
        let response = update_token(
            State(app.clone()),
            headers,
            AxumPath("agent".into()),
            Json(TokenUpdate {
                enabled: Some(false),
                expires_at: None,
                grants: None,
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(app.store.read().await.document.tokens["agent"].enabled);
        let reopened = GatewayStore::open(state_path, key_path).unwrap();
        assert!(reopened.document.tokens["agent"].enabled);
    }

    #[test]
    fn bearer_scheme_is_case_insensitive() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("bEaReR secret"),
        );
        assert_eq!(bearer(&headers).as_deref(), Some("secret"));
    }

    #[test]
    fn redaction_covers_refresh_tokens() {
        let profile = Profile {
            refresh_token: Some("refresh-secret".into()),
            ..Profile::default()
        };
        let content_type = HeaderValue::from_static("application/json");
        assert_eq!(
            redact(
                br#"{"refresh_token":"refresh-secret"}"#,
                &profile,
                Some(&content_type),
            ),
            br#"{"refresh_token":"[REDACTED]"}"#
        );
    }

    #[test]
    fn server_targets_reject_private_redirects_and_other_profile_origins() {
        let profile = Profile {
            site_url: Some("https://example.atlassian.net".into()),
            ..Profile::default()
        };
        assert!(target_allowed(
            "jira",
            "https://example.atlassian.net/rest/api/3/issue/1",
            &profile,
        ));
        assert!(target_allowed(
            "jira",
            "https://api.media.atlassian.com/file/1",
            &profile,
        ));
        assert!(!target_allowed(
            "jira",
            "http://127.0.0.1:8080/internal",
            &profile,
        ));
        assert!(!target_allowed(
            "jira",
            "https://other.example.com/internal",
            &profile,
        ));
    }

    #[tokio::test]
    #[ignore = "requires AAI_E2E_CONFIG and saved GitHub credentials; calls a live read-only API"]
    async fn live_saved_github_credentials_match_direct_and_gateway_results() {
        run_live_pair(
            "AAI_E2E_GITHUB_PROFILE",
            "github-work",
            "github",
            "repos.get",
            &["aai-cli", "github", "repos", "get"],
        )
        .await;
    }

    #[tokio::test]
    #[ignore = "requires AAI_E2E_CONFIG and saved Bitbucket credentials; calls a live read-only API"]
    async fn live_saved_bitbucket_credentials_match_direct_and_gateway_results() {
        run_live_pair(
            "AAI_E2E_BITBUCKET_PROFILE",
            "bitbucket-work",
            "bitbucket",
            "repos.list",
            &["aai-cli", "bitbucket", "repos", "list", "--limit", "1"],
        )
        .await;
    }

    async fn run_live_pair(
        profile_env: &str,
        fallback_profile: &str,
        service: &str,
        operation: &str,
        argv: &[&str],
    ) {
        let config = std::env::var("AAI_E2E_CONFIG").expect("AAI_E2E_CONFIG is required");
        let profile_name =
            std::env::var(profile_env).unwrap_or_else(|_| fallback_profile.to_string());
        let ctx = Context::load(Some(&config), Some(&profile_name), None, None)
            .unwrap_or_else(|err| panic!("{service} test profile could not load: {}", err.code));
        let argv = argv.iter().map(ToString::to_string).collect::<Vec<_>>();
        let direct_command =
            Cli::try_parse_from(argv.clone()).expect("read-only live test command must parse");
        let direct = services::dispatch(&ctx, direct_command.command)
            .await
            .unwrap_or_else(|err| {
                panic!(
                    "direct {service}.{operation} failed: {} (HTTP {:?})",
                    err.code, err.status
                )
            });

        let temporary = tempfile::tempdir().unwrap();
        let mut store = GatewayStore::open(
            temporary.path().join("gateway.enc.json"),
            temporary.path().join("gateway.key"),
        )
        .unwrap();
        let mut managed_profile = ctx.profile.clone();
        managed_profile.provider = Some(service.to_string());
        managed_profile.username_env = None;
        managed_profile.username_secret = None;
        managed_profile.token_env = None;
        managed_profile.token_secret = None;
        managed_profile.api_token_env = None;
        managed_profile.api_token_secret = None;
        managed_profile.password_env = None;
        managed_profile.password_secret = None;
        managed_profile.refresh_token_env = None;
        managed_profile.refresh_token_secret = None;
        managed_profile.client_secret_env = None;
        managed_profile.client_secret_secret = None;
        store
            .document
            .profiles
            .insert("live".into(), managed_profile);
        let mut secret = [0_u8; 32];
        rand::rng().fill_bytes(&mut secret);
        let proxy_token = format!("gw_live.{}", B64.encode(secret));
        store.document.tokens.insert(
            "live".into(),
            StoredToken {
                label: None,
                digest: hex_digest(&Sha256::digest(proxy_token.as_bytes())),
                enabled: true,
                expires_at: None,
                grants: HashMap::from([(
                    "live".into(),
                    Grant {
                        allow_operations: vec![operation.into()],
                        allow_methods: vec!["GET".into()],
                        ..Grant::default()
                    },
                )]),
            },
        );
        store.save().unwrap();
        let app = GatewayApp {
            store: Arc::new(RwLock::new(store)),
            admin_token: "unused-live-test-admin".into(),
            client: Client::new(),
        };
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let gateway_url = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move { axum::serve(listener, router(app)).await });
        let remote_profile = Profile {
            provider: Some(service.into()),
            credential_source: Some("gateway".into()),
            gateway_url: Some(gateway_url),
            gateway_profile_id: Some("live".into()),
            gateway_token: Some(proxy_token),
            ..Profile::default()
        };
        let gateway = crate::gateway_command::execute(&remote_profile, argv)
            .await
            .unwrap_or_else(|err| panic!("gateway {service}.{operation} failed: {}", err.code));
        assert!(
            direct == gateway,
            "direct and gateway {service}.{operation} results differ"
        );
        let gateway_json = serde_json::to_string(&gateway).unwrap();
        for secret in [
            ctx.profile.token.as_deref(),
            ctx.profile.api_token.as_deref(),
        ]
        .into_iter()
        .flatten()
        .filter(|secret| !secret.is_empty())
        {
            assert!(
                !gateway_json.contains(secret),
                "gateway {service}.{operation} returned credential material"
            );
        }
        task.abort();
    }
}
