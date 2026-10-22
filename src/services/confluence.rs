use reqwest::Method;
use serde_json::{json, Value};

use crate::{
    cli::*,
    config::Context,
    error::AppError,
    http::ApiClient,
    input,
    services::{
        generic_request,
        shared::{enc, pick, site_url, write_download, CtxProfile},
    },
};

pub(crate) async fn dispatch(
    client: &ApiClient,
    ctx: &Context,
    command: ConfluenceCommand,
) -> Result<Value, AppError> {
    match command.resource {
        ConfluenceResource::Request(args) => {
            let base = site_url(ctx.profile(), "confluence", "request")?;
            generic_request::dispatch(client, ctx, "confluence", base, args).await
        }
        ConfluenceResource::Search(args) => {
            let site = site_url(ctx.profile(), "confluence", "search")?;
            let mut url = format!("{site}/wiki/rest/api/content/search");
            let cql = match args.cql {
                Some(cql) => cql,
                None => {
                    let query = args.query.unwrap_or_default();
                    format!(
                        "text ~ \"{}\"",
                        query.replace('\\', "\\\\").replace('"', "\\\"")
                    )
                }
            };
            append_query(&mut url, "cql", &cql);
            append_query(&mut url, "limit", &page_size(args.limit, 100).to_string());
            collect_cursor_results(client, ctx, "search", url, args.limit, None).await
        }
        ConfluenceResource::Databases(command) => database_dispatch(client, ctx, command).await,
        ConfluenceResource::Spaces(command) => match command.action {
            ConfluenceSpacesAction::List(args) => {
                let url = spaces_list_url(
                    &site_url(ctx.profile(), "confluence", "spaces.list")?,
                    &args,
                );
                collect_cursor_results(
                    client,
                    ctx,
                    "spaces.list",
                    url,
                    args.limit,
                    Some(trim_space),
                )
                .await
            }
            ConfluenceSpacesAction::Get(args) => get_space(client, ctx, &args.id).await,
        },
        ConfluenceResource::Pages(command) => match command.action {
            ConfluencePagesAction::List(args) => {
                let space_id = match args.space.as_deref() {
                    Some(value) => Some(resolve_space_id(client, ctx, "pages.list", value).await?),
                    None => None,
                };
                let url = pages_list_url(
                    &site_url(ctx.profile(), "confluence", "pages.list")?,
                    &args,
                    space_id.as_deref(),
                );
                collect_cursor_results(client, ctx, "pages.list", url, args.limit, Some(trim_page))
                    .await
            }
            ConfluencePagesAction::Get(args) => {
                let url = format!(
                    "{}/wiki/api/v2/pages/{}?body-format=storage",
                    site_url(ctx.profile(), "confluence", "pages.get")?,
                    enc(&args.id)
                );
                client
                    .request(
                        "confluence",
                        "pages.get",
                        ctx.profile(),
                        Method::GET,
                        url,
                        None,
                    )
                    .await
            }
            ConfluencePagesAction::Create(args) => {
                let body = page_create_body(client, ctx, args).await?;
                let url = format!(
                    "{}/wiki/api/v2/pages",
                    site_url(ctx.profile(), "confluence", "pages.create")?
                );
                client
                    .request(
                        "confluence",
                        "pages.create",
                        ctx.profile(),
                        Method::POST,
                        url,
                        Some(body),
                    )
                    .await
            }
            ConfluencePagesAction::Update(args) => {
                let id = args.id.clone();
                let body = page_update_body(client, ctx, args).await?;
                let url = format!(
                    "{}/wiki/api/v2/pages/{}",
                    site_url(ctx.profile(), "confluence", "pages.update")?,
                    enc(&id)
                );
                client
                    .request(
                        "confluence",
                        "pages.update",
                        ctx.profile(),
                        Method::PUT,
                        url,
                        Some(body),
                    )
                    .await
            }
            ConfluencePagesAction::Move(args) => {
                let url = format!(
                    "{}/wiki/rest/api/content/{}/move/{}/{}",
                    site_url(ctx.profile(), "confluence", "pages.move")?,
                    enc(&args.id),
                    args.position.as_str(),
                    enc(&args.target_id)
                );
                client
                    .request(
                        "confluence",
                        "pages.move",
                        ctx.profile(),
                        Method::PUT,
                        url,
                        None,
                    )
                    .await
            }
            ConfluencePagesAction::Delete(args) => {
                let url = format!(
                    "{}/wiki/api/v2/pages/{}",
                    site_url(ctx.profile(), "confluence", "pages.delete")?,
                    enc(&args.id)
                );
                client
                    .request(
                        "confluence",
                        "pages.delete",
                        ctx.profile(),
                        Method::DELETE,
                        url,
                        None,
                    )
                    .await
            }
            ConfluencePagesAction::Comments(command) => match command.action {
                ConfluencePageCommentsAction::List(args) => {
                    list_page_comments(client, ctx, args).await
                }
                ConfluencePageCommentsAction::Create(args) => {
                    create_page_comment(client, ctx, args).await
                }
                ConfluencePageCommentsAction::Get(args) => {
                    comment_request(client, ctx, args, Method::GET).await
                }
                ConfluencePageCommentsAction::Update(args) => {
                    update_comment(client, ctx, args).await
                }
                ConfluencePageCommentsAction::Delete(args) => {
                    comment_request(client, ctx, args, Method::DELETE).await
                }
            },
            ConfluencePagesAction::Attachments(command) => match command.action {
                ConfluencePageAttachmentsAction::List(args) => {
                    list_page_attachments(client, ctx, args).await
                }
                ConfluencePageAttachmentsAction::Download(args) => {
                    download_page_attachment(client, ctx, args).await
                }
                ConfluencePageAttachmentsAction::Upload(args) => {
                    upload_page_attachment(client, ctx, args).await
                }
                ConfluencePageAttachmentsAction::Replace(args) => {
                    replace_page_attachment(client, ctx, args).await
                }
                ConfluencePageAttachmentsAction::Delete(args) => {
                    let url = format!(
                        "{}/wiki/api/v2/attachments/{}",
                        site_url(ctx.profile(), "confluence", "pages.attachments.delete")?,
                        enc(&args.id)
                    );
                    client
                        .request(
                            "confluence",
                            "pages.attachments.delete",
                            ctx.profile(),
                            Method::DELETE,
                            url,
                            None,
                        )
                        .await
                }
            },
            ConfluencePagesAction::Versions(command) => {
                page_versions_dispatch(client, ctx, command).await
            }
            ConfluencePagesAction::Permissions(command) => {
                page_permissions_dispatch(client, ctx, command).await
            }
        },
    }
}

