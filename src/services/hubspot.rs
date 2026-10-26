use reqwest::Method;
use serde_json::{json, Value};

use crate::{
    cli::*,
    config::{Context, Profile},
    error::AppError,
    http::ApiClient,
    input,
    services::{
        generic_request,
        shared::{enc, hubspot_base, CtxProfile},
    },
};

#[derive(Clone, Copy, Debug)]
struct Capability {
    endpoint: &'static str,
    required_scopes: &'static [&'static str],
    caveat: Option<&'static str>,
}

const CRM_READ_SCOPES: &[&str] = &["crm.objects.contacts.read"];
const FILES_SCOPES: &[&str] = &["files"];
const HIDDEN_FILES_SCOPES: &[&str] = &["files", "files.ui_hidden.read"];
const EVENT_OCCURRENCE_SCOPES: &[&str] = &["business-intelligence"];
const CUSTOM_EVENT_SCOPES: &[&str] = &["analytics.behavioral_events.send"];
const CONVERSATIONS_READ_SCOPES: &[&str] = &["conversations.read"];
const CONVERSATIONS_WRITE_SCOPES: &[&str] = &["conversations.read", "conversations.write"];
const CUSTOM_CHANNEL_READ_SCOPES: &[&str] = &["conversations.custom_channels.read"];
const CUSTOM_CHANNEL_WRITE_SCOPES: &[&str] = &[
    "conversations.custom_channels.read",
    "conversations.custom_channels.write",
];
const VISITOR_TOKEN_SCOPES: &[&str] = &["conversations.visitor_identification.tokens.create"];

pub(crate) async fn dispatch(
    client: &ApiClient,
    ctx: &Context,
    command: HubspotCommand,
) -> Result<Value, AppError> {
    match command.resource {
        HubspotResource::Health => {
            let cap = Capability {
                endpoint: "/account-info/v3/details",
                required_scopes: &[],
                caveat: None,
            };
            request(
                client,
                ctx,
                "health",
                Method::GET,
                "/account-info/v3/details",
                None,
                cap,
            )
            .await
        }
        HubspotResource::Crm(command) => crm(client, ctx, command).await,
        HubspotResource::Files(command) => files(client, ctx, command).await,
        HubspotResource::Events(command) => events(client, ctx, command).await,
        HubspotResource::Conversations(command) => conversations(client, ctx, command).await,
        HubspotResource::Request(args) => {
            let cap = capability_for_path(&args.path, args.method);
            if is_custom_channels_path(&args.path) {
                reject_legacy_custom_channels(ctx.profile(), "request", cap)?;
            }
            generic_request::dispatch(client, ctx, "hubspot", hubspot_base(ctx.profile()), args)
                .await
                .map_err(|err| enrich_auth_error(ctx.profile(), "request", cap, err))
        }
    }
}

async fn crm(
    client: &ApiClient,
    ctx: &Context,
    command: HubspotCrmCommand,
) -> Result<Value, AppError> {
    match command.object {
        HubspotCrmObjectCommand::Contacts(actions) => {
            crm_object(client, ctx, "contacts", "crm.contacts", actions).await
        }
        HubspotCrmObjectCommand::Companies(actions) => {
            crm_object(client, ctx, "companies", "crm.companies", actions).await
        }
        HubspotCrmObjectCommand::Deals(actions) => {
            crm_object(client, ctx, "deals", "crm.deals", actions).await
        }
        HubspotCrmObjectCommand::Tickets(actions) => {
            crm_object(client, ctx, "tickets", "crm.tickets", actions).await
        }
    }
}

