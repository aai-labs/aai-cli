use reqwest::Method;
use serde_json::Value;

use crate::{
    cli::{
        MicrosoftCalendarCommand, MicrosoftCalendarResource, MicrosoftContactsAction,
        MicrosoftContactsCommand, MicrosoftEventsAction, MicrosoftMailCommand,
        MicrosoftMailResource, MicrosoftMessagesAction, MicrosoftUserCreate, MicrosoftUserItemArg,
        MicrosoftUserListArgs, MicrosoftUserUpdate,
    },
    config::Context,
    error::AppError,
    http::ApiClient,
};

use super::common::{collection, parse_body, request, user_root};

pub(super) async fn mail(
    client: &ApiClient,
    ctx: &Context,
    command: MicrosoftMailCommand,
) -> Result<Value, AppError> {
    match command.resource {
        MicrosoftMailResource::Messages(command) => match command.action {
            MicrosoftMessagesAction::List(args) => {
                user_list(client, ctx, "mail.messages.list", "messages", args).await
            }
            MicrosoftMessagesAction::Get(args) => {
                user_get(client, ctx, "mail.messages.get", "messages", args).await
            }
            MicrosoftMessagesAction::Create(args) => {
                user_create(client, ctx, "mail.messages.create", "messages", args).await
            }
            MicrosoftMessagesAction::Update(args) => {
                user_update(client, ctx, "mail.messages.update", "messages", args).await
            }
            MicrosoftMessagesAction::Delete(args) => {
                user_delete(client, ctx, "mail.messages.delete", "messages", args).await
            }
        },
        MicrosoftMailResource::Send(args) => {
            let root = user_root(ctx, args.user.user_id.as_deref())?;
            let body = parse_body("mail.send", args.json.as_deref())?;
            request(
                client,
                ctx,
                "mail.send",
                Method::POST,
                &format!("{root}/sendMail"),
                Some(body),
            )
            .await
        }
    }
}

pub(super) async fn calendar(
    client: &ApiClient,
    ctx: &Context,
    command: MicrosoftCalendarCommand,
) -> Result<Value, AppError> {
    match command.resource {
        MicrosoftCalendarResource::Events(command) => match command.action {
            MicrosoftEventsAction::List(args) => {
                user_list(client, ctx, "calendar.events.list", "events", args).await
            }
            MicrosoftEventsAction::Get(args) => {
                user_get(client, ctx, "calendar.events.get", "events", args).await
            }
            MicrosoftEventsAction::Create(args) => {
                user_create(client, ctx, "calendar.events.create", "events", args).await
            }
            MicrosoftEventsAction::Update(args) => {
                user_update(client, ctx, "calendar.events.update", "events", args).await
            }
            MicrosoftEventsAction::Delete(args) => {
                user_delete(client, ctx, "calendar.events.delete", "events", args).await
            }
        },
    }
}

pub(super) async fn contacts(
    client: &ApiClient,
    ctx: &Context,
    command: MicrosoftContactsCommand,
) -> Result<Value, AppError> {
    match command.action {
        MicrosoftContactsAction::List(args) => {
            user_list(client, ctx, "contacts.list", "contacts", args).await
        }
        MicrosoftContactsAction::Get(args) => {
            user_get(client, ctx, "contacts.get", "contacts", args).await
        }
        MicrosoftContactsAction::Create(args) => {
            user_create(client, ctx, "contacts.create", "contacts", args).await
        }
        MicrosoftContactsAction::Update(args) => {
            user_update(client, ctx, "contacts.update", "contacts", args).await
        }
        MicrosoftContactsAction::Delete(args) => {
            user_delete(client, ctx, "contacts.delete", "contacts", args).await
        }
    }
}

async fn user_list(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    resource: &str,
    args: MicrosoftUserListArgs,
) -> Result<Value, AppError> {
    let root = user_root(ctx, args.user.user_id.as_deref())?;
    collection(
        client,
        ctx,
        operation,
        &format!("{root}/{resource}"),
        args.list.limit,
    )
    .await
}

async fn user_get(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    resource: &str,
    args: MicrosoftUserItemArg,
) -> Result<Value, AppError> {
    let root = user_root(ctx, args.user.user_id.as_deref())?;
    request(
        client,
        ctx,
        operation,
        Method::GET,
        &format!("{root}/{resource}/{}", urlencoding::encode(&args.id)),
        None,
    )
    .await
}

async fn user_create(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    resource: &str,
    args: MicrosoftUserCreate,
) -> Result<Value, AppError> {
    let root = user_root(ctx, args.user.user_id.as_deref())?;
    let body = parse_body(operation, args.json.as_deref())?;
    request(
        client,
        ctx,
        operation,
        Method::POST,
        &format!("{root}/{resource}"),
        Some(body),
    )
    .await
}

async fn user_update(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    resource: &str,
    args: MicrosoftUserUpdate,
) -> Result<Value, AppError> {
    let root = user_root(ctx, args.user.user_id.as_deref())?;
    let body = parse_body(operation, args.json.as_deref())?;
    request(
        client,
        ctx,
        operation,
        Method::PATCH,
        &format!("{root}/{resource}/{}", urlencoding::encode(&args.id)),
        Some(body),
    )
    .await
}

async fn user_delete(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    resource: &str,
    args: MicrosoftUserItemArg,
) -> Result<Value, AppError> {
    let root = user_root(ctx, args.user.user_id.as_deref())?;
    request(
        client,
        ctx,
        operation,
        Method::DELETE,
        &format!("{root}/{resource}/{}", urlencoding::encode(&args.id)),
        None,
    )
    .await
}