async fn database_dispatch(
    client: &ApiClient,
    ctx: &Context,
    command: ConfluenceDatabasesCommand,
) -> Result<Value, AppError> {
    let site = site_url(ctx.profile(), "confluence", "databases")?;
    match command.action {
        ConfluenceDatabasesAction::Get(args) => {
            let url = format!("{site}/wiki/api/v2/databases/{}", enc(&args.id));
            client
                .request(
                    "confluence",
                    "databases.get",
                    ctx.profile(),
                    Method::GET,
                    url,
                    None,
                )
                .await
        }
        ConfluenceDatabasesAction::Entry(args) => {
            let url = format!(
                "{site}/wiki/api/v2/databases/{}/entries/{}",
                enc(&args.id),
                enc(&args.entry_id)
            );
            client
                .request(
                    "confluence",
                    "databases.entry",
                    ctx.profile(),
                    Method::GET,
                    url,
                    None,
                )
                .await
        }
        ConfluenceDatabasesAction::Fields(args) => {
            database_list(client, ctx, &site, &args.id, "fields", args.limit).await
        }
        ConfluenceDatabasesAction::Views(args) => {
            database_list(client, ctx, &site, &args.id, "views", args.limit).await
        }
        ConfluenceDatabasesAction::Entries(args) => {
            let mut url = format!("{site}/wiki/api/v2/databases/{}/entries", enc(&args.id));
            append_query(&mut url, "limit", &page_size(args.limit, 100).to_string());
            if let Some(sort) = args.sort {
                append_query(&mut url, "sort", &sort);
            }
            for filter in args.filters {
                append_database_filter(&mut url, &filter)?;
            }
            collect_cursor_results(client, ctx, "databases.entries", url, args.limit, None).await
        }
    }
}

fn append_database_filter(url: &mut String, filter: &str) -> Result<(), AppError> {
    let invalid = || {
        AppError::invalid_input(
            "confluence",
            "databases.entries",
            "--filter must use fieldId[operator]=value",
        )
    };
    let (name, value) = filter.split_once('=').ok_or_else(invalid)?;
    let (field, operator) = name
        .split_once('[')
        .and_then(|(field, op)| op.strip_suffix(']').map(|op| (field, op)))
        .filter(|(field, op)| {
            !field.is_empty() && !op.is_empty() && !field.contains(']') && !op.contains('[')
        })
        .ok_or_else(invalid)?;
    append_query(url, &format!("filter[{field}][{operator}]"), value);
    Ok(())
}

async fn database_list(
    client: &ApiClient,
    ctx: &Context,
    site: &str,
    id: &str,
    segment: &'static str,
    limit: u32,
) -> Result<Value, AppError> {
    let mut url = format!("{site}/wiki/api/v2/databases/{}/{segment}", enc(id));
    append_query(&mut url, "limit", &page_size(limit, 100).to_string());
    collect_cursor_results(client, ctx, "databases.list", url, limit, None).await
}

async fn comment_request(
    client: &ApiClient,
    ctx: &Context,
    args: ConfluenceCommentGet,
    method: Method,
) -> Result<Value, AppError> {
    let url = format!(
        "{}/wiki/api/v2/{}/{}",
        site_url(ctx.profile(), "confluence", "comments")?,
        args.kind.segment(),
        enc(&args.id)
    );
    client
        .request("confluence", "comments", ctx.profile(), method, url, None)
        .await
}

async fn update_comment(
    client: &ApiClient,
    ctx: &Context,
    args: ConfluenceCommentUpdate,
) -> Result<Value, AppError> {
    let url = format!(
        "{}/wiki/api/v2/{}/{}",
        site_url(ctx.profile(), "confluence", "comments.update")?,
        args.kind.segment(),
        enc(&args.id)
    );
    let mut body = input::read_json_arg("confluence", "comments.update", args.json.as_deref())?;
    if let Some(text) = args.body {
        input::ensure_object(&mut body).insert(
            "body".into(),
            json!({"representation":"storage","value":text}),
        );
    }
    if body.get("body").is_none() && body.get("resolved").is_none() {
        return Err(AppError::invalid_input(
            "confluence",
            "comments.update",
            "--body or --json with body/resolved is required",
        ));
    }
    let version = match args.version {
        Some(v) => v,
        None => {
            let current = client
                .request(
                    "confluence",
                    "comments.update",
                    ctx.profile(),
                    Method::GET,
                    url.clone(),
                    None,
                )
                .await?;
            current
                .pointer("/version/number")
                .and_then(Value::as_u64)
                .ok_or_else(|| {
                    AppError::internal(
                        "confluence",
                        "comments.update",
                        "comment response missing version.number",
                    )
                })?
                + 1
        }
    };
    input::ensure_object(&mut body).insert("version".into(), json!({"number":version}));
    client
        .request(
            "confluence",
            "comments.update",
            ctx.profile(),
            Method::PUT,
            url,
            Some(body),
        )
        .await
}