async fn crm_object(
    client: &ApiClient,
    ctx: &Context,
    object: &'static str,
    operation_prefix: &'static str,
    actions: HubspotCrmObjectActions,
) -> Result<Value, AppError> {
    match actions.action {
        HubspotCrmObjectAction::List(args) => {
            let operation = crm_operation(operation_prefix, "list");
            let capability = crm_capability(object);
            let HubspotListArgs {
                limit,
                after,
                properties,
            } = args;
            paginate(
                client,
                ctx,
                operation,
                limit,
                capability,
                after,
                |per_page, cursor| {
                    let mut path = format!("/crm/v3/objects/{object}?limit={}", per_page);
                    push_query(&mut path, "after", cursor.as_deref());
                    push_query(&mut path, "properties", properties.as_deref());
                    (
                        Method::GET,
                        format!("{}{}", hubspot_base(ctx.profile()), path),
                        None,
                    )
                },
            )
            .await
        }
        HubspotCrmObjectAction::Get(args) => {
            let mut path = format!("/crm/v3/objects/{}/{}", object, enc(&args.id));
            let mut sep = "?";
            if let Some(properties) = args.properties.as_deref() {
                path.push_str(sep);
                sep = "&";
                path.push_str("properties=");
                path.push_str(&enc(properties));
            }
            if args.archived {
                path.push_str(sep);
                path.push_str("archived=true");
            }
            let operation = crm_operation(operation_prefix, "get");
            request(
                client,
                ctx,
                operation,
                Method::GET,
                &path,
                None,
                crm_capability(object),
            )
            .await
        }
        HubspotCrmObjectAction::Search(args) => {
            let operation = crm_operation(operation_prefix, "search");
            let capability = crm_capability(object);
            let HubspotSearchArgs { json, limit } = args;
            let base_body = input::read_json_arg("hubspot", "crm.search", json.as_deref())?;
            let initial_after = base_body
                .get("after")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string);
            let search_path = format!("/crm/v3/objects/{object}/search");
            paginate(
                client,
                ctx,
                operation,
                limit,
                capability,
                initial_after,
                |per_page, cursor| {
                    let mut body = base_body.clone();
                    input::set_u64(&mut body, "limit", Some(per_page as u64));
                    match cursor {
                        Some(cursor) => input::ensure_object(&mut body)
                            .insert("after".to_string(), json!(cursor)),
                        None => input::ensure_object(&mut body).remove("after"),
                    };
                    (
                        Method::POST,
                        format!("{}{search_path}", hubspot_base(ctx.profile())),
                        Some(body),
                    )
                },
            )
            .await
        }
    }
}

async fn files(
    client: &ApiClient,
    ctx: &Context,
    command: HubspotFilesCommand,
) -> Result<Value, AppError> {
    match command.action {
        HubspotFilesAction::List(args) => {
            let capability = Capability {
                endpoint: "/files/v3/files",
                required_scopes: FILES_SCOPES,
                caveat: None,
            };
            let HubspotFilesList {
                limit,
                after,
                folder_id,
            } = args;
            paginate(
                client,
                ctx,
                "files.list",
                limit,
                capability,
                after,
                |per_page, cursor| {
                    let mut path = format!("/files/v3/files?limit={}", per_page);
                    push_query(&mut path, "after", cursor.as_deref());
                    push_query(&mut path, "parentFolderId", folder_id.as_deref());
                    (
                        Method::GET,
                        format!("{}{}", hubspot_base(ctx.profile()), path),
                        None,
                    )
                },
            )
            .await
        }
        HubspotFilesAction::Get(args) => {
            let scopes = if args.hidden_or_deleted {
                HIDDEN_FILES_SCOPES
            } else {
                FILES_SCOPES
            };
            request(
                client,
                ctx,
                "files.get",
                Method::GET,
                &format!("/files/v3/files/{}", enc(&args.id)),
                None,
                Capability {
                    endpoint: "/files/v3/files/{id}",
                    required_scopes: scopes,
                    caveat: Some("Hidden or deleted file reads may require files.ui_hidden.read."),
                },
            )
            .await
        }
    }
}

