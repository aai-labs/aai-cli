//! Multica application API. Contract: 83527872634824012dd26bb183aa216fab631c05.
mod agents;
mod catalogs;
mod cli;
mod comments;
mod endpoint;
mod inbox;
mod issues;
mod pagination;
mod projects;
mod registry;
mod scheduling;
mod squads;
mod tasks;
mod workspaces;

use crate::{config::Context, error::AppError, http::ApiClient, input, services::generic_request};
use clap::ArgMatches;
use cli::Invocation;
pub use cli::MulticaCommand;
use endpoint::{Endpoint, Paging};
use reqwest::{Method, Url};
use serde_json::Value;
const SERVICE: &str = "multica";

pub(crate) async fn dispatch(
    client: &ApiClient,
    ctx: &Context,
    command: MulticaCommand,
) -> Result<Value, AppError> {
    let mut profile = ctx.profile.clone();
    if let Some(workspace) = command.workspace {
        profile.workspace = Some(workspace);
    }
    let base = base_url(profile.base_url.as_deref())?;
    if !matches!(
        profile.auth_type.as_deref(),
        None | Some("bearer_token") | Some("bearer")
    ) {
        return Err(AppError::auth(
            SERVICE,
            "request",
            "Multica requires bearer_token authentication with a personal access token",
        ));
    }
    // Multica uses a supplied PAT; never exchange unrelated OAuth credentials.
    profile.refresh_token = None;
    let ctx = Context {
        profile,
        secrets_file: ctx.secrets_file.clone(),
        key_file: ctx.key_file.clone(),
    };
    match command.invocation {
        Invocation::Request(args) => {
            if args
                .query
                .iter()
                .any(|q| q.starts_with("workspace_id=") || q.starts_with("workspace_slug="))
            {
                return Err(AppError::invalid_input(
                    SERVICE,
                    "request",
                    "use --workspace for request scoping",
                ));
            }
            let prepared = generic_request::prepare(SERVICE, &base, args)?;
            let (value, headers) = client
                .request_with_headers(
                    SERVICE,
                    "request",
                    &ctx.profile,
                    prepared.method,
                    prepared.url,
                    prepared.body,
                )
                .await?;
            Ok(pagination::raw(value, &headers))
        }
        Invocation::Endpoint(endpoint, args) => {
            if endpoint.workspace
                && ctx
                    .profile
                    .workspace
                    .as_deref()
                    .is_none_or(|w| w.trim().is_empty())
            {
                return Err(AppError::invalid_input(
                    SERVICE,
                    endpoint.command,
                    "set --workspace or profile.workspace (UUID or slug)",
                ));
            }
            let request = prepare(&base, &endpoint, &args)?;
            if endpoint.paging == Paging::None {
                client
                    .request_with_headers(
                        SERVICE,
                        endpoint.command,
                        &ctx.profile,
                        request.method,
                        request.url.to_string(),
                        request.body,
                    )
                    .await
                    .map(|(body, _)| body)
            } else {
                pagination::collect(client, &ctx.profile, &endpoint, &args, request.url).await
            }
        }
    }
}

fn base_url(base: Option<&str>) -> Result<String, AppError> {
    let base = base
        .ok_or_else(|| AppError::config("Multica profile requires base_url, including /api"))?
        .trim_end_matches('/');
    let url = Url::parse(base).map_err(|_| {
        AppError::config("Multica base_url must be an HTTP(S) API URL ending in /api")
    })?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.path().ends_with("/api")
    {
        return Err(AppError::config("Multica base_url must be an HTTP(S) URL ending in /api, without credentials, query, or fragment"));
    }
    Ok(base.to_owned())
}

struct Prepared {
    method: Method,
    url: Url,
    body: Option<Value>,
}
fn string<'a>(args: &'a ArgMatches, key: &str) -> Option<&'a str> {
    args.try_get_one::<String>(key)
        .ok()
        .flatten()
        .map(String::as_str)
}

