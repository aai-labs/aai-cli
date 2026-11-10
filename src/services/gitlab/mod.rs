//! GitLab REST v4, for gitlab.com and self-hosted instances alike.
//!
//! `profile.base_url` is the API root (`https://host/api/v4`), not the instance origin,
//! so a credential-gateway profile can replace it whole. GitLab addresses projects, files
//! and branches by a single URL-encoded path segment, so those go through [`enc`] and
//! must never be built with `/` between their parts: `group%2Fapp` is a project, and
//! `group/app` is a 404.
//!
//! Lists walk `page=N` rather than following `Link`, whose URLs name the upstream host
//! and would step around a credential gateway.

mod cli;

use reqwest::Method;
use serde_json::{json, Value};

pub use cli::*;

use crate::{
    config::Context,
    error::AppError,
    http::ApiClient,
    input,
    services::{
        generic_request,
        shared::{enc, trim_url, write_download, CtxProfile},
    },
};

const SERVICE: &str = "gitlab";
const DEFAULT_API_ROOT: &str = "https://gitlab.com/api/v4";
const PER_PAGE_MAX: u32 = 100;

fn api_root(ctx: &Context) -> String {
    ctx.profile()
        .base_url
        .as_deref()
        .map(trim_url)
        .unwrap_or_else(|| DEFAULT_API_ROOT.to_string())
}

fn project_url(
    ctx: &Context,
    project: Option<&str>,
    operation: &'static str,
) -> Result<String, AppError> {
    let project = project
        .or(ctx.profile().repo.as_deref())
        .filter(|project| !project.is_empty())
        .ok_or_else(|| {
            AppError::service_config(
                SERVICE,
                operation,
                format!("gitlab.{operation} requires --project or profile.repo"),
            )
        })?;
    Ok(format!("{}/projects/{}", api_root(ctx), enc(project)))
}

fn scoped(
    ctx: &Context,
    project: &GitlabProjectArg,
    operation: &'static str,
    suffix: &str,
) -> Result<String, AppError> {
    Ok(format!(
        "{}{suffix}",
        project_url(ctx, project.project.as_deref(), operation)?
    ))
}

fn with_query(mut url: String, query: &[(&str, Option<&str>)]) -> String {
    for (key, value) in query {
        if let Some(value) = value {
            url.push(if url.contains('?') { '&' } else { '?' });
            url.push_str(key);
            url.push('=');
            url.push_str(&enc(value));
        }
    }
    url
}

async fn send(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    method: Method,
    url: String,
    body: Option<Value>,
) -> Result<Value, AppError> {
    client
        .request(SERVICE, operation, ctx.profile(), method, url, body)
        .await
}

/// Walks `page=N` until `limit` is reached or the provider returns a short page.
/// Returns `{ values, size, truncated }`, the shape GitHub lists use.
async fn paginate(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    url: String,
    limit: u32,
) -> Result<Value, AppError> {
    let limit = limit.max(1);
    let per_page = limit.min(PER_PAGE_MAX);
    let separator = if url.contains('?') { '&' } else { '?' };
    let mut values = Vec::new();
    let mut truncated = false;
    for page in 1.. {
        let response = send(
            client,
            ctx,
            operation,
            Method::GET,
            format!("{url}{separator}per_page={per_page}&page={page}"),
            None,
        )
        .await?;
        let Value::Array(items) = response else {
            return Err(AppError::internal(
                SERVICE,
                operation,
                "expected a JSON array from GitLab",
            ));
        };
        let short = (items.len() as u32) < per_page;
        values.extend(items);
        if values.len() >= limit as usize {
            values.truncate(limit as usize);
            truncated = true;
            break;
        }
        if short {
            break;
        }
    }
    let size = values.len();
    Ok(json!({ "values": values, "size": size, "truncated": truncated }))
}

async fn text_or_file(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    url: String,
    output: Option<String>,
    field: &str,
) -> Result<Value, AppError> {
    let bytes = client
        .download_with_accept(SERVICE, operation, ctx.profile(), url, "*/*")
        .await?;
    if let Some(output) = output {
        return write_download(SERVICE, operation, &output, &bytes);
    }
    let text = String::from_utf8(bytes).map_err(|_| {
        AppError::invalid_input(
            SERVICE,
            operation,
            "content is not UTF-8 text; pass --output to save it to a file",
        )
    })?;
    Ok(json!({ field: text }))
}

