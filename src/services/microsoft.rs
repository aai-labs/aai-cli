use std::{
    fs,
    time::{Duration, Instant},
};

use reqwest::Method;
use serde_json::{json, Value};

use crate::{
    cli::{
        MicrosoftAuthAction, MicrosoftCommand, MicrosoftFileDownload, MicrosoftFileTarget,
        MicrosoftFileUpload, MicrosoftFilesAction, MicrosoftResource,
    },
    config::Context,
    error::AppError,
    http::{ApiClient, BytesRequest},
    secrets,
    services::{
        generic_request,
        shared::{write_download, CtxProfile},
    },
};

mod common;
mod outlook;
mod planner;
mod sharepoint;
mod teams;
mod todo;

const SERVICE: &str = "microsoft";
const GRAPH_BASE: &str = "https://graph.microsoft.com/v1.0";

pub(crate) async fn dispatch(
    client: &ApiClient,
    ctx: &Context,
    command: MicrosoftCommand,
) -> Result<Value, AppError> {
    match command.resource {
        MicrosoftResource::Auth(command) => match command.action {
            MicrosoftAuthAction::Login => login(ctx).await,
            MicrosoftAuthAction::Status => status(client, ctx).await,
        },
        MicrosoftResource::Files(command) => files(client, ctx, command.action).await,
        MicrosoftResource::Mail(command) => outlook::mail(client, ctx, command).await,
        MicrosoftResource::Calendar(command) => outlook::calendar(client, ctx, command).await,
        MicrosoftResource::Contacts(command) => outlook::contacts(client, ctx, command).await,
        MicrosoftResource::Sharepoint(command) => sharepoint::dispatch(client, ctx, command).await,
        MicrosoftResource::Teams(command) => teams::dispatch(client, ctx, command).await,
        MicrosoftResource::Todo(command) => todo::dispatch(client, ctx, command).await,
        MicrosoftResource::Planner(command) => planner::dispatch(client, ctx, command).await,
        MicrosoftResource::Request(args) => {
            generic_request::dispatch(client, ctx, SERVICE, graph_base(ctx), args).await
        }
    }
}

async fn files(
    client: &ApiClient,
    ctx: &Context,
    action: MicrosoftFilesAction,
) -> Result<Value, AppError> {
    match action {
        MicrosoftFilesAction::Upload(args) => upload(client, ctx, args).await,
        MicrosoftFilesAction::Download(args) => download(client, ctx, args).await,
        MicrosoftFilesAction::Delete(args) => delete(client, ctx, args).await,
    }
}

async fn upload(
    client: &ApiClient,
    ctx: &Context,
    args: MicrosoftFileUpload,
) -> Result<Value, AppError> {
    let bytes = fs::read(&args.file).map_err(|err| {
        AppError::invalid_input(
            SERVICE,
            "files.upload",
            format!("failed to read {}: {err}", args.file),
        )
    })?;
    let response = client
        .request_bytes(
            SERVICE,
            "files.upload",
            ctx.profile(),
            BytesRequest {
                method: Method::PUT,
                url: content_url(ctx, &args.target),
                content_type: args.mime_type,
                headers: Vec::new(),
                body: bytes,
            },
        )
        .await?;
    Ok(redact_drive_item(response.body))
}

async fn download(
    client: &ApiClient,
    ctx: &Context,
    args: MicrosoftFileDownload,
) -> Result<Value, AppError> {
    let bytes = client
        .download(
            SERVICE,
            "files.download",
            ctx.profile(),
            content_url(ctx, &args.target),
        )
        .await?;
    let mut result = write_download(SERVICE, "files.download", &args.output, &bytes)?;
    result["path"] = json!(args.target.path);
    Ok(result)
}

async fn delete(
    client: &ApiClient,
    ctx: &Context,
    args: MicrosoftFileTarget,
) -> Result<Value, AppError> {
    client
        .request(
            SERVICE,
            "files.delete",
            ctx.profile(),
            Method::DELETE,
            item_url(ctx, &args),
            None,
        )
        .await
}

fn item_url(ctx: &Context, target: &MicrosoftFileTarget) -> String {
    let root = if let Some(drive_id) = target.drive_id.as_deref() {
        format!("{GRAPH_BASE}/drives/{}/root", urlencoding::encode(drive_id))
    } else if let Some(user_id) = target
        .user_id
        .as_deref()
        .or(ctx.profile().user_id.as_deref())
    {
        format!(
            "{GRAPH_BASE}/users/{}/drive/root",
            urlencoding::encode(user_id)
        )
    } else {
        format!("{GRAPH_BASE}/me/drive/root")
    };
    format!("{root}:/{}:", encode_drive_path(&target.path))
}

