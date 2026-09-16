use reqwest::Method;
use serde_json::Value;

use crate::{
    config::Context,
    error::AppError,
    http::{ApiClient, BytesRequest},
    input,
    services::shared::CtxProfile,
};

use super::{GRAPH_BASE, SERVICE};

pub(super) fn graph_url(path: &str) -> String {
    format!("{GRAPH_BASE}/{}", path.trim_start_matches('/'))
}

pub(super) fn user_root(ctx: &Context, user_id: Option<&str>) -> Result<String, AppError> {
    let user_id = user_id
        .or(ctx.profile().user_id.as_deref())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            AppError::service_config(
                SERVICE,
                "user.resolve",
                "command requires --user-id or profile.user_id",
            )
        })?;
    Ok(format!("users/{}", urlencoding::encode(user_id)))
}

pub(super) fn parse_body(operation: &'static str, json: Option<&str>) -> Result<Value, AppError> {
    let body = input::read_json_arg(SERVICE, operation, json)?;
    if !body.is_object() || body.as_object().is_some_and(serde_json::Map::is_empty) {
        return Err(AppError::invalid_input(
            SERVICE,
            operation,
            "provide a non-empty JSON object with --json",
        ));
    }
    Ok(body)
}

pub(super) async fn request(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> Result<Value, AppError> {
    client
        .request(
            SERVICE,
            operation,
            ctx.profile(),
            method,
            graph_url(path),
            body,
        )
        .await
}

pub(super) async fn request_with_headers(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    method: Method,
    path: &str,
    body: Option<Value>,
    headers: Vec<(String, String)>,
) -> Result<Value, AppError> {
    let bytes = body
        .map(|value| serde_json::to_vec(&value))
        .transpose()
        .map_err(|err| AppError::internal(SERVICE, operation, err.to_string()))?
        .unwrap_or_default();
    client
        .request_bytes(
            SERVICE,
            operation,
            ctx.profile(),
            BytesRequest {
                method,
                url: graph_url(path),
                content_type: "application/json".to_string(),
                headers,
                body: bytes,
            },
        )
        .await
        .map(|response| response.body)
}

pub(super) async fn collection(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    path: &str,
    limit: u32,
) -> Result<Value, AppError> {
    collection_with_page_size(client, ctx, operation, path, limit, true).await
}

pub(super) async fn collection_without_page_size(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    path: &str,
    limit: u32,
) -> Result<Value, AppError> {
    collection_with_page_size(client, ctx, operation, path, limit, false).await
}

async fn collection_with_page_size(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    path: &str,
    limit: u32,
    include_top: bool,
) -> Result<Value, AppError> {
    if limit == 0 {
        return Err(AppError::invalid_input(
            SERVICE,
            operation,
            "--limit must be greater than zero",
        ));
    }
    let first_url = if include_top {
        let separator = if path.contains('?') { '&' } else { '?' };
        graph_url(&format!("{path}{separator}%24top={limit}"))
    } else {
        graph_url(path)
    };
    let mut next = Some(first_url);
    let mut first: Option<serde_json::Map<String, Value>> = None;
    let mut values = Vec::new();
    let mut continuation = None;
    let mut truncated = false;

    while let Some(url) = next.take() {
        if !url.starts_with("https://graph.microsoft.com/") {
            return Err(AppError::internal(
                SERVICE,
                operation,
                "Microsoft Graph returned an untrusted pagination URL",
            ));
        }
        let page = client
            .request(SERVICE, operation, ctx.profile(), Method::GET, url, None)
            .await?;
        let mut object = page.as_object().cloned().ok_or_else(|| {
            AppError::internal(SERVICE, operation, "collection response is not an object")
        })?;
        let page_values = object
            .remove("value")
            .and_then(|value| value.as_array().cloned())
            .ok_or_else(|| {
                AppError::internal(SERVICE, operation, "collection response is missing value[]")
            })?;
        continuation = object
            .get("@odata.nextLink")
            .and_then(Value::as_str)
            .map(str::to_string);
        if first.is_none() {
            first = Some(object);
        }
        let remaining = limit as usize - values.len();
        truncated |= page_values.len() > remaining;
        values.extend(page_values.into_iter().take(remaining));
        if values.len() >= limit as usize {
            break;
        }
        next = continuation.clone();
    }

    let mut output = first.unwrap_or_default();
    output.insert("value".to_string(), Value::Array(values));
    if truncated {
        output.insert("truncated".to_string(), Value::Bool(true));
    }
    match continuation {
        Some(next_link) => {
            output.insert("@odata.nextLink".to_string(), Value::String(next_link));
        }
        None => {
            output.remove("@odata.nextLink");
        }
    }
    Ok(Value::Object(output))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_root_requires_explicit_or_profile_user() {
        let ctx = Context {
            profile: Default::default(),
            secrets_file: Default::default(),
            key_file: Default::default(),
        };
        assert!(user_root(&ctx, None).is_err());
        assert_eq!(
            user_root(&ctx, Some("a@b.test")).unwrap(),
            "users/a%40b.test"
        );
    }

    #[test]
    fn body_must_be_non_empty_object() {
        assert!(parse_body("create", None).is_err());
        assert!(parse_body("create", Some("[]")).is_err());
        assert_eq!(
            parse_body("create", Some(r#"{"title":"x"}"#)).unwrap()["title"],
            "x"
        );
    }
}