fn prepare(base: &str, e: &Endpoint, args: &ArgMatches) -> Result<Prepared, AppError> {
    let mut path = e.path.to_owned();
    for key in e.path_fields() {
        let value = string(args, key).ok_or_else(|| {
            AppError::invalid_input(
                SERVICE,
                e.command,
                format!("missing --{}", key.replace('_', "-")),
            )
        })?;
        // URL parsers normalize dot path segments, even percent-encoded ones.
        if value.is_empty() || matches!(value, "." | "..") {
            return Err(AppError::invalid_input(
                SERVICE,
                e.command,
                "path identifiers must be nonempty and cannot be . or ..",
            ));
        }
        path = path.replace(&format!("{{{key}}}"), &urlencoding::encode(value));
    }
    let mut url = Url::parse(&format!("{base}{path}"))
        .map_err(|err| AppError::invalid_input(SERVICE, e.command, err.to_string()))?;
    let mut query = std::collections::BTreeMap::new();
    if let Some(values) = args.try_get_many::<String>("query").ok().flatten() {
        for value in values {
            let (key, value) = value
                .split_once('=')
                .filter(|(key, _)| !key.is_empty())
                .ok_or_else(|| {
                    AppError::invalid_input(SERVICE, e.command, "--query requires KEY=VALUE")
                })?;
            if matches!(
                key,
                "workspace_id"
                    | "workspace_slug"
                    | "limit"
                    | "offset"
                    | "cursor"
                    | "before"
                    | "before_id"
                    | "before-id"
            ) {
                return Err(AppError::invalid_input(
                    SERVICE,
                    e.command,
                    "use typed workspace/pagination options instead of --query",
                ));
            }
            query.insert(key.to_owned(), value.to_owned());
        }
    }
    for key in e
        .query
        .split_whitespace()
        .chain((matches!(e.paging, Paging::Offset | Paging::OffsetUntilShort)).then_some("offset"))
    {
        if let Some(value) = string(args, key) {
            query.insert(key.to_owned(), value.to_owned());
        }
    }
    if let Some(offset) = query.get("offset") {
        offset.parse::<usize>().map_err(|_| {
            AppError::invalid_input(SERVICE, e.command, "offset must be a nonnegative integer")
        })?;
    }
    if !query.is_empty() {
        url.query_pairs_mut().extend_pairs(query);
    }
    let body = if e.method == "GET" {
        None
    } else {
        let mut body = input::read_json_arg(SERVICE, e.command, string(args, "json"))?;
        let object = body.as_object_mut().ok_or_else(|| {
            AppError::invalid_input(SERVICE, e.command, "--json must contain a JSON object")
        })?;
        for key in e.fields.split_whitespace() {
            if let Some(value) = string(args, key) {
                let value = if cli::boolean(key) {
                    Value::Bool(value == "true")
                } else if matches!(
                    key,
                    "expected_revision" | "max_concurrent_tasks" | "position"
                ) {
                    serde_json::from_str::<Value>(value)
                        .ok()
                        .filter(Value::is_number)
                        .ok_or_else(|| {
                            AppError::invalid_input(
                                SERVICE,
                                e.command,
                                format!("{key} must be numeric"),
                            )
                        })?
                } else {
                    Value::String(value.to_owned())
                };
                object.insert(key.to_owned(), value);
            }
        }
        for key in e.required.split_whitespace() {
            if object
                .get(key)
                .is_none_or(|v| v.is_null() || v.as_str() == Some(""))
            {
                return Err(AppError::invalid_input(
                    SERVICE,
                    e.command,
                    format!(
                        "provide {key} through --{} or --json",
                        key.replace('_', "-")
                    ),
                ));
            }
        }
        Some(body)
    };
    Ok(Prepared {
        method: Method::from_bytes(e.method.as_bytes()).expect("static HTTP method"),
        url,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{CommandFactory, Parser};

    #[test]
    fn multica_command_tree_and_reference_examples_parse() {
        crate::cli::Cli::command().debug_assert();
        for document in [
            include_str!("../../../bundled/skills/aai-multica/references/workflows.md"),
            include_str!("../../../bundled/skills/aai-multica/references/command-reference.md"),
        ] {
            for line in document.lines().filter(|line| line.starts_with("aai-cli ")) {
                // The documented examples use single-quoted shell arguments, with no expansions.
                let mut quoted = false;
                let mut word = String::new();
                let mut words = Vec::new();
                for c in line.chars() {
                    if c == '\'' {
                        quoted = !quoted;
                    } else if c.is_whitespace() && !quoted {
                        if !word.is_empty() {
                            words.push(std::mem::take(&mut word));
                        }
                    } else {
                        word.push(c);
                    }
                }
                if !word.is_empty() {
                    words.push(word);
                }
                assert!(!quoted, "unbalanced example: {line}");
                crate::cli::Cli::try_parse_from(words).unwrap_or_else(|e| panic!("{line}: {e}"));
            }
        }
        let fixtures: Vec<Value> =
            serde_json::from_str(include_str!("../../../tests/fixtures/multica/routes.json"))
                .unwrap();
        assert_eq!(
            registry::endpoints().len(),
            fixtures.len(),
            "update the upstream route fixtures when adding commands"
        );
    }

    #[test]
    fn multica_base_urls_accept_deployment_prefixes_and_reject_ambiguous_roots() {
        for url in [
            "http://localhost:8080/api",
            "https://example.test/prefix/api/",
        ] {
            assert_eq!(base_url(Some(url)).unwrap(), url.trim_end_matches('/'));
        }
        for url in [
            "https://example.test",
            "https://user:secret@example.test/api",
            "https://example.test/api?token=x",
            "https://example.test/api#fragment",
            "file:///api",
        ] {
            assert!(base_url(Some(url)).is_err());
        }
        assert!(base_url(None).is_err());
    }
}