async fn events(
    client: &ApiClient,
    ctx: &Context,
    command: HubspotEventsCommand,
) -> Result<Value, AppError> {
    match command.resource {
        HubspotEventsResource::Occurrences(command) => match command.action {
            HubspotEventOccurrencesAction::List(args) => {
                let capability = Capability {
                    endpoint: "/events/v3/events/{eventType}",
                    required_scopes: EVENT_OCCURRENCE_SCOPES,
                    caveat: Some("Event occurrence reads can also be limited by account tier."),
                };
                let HubspotEventOccurrencesList {
                    event_type,
                    limit,
                    after,
                } = args;
                paginate(
                    client,
                    ctx,
                    "events.occurrences.list",
                    limit,
                    capability,
                    after,
                    |per_page, cursor| {
                        let mut path =
                            format!("/events/v3/events/{}?limit={}", enc(&event_type), per_page);
                        push_query(&mut path, "after", cursor.as_deref());
                        (
                            Method::GET,
                            format!("{}{}", hubspot_base(ctx.profile()), path),
                            None,
                        )
                    },
                )
                .await
            }
        },
        HubspotEventsResource::Custom(command) => match command.action {
            HubspotCustomEventsAction::Send(args) => {
                let body = input::read_json_arg("hubspot", "events.custom.send", Some(&args.json))?;
                request(
                    client,
                    ctx,
                    "events.custom.send",
                    Method::POST,
                    "/events/v3/send",
                    Some(body),
                    Capability {
                        endpoint: "/events/v3/send",
                        required_scopes: CUSTOM_EVENT_SCOPES,
                        caveat: None,
                    },
                )
                .await
            }
        },
    }
}

async fn conversations(
    client: &ApiClient,
    ctx: &Context,
    command: HubspotConversationsCommand,
) -> Result<Value, AppError> {
    match command.resource {
        HubspotConversationsResource::Inboxes(command) => match command.action {
            HubspotConversationsInboxesAction::List(args) => {
                simple_list(
                    client,
                    ctx,
                    "conversations.inboxes.list",
                    "/conversations/v3/conversations/inboxes",
                    args,
                    CONVERSATIONS_READ_SCOPES,
                )
                .await
            }
        },
        HubspotConversationsResource::Threads(command) => match command.action {
            HubspotConversationsThreadsAction::List(args) => {
                simple_list(
                    client,
                    ctx,
                    "conversations.threads.list",
                    "/conversations/v3/conversations/threads",
                    args,
                    CONVERSATIONS_READ_SCOPES,
                )
                .await
            }
            HubspotConversationsThreadsAction::Get(args) => {
                request(
                    client,
                    ctx,
                    "conversations.threads.get",
                    Method::GET,
                    &format!("/conversations/v3/conversations/threads/{}", enc(&args.id)),
                    None,
                    Capability {
                        endpoint: "/conversations/v3/conversations/threads/{threadId}",
                        required_scopes: CONVERSATIONS_READ_SCOPES,
                        caveat: None,
                    },
                )
                .await
            }
        },
        HubspotConversationsResource::VisitorIdentification(command) => match command.action {
            HubspotVisitorIdentificationAction::Tokens(command) => match command.action {
                HubspotVisitorTokensAction::Create(args) => {
                    let body = input::read_json_arg(
                        "hubspot",
                        "conversations.visitor-identification.tokens.create",
                        Some(&args.json),
                    )?;
                    request(
                        client,
                        ctx,
                        "conversations.visitor-identification.tokens.create",
                        Method::POST,
                        "/conversations/v3/visitor-identification/tokens/create",
                        Some(body),
                        Capability {
                            endpoint: "/conversations/v3/visitor-identification/tokens/create",
                            required_scopes: VISITOR_TOKEN_SCOPES,
                            caveat: None,
                        },
                    )
                    .await
                }
            },
        },
        HubspotConversationsResource::CustomChannels(command) => {
            custom_channels(client, ctx, command).await
        }
    }
}

