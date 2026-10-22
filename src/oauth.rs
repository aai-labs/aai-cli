use reqwest::Client;
use serde_json::Value;

use crate::{config::Profile, error::AppError};

/// Resolve an access token for the given profile.
///
/// Priority:
///   1. If auth_type is token_url → fetch a bearer from the configured endpoint
///   2. If refresh_token + client_id + client_secret are all set → exchange for a fresh token
///   3. If a stored access token (token / api_token) is present → use it directly
///   4. Otherwise → error
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
        // Must precede the stored-token fallthrough, which would hand back the
        // platform key itself as the provider bearer.
        Some("token_url") => {
            return fetch_from_token_url(client, profile, service).await;
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
    access_token(response, "microsoft", "client_credentials").await
}

/// Fetch a short-lived bearer from a platform endpoint that holds the real
/// credential, presenting the profile's api_token to authenticate.
async fn fetch_from_token_url(
    client: &Client,
    profile: &Profile,
    service: &'static str,
) -> Result<String, AppError> {
    let token_url = profile
        .token_url
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::auth(service, "token_url", "profile is missing token_url"))?;
    let api_token = profile.api_token.as_deref().ok_or_else(|| {
        AppError::auth(service, "token_url", "profile is missing api_token_secret")
    })?;
    let response = client
        .post(token_url)
        .bearer_auth(api_token)
        .send()
        .await
        .map_err(|err| AppError::internal(service, "token_url", err.to_string()))?;
    access_token(response, service, "token_url").await
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

async fn access_token(
    response: reqwest::Response,
    service: &'static str,
    flow: &'static str,
) -> Result<String, AppError> {
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|err| AppError::internal(service, flow, err.to_string()))?;
    // The status comes first: an error from a proxy in front of the endpoint is often not JSON.
    if !status.is_success() {
        let detail = serde_json::from_str::<Value>(&text)
            .map(|body| body.to_string())
            .unwrap_or_else(|_| text.chars().take(512).collect());
        return Err(AppError::auth(
            service,
            flow,
            format!("token request failed (HTTP {}): {detail}", status.as_u16()),
        ));
    }
    let body: Value = serde_json::from_str(&text)
        .map_err(|err| AppError::internal(service, flow, err.to_string()))?;
    body.get("access_token")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| AppError::auth(service, flow, "token response missing access_token"))
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

#[cfg(test)]
mod token_url_tests {
    use super::*;
    use crate::test_support::{json_response, serve};

    fn token_url_profile(address: &str) -> Profile {
        Profile {
            provider: Some("microsoft".to_string()),
            auth_type: Some("token_url".to_string()),
            token_url: Some(format!("http://{address}/token")),
            api_token: Some("platform-key".to_string()),
            ..Profile::default()
        }
    }

    #[tokio::test]
    async fn token_url_fetches_bearer_with_platform_key() {
        let (address, server) = serve(vec![json_response(
            r#"{"access_token":"graph-token","expires_in":3599}"#,
        )]);

        let token = resolve_token(
            &token_url_profile(&address),
            &Client::new(),
            "microsoft",
            "sites.list",
        )
        .await
        .unwrap();
        let requests = server.join().unwrap();

        assert_eq!(token, "graph-token");
        assert!(requests[0].starts_with("post /token "));
        assert!(requests[0].contains("authorization: bearer platform-key"));
    }

    #[tokio::test]
    async fn token_url_reports_non_success_status_as_auth_error() {
        let body = r#"{"detail":"not connected"}"#;
        let (address, server) = serve(vec![format!(
            "HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .into_bytes()]);

        let error = resolve_token(
            &token_url_profile(&address),
            &Client::new(),
            "microsoft",
            "sites.list",
        )
        .await
        .unwrap_err();
        server.join().unwrap();

        assert_eq!(error.code, "auth_error");
        assert_eq!(error.service, "microsoft");
        assert!(error.message.contains("HTTP 404"));
    }

    #[tokio::test]
    async fn token_url_reports_the_status_of_a_non_json_error() {
        // A proxy in front of the endpoint answers with HTML, not JSON.
        let body = "<html><body>502 Bad Gateway</body></html>";
        let (address, server) = serve(vec![format!(
            "HTTP/1.1 502 Bad Gateway\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .into_bytes()]);

        let error = resolve_token(
            &token_url_profile(&address),
            &Client::new(),
            "microsoft",
            "sites.list",
        )
        .await
        .unwrap_err();
        server.join().unwrap();

        assert_eq!(error.code, "auth_error");
        assert!(error.message.contains("HTTP 502"));
    }

    #[tokio::test]
    async fn token_url_rejects_response_without_access_token() {
        let (address, server) = serve(vec![json_response(r#"{"token_type":"Bearer"}"#)]);

        let error = resolve_token(
            &token_url_profile(&address),
            &Client::new(),
            "microsoft",
            "sites.list",
        )
        .await
        .unwrap_err();
        server.join().unwrap();

        assert_eq!(error.code, "auth_error");
        assert!(error.message.contains("missing access_token"));
    }

    #[tokio::test]
    async fn token_url_never_falls_back_to_the_platform_key() {
        let profile = Profile {
            token_url: None,
            ..token_url_profile("127.0.0.1:9")
        };

        let error = resolve_token(&profile, &Client::new(), "microsoft", "sites.list")
            .await
            .unwrap_err();

        assert_eq!(error.code, "auth_error");
        assert!(error.message.contains("token_url"));
    }

    #[tokio::test]
    async fn token_url_requires_the_platform_key() {
        let profile = Profile {
            api_token: None,
            ..token_url_profile("127.0.0.1:9")
        };

        let error = resolve_token(&profile, &Client::new(), "microsoft", "sites.list")
            .await
            .unwrap_err();

        assert_eq!(error.code, "auth_error");
        assert!(error.message.contains("api_token_secret"));
    }
}
