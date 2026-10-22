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
    command: JiraCommand,
) -> Result<Value, AppError> {
    match command.resource {
        JiraResource::Request(args) => {
            let base = site_url(ctx.profile(), "jira", "request")?;
            generic_request::dispatch(client, ctx, "jira", base, args).await
        }
        JiraResource::Issues(command) => match command.action {
            JiraIssuesAction::List(args) => {
                let jql = build_jql(&args);
                search_issues(
                    client,
                    ctx,
                    "issues.list",
                    jql.as_deref(),
                    args.fields.as_deref(),
                    args.limit,
                )
                .await
            }
            JiraIssuesAction::Get(args) => {
                let url = format!(
                    "{}/rest/api/3/issue/{}",
                    site_url(ctx.profile(), "jira", "issues.get")?,
                    enc(&args.id)
                );
                client
                    .request("jira", "issues.get", ctx.profile(), Method::GET, url, None)
                    .await
            }
            JiraIssuesAction::Create(args) => {
                let body = issue_create_body(args)?;
                let url = format!(
                    "{}/rest/api/3/issue",
                    site_url(ctx.profile(), "jira", "issues.create")?
                );
                client
                    .request(
                        "jira",
                        "issues.create",
                        ctx.profile(),
                        Method::POST,
                        url,
                        Some(body),
                    )
                    .await
            }
            JiraIssuesAction::Update(args) => {
                let id = args.id.clone();
                let body = issue_update_body(args)?;
                let url = format!(
                    "{}/rest/api/3/issue/{}",
                    site_url(ctx.profile(), "jira", "issues.update")?,
                    enc(&id)
                );
                client
                    .request(
                        "jira",
                        "issues.update",
                        ctx.profile(),
                        Method::PUT,
                        url,
                        Some(body),
                    )
                    .await
            }
            JiraIssuesAction::Delete(args) => {
                let url = format!(
                    "{}/rest/api/3/issue/{}",
                    site_url(ctx.profile(), "jira", "issues.delete")?,
                    enc(&args.id)
                );
                client
                    .request(
                        "jira",
                        "issues.delete",
                        ctx.profile(),
                        Method::DELETE,
                        url,
                        None,
                    )
                    .await
            }
            JiraIssuesAction::Comments(command) => match command.action {
                JiraIssueCommentsAction::List(args) => {
                    list_issue_comments(client, ctx, &args.issue, args.limit).await
                }
                JiraIssueCommentsAction::Get(args) => {
                    let url = format!(
                        "{}/rest/api/3/issue/{}/comment/{}",
                        site_url(ctx.profile(), "jira", "issues.comments.get")?,
                        enc(&args.issue),
                        enc(&args.comment),
                    );
                    client
                        .request(
                            "jira",
                            "issues.comments.get",
                            ctx.profile(),
                            Method::GET,
                            url,
                            None,
                        )
                        .await
                }
                JiraIssueCommentsAction::Create(args) => {
                    let issue = args.issue.clone();
                    let body = comment_create_body(args)?;
                    let url = format!(
                        "{}/rest/api/3/issue/{}/comment",
                        site_url(ctx.profile(), "jira", "issues.comments.create")?,
                        enc(&issue),
                    );
                    client
                        .request(
                            "jira",
                            "issues.comments.create",
                            ctx.profile(),
                            Method::POST,
                            url,
                            Some(body),
                        )
                        .await
                }
            },
            JiraIssuesAction::Attachments(command) => match command.action {
                JiraIssueAttachmentsAction::List(args) => {
                    list_issue_attachments(client, ctx, args).await
                }
                JiraIssueAttachmentsAction::Download(args) => {
                    download_issue_attachment(client, ctx, args).await
                }
                JiraIssueAttachmentsAction::Upload(args) => {
                    upload_issue_attachment(client, ctx, args).await
                }
            },
        },
        JiraResource::Ideas(command) => match command.action {
            JiraIdeasAction::List(args) => {
                let jql = build_ideas_jql(&args);
                search_issues(
                    client,
                    ctx,
                    "ideas.list",
                    Some(&jql),
                    args.fields.as_deref(),
                    args.limit,
                )
                .await
            }
            JiraIdeasAction::Get(args) => {
                let url = format!(
                    "{}/rest/api/3/issue/{}",
                    site_url(ctx.profile(), "jira", "ideas.get")?,
                    enc(&args.id)
                );
                client
                    .request("jira", "ideas.get", ctx.profile(), Method::GET, url, None)
                    .await
            }
            JiraIdeasAction::Create(args) => {
                let body = idea_create_body(args)?;
                let url = format!(
                    "{}/rest/api/3/issue",
                    site_url(ctx.profile(), "jira", "ideas.create")?
                );
                client
                    .request(
                        "jira",
                        "ideas.create",
                        ctx.profile(),
                        Method::POST,
                        url,
                        Some(body),
                    )
                    .await
            }
            JiraIdeasAction::Update(args) => {
                let id = args.id.clone();
                let body = idea_update_body(args)?;
                let url = format!(
                    "{}/rest/api/3/issue/{}",
                    site_url(ctx.profile(), "jira", "ideas.update")?,
                    enc(&id)
                );
                client
                    .request(
                        "jira",
                        "ideas.update",
                        ctx.profile(),
                        Method::PUT,
                        url,
                        Some(body),
                    )
                    .await
            }
            JiraIdeasAction::Fields(args) => list_idea_fields(client, ctx, args).await,
            JiraIdeasAction::ParseUrl(args) => parse_idea_url(&args.url),
            JiraIdeasAction::Transitions(command) => match command.action {
                JiraIdeaTransitionsAction::List(args) => {
                    list_idea_transitions(client, ctx, &args.id).await
                }
                JiraIdeaTransitionsAction::Perform(args) => {
                    perform_idea_transition(client, ctx, args).await
                }
            },
        },
        JiraResource::Sprints(command) => match command.action {
            JiraSprintsAction::List(args) => {
                list_sprints(client, ctx, args.board, args.state.as_deref(), args.limit).await
            }
            JiraSprintsAction::Get(args) => {
                let url = format!(
                    "{}/rest/agile/1.0/sprint/{}",
                    site_url(ctx.profile(), "jira", "sprints.get")?,
                    args.id,
                );
                client
                    .request("jira", "sprints.get", ctx.profile(), Method::GET, url, None)
                    .await
            }
            JiraSprintsAction::Create(args) => {
                let body = sprint_create_body(args)?;
                let url = format!(
                    "{}/rest/agile/1.0/sprint",
                    site_url(ctx.profile(), "jira", "sprints.create")?,
                );
                client
                    .request(
                        "jira",
                        "sprints.create",
                        ctx.profile(),
                        Method::POST,
                        url,
                        Some(body),
                    )
                    .await
            }
            JiraSprintsAction::Issues(command) => match command.action {
                JiraSprintsIssuesAction::Add(args) => {
                    let sprint = args.sprint;
                    let body = sprint_add_issues_body(args)?;
                    let url = format!(
                        "{}/rest/agile/1.0/sprint/{}/issue",
                        site_url(ctx.profile(), "jira", "sprints.issues.add")?,
                        sprint,
                    );
                    client
                        .request(
                            "jira",
                            "sprints.issues.add",
                            ctx.profile(),
                            Method::POST,
                            url,
                            Some(body),
                        )
                        .await
                }
            },
        },
        JiraResource::Boards(command) => match command.action {
            JiraBoardsAction::List(args) => {
                list_boards(
                    client,
                    ctx,
                    args.board_type.as_deref(),
                    args.project.as_deref(),
                    args.name.as_deref(),
                    args.limit,
                )
                .await
            }
            JiraBoardsAction::Get(args) => {
                let url = format!(
                    "{}/rest/agile/1.0/board/{}",
                    site_url(ctx.profile(), "jira", "boards.get")?,
                    args.id,
                );
                client
                    .request("jira", "boards.get", ctx.profile(), Method::GET, url, None)
                    .await
            }
        },
        JiraResource::Users(command) => match command.action {
            JiraUsersAction::Get(args) => {
                let url = format!(
                    "{}/rest/api/3/user?accountId={}",
                    site_url(ctx.profile(), "jira", "users.get")?,
                    urlencoding::encode(&args.account_id),
                );
                client
                    .request("jira", "users.get", ctx.profile(), Method::GET, url, None)
                    .await
            }
        },
        JiraResource::Projects(command) => match command.action {
            JiraProjectsAction::List(args) => {
                list_projects(client, ctx, args.project_type.as_deref(), args.limit).await
            }
            JiraProjectsAction::Get(args) => {
                let url = format!(
                    "{}/rest/api/3/project/{}",
                    site_url(ctx.profile(), "jira", "projects.get")?,
                    enc(&args.id)
                );
                client
                    .request(
                        "jira",
                        "projects.get",
                        ctx.profile(),
                        Method::GET,
                        url,
                        None,
                    )
                    .await
            }
        },
    }
}