fn body_with(
    operation: &'static str,
    json_arg: Option<&str>,
    fields: &[(&str, &Option<String>)],
) -> Result<Value, AppError> {
    let mut body = input::read_json_arg(SERVICE, operation, json_arg)?;
    for (key, value) in fields {
        input::set_string(&mut body, key, value);
    }
    Ok(body)
}

fn require_fields(operation: &'static str, body: &Value, fields: &[&str]) -> Result<(), AppError> {
    let missing: Vec<&str> = fields
        .iter()
        .copied()
        .filter(|field| body.get(field).is_none())
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    Err(AppError::invalid_input(
        SERVICE,
        operation,
        format!("gitlab.{operation} requires {}", missing.join(", ")),
    ))
}

fn note_body(operation: &'static str, args: &GitlabNoteCreate) -> Result<Value, AppError> {
    let body = body_with(operation, args.json.as_deref(), &[("body", &args.body)])?;
    require_fields(operation, &body, &["body"])?;
    Ok(body)
}

pub(crate) async fn dispatch(
    client: &ApiClient,
    ctx: &Context,
    command: GitlabCommand,
) -> Result<Value, AppError> {
    match command.resource {
        GitlabResource::Request(args) => {
            generic_request::dispatch(client, ctx, SERVICE, api_root(ctx), args).await
        }
        GitlabResource::Projects(command) => projects(client, ctx, command.action).await,
        GitlabResource::Mrs(command) => mrs(client, ctx, command.action).await,
        GitlabResource::Issues(command) => issues(client, ctx, command.action).await,
        GitlabResource::Branches(command) => branches(client, ctx, command.action).await,
        GitlabResource::Commits(command) => commits(client, ctx, command.action).await,
        GitlabResource::Source(command) => source(client, ctx, command.action).await,
        GitlabResource::Pipelines(command) => pipelines(client, ctx, command.action).await,
        GitlabResource::Jobs(command) => jobs(client, ctx, command.action).await,
    }
}

async fn projects(
    client: &ApiClient,
    ctx: &Context,
    action: GitlabProjectsAction,
) -> Result<Value, AppError> {
    match action {
        GitlabProjectsAction::List(args) => {
            let url = with_query(
                format!("{}/projects", api_root(ctx)),
                &[
                    ("membership", (!args.all).then_some("true")),
                    ("search", args.search.as_deref()),
                ],
            );
            paginate(client, ctx, "projects.list", url, args.limit.limit).await
        }
        GitlabProjectsAction::Get(args) => {
            let url = project_url(ctx, args.project.as_deref(), "projects.get")?;
            send(client, ctx, "projects.get", Method::GET, url, None).await
        }
    }
}

fn mr_url(
    ctx: &Context,
    args: &GitlabIid,
    operation: &'static str,
    suffix: &str,
) -> Result<String, AppError> {
    scoped(
        ctx,
        &args.project,
        operation,
        &format!("/merge_requests/{}{suffix}", args.iid),
    )
}