async fn page_versions_dispatch(
    client: &ApiClient,
    ctx: &Context,
    command: ConfluencePageVersionsCommand,
) -> Result<Value, AppError> {
    let site = site_url(ctx.profile(), "confluence", "pages.versions")?;
    match command.action {
        ConfluencePageVersionsAction::List(args) => {
            let mut url = format!("{site}/wiki/api/v2/pages/{}/versions", enc(&args.id));
            append_query(&mut url, "limit", &page_size(args.limit, 100).to_string());
            collect_cursor_results(client, ctx, "pages.versions", url, args.limit, None).await
        }
        ConfluencePageVersionsAction::Get(args) => {
            let url = format!(
                "{site}/wiki/api/v2/pages/{}/versions/{}",
                enc(&args.id),
                args.version
            );
            client
                .request(
                    "confluence",
                    "pages.versions",
                    ctx.profile(),
                    Method::GET,
                    url,
                    None,
                )
                .await
        }
    }
}

async fn page_permissions_dispatch(
    client: &ApiClient,
    ctx: &Context,
    command: ConfluencePagePermissionsCommand,
) -> Result<Value, AppError> {
    let site = site_url(ctx.profile(), "confluence", "pages.permissions")?;
    match command.action {
        ConfluencePagePermissionsAction::List(args) => {
            let mut url = format!("{site}/wiki/api/v2/pages/{}/permissions", enc(&args.id));
            append_query(&mut url, "limit", &page_size(args.limit, 250).to_string());
            collect_cursor_results(client, ctx, "pages.permissions.list", url, args.limit, None)
                .await
        }
        ConfluencePagePermissionsAction::RestrictionState(args) => {
            let url = format!(
                "{site}/wiki/api/v2/pages/{}/restriction-state",
                enc(&args.id)
            );
            client
                .request(
                    "confluence",
                    "pages.permissions.restriction-state",
                    ctx.profile(),
                    Method::GET,
                    url,
                    None,
                )
                .await
        }
        ConfluencePagePermissionsAction::SetRestrictionState(args) => {
            permission_write(client, ctx, &site, args, "restriction-state", Method::PUT).await
        }
        ConfluencePagePermissionsAction::Add(args) => {
            permission_write(client, ctx, &site, args, "permissions", Method::POST).await
        }
        ConfluencePagePermissionsAction::Set(args) => {
            permission_write(client, ctx, &site, args, "permissions", Method::PUT).await
        }
        ConfluencePagePermissionsAction::Delete(args) => {
            let mut url = format!("{site}/wiki/api/v2/pages/{}/permissions", enc(&args.id));
            let has_pair = args.principal_type.is_some() && args.principal_id.is_some();
            if (args.all && (has_pair || args.operation.is_some()))
                || (!args.all && !has_pair)
                || (args.principal_type.is_some() != args.principal_id.is_some())
            {
                return Err(AppError::invalid_input(
                    "confluence",
                    "pages.permissions.delete",
                    "provide --principal-type and --principal-id, optionally --operation; or --all",
                ));
            }
            if let Some(kind) = args.principal_type {
                append_query(&mut url, "principal-type", &kind);
            }
            if let Some(id) = args.principal_id {
                append_query(&mut url, "principal-id", &id);
            }
            if let Some(op) = args.operation {
                append_query(&mut url, "operation", &op);
            }
            client
                .request(
                    "confluence",
                    "pages.permissions.delete",
                    ctx.profile(),
                    Method::DELETE,
                    url,
                    None,
                )
                .await
        }
    }
}

async fn permission_write(
    client: &ApiClient,
    ctx: &Context,
    site: &str,
    args: ConfluenceJsonWrite,
    segment: &str,
    method: Method,
) -> Result<Value, AppError> {
    let url = format!("{site}/wiki/api/v2/pages/{}/{segment}", enc(&args.id));
    let body = input::read_json_arg("confluence", "pages.permissions", Some(&args.json))?;
    client
        .request(
            "confluence",
            "pages.permissions",
            ctx.profile(),
            method,
            url,
            Some(body),
        )
        .await
}

async fn replace_page_attachment(
    client: &ApiClient,
    ctx: &Context,
    args: ConfluencePageAttachmentsReplace,
) -> Result<Value, AppError> {
    let base = site_url(ctx.profile(), "confluence", "pages.attachments.replace")?;
    let url = format!(
        "{base}/wiki/rest/api/content/{}/child/attachment/{}/data",
        enc(&args.page_id),
        enc(&args.attachment_id)
    );
    client
        .upload(
            "confluence",
            "pages.attachments.replace",
            ctx.profile(),
            url,
            &args.file,
            args.comment.as_deref(),
        )
        .await
}