async fn custom_channels(
    client: &ApiClient,
    ctx: &Context,
    command: HubspotCustomChannelsCommand,
) -> Result<Value, AppError> {
    let operation = match command.action {
        HubspotCustomChannelsAction::List(_) => "conversations.custom-channels.list",
        HubspotCustomChannelsAction::Get(_) => "conversations.custom-channels.get",
        HubspotCustomChannelsAction::Create(_) => "conversations.custom-channels.create",
    };
    let capability = match command.action {
        HubspotCustomChannelsAction::Create(_) => Capability {
            endpoint: "/conversations/v3/custom-channels",
            required_scopes: CUSTOM_CHANNEL_WRITE_SCOPES,
            caveat: Some(
                "HubSpot custom channel endpoints are not supported for legacy private apps.",
            ),
        },
        _ => Capability {
            endpoint: "/conversations/v3/custom-channels",
            required_scopes: CUSTOM_CHANNEL_READ_SCOPES,
            caveat: Some(
                "HubSpot custom channel endpoints are not supported for legacy private apps.",
            ),
        },
    };
    reject_legacy_custom_channels(ctx.profile(), operation, capability)?;

    match command.action {
        HubspotCustomChannelsAction::List(args) => {
            simple_list(
                client,
                ctx,
                operation,
                "/conversations/v3/custom-channels",
                args,
                CUSTOM_CHANNEL_READ_SCOPES,
            )
            .await
        }
        HubspotCustomChannelsAction::Get(args) => {
            request(
                client,
                ctx,
                operation,
                Method::GET,
                &format!("/conversations/v3/custom-channels/{}", enc(&args.id)),
                None,
                capability,
            )
            .await
        }
        HubspotCustomChannelsAction::Create(args) => {
            let body = input::read_json_arg("hubspot", operation, Some(&args.json))?;
            request(
                client,
                ctx,
                operation,
                Method::POST,
                "/conversations/v3/custom-channels",
                Some(body),
                capability,
            )
            .await
        }
    }
}

async fn simple_list(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    endpoint: &'static str,
    args: HubspotSimpleList,
    required_scopes: &'static [&'static str],
) -> Result<Value, AppError> {
    let capability = Capability {
        endpoint,
        required_scopes,
        caveat: None,
    };
    let HubspotSimpleList { limit, after } = args;
    paginate(
        client,
        ctx,
        operation,
        limit,
        capability,
        after,
        |per_page, cursor| {
            let mut path = format!("{endpoint}?limit={}", per_page);
            push_query(&mut path, "after", cursor.as_deref());
            (
                Method::GET,
                format!("{}{}", hubspot_base(ctx.profile()), path),
                None,
            )
        },
    )
    .await
}

async fn request(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    method: Method,
    path: &str,
    body: Option<Value>,
    capability: Capability,
) -> Result<Value, AppError> {
    let url = format!("{}{}", hubspot_base(ctx.profile()), path);
    client
        .request("hubspot", operation, ctx.profile(), method, url, body)
        .await
        .map_err(|err| enrich_auth_error(ctx.profile(), operation, capability, err))
}

const HUBSPOT_MAX_PER_PAGE: u32 = 100;

async fn paginate<F>(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    limit: u32,
    capability: Capability,
    initial_after: Option<String>,
    mut build: F,
) -> Result<Value, AppError>
where
    F: FnMut(u32, Option<String>) -> (Method, String, Option<Value>),
{
    if limit == 0 {
        return Ok(empty_results());
    }
    let mut after = initial_after;
    let mut first: Option<Value> = None;
    let mut values: Vec<Value> = Vec::new();
    loop {
        let per_page = (limit - values.len() as u32).clamp(1, HUBSPOT_MAX_PER_PAGE);
        let (method, url, body) = build(per_page, after.clone());
        let page = client
            .request("hubspot", operation, ctx.profile(), method, url, body)
            .await
            .map_err(|err| enrich_auth_error(ctx.profile(), operation, capability, err))?;
        if first.is_none() {
            first = Some(page.clone());
        }
        let next_after = page
            .pointer("/paging/next/after")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        let page_values = page
            .get("results")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let count = page_values.len();
        values.extend(
            page_values
                .into_iter()
                .take((limit as usize).saturating_sub(values.len())),
        );
        let cursor_advanced = cursor_advanced(after.as_deref(), next_after.as_deref());
        after = next_after;
        if values.len() >= limit as usize
            || after.is_none()
            || count < per_page as usize
            || !cursor_advanced
        {
            break;
        }
    }
    Ok(aggregate_results(first, values, after))
}

fn cursor_advanced(current: Option<&str>, next: Option<&str>) -> bool {
    next != current
}

fn empty_results() -> Value {
    json!({ "results": [] })
}

fn aggregate_results(first: Option<Value>, values: Vec<Value>, after: Option<String>) -> Value {
    let mut response = first.unwrap_or_else(empty_results);
    if let Some(object) = response.as_object_mut() {
        object.insert("results".to_string(), Value::Array(values));
        match after {
            Some(after) => {
                object.insert("paging".to_string(), json!({ "next": { "after": after } }));
            }
            None => {
                object.remove("paging");
            }
        }
    }
    response
}