fn content_url(ctx: &Context, target: &MicrosoftFileTarget) -> String {
    format!("{}/content", item_url(ctx, target))
}

fn encode_drive_path(path: &str) -> String {
    path.trim_matches('/')
        .split('/')
        .map(|segment| urlencoding::encode(segment).into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn redact_drive_item(mut value: Value) -> Value {
    if let Some(object) = value.as_object_mut() {
        object.remove("@microsoft.graph.downloadUrl");
    }
    value
}

fn graph_base(ctx: &Context) -> String {
    ctx.profile()
        .base_url
        .clone()
        .unwrap_or_else(|| GRAPH_BASE.to_string())
}

async fn status(client: &ApiClient, ctx: &Context) -> Result<Value, AppError> {
    match ctx.profile().auth_type.as_deref() {
        Some("microsoft_delegated") => {
            let identity = client
                .request(
                    SERVICE,
                    "auth.status",
                    ctx.profile(),
                    Method::GET,
                    format!("{GRAPH_BASE}/me?%24select=id,displayName,userPrincipalName,mail"),
                    None,
                )
                .await?;
            verify_expected_user(ctx, &identity)?;
            Ok(json!({
                "authenticated": true,
                "mode": "delegated",
                "identity": identity,
                "credential": "encrypted_refresh_token",
            }))
        }
        Some("microsoft_client_credentials") => {
            let organization = client
                .request(
                    SERVICE,
                    "auth.status",
                    ctx.profile(),
                    Method::GET,
                    format!("{GRAPH_BASE}/organization?%24select=id,displayName"),
                    None,
                )
                .await?;
            Ok(json!({
                "authenticated": true,
                "mode": "application",
                "organization": organization,
                "credential": "encrypted_client_secret",
            }))
        }
        _ => Err(AppError::service_config(
            SERVICE,
            "auth.status",
            "profile auth_type must be microsoft_delegated or microsoft_client_credentials",
        )),
    }
}

async fn login(ctx: &Context) -> Result<Value, AppError> {
    if ctx.profile().auth_type.as_deref() != Some("microsoft_delegated") {
        return Err(AppError::service_config(
            SERVICE,
            "auth.login",
            "device login requires a microsoft_delegated profile",
        ));
    }
    let tenant_id = required(ctx.profile().tenant_id.as_deref(), "tenant_id")?;
    let client_id = required(ctx.profile().client_id.as_deref(), "client_id")?;
    let scope = required(ctx.profile().scope.as_deref(), "scope")?;
    let secret_key = required(
        ctx.profile().refresh_token_secret.as_deref(),
        "refresh_token_secret",
    )?;
    if !scope
        .split_whitespace()
        .any(|value| value == "offline_access")
    {
        return Err(AppError::service_config(
            SERVICE,
            "auth.login",
            "profile scope must include offline_access so the session can be persisted",
        ));
    }

    let http = reqwest::Client::new();
    let authority = format!(
        "https://login.microsoftonline.com/{}",
        urlencoding::encode(tenant_id)
    );
    let device_response = http
        .post(format!("{authority}/oauth2/v2.0/devicecode"))
        .form(&[("client_id", client_id), ("scope", scope)])
        .send()
        .await
        .map_err(|err| AppError::internal(SERVICE, "auth.login", err.to_string()))?;
    let device_status = device_response.status();
    let device: Value = device_response
        .json()
        .await
        .map_err(|err| AppError::internal(SERVICE, "auth.login", err.to_string()))?;
    if !device_status.is_success() {
        return Err(AppError::auth(
            SERVICE,
            "auth.login",
            format!(
                "device authorization failed (HTTP {}): {device}",
                device_status.as_u16()
            ),
        ));
    }

    let device_code = value_str(&device, "device_code")?;
    let user_code = value_str(&device, "user_code")?;
    let verification_uri = value_str(&device, "verification_uri")?;
    let expires_in = value_u64(&device, "expires_in")?;
    let mut interval = value_u64(&device, "interval").unwrap_or(5);
    eprintln!(
        "{}",
        json!({
            "event": "device_login_required",
            "verification_uri": verification_uri,
            "user_code": user_code,
            "message": device.get("message").and_then(Value::as_str),
        })
    );

    let deadline = Instant::now() + Duration::from_secs(expires_in);
    let token = loop {
        if Instant::now() >= deadline {
            return Err(AppError::auth(
                SERVICE,
                "auth.login",
                "device authorization expired before sign-in completed",
            ));
        }
        tokio::time::sleep(Duration::from_secs(interval)).await;
        let response = http
            .post(format!("{authority}/oauth2/v2.0/token"))
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("client_id", client_id),
                ("device_code", device_code),
            ])
            .send()
            .await
            .map_err(|err| AppError::internal(SERVICE, "auth.login", err.to_string()))?;
        let status = response.status();
        let body: Value = response
            .json()
            .await
            .map_err(|err| AppError::internal(SERVICE, "auth.login", err.to_string()))?;
        if status.is_success() {
            break body;
        }
        match body.get("error").and_then(Value::as_str) {
            Some("authorization_pending") => continue,
            Some("slow_down") => {
                interval += 5;
                continue;
            }
            _ => {
                return Err(AppError::auth(
                    SERVICE,
                    "auth.login",
                    format!(
                        "device authorization failed (HTTP {}): {body}",
                        status.as_u16()
                    ),
                ));
            }
        }
    };

    let access_token = value_str(&token, "access_token")?;
    let refresh_token = value_str(&token, "refresh_token")?;
    let identity_response = http
        .get(format!(
            "{GRAPH_BASE}/me?%24select=id,displayName,userPrincipalName,mail"
        ))
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|err| AppError::internal(SERVICE, "auth.login", err.to_string()))?;
    let identity_status = identity_response.status();
    let identity: Value = identity_response
        .json()
        .await
        .map_err(|err| AppError::internal(SERVICE, "auth.login", err.to_string()))?;
    if !identity_status.is_success() {
        return Err(AppError::api(
            SERVICE,
            "auth.login",
            identity_status,
            "failed to validate the authenticated Microsoft user",
            Some(identity),
        ));
    }
    verify_expected_user(ctx, &identity)?;
    secrets::set(ctx, secret_key, refresh_token)?;

    Ok(json!({
        "authenticated": true,
        "mode": "delegated",
        "identity": identity,
        "refresh_token_saved": true,
        "refresh_token_secret": secret_key,
    }))
}