async fn collect_cursor_results(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    first_url: String,
    limit: u32,
    trim: Option<fn(&Value) -> Value>,
) -> Result<Value, AppError> {
    if limit == 0 {
        return Ok(json!({ "results": [], "limit": 0, "size": 0 }));
    }

    let (first_page, mut results) =
        fetch_cursor_pages(client, ctx, operation, first_url, limit).await?;

    if let Some(trim_fn) = trim {
        for item in results.iter_mut() {
            *item = trim_fn(item);
        }
    }

    let mut response = first_page.unwrap_or_else(|| json!({}));
    let object = input::ensure_object(&mut response);
    let size = results.len();
    object.insert("results".to_string(), Value::Array(results));
    object.insert("limit".to_string(), json!(limit));
    object.insert("size".to_string(), json!(size));
    object.remove("_links");
    Ok(response)
}

async fn fetch_cursor_pages(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    first_url: String,
    limit: u32,
) -> Result<(Option<Value>, Vec<Value>), AppError> {
    let site = site_url(ctx.profile(), "confluence", operation)?;
    let mut url = first_url;
    let mut first_page = None;
    let mut results: Vec<Value> = Vec::new();

    loop {
        let page = client
            .request(
                "confluence",
                operation,
                ctx.profile(),
                Method::GET,
                url,
                None,
            )
            .await?;
        if first_page.is_none() {
            first_page = Some(page.clone());
        }

        let page_results = page
            .get("results")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if page_results.is_empty() {
            break;
        }
        for result in page_results {
            if results.len() >= limit as usize {
                break;
            }
            results.push(result);
        }

        if results.len() >= limit as usize {
            break;
        }
        let Some(next) = page
            .pointer("/_links/next")
            .and_then(Value::as_str)
            .filter(|next| !next.is_empty())
        else {
            break;
        };
        url = confluence_link_url(&site, next);
    }

    Ok((first_page, results))
}

fn append_query(url: &mut String, key: &str, value: &str) {
    let separator = if url.contains('?') { '&' } else { '?' };
    url.push(separator);
    url.push_str(key);
    url.push('=');
    url.push_str(&enc(value));
}

fn spaces_list_url(site: &str, args: &ConfluenceSpacesList) -> String {
    let mut url = format!("{site}/wiki/api/v2/spaces");
    append_query(&mut url, "limit", &page_size(args.limit, 250).to_string());
    if let Some(t) = args.space_type.as_deref() {
        append_query(&mut url, "type", t);
    }
    if let Some(s) = args.status.as_deref() {
        append_query(&mut url, "status", s);
    }
    if let Some(keys) = args.key.as_deref() {
        for key in keys.split(',').map(str::trim).filter(|k| !k.is_empty()) {
            append_query(&mut url, "keys", key);
        }
    }
    url
}

fn pages_list_url(site: &str, args: &ConfluencePagesList, space_id: Option<&str>) -> String {
    let mut url = format!("{site}/wiki/api/v2/pages");
    append_query(&mut url, "limit", &page_size(args.limit, 250).to_string());
    if let Some(id) = space_id {
        append_query(&mut url, "space-id", id);
    }
    if let Some(statuses) = args.status.as_deref() {
        for status in statuses.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            append_query(&mut url, "status", status);
        }
    }
    if let Some(parent) = args.parent.as_deref() {
        append_query(&mut url, "parent-id", parent);
    }
    if let Some(title) = args.title.as_deref() {
        append_query(&mut url, "title", title);
    }
    url
}

fn trim_space(src: &Value) -> Value {
    pick(src, &["id", "key", "name", "type", "status", "homepageId"])
}

fn trim_page(src: &Value) -> Value {
    let mut out = serde_json::Map::new();
    let Some(obj) = src.as_object() else {
        return src.clone();
    };
    for k in [
        "id",
        "title",
        "status",
        "spaceId",
        "parentId",
        "parentType",
        "authorId",
        "createdAt",
    ] {
        if let Some(v) = obj.get(k) {
            out.insert(k.to_string(), v.clone());
        }
    }
    if let Some(version) = obj.get("version") {
        out.insert(
            "version".to_string(),
            pick(version, &["number", "createdAt"]),
        );
    }
    Value::Object(out)
}

const MAX_COMMENT_DEPTH: u32 = 10;
const MAX_CHILDREN_PER_PARENT: u32 = 100;

#[derive(Clone, Copy)]
enum CommentKind {
    Footer,
    Inline,
}

impl CommentKind {
    fn segment(self) -> &'static str {
        match self {
            Self::Footer => "footer-comments",
            Self::Inline => "inline-comments",
        }
    }
}

async fn list_page_comments(
    client: &ApiClient,
    ctx: &Context,
    args: ConfluencePageCommentsList,
) -> Result<Value, AppError> {
    let footer =
        collect_comment_thread(client, ctx, CommentKind::Footer, &args.page_id, args.limit).await?;
    let inline =
        collect_comment_thread(client, ctx, CommentKind::Inline, &args.page_id, args.limit).await?;
    Ok(json!({
        "page_comments": footer,
        "inline_comments": inline,
    }))
}

async fn collect_comment_thread(
    client: &ApiClient,
    ctx: &Context,
    kind: CommentKind,
    page_id: &str,
    limit: u32,
) -> Result<Vec<Value>, AppError> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let site = site_url(ctx.profile(), "confluence", "pages.comments.list")?;
    let mut url = format!(
        "{site}/wiki/api/v2/pages/{}/{}?body-format=storage",
        enc(page_id),
        kind.segment(),
    );
    append_query(&mut url, "limit", &page_size(limit, 250).to_string());

    let (_, top_level) = fetch_cursor_pages(client, ctx, "pages.comments.list", url, limit).await?;

    let mut out = Vec::with_capacity(top_level.len());
    for comment in top_level {
        let id = comment.get("id").and_then(Value::as_str).unwrap_or("");
        let replies = if id.is_empty() {
            Vec::new()
        } else {
            walk_replies(client, ctx, kind, id, 1).await?
        };
        out.push(trim_comment(&comment, replies));
    }
    Ok(out)
}