async fn mrs(client: &ApiClient, ctx: &Context, action: GitlabMrAction) -> Result<Value, AppError> {
    match action {
        GitlabMrAction::List(args) => {
            let url = with_query(
                scoped(ctx, &args.project, "mrs.list", "/merge_requests")?,
                &[
                    ("state", args.state.as_deref()),
                    ("source_branch", args.source_branch.as_deref()),
                    ("target_branch", args.target_branch.as_deref()),
                    ("search", args.search.as_deref()),
                ],
            );
            paginate(client, ctx, "mrs.list", url, args.limit.limit).await
        }
        GitlabMrAction::Get(args) => {
            let url = mr_url(ctx, &args, "mrs.get", "")?;
            send(client, ctx, "mrs.get", Method::GET, url, None).await
        }
        GitlabMrAction::Create(args) => {
            let operation = "mrs.create";
            let url = scoped(ctx, &args.project, operation, "/merge_requests")?;
            let body = body_with(
                operation,
                args.json.as_deref(),
                &[
                    ("source_branch", &args.source_branch),
                    ("target_branch", &args.target_branch),
                    ("title", &args.title),
                    ("description", &args.description),
                ],
            )?;
            require_fields(
                operation,
                &body,
                &["source_branch", "target_branch", "title"],
            )?;
            send(client, ctx, operation, Method::POST, url, Some(body)).await
        }
        GitlabMrAction::Update(args) => {
            let operation = "mrs.update";
            let url = scoped(
                ctx,
                &args.project,
                operation,
                &format!("/merge_requests/{}", args.iid),
            )?;
            let body = body_with(
                operation,
                args.json.as_deref(),
                &[
                    ("title", &args.title),
                    ("description", &args.description),
                    ("target_branch", &args.target_branch),
                ],
            )?;
            send(client, ctx, operation, Method::PUT, url, Some(body)).await
        }
        GitlabMrAction::Close(args) => {
            let url = mr_url(ctx, &args, "mrs.close", "")?;
            let body = json!({ "state_event": "close" });
            send(client, ctx, "mrs.close", Method::PUT, url, Some(body)).await
        }
        GitlabMrAction::Reopen(args) => {
            let url = mr_url(ctx, &args, "mrs.reopen", "")?;
            let body = json!({ "state_event": "reopen" });
            send(client, ctx, "mrs.reopen", Method::PUT, url, Some(body)).await
        }
        GitlabMrAction::Diffs(args) => {
            let operation = "mrs.diffs";
            let url = scoped(
                ctx,
                &args.project,
                operation,
                &format!("/merge_requests/{}/diffs", args.iid),
            )?;
            let value = paginate(client, ctx, operation, url, args.limit.limit).await?;
            let Some(output) = args.output else {
                return Ok(value);
            };
            let bytes = serde_json::to_vec(&value).map_err(|err| {
                AppError::internal(SERVICE, operation, format!("failed to encode diffs: {err}"))
            })?;
            write_download(SERVICE, operation, &output, &bytes)
        }
        GitlabMrAction::Commits(args) => {
            list_under_mr(client, ctx, args, "mrs.commits", "commits").await
        }
        GitlabMrAction::Approve(args) => {
            let url = mr_url(ctx, &args, "mrs.approve", "/approve")?;
            send(client, ctx, "mrs.approve", Method::POST, url, None).await
        }
        GitlabMrAction::Unapprove(args) => {
            let url = mr_url(ctx, &args, "mrs.unapprove", "/unapprove")?;
            send(client, ctx, "mrs.unapprove", Method::POST, url, None).await
        }
        GitlabMrAction::Approvals(args) => {
            let url = mr_url(ctx, &args, "mrs.approvals", "/approvals")?;
            send(client, ctx, "mrs.approvals", Method::GET, url, None).await
        }
        GitlabMrAction::Notes(command) => mr_notes(client, ctx, command.action).await,
        GitlabMrAction::Discussions(command) => match command.action {
            GitlabDiscussionAction::List(args) => {
                list_under_mr(client, ctx, args, "mrs.discussions.list", "discussions").await
            }
            GitlabDiscussionAction::Create(args) => {
                let operation = "mrs.discussions.create";
                let url = scoped(
                    ctx,
                    &args.project,
                    operation,
                    &format!("/merge_requests/{}/discussions", args.iid),
                )?;
                let body = note_body(operation, &args)?;
                send(client, ctx, operation, Method::POST, url, Some(body)).await
            }
        },
    }
}

async fn list_under_mr(
    client: &ApiClient,
    ctx: &Context,
    args: GitlabIidList,
    operation: &'static str,
    collection: &str,
) -> Result<Value, AppError> {
    let url = scoped(
        ctx,
        &args.project,
        operation,
        &format!("/merge_requests/{}/{collection}", args.iid),
    )?;
    paginate(client, ctx, operation, url, args.limit.limit).await
}

fn mr_note_url(
    ctx: &Context,
    project: &GitlabProjectArg,
    iid: u64,
    note_id: u64,
    operation: &'static str,
) -> Result<String, AppError> {
    scoped(
        ctx,
        project,
        operation,
        &format!("/merge_requests/{iid}/notes/{note_id}"),
    )
}

async fn mr_notes(
    client: &ApiClient,
    ctx: &Context,
    action: GitlabNoteAction,
) -> Result<Value, AppError> {
    match action {
        GitlabNoteAction::List(args) => {
            list_under_mr(client, ctx, args, "mrs.notes.list", "notes").await
        }
        GitlabNoteAction::Get(args) => {
            let operation = "mrs.notes.get";
            let url = mr_note_url(ctx, &args.project, args.iid, args.note_id, operation)?;
            send(client, ctx, operation, Method::GET, url, None).await
        }
        GitlabNoteAction::Create(args) => {
            let operation = "mrs.notes.create";
            let url = scoped(
                ctx,
                &args.project,
                operation,
                &format!("/merge_requests/{}/notes", args.iid),
            )?;
            let body = note_body(operation, &args)?;
            send(client, ctx, operation, Method::POST, url, Some(body)).await
        }
        GitlabNoteAction::Update(args) => {
            let operation = "mrs.notes.update";
            let url = mr_note_url(ctx, &args.project, args.iid, args.note_id, operation)?;
            let body = body_with(operation, args.json.as_deref(), &[("body", &args.body)])?;
            require_fields(operation, &body, &["body"])?;
            send(client, ctx, operation, Method::PUT, url, Some(body)).await
        }
        GitlabNoteAction::Delete(args) => {
            let operation = "mrs.notes.delete";
            let url = mr_note_url(ctx, &args.project, args.iid, args.note_id, operation)?;
            send(client, ctx, operation, Method::DELETE, url, None).await
        }
    }
}

