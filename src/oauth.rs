use reqwest::Client;
use serde_json::Value;

use crate::{config::Profile, error::AppError};

/// Resolve an access token for the given profile.
///
/// Priority:
///   1. If refresh_token + client_id + client_secret are all set → exchange for a fresh token
///   2. If a stored access token (token / api_token) is present → use it directly
///   3. Otherwise → error
pub(crate) async fn resolve_token(
    profile: &Profile,
    client: &Client,
    service: &'static str,
    operation: &'static str,
) -> Result<String, AppError> {
    if matches!(profile.auth_type.as_deref(), Some("none")) {
        return Ok(String::new());
    }

    match profile.auth_type.as_deref() {
        Some("microsoft_client_credentials") => {
            return microsoft_client_credentials(client, profile).await;
        }
        Some("microsoft_delegated") => {
            return microsoft_refresh(client, profile).await;
        }
        _ => {}
    }

    if let (Some(refresh_token), Some(client_id), Some(client_secret)) = (
        profile.refresh_token.as_deref(),
        profile.client_id.as_deref(),
        profile.client_secret.as_deref(),
    ) {
        return exchange(client, profile, refresh_token, client_id, client_secret).await;
    }

    if let Some(token) = profile.token.as_deref().or(profile.api_token.as_deref()) {
        return Ok(token.to_string());
    }

    Err(AppError::auth(
        service,
        operation,
        "profile has no access token or refresh credentials; \
         set token_secret, or set refresh_token_secret + client_id + client_secret_secret",
    ))
}

fn microsoft_endpoint(profile: &Profile) -> Result<String, AppError> {
    let tenant_id = profile
        .tenant_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::auth("microsoft", "token", "profile is missing tenant_id"))?;
    Ok(format!(
        "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
        urlencoding::encode(tenant_id)
    ))
}

async fn microsoft_client_credentials(
    client: &Client,
    profile: &Profile,
) -> Result<String, AppError> {
    let client_id = profile
        .client_id
        .as_deref()
        .ok_or_else(|| AppError::auth("microsoft", "token", "profile is missing client_id"))?;
    let client_secret = profile.client_secret.as_deref().ok_or_else(|| {
        AppError::auth(
            "microsoft",
            "token",
            "profile is missing client_secret_secret",
        )
    })?;
    let endpoint = microsoft_endpoint(profile)?;
    let response = client
        .post(endpoint)
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("scope", "https://graph.microsoft.com/.default"),
            ("grant_type", "client_credentials"),
        ])
        .send()
        .await
        .map_err(|err| AppError::internal("microsoft", "token", err.to_string()))?;
    access_token(response, "client_credentials").await
}

async fn microsoft_refresh(client: &Client, profile: &Profile) -> Result<String, AppError> {
    let refresh_token = profile.refresh_token.as_deref().ok_or_else(|| {
        AppError::auth(
            "microsoft",
            "refresh",
            "saved delegated credential is missing; run `aai-cli microsoft auth login`",
        )
    })?;
    let client_id = profile
        .client_id
        .as_deref()
        .ok_or_else(|| AppError::auth("microsoft", "refresh", "profile is missing client_id"))?;
    let scope = profile
        .scope
        .as_deref()
        .ok_or_else(|| AppError::auth("microsoft", "refresh", "profile is missing scope"))?;
    let endpoint = microsoft_endpoint(profile)?;
    let response = client
        .post(endpoint)
        .form(&[
            ("client_id", client_id),
            ("refresh_token", refresh_token),
            ("scope", scope),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .map_err(|err| AppError::internal("microsoft", "refresh", err.to_string()))?;
    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|err| AppError::internal("microsoft", "refresh", err.to_string()))?;
    if !status.is_success() {
        return Err(AppError::auth(
            "microsoft",
            "refresh",
            format!(
                "saved delegated credential could not be refreshed (HTTP {}): {body}; run `aai-cli microsoft auth login` to reauthorize",
                status.as_u16()
            ),
        ));
    }
    if let Some(replacement) = body.get("refresh_token").and_then(Value::as_str) {
        persist_microsoft_refresh_token(profile, replacement)?;
    }
    body.get("access_token")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            AppError::auth(
                "microsoft",
                "refresh",
                "token response missing access_token",
            )
        })
}

fn persist_microsoft_refresh_token(profile: &Profile, token: &str) -> Result<(), AppError> {
    let key = profile.refresh_token_secret.as_deref().ok_or_else(|| {
        AppError::auth(
            "microsoft",
            "refresh",
            "profile is missing refresh_token_secret",
        )
    })?;
    let secrets_file = profile.runtime_secrets_file.as_deref().ok_or_else(|| {
        AppError::internal("microsoft", "refresh", "secret-store path is unavailable")
    })?;
    let key_file = profile.runtime_key_file.as_deref().ok_or_else(|| {
        AppError::internal("microsoft", "refresh", "secret-key path is unavailable")
    })?;
    crate::secrets::set_at(secrets_file, key_file, key, token)
}

async fn access_token(response: reqwest::Response, flow: &'static str) -> Result<String, AppError> {
    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|err| AppError::internal("microsoft", flow, err.to_string()))?;
    if !status.is_success() {
        return Err(AppError::auth(
            "microsoft",
            flow,
            format!("token request failed (HTTP {}): {body}", status.as_u16()),
        ));
    }
    body.get("access_token")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| AppError::auth("microsoft", flow, "token response missing access_token"))
}

async fn exchange(
    client: &Client,
    profile: &Profile,
    refresh_token: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<String, AppError> {
    let endpoint = token_endpoint(profile);
    let body = format!(
        "grant_type=refresh_token&refresh_token={}&client_id={}&client_secret={}",
        urlencoding::encode(refresh_token),
        urlencoding::encode(client_id),
        urlencoding::encode(client_secret),
    );
    let resp = client
        .post(endpoint)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|e| AppError::internal("oauth", "refresh", e.to_string()))?;
    let status = resp.status();
    let body: Value = resp
        .json()
        .await
        .map_err(|e| AppError::internal("oauth", "refresh", e.to_string()))?;
    if !status.is_success() {
        return Err(AppError::auth(
            "oauth",
            "refresh",
            format!("token refresh failed (HTTP {}): {body}", status.as_u16()),
        ));
    }
    body.get("access_token")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| AppError::auth("oauth", "refresh", "token response missing access_token"))
}

fn token_endpoint(profile: &Profile) -> &'static str {
    match profile.auth_type.as_deref() {
        Some("zoho_oauth" | "zoho-oauth") => "https://accounts.zoho.com/oauth/v2/token",
        _ => "https://oauth2.googleapis.com/token",
    }
}

#[cfg(test)]
mod microsoft_tests {
    use super::*;

    #[test]
    fn rotated_microsoft_refresh_token_replaces_encrypted_value() {
        let temp = tempfile::tempdir().unwrap();
        let secrets_file = temp.path().join("secrets.enc.json");
        let key_file = temp.path().join("key");
        let profile = Profile {
            refresh_token_secret: Some("microsoft.refresh".to_string()),
            runtime_secrets_file: Some(secrets_file.clone()),
            runtime_key_file: Some(key_file.clone()),
            ..Profile::default()
        };

        persist_microsoft_refresh_token(&profile, "rotated-token").unwrap();
        let ctx = crate::config::Context {
            profile: Profile::default(),
            secrets_file,
            key_file,
        };
        assert_eq!(
            crate::secrets::get(&ctx, "microsoft.refresh").unwrap(),
            Some("rotated-token".to_string())
        );
    }
}