fn walk_replies<'a>(
    client: &'a ApiClient,
    ctx: &'a Context,
    kind: CommentKind,
    parent_id: &'a str,
    depth: u32,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<Value>, AppError>> + Send + 'a>>
{
    Box::pin(async move {
        if depth > MAX_COMMENT_DEPTH {
            return Ok(vec![json!({ "truncated": true, "reason": "max-depth" })]);
        }
        let site = site_url(ctx.profile(), "confluence", "pages.comments.list")?;
        let mut url = format!(
            "{site}/wiki/api/v2/{}/{}/children?body-format=storage",
            kind.segment(),
            enc(parent_id),
        );
        append_query(
            &mut url,
            "limit",
            &page_size(MAX_CHILDREN_PER_PARENT, 250).to_string(),
        );

        let (_, children) = fetch_cursor_pages(
            client,
            ctx,
            "pages.comments.list",
            url,
            MAX_CHILDREN_PER_PARENT,
        )
        .await?;

        let mut out = Vec::with_capacity(children.len());
        for child in children {
            let id = child.get("id").and_then(Value::as_str).unwrap_or("");
            let nested = if id.is_empty() {
                Vec::new()
            } else {
                walk_replies(client, ctx, kind, id, depth + 1).await?
            };
            out.push(trim_comment(&child, nested));
        }
        Ok(out)
    })
}

fn trim_comment(src: &Value, replies: Vec<Value>) -> Value {
    let mut out = serde_json::Map::new();
    let Some(obj) = src.as_object() else {
        return src.clone();
    };
    for k in [
        "id",
        "title",
        "status",
        "parentCommentId",
        "createdAt",
        "resolutionStatus",
    ] {
        if let Some(v) = obj.get(k) {
            out.insert(k.to_string(), v.clone());
        }
    }
    if let Some(version) = obj.get("version") {
        out.insert(
            "version".to_string(),
            pick(version, &["number", "createdAt", "authorId"]),
        );
    }
    if let Some(body) = obj.get("body") {
        out.insert("body".to_string(), body.clone());
    }
    if let Some(props) = obj.get("properties") {
        out.insert(
            "properties".to_string(),
            pick(props, &["inline-original-selection"]),
        );
    }
    out.insert("replies".to_string(), Value::Array(replies));
    Value::Object(out)
}

async fn create_page_comment(
    client: &ApiClient,
    ctx: &Context,
    args: ConfluencePageCommentsCreate,
) -> Result<Value, AppError> {
    if args.body.is_none() && args.json.is_none() {
        return Err(AppError::invalid_input(
            "confluence",
            "pages.comments.create",
            "--body or --json is required",
        ));
    }

    let mut payload =
        input::read_json_arg("confluence", "pages.comments.create", args.json.as_deref())?;
    let kind = match args.reply_to.as_deref() {
        Some(parent_id) => detect_comment_kind(client, ctx, parent_id).await?,
        None if args.inline_marker_ref.is_some()
            || payload.get("inlineCommentProperties").is_some() =>
        {
            CommentKind::Inline
        }
        None => CommentKind::Footer,
    };
    let map = input::ensure_object(&mut payload);
    if let Some(text) = args.body {
        map.insert(
            "body".to_string(),
            json!({ "representation": "storage", "value": text }),
        );
    }
    if let Some(marker) = args.inline_marker_ref {
        input::ensure_object(
            map.entry("inlineCommentProperties")
                .or_insert_with(|| json!({})),
        )
        .insert("inlineMarkerRef".to_string(), Value::String(marker));
    }
    match args.reply_to {
        Some(pid) => {
            map.insert("parentCommentId".to_string(), Value::String(pid));
        }
        None => {
            map.insert("pageId".to_string(), Value::String(args.page_id.clone()));
        }
    }

    let url = format!(
        "{}/wiki/api/v2/{}",
        site_url(ctx.profile(), "confluence", "pages.comments.create")?,
        kind.segment(),
    );
    client
        .request(
            "confluence",
            "pages.comments.create",
            ctx.profile(),
            Method::POST,
            url,
            Some(payload),
        )
        .await
}

async fn detect_comment_kind(
    client: &ApiClient,
    ctx: &Context,
    comment_id: &str,
) -> Result<CommentKind, AppError> {
    if probe_comment(client, ctx, CommentKind::Footer, comment_id).await? {
        return Ok(CommentKind::Footer);
    }
    if probe_comment(client, ctx, CommentKind::Inline, comment_id).await? {
        return Ok(CommentKind::Inline);
    }
    Err(AppError::not_found(
        "confluence",
        "pages.comments.create",
        format!("comment {comment_id} was not found as footer or inline"),
    ))
}

async fn probe_comment(
    client: &ApiClient,
    ctx: &Context,
    kind: CommentKind,
    comment_id: &str,
) -> Result<bool, AppError> {
    let url = format!(
        "{}/wiki/api/v2/{}/{}",
        site_url(ctx.profile(), "confluence", "pages.comments.create")?,
        kind.segment(),
        enc(comment_id),
    );
    match client
        .request(
            "confluence",
            "pages.comments.create",
            ctx.profile(),
            Method::GET,
            url,
            None,
        )
        .await
    {
        Ok(_) => Ok(true),
        Err(err) if err.status == Some(404) => Ok(false),
        Err(err) => Err(err),
    }
}

