//! Typed Microsoft Graph workbook operations.
//!
//! This module deliberately stays at the Graph workbook boundary.  It does not
//! download workbooks or attempt to edit OOXML locally; callers that need a
//! Word (or unsupported Excel) operation should use the Microsoft file commands
//! and an external document library.

use reqwest::Method;
use serde_json::{json, Map, Value};

use crate::{
    cli::{
        MicrosoftExcelCommand, MicrosoftExcelRangeClear, MicrosoftExcelRangeUpdate,
        MicrosoftExcelResource, MicrosoftExcelTableCreate, MicrosoftExcelTableRowsAppend,
        MicrosoftExcelTablesAction, MicrosoftExcelWorksheetDelete, MicrosoftExcelWorksheetMutation,
        MicrosoftExcelWorksheetRename, MicrosoftExcelWorksheetsAction,
        MicrosoftExcelWorksheetsCommand, MicrosoftWorkbookTarget,
    },
    config::Context,
    error::AppError,
    http::ApiClient,
    input,
    services::shared::CtxProfile,
};

use super::common::{collection, request};

const SERVICE: &str = "microsoft";

pub(super) async fn dispatch(
    client: &ApiClient,
    ctx: &Context,
    mut command: MicrosoftExcelCommand,
) -> Result<Value, AppError> {
    ensure_delegated(ctx, "excel")?;
    resolve_command_targets(client, ctx, &mut command).await?;

    match command.resource {
        MicrosoftExcelResource::Worksheets(command) => worksheets(client, ctx, command).await,
        MicrosoftExcelResource::Ranges(command) => ranges(client, ctx, command).await,
        MicrosoftExcelResource::Tables(command) => tables(client, ctx, command).await,
    }
}

/// Resolve path targets to drive-item IDs before calling a workbook endpoint.
///
/// Graph's drive-item path form is reliable for metadata, while some tenants
/// reject a path directly on the workbook relationship with a WAC token error
/// (notably SharePoint document libraries). Resolving once and using the
/// documented item-ID workbook form keeps both `--path` and `--item-id`
/// targeting equivalent and preserves the final Graph response unchanged.
async fn resolve_command_targets(
    client: &ApiClient,
    ctx: &Context,
    command: &mut MicrosoftExcelCommand,
) -> Result<(), AppError> {
    match &mut command.resource {
        MicrosoftExcelResource::Worksheets(command) => match &mut command.action {
            MicrosoftExcelWorksheetsAction::List(args) => {
                resolve_target(client, ctx, &mut args.workbook).await
            }
            MicrosoftExcelWorksheetsAction::Add(args) => {
                resolve_target(client, ctx, &mut args.workbook).await
            }
            MicrosoftExcelWorksheetsAction::Rename(args) => {
                resolve_target(client, ctx, &mut args.workbook).await
            }
            MicrosoftExcelWorksheetsAction::Delete(args) => {
                resolve_target(client, ctx, &mut args.workbook).await
            }
        },
        MicrosoftExcelResource::Ranges(command) => match &mut command.action {
            crate::cli::MicrosoftExcelRangesAction::Get(args) => {
                resolve_target(client, ctx, &mut args.workbook).await
            }
            crate::cli::MicrosoftExcelRangesAction::Update(args) => {
                resolve_target(client, ctx, &mut args.target.workbook).await
            }
            crate::cli::MicrosoftExcelRangesAction::Clear(args) => {
                resolve_target(client, ctx, &mut args.target.workbook).await
            }
        },
        MicrosoftExcelResource::Tables(command) => match &mut command.action {
            MicrosoftExcelTablesAction::List(args) => {
                resolve_target(client, ctx, &mut args.workbook).await
            }
            MicrosoftExcelTablesAction::Create(args) => {
                resolve_target(client, ctx, &mut args.workbook).await
            }
            MicrosoftExcelTablesAction::Delete(args) => {
                resolve_target(client, ctx, &mut args.workbook).await
            }
            MicrosoftExcelTablesAction::Rows(command) => match command {
                crate::cli::MicrosoftExcelTableRowsCommand::List(args) => {
                    resolve_target(client, ctx, &mut args.target.workbook).await
                }
                crate::cli::MicrosoftExcelTableRowsCommand::Append(args) => {
                    resolve_target(client, ctx, &mut args.target.workbook).await
                }
            },
        },
    }
}