fn crm_operation(prefix: &'static str, action: &'static str) -> &'static str {
    match (prefix, action) {
        ("crm.contacts", "list") => "crm.contacts.list",
        ("crm.contacts", "get") => "crm.contacts.get",
        ("crm.contacts", "search") => "crm.contacts.search",
        ("crm.companies", "list") => "crm.companies.list",
        ("crm.companies", "get") => "crm.companies.get",
        ("crm.companies", "search") => "crm.companies.search",
        ("crm.deals", "list") => "crm.deals.list",
        ("crm.deals", "get") => "crm.deals.get",
        ("crm.deals", "search") => "crm.deals.search",
        ("crm.tickets", "list") => "crm.tickets.list",
        ("crm.tickets", "get") => "crm.tickets.get",
        ("crm.tickets", "search") => "crm.tickets.search",
        _ => "crm.objects",
    }
}

fn push_query(path: &mut String, key: &str, value: Option<&str>) {
    let Some(value) = value else {
        return;
    };
    path.push('&');
    path.push_str(&enc(key));
    path.push('=');
    path.push_str(&enc(value));
}

fn crm_capability(object: &str) -> Capability {
    Capability {
        endpoint: "/crm/v3/objects/{object}",
        required_scopes: match object {
            "companies" => &["crm.objects.companies.read"],
            "deals" => &["crm.objects.deals.read"],
            "tickets" => &["crm.objects.tickets.read"],
            _ => CRM_READ_SCOPES,
        },
        caveat: Some("CRM endpoints can also be limited by account tier and object permissions."),
    }
}

fn capability_for_path(path: &str, method: GenericHttpMethod) -> Capability {
    let normalized = path.trim_start_matches('/');
    let is_read = matches!(method, GenericHttpMethod::Get | GenericHttpMethod::Head);
    if normalized.starts_with("files/") {
        Capability {
            endpoint: "/files/*",
            required_scopes: FILES_SCOPES,
            caveat: Some("Most file calls need files; hidden or deleted file reads may also need files.ui_hidden.read."),
        }
    } else if normalized.starts_with("events/") {
        Capability {
            endpoint: "/events/*",
            required_scopes: EVENT_OCCURRENCE_SCOPES,
            caveat: Some("Event occurrence reads need business-intelligence; custom sends need analytics.behavioral_events.send."),
        }
    } else if normalized.starts_with("conversations/v3/custom-channels") {
        Capability {
            endpoint: "/conversations/v3/custom-channels/*",
            required_scopes: if is_read {
                CUSTOM_CHANNEL_READ_SCOPES
            } else {
                CUSTOM_CHANNEL_WRITE_SCOPES
            },
            caveat: Some(
                "HubSpot custom channel endpoints are not supported for legacy private apps.",
            ),
        }
    } else if normalized.starts_with("conversations/") {
        Capability {
            endpoint: "/conversations/*",
            required_scopes: if is_read {
                CONVERSATIONS_READ_SCOPES
            } else {
                CONVERSATIONS_WRITE_SCOPES
            },
            caveat: Some("Read calls need conversations.read; write flows usually also need conversations.write."),
        }
    } else if normalized.starts_with("crm/") {
        Capability {
            endpoint: "/crm/*",
            required_scopes: &[
                "crm.objects.contacts.read",
                "crm.objects.companies.read",
                "crm.objects.deals.read",
                "crm.objects.tickets.read",
            ],
            caveat: Some("Specific CRM object scopes and account permissions vary by endpoint."),
        }
    } else {
        Capability {
            endpoint: "/{path}",
            required_scopes: &[],
            caveat: Some("HubSpot may require endpoint-specific scopes or account tier access."),
        }
    }
}

fn is_custom_channels_path(path: &str) -> bool {
    path.trim_start_matches('/')
        .starts_with("conversations/v3/custom-channels")
}

fn reject_legacy_custom_channels(
    profile: &Profile,
    operation: &'static str,
    capability: Capability,
) -> Result<(), AppError> {
    if matches!(
        profile.auth_type.as_deref(),
        Some("hubspot_legacy_private_app" | "hubspot-legacy-private-app")
    ) {
        return Err(AppError::unsupported_auth(
            "hubspot",
            operation,
            "HubSpot custom channel endpoints are not supported for legacy private app tokens",
            Some(auth_details(profile, capability, Value::Null)),
        ));
    }
    Ok(())
}