async fn search_issues(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    jql: Option<&str>,
    fields: Option<&str>,
    limit: u32,
) -> Result<Value, AppError> {
    if limit == 0 {
        return Ok(json!({ "issues": [], "maxResults": 0, "total": 0, "isLast": true }));
    }

    let base = site_url(ctx.profile(), "jira", operation)?;
    let page_size = limit.clamp(1, 100);
    let mut next_page_token = None;
    let mut first_page = None;
    let mut issues = Vec::new();

    loop {
        let mut url = format!("{base}/rest/api/3/search/jql?maxResults={page_size}");
        if let Some(jql) = jql {
            url.push_str("&jql=");
            url.push_str(&urlencoding::encode(jql));
        }
        url.push_str("&fields=");
        url.push_str(&urlencoding::encode(fields.unwrap_or(
            "key,summary,status,issuetype,assignee,created,updated,description,project",
        )));
        if let Some(token) = next_page_token.as_deref() {
            url.push_str("&nextPageToken=");
            url.push_str(&urlencoding::encode(token));
        }

        let page = client
            .request("jira", operation, ctx.profile(), Method::GET, url, None)
            .await?;
        if first_page.is_none() {
            first_page = Some(page.clone());
        }

        let page_issues = page
            .get("issues")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for issue in page_issues {
            if issues.len() >= limit as usize {
                break;
            }
            issues.push(issue);
        }

        if issues.len() >= limit as usize {
            break;
        }
        next_page_token = page
            .get("nextPageToken")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        if next_page_token.is_none() || page.get("isLast").and_then(Value::as_bool) == Some(true) {
            break;
        }
    }

    let mut response = first_page.unwrap_or_else(|| json!({}));
    let object = input::ensure_object(&mut response);
    object.insert("issues".to_string(), Value::Array(issues));
    object.insert("maxResults".to_string(), json!(limit));
    object.insert("isLast".to_string(), Value::Bool(next_page_token.is_none()));
    object.remove("nextPageToken");
    object.remove("self");
    if let Some(items) = object.get_mut("issues").and_then(Value::as_array_mut) {
        for item in items.iter_mut() {
            *item = trim_issue_fields(item, fields);
        }
    }
    Ok(response)
}

async fn list_issue_comments(
    client: &ApiClient,
    ctx: &Context,
    issue: &str,
    limit: u32,
) -> Result<Value, AppError> {
    if limit == 0 {
        return Ok(json!({ "comments": [], "maxResults": 0, "total": 0, "startAt": 0 }));
    }

    let base = site_url(ctx.profile(), "jira", "issues.comments.list")?;
    let issue_path = enc(issue);
    let page_size = limit.clamp(1, 100);
    let mut start_at: usize = 0;
    let mut first_page = None;
    let mut comments = Vec::new();

    loop {
        let url = format!(
            "{base}/rest/api/3/issue/{issue_path}/comment?maxResults={page_size}&startAt={start_at}"
        );
        let page = client
            .request(
                "jira",
                "issues.comments.list",
                ctx.profile(),
                Method::GET,
                url,
                None,
            )
            .await?;
        if first_page.is_none() {
            first_page = Some(page.clone());
        }

        let page_comments = page
            .get("comments")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if page_comments.is_empty() {
            break;
        }
        let page_len = page_comments.len();
        for comment in page_comments {
            if comments.len() >= limit as usize {
                break;
            }
            comments.push(comment);
        }

        if comments.len() >= limit as usize {
            break;
        }
        let total = page.get("total").and_then(Value::as_u64);
        let advanced = page
            .get("maxResults")
            .and_then(Value::as_u64)
            .unwrap_or(page_len as u64) as usize;
        start_at += advanced;
        if let Some(total) = total {
            if start_at as u64 >= total {
                break;
            }
        }
    }

    let mut response = first_page.unwrap_or_else(|| json!({}));
    let object = input::ensure_object(&mut response);
    object.insert("comments".to_string(), Value::Array(comments));
    object.insert("maxResults".to_string(), json!(limit));
    object.insert("startAt".to_string(), json!(0));
    object.remove("self");
    trim_array(object, "comments", trim_comment);
    Ok(response)
}

async fn list_sprints(
    client: &ApiClient,
    ctx: &Context,
    board: u64,
    state: Option<&str>,
    limit: u32,
) -> Result<Value, AppError> {
    if limit == 0 {
        return Ok(json!({ "values": [], "maxResults": 0, "startAt": 0, "isLast": true }));
    }

    let base = site_url(ctx.profile(), "jira", "sprints.list")?;
    let page_size = limit.clamp(1, 50);
    let mut start_at: usize = 0;
    let mut first_page = None;
    let mut values = Vec::new();

    loop {
        let mut url = format!(
            "{base}/rest/agile/1.0/board/{board}/sprint?maxResults={page_size}&startAt={start_at}"
        );
        if let Some(state) = state {
            url.push_str("&state=");
            url.push_str(&urlencoding::encode(state));
        }
        let page = client
            .request(
                "jira",
                "sprints.list",
                ctx.profile(),
                Method::GET,
                url,
                None,
            )
            .await?;
        if first_page.is_none() {
            first_page = Some(page.clone());
        }

        let page_values = page
            .get("values")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if page_values.is_empty() {
            break;
        }
        let page_len = page_values.len();
        for value in page_values {
            if values.len() >= limit as usize {
                break;
            }
            values.push(value);
        }

        if values.len() >= limit as usize
            || page.get("isLast").and_then(Value::as_bool) == Some(true)
        {
            break;
        }
        start_at += page
            .get("maxResults")
            .and_then(Value::as_u64)
            .unwrap_or(page_len as u64) as usize;
    }

    let mut response = first_page.unwrap_or_else(|| json!({}));
    let object = input::ensure_object(&mut response);
    object.insert("values".to_string(), Value::Array(values));
    object.insert("maxResults".to_string(), json!(limit));
    object.insert("startAt".to_string(), json!(0));
    object.remove("self");
    trim_array(object, "values", trim_sprint);
    Ok(response)
}

async fn list_boards(
    client: &ApiClient,
    ctx: &Context,
    board_type: Option<&str>,
    project: Option<&str>,
    name: Option<&str>,
    limit: u32,
) -> Result<Value, AppError> {
    if limit == 0 {
        return Ok(json!({ "values": [], "maxResults": 0, "startAt": 0, "isLast": true }));
    }

    let base = site_url(ctx.profile(), "jira", "boards.list")?;
    let page_size = limit.clamp(1, 50);
    let mut start_at: usize = 0;
    let mut first_page = None;
    let mut values = Vec::new();

    loop {
        let mut url =
            format!("{base}/rest/agile/1.0/board?maxResults={page_size}&startAt={start_at}");
        if let Some(t) = board_type {
            url.push_str("&type=");
            url.push_str(&urlencoding::encode(t));
        }
        if let Some(p) = project {
            url.push_str("&projectKeyOrId=");
            url.push_str(&urlencoding::encode(p));
        }
        if let Some(n) = name {
            url.push_str("&name=");
            url.push_str(&urlencoding::encode(n));
        }
        let page = client
            .request("jira", "boards.list", ctx.profile(), Method::GET, url, None)
            .await?;
        if first_page.is_none() {
            first_page = Some(page.clone());
        }

        let page_values = page
            .get("values")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if page_values.is_empty() {
            break;
        }
        let page_len = page_values.len();
        for value in page_values {
            if values.len() >= limit as usize {
                break;
            }
            values.push(value);
        }

        if values.len() >= limit as usize
            || page.get("isLast").and_then(Value::as_bool) == Some(true)
        {
            break;
        }
        start_at += page
            .get("maxResults")
            .and_then(Value::as_u64)
            .unwrap_or(page_len as u64) as usize;
    }

    let mut response = first_page.unwrap_or_else(|| json!({}));
    let object = input::ensure_object(&mut response);
    object.insert("values".to_string(), Value::Array(values));
    object.insert("maxResults".to_string(), json!(limit));
    object.insert("startAt".to_string(), json!(0));
    object.remove("self");
    trim_array(object, "values", trim_board);
    Ok(response)
}

async fn list_projects(
    client: &ApiClient,
    ctx: &Context,
    project_type: Option<&str>,
    limit: u32,
) -> Result<Value, AppError> {
    if limit == 0 {
        return Ok(json!({ "values": [], "maxResults": 0, "total": 0, "isLast": true }));
    }

    let base = site_url(ctx.profile(), "jira", "projects.list")?;
    let page_size = limit.clamp(1, 100);
    let mut start_at = 0usize;
    let mut first_page = None;
    let mut values = Vec::new();

    loop {
        let mut url =
            format!("{base}/rest/api/3/project/search?maxResults={page_size}&startAt={start_at}");
        if let Some(type_key) = project_type {
            url.push_str("&typeKey=");
            url.push_str(&urlencoding::encode(type_key));
        }
        let page = client
            .request(
                "jira",
                "projects.list",
                ctx.profile(),
                Method::GET,
                url,
                None,
            )
            .await?;
        if first_page.is_none() {
            first_page = Some(page.clone());
        }

        let page_values = page
            .get("values")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if page_values.is_empty() {
            break;
        }
        for value in page_values {
            if values.len() >= limit as usize {
                break;
            }
            values.push(value);
        }

        if values.len() >= limit as usize
            || page.get("isLast").and_then(Value::as_bool) == Some(true)
        {
            break;
        }
        start_at += page
            .get("maxResults")
            .and_then(Value::as_u64)
            .unwrap_or(page_size as u64) as usize;
    }

    let mut response = first_page.unwrap_or_else(|| json!({}));
    let object = input::ensure_object(&mut response);
    object.insert("values".to_string(), Value::Array(values));
    object.insert("maxResults".to_string(), json!(limit));
    object.insert("startAt".to_string(), json!(0));
    object.remove("self");
    trim_array(object, "values", trim_project);
    Ok(response)
}