fn confluence_link_url(site: &str, link: &str) -> String {
    if link.starts_with("http://") || link.starts_with("https://") {
        link.to_string()
    } else if link.starts_with("/wiki/") {
        format!("{site}{link}")
    } else if link.starts_with('/') {
        format!("{site}/wiki{link}")
    } else {
        format!("{site}/wiki/{link}")
    }
}

fn page_size(limit: u32, max: u32) -> u32 {
    limit.clamp(1, max)
}

async fn get_space(client: &ApiClient, ctx: &Context, id_or_key: &str) -> Result<Value, AppError> {
    if is_numeric_id(id_or_key) {
        let url = format!(
            "{}/wiki/api/v2/spaces/{}",
            site_url(ctx.profile(), "confluence", "spaces.get")?,
            enc(id_or_key)
        );
        return client
            .request(
                "confluence",
                "spaces.get",
                ctx.profile(),
                Method::GET,
                url,
                None,
            )
            .await;
    }
    space_by_key(client, ctx, "spaces.get", id_or_key).await
}

async fn page_create_body(
    client: &ApiClient,
    ctx: &Context,
    args: ConfluencePageCreate,
) -> Result<Value, AppError> {
    let mut body = input::read_json_arg("confluence", "pages.create", args.json.as_deref())?;
    let space_ref = args.space_key.as_deref().or(args.space_id.as_deref());
    if let Some(space_ref) = space_ref {
        let space_id = resolve_space_id(client, ctx, "pages.create", space_ref).await?;
        input::ensure_object(&mut body).insert("spaceId".to_string(), Value::String(space_id));
    }
    input::set_string(&mut body, "title", &args.title);
    input::set_string(&mut body, "parentId", &args.parent_id);
    if let Some(body_text) = args.body {
        input::ensure_object(&mut body).insert(
            "body".to_string(),
            serde_json::json!({ "representation": "storage", "value": body_text }),
        );
    }
    Ok(body)
}

async fn resolve_space_id(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    id_or_key: &str,
) -> Result<String, AppError> {
    if is_numeric_id(id_or_key) {
        return Ok(id_or_key.to_string());
    }
    let space = space_by_key(client, ctx, operation, id_or_key).await?;
    space
        .get("id")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| AppError::internal("confluence", operation, "space response missing id"))
}

async fn space_by_key(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    key: &str,
) -> Result<Value, AppError> {
    let url = format!(
        "{}/wiki/api/v2/spaces?keys={}&limit=1",
        site_url(ctx.profile(), "confluence", operation)?,
        enc(key)
    );
    let response = client
        .request(
            "confluence",
            operation,
            ctx.profile(),
            Method::GET,
            url,
            None,
        )
        .await?;
    response
        .get("results")
        .and_then(Value::as_array)
        .and_then(|spaces| {
            spaces
                .iter()
                .find(|space| space.get("key").and_then(Value::as_str) == Some(key))
                .or_else(|| spaces.first())
        })
        .cloned()
        .ok_or_else(|| {
            AppError::not_found(
                "confluence",
                operation,
                format!("space key {key} was not found"),
            )
        })
}

fn is_numeric_id(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|ch| ch.is_ascii_digit())
}

async fn page_update_body(
    client: &ApiClient,
    ctx: &Context,
    args: ConfluencePageUpdate,
) -> Result<Value, AppError> {
    // Fetch current page to get version.number and status — both required by the v2 PUT endpoint.
    let base = site_url(ctx.profile(), "confluence", "pages.update")?;
    let current = client
        .request(
            "confluence",
            "pages.update",
            ctx.profile(),
            Method::GET,
            format!("{base}/wiki/api/v2/pages/{}", enc(&args.id)),
            None,
        )
        .await?;
    let current_version = current
        .get("version")
        .and_then(|v| v.get("number"))
        .and_then(Value::as_u64)
        .unwrap_or(1);
    let current_status = current
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("current")
        .to_string();
    let current_title = current
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    let mut body = input::read_json_arg("confluence", "pages.update", args.json.as_deref())?;
    let obj = input::ensure_object(&mut body);
    obj.insert("id".to_string(), Value::String(args.id.clone()));
    obj.insert("status".to_string(), Value::String(current_status));
    obj.entry("title".to_string())
        .or_insert_with(|| Value::String(current_title));
    let next_version = args.version.unwrap_or(current_version + 1);
    obj.insert("version".to_string(), json!({ "number": next_version }));
    if let Some(title) = args.title {
        obj.insert("title".to_string(), Value::String(title));
    }
    if let Some(body_text) = args.body {
        obj.insert(
            "body".to_string(),
            json!({ "representation": "storage", "value": body_text }),
        );
    }
    Ok(body)
}

async fn list_page_attachments(
    client: &ApiClient,
    ctx: &Context,
    args: ConfluencePageAttachmentsList,
) -> Result<Value, AppError> {
    let base = site_url(ctx.profile(), "confluence", "pages.attachments.list")?;
    let mut url = format!(
        "{base}/wiki/api/v2/pages/{}/attachments",
        enc(&args.page_id)
    );
    append_query(&mut url, "limit", &args.limit.to_string());
    collect_cursor_results(
        client,
        ctx,
        "pages.attachments.list",
        url,
        args.limit,
        Some(trim_attachment),
    )
    .await
}