async fn issues(
    client: &ApiClient,
    ctx: &Context,
    action: GitlabIssueAction,
) -> Result<Value, AppError> {
    match action {
        GitlabIssueAction::List(args) => {
            let url = with_query(
                scoped(ctx, &args.project, "issues.list", "/issues")?,
                &[
                    ("state", args.state.as_deref()),
                    ("search", args.search.as_deref()),
                    ("labels", args.labels.as_deref()),
                ],
            );
            paginate(client, ctx, "issues.list", url, args.limit.limit).await
        }
        GitlabIssueAction::Get(args) => {
            let url = scoped(
                ctx,
                &args.project,
                "issues.get",
                &format!("/issues/{}", args.iid),
            )?;
            send(client, ctx, "issues.get", Method::GET, url, None).await
        }
        GitlabIssueAction::Create(args) => {
            let operation = "issues.create";
            let url = scoped(ctx, &args.project, operation, "/issues")?;
            let body = body_with(
                operation,
                args.json.as_deref(),
                &[("title", &args.title), ("description", &args.description)],
            )?;
            require_fields(operation, &body, &["title"])?;
            send(client, ctx, operation, Method::POST, url, Some(body)).await
        }
        GitlabIssueAction::Update(args) => {
            let operation = "issues.update";
            let url = scoped(
                ctx,
                &args.project,
                operation,
                &format!("/issues/{}", args.iid),
            )?;
            let body = body_with(
                operation,
                args.json.as_deref(),
                &[
                    ("title", &args.title),
                    ("description", &args.description),
                    ("state_event", &args.state_event),
                ],
            )?;
            send(client, ctx, operation, Method::PUT, url, Some(body)).await
        }
        GitlabIssueAction::Notes(command) => match command.action {
            GitlabIssueNoteAction::List(args) => {
                let operation = "issues.notes.list";
                let url = scoped(
                    ctx,
                    &args.project,
                    operation,
                    &format!("/issues/{}/notes", args.iid),
                )?;
                paginate(client, ctx, operation, url, args.limit.limit).await
            }
            GitlabIssueNoteAction::Create(args) => {
                let operation = "issues.notes.create";
                let url = scoped(
                    ctx,
                    &args.project,
                    operation,
                    &format!("/issues/{}/notes", args.iid),
                )?;
                let body = note_body(operation, &args)?;
                send(client, ctx, operation, Method::POST, url, Some(body)).await
            }
        },
    }
}

async fn branches(
    client: &ApiClient,
    ctx: &Context,
    action: GitlabBranchAction,
) -> Result<Value, AppError> {
    match action {
        GitlabBranchAction::List(args) => {
            let url = with_query(
                scoped(ctx, &args.project, "branches.list", "/repository/branches")?,
                &[("search", args.search.as_deref())],
            );
            paginate(client, ctx, "branches.list", url, args.limit.limit).await
        }
        GitlabBranchAction::Get(args) => {
            let url = scoped(
                ctx,
                &args.project,
                "branches.get",
                &format!("/repository/branches/{}", enc(&args.branch)),
            )?;
            send(client, ctx, "branches.get", Method::GET, url, None).await
        }
        GitlabBranchAction::Create(args) => {
            let url = scoped(
                ctx,
                &args.project,
                "branches.create",
                "/repository/branches",
            )?;
            let body = json!({ "branch": args.branch, "ref": args.reference });
            send(
                client,
                ctx,
                "branches.create",
                Method::POST,
                url,
                Some(body),
            )
            .await
        }
        GitlabBranchAction::Delete(args) => {
            let url = scoped(
                ctx,
                &args.project,
                "branches.delete",
                &format!("/repository/branches/{}", enc(&args.branch)),
            )?;
            send(client, ctx, "branches.delete", Method::DELETE, url, None).await
        }
    }
}