async fn list_idea_fields(
    client: &ApiClient,
    ctx: &Context,
    args: JiraIdeasFields,
) -> Result<Value, AppError> {
    let base = site_url(ctx.profile(), "jira", "ideas.fields")?;
    let project = enc(&args.project);
    let url = format!("{base}/rest/api/3/issue/createmeta/{project}/issuetypes?maxResults=200");
    let issue_types_page = client
        .request(
            "jira",
            "ideas.fields",
            ctx.profile(),
            Method::GET,
            url,
            None,
        )
        .await?;
    let issue_types = issue_types_page
        .get("issueTypes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let issue_type = select_issue_type(&issue_types, args.issue_type.as_deref())?;
    let issue_type_id = issue_type
        .get("id")
        .and_then(json_id_string)
        .ok_or_else(|| {
            AppError::internal(
                "jira",
                "ideas.fields",
                "issue type id missing from createmeta response",
            )
        })?;
    let trimmed_type = pick(&issue_type, &["id", "name", "subtask"]);

    if args.limit == 0 {
        return Ok(json!({
            "fields": [],
            "maxResults": 0,
            "startAt": 0,
            "total": 0,
            "issueType": trimmed_type,
        }));
    }

    let page_size = args.limit.clamp(1, 200);
    let mut start_at = 0usize;
    let mut first_page = None;
    let mut fields = Vec::new();

    loop {
        let url = format!(
            "{base}/rest/api/3/issue/createmeta/{project}/issuetypes/{issue_type_id}?maxResults={page_size}&startAt={start_at}"
        );
        let page = client
            .request(
                "jira",
                "ideas.fields",
                ctx.profile(),
                Method::GET,
                url,
                None,
            )
            .await?;
        if first_page.is_none() {
            first_page = Some(page.clone());
        }

        let page_fields = page
            .get("fields")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if page_fields.is_empty() {
            break;
        }
        let page_len = page_fields.len();
        for field in page_fields {
            if fields.len() >= args.limit as usize {
                break;
            }
            fields.push(field);
        }

        if fields.len() >= args.limit as usize {
            break;
        }
        let total = page.get("total").and_then(Value::as_u64);
        start_at += page
            .get("maxResults")
            .and_then(Value::as_u64)
            .unwrap_or(page_len as u64) as usize;
        if let Some(total) = total {
            if start_at as u64 >= total {
                break;
            }
        }
    }

    let mut response = first_page.unwrap_or_else(|| json!({}));
    let object = input::ensure_object(&mut response);
    object.insert("fields".to_string(), Value::Array(fields));
    object.insert("maxResults".to_string(), json!(args.limit));
    object.insert("startAt".to_string(), json!(0));
    object.insert("issueType".to_string(), trimmed_type);
    object.remove("self");
    trim_array(object, "fields", trim_createmeta_field);
    Ok(response)
}

pub(crate) fn parse_idea_url(raw_url: &str) -> Result<Value, AppError> {
    let url = reqwest::Url::parse(raw_url).map_err(|err| {
        AppError::invalid_input(
            "jira",
            "ideas.parse-url",
            format!("invalid Jira URL: {err}"),
        )
    })?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(AppError::invalid_input(
            "jira",
            "ideas.parse-url",
            "Jira URL must use http or https and include a host",
        ));
    }

    let site_url = url.origin().ascii_serialization();
    let segments = url
        .path_segments()
        .map(|parts| parts.collect::<Vec<_>>())
        .unwrap_or_default();

    if let ["jira", "polaris", "projects", project, "ideas", "view", view, ..] = segments.as_slice()
    {
        let issue_key = url
            .query_pairs()
            .find_map(|(key, value)| (key == "selectedIssue").then(|| value.into_owned()));
        if issue_key
            .as_deref()
            .is_some_and(|key| issue_project_key(key).is_none())
        {
            return Err(AppError::invalid_input(
                "jira",
                "ideas.parse-url",
                "selectedIssue is not a Jira issue key such as MDP-123",
            ));
        }
        let target_type = if issue_key.is_some() {
            "product_discovery_idea"
        } else {
            "product_discovery_view"
        };
        return Ok(json!({
            "targetType": target_type,
            "siteUrl": site_url,
            "projectKey": project,
            "viewId": view,
            "issueKey": issue_key,
        }));
    }

    if let ["browse", issue_key, ..] = segments.as_slice() {
        let project_key = issue_project_key(issue_key).ok_or_else(|| {
            AppError::invalid_input(
                "jira",
                "ideas.parse-url",
                "browse URL does not contain a Jira issue key such as MDP-123",
            )
        })?;
        return Ok(json!({
            "targetType": "jira_issue",
            "siteUrl": site_url,
            "projectKey": project_key,
            "viewId": Value::Null,
            "issueKey": issue_key,
        }));
    }

    Err(AppError::invalid_input(
        "jira",
        "ideas.parse-url",
        "expected a Jira Product Discovery /jira/polaris/projects/PROJECT/ideas/view/VIEW URL or /browse/ISSUE-KEY URL",
    ))
}

fn issue_project_key(issue_key: &str) -> Option<&str> {
    let (project, number) = issue_key.rsplit_once('-')?;
    (!project.is_empty()
        && !number.is_empty()
        && number.chars().all(|character| character.is_ascii_digit()))
    .then_some(project)
}

async fn list_idea_transitions(
    client: &ApiClient,
    ctx: &Context,
    idea: &str,
) -> Result<Value, AppError> {
    let url = format!(
        "{}/rest/api/3/issue/{}/transitions",
        site_url(ctx.profile(), "jira", "ideas.transitions.list")?,
        enc(idea),
    );
    client
        .request(
            "jira",
            "ideas.transitions.list",
            ctx.profile(),
            Method::GET,
            url,
            None,
        )
        .await
}

async fn perform_idea_transition(
    client: &ApiClient,
    ctx: &Context,
    args: JiraIdeaTransitionPerform,
) -> Result<Value, AppError> {
    let idea = args.id;
    let mut body = input::read_json_arg("jira", "ideas.transitions.perform", args.json.as_deref())?;
    let requested_transition = args.transition.trim();
    if requested_transition.is_empty() {
        return Err(AppError::invalid_input(
            "jira",
            "ideas.transitions.perform",
            "--transition must contain an available transition id or name",
        ));
    }
    let transition_id = if requested_transition
        .chars()
        .all(|character| character.is_ascii_digit())
    {
        requested_transition.to_string()
    } else {
        let available = list_idea_transitions(client, ctx, &idea).await?;
        resolve_transition_id(&available, requested_transition)?
    };
    set_transition_id(&mut body, &transition_id);
    let url = format!(
        "{}/rest/api/3/issue/{}/transitions",
        site_url(ctx.profile(), "jira", "ideas.transitions.perform")?,
        enc(&idea),
    );
    client
        .request(
            "jira",
            "ideas.transitions.perform",
            ctx.profile(),
            Method::POST,
            url,
            Some(body),
        )
        .await
}

fn resolve_transition_id(response: &Value, requested_name: &str) -> Result<String, AppError> {
    let transitions = response
        .get("transitions")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let matches = transitions
        .iter()
        .filter(|transition| {
            transition
                .get("name")
                .and_then(Value::as_str)
                .is_some_and(|name| name.eq_ignore_ascii_case(requested_name))
        })
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        return matches[0]
            .get("id")
            .and_then(json_id_string)
            .ok_or_else(|| {
                AppError::internal(
                    "jira",
                    "ideas.transitions.perform",
                    "matching Jira transition has no id",
                )
            });
    }

    let available = transitions
        .iter()
        .filter_map(|transition| {
            let id = transition.get("id").and_then(json_id_string)?;
            let name = transition.get("name").and_then(Value::as_str)?;
            Some(format!("{id}: {name}"))
        })
        .collect::<Vec<_>>()
        .join(", ");
    let message = if matches.is_empty() {
        format!(
            "transition {requested_name:?} is not available; available transitions: {available}"
        )
    } else {
        format!(
            "multiple transitions are named {requested_name:?}; pass a transition id instead: {available}"
        )
    };
    Err(AppError::invalid_input(
        "jira",
        "ideas.transitions.perform",
        message,
    ))
}

fn set_transition_id(body: &mut Value, transition_id: &str) {
    input::ensure_object(body).insert("transition".to_string(), json!({ "id": transition_id }));
}

fn json_id_string(value: &Value) -> Option<String> {
    match value {
        Value::String(id) => Some(id.clone()),
        Value::Number(id) => Some(id.to_string()),
        _ => None,
    }
}

