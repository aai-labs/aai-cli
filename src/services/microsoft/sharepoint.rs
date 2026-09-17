use reqwest::Method;
use serde_json::Value;

use crate::{
    cli::{
        MicrosoftFileDownload, MicrosoftFileTarget, MicrosoftFileUpload, MicrosoftListItemArg,
        MicrosoftSharepointCommand, MicrosoftSharepointFileDownload, MicrosoftSharepointFileTarget,
        MicrosoftSharepointFileUpload, MicrosoftSharepointFilesAction,
        MicrosoftSharepointItemsAction, MicrosoftSharepointListsAction,
        MicrosoftSharepointResource,
    },
    config::Context,
    error::AppError,
    http::ApiClient,
};

use super::common::{collection, parse_body, request};

pub(super) async fn dispatch(
    client: &ApiClient,
    ctx: &Context,
    command: MicrosoftSharepointCommand,
) -> Result<Value, AppError> {
    match command.resource {
        MicrosoftSharepointResource::Files(command) => match command.action {
            MicrosoftSharepointFilesAction::Upload(args) => {
                super::upload(client, ctx, "sharepoint.files.upload", file_upload(args)).await
            }
            MicrosoftSharepointFilesAction::Download(args) => {
                super::download(
                    client,
                    ctx,
                    "sharepoint.files.download",
                    file_download(args),
                )
                .await
            }
            MicrosoftSharepointFilesAction::Delete(args) => {
                super::delete(client, ctx, "sharepoint.files.delete", file_target(args)).await
            }
        },
        MicrosoftSharepointResource::Lists(command) => match command.action {
            MicrosoftSharepointListsAction::List(args) => {
                collection(
                    client,
                    ctx,
                    "sharepoint.lists.list",
                    &format!("sites/{}/lists", enc(&args.site_id)),
                    args.list.limit,
                )
                .await
            }
            MicrosoftSharepointListsAction::Get(args) => {
                request(
                    client,
                    ctx,
                    "sharepoint.lists.get",
                    Method::GET,
                    &format!("sites/{}/lists/{}", enc(&args.site_id), enc(&args.list_id)),
                    None,
                )
                .await
            }
        },
        MicrosoftSharepointResource::Items(command) => match command.action {
            MicrosoftSharepointItemsAction::List(args) => {
                collection(
                    client,
                    ctx,
                    "sharepoint.items.list",
                    &format!(
                        "sites/{}/lists/{}/items?%24expand=fields",
                        enc(&args.site_id),
                        enc(&args.list_id)
                    ),
                    args.list.limit,
                )
                .await
            }
            MicrosoftSharepointItemsAction::Get(args) => {
                request(
                    client,
                    ctx,
                    "sharepoint.items.get",
                    Method::GET,
                    &format!("{}?%24expand=fields", item_path(&args)),
                    None,
                )
                .await
            }
            MicrosoftSharepointItemsAction::Create(args) => {
                let body = parse_body("sharepoint.items.create", args.json.as_deref())?;
                request(
                    client,
                    ctx,
                    "sharepoint.items.create",
                    Method::POST,
                    &format!(
                        "sites/{}/lists/{}/items",
                        enc(&args.site_id),
                        enc(&args.list_id)
                    ),
                    Some(body),
                )
                .await
            }
            MicrosoftSharepointItemsAction::Update(args) => {
                let body = parse_body("sharepoint.items.update", args.json.as_deref())?;
                request(
                    client,
                    ctx,
                    "sharepoint.items.update",
                    Method::PATCH,
                    &format!(
                        "sites/{}/lists/{}/items/{}/fields",
                        enc(&args.site_id),
                        enc(&args.list_id),
                        enc(&args.item_id)
                    ),
                    Some(body),
                )
                .await
            }
            MicrosoftSharepointItemsAction::Delete(args) => {
                request(
                    client,
                    ctx,
                    "sharepoint.items.delete",
                    Method::DELETE,
                    &item_path(&args),
                    None,
                )
                .await
            }
        },
    }
}

fn file_target(args: MicrosoftSharepointFileTarget) -> MicrosoftFileTarget {
    MicrosoftFileTarget {
        path: args.path,
        drive_id: Some(args.drive_id),
        user_id: None,
    }
}

fn file_upload(args: MicrosoftSharepointFileUpload) -> MicrosoftFileUpload {
    MicrosoftFileUpload {
        file: args.file,
        target: file_target(args.target),
        mime_type: args.mime_type,
    }
}

fn file_download(args: MicrosoftSharepointFileDownload) -> MicrosoftFileDownload {
    MicrosoftFileDownload {
        target: file_target(args.target),
        output: args.output,
    }
}

fn item_path(args: &MicrosoftListItemArg) -> String {
    format!(
        "sites/{}/lists/{}/items/{}",
        enc(&args.site_id),
        enc(&args.list_id),
        enc(&args.item_id)
    )
}

fn enc(value: &str) -> String {
    urlencoding::encode(value).into_owned()
}
