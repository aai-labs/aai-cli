use reqwest::Method;
use serde_json::Value;

use crate::{
    cli::{
        MicrosoftPlannerCommand, MicrosoftPlannerGetAction, MicrosoftPlannerResource,
        MicrosoftPlannerTasksAction,
    },
    config::Context,
    error::AppError,
    http::ApiClient,
};

use super::common::{parse_body, request, request_with_headers};

pub(super) async fn dispatch(
    client: &ApiClient,
    ctx: &Context,
    command: MicrosoftPlannerCommand,
) -> Result<Value, AppError> {
    match command.resource {
        MicrosoftPlannerResource::Plans(command) => match command.action {
            MicrosoftPlannerGetAction::Get(args) => {
                get(client, ctx, "planner.plans.get", "plans", &args.id).await
            }
        },
        MicrosoftPlannerResource::Buckets(command) => match command.action {
            MicrosoftPlannerGetAction::Get(args) => {
                get(client, ctx, "planner.buckets.get", "buckets", &args.id).await
            }
        },
        MicrosoftPlannerResource::Tasks(command) => match command.action {
            MicrosoftPlannerTasksAction::Get(args) => {
                get(client, ctx, "planner.tasks.get", "tasks", &args.id).await
            }
            MicrosoftPlannerTasksAction::Create(args) => {
                let body = parse_body("planner.tasks.create", args.json.as_deref())?;
                request(
                    client,
                    ctx,
                    "planner.tasks.create",
                    Method::POST,
                    "planner/tasks",
                    Some(body),
                )
                .await
            }
            MicrosoftPlannerTasksAction::Update(args) => {
                let body = parse_body("planner.tasks.update", args.json.as_deref())?;
                request_with_headers(
                    client,
                    ctx,
                    "planner.tasks.update",
                    Method::PATCH,
                    &format!("planner/tasks/{}", enc(&args.id)),
                    Some(body),
                    vec![("If-Match".to_string(), args.etag)],
                )
                .await
            }
            MicrosoftPlannerTasksAction::Delete(args) => {
                request_with_headers(
                    client,
                    ctx,
                    "planner.tasks.delete",
                    Method::DELETE,
                    &format!("planner/tasks/{}", enc(&args.id)),
                    None,
                    vec![("If-Match".to_string(), args.etag)],
                )
                .await
            }
        },
    }
}

async fn get(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    resource: &str,
    id: &str,
) -> Result<Value, AppError> {
    request(
        client,
        ctx,
        operation,
        Method::GET,
        &format!("planner/{resource}/{}", enc(id)),
        None,
    )
    .await
}

fn enc(value: &str) -> String {
    urlencoding::encode(value).into_owned()
}