fn select_issue_type(issue_types: &[Value], requested: Option<&str>) -> Result<Value, AppError> {
    let type_names = || {
        issue_types
            .iter()
            .filter_map(|t| t.get("name").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let find_by_name = |name: &str| {
        issue_types
            .iter()
            .find(|t| {
                t.get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|candidate| candidate.eq_ignore_ascii_case(name))
            })
            .cloned()
    };
    if let Some(requested) = requested {
        return find_by_name(requested).ok_or_else(|| {
            AppError::invalid_input(
                "jira",
                "ideas.fields",
                format!(
                    "issue type {requested:?} not found in project; available: {}",
                    type_names()
                ),
            )
        });
    }
    match issue_types.len() {
        0 => Err(AppError::invalid_input(
            "jira",
            "ideas.fields",
            "project has no issue types visible to this profile; check the project key and permissions",
        )),
        1 => Ok(issue_types[0].clone()),
        _ => find_by_name("Idea").ok_or_else(|| {
            AppError::invalid_input(
                "jira",
                "ideas.fields",
                format!(
                    "project has multiple issue types; pass --type NAME (available: {})",
                    type_names()
                ),
            )
        }),
    }
}

fn issue_create_body(args: JiraIssueCreate) -> Result<Value, AppError> {
    let mut body = input::read_json_arg("jira", "issues.create", args.json.as_deref())?;
    let fields = fields_object(&mut body);
    if let Some(project) = args.project {
        fields.insert("project".to_string(), json!({ "key": project }));
    }
    if let Some(issue_type) = args.issue_type.or_else(|| Some("Task".to_string())) {
        fields.insert("issuetype".to_string(), json!({ "name": issue_type }));
    }
    if let Some(summary) = args.summary {
        fields.insert("summary".to_string(), Value::String(summary));
    }
    if let Some(description) = args.description {
        fields.insert("description".to_string(), input::minimal_adf(&description));
    }
    Ok(body)
}

fn issue_update_body(args: JiraIssueUpdate) -> Result<Value, AppError> {
    let mut body = input::read_json_arg("jira", "issues.update", args.json.as_deref())?;
    let fields = fields_object(&mut body);
    if let Some(summary) = args.summary {
        fields.insert("summary".to_string(), Value::String(summary));
    }
    if let Some(description) = args.description {
        fields.insert("description".to_string(), input::minimal_adf(&description));
    }
    Ok(body)
}

fn idea_create_body(args: JiraIdeasCreate) -> Result<Value, AppError> {
    let mut body = input::read_json_arg("jira", "ideas.create", args.json.as_deref())?;
    let fields = fields_object(&mut body);
    if let Some(project) = args.project {
        fields.insert("project".to_string(), json!({ "key": project }));
    }
    let issue_type = args.issue_type.unwrap_or_else(|| "Idea".to_string());
    fields.insert("issuetype".to_string(), json!({ "name": issue_type }));
    if let Some(summary) = args.summary {
        fields.insert("summary".to_string(), Value::String(summary));
    }
    if let Some(description) = args.description {
        fields.insert("description".to_string(), input::minimal_adf(&description));
    }
    Ok(body)
}

fn idea_update_body(args: JiraIdeasUpdate) -> Result<Value, AppError> {
    let mut body = input::read_json_arg("jira", "ideas.update", args.json.as_deref())?;
    let fields = fields_object(&mut body);
    if let Some(summary) = args.summary {
        fields.insert("summary".to_string(), Value::String(summary));
    }
    if let Some(description) = args.description {
        fields.insert("description".to_string(), input::minimal_adf(&description));
    }
    Ok(body)
}

fn comment_create_body(args: JiraIssueCommentsCreate) -> Result<Value, AppError> {
    let mut body = input::read_json_arg("jira", "issues.comments.create", args.json.as_deref())?;
    let root = input::ensure_object(&mut body);
    if let Some(text) = args.body {
        root.insert("body".to_string(), input::minimal_adf(&text));
    }
    if !root.contains_key("body") {
        return Err(AppError::invalid_input(
            "jira",
            "issues.comments.create",
            "comment body is required: pass --body TEXT or include a `body` field in --json",
        ));
    }
    Ok(body)
}

fn sprint_create_body(args: JiraSprintsCreate) -> Result<Value, AppError> {
    let mut body = input::read_json_arg("jira", "sprints.create", args.json.as_deref())?;
    let root = input::ensure_object(&mut body);
    if let Some(board) = args.board {
        root.insert("originBoardId".to_string(), json!(board));
    }
    if let Some(name) = args.name {
        root.insert("name".to_string(), Value::String(name));
    }
    if let Some(goal) = args.goal {
        root.insert("goal".to_string(), Value::String(goal));
    }
    if let Some(start_date) = args.start_date {
        root.insert("startDate".to_string(), Value::String(start_date));
    }
    if let Some(end_date) = args.end_date {
        root.insert("endDate".to_string(), Value::String(end_date));
    }
    if !root.contains_key("originBoardId") {
        return Err(AppError::invalid_input(
            "jira",
            "sprints.create",
            "sprint create requires originBoardId: pass --board or include `originBoardId` in --json",
        ));
    }
    if !root.contains_key("name") {
        return Err(AppError::invalid_input(
            "jira",
            "sprints.create",
            "sprint create requires a name: pass --name or include `name` in --json",
        ));
    }
    Ok(body)
}

fn sprint_add_issues_body(args: JiraSprintsIssuesAdd) -> Result<Value, AppError> {
    let issues: Vec<Value> = args
        .issues
        .split(',')
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(|key| Value::String(key.to_string()))
        .collect();
    if issues.is_empty() {
        return Err(AppError::invalid_input(
            "jira",
            "sprints.issues.add",
            "--issues must contain at least one issue key (comma-separated)",
        ));
    }
    Ok(json!({ "issues": issues }))
}

fn build_jql(args: &JiraIssueList) -> Option<String> {
    let mut clauses: Vec<String> = Vec::new();
    if let Some(v) = args.project.as_deref() {
        clauses.push(jql_eq("project", v));
    }
    if let Some(v) = args.status.as_deref() {
        clauses.push(jql_in_or_eq("status", v));
    }
    if let Some(v) = args.assignee.as_deref() {
        clauses.push(jql_user_clause("assignee", v));
    }
    if let Some(v) = args.issue_type.as_deref() {
        clauses.push(jql_in_or_eq("issuetype", v));
    }
    if let Some(v) = args.sprint.as_deref() {
        clauses.push(jql_sprint_clause(v));
    }
    if let Some(v) = args.text.as_deref() {
        clauses.push(format!("text ~ {}", jql_quote(v)));
    }
    if let Some(v) = args.updated_since.as_deref() {
        clauses.push(jql_updated_since_clause(v));
    }
    if clauses.is_empty() {
        None
    } else {
        Some(clauses.join(" AND "))
    }
}

fn build_ideas_jql(args: &JiraIdeasList) -> String {
    let mut clauses = vec!["projectType = product_discovery".to_string()];
    if let Some(v) = args.project.as_deref() {
        clauses.push(jql_eq("project", v));
    }
    if let Some(v) = args.status.as_deref() {
        clauses.push(jql_in_or_eq("status", v));
    }
    if let Some(v) = args.assignee.as_deref() {
        clauses.push(jql_user_clause("assignee", v));
    }
    if let Some(v) = args.text.as_deref() {
        clauses.push(format!("text ~ {}", jql_quote(v)));
    }
    if let Some(v) = args.updated_since.as_deref() {
        clauses.push(jql_updated_since_clause(v));
    }
    clauses.join(" AND ")
}

fn jql_quote(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn jql_eq(field: &str, value: &str) -> String {
    format!("{field} = {}", jql_quote(value))
}

fn jql_in_or_eq(field: &str, csv: &str) -> String {
    let values: Vec<&str> = csv
        .split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .collect();
    match values.len() {
        0 => format!("{field} = \"\""),
        1 => format!("{field} = {}", jql_quote(values[0])),
        _ => {
            let quoted: Vec<String> = values.iter().map(|v| jql_quote(v)).collect();
            format!("{field} in ({})", quoted.join(","))
        }
    }
}

fn jql_user_clause(field: &str, value: &str) -> String {
    if value == "me" {
        format!("{field} = currentUser()")
    } else {
        format!("{field} = {}", jql_quote(value))
    }
}

fn jql_sprint_clause(value: &str) -> String {
    match value {
        "current" => "sprint in openSprints()".to_string(),
        "future" => "sprint in futureSprints()".to_string(),
        "closed" => "sprint in closedSprints()".to_string(),
        v if v.chars().all(|c| c.is_ascii_digit()) => format!("sprint = {v}"),
        v => format!("sprint = {}", jql_quote(v)),
    }
}

fn jql_updated_since_clause(value: &str) -> String {
    let is_relative = value.len() >= 2
        && value
            .chars()
            .next_back()
            .is_some_and(|c| "dwmy".contains(c))
        && value[..value.len() - 1].chars().all(|c| c.is_ascii_digit());
    if is_relative {
        format!("updated >= -{value}")
    } else {
        format!("updated >= {}", jql_quote(value))
    }
}

fn trim_issue_fields(src: &Value, requested_fields: Option<&str>) -> Value {
    let mut out = serde_json::Map::new();
    let Some(obj) = src.as_object() else {
        return src.clone();
    };
    for k in ["id", "key"] {
        if let Some(v) = obj.get(k) {
            out.insert(k.to_string(), v.clone());
        }
    }
    if let Some(fields) = obj.get("fields") {
        let mut f = serde_json::Map::new();
        for k in ["summary", "created", "updated", "description"] {
            if let Some(v) = fields.get(k) {
                f.insert(k.to_string(), v.clone());
            }
        }
        if let Some(status) = fields.get("status") {
            let mut s = serde_json::Map::new();
            if let Some(n) = status.get("name") {
                s.insert("name".to_string(), n.clone());
            }
            if let Some(cat) = status.get("statusCategory") {
                s.insert("statusCategory".to_string(), pick(cat, &["name"]));
            }
            f.insert("status".to_string(), Value::Object(s));
        }
        if let Some(it) = fields.get("issuetype") {
            f.insert("issuetype".to_string(), pick(it, &["name", "subtask"]));
        }
        for person_key in ["assignee", "reporter"] {
            if let Some(p) = fields.get(person_key) {
                if p.is_null() {
                    f.insert(person_key.to_string(), Value::Null);
                } else {
                    f.insert(
                        person_key.to_string(),
                        pick(p, &["accountId", "displayName", "emailAddress"]),
                    );
                }
            }
        }
        if let Some(pr) = fields.get("priority") {
            f.insert("priority".to_string(), pick(pr, &["name"]));
        }
        if let Some(pj) = fields.get("project") {
            f.insert("project".to_string(), pick(pj, &["key", "name"]));
        }
        for requested in requested_fields
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|field| !field.is_empty())
        {
            if !f.contains_key(requested) {
                if let Some(value) = fields.get(requested) {
                    f.insert(requested.to_string(), value.clone());
                }
            }
        }
        out.insert("fields".to_string(), Value::Object(f));
    }
    Value::Object(out)
}

fn trim_project(src: &Value) -> Value {
    pick(src, &["id", "key", "name", "projectTypeKey", "isPrivate"])
}

fn trim_sprint(src: &Value) -> Value {
    pick(
        src,
        &[
            "id",
            "name",
            "state",
            "originBoardId",
            "startDate",
            "endDate",
            "createdDate",
        ],
    )
}

fn trim_board(src: &Value) -> Value {
    let mut out = serde_json::Map::new();
    let Some(obj) = src.as_object() else {
        return src.clone();
    };
    for k in ["id", "name", "type"] {
        if let Some(v) = obj.get(k) {
            out.insert(k.to_string(), v.clone());
        }
    }
    if let Some(loc) = obj.get("location") {
        out.insert(
            "location".to_string(),
            pick(loc, &["projectKey", "projectName", "projectTypeKey"]),
        );
    }
    Value::Object(out)
}

fn trim_comment(src: &Value) -> Value {
    let mut out = serde_json::Map::new();
    let Some(obj) = src.as_object() else {
        return src.clone();
    };
    for k in ["id", "created", "updated", "jsdPublic", "body"] {
        if let Some(v) = obj.get(k) {
            out.insert(k.to_string(), v.clone());
        }
    }
    for person_key in ["author", "updateAuthor"] {
        if let Some(p) = obj.get(person_key) {
            out.insert(
                person_key.to_string(),
                pick(p, &["accountId", "displayName", "emailAddress"]),
            );
        }
    }
    Value::Object(out)
}

fn trim_createmeta_field(src: &Value) -> Value {
    let mut out = serde_json::Map::new();
    let Some(obj) = src.as_object() else {
        return src.clone();
    };
    for k in ["fieldId", "key", "name", "required", "hasDefaultValue"] {
        if let Some(v) = obj.get(k) {
            out.insert(k.to_string(), v.clone());
        }
    }
    if let Some(schema) = obj.get("schema") {
        out.insert(
            "schema".to_string(),
            pick(schema, &["type", "items", "system", "custom", "customId"]),
        );
    }
    if let Some(operations) = obj.get("operations") {
        out.insert("operations".to_string(), operations.clone());
    }
    if let Some(allowed) = obj.get("allowedValues").and_then(Value::as_array) {
        let trimmed: Vec<Value> = allowed.iter().map(trim_allowed_value).collect();
        out.insert("allowedValues".to_string(), Value::Array(trimmed));
    }
    Value::Object(out)
}

fn trim_allowed_value(src: &Value) -> Value {
    if !src.is_object() {
        return src.clone();
    }
    let picked = pick(src, &["id", "value", "name", "key"]);
    if picked.as_object().is_some_and(|obj| obj.is_empty()) {
        src.clone()
    } else {
        picked
    }
}

fn trim_array(object: &mut serde_json::Map<String, Value>, key: &str, trim: fn(&Value) -> Value) {
    if let Some(arr) = object.get_mut(key).and_then(Value::as_array_mut) {
        for item in arr.iter_mut() {
            *item = trim(item);
        }
    }
}

fn fields_object(value: &mut Value) -> &mut serde_json::Map<String, Value> {
    let root = input::ensure_object(value);
    let fields = root
        .entry("fields")
        .or_insert_with(|| Value::Object(Default::default()));
    input::ensure_object(fields)
}

async fn list_issue_attachments(
    client: &ApiClient,
    ctx: &Context,
    args: JiraIssueAttachmentsList,
) -> Result<Value, AppError> {
    let base = site_url(ctx.profile(), "jira", "issues.attachments.list")?;
    let url = format!(
        "{base}/rest/api/3/issue/{}?fields=attachment",
        enc(&args.issue)
    );
    let issue = client
        .request(
            "jira",
            "issues.attachments.list",
            ctx.profile(),
            Method::GET,
            url,
            None,
        )
        .await?;
    let attachments = issue
        .get("fields")
        .and_then(|f| f.get("attachment"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let trimmed: Vec<Value> = attachments.iter().map(trim_jira_attachment).collect();
    let total = trimmed.len();
    Ok(json!({ "attachments": trimmed, "total": total }))
}

async fn download_issue_attachment(
    client: &ApiClient,
    ctx: &Context,
    args: JiraAttachmentDownload,
) -> Result<Value, AppError> {
    let base = site_url(ctx.profile(), "jira", "issues.attachments.download")?;
    let meta_url = format!("{base}/rest/api/3/attachment/{}", enc(&args.attachment_id));
    let meta = client
        .request(
            "jira",
            "issues.attachments.download",
            ctx.profile(),
            Method::GET,
            meta_url,
            None,
        )
        .await?;
    let dl_url = meta
        .get("content")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AppError::internal(
                "jira",
                "issues.attachments.download",
                "missing content URL in attachment metadata",
            )
        })?
        .to_string();
    let bytes = client
        .download("jira", "issues.attachments.download", ctx.profile(), dl_url)
        .await?;
    write_download("jira", "issues.attachments.download", &args.output, &bytes)
}

async fn upload_issue_attachment(
    client: &ApiClient,
    ctx: &Context,
    args: JiraAttachmentUpload,
) -> Result<Value, AppError> {
    let base = site_url(ctx.profile(), "jira", "issues.attachments.upload")?;
    let url = format!("{base}/rest/api/2/issue/{}/attachments", enc(&args.issue));
    client
        .upload(
            "jira",
            "issues.attachments.upload",
            ctx.profile(),
            url,
            &args.file,
            None,
        )
        .await
}

fn trim_jira_attachment(src: &Value) -> Value {
    let mut out = pick(src, &["id", "filename", "mimeType", "size", "created"]);
    if let Some(author) = src.get("author") {
        out.as_object_mut().unwrap().insert(
            "author".to_string(),
            pick(author, &["accountId", "displayName"]),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn description_flag_becomes_adf() {
        let body = issue_create_body(JiraIssueCreate {
            json: None,
            project: Some("ENG".to_string()),
            issue_type: None,
            summary: Some("Test".to_string()),
            description: Some("Details".to_string()),
        })
        .unwrap();
        assert_eq!(body["fields"]["project"]["key"], "ENG");
        assert_eq!(body["fields"]["issuetype"]["name"], "Task");
        assert_eq!(body["fields"]["description"]["type"], "doc");
    }

    #[test]
    fn idea_create_body_defaults_issue_type_to_idea() {
        let body = idea_create_body(JiraIdeasCreate {
            json: None,
            project: Some("BAW".to_string()),
            issue_type: None,
            summary: Some("Faster onboarding".to_string()),
            description: Some("Details".to_string()),
        })
        .unwrap();
        assert_eq!(body["fields"]["project"]["key"], "BAW");
        assert_eq!(body["fields"]["issuetype"]["name"], "Idea");
        assert_eq!(body["fields"]["summary"], "Faster onboarding");
        assert_eq!(body["fields"]["description"]["type"], "doc");
    }

    #[test]
    fn idea_create_body_type_flag_overrides_default() {
        let body = idea_create_body(JiraIdeasCreate {
            json: None,
            project: Some("BAW".to_string()),
            issue_type: Some("Opportunity".to_string()),
            summary: None,
            description: None,
        })
        .unwrap();
        assert_eq!(body["fields"]["issuetype"]["name"], "Opportunity");
    }

    #[test]
    fn idea_create_body_merges_custom_fields_from_json() {
        let body = idea_create_body(JiraIdeasCreate {
            json: Some(
                r#"{"fields":{"customfield_10011":{"id":"3"},"summary":"old"}}"#.to_string(),
            ),
            project: Some("BAW".to_string()),
            issue_type: None,
            summary: Some("flag wins".to_string()),
            description: None,
        })
        .unwrap();
        assert_eq!(body["fields"]["customfield_10011"]["id"], "3");
        assert_eq!(body["fields"]["summary"], "flag wins");
        assert_eq!(body["fields"]["issuetype"]["name"], "Idea");
    }

    #[test]
    fn idea_update_body_sets_summary_and_adf_description() {
        let body = idea_update_body(JiraIdeasUpdate {
            id: "BAW-1".to_string(),
            json: Some(r#"{"fields":{"customfield_10012":7}}"#.to_string()),
            summary: Some("New summary".to_string()),
            description: Some("New description".to_string()),
        })
        .unwrap();
        assert_eq!(body["fields"]["customfield_10012"], 7);
        assert_eq!(body["fields"]["summary"], "New summary");
        assert_eq!(body["fields"]["description"]["type"], "doc");
    }

    #[test]
    fn parse_idea_url_identifies_view_without_inventing_an_issue() {
        let parsed = parse_idea_url(
            "https://example.atlassian.net/jira/polaris/projects/MDP/ideas/view/24ea4b72-37ad-4950-b41c-9bc93ee5f2c8",
        )
        .unwrap();
        assert_eq!(parsed["targetType"], "product_discovery_view");
        assert_eq!(parsed["projectKey"], "MDP");
        assert_eq!(parsed["viewId"], "24ea4b72-37ad-4950-b41c-9bc93ee5f2c8");
        assert!(parsed["issueKey"].is_null());
    }

    #[test]
    fn parse_idea_url_reads_selected_issue_from_view_query() {
        let parsed = parse_idea_url(
            "https://example.atlassian.net/jira/polaris/projects/MDP/ideas/view/123?selectedIssue=MDP-42&issueViewSection=comments",
        )
        .unwrap();
        assert_eq!(parsed["targetType"], "product_discovery_idea");
        assert_eq!(parsed["projectKey"], "MDP");
        assert_eq!(parsed["viewId"], "123");
        assert_eq!(parsed["issueKey"], "MDP-42");
    }

    #[test]
    fn parse_idea_url_rejects_malformed_selected_issue() {
        let err = parse_idea_url(
            "https://example.atlassian.net/jira/polaris/projects/MDP/ideas/view/123?selectedIssue=not-an-issue",
        )
        .unwrap_err();
        assert_eq!(err.code, "invalid_input");
        assert!(err.message.contains("selectedIssue"));
    }

    #[test]
    fn parse_idea_url_accepts_browse_issue_url() {
        let parsed = parse_idea_url("https://example.atlassian.net/browse/MDP-42").unwrap();
        assert_eq!(parsed["targetType"], "jira_issue");
        assert_eq!(parsed["projectKey"], "MDP");
        assert_eq!(parsed["issueKey"], "MDP-42");
        assert!(parsed["viewId"].is_null());
    }

    #[test]
    fn parse_idea_url_rejects_opaque_uuid_as_issue_target() {
        let err =
            parse_idea_url("https://example.atlassian.net/24ea4b72-37ad-4950-b41c").unwrap_err();
        assert_eq!(err.code, "invalid_input");
        assert!(err.message.contains("Product Discovery"));
    }

    #[test]
    fn transition_name_resolves_case_insensitively() {
        let response = json!({
            "transitions": [
                {"id": "21", "name": "Parking lot"},
                {"id": "31", "name": "Research"}
            ]
        });
        assert_eq!(resolve_transition_id(&response, "research").unwrap(), "31");
    }

    #[test]
    fn unknown_transition_reports_available_ids_and_names() {
        let response = json!({"transitions": [{"id": "31", "name": "Research"}]});
        let err = resolve_transition_id(&response, "Delivery").unwrap_err();
        assert_eq!(err.code, "invalid_input");
        assert!(err.message.contains("31: Research"));
    }

    #[test]
    fn transition_flag_overrides_json_transition_and_preserves_fields() {
        let mut body = json!({
            "transition": {"id": "21"},
            "fields": {"resolution": {"name": "Done"}}
        });
        set_transition_id(&mut body, "31");
        assert_eq!(body["transition"]["id"], "31");
        assert_eq!(body["fields"]["resolution"]["name"], "Done");
    }

    fn ideas_list(
        project: Option<&str>,
        status: Option<&str>,
        assignee: Option<&str>,
        text: Option<&str>,
        updated_since: Option<&str>,
    ) -> JiraIdeasList {
        JiraIdeasList {
            project: project.map(str::to_string),
            status: status.map(str::to_string),
            assignee: assignee.map(str::to_string),
            text: text.map(str::to_string),
            updated_since: updated_since.map(str::to_string),
            fields: None,
            limit: 50,
        }
    }

    #[test]
    fn build_ideas_jql_always_scopes_to_product_discovery() {
        let args = ideas_list(None, None, None, None, None);
        assert_eq!(build_ideas_jql(&args), "projectType = product_discovery");
    }

    #[test]
    fn build_ideas_jql_combines_filters() {
        let args = ideas_list(Some("BAW"), Some("Discovery"), Some("me"), None, Some("7d"));
        assert_eq!(
            build_ideas_jql(&args),
            r#"projectType = product_discovery AND project = "BAW" AND status = "Discovery" AND assignee = currentUser() AND updated >= -7d"#
        );
    }

    #[test]
    fn select_issue_type_uses_single_type_without_flag() {
        let types = vec![json!({"id": "11444", "name": "Idea"})];
        let selected = select_issue_type(&types, None).unwrap();
        assert_eq!(selected["id"], "11444");
    }

    #[test]
    fn select_issue_type_matches_requested_name_case_insensitively() {
        let types = vec![
            json!({"id": "1", "name": "Idea"}),
            json!({"id": "2", "name": "Opportunity"}),
        ];
        let selected = select_issue_type(&types, Some("opportunity")).unwrap();
        assert_eq!(selected["id"], "2");
    }

    #[test]
    fn select_issue_type_rejects_unknown_requested_name() {
        let types = vec![json!({"id": "1", "name": "Idea"})];
        let err = select_issue_type(&types, Some("Epic")).unwrap_err();
        assert_eq!(err.code, "invalid_input");
        assert!(err.message.contains("Idea"), "error lists available types");
    }

    #[test]
    fn select_issue_type_prefers_idea_when_multiple_types() {
        let types = vec![
            json!({"id": "1", "name": "Epic"}),
            json!({"id": "2", "name": "Idea"}),
        ];
        let selected = select_issue_type(&types, None).unwrap();
        assert_eq!(selected["id"], "2");
    }

    #[test]
    fn select_issue_type_requires_flag_for_ambiguous_types() {
        let types = vec![
            json!({"id": "1", "name": "Epic"}),
            json!({"id": "2", "name": "Story"}),
        ];
        let err = select_issue_type(&types, None).unwrap_err();
        assert_eq!(err.code, "invalid_input");
        assert!(err.message.contains("--type"));
    }

    #[test]
    fn select_issue_type_rejects_empty_project() {
        let err = select_issue_type(&[], None).unwrap_err();
        assert_eq!(err.code, "invalid_input");
    }

    #[test]
    fn trim_createmeta_field_keeps_schema_and_allowed_values() {
        let src = json!({
            "allowedValues": [
                {
                    "id": "10",
                    "self": "https://example.atlassian.net/rest/api/3/customFieldOption/10",
                    "value": "High"
                },
                {
                    "id": "11",
                    "self": "https://example.atlassian.net/rest/api/3/customFieldOption/11",
                    "value": "Low"
                }
            ],
            "autoCompleteUrl": "",
            "fieldId": "customfield_10011",
            "hasDefaultValue": false,
            "key": "customfield_10011",
            "name": "Impact",
            "operations": ["set"],
            "required": false,
            "schema": {
                "custom": "com.atlassian.jira.plugin.system.customfieldtypes:select",
                "customId": 10011,
                "type": "option"
            }
        });
        let trimmed = trim_createmeta_field(&src);
        let obj = trimmed.as_object().unwrap();
        assert_eq!(obj.get("fieldId").unwrap(), "customfield_10011");
        assert_eq!(obj.get("name").unwrap(), "Impact");
        assert_eq!(obj.get("required").unwrap(), &Value::Bool(false));
        assert_eq!(obj.get("operations").unwrap(), &json!(["set"]));
        assert!(!obj.contains_key("autoCompleteUrl"));
        let schema = obj.get("schema").unwrap().as_object().unwrap();
        assert_eq!(schema.get("type").unwrap(), "option");
        assert_eq!(schema.get("customId").unwrap(), 10011);
        let allowed = obj.get("allowedValues").unwrap().as_array().unwrap();
        assert_eq!(allowed[0], json!({"id": "10", "value": "High"}));
        assert_eq!(allowed[1], json!({"id": "11", "value": "Low"}));
    }

    #[test]
    fn comment_body_flag_becomes_adf() {
        let body = comment_create_body(JiraIssueCommentsCreate {
            issue: "ENG-1".to_string(),
            json: None,
            body: Some("hello from agent".to_string()),
        })
        .unwrap();
        assert_eq!(body["body"]["type"], "doc");
        assert_eq!(
            body["body"]["content"][0]["content"][0]["text"],
            "hello from agent"
        );
    }

    #[test]
    fn comment_body_flag_overrides_json() {
        let body = comment_create_body(JiraIssueCommentsCreate {
            issue: "ENG-1".to_string(),
            json: Some(r#"{"body":{"type":"doc","version":1,"content":[]}}"#.to_string()),
            body: Some("flag wins".to_string()),
        })
        .unwrap();
        assert_eq!(
            body["body"]["content"][0]["content"][0]["text"],
            "flag wins"
        );
    }

    #[test]
    fn comment_create_requires_body() {
        let err = comment_create_body(JiraIssueCommentsCreate {
            issue: "ENG-1".to_string(),
            json: None,
            body: None,
        })
        .unwrap_err();
        assert_eq!(err.code, "invalid_input");
    }

    #[test]
    fn sprint_create_flags_populate_body() {
        let body = sprint_create_body(JiraSprintsCreate {
            json: None,
            board: Some(7),
            name: Some("Sprint 1".to_string()),
            goal: Some("ship it".to_string()),
            start_date: Some("2026-05-18T00:00:00.000Z".to_string()),
            end_date: Some("2026-06-01T00:00:00.000Z".to_string()),
        })
        .unwrap();
        assert_eq!(body["originBoardId"], 7);
        assert_eq!(body["name"], "Sprint 1");
        assert_eq!(body["goal"], "ship it");
        assert_eq!(body["startDate"], "2026-05-18T00:00:00.000Z");
        assert_eq!(body["endDate"], "2026-06-01T00:00:00.000Z");
    }

    #[test]
    fn sprint_create_flag_overrides_json() {
        let body = sprint_create_body(JiraSprintsCreate {
            json: Some(r#"{"originBoardId":1,"name":"old"}"#.to_string()),
            board: Some(9),
            name: Some("new".to_string()),
            goal: None,
            start_date: None,
            end_date: None,
        })
        .unwrap();
        assert_eq!(body["originBoardId"], 9);
        assert_eq!(body["name"], "new");
    }

    #[test]
    fn sprint_create_requires_origin_board_id() {
        let err = sprint_create_body(JiraSprintsCreate {
            json: None,
            board: None,
            name: Some("Sprint 1".to_string()),
            goal: None,
            start_date: None,
            end_date: None,
        })
        .unwrap_err();
        assert_eq!(err.code, "invalid_input");
    }

    #[test]
    fn sprint_create_requires_name() {
        let err = sprint_create_body(JiraSprintsCreate {
            json: None,
            board: Some(7),
            name: None,
            goal: None,
            start_date: None,
            end_date: None,
        })
        .unwrap_err();
        assert_eq!(err.code, "invalid_input");
    }

    #[test]
    fn sprint_add_issues_body_parses_comma_list() {
        let body = sprint_add_issues_body(JiraSprintsIssuesAdd {
            sprint: 1,
            issues: "ENG-1,ENG-2".to_string(),
        })
        .unwrap();
        assert_eq!(body["issues"][0], "ENG-1");
        assert_eq!(body["issues"][1], "ENG-2");
        assert_eq!(body["issues"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn sprint_add_issues_body_trims_whitespace() {
        let body = sprint_add_issues_body(JiraSprintsIssuesAdd {
            sprint: 1,
            issues: " ENG-1 , ENG-2 ".to_string(),
        })
        .unwrap();
        assert_eq!(body["issues"][0], "ENG-1");
        assert_eq!(body["issues"][1], "ENG-2");
    }

    #[test]
    fn sprint_add_issues_body_rejects_empty() {
        for input in ["", ",", " , "] {
            let err = sprint_add_issues_body(JiraSprintsIssuesAdd {
                sprint: 1,
                issues: input.to_string(),
            })
            .unwrap_err();
            assert_eq!(err.code, "invalid_input", "input {input:?} should reject");
        }
    }

    #[test]
    fn trim_issue_keeps_allowlist_and_drops_noise() {
        let src = json!({
            "expand": "renderedFields,names,schema",
            "id": "10000",
            "key": "SCRUM-1",
            "self": "https://example.atlassian.net/rest/api/3/issue/10000",
            "fields": {
                "summary": "Task 1",
                "created": "2026-05-13T17:37:03.156+0300",
                "updated": "2026-05-19T14:57:46.997+0300",
                "description": { "type": "doc", "version": 1, "content": [] },
                "status": {
                    "description": "",
                    "iconUrl": "https://example.atlassian.net/images/icons/statuses/generic.png",
                    "id": "10000",
                    "name": "To Do",
                    "self": "https://example.atlassian.net/rest/api/3/status/10000",
                    "statusCategory": {
                        "colorName": "blue-gray",
                        "id": 2,
                        "key": "new",
                        "name": "To Do",
                        "self": "https://example.atlassian.net/rest/api/3/statuscategory/2"
                    }
                },
                "issuetype": {
                    "avatarId": 10318,
                    "entityId": "32cd4f8d-44cd-4d43-b76f-b06e517b12fc",
                    "hierarchyLevel": 0,
                    "iconUrl": "https://example.atlassian.net/issuetype/10003",
                    "id": "10003",
                    "name": "Task",
                    "self": "https://example.atlassian.net/rest/api/3/issuetype/10003",
                    "subtask": false
                },
                "assignee": null,
                "project": {
                    "avatarUrls": { "16x16": "..." },
                    "id": "10000",
                    "key": "SCRUM",
                    "name": "test-space",
                    "projectTypeKey": "software",
                    "self": "https://example.atlassian.net/rest/api/3/project/10000",
                    "simplified": true
                }
            }
        });
        let trimmed = trim_issue_fields(&src, None);
        let obj = trimmed.as_object().unwrap();
        assert!(!obj.contains_key("expand"));
        assert!(!obj.contains_key("self"));
        assert_eq!(obj.get("key").unwrap(), "SCRUM-1");
        assert_eq!(obj.get("id").unwrap(), "10000");
        let fields = obj.get("fields").unwrap().as_object().unwrap();
        assert_eq!(fields.get("summary").unwrap(), "Task 1");
        let status = fields.get("status").unwrap().as_object().unwrap();
        assert_eq!(status.get("name").unwrap(), "To Do");
        assert!(!status.contains_key("iconUrl"));
        assert!(!status.contains_key("self"));
        let category = status.get("statusCategory").unwrap().as_object().unwrap();
        assert_eq!(category.get("name").unwrap(), "To Do");
        assert!(!category.contains_key("colorName"));
        let issuetype = fields.get("issuetype").unwrap().as_object().unwrap();
        assert_eq!(issuetype.get("name").unwrap(), "Task");
        assert!(!issuetype.contains_key("iconUrl"));
        assert!(!issuetype.contains_key("avatarId"));
        let project = fields.get("project").unwrap().as_object().unwrap();
        assert_eq!(project.get("key").unwrap(), "SCRUM");
        assert!(!project.contains_key("avatarUrls"));
        assert!(fields.get("assignee").unwrap().is_null());
        assert!(fields.get("description").unwrap().is_object());
    }

    #[test]
    fn trim_issue_preserves_explicitly_requested_custom_fields() {
        let src = json!({
            "id": "10101",
            "key": "MDP-42",
            "fields": {
                "summary": "Multiplexing",
                "customfield_10011": {"id": "10", "value": "Research"},
                "customfield_10012": null,
                "unrequested": "noise"
            }
        });
        let trimmed = trim_issue_fields(
            &src,
            Some("key,summary,customfield_10011,customfield_10012"),
        );
        assert_eq!(
            trimmed["fields"]["customfield_10011"],
            json!({"id": "10", "value": "Research"})
        );
        assert!(trimmed["fields"]["customfield_10012"].is_null());
        assert!(trimmed["fields"].get("unrequested").is_none());
    }

    #[test]
    fn trim_project_strips_avatars_and_self() {
        let src = json!({
            "avatarUrls": { "16x16": "..." },
            "entityId": "3b03af20-229e-4513-89bb-602eb471f98f",
            "expand": "description,lead,issueTypes",
            "id": "10000",
            "isPrivate": false,
            "key": "SCRUM",
            "name": "test-space",
            "projectTypeKey": "software",
            "properties": {},
            "self": "https://example.atlassian.net/rest/api/3/project/10000",
            "simplified": true,
            "style": "next-gen",
            "uuid": "3b03af20-229e-4513-89bb-602eb471f98f"
        });
        let trimmed = trim_project(&src);
        let obj = trimmed.as_object().unwrap();
        assert_eq!(obj.get("id").unwrap(), "10000");
        assert_eq!(obj.get("key").unwrap(), "SCRUM");
        assert_eq!(obj.get("name").unwrap(), "test-space");
        assert_eq!(obj.get("projectTypeKey").unwrap(), "software");
        assert_eq!(obj.get("isPrivate").unwrap(), &Value::Bool(false));
        for noise in [
            "avatarUrls",
            "entityId",
            "expand",
            "uuid",
            "self",
            "style",
            "properties",
            "simplified",
        ] {
            assert!(!obj.contains_key(noise), "should drop {noise}");
        }
    }

    #[test]
    fn trim_sprint_keeps_dates_and_state() {
        let active = json!({
            "endDate": "2026-05-27T14:37:05.363Z",
            "id": 2,
            "name": "SCRUM Sprint 0",
            "originBoardId": 1,
            "self": "https://example.atlassian.net/rest/agile/1.0/sprint/2",
            "startDate": "2026-05-13T14:37:05.363Z",
            "state": "active"
        });
        let trimmed = trim_sprint(&active);
        let obj = trimmed.as_object().unwrap();
        assert_eq!(obj.get("id").unwrap(), 2);
        assert_eq!(obj.get("name").unwrap(), "SCRUM Sprint 0");
        assert_eq!(obj.get("state").unwrap(), "active");
        assert_eq!(obj.get("endDate").unwrap(), "2026-05-27T14:37:05.363Z");
        assert!(!obj.contains_key("self"));

        let future = json!({ "id": 34, "name": "future-sprint", "originBoardId": 1, "self": "x", "state": "future" });
        let trimmed = trim_sprint(&future);
        let obj = trimmed.as_object().unwrap();
        assert!(!obj.contains_key("endDate"));
        assert!(!obj.contains_key("self"));
        assert_eq!(obj.get("state").unwrap(), "future");
    }

    #[test]
    fn trim_board_strips_location_avatar() {
        let src = json!({
            "id": 1,
            "isPrivate": false,
            "location": {
                "avatarURI": "https://example.atlassian.net/avatar.png",
                "displayName": "test-space (SCRUM)",
                "name": "test-space (SCRUM)",
                "projectId": 10000,
                "projectKey": "SCRUM",
                "projectName": "test-space",
                "projectTypeKey": "software"
            },
            "name": "SCRUM board",
            "self": "https://example.atlassian.net/rest/agile/1.0/board/1",
            "type": "simple"
        });
        let trimmed = trim_board(&src);
        let obj = trimmed.as_object().unwrap();
        assert_eq!(obj.get("id").unwrap(), 1);
        assert_eq!(obj.get("name").unwrap(), "SCRUM board");
        assert_eq!(obj.get("type").unwrap(), "simple");
        assert!(!obj.contains_key("self"));
        assert!(!obj.contains_key("isPrivate"));
        let loc = obj.get("location").unwrap().as_object().unwrap();
        assert_eq!(loc.get("projectKey").unwrap(), "SCRUM");
        assert_eq!(loc.get("projectName").unwrap(), "test-space");
        assert!(!loc.contains_key("avatarURI"));
        assert!(!loc.contains_key("projectId"));
        assert!(!loc.contains_key("displayName"));
    }

    #[test]
    fn trim_comment_keeps_author_drops_avatar() {
        let src = json!({
            "author": {
                "accountId": "712020:abc",
                "accountType": "atlassian",
                "active": true,
                "avatarUrls": { "16x16": "..." },
                "displayName": "Marselle Wing",
                "emailAddress": "marsellewing@gmail.com",
                "self": "https://example.atlassian.net/rest/api/3/user?accountId=abc",
                "timeZone": "Africa/Addis_Ababa"
            },
            "body": { "type": "doc", "version": 1, "content": [
                { "type": "paragraph", "content": [{ "type": "text", "text": "hello" }] }
            ]},
            "created": "2026-05-19T14:56:55.744+0300",
            "id": "10001",
            "jsdPublic": true,
            "self": "https://example.atlassian.net/rest/api/3/issue/10000/comment/10001",
            "updateAuthor": {
                "accountId": "712020:abc",
                "avatarUrls": { "16x16": "..." },
                "displayName": "Marselle Wing",
                "emailAddress": "marsellewing@gmail.com",
                "self": "https://example.atlassian.net/rest/api/3/user?accountId=abc"
            },
            "updated": "2026-05-19T14:56:55.744+0300"
        });
        let trimmed = trim_comment(&src);
        let obj = trimmed.as_object().unwrap();
        assert_eq!(obj.get("id").unwrap(), "10001");
        assert_eq!(obj.get("jsdPublic").unwrap(), &Value::Bool(true));
        assert!(!obj.contains_key("self"));
        let author = obj.get("author").unwrap().as_object().unwrap();
        assert_eq!(author.get("displayName").unwrap(), "Marselle Wing");
        assert_eq!(author.get("accountId").unwrap(), "712020:abc");
        assert!(!author.contains_key("avatarUrls"));
        assert!(!author.contains_key("accountType"));
        assert!(!author.contains_key("active"));
        assert!(!author.contains_key("timeZone"));
        assert!(!author.contains_key("self"));
        let update_author = obj.get("updateAuthor").unwrap().as_object().unwrap();
        assert_eq!(update_author.get("displayName").unwrap(), "Marselle Wing");
        assert!(!update_author.contains_key("avatarUrls"));
        let body = obj.get("body").unwrap();
        assert_eq!(body.get("type").unwrap(), "doc");
        assert!(body.get("content").unwrap().is_array());
    }

    fn issue_list(
        project: Option<&str>,
        status: Option<&str>,
        assignee: Option<&str>,
        issue_type: Option<&str>,
        sprint: Option<&str>,
        text: Option<&str>,
        updated_since: Option<&str>,
    ) -> JiraIssueList {
        JiraIssueList {
            project: project.map(str::to_string),
            status: status.map(str::to_string),
            assignee: assignee.map(str::to_string),
            issue_type: issue_type.map(str::to_string),
            sprint: sprint.map(str::to_string),
            text: text.map(str::to_string),
            updated_since: updated_since.map(str::to_string),
            fields: None,
            limit: 50,
        }
    }

    #[test]
    fn build_jql_empty_inputs_returns_none() {
        let args = issue_list(None, None, None, None, None, None, None);
        assert_eq!(build_jql(&args), None);
    }

    #[test]
    fn build_jql_single_project() {
        let args = issue_list(Some("SCRUM"), None, None, None, None, None, None);
        assert_eq!(build_jql(&args).unwrap(), r#"project = "SCRUM""#);
    }

    #[test]
    fn build_jql_status_single_vs_multi() {
        let single = issue_list(None, Some("To Do"), None, None, None, None, None);
        assert_eq!(build_jql(&single).unwrap(), r#"status = "To Do""#);

        let multi = issue_list(
            None,
            Some("To Do,In Progress"),
            None,
            None,
            None,
            None,
            None,
        );
        assert_eq!(
            build_jql(&multi).unwrap(),
            r#"status in ("To Do","In Progress")"#
        );
    }

    #[test]
    fn build_jql_assignee_me_uses_currentuser() {
        let me = issue_list(None, None, Some("me"), None, None, None, None);
        assert_eq!(build_jql(&me).unwrap(), "assignee = currentUser()");

        let id = issue_list(None, None, Some("712020:abc"), None, None, None, None);
        assert_eq!(build_jql(&id).unwrap(), r#"assignee = "712020:abc""#);
    }

    #[test]
    fn build_jql_sprint_keywords_and_id() {
        for (input, expected) in [
            ("current", "sprint in openSprints()"),
            ("future", "sprint in futureSprints()"),
            ("closed", "sprint in closedSprints()"),
            ("42", "sprint = 42"),
        ] {
            let args = issue_list(None, None, None, None, Some(input), None, None);
            assert_eq!(
                build_jql(&args).unwrap(),
                expected,
                "input {input:?} should map to {expected:?}"
            );
        }
        let named = issue_list(None, None, None, None, Some("My Sprint"), None, None);
        assert_eq!(build_jql(&named).unwrap(), r#"sprint = "My Sprint""#);
    }

    #[test]
    fn build_jql_updated_since_relative_vs_iso() {
        let rel = issue_list(None, None, None, None, None, None, Some("7d"));
        assert_eq!(build_jql(&rel).unwrap(), "updated >= -7d");

        let iso = issue_list(None, None, None, None, None, None, Some("2026-05-01"));
        assert_eq!(build_jql(&iso).unwrap(), r#"updated >= "2026-05-01""#);
    }

    #[test]
    fn build_jql_combines_clauses_with_and() {
        let args = issue_list(
            Some("SCRUM"),
            Some("To Do"),
            Some("me"),
            None,
            None,
            None,
            None,
        );
        assert_eq!(
            build_jql(&args).unwrap(),
            r#"project = "SCRUM" AND status = "To Do" AND assignee = currentUser()"#
        );
    }

    #[test]
    fn jql_quote_escapes_inner_quotes() {
        assert_eq!(jql_quote(r#"with "x""#), r#""with \"x\"""#);
    }
}