fn verify_expected_user(ctx: &Context, identity: &Value) -> Result<(), AppError> {
    let Some(expected) = ctx.profile().user_id.as_deref() else {
        return Ok(());
    };
    let actual = identity.get("id").and_then(Value::as_str).unwrap_or("");
    if actual != expected {
        return Err(AppError::auth(
            SERVICE,
            "auth.identity",
            format!("authenticated Microsoft user ID '{actual}' does not match configured user_id '{expected}'"),
        ));
    }
    Ok(())
}

fn required<'a>(value: Option<&'a str>, field: &str) -> Result<&'a str, AppError> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            AppError::service_config(SERVICE, "auth.login", format!("profile is missing {field}"))
        })
}

fn value_str<'a>(value: &'a Value, field: &str) -> Result<&'a str, AppError> {
    value.get(field).and_then(Value::as_str).ok_or_else(|| {
        AppError::auth(
            SERVICE,
            "auth.login",
            format!("Microsoft response missing {field}"),
        )
    })
}

fn value_u64(value: &Value, field: &str) -> Result<u64, AppError> {
    value.get(field).and_then(Value::as_u64).ok_or_else(|| {
        AppError::auth(
            SERVICE,
            "auth.login",
            format!("Microsoft response missing {field}"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drive_paths_encode_each_segment_without_losing_folders() {
        assert_eq!(
            encode_drive_path("/Reports/Q3 plan #1.docx/"),
            "Reports/Q3%20plan%20%231.docx"
        );
    }

    #[test]
    fn drive_url_uses_me_without_a_configured_user() {
        let ctx = Context {
            profile: Default::default(),
            secrets_file: Default::default(),
            key_file: Default::default(),
        };
        let target = MicrosoftFileTarget {
            path: "report.docx".to_string(),
            drive_id: None,
            user_id: None,
        };

        assert_eq!(
            item_url(&ctx, &target),
            "https://graph.microsoft.com/v1.0/me/drive/root:/report.docx:"
        );
    }

    #[test]
    fn upload_output_does_not_expose_temporary_download_capability() {
        let value = redact_drive_item(json!({
            "id": "item-1",
            "@microsoft.graph.downloadUrl": "https://example.test/?tempauth=secret"
        }));
        assert_eq!(value, json!({"id": "item-1"}));
    }
}