fn enrich_auth_error(
    profile: &Profile,
    operation: &'static str,
    capability: Capability,
    err: AppError,
) -> AppError {
    if err.service.as_ref() != "hubspot" || !matches!(err.status, Some(401 | 403)) {
        return err;
    }
    AppError {
        code: err.code,
        message: format!(
            "HubSpot rejected this request; the token may be missing {} scope or this endpoint may not support {}",
            primary_scope(capability),
            auth_type(profile)
        ),
        service: err.service,
        operation: operation.into(),
        status: err.status,
        details: Some(auth_details(
            profile,
            capability,
            err.details.unwrap_or(Value::Null),
        )),
    }
}

fn auth_details(profile: &Profile, capability: Capability, provider: Value) -> Value {
    let remediation = remediation(profile, capability);
    json!({
        "provider": provider,
        "auth_type": auth_type(profile),
        "endpoint": capability.endpoint,
        "required_scopes": capability.required_scopes,
        "remediation": remediation,
    })
}

fn remediation(profile: &Profile, capability: Capability) -> String {
    let mut parts = Vec::new();
    if !capability.required_scopes.is_empty() {
        parts.push(format!(
            "Grant the token these HubSpot scopes if the account tier supports them: {}.",
            capability.required_scopes.join(", ")
        ));
    }
    if matches!(
        profile.auth_type.as_deref(),
        Some("hubspot_service_key" | "hubspot-service-key")
    ) {
        parts.push("Service keys can authenticate HubSpot REST API requests, but not webhooks, UI extensions, or other developer-platform functionality.".to_string());
    }
    if let Some(caveat) = capability.caveat {
        parts.push(caveat.to_string());
    }
    if parts.is_empty() {
        parts.push(
            "Check the token scopes, auth model, and account tier for this endpoint.".to_string(),
        );
    }
    parts.join(" ")
}

fn primary_scope(capability: Capability) -> &'static str {
    capability
        .required_scopes
        .first()
        .copied()
        .unwrap_or("the required")
}

