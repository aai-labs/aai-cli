use reqwest::Method;
use serde_json::{json, Value};

use crate::{
    cli::{
        MicrosoftTodoCommand, MicrosoftTodoListsAction, MicrosoftTodoResource,
        MicrosoftTodoTasksAction,
    },
    config::Context,
    error::AppError,
    http::ApiClient,
    services::shared::CtxProfile,
};

use super::common::{collection, parse_body, request, user_root};

pub(super) async fn dispatch(
    client: &ApiClient,
    ctx: &Context,
    command: MicrosoftTodoCommand,
) -> Result<Value, AppError> {
    ensure_delegated(ctx)?;
    match command.resource {
        MicrosoftTodoResource::Lists(command) => match command.action {
            MicrosoftTodoListsAction::List(args) => {
                let root = user_root(ctx, args.user.user_id.as_deref())?;
                collection(
                    client,
                    ctx,
                    "todo.lists.list",
                    &format!("{root}/todo/lists"),
                    args.list.limit,
                )
                .await
            }
            MicrosoftTodoListsAction::Get(args) => {
                let root = user_root(ctx, args.user.user_id.as_deref())?;
                request(
                    client,
                    ctx,
                    "todo.lists.get",
                    Method::GET,
                    &format!("{root}/todo/lists/{}", enc(&args.id)),
                    None,
                )
                .await
            }
            MicrosoftTodoListsAction::Create(args) => {
                let root = user_root(ctx, args.user.user_id.as_deref())?;
                let body = parse_body("todo.lists.create", args.json.as_deref())?;
                request(
                    client,
                    ctx,
                    "todo.lists.create",
                    Method::POST,
                    &format!("{root}/todo/lists"),
                    Some(body),
                )
                .await
            }
            MicrosoftTodoListsAction::Update(args) => {
                let root = user_root(ctx, args.user.user_id.as_deref())?;
                let body = parse_body("todo.lists.update", args.json.as_deref())?;
                request(
                    client,
                    ctx,
                    "todo.lists.update",
                    Method::PATCH,
                    &format!("{root}/todo/lists/{}", enc(&args.id)),
                    Some(body),
                )
                .await
            }
            MicrosoftTodoListsAction::Delete(args) => {
                let root = user_root(ctx, args.user.user_id.as_deref())?;
                request(
                    client,
                    ctx,
                    "todo.lists.delete",
                    Method::DELETE,
                    &format!("{root}/todo/lists/{}", enc(&args.id)),
                    None,
                )
                .await
            }
        },
        MicrosoftTodoResource::Tasks(command) => match command.action {
            MicrosoftTodoTasksAction::List(args) => {
                let root = user_root(ctx, args.user.user_id.as_deref())?;
                collection(
                    client,
                    ctx,
                    "todo.tasks.list",
                    &format!("{root}/todo/lists/{}/tasks", enc(&args.list_id)),
                    args.list.limit,
                )
                .await
            }
            MicrosoftTodoTasksAction::Get(args) => {
                let root = user_root(ctx, args.user.user_id.as_deref())?;
                request(
                    client,
                    ctx,
                    "todo.tasks.get",
                    Method::GET,
                    &task_path(&root, &args.list_id, &args.task_id),
                    None,
                )
                .await
            }
            MicrosoftTodoTasksAction::Create(args) => {
                let root = user_root(ctx, args.user.user_id.as_deref())?;
                let body = parse_body("todo.tasks.create", args.json.as_deref())?;
                request(
                    client,
                    ctx,
                    "todo.tasks.create",
                    Method::POST,
                    &format!("{root}/todo/lists/{}/tasks", enc(&args.list_id)),
                    Some(body),
                )
                .await
            }
            MicrosoftTodoTasksAction::Update(args) => {
                let root = user_root(ctx, args.user.user_id.as_deref())?;
                let body = parse_body("todo.tasks.update", args.json.as_deref())?;
                request(
                    client,
                    ctx,
                    "todo.tasks.update",
                    Method::PATCH,
                    &task_path(&root, &args.list_id, &args.task_id),
                    Some(body),
                )
                .await
            }
            MicrosoftTodoTasksAction::Delete(args) => {
                let root = user_root(ctx, args.user.user_id.as_deref())?;
                request(
                    client,
                    ctx,
                    "todo.tasks.delete",
                    Method::DELETE,
                    &task_path(&root, &args.list_id, &args.task_id),
                    None,
                )
                .await
            }
        },
    }
}

fn ensure_delegated(ctx: &Context) -> Result<(), AppError> {
    if ctx.profile().auth_type.as_deref() == Some("microsoft_delegated") {
        return Ok(());
    }
    Err(AppError::unsupported_auth(
        super::SERVICE,
        "todo",
        "Microsoft To Do Graph APIs do not support application permissions; use a microsoft_delegated profile",
        Some(json!({"required_auth_type": "microsoft_delegated", "required_scope": "Tasks.ReadWrite"})),
    ))
}

fn task_path(root: &str, list_id: &str, task_id: &str) -> String {
    format!("{root}/todo/lists/{}/tasks/{}", enc(list_id), enc(task_id))
}

fn enc(value: &str) -> String {
    urlencoding::encode(value).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todo_rejects_application_auth_before_calling_graph() {
        let ctx = Context {
            profile: crate::config::Profile {
                auth_type: Some("microsoft_client_credentials".to_string()),
                ..Default::default()
            },
            secrets_file: Default::default(),
            key_file: Default::default(),
        };
        let error = ensure_delegated(&ctx).unwrap_err();
        assert_eq!(error.code, "unsupported_auth");
    }
}