async fn download_page_attachment(
    client: &ApiClient,
    ctx: &Context,
    args: ConfluencePageAttachmentsDownload,
) -> Result<Value, AppError> {
    let base = site_url(ctx.profile(), "confluence", "pages.attachments.download")?;
    // v1 REST download endpoint — the /download/attachments/ URLs reject API token auth.
    // This endpoint returns a 302 to a presigned binary URL; reqwest strips auth headers
    // on cross-origin redirects, which is correct behaviour for presigned CDN URLs.
    let dl_url = format!(
        "{base}/wiki/rest/api/content/{}/child/attachment/{}/download",
        enc(&args.page_id),
        enc(&args.attachment_id)
    );
    let bytes = client
        .download(
            "confluence",
            "pages.attachments.download",
            ctx.profile(),
            dl_url,
        )
        .await?;
    write_download(
        "confluence",
        "pages.attachments.download",
        &args.output,
        &bytes,
    )
}

async fn upload_page_attachment(
    client: &ApiClient,
    ctx: &Context,
    args: ConfluencePageAttachmentsUpload,
) -> Result<Value, AppError> {
    let base = site_url(ctx.profile(), "confluence", "pages.attachments.upload")?;
    let url = format!(
        "{base}/wiki/rest/api/content/{}/child/attachment",
        enc(&args.page_id)
    );
    let result = client
        .upload(
            "confluence",
            "pages.attachments.upload",
            ctx.profile(),
            url,
            &args.file,
            args.comment.as_deref(),
        )
        .await?;
    // v1 wraps the created attachment in a results array
    if let Some(arr) = result.get("results").and_then(Value::as_array) {
        Ok(arr.first().cloned().unwrap_or(result))
    } else {
        Ok(result)
    }
}