async fn commits(
    client: &ApiClient,
    ctx: &Context,
    action: GitlabCommitAction,
) -> Result<Value, AppError> {
    match action {
        GitlabCommitAction::List(args) => {
            let url = with_query(
                scoped(ctx, &args.project, "commits.list", "/repository/commits")?,
                &[
                    ("ref_name", args.ref_name.as_deref()),
                    ("path", args.path.as_deref()),
                ],
            );
            paginate(client, ctx, "commits.list", url, args.limit.limit).await
        }
        GitlabCommitAction::Get(args) => {
            let url = scoped(
                ctx,
                &args.project,
                "commits.get",
                &format!("/repository/commits/{}", enc(&args.sha)),
            )?;
            send(client, ctx, "commits.get", Method::GET, url, None).await
        }
        GitlabCommitAction::Diff(args) => {
            let url = scoped(
                ctx,
                &args.project,
                "commits.diff",
                &format!("/repository/commits/{}/diff", enc(&args.sha)),
            )?;
            paginate(client, ctx, "commits.diff", url, args.limit.limit).await
        }
    }
}

async fn source(
    client: &ApiClient,
    ctx: &Context,
    action: GitlabSourceAction,
) -> Result<Value, AppError> {
    match action {
        GitlabSourceAction::Get(args) => {
            let operation = "source.get";
            let url = scoped(
                ctx,
                &args.project,
                operation,
                &format!(
                    "/repository/files/{}/raw?ref={}",
                    enc(args.path.trim_start_matches('/')),
                    enc(&args.reference)
                ),
            )?;
            text_or_file(client, ctx, operation, url, args.output, "content").await
        }
        GitlabSourceAction::Tree(args) => {
            let url = with_query(
                scoped(ctx, &args.project, "source.tree", "/repository/tree")?,
                &[
                    ("ref", args.reference.as_deref()),
                    ("path", args.path.as_deref()),
                    ("recursive", args.recursive.then_some("true")),
                ],
            );
            paginate(client, ctx, "source.tree", url, args.limit.limit).await
        }
    }
}

async fn pipelines(
    client: &ApiClient,
    ctx: &Context,
    action: GitlabPipelineAction,
) -> Result<Value, AppError> {
    match action {
        GitlabPipelineAction::List(args) => {
            let url = with_query(
                scoped(ctx, &args.project, "pipelines.list", "/pipelines")?,
                &[
                    ("ref", args.reference.as_deref()),
                    ("status", args.status.as_deref()),
                ],
            );
            paginate(client, ctx, "pipelines.list", url, args.limit.limit).await
        }
        GitlabPipelineAction::Get(args) => {
            let url = pipeline_url(ctx, &args, "pipelines.get", "")?;
            send(client, ctx, "pipelines.get", Method::GET, url, None).await
        }
        GitlabPipelineAction::Jobs(args) => {
            let url = scoped(
                ctx,
                &args.project,
                "pipelines.jobs",
                &format!("/pipelines/{}/jobs", args.pipeline_id),
            )?;
            paginate(client, ctx, "pipelines.jobs", url, args.limit.limit).await
        }
        GitlabPipelineAction::Retry(args) => {
            let url = pipeline_url(ctx, &args, "pipelines.retry", "/retry")?;
            send(client, ctx, "pipelines.retry", Method::POST, url, None).await
        }
        GitlabPipelineAction::Cancel(args) => {
            let url = pipeline_url(ctx, &args, "pipelines.cancel", "/cancel")?;
            send(client, ctx, "pipelines.cancel", Method::POST, url, None).await
        }
    }
}

fn pipeline_url(
    ctx: &Context,
    args: &GitlabPipelineId,
    operation: &'static str,
    suffix: &str,
) -> Result<String, AppError> {
    scoped(
        ctx,
        &args.project,
        operation,
        &format!("/pipelines/{}{suffix}", args.pipeline_id),
    )
}

async fn jobs(
    client: &ApiClient,
    ctx: &Context,
    action: GitlabJobAction,
) -> Result<Value, AppError> {
    match action {
        GitlabJobAction::Get(args) => {
            let url = scoped(
                ctx,
                &args.project,
                "jobs.get",
                &format!("/jobs/{}", args.job_id),
            )?;
            send(client, ctx, "jobs.get", Method::GET, url, None).await
        }
        GitlabJobAction::Log(args) => {
            let operation = "jobs.log";
            let url = scoped(
                ctx,
                &args.project,
                operation,
                &format!("/jobs/{}/trace", args.job_id),
            )?;
            text_or_file(client, ctx, operation, url, args.output, "log").await
        }
    }
}