fn auth_type(profile: &Profile) -> &str {
    profile.auth_type.as_deref().unwrap_or("bearer_token")
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;

    fn profile(auth_type: &str) -> Profile {
        Profile {
            auth_type: Some(auth_type.to_string()),
            token: Some("token".to_string()),
            ..Profile::default()
        }
    }

    #[test]
    fn capability_hints_cover_named_hubspot_surfaces() {
        assert_eq!(
            capability_for_path("/files/v3/files", GenericHttpMethod::Get).required_scopes,
            FILES_SCOPES
        );
        assert_eq!(
            capability_for_path("/events/v3/events/contact.view", GenericHttpMethod::Get)
                .required_scopes,
            EVENT_OCCURRENCE_SCOPES
        );
        assert_eq!(
            capability_for_path(
                "/conversations/v3/conversations/threads",
                GenericHttpMethod::Get
            )
            .required_scopes,
            CONVERSATIONS_READ_SCOPES
        );
        assert_eq!(
            capability_for_path("/crm/v3/objects/contacts", GenericHttpMethod::Get).required_scopes,
            &[
                "crm.objects.contacts.read",
                "crm.objects.companies.read",
                "crm.objects.deals.read",
                "crm.objects.tickets.read"
            ]
        );
        assert_eq!(
            capability_for_path("/conversations/v3/custom-channels", GenericHttpMethod::Post)
                .required_scopes,
            CUSTOM_CHANNEL_WRITE_SCOPES
        );
    }

    #[test]
    fn generic_read_capabilities_do_not_require_write_scopes() {
        assert_eq!(
            capability_for_path("/conversations/v3/custom-channels", GenericHttpMethod::Get)
                .required_scopes,
            CUSTOM_CHANNEL_READ_SCOPES
        );
        assert_eq!(
            capability_for_path(
                "/conversations/v3/conversations/threads",
                GenericHttpMethod::Head
            )
            .required_scopes,
            CONVERSATIONS_READ_SCOPES
        );
    }

    #[test]
    fn custom_channel_path_detection_covers_generic_request_paths() {
        assert!(is_custom_channels_path(
            "/conversations/v3/custom-channels/channel-1"
        ));
        assert!(!is_custom_channels_path(
            "/conversations/v3/conversations/threads"
        ));
    }

    #[test]
    fn pagination_cursor_must_advance_before_requesting_another_page() {
        assert!(cursor_advanced(Some("page-1"), Some("page-2")));
        assert!(!cursor_advanced(Some("page-1"), Some("page-1")));
        assert!(cursor_advanced(None, Some("page-1")));
        assert!(cursor_advanced(Some("page-1"), None));
    }

    #[test]
    fn auth_error_preserves_provider_and_adds_scope_remediation() {
        let err = AppError::api(
            "hubspot",
            "files.get",
            StatusCode::FORBIDDEN,
            "provider returned HTTP 403",
            Some(json!({ "status": "error", "message": "missing scopes" })),
        );
        let enriched = enrich_auth_error(
            &profile("hubspot_service_key"),
            "files.get",
            Capability {
                endpoint: "/files/v3/files/{id}",
                required_scopes: HIDDEN_FILES_SCOPES,
                caveat: Some("Hidden or deleted file reads may require files.ui_hidden.read."),
            },
            err,
        );
        assert_eq!(enriched.code, "auth_error");
        assert_eq!(enriched.status, Some(403));
        let details = enriched.details.unwrap();
        assert_eq!(details["auth_type"], "hubspot_service_key");
        assert_eq!(details["endpoint"], "/files/v3/files/{id}");
        assert_eq!(details["required_scopes"][0], "files");
        assert_eq!(details["provider"]["message"], "missing scopes");
        assert!(details["remediation"]
            .as_str()
            .unwrap()
            .contains("files.ui_hidden.read"));
        assert!(enriched.message.contains("missing files scope"));
    }

    #[test]
    fn legacy_private_app_custom_channels_are_blocked_before_request() {
        let capability = Capability {
            endpoint: "/conversations/v3/custom-channels",
            required_scopes: CUSTOM_CHANNEL_READ_SCOPES,
            caveat: Some(
                "HubSpot custom channel endpoints are not supported for legacy private apps.",
            ),
        };
        let err =
            reject_legacy_custom_channels(&profile("hubspot_legacy_private_app"), "x", capability)
                .unwrap_err();
        assert_eq!(err.code, "unsupported_auth");
        assert_eq!(
            err.details.unwrap()["auth_type"],
            "hubspot_legacy_private_app"
        );
    }

    #[test]
    fn aggregate_results_advances_cursor_and_preserves_extra_fields() {
        let first = json!({
            "results": [{"id": 1}, {"id": 2}],
            "paging": { "next": { "after": "page2" } },
            "extra": "kept"
        });
        let aggregated = aggregate_results(
            Some(first),
            vec![json!({"id": 1}), json!({"id": 2}), json!({"id": 3})],
            Some("page3".to_string()),
        );
        assert_eq!(aggregated["results"].as_array().unwrap().len(), 3);
        assert_eq!(aggregated["paging"]["next"]["after"], "page3");
        assert_eq!(aggregated["extra"], "kept");
    }

    #[test]
    fn aggregate_results_drops_paging_when_exhausted() {
        let first = json!({
            "results": [{"id": 1}],
            "paging": { "next": { "after": "page2" } }
        });
        let aggregated = aggregate_results(Some(first), vec![json!({"id": 1})], None);
        assert!(aggregated.get("paging").is_none());
        assert_eq!(aggregated["results"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn crm_capability_reports_dot_separated_tickets_scope() {
        assert_eq!(
            crm_capability("tickets").required_scopes,
            &["crm.objects.tickets.read"]
        );
        assert_eq!(
            crm_capability("contacts").required_scopes,
            &["crm.objects.contacts.read"]
        );
        assert_eq!(
            crm_capability("companies").required_scopes,
            &["crm.objects.companies.read"]
        );
        assert_eq!(
            crm_capability("deals").required_scopes,
            &["crm.objects.deals.read"]
        );
    }
}
