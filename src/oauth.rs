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

    if matches!(
        profile.auth_type.as_deref(),
        Some("microsoft_client_credentials")
    ) {
        return client_credentials(profile, client, service, operation).await;
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

/// Graph's app-only scope: whatever application permissions the tenant admin granted.
const GRAPH_DEFAULT_SCOPE: &str = "https://graph.microsoft.com/.default";

/// App-only token via the client-credentials grant — no user, no refresh token.
///
/// The app acts as itself, so it reaches exactly what the tenant granted it (for SharePoint,
/// the sites granted under `Sites.Selected`). Nothing is stored between invocations; a fresh
/// token is minted per run, like the refresh-token path.
async fn client_credentials(
    profile: &Profile,
    client: &Client,
    service: &'static str,
    operation: &'static str,
) -> Result<String, AppError> {
    let (Some(client_id), Some(client_secret)) = (
        profile.client_id.as_deref(),
        profile.client_secret.as_deref(),
    ) else {
        return Err(AppError::auth(
            service,
            operation,
            "microsoft_client_credentials requires client_id and client_secret_secret",
        ));
    };
    let endpoint = client_credentials_endpoint(profile, service, operation)?;
    let body = client_credentials_body(profile, client_id, client_secret);
    let resp = client
        .post(endpoint)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|e| AppError::internal("oauth", "client_credentials", e.to_string()))?;
    let status = resp.status();
    let body: Value = resp
        .json()
        .await
        .map_err(|e| AppError::internal("oauth", "client_credentials", e.to_string()))?;
    if !status.is_success() {
        return Err(AppError::auth(
            "oauth",
            "client_credentials",
            format!("token request failed (HTTP {}): {body}", status.as_u16()),
        ));
    }
    body.get("access_token")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| {
            AppError::auth(
                "oauth",
                "client_credentials",
                "token response missing access_token",
            )
        })
}

/// Tenant-scoped token endpoint. There is deliberately no `common` fallback: the
/// multi-tenant authority cannot issue app-only tokens, so a missing tenant is a
/// configuration error rather than something to paper over.
fn client_credentials_endpoint(
    profile: &Profile,
    service: &'static str,
    operation: &'static str,
) -> Result<String, AppError> {
    let tenant = profile
        .tenant_id
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| {
            AppError::auth(
                service,
                operation,
                "microsoft_client_credentials requires tenant_id",
            )
        })?;
    Ok(format!(
        "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
        urlencoding::encode(tenant)
    ))
}

fn client_credentials_body(profile: &Profile, client_id: &str, client_secret: &str) -> String {
    let scope = profile.scope.as_deref().unwrap_or(GRAPH_DEFAULT_SCOPE);
    format!(
        "grant_type=client_credentials&client_id={}&client_secret={}&scope={}",
        urlencoding::encode(client_id),
        urlencoding::encode(client_secret),
        urlencoding::encode(scope),
    )
}

fn token_endpoint(profile: &Profile) -> &'static str {
    match profile.auth_type.as_deref() {
        Some("zoho_oauth" | "zoho-oauth") => "https://accounts.zoho.com/oauth/v2/token",
        _ => "https://oauth2.googleapis.com/token",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(auth_type: &str) -> Profile {
        Profile {
            auth_type: Some(auth_type.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn client_credentials_body_requests_the_graph_default_scope() {
        let body = client_credentials_body(&profile("microsoft_client_credentials"), "c id", "s&s");
        assert!(body.contains("grant_type=client_credentials"), "{body}");
        assert!(body.contains("client_id=c%20id"), "{body}");
        assert!(body.contains("client_secret=s%26s"), "{body}");
        assert!(
            body.contains("scope=https%3A%2F%2Fgraph.microsoft.com%2F.default"),
            "{body}"
        );
        assert!(!body.contains("refresh_token"), "{body}");
    }

    #[test]
    fn client_credentials_body_honours_an_explicit_scope() {
        let mut p = profile("microsoft_client_credentials");
        p.scope = Some("https://example.com/.default".to_string());
        let body = client_credentials_body(&p, "c", "s");
        assert!(
            body.contains("scope=https%3A%2F%2Fexample.com%2F.default"),
            "{body}"
        );
    }

    #[test]
    fn client_credentials_endpoint_is_tenant_scoped() {
        let mut p = profile("microsoft_client_credentials");
        p.tenant_id = Some("contoso.onmicrosoft.com".to_string());
        assert_eq!(
            client_credentials_endpoint(&p, "sharepoint", "sites.get").unwrap(),
            "https://login.microsoftonline.com/contoso.onmicrosoft.com/oauth2/v2.0/token"
        );
    }

    #[test]
    fn client_credentials_require_a_tenant() {
        // The multi-tenant `common` authority cannot issue app-only tokens.
        let err = client_credentials_endpoint(
            &profile("microsoft_client_credentials"),
            "sharepoint",
            "sites.get",
        )
        .unwrap_err();
        assert_eq!(err.code, "auth_error");
        assert!(err.message.contains("tenant_id"), "{}", err.message);
    }
}
