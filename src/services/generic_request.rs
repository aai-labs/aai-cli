use reqwest::Method;
use serde_json::Value;

use crate::{
    cli::{GenericHttpMethod, GenericRequest},
    config::Context,
    error::AppError,
    http::ApiClient,
    input,
    services::shared::{trim_url, CtxProfile},
};

pub(crate) async fn dispatch(
    client: &ApiClient,
    ctx: &Context,
    service: &'static str,
    base_url: String,
    args: GenericRequest,
) -> Result<Value, AppError> {
    let prepared = prepare(service, &base_url, args)?;
    client
        .request_no_redirect(
            service,
            "request",
            ctx.profile(),
            prepared.method,
            prepared.url,
            prepared.body,
        )
        .await
}

pub(super) struct PreparedRequest {
    pub method: Method,
    pub url: String,
    pub body: Option<Value>,
}

/// Build the existing generic request independently of response handling.
/// Providers with header pagination can reuse all path/write/query validation.
pub(super) fn prepare(
    service: &'static str,
    base_url: &str,
    args: GenericRequest,
) -> Result<PreparedRequest, AppError> {
    let operation = "request";
    let method = method(args.method);
    validate_options(&method, args.allow_write, args.json.is_some(), service)?;
    let mut url = relative_url(base_url, &args.path, service)?;
    append_query(&mut url, &args.query, service)?;
    let is_write = !matches!(method, Method::GET | Method::HEAD);
    let body = if is_write {
        args.json
            .as_deref()
            .map(|json| input::read_json_arg(service, operation, Some(json)))
            .transpose()?
    } else {
        None
    };

    Ok(PreparedRequest { method, url, body })
}

fn validate_options(
    method: &Method,
    allow_write: bool,
    has_json: bool,
    service: &'static str,
) -> Result<(), AppError> {
    let is_write = !matches!(*method, Method::GET | Method::HEAD);
    if is_write && !allow_write {
        return Err(AppError::invalid_input(
            service,
            "request",
            "mutating requests require --allow-write",
        ));
    }
    if !is_write && has_json {
        return Err(AppError::invalid_input(
            service,
            "request",
            "GET and HEAD requests do not accept --json",
        ));
    }
    Ok(())
}

fn method(method: GenericHttpMethod) -> Method {
    match method {
        GenericHttpMethod::Get => Method::GET,
        GenericHttpMethod::Head => Method::HEAD,
        GenericHttpMethod::Post => Method::POST,
        GenericHttpMethod::Put => Method::PUT,
        GenericHttpMethod::Patch => Method::PATCH,
        GenericHttpMethod::Delete => Method::DELETE,
    }
}

fn relative_url(base_url: &str, path: &str, service: &'static str) -> Result<String, AppError> {
    let path = path.trim();
    if path.is_empty() {
        return Err(AppError::invalid_input(
            service,
            "request",
            "relative endpoint path must not be empty",
        ));
    }
    if path.contains("://")
        || path.starts_with("//")
        || path.contains('\\')
        || path.contains('?')
        || path.contains('#')
    {
        return Err(AppError::invalid_input(
            service,
            "request",
            "path must be relative and must not contain a query, fragment, or backslash",
        ));
    }
    Ok(format!(
        "{}/{}",
        trim_url(base_url),
        path.trim_start_matches('/')
    ))
}

fn append_query(url: &mut String, query: &[String], service: &'static str) -> Result<(), AppError> {
    for (index, item) in query.iter().enumerate() {
        let (key, value) = item.split_once('=').ok_or_else(|| {
            AppError::invalid_input(
                service,
                "request",
                format!("query parameter must use key=value: {item}"),
            )
        })?;
        if key.trim().is_empty() {
            return Err(AppError::invalid_input(
                service,
                "request",
                "query parameter key must not be empty",
            ));
        }
        url.push(if index == 0 { '?' } else { '&' });
        url.push_str(&urlencoding::encode(key));
        url.push('=');
        url.push_str(&urlencoding::encode(value));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_path_is_appended_to_provider_base_path() {
        assert_eq!(
            relative_url(
                "https://api.bitbucket.org/2.0",
                "/repositories/acme/app",
                "bitbucket"
            )
            .unwrap(),
            "https://api.bitbucket.org/2.0/repositories/acme/app"
        );
    }

    #[test]
    fn absolute_and_embedded_query_paths_are_rejected() {
        for path in [
            "https://evil.example/data",
            "//evil.example/data",
            "/data?token=x",
            "/data#fragment",
            r"\evil",
        ] {
            assert!(relative_url("https://api.github.com", path, "github").is_err());
        }
    }

    #[test]
    fn query_values_are_encoded() {
        let mut url = "https://example.test/items".to_string();
        append_query(
            &mut url,
            &["status=open".to_string(), "q=hello world".to_string()],
            "test",
        )
        .unwrap();
        assert_eq!(
            url,
            "https://example.test/items?status=open&q=hello%20world"
        );
    }

    #[test]
    fn writes_require_explicit_opt_in() {
        let error = validate_options(&Method::POST, false, false, "github").unwrap_err();
        assert_eq!(error.code, "invalid_input");
        assert!(error.message.contains("--allow-write"));
        validate_options(&Method::POST, true, true, "github").unwrap();
    }

    #[test]
    fn reads_reject_json_bodies() {
        let error = validate_options(&Method::GET, false, true, "github").unwrap_err();
        assert_eq!(error.code, "invalid_input");
        assert!(error.message.contains("do not accept --json"));
    }
}