async fn resolve_target(
    client: &ApiClient,
    ctx: &Context,
    target: &mut MicrosoftWorkbookTarget,
) -> Result<(), AppError> {
    if target.item_id.is_some() {
        return Ok(());
    }
    let path = target.path.as_deref().ok_or_else(|| {
        AppError::invalid_input(
            SERVICE,
            "excel.workbook.resolve",
            "workbook requires --item-id or --path",
        )
    })?;
    let metadata = request(
        client,
        ctx,
        "excel.workbook.resolve",
        Method::GET,
        &format!(
            "{}/root:/{}:",
            drive_root(ctx, target),
            encode_drive_path(path)
        ),
        None,
    )
    .await?;
    let item_id = metadata
        .get("id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::internal(
                SERVICE,
                "excel.workbook.resolve",
                "drive-item response is missing id",
            )
        })?;
    target.item_id = Some(item_id.to_string());
    target.path = None;
    Ok(())
}

async fn worksheets(
    client: &ApiClient,
    ctx: &Context,
    command: MicrosoftExcelWorksheetsCommand,
) -> Result<Value, AppError> {
    match command.action {
        MicrosoftExcelWorksheetsAction::List(args) => {
            collection(
                client,
                ctx,
                "excel.worksheets.list",
                &workbook_path(ctx, &args.workbook, "worksheets"),
                args.list.limit,
            )
            .await
        }
        MicrosoftExcelWorksheetsAction::Add(args) => worksheet_add(client, ctx, &args).await,
        MicrosoftExcelWorksheetsAction::Rename(args) => worksheet_rename(client, ctx, &args).await,
        MicrosoftExcelWorksheetsAction::Delete(args) => worksheet_delete(client, ctx, &args).await,
    }
}

async fn worksheet_add(
    client: &ApiClient,
    ctx: &Context,
    args: &MicrosoftExcelWorksheetMutation,
) -> Result<Value, AppError> {
    request(
        client,
        ctx,
        "excel.worksheets.add",
        Method::POST,
        &format!("{}/worksheets/add", workbook_root(ctx, &args.workbook)),
        Some(json!({"name": args.name})),
    )
    .await
}

async fn worksheet_rename(
    client: &ApiClient,
    ctx: &Context,
    args: &MicrosoftExcelWorksheetRename,
) -> Result<Value, AppError> {
    request(
        client,
        ctx,
        "excel.worksheets.rename",
        Method::PATCH,
        &worksheet_path(ctx, &args.workbook, &args.worksheet),
        Some(json!({"name": args.name})),
    )
    .await
}

async fn worksheet_delete(
    client: &ApiClient,
    ctx: &Context,
    args: &MicrosoftExcelWorksheetDelete,
) -> Result<Value, AppError> {
    request(
        client,
        ctx,
        "excel.worksheets.delete",
        Method::DELETE,
        &worksheet_path(ctx, &args.workbook, &args.worksheet),
        None,
    )
    .await
}

async fn ranges(
    client: &ApiClient,
    ctx: &Context,
    command: crate::cli::MicrosoftExcelRangesCommand,
) -> Result<Value, AppError> {
    match command.action {
        crate::cli::MicrosoftExcelRangesAction::Get(args) => {
            request(
                client,
                ctx,
                "excel.ranges.get",
                Method::GET,
                &range_path(ctx, &args.workbook, &args.worksheet, &args.range),
                None,
            )
            .await
        }
        crate::cli::MicrosoftExcelRangesAction::Update(args) => {
            range_update(client, ctx, &args).await
        }
        crate::cli::MicrosoftExcelRangesAction::Clear(args) => {
            range_clear(client, ctx, &args).await
        }
    }
}

async fn range_update(
    client: &ApiClient,
    ctx: &Context,
    args: &MicrosoftExcelRangeUpdate,
) -> Result<Value, AppError> {
    let operation = "excel.ranges.update";
    let mut body = Map::new();
    add_matrix(&mut body, "values", args.values.as_deref(), operation)?;
    add_matrix(&mut body, "formulas", args.formulas.as_deref(), operation)?;
    add_matrix(
        &mut body,
        "numberFormat",
        args.number_format.as_deref(),
        operation,
    )?;
    if body.is_empty() {
        return Err(AppError::invalid_input(
            SERVICE,
            operation,
            "provide at least one of --values, --formulas, or --number-format",
        ));
    }
    request(
        client,
        ctx,
        operation,
        Method::PATCH,
        &range_path(
            ctx,
            &args.target.workbook,
            &args.target.worksheet,
            &args.target.range,
        ),
        Some(Value::Object(body)),
    )
    .await
}