fn trim_attachment(src: &Value) -> Value {
    let mut out = pick(
        src,
        &[
            "id",
            "title",
            "mediaType",
            "fileSize",
            "comment",
            "createdAt",
            "downloadLink",
        ],
    );
    if let Some(v) = src.get("version") {
        out.as_object_mut().unwrap().insert(
            "version".to_string(),
            pick(v, &["number", "authorId", "createdAt"]),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_filter_encodes_provider_bracket_query() {
        let mut url = "https://example.atlassian.net/wiki/api/v2/databases/1/entries".to_string();
        append_database_filter(&mut url, "field-1[eq]=Ready & waiting").unwrap();
        assert_eq!(url, "https://example.atlassian.net/wiki/api/v2/databases/1/entries?filter[field-1][eq]=Ready%20%26%20waiting");
        assert!(append_database_filter(&mut url, "field-1=Ready").is_err());
    }

    fn spaces_args(
        space_type: Option<&str>,
        status: Option<&str>,
        key: Option<&str>,
    ) -> ConfluenceSpacesList {
        ConfluenceSpacesList {
            space_type: space_type.map(str::to_string),
            status: status.map(str::to_string),
            key: key.map(str::to_string),
            limit: 50,
        }
    }

    fn pages_args(
        status: Option<&str>,
        parent: Option<&str>,
        title: Option<&str>,
    ) -> ConfluencePagesList {
        ConfluencePagesList {
            space: None,
            status: status.map(str::to_string),
            parent: parent.map(str::to_string),
            title: title.map(str::to_string),
            limit: 50,
        }
    }

    #[test]
    fn trim_space_keeps_allowlist_drops_links() {
        let src = json!({
            "id": "98306",
            "key": "DOCS",
            "name": "Docs",
            "type": "global",
            "status": "current",
            "homepageId": "131073",
            "description": { "plain": { "value": "ignore me" } },
            "icon": { "path": "/some/icon.png" },
            "authorId": "712020:abc",
            "createdAt": "2026-01-01T00:00:00Z",
            "_links": { "webui": "/spaces/DOCS" }
        });
        let trimmed = trim_space(&src);
        let obj = trimmed.as_object().unwrap();
        assert_eq!(obj.get("id").unwrap(), "98306");
        assert_eq!(obj.get("key").unwrap(), "DOCS");
        assert_eq!(obj.get("name").unwrap(), "Docs");
        assert_eq!(obj.get("type").unwrap(), "global");
        assert_eq!(obj.get("status").unwrap(), "current");
        assert_eq!(obj.get("homepageId").unwrap(), "131073");
        for noise in ["description", "icon", "authorId", "createdAt", "_links"] {
            assert!(!obj.contains_key(noise), "should drop {noise}");
        }
    }

    #[test]
    fn trim_space_handles_missing_fields() {
        let src = json!({ "id": "1", "key": "X", "name": "X-space", "type": "personal", "status": "current" });
        let trimmed = trim_space(&src);
        let obj = trimmed.as_object().unwrap();
        assert!(!obj.contains_key("homepageId"));
        assert_eq!(obj.get("id").unwrap(), "1");
    }

    #[test]
    fn trim_page_strips_body_and_links() {
        let src = json!({
            "id": "131073",
            "title": "Hello",
            "status": "current",
            "spaceId": "98306",
            "parentId": null,
            "parentType": "page",
            "position": 12,
            "authorId": "712020:abc",
            "ownerId": "712020:abc",
            "lastOwnerId": "712020:abc",
            "createdAt": "2026-05-13T17:37:03+0000",
            "version": {
                "number": 3,
                "createdAt": "2026-05-19T14:57:46+0000",
                "message": "minor tweak",
                "minorEdit": true,
                "authorId": "712020:abc"
            },
            "body": { "storage": { "value": "<p>huge body</p>", "representation": "storage" } },
            "_links": { "editui": "/pages/edit-v2/131073" }
        });
        let trimmed = trim_page(&src);
        let obj = trimmed.as_object().unwrap();
        assert_eq!(obj.get("id").unwrap(), "131073");
        assert_eq!(obj.get("title").unwrap(), "Hello");
        assert_eq!(obj.get("spaceId").unwrap(), "98306");
        for noise in ["position", "ownerId", "lastOwnerId", "body", "_links"] {
            assert!(!obj.contains_key(noise), "should drop {noise}");
        }
    }

    #[test]
    fn trim_page_trims_version_object() {
        let src = json!({
            "id": "1",
            "version": { "number": 3, "createdAt": "x", "message": "drop", "minorEdit": false, "authorId": "drop" }
        });
        let trimmed = trim_page(&src);
        let version = trimmed.get("version").unwrap().as_object().unwrap();
        assert_eq!(version.get("number").unwrap(), 3);
        assert_eq!(version.get("createdAt").unwrap(), "x");
        for noise in ["message", "minorEdit", "authorId"] {
            assert!(!version.contains_key(noise), "should drop version.{noise}");
        }
    }

    #[test]
    fn pages_list_url_combines_filters() {
        let args = pages_args(
            Some("current,archived"),
            Some("123"),
            Some("Quarterly Plan"),
        );
        let url = pages_list_url("https://example.atlassian.net", &args, Some("98306"));
        assert!(url.starts_with("https://example.atlassian.net/wiki/api/v2/pages?limit=50"));
        assert!(url.contains("&space-id=98306"));
        assert!(url.contains("&status=current"));
        assert!(url.contains("&status=archived"));
        assert!(url.contains("&parent-id=123"));
        assert!(url.contains("&title=Quarterly%20Plan"));
    }

    #[test]
    fn pages_list_url_omits_unset_filters() {
        let args = pages_args(None, None, None);
        let url = pages_list_url("https://example.atlassian.net", &args, None);
        assert_eq!(
            url,
            "https://example.atlassian.net/wiki/api/v2/pages?limit=50"
        );
    }

    #[test]
    fn spaces_list_url_combines_filters() {
        let args = spaces_args(Some("global"), Some("current"), Some("DOCS,ENG"));
        let url = spaces_list_url("https://example.atlassian.net", &args);
        assert!(url.starts_with("https://example.atlassian.net/wiki/api/v2/spaces?limit=50"));
        assert!(url.contains("&type=global"));
        assert!(url.contains("&status=current"));
        assert!(url.contains("&keys=DOCS"));
        assert!(url.contains("&keys=ENG"));
    }

    #[test]
    fn trim_comment_keeps_body_drops_links() {
        let src = json!({
            "id": "456",
            "title": "Re: Some page",
            "status": "current",
            "pageId": "98422",
            "parentCommentId": "123",
            "createdAt": "2026-05-19T10:00:00Z",
            "version": {
                "number": 1,
                "createdAt": "2026-05-19T10:00:00Z",
                "message": "drop me",
                "minorEdit": false,
                "authorId": "712020:abc",
            },
            "body": { "storage": { "value": "<p>hi</p>", "representation": "storage" } },
            "_links": { "webui": "/comment/456" }
        });
        let trimmed = trim_comment(&src, vec![]);
        let obj = trimmed.as_object().unwrap();
        assert_eq!(obj.get("id").unwrap(), "456");
        assert_eq!(obj.get("parentCommentId").unwrap(), "123");
        let body = obj.get("body").unwrap();
        assert_eq!(body.pointer("/storage/value").unwrap(), "<p>hi</p>");
        let version = obj.get("version").unwrap().as_object().unwrap();
        assert_eq!(version.get("number").unwrap(), 1);
        assert_eq!(version.get("authorId").unwrap(), "712020:abc");
        assert!(!version.contains_key("message"));
        assert!(!version.contains_key("minorEdit"));
        for noise in ["_links", "pageId"] {
            assert!(!obj.contains_key(noise), "should drop {noise}");
        }
        assert!(obj.contains_key("replies"));
        assert!(obj.get("replies").unwrap().as_array().unwrap().is_empty());
    }

    #[test]
    fn trim_comment_inline_keeps_selection() {
        let src = json!({
            "id": "789",
            "resolutionStatus": "open",
            "body": { "storage": { "value": "<p>nit</p>", "representation": "storage" } },
            "properties": {
                "inline-marker-ref": "abc-hash",
                "inline-original-selection": "the highlighted text"
            }
        });
        let trimmed = trim_comment(&src, vec![]);
        let obj = trimmed.as_object().unwrap();
        assert_eq!(obj.get("resolutionStatus").unwrap(), "open");
        let props = obj.get("properties").unwrap().as_object().unwrap();
        assert_eq!(
            props.get("inline-original-selection").unwrap(),
            "the highlighted text"
        );
        assert!(!props.contains_key("inline-marker-ref"));
    }

    #[test]
    fn trim_comment_nests_replies() {
        let src = json!({ "id": "1", "body": { "storage": { "value": "p" } } });
        let reply = json!({ "id": "2", "truncated": true });
        let trimmed = trim_comment(&src, vec![reply.clone()]);
        let replies = trimmed.get("replies").unwrap().as_array().unwrap();
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0], reply);
    }
}
