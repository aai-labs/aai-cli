//! Agent Barn product API. See tests/fixtures/agent_barn/provenance.json.
mod cli;
mod pagination;
mod registry;
#[cfg(test)]
mod tests;

use crate::{config::Context, error::AppError, http::ApiClient, input, services::generic_request};
use clap::ArgMatches;
pub use cli::AgentBarnCommand;
use reqwest::{Method, Url};
use serde::Deserialize;
use serde_json::{json, Value};

const SERVICE: &str = "agent-barn";
const OP: &str = "request";
#[derive(Debug, Deserialize)]
struct Field {
    name: String,
    kind: String,
    required: bool,
    choices: Vec<String>,
    minimum: Option<f64>,
    maximum: Option<f64>,
}
#[derive(Debug, Deserialize)]
struct Endpoint {
    command: String,
    method: String,
    path: String,
    operation_id: String,
    query: Vec<Field>,
    path_fields: Vec<Field>,
    fields: Vec<Field>,
    body_required: bool,
    paging: String,
    key: String,
    mode: String,
    deprecated: bool,
}
fn invalid(message: impl Into<String>) -> AppError {
    AppError::invalid_input(SERVICE, OP, message)
}
fn string<'a>(args: &'a ArgMatches, key: &str) -> Option<&'a str> {
    args.try_get_one::<String>(key)
        .ok()
        .flatten()
        .map(String::as_str)
}
pub(crate) fn base_url(base: Option<&str>) -> Result<String, AppError> {
    let base = base
        .ok_or_else(|| AppError::config("Agent Barn requires base_url ending in /api/v1"))?
        .trim_end_matches('/');
    let url = Url::parse(base)
        .map_err(|_| AppError::config("Agent Barn base_url must be an HTTP(S) API URL"))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.path().ends_with("/api/v1")
    {
        return Err(AppError::config("Agent Barn base_url must end in /api/v1 and contain no credentials, query, or fragment"));
    }
    Ok(base.to_owned())
}
pub(crate) async fn dispatch(
    client: &ApiClient,
    ctx: &Context,
    command: AgentBarnCommand,
) -> Result<Value, AppError> {
    let base = base_url(ctx.profile.base_url.as_deref())?;
    let mut profile = ctx.profile.clone();
    if !matches!(
        profile.auth_type.as_deref(),
        None | Some("bearer_token") | Some("bearer")
    ) {
        return Err(AppError::auth(
            SERVICE,
            OP,
            "Agent Barn requires bearer_token authentication with an existing personal API key",
        ));
    }
    profile.refresh_token = None;
    let organization = command.organization.as_deref().or(profile.org.as_deref());
    match command.invocation {
        cli::Invocation::Request(args) => {
            let prepared = generic_request::prepare(SERVICE, &base, args)?;
            let url = Url::parse(&prepared.url).map_err(|_| invalid("invalid request URL"))?;
            let root = Url::parse(&base).map_err(|_| invalid("invalid base URL"))?;
            if url.origin() != root.origin()
                || !url.path().starts_with(&format!("{}/", root.path()))
            {
                return Err(invalid(
                    "request path must stay within the configured API base",
                ));
            }
            client
                .request_no_redirect(
                    SERVICE,
                    OP,
                    &profile,
                    prepared.method,
                    prepared.url,
                    prepared.body,
                )
                .await
        }
        cli::Invocation::Endpoint(e, args) => {
            let (url, body) = prepare(&base, e, &args, organization)?;
            if e.mode != "json" {
                let output =
                    string(&args, "output").ok_or_else(|| invalid("--output is required"))?;
                let duration = if e.mode == "stream" {
                    args.get_one::<u64>("duration_seconds").copied()
                } else {
                    None
                };
                return client
                    .capture_to_file(SERVICE, &profile, url.to_string(), output, duration)
                    .await;
            }
            if e.paging != "none" {
                return pagination::collect(client, &profile, e, &args, url).await;
            }
            let method = Method::from_bytes(e.method.as_bytes())
                .map_err(|_| invalid("invalid contract method"))?;
            client
                .request_no_redirect(SERVICE, OP, &profile, method, url.to_string(), body)
                .await
        }
    }
}
fn value(field: &Field, text: &str) -> Result<Value, AppError> {
    let v = match field.kind.as_str() {
        "integer" => json!(text.parse::<i64>().map_err(|_| invalid(format!(
            "--{} requires an integer",
            field.name.replace('_', "-")
        )))?),
        "number" => {
            let n = text
                .parse::<f64>()
                .map_err(|_| invalid("expected a finite number"))?;
            if !n.is_finite() {
                return Err(invalid("expected a finite number"));
            }
            json!(n)
        }
        "boolean" => json!(text
            .parse::<bool>()
            .map_err(|_| invalid("expected true or false"))?),
        _ => json!(text),
    };
    if let Some(n) = v.as_f64() {
        if field.minimum.is_some_and(|min| n < min) || field.maximum.is_some_and(|max| n > max) {
            return Err(invalid(format!(
                "--{} is outside the provider's allowed range",
                field.name.replace('_', "-")
            )));
        }
    }
    Ok(v)
}
fn prepare(
    base: &str,
    e: &Endpoint,
    args: &ArgMatches,
    organization: Option<&str>,
) -> Result<(Url, Option<Value>), AppError> {
    let mut path = e.path.clone();
    for f in &e.path_fields {
        let text = if f.name == "organization_id" {
            organization
        } else {
            string(args, &f.name)
        }
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| {
            invalid(format!(
                "set --{}{}",
                f.name.replace('_', "-"),
                if f.name == "organization_id" {
                    " or profile.org"
                } else {
                    ""
                }
            ))
        })?;
        if matches!(text, "." | "..") {
            return Err(invalid("path identifiers cannot be dot segments"));
        }
        path = path.replace(&format!("{{{}}}", f.name), &urlencoding::encode(text));
    }
    let mut url =
        Url::parse(&format!("{base}{path}")).map_err(|_| invalid("invalid endpoint URL"))?;
    for f in &e.query {
        if f.name == "limit" && e.paging != "none" {
            continue;
        }
        // The profile organization scopes tenant paths, not optional platform filters.
        let values: Vec<&str> = if f.name == "organization_id" {
            string(args, &f.name).into_iter().collect()
        } else {
            args.try_get_many::<String>(&f.name)
                .ok()
                .flatten()
                .into_iter()
                .flatten()
                .map(String::as_str)
                .collect()
        };
        for text in values {
            value(f, text)?;
            url.query_pairs_mut().append_pair(&f.name, text);
        }
    }
    let mut body = string(args, "json")
        .map(|p| input::read_json_arg(SERVICE, OP, Some(p)))
        .transpose()?;
    for f in &e.fields {
        if let Some(text) = string(args, &f.name) {
            let object = body
                .get_or_insert_with(|| json!({}))
                .as_object_mut()
                .ok_or_else(|| invalid("typed body flags require a JSON object"))?;
            object.insert(f.name.clone(), value(f, text)?);
        }
    }
    if e.body_required && body.is_none() {
        return Err(invalid("this operation requires --json or body flags"));
    }
    Ok((url, body))
}