async fn range_clear(
    client: &ApiClient,
    ctx: &Context,
    args: &MicrosoftExcelRangeClear,
) -> Result<Value, AppError> {
    let operation = "excel.ranges.clear";
    let apply_to = args.apply_to.to_ascii_lowercase();
    if !matches!(apply_to.as_str(), "all" | "formats" | "contents") {
        return Err(AppError::invalid_input(
            SERVICE,
            operation,
            "--apply-to must be All, Formats, or Contents",
        ));
    }
    request(
        client,
        ctx,
        operation,
        Method::POST,
        &format!(
            "{}/clear",
            range_path(
                ctx,
                &args.target.workbook,
                &args.target.worksheet,
                &args.target.range,
            )
        ),
        Some(json!({"applyTo": canonical_apply_to(&apply_to)})),
    )
    .await
}

async fn tables(
    client: &ApiClient,
    ctx: &Context,
    command: crate::cli::MicrosoftExcelTablesCommand,
) -> Result<Value, AppError> {
    match command.action {
        MicrosoftExcelTablesAction::List(args) => {
            collection(
                client,
                ctx,
                "excel.tables.list",
                &workbook_path(ctx, &args.workbook, "tables"),
                args.list.limit,
            )
            .await
        }
        MicrosoftExcelTablesAction::Create(args) => table_create(client, ctx, &args).await,
        MicrosoftExcelTablesAction::Delete(args) => {
            request(
                client,
                ctx,
                "excel.tables.delete",
                Method::DELETE,
                &table_path(ctx, &args.workbook, &args.table),
                None,
            )
            .await
        }
        MicrosoftExcelTablesAction::Rows(command) => match command {
            crate::cli::MicrosoftExcelTableRowsCommand::List(args) => {
                collection(
                    client,
                    ctx,
                    "excel.tables.rows.list",
                    &format!(
                        "{}/rows",
                        table_path(ctx, &args.target.workbook, &args.target.table)
                    ),
                    args.list.limit,
                )
                .await
            }
            crate::cli::MicrosoftExcelTableRowsCommand::Append(args) => {
                table_rows_append(client, ctx, &args).await
            }
        },
    }
}

async fn table_create(
    client: &ApiClient,
    ctx: &Context,
    args: &MicrosoftExcelTableCreate,
) -> Result<Value, AppError> {
    request(
        client,
        ctx,
        "excel.tables.create",
        Method::POST,
        &format!(
            "{}/tables/add",
            worksheet_path(ctx, &args.workbook, &args.worksheet)
        ),
        Some(json!({"address": args.range, "hasHeaders": args.has_headers})),
    )
    .await
}

async fn table_rows_append(
    client: &ApiClient,
    ctx: &Context,
    args: &MicrosoftExcelTableRowsAppend,
) -> Result<Value, AppError> {
    let operation = "excel.tables.rows.append";
    let values = input::read_json_arg(SERVICE, operation, Some(&args.values))?;
    if !values.is_array()
        || values
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| !row.is_array()))
    {
        return Err(AppError::invalid_input(
            SERVICE,
            operation,
            "--values must be a JSON array of row arrays",
        ));
    }
    request(
        client,
        ctx,
        operation,
        Method::POST,
        &format!(
            "{}/rows/add",
            table_path(ctx, &args.target.workbook, &args.target.table)
        ),
        Some(json!({"values": values})),
    )
    .await
}

fn add_matrix(
    body: &mut Map<String, Value>,
    key: &str,
    raw: Option<&str>,
    operation: &'static str,
) -> Result<(), AppError> {
    let Some(raw) = raw else {
        return Ok(());
    };
    let value = input::read_json_arg(SERVICE, operation, Some(raw))?;
    if !value.is_array()
        || value
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| !row.is_array()))
    {
        return Err(AppError::invalid_input(
            SERVICE,
            operation,
            format!("--{key} must be a JSON matrix (array of row arrays)"),
        ));
    }
    body.insert(key.to_string(), value);
    Ok(())
}

fn ensure_delegated(ctx: &Context, operation: &'static str) -> Result<(), AppError> {
    if ctx.profile().auth_type.as_deref() == Some("microsoft_delegated") {
        return Ok(());
    }
    Err(AppError::unsupported_auth(
        SERVICE,
        operation,
        "Microsoft Graph workbook operations require a microsoft_delegated profile with Files.ReadWrite access",
        Some(json!({
            "required_auth_type": "microsoft_delegated",
            "required_scope": "Files.ReadWrite",
        })),
    ))
}

