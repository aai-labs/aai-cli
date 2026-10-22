use reqwest::Method;
use serde_json::Value;

use crate::{cli::*, error::AppError, input};

pub async fn dispatch(command: GatewayCommand) -> Result<Value, AppError> {
    let base = std::env::var("AAI_GATEWAY_URL").map_err(|_| {
        AppError::config(
            "gateway commands require AAI_GATEWAY_URL or a gateway URL in the command environment",
        )
    })?;
    let token = std::env::var("AAI_GATEWAY_ADMIN_TOKEN").map_err(|_| {
        AppError::auth(
            "gateway",
            "admin",
            "gateway admin commands require AAI_GATEWAY_ADMIN_TOKEN",
        )
    })?;
    let client = reqwest::Client::new();
    match command.action {
        GatewayAction::Profiles(command) => match command.action {
            GatewayProfilesAction::List => {
                call(
                    &client,
                    &base,
                    &token,
                    Method::GET,
                    "/api/v1/admin/profiles",
                    None,
                )
                .await
            }
            GatewayProfilesAction::Get(args) => {
                call(
                    &client,
                    &base,
                    &token,
                    Method::GET,
                    &format!("/api/v1/admin/profiles/{}", encode(&args.id)),
                    None,
                )
                .await
            }
            GatewayProfilesAction::Create(args) => {
                call(
                    &client,
                    &base,
                    &token,
                    Method::POST,
                    "/api/v1/admin/profiles",
                    Some(read_json("gateway", "profiles.create", &args.json)?),
                )
                .await
            }
            GatewayProfilesAction::Replace(args) => {
                call(
                    &client,
                    &base,
                    &token,
                    Method::PUT,
                    &format!("/api/v1/admin/profiles/{}", encode(&args.id)),
                    Some(read_json("gateway", "profiles.replace", &args.json)?),
                )
                .await
            }
            GatewayProfilesAction::Remove(args) => {
                call(
                    &client,
                    &base,
                    &token,
                    Method::DELETE,
                    &format!("/api/v1/admin/profiles/{}", encode(&args.id)),
                    None,
                )
                .await
            }
        },
        GatewayAction::Tokens(command) => match command.action {
            GatewayTokensAction::List => {
                call(
                    &client,
                    &base,
                    &token,
                    Method::GET,
                    "/api/v1/admin/tokens",
                    None,
                )
                .await
            }
            GatewayTokensAction::Create(args) => {
                call(
                    &client,
                    &base,
                    &token,
                    Method::POST,
                    "/api/v1/admin/tokens",
                    Some(read_json("gateway", "tokens.create", &args.json)?),
                )
                .await
            }
            GatewayTokensAction::Update(args) => {
                call(
                    &client,
                    &base,
                    &token,
                    Method::PATCH,
                    &format!("/api/v1/admin/tokens/{}", encode(&args.id)),
                    Some(read_json("gateway", "tokens.update", &args.json)?),
                )
                .await
            }
            GatewayTokensAction::Remove(args) => {
                call(
                    &client,
                    &base,
                    &token,
                    Method::DELETE,
                    &format!("/api/v1/admin/tokens/{}", encode(&args.id)),
                    None,
                )
                .await
            }
        },
    }
}

fn read_json(service: &'static str, operation: &'static str, arg: &str) -> Result<Value, AppError> {
    input::read_json_arg(service, operation, Some(arg))
}

async fn call(
    client: &reqwest::Client,
    base: &str,
    token: &str,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> Result<Value, AppError> {
    let mut request = client
        .request(method, format!("{}{}", base.trim_end_matches('/'), path))
        .bearer_auth(token)
        .header("Accept", "application/json");
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = request
        .send()
        .await
        .map_err(|err| AppError::internal("gateway", "admin", err.to_string()))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|err| AppError::internal("gateway", "admin", err.to_string()))?;
    let value = if text.trim().is_empty() {
        Value::Object(Default::default())
    } else {
        serde_json::from_str(&text).unwrap_or(Value::String(text))
    };
    if status.is_success() {
        Ok(value)
    } else {
        Err(AppError::gateway(
            "gateway",
            "admin",
            status,
            "gateway admin request failed",
            Some(value),
        ))
    }
}

fn encode(value: &str) -> String {
    urlencoding::encode(value).into_owned()
}