fn workbook_path(ctx: &Context, target: &MicrosoftWorkbookTarget, suffix: &str) -> String {
    format!("{}/workbook/{suffix}", workbook_root(ctx, target))
}

fn worksheet_path(ctx: &Context, target: &MicrosoftWorkbookTarget, worksheet: &str) -> String {
    format!(
        "{}/workbook/worksheets/{}",
        workbook_root(ctx, target),
        enc(worksheet)
    )
}

fn range_path(
    ctx: &Context,
    target: &MicrosoftWorkbookTarget,
    worksheet: &str,
    range: &str,
) -> String {
    format!(
        "{}/range(address='{}')",
        worksheet_path(ctx, target, worksheet),
        enc(&odata_escape(range))
    )
}

fn table_path(ctx: &Context, target: &MicrosoftWorkbookTarget, table: &str) -> String {
    format!(
        "{}/workbook/tables/{}",
        workbook_root(ctx, target),
        enc(table)
    )
}

fn workbook_root(ctx: &Context, target: &MicrosoftWorkbookTarget) -> String {
    let drive = drive_root(ctx, target);

    if let Some(item_id) = target.item_id.as_deref() {
        return format!("{drive}/items/{}", enc(item_id));
    }
    let path = target.path.as_deref().unwrap_or_default();
    format!("{drive}/root:/{}:", encode_drive_path(path))
}

fn drive_root(ctx: &Context, target: &MicrosoftWorkbookTarget) -> String {
    if let Some(drive_id) = target.drive_id.as_deref() {
        return format!("drives/{}", enc(drive_id));
    }
    target
        .user_id
        .as_deref()
        .or(ctx.profile().user_id.as_deref())
        .map(|user_id| format!("users/{}/drive", enc(user_id)))
        .unwrap_or_else(|| "me/drive".to_string())
}

fn encode_drive_path(path: &str) -> String {
    path.trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(enc)
        .collect::<Vec<_>>()
        .join("/")
}

fn canonical_apply_to(value: &str) -> &'static str {
    match value {
        "all" => "All",
        "formats" => "Formats",
        _ => "Contents",
    }
}

fn enc(value: &str) -> String {
    urlencoding::encode(value).into_owned()
}

fn odata_escape(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Context, Profile};

    fn target() -> MicrosoftWorkbookTarget {
        MicrosoftWorkbookTarget {
            item_id: Some("item/1".to_string()),
            path: None,
            drive_id: Some("drive id".to_string()),
            user_id: None,
        }
    }

    fn context() -> Context {
        Context {
            profile: Profile::default(),
            secrets_file: Default::default(),
            key_file: Default::default(),
        }
    }

    #[test]
    fn workbook_paths_support_drive_item_and_odata_encoding() {
        let target = target();
        assert_eq!(
            range_path(&context(), &target, "Q1 plan", "A1:B2"),
            "drives/drive%20id/items/item%2F1/workbook/worksheets/Q1%20plan/range(address='A1%3AB2')"
        );
    }

    #[test]
    fn path_targets_preserve_each_drive_segment() {
        let target = MicrosoftWorkbookTarget {
            item_id: None,
            path: Some("Reports/Q1 plan.xlsx".to_string()),
            drive_id: Some("drive".to_string()),
            user_id: None,
        };
        assert_eq!(
            workbook_path(&context(), &target, "worksheets"),
            "drives/drive/root:/Reports/Q1%20plan.xlsx:/workbook/worksheets"
        );
    }

    #[test]
    fn range_addresses_escape_odata_quotes_before_url_encoding() {
        let target = target();
        assert_eq!(
            range_path(&context(), &target, "Sheet'Name", "'Sheet'!A1"),
            "drives/drive%20id/items/item%2F1/workbook/worksheets/Sheet%27Name/range(address='%27%27Sheet%27%27%21A1')"
        );
    }

    #[test]
    fn app_profiles_are_rejected_before_requests() {
        let ctx = Context {
            profile: Profile {
                auth_type: Some("microsoft_client_credentials".to_string()),
                ..Default::default()
            },
            secrets_file: Default::default(),
            key_file: Default::default(),
        };
        let error = ensure_delegated(&ctx, "excel").expect_err("app auth is unsupported");
        assert_eq!(error.code, "unsupported_auth");
        assert!(error.message.contains("microsoft_delegated"));
    }
}
