use reqwest::Method;
use serde_json::{json, Number, Value};

use crate::{
    cli::*,
    config::Context,
    error::AppError,
    http::ApiClient,
    input,
    services::{
        generic_request,
        shared::{enc, pipedrive_base, write_download, CtxProfile},
    },
};

pub(crate) async fn dispatch(
    client: &ApiClient,
    ctx: &Context,
    command: PipedriveCommand,
) -> Result<Value, AppError> {
    match command.resource {
        PipedriveResource::Leads(command) => leads(client, ctx, command).await,
        PipedriveResource::Persons(command) => persons(client, ctx, command).await,
        PipedriveResource::Organizations(command) => organizations(client, ctx, command).await,
        PipedriveResource::Deals(command) => deals(client, ctx, command).await,
        PipedriveResource::Labels(command) => labels(client, ctx, command).await,
        PipedriveResource::Activities(command) => activities(client, ctx, command).await,
        PipedriveResource::Notes(command) => notes(client, ctx, command).await,
        PipedriveResource::Files(command) => files(client, ctx, command).await,
        PipedriveResource::Fields(command) => fields(client, ctx, command).await,
        PipedriveResource::Users(command) => users(client, ctx, command).await,
        PipedriveResource::Pipelines(command) => match command.action {
            PipedrivePipelinesAction::List(args) => {
                sorted_list(
                    client,
                    ctx,
                    "pipelines.list",
                    "/api/v2/pipelines",
                    Query::new(),
                    args,
                )
                .await
            }
            PipedrivePipelinesAction::Get(args) => {
                get(client, ctx, "pipelines.get", "/api/v2/pipelines", &args.id).await
            }
        },
        PipedriveResource::Stages(command) => match command.action {
            PipedriveStagesAction::List(args) => {
                let mut query = Query::new();
                query.push("pipeline_id", args.pipeline_id.as_deref());
                sorted_list(
                    client,
                    ctx,
                    "stages.list",
                    "/api/v2/stages",
                    query,
                    args.list,
                )
                .await
            }
            PipedriveStagesAction::Get(args) => {
                get(client, ctx, "stages.get", "/api/v2/stages", &args.id).await
            }
        },
        PipedriveResource::Mailbox(command) => mailbox(client, ctx, command).await,
        PipedriveResource::Request(args) => {
            generic_request::dispatch(
                client,
                ctx,
                "pipedrive",
                pipedrive_base(ctx.profile()),
                args,
            )
            .await
        }
    }
}

async fn leads(
    client: &ApiClient,
    ctx: &Context,
    command: PipedriveLeadsCommand,
) -> Result<Value, AppError> {
    match command.action {
        PipedriveLeadsAction::List(args) => {
            let mut query = Query::new();
            query.push("owner_id", args.owner_id.as_deref());
            query.push("person_id", args.person_id.as_deref());
            query.push("organization_id", args.organization_id.as_deref());
            query.push("filter_id", args.filter_id.as_deref());
            query.push("updated_since", args.updated_since.as_deref());
            query.push("sort", args.sort.as_deref());
            let path = if args.archived {
                "/v1/leads/archived"
            } else {
                "/v1/leads"
            };
            list_v1(client, ctx, "leads.list", path, query, args.limit).await
        }
        PipedriveLeadsAction::Search(args) => {
            let mut query = Query::new();
            query.push_value("term", &args.term);
            query.push("fields", args.fields.as_deref());
            query.push_bool("exact_match", args.exact_match);
            query.push("person_id", args.person_id.as_deref());
            query.push("organization_id", args.organization_id.as_deref());
            search_v2(
                client,
                ctx,
                "leads.search",
                "/api/v2/leads/search",
                query,
                args.limit,
            )
            .await
        }
        PipedriveLeadsAction::Get(args) => {
            get(client, ctx, "leads.get", "/v1/leads", &args.id).await
        }
        PipedriveLeadsAction::Create(args) => {
            let body = lead_create_body(args)?;
            request_json(
                client,
                ctx,
                "leads.create",
                Method::POST,
                "/v1/leads",
                Some(body),
            )
            .await
        }
        PipedriveLeadsAction::Update(args) => {
            let id = args.id.clone();
            let labels = args.labels.clone();
            let body = lead_update_body(args)?;
            update_record(
                client,
                ctx,
                "leads.update",
                &format!("/v1/leads/{}", enc(&id)),
                labels,
                body,
            )
            .await
        }
        PipedriveLeadsAction::Delete(args) => {
            delete(client, ctx, "leads.delete", "/v1/leads", &args.id).await
        }
        PipedriveLeadsAction::Convert(args) => {
            let id = args.id.clone();
            let body = input::read_json_arg("pipedrive", "leads.convert", args.json.as_deref())?;
            request_json(
                client,
                ctx,
                "leads.convert",
                Method::POST,
                &format!("/api/v2/leads/{}/convert/deal", enc(&id)),
                Some(body),
            )
            .await
        }
    }
}

async fn persons(
    client: &ApiClient,
    ctx: &Context,
    command: PipedrivePersonsCommand,
) -> Result<Value, AppError> {
    match command.action {
        PipedrivePersonsAction::List(args) => {
            let mut query = Query::new();
            query.push("filter_id", args.filter_id.as_deref());
            query.push("ids", args.ids.as_deref());
            query.push("owner_id", args.owner_id.as_deref());
            query.push("org_id", args.org_id.as_deref());
            query.push("deal_id", args.deal_id.as_deref());
            query.push("updated_since", args.updated_since.as_deref());
            query.push("updated_until", args.updated_until.as_deref());
            query.push("sort_by", args.sort_by.as_deref());
            query.push(
                "sort_direction",
                sort_direction(args.sort_direction.as_ref()),
            );
            query.push_bool("include_labels", args.include_labels);
            list_v2(
                client,
                ctx,
                "persons.list",
                "/api/v2/persons",
                query,
                args.limit,
            )
            .await
        }
        PipedrivePersonsAction::Search(args) => {
            let mut query = Query::new();
            query.push_value("term", &args.term);
            query.push("fields", args.fields.as_deref());
            query.push_bool("exact_match", args.exact_match);
            query.push("organization_id", args.organization_id.as_deref());
            search_v2(
                client,
                ctx,
                "persons.search",
                "/api/v2/persons/search",
                query,
                args.limit,
            )
            .await
        }
        PipedrivePersonsAction::Get(args) => {
            let mut query = Query::new();
            query.push_bool("include_labels", args.include_labels);
            get_with_query(
                client,
                ctx,
                "persons.get",
                "/api/v2/persons",
                &args.id,
                query,
            )
            .await
        }
        PipedrivePersonsAction::View(args) => {
            associated_view(client, ctx, "persons", "person_id", args).await
        }
        PipedrivePersonsAction::Activities(args) => {
            associated_activities(client, ctx, "persons.activities", "person_id", args).await
        }
        PipedrivePersonsAction::Notes(args) => {
            associated_notes(client, ctx, "persons.notes", "person_id", args).await
        }
        PipedrivePersonsAction::MailMessages(args) => {
            associated_mail(client, ctx, "persons.mail_messages", "persons", args).await
        }
        PipedrivePersonsAction::Create(args) => {
            let body = person_create_body(args)?;
            request_json(
                client,
                ctx,
                "persons.create",
                Method::POST,
                "/api/v2/persons",
                Some(body),
            )
            .await
        }
        PipedrivePersonsAction::Update(args) => {
            let id = args.id.clone();
            let labels = args.labels.clone();
            let body = person_update_body(args)?;
            update_record(
                client,
                ctx,
                "persons.update",
                &format!("/api/v2/persons/{}", enc(&id)),
                labels,
                body,
            )
            .await
        }
        PipedrivePersonsAction::Delete(args) => {
            delete(client, ctx, "persons.delete", "/api/v2/persons", &args.id).await
        }
        PipedrivePersonsAction::Merge(args) => {
            merge(client, ctx, "persons.merge", "persons", args).await
        }
    }
}

async fn organizations(
    client: &ApiClient,
    ctx: &Context,
    command: PipedriveOrganizationsCommand,
) -> Result<Value, AppError> {
    match command.action {
        PipedriveOrganizationsAction::List(args) => {
            let mut query = Query::new();
            query.push("filter_id", args.filter_id.as_deref());
            query.push("ids", args.ids.as_deref());
            query.push("owner_id", args.owner_id.as_deref());
            query.push("updated_since", args.updated_since.as_deref());
            query.push("updated_until", args.updated_until.as_deref());
            query.push("sort_by", args.sort_by.as_deref());
            query.push(
                "sort_direction",
                sort_direction(args.sort_direction.as_ref()),
            );
            query.push_bool("include_labels", args.include_labels);
            list_v2(
                client,
                ctx,
                "organizations.list",
                "/api/v2/organizations",
                query,
                args.limit,
            )
            .await
        }
        PipedriveOrganizationsAction::Search(args) => {
            let mut query = Query::new();
            query.push_value("term", &args.term);
            query.push("fields", args.fields.as_deref());
            query.push_bool("exact_match", args.exact_match);
            search_v2(
                client,
                ctx,
                "organizations.search",
                "/api/v2/organizations/search",
                query,
                args.limit,
            )
            .await
        }
        PipedriveOrganizationsAction::Get(args) => {
            let mut query = Query::new();
            query.push_bool("include_labels", args.include_labels);
            get_with_query(
                client,
                ctx,
                "organizations.get",
                "/api/v2/organizations",
                &args.id,
                query,
            )
            .await
        }
        PipedriveOrganizationsAction::View(args) => {
            associated_view(client, ctx, "organizations", "org_id", args).await
        }
        PipedriveOrganizationsAction::Activities(args) => {
            associated_activities(client, ctx, "organizations.activities", "org_id", args).await
        }
        PipedriveOrganizationsAction::Notes(args) => {
            associated_notes(client, ctx, "organizations.notes", "org_id", args).await
        }
        PipedriveOrganizationsAction::MailMessages(args) => {
            associated_mail(
                client,
                ctx,
                "organizations.mail_messages",
                "organizations",
                args,
            )
            .await
        }
        PipedriveOrganizationsAction::Create(args) => {
            let body = organization_create_body(args)?;
            request_json(
                client,
                ctx,
                "organizations.create",
                Method::POST,
                "/api/v2/organizations",
                Some(body),
            )
            .await
        }
        PipedriveOrganizationsAction::Update(args) => {
            let id = args.id.clone();
            let labels = args.labels.clone();
            let body = organization_update_body(args)?;
            update_record(
                client,
                ctx,
                "organizations.update",
                &format!("/api/v2/organizations/{}", enc(&id)),
                labels,
                body,
            )
            .await
        }
        PipedriveOrganizationsAction::Delete(args) => {
            delete(
                client,
                ctx,
                "organizations.delete",
                "/api/v2/organizations",
                &args.id,
            )
            .await
        }
        PipedriveOrganizationsAction::Merge(args) => {
            merge(client, ctx, "organizations.merge", "organizations", args).await
        }
    }
}

async fn deals(
    client: &ApiClient,
    ctx: &Context,
    command: PipedriveDealsCommand,
) -> Result<Value, AppError> {
    match command.action {
        PipedriveDealsAction::List(args) => {
            let mut query = Query::new();
            query.push("filter_id", args.filter_id.as_deref());
            query.push("ids", args.ids.as_deref());
            query.push("owner_id", args.owner_id.as_deref());
            query.push("person_id", args.person_id.as_deref());
            query.push("org_id", args.org_id.as_deref());
            query.push("pipeline_id", args.pipeline_id.as_deref());
            query.push("stage_id", args.stage_id.as_deref());
            query.push("status", deal_status(args.status.as_ref()));
            query.push("updated_since", args.updated_since.as_deref());
            query.push("updated_until", args.updated_until.as_deref());
            query.push("sort_by", args.sort_by.as_deref());
            query.push(
                "sort_direction",
                sort_direction(args.sort_direction.as_ref()),
            );
            query.push_bool("include_labels", args.include_labels);
            list_v2(
                client,
                ctx,
                "deals.list",
                "/api/v2/deals",
                query,
                args.limit,
            )
            .await
        }
        PipedriveDealsAction::Search(args) => {
            let mut query = Query::new();
            query.push_value("term", &args.term);
            query.push("fields", args.fields.as_deref());
            query.push_bool("exact_match", args.exact_match);
            query.push("person_id", args.person_id.as_deref());
            query.push("organization_id", args.organization_id.as_deref());
            query.push("status", search_deal_status(args.status.as_ref()));
            search_v2(
                client,
                ctx,
                "deals.search",
                "/api/v2/deals/search",
                query,
                args.limit,
            )
            .await
        }
        PipedriveDealsAction::Get(args) => {
            let mut query = Query::new();
            query.push_bool("include_labels", args.include_labels);
            get_with_query(client, ctx, "deals.get", "/api/v2/deals", &args.id, query).await
        }
        PipedriveDealsAction::View(args) => {
            associated_view(client, ctx, "deals", "deal_id", args).await
        }
        PipedriveDealsAction::Activities(args) => {
            associated_activities(client, ctx, "deals.activities", "deal_id", args).await
        }
        PipedriveDealsAction::Notes(args) => {
            associated_notes(client, ctx, "deals.notes", "deal_id", args).await
        }
        PipedriveDealsAction::MailMessages(args) => {
            associated_mail(client, ctx, "deals.mail_messages", "deals", args).await
        }
        PipedriveDealsAction::Flow(args) => {
            list_v1(
                client,
                ctx,
                "deals.flow",
                &format!("/v1/deals/{}/flow", enc(&args.id)),
                Query::new(),
                args.limit,
            )
            .await
        }
        PipedriveDealsAction::Create(args) => {
            let body = deal_create_body(args)?;
            request_json(
                client,
                ctx,
                "deals.create",
                Method::POST,
                "/api/v2/deals",
                Some(body),
            )
            .await
        }
        PipedriveDealsAction::Update(args) => {
            let id = args.id.clone();
            let labels = args.labels.clone();
            let body = deal_update_body(args)?;
            update_record(
                client,
                ctx,
                "deals.update",
                &format!("/api/v2/deals/{}", enc(&id)),
                labels,
                body,
            )
            .await
        }
        PipedriveDealsAction::Delete(args) => {
            delete(client, ctx, "deals.delete", "/api/v2/deals", &args.id).await
        }
        PipedriveDealsAction::Merge(args) => merge(client, ctx, "deals.merge", "deals", args).await,
    }
}

async fn activities(
    client: &ApiClient,
    ctx: &Context,
    command: PipedriveActivitiesCommand,
) -> Result<Value, AppError> {
    match command.action {
        PipedriveActivitiesAction::List(args) => {
            let mut query = Query::new();
            query.push("filter_id", args.filter_id.as_deref());
            query.push("ids", args.ids.as_deref());
            query.push("owner_id", args.owner_id.as_deref());
            query.push("deal_id", args.deal_id.as_deref());
            query.push("lead_id", args.lead_id.as_deref());
            query.push("person_id", args.person_id.as_deref());
            query.push("org_id", args.org_id.as_deref());
            query.push_optional_bool("done", args.done);
            query.push("updated_since", args.updated_since.as_deref());
            query.push("updated_until", args.updated_until.as_deref());
            query.push("sort_by", args.sort_by.as_deref());
            query.push(
                "sort_direction",
                sort_direction(args.sort_direction.as_ref()),
            );
            if args.include_attendees {
                query.push_value("include_fields", "attendees");
            }
            list_v2(
                client,
                ctx,
                "activities.list",
                "/api/v2/activities",
                query,
                args.limit,
            )
            .await
        }
        PipedriveActivitiesAction::Get(args) => {
            get(
                client,
                ctx,
                "activities.get",
                "/api/v2/activities",
                &args.id,
            )
            .await
        }
        PipedriveActivitiesAction::Create(args) => {
            let body = activity_create_body(*args)?;
            request_json(
                client,
                ctx,
                "activities.create",
                Method::POST,
                "/api/v2/activities",
                Some(body),
            )
            .await
        }
        PipedriveActivitiesAction::Update(args) => {
            let id = args.id.clone();
            let body = activity_update_body(*args)?;
            request_json(
                client,
                ctx,
                "activities.update",
                Method::PATCH,
                &format!("/api/v2/activities/{}", enc(&id)),
                Some(body),
            )
            .await
        }
        PipedriveActivitiesAction::Delete(args) => {
            delete(
                client,
                ctx,
                "activities.delete",
                "/api/v2/activities",
                &args.id,
            )
            .await
        }
    }
}

/// PATCH a record, first resolving any label additions or removals against the labels
/// it currently has.
async fn update_record(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    path: &str,
    labels: PipedriveLabelEdits,
    mut body: Value,
) -> Result<Value, AppError> {
    if labels.add_label_ids.is_some() || labels.remove_label_ids.is_some() {
        let record = request_json(client, ctx, operation, Method::GET, path, None).await?;
        let current = current_label_ids(&record, operation)?;
        let add = labels.add_label_ids.as_deref().map(parse_csv).transpose()?;
        let remove = labels
            .remove_label_ids
            .as_deref()
            .map(parse_csv)
            .transpose()?;
        input::ensure_object(&mut body).insert(
            "label_ids".to_string(),
            Value::Array(merge_label_ids(
                current,
                &add.unwrap_or_default(),
                &remove.unwrap_or_default(),
            )),
        );
    }
    request_json(client, ctx, operation, Method::PATCH, path, Some(body)).await
}

/// The labels a record has now. A record without the `label_ids` key is refused rather
/// than read as unlabelled: writing back an empty list would erase its real labels.
/// Pipedrive sends a literal `null` for a record with no labels.
fn current_label_ids(record: &Value, operation: &'static str) -> Result<Vec<Value>, AppError> {
    match record.pointer("/data/label_ids") {
        Some(Value::Array(ids)) => Ok(ids.clone()),
        Some(Value::Null) => Ok(Vec::new()),
        Some(_) => Err(AppError::internal(
            "pipedrive",
            operation,
            "the record's label_ids is not a list; refusing to rewrite its labels",
        )),
        None => Err(AppError::internal(
            "pipedrive",
            operation,
            "the record came back without label_ids; refusing to rewrite its labels",
        )),
    }
}

fn merge_label_ids(current: Vec<Value>, add: &[Value], remove: &[Value]) -> Vec<Value> {
    // Deal/person/organization label IDs are numbers and lead label IDs are UUIDs, and a
    // CSV "5" parses to a number, so compare by rendered text.
    let key = |id: &Value| match id {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    };
    let removed = |id: &Value| remove.iter().any(|other| key(other) == key(id));
    let mut ids: Vec<Value> = current.into_iter().filter(|id| !removed(id)).collect();
    for id in add {
        if !removed(id) && !ids.iter().any(|existing| key(existing) == key(id)) {
            ids.push(id.clone());
        }
    }
    ids
}

/// Pipedrive merges `id` into `merge_with_id`: the latter remains and its data wins.
async fn merge(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    collection: &str,
    args: PipedriveMergeArgs,
) -> Result<Value, AppError> {
    request_json(
        client,
        ctx,
        operation,
        Method::PUT,
        &format!("/v1/{collection}/{}/merge", enc(&args.id)),
        Some(json!({ "merge_with_id": id_value(&args.merge_with_id) })),
    )
    .await
}

async fn users(
    client: &ApiClient,
    ctx: &Context,
    command: PipedriveUsersCommand,
) -> Result<Value, AppError> {
    match command.action {
        PipedriveUsersAction::List => {
            let mut users =
                request_json(client, ctx, "users.list", Method::GET, "/v1/users", None).await?;
            // Pipedrive returns every user in one unpaginated response, so the list is
            // complete; say so instead of leaving pagination status "unknown".
            if let Some(object) = users.as_object_mut() {
                object.insert("truncated".to_string(), Value::Bool(false));
            }
            Ok(users)
        }
        PipedriveUsersAction::Get(args) => {
            get(client, ctx, "users.get", "/v1/users", &args.id).await
        }
        PipedriveUsersAction::Me => {
            request_json(client, ctx, "users.me", Method::GET, "/v1/users/me", None).await
        }
        PipedriveUsersAction::Find(args) => {
            let mut query = Query::new();
            query.push_value("term", &args.term);
            if args.search_by_email {
                query.push_value("search_by_email", "1");
            }
            let mut path = "/v1/users/find".to_string();
            query.append_to(&mut path);
            request_json(client, ctx, "users.find", Method::GET, &path, None).await
        }
    }
}

async fn sorted_list(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    path: &str,
    mut query: Query,
    args: PipedriveSortedList,
) -> Result<Value, AppError> {
    query.push("sort_by", args.sort_by.as_deref());
    query.push(
        "sort_direction",
        sort_direction(args.sort_direction.as_ref()),
    );
    list_v2(client, ctx, operation, path, query, args.limit).await
}

async fn fields(
    client: &ApiClient,
    ctx: &Context,
    command: PipedriveFieldsCommand,
) -> Result<Value, AppError> {
    let (path, (list_operation, get_operation), command) = match command.resource {
        PipedriveFieldResource::Deals(command) => (
            "/api/v2/dealFields",
            ("fields.deals.list", "fields.deals.get"),
            command,
        ),
        PipedriveFieldResource::Persons(command) => (
            "/api/v2/personFields",
            ("fields.persons.list", "fields.persons.get"),
            command,
        ),
        PipedriveFieldResource::Organizations(command) => (
            "/api/v2/organizationFields",
            ("fields.organizations.list", "fields.organizations.get"),
            command,
        ),
        PipedriveFieldResource::Activities(command) => (
            "/api/v2/activityFields",
            ("fields.activities.list", "fields.activities.get"),
            command,
        ),
    };
    match command.action {
        PipedriveFieldAction::List(args) => {
            list_v2(client, ctx, list_operation, path, Query::new(), args.limit).await
        }
        PipedriveFieldAction::Get(args) => {
            get(client, ctx, get_operation, path, &args.field_code).await
        }
    }
}

async fn files(
    client: &ApiClient,
    ctx: &Context,
    command: PipedriveFilesCommand,
) -> Result<Value, AppError> {
    match command.action {
        PipedriveFilesAction::List(args) => {
            let mut query = Query::new();
            query.push("sort", args.sort.as_deref());
            list_v1(
                client,
                ctx,
                "files.list",
                &file_list_path(&args),
                query,
                args.limit,
            )
            .await
        }
        PipedriveFilesAction::Get(args) => {
            get(client, ctx, "files.get", "/v1/files", &args.id).await
        }
        PipedriveFilesAction::Download(args) => {
            let operation = "files.download";
            let url = format!(
                "{}/v1/files/{}/download",
                pipedrive_base(ctx.profile()),
                enc(&args.id)
            );
            let bytes = client
                .download("pipedrive", operation, ctx.profile(), url)
                .await?;
            let mut result = write_download("pipedrive", operation, &args.output, &bytes)?;
            result["file_id"] = json!(args.id);
            Ok(result)
        }
    }
}

/// Pipedrive lists a record's files under the record itself, not through a filter on
/// `/v1/files`, so the scope picks the endpoint.
fn file_list_path(args: &PipedriveFileList) -> String {
    [
        ("deals", &args.deal_id),
        ("persons", &args.person_id),
        ("organizations", &args.org_id),
    ]
    .into_iter()
    .find_map(|(collection, id)| {
        id.as_deref()
            .map(|id| format!("/v1/{collection}/{}/files", enc(id)))
    })
    .unwrap_or_else(|| "/v1/files".to_string())
}

async fn notes(
    client: &ApiClient,
    ctx: &Context,
    command: PipedriveNotesCommand,
) -> Result<Value, AppError> {
    match command.action {
        PipedriveNotesAction::List(args) => {
            let mut query = Query::new();
            query.push("user_id", args.user_id.as_deref());
            query.push("lead_id", args.lead_id.as_deref());
            query.push("deal_id", args.deal_id.as_deref());
            query.push("person_id", args.person_id.as_deref());
            query.push("org_id", args.org_id.as_deref());
            query.push("sort", args.sort.as_deref());
            query.push("start_date", args.start_date.as_deref());
            query.push("end_date", args.end_date.as_deref());
            query.push("updated_since", args.updated_since.as_deref());
            list_v1(client, ctx, "notes.list", "/v1/notes", query, args.limit).await
        }
        PipedriveNotesAction::Get(args) => {
            get(client, ctx, "notes.get", "/v1/notes", &args.id).await
        }
        PipedriveNotesAction::Create(args) => {
            let body = note_create_body(args)?;
            request_json(
                client,
                ctx,
                "notes.create",
                Method::POST,
                "/v1/notes",
                Some(body),
            )
            .await
        }
        PipedriveNotesAction::Update(args) => {
            let id = args.id.clone();
            let body = note_update_body(args)?;
            request_json(
                client,
                ctx,
                "notes.update",
                Method::PUT,
                &format!("/v1/notes/{}", enc(&id)),
                Some(body),
            )
            .await
        }
        PipedriveNotesAction::Delete(args) => {
            delete(client, ctx, "notes.delete", "/v1/notes", &args.id).await
        }
    }
}

async fn mailbox(
    client: &ApiClient,
    ctx: &Context,
    command: PipedriveMailboxCommand,
) -> Result<Value, AppError> {
    match command.resource {
        PipedriveMailboxResource::Messages(command) => match command.action {
            PipedriveMailboxMessagesAction::Get(args) => {
                let mut query = Query::new();
                if args.include_body {
                    query.push_value("include_body", "1");
                }
                get_with_query(
                    client,
                    ctx,
                    "mailbox.messages.get",
                    "/v1/mailbox/mailMessages",
                    &args.id,
                    query,
                )
                .await
            }
        },
        PipedriveMailboxResource::Threads(command) => match command.action {
            PipedriveMailboxThreadsAction::List(args) => {
                let mut query = Query::new();
                query.push_value("folder", mail_folder(&args.folder));
                list_v1(
                    client,
                    ctx,
                    "mailbox.threads.list",
                    "/v1/mailbox/mailThreads",
                    query,
                    args.limit,
                )
                .await
            }
            PipedriveMailboxThreadsAction::Get(args) => {
                get(
                    client,
                    ctx,
                    "mailbox.threads.get",
                    "/v1/mailbox/mailThreads",
                    &args.id,
                )
                .await
            }
            PipedriveMailboxThreadsAction::Messages(args) => {
                request_json(
                    client,
                    ctx,
                    "mailbox.threads.messages",
                    Method::GET,
                    &format!("/v1/mailbox/mailThreads/{}/mailMessages", enc(&args.id)),
                    None,
                )
                .await
            }
        },
    }
}

async fn associated_view(
    client: &ApiClient,
    ctx: &Context,
    resource: &'static str,
    filter_key: &'static str,
    args: PipedriveAssociatedView,
) -> Result<Value, AppError> {
    let mut resource_query = Query::new();
    resource_query.push_bool("include_labels", args.include_labels);
    let record = get_with_query(
        client,
        ctx,
        match resource {
            "deals" => "deals.view",
            "persons" => "persons.view",
            _ => "organizations.view",
        },
        &format!("/api/v2/{resource}"),
        &args.id,
        resource_query,
    )
    .await?;
    let activities = associated_activities(
        client,
        ctx,
        "view.activities",
        filter_key,
        PipedriveAssociatedList {
            id: args.id.clone(),
            limit: args.limit,
        },
    )
    .await?;
    let notes = associated_notes(
        client,
        ctx,
        "view.notes",
        filter_key,
        PipedriveAssociatedList {
            id: args.id.clone(),
            limit: args.limit,
        },
    )
    .await?;
    let mut response = json!({
        "record": record,
        "activities": activities,
        "notes": notes,
    });
    if args.include_mail {
        let mail = associated_mail(
            client,
            ctx,
            "view.mail_messages",
            resource,
            PipedriveAssociatedList {
                id: args.id,
                limit: args.limit,
            },
        )
        .await?;
        input::ensure_object(&mut response).insert("mail_messages".to_string(), mail);
    }
    Ok(response)
}

async fn associated_activities(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    filter_key: &'static str,
    args: PipedriveAssociatedList,
) -> Result<Value, AppError> {
    let mut query = Query::new();
    query.push_value(filter_key, &args.id);
    list_v2(
        client,
        ctx,
        operation,
        "/api/v2/activities",
        query,
        args.limit,
    )
    .await
}

async fn associated_notes(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    filter_key: &'static str,
    args: PipedriveAssociatedList,
) -> Result<Value, AppError> {
    let mut query = Query::new();
    query.push_value(filter_key, &args.id);
    list_v1(client, ctx, operation, "/v1/notes", query, args.limit).await
}

async fn associated_mail(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    resource: &'static str,
    args: PipedriveAssociatedList,
) -> Result<Value, AppError> {
    list_v1(
        client,
        ctx,
        operation,
        &format!("/v1/{resource}/{}/mailMessages", enc(&args.id)),
        Query::new(),
        args.limit,
    )
    .await
}

async fn labels(
    client: &ApiClient,
    ctx: &Context,
    command: PipedriveLabelsCommand,
) -> Result<Value, AppError> {
    match command.resource {
        PipedriveLabelResource::Leads(command) => match command.action {
            PipedriveLeadLabelsAction::List => {
                request_json(
                    client,
                    ctx,
                    "labels.leads.list",
                    Method::GET,
                    "/v1/leadLabels",
                    None,
                )
                .await
            }
            PipedriveLeadLabelsAction::Create(args) => {
                request_json(
                    client,
                    ctx,
                    "labels.leads.create",
                    Method::POST,
                    "/v1/leadLabels",
                    Some(json!({ "name": args.name, "color": args.color })),
                )
                .await
            }
            PipedriveLeadLabelsAction::Update(args) => {
                let mut body = json!({});
                input::set_string(&mut body, "name", &args.name);
                input::set_string(&mut body, "color", &args.color);
                request_json(
                    client,
                    ctx,
                    "labels.leads.update",
                    Method::PATCH,
                    &format!("/v1/leadLabels/{}", enc(&args.id)),
                    Some(body),
                )
                .await
            }
            PipedriveLeadLabelsAction::Delete(args) => {
                delete(
                    client,
                    ctx,
                    "labels.leads.delete",
                    "/v1/leadLabels",
                    &args.id,
                )
                .await
            }
        },
        PipedriveLabelResource::Deals(command) => match command.action {
            PipedriveLabelListAction::List => {
                field_label_list(client, ctx, "labels.deals.list", "/api/v2/dealFields").await
            }
        },
        PipedriveLabelResource::Persons(command) => match command.action {
            PipedriveLabelListAction::List => {
                field_label_list(client, ctx, "labels.persons.list", "/api/v2/personFields").await
            }
        },
        PipedriveLabelResource::Organizations(command) => match command.action {
            PipedriveLabelListAction::List => {
                field_label_list(
                    client,
                    ctx,
                    "labels.organizations.list",
                    "/api/v2/organizationFields",
                )
                .await
            }
        },
    }
}

async fn field_label_list(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    path: &str,
) -> Result<Value, AppError> {
    let mut query = Query::new();
    query.push_value("include_option_labels", "true");
    let mut url = format!("{}{}", pipedrive_base(ctx.profile()), path);
    query.append_to(&mut url);
    let mut response = client
        .request(
            "pipedrive",
            operation,
            ctx.profile(),
            Method::GET,
            url,
            None,
        )
        .await?;
    let labels = extract_label_options(&response);
    input::ensure_object(&mut response).insert("data".to_string(), Value::Array(labels));
    Ok(response)
}

async fn get(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    collection_path: &str,
    id: &str,
) -> Result<Value, AppError> {
    get_with_query(client, ctx, operation, collection_path, id, Query::new()).await
}

async fn get_with_query(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    collection_path: &str,
    id: &str,
    query: Query,
) -> Result<Value, AppError> {
    let mut url = format!(
        "{}{}/{}",
        pipedrive_base(ctx.profile()),
        collection_path,
        enc(id)
    );
    query.append_to(&mut url);
    client
        .request(
            "pipedrive",
            operation,
            ctx.profile(),
            Method::GET,
            url,
            None,
        )
        .await
}

async fn delete(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    collection_path: &str,
    id: &str,
) -> Result<Value, AppError> {
    request_json(
        client,
        ctx,
        operation,
        Method::DELETE,
        &format!("{}/{}", collection_path, enc(id)),
        None,
    )
    .await
}

async fn request_json(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> Result<Value, AppError> {
    client
        .request(
            "pipedrive",
            operation,
            ctx.profile(),
            method,
            format!("{}{}", pipedrive_base(ctx.profile()), path),
            body,
        )
        .await
}

async fn list_v2(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    path: &str,
    mut query: Query,
    limit: u32,
) -> Result<Value, AppError> {
    paginate_cursor(client, ctx, operation, path, &mut query, limit, &["data"]).await
}

async fn search_v2(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    path: &str,
    mut query: Query,
    limit: u32,
) -> Result<Value, AppError> {
    paginate_cursor(
        client,
        ctx,
        operation,
        path,
        &mut query,
        limit,
        &["data", "items"],
    )
    .await
}

async fn paginate_cursor(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    path: &str,
    query: &mut Query,
    limit: u32,
    array_path: &[&str],
) -> Result<Value, AppError> {
    if limit == 0 {
        return Ok(empty_aggregate(array_path));
    }

    let mut cursor: Option<String> = None;
    let mut first_page = None;
    let mut values = Vec::new();

    loop {
        query.set("limit", page_limit(limit, values.len()).to_string());
        if let Some(cursor) = cursor.as_deref() {
            query.set("cursor", cursor.to_string());
        }
        let mut url = format!("{}{}", pipedrive_base(ctx.profile()), path);
        query.append_to(&mut url);
        let page = client
            .request(
                "pipedrive",
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
        cursor = page
            .pointer("/additional_data/next_cursor")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        for value in values_at(&page, array_path) {
            if values.len() >= limit as usize {
                break;
            }
            values.push(value);
        }
        if values.len() >= limit as usize {
            break;
        }
        if cursor.is_none() {
            break;
        }
    }

    finish_cursor_pagination(&mut first_page, cursor);
    Ok(aggregate_response(first_page, array_path, values))
}

async fn list_v1(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    path: &str,
    mut query: Query,
    limit: u32,
) -> Result<Value, AppError> {
    paginate_start(client, ctx, operation, path, &mut query, limit, &["data"]).await
}

async fn paginate_start(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    path: &str,
    query: &mut Query,
    limit: u32,
    array_path: &[&str],
) -> Result<Value, AppError> {
    if limit == 0 {
        return Ok(empty_aggregate(array_path));
    }

    let mut start = 0u64;
    let mut continuation: Option<u64>;
    let mut first_page = None;
    let mut values = Vec::new();

    loop {
        query.set("limit", page_limit(limit, values.len()).to_string());
        query.set("start", start.to_string());
        let mut url = format!("{}{}", pipedrive_base(ctx.profile()), path);
        query.append_to(&mut url);
        let page = client
            .request(
                "pipedrive",
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
        continuation = page
            .pointer("/additional_data/pagination/next_start")
            .and_then(Value::as_u64);
        for value in values_at(&page, array_path) {
            if values.len() >= limit as usize {
                break;
            }
            values.push(value);
        }
        if values.len() >= limit as usize {
            break;
        }
        match continuation {
            Some(value) => start = value,
            None => break,
        }
    }

    finish_start_pagination(&mut first_page, continuation);
    Ok(aggregate_response(first_page, array_path, values))
}

fn lead_create_body(args: PipedriveLeadWrite) -> Result<Value, AppError> {
    let mut body = input::read_json_arg("pipedrive", "leads.create", args.json.as_deref())?;
    input::set_string(&mut body, "title", &Some(args.title));
    set_id(&mut body, "person_id", args.person_id.as_deref());
    set_id(
        &mut body,
        "organization_id",
        args.organization_id.as_deref(),
    );
    set_csv(&mut body, "label_ids", args.label_ids.as_deref())?;
    Ok(body)
}

fn lead_update_body(args: PipedriveLeadUpdate) -> Result<Value, AppError> {
    let mut body = input::read_json_arg("pipedrive", "leads.update", args.json.as_deref())?;
    input::set_string(&mut body, "title", &args.title);
    set_id(&mut body, "person_id", args.person_id.as_deref());
    set_id(
        &mut body,
        "organization_id",
        args.organization_id.as_deref(),
    );
    set_csv(&mut body, "label_ids", args.label_ids.as_deref())?;
    Ok(body)
}

fn person_create_body(args: PipedrivePersonWrite) -> Result<Value, AppError> {
    let mut body = input::read_json_arg("pipedrive", "persons.create", args.json.as_deref())?;
    input::set_string(&mut body, "name", &Some(args.name));
    set_id(&mut body, "org_id", args.org_id.as_deref());
    set_contact_field(&mut body, "emails", args.email.as_deref());
    set_contact_field(&mut body, "phones", args.phone.as_deref());
    set_csv(&mut body, "label_ids", args.label_ids.as_deref())?;
    Ok(body)
}

fn person_update_body(args: PipedrivePersonUpdate) -> Result<Value, AppError> {
    let mut body = input::read_json_arg("pipedrive", "persons.update", args.json.as_deref())?;
    input::set_string(&mut body, "name", &args.name);
    set_id(&mut body, "org_id", args.org_id.as_deref());
    set_contact_field(&mut body, "emails", args.email.as_deref());
    set_contact_field(&mut body, "phones", args.phone.as_deref());
    set_csv(&mut body, "label_ids", args.label_ids.as_deref())?;
    Ok(body)
}

fn organization_create_body(args: PipedriveOrganizationWrite) -> Result<Value, AppError> {
    let mut body = input::read_json_arg("pipedrive", "organizations.create", args.json.as_deref())?;
    input::set_string(&mut body, "name", &Some(args.name));
    set_address(&mut body, args.address.as_deref());
    set_csv(&mut body, "label_ids", args.label_ids.as_deref())?;
    Ok(body)
}

fn organization_update_body(args: PipedriveOrganizationUpdate) -> Result<Value, AppError> {
    let mut body = input::read_json_arg("pipedrive", "organizations.update", args.json.as_deref())?;
    input::set_string(&mut body, "name", &args.name);
    set_address(&mut body, args.address.as_deref());
    set_csv(&mut body, "label_ids", args.label_ids.as_deref())?;
    Ok(body)
}

fn deal_create_body(args: PipedriveDealWrite) -> Result<Value, AppError> {
    let mut body = input::read_json_arg("pipedrive", "deals.create", args.json.as_deref())?;
    input::set_string(&mut body, "title", &Some(args.title));
    set_id(&mut body, "person_id", args.person_id.as_deref());
    set_id(&mut body, "org_id", args.org_id.as_deref());
    set_f64(&mut body, "value", args.value);
    input::set_string(&mut body, "currency", &args.currency);
    set_id(&mut body, "pipeline_id", args.pipeline_id.as_deref());
    set_id(&mut body, "stage_id", args.stage_id.as_deref());
    set_csv(&mut body, "label_ids", args.label_ids.as_deref())?;
    Ok(body)
}

fn deal_update_body(args: PipedriveDealUpdate) -> Result<Value, AppError> {
    let mut body = input::read_json_arg("pipedrive", "deals.update", args.json.as_deref())?;
    input::set_string(&mut body, "title", &args.title);
    set_id(&mut body, "person_id", args.person_id.as_deref());
    set_id(&mut body, "org_id", args.org_id.as_deref());
    set_f64(&mut body, "value", args.value);
    input::set_string(&mut body, "currency", &args.currency);
    set_id(&mut body, "pipeline_id", args.pipeline_id.as_deref());
    set_id(&mut body, "stage_id", args.stage_id.as_deref());
    set_csv(&mut body, "label_ids", args.label_ids.as_deref())?;
    Ok(body)
}

fn note_create_body(args: PipedriveNoteWrite) -> Result<Value, AppError> {
    let mut body = input::read_json_arg("pipedrive", "notes.create", args.json.as_deref())?;
    input::set_string(&mut body, "content", &Some(args.content));
    set_note_links(&mut body, args.links);
    Ok(body)
}

fn note_update_body(args: PipedriveNoteUpdate) -> Result<Value, AppError> {
    let mut body = input::read_json_arg("pipedrive", "notes.update", args.json.as_deref())?;
    input::set_string(&mut body, "content", &args.content);
    set_note_links(&mut body, args.links);
    Ok(body)
}

fn set_note_links(body: &mut Value, links: PipedriveNoteLinks) {
    set_id(body, "deal_id", links.deal_id.as_deref());
    set_id(body, "person_id", links.person_id.as_deref());
    set_id(body, "org_id", links.org_id.as_deref());
    // Lead IDs are UUIDs; Pipedrive rejects them as numbers.
    input::set_string(body, "lead_id", &links.lead_id);
}

fn activity_create_body(args: PipedriveActivityWrite) -> Result<Value, AppError> {
    let mut body = input::read_json_arg("pipedrive", "activities.create", args.json.as_deref())?;
    input::set_string(&mut body, "subject", &Some(args.subject));
    set_activity_fields(&mut body, args.fields);
    Ok(body)
}

fn activity_update_body(args: PipedriveActivityUpdate) -> Result<Value, AppError> {
    let mut body = input::read_json_arg("pipedrive", "activities.update", args.json.as_deref())?;
    input::set_string(&mut body, "subject", &args.subject);
    set_activity_fields(&mut body, args.fields);
    Ok(body)
}

fn set_activity_fields(body: &mut Value, fields: PipedriveActivityFields) {
    input::set_string(body, "type", &fields.activity_type);
    input::set_string(body, "due_date", &fields.due_date);
    input::set_string(body, "due_time", &fields.due_time);
    input::set_string(body, "duration", &fields.duration);
    set_id(body, "deal_id", fields.deal_id.as_deref());
    input::set_string(body, "lead_id", &fields.lead_id);
    set_id(body, "person_id", fields.person_id.as_deref());
    set_id(body, "org_id", fields.org_id.as_deref());
    set_id(body, "owner_id", fields.owner_id.as_deref());
    input::set_string(body, "note", &fields.note);
    if let Some(done) = fields.done {
        input::ensure_object(body).insert("done".to_string(), Value::Bool(done));
    }
}

fn set_contact_field(body: &mut Value, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        input::ensure_object(body).insert(key.to_string(), json!([{ "value": value }]));
    }
}

fn set_address(body: &mut Value, value: Option<&str>) {
    if let Some(value) = value {
        input::ensure_object(body).insert("address".to_string(), json!([{ "value": value }]));
    }
}

fn set_id(body: &mut Value, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        input::ensure_object(body).insert(key.to_string(), id_value(value));
    }
}

fn set_f64(body: &mut Value, key: &str, value: Option<f64>) {
    if let Some(value) = value.and_then(Number::from_f64) {
        input::ensure_object(body).insert(key.to_string(), Value::Number(value));
    }
}

fn set_csv(body: &mut Value, key: &str, value: Option<&str>) -> Result<(), AppError> {
    if let Some(value) = value {
        let values = parse_csv(value)?;
        input::ensure_object(body).insert(key.to_string(), Value::Array(values));
    }
    Ok(())
}

fn parse_csv(value: &str) -> Result<Vec<Value>, AppError> {
    let values = value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(id_value)
        .collect::<Vec<_>>();
    if values.is_empty() {
        return Err(AppError::invalid_input(
            "pipedrive",
            "csv.parse",
            "CSV value must contain at least one id",
        ));
    }
    Ok(values)
}

fn id_value(value: &str) -> Value {
    value
        .parse::<u64>()
        .map(|id| json!(id))
        .unwrap_or_else(|_| Value::String(value.to_string()))
}

fn values_at(value: &Value, path: &[&str]) -> Vec<Value> {
    let mut current = value;
    for segment in path {
        let Some(next) = current.get(*segment) else {
            return Vec::new();
        };
        current = next;
    }
    current.as_array().cloned().unwrap_or_default()
}

fn empty_aggregate(array_path: &[&str]) -> Value {
    aggregate_response(None, array_path, Vec::new())
}

fn aggregate_response(first_page: Option<Value>, array_path: &[&str], values: Vec<Value>) -> Value {
    let mut response = first_page.unwrap_or_else(|| json!({}));
    set_array_at(&mut response, array_path, values);
    response
}

/// Request only what is still wanted, so a page is never cut short: the continuation
/// marker points past the whole page, and records dropped from its tail would be skipped
/// by the next call.
fn page_limit(limit: u32, collected: usize) -> u32 {
    let remaining = (limit as usize).saturating_sub(collected);
    remaining.clamp(1, 500) as u32
}

/// The aggregate carries the first page's metadata; rewrite its cursor to the final
/// state. Pipedrive marks the last page with an explicit `null` cursor.
fn finish_cursor_pagination(first_page: &mut Option<Value>, cursor: Option<String>) {
    set_optional_at(
        first_page,
        &["additional_data", "next_cursor"],
        Some(cursor.map_or(Value::Null, Value::String)),
    );
}

/// As `finish_cursor_pagination`, for v1 offsets: the first page's
/// `more_items_in_collection` is stale once later pages have been read.
fn finish_start_pagination(first_page: &mut Option<Value>, next_start: Option<u64>) {
    set_optional_at(
        first_page,
        &["additional_data", "pagination", "more_items_in_collection"],
        Some(Value::Bool(next_start.is_some())),
    );
    set_optional_at(
        first_page,
        &["additional_data", "pagination", "next_start"],
        next_start.map(|value| json!(value)),
    );
}

fn set_optional_at(response: &mut Option<Value>, path: &[&str], value: Option<Value>) {
    let Some(response) = response.as_mut() else {
        return;
    };
    let mut current = response;
    for segment in &path[..path.len() - 1] {
        let object = input::ensure_object(current);
        current = object
            .entry((*segment).to_string())
            .or_insert_with(|| Value::Object(Default::default()));
    }
    let object = input::ensure_object(current);
    if let Some(value) = value {
        object.insert(path[path.len() - 1].to_string(), value);
    } else {
        object.remove(path[path.len() - 1]);
    }
}

fn set_array_at(response: &mut Value, path: &[&str], values: Vec<Value>) {
    if path.is_empty() {
        *response = Value::Array(values);
        return;
    }
    let mut current = response;
    for segment in &path[..path.len() - 1] {
        let object = input::ensure_object(current);
        current = object
            .entry((*segment).to_string())
            .or_insert_with(|| Value::Object(Default::default()));
    }
    input::ensure_object(current).insert(path[path.len() - 1].to_string(), Value::Array(values));
}

fn extract_label_options(value: &Value) -> Vec<Value> {
    value
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|field| {
            field.get("key").and_then(Value::as_str) == Some("label")
                || field.get("name").and_then(Value::as_str) == Some("label")
        })
        .and_then(|field| field.get("options"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn sort_direction(value: Option<&PipedriveSortDirection>) -> Option<&'static str> {
    value.map(|value| match value {
        PipedriveSortDirection::Asc => "asc",
        PipedriveSortDirection::Desc => "desc",
    })
}

fn deal_status(value: Option<&PipedriveDealStatus>) -> Option<&'static str> {
    value.map(|value| match value {
        PipedriveDealStatus::Open => "open",
        PipedriveDealStatus::Won => "won",
        PipedriveDealStatus::Lost => "lost",
        PipedriveDealStatus::Deleted => "deleted",
    })
}

fn search_deal_status(value: Option<&PipedriveSearchDealStatus>) -> Option<&'static str> {
    value.map(|value| match value {
        PipedriveSearchDealStatus::Open => "open",
        PipedriveSearchDealStatus::Won => "won",
        PipedriveSearchDealStatus::Lost => "lost",
    })
}

fn mail_folder(value: &PipedriveMailFolder) -> &'static str {
    match value {
        PipedriveMailFolder::Inbox => "inbox",
        PipedriveMailFolder::Drafts => "drafts",
        PipedriveMailFolder::Sent => "sent",
        PipedriveMailFolder::Archive => "archive",
    }
}

#[derive(Debug, Default)]
struct Query(Vec<(String, String)>);

impl Query {
    fn new() -> Self {
        Self::default()
    }

    fn push(&mut self, key: &str, value: Option<&str>) {
        if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
            self.push_value(key, value);
        }
    }

    fn push_value(&mut self, key: &str, value: &str) {
        self.0.push((key.to_string(), value.to_string()));
    }

    fn push_bool(&mut self, key: &str, value: bool) {
        if value {
            self.push_value(key, "true");
        }
    }

    fn push_optional_bool(&mut self, key: &str, value: Option<bool>) {
        if let Some(value) = value {
            self.push_value(key, if value { "true" } else { "false" });
        }
    }

    fn set(&mut self, key: &str, value: String) {
        if let Some((_, existing)) = self.0.iter_mut().find(|(existing, _)| existing == key) {
            *existing = value;
        } else {
            self.0.push((key.to_string(), value));
        }
    }

    fn append_to(&self, url: &mut String) {
        let mut separator = if url.contains('?') { '&' } else { '?' };
        for (key, value) in &self.0 {
            url.push(separator);
            separator = '&';
            url.push_str(&urlencoding::encode(key));
            url.push('=');
            url.push_str(&urlencoding::encode(value));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Profile,
        test_support::{json_response, request_body, serve},
    };
    use clap::Parser;

    fn served_ctx(address: &str) -> Context {
        Context {
            profile: Profile {
                auth_type: Some("pipedrive_personal_token".to_string()),
                api_token: Some("pd-secret".to_string()),
                base_url: Some(format!("http://{address}")),
                ..Profile::default()
            },
            secrets_file: Default::default(),
            key_file: Default::default(),
        }
    }

    fn pipedrive_command(args: &[&str]) -> PipedriveCommand {
        let cli = Cli::try_parse_from(["aai-cli", "pipedrive"].iter().chain(args)).unwrap();
        let Command::Pipedrive(command) = cli.command else {
            panic!("not a pipedrive command");
        };
        command
    }

    #[tokio::test]
    async fn label_edits_read_then_write_every_record_type() {
        // Leads are v1 with UUID labels; deals, persons and organizations are v2 with
        // numeric labels. Each must GET the record, then PATCH the merged list.
        let cases = [
            (
                "leads",
                "/v1/leads/l1",
                r#"["a1","b2"]"#,
                "b2",
                "c3",
                json!(["a1", "c3"]),
            ),
            ("deals", "/api/v2/deals/5", "[1,2]", "2", "3", json!([1, 3])),
            (
                "persons",
                "/api/v2/persons/6",
                "[1,2]",
                "1",
                "4",
                json!([2, 4]),
            ),
            (
                "organizations",
                "/api/v2/organizations/7",
                "[9]",
                "9",
                "8",
                json!([8]),
            ),
        ];
        for (resource, path, current, remove, add, expected) in cases {
            let id = path.rsplit('/').next().unwrap();
            let (address, server) = serve(vec![
                json_response(&format!(
                    r#"{{"data":{{"id":"{id}","label_ids":{current}}}}}"#
                )),
                json_response(r#"{"data":{"id":1}}"#),
            ]);
            let command = pipedrive_command(&[
                resource,
                "update",
                id,
                "--remove-label-ids",
                remove,
                "--add-label-ids",
                add,
            ]);

            dispatch(&ApiClient::new().unwrap(), &served_ctx(&address), command)
                .await
                .unwrap();
            let requests = server.join().unwrap();

            assert!(
                requests[0].starts_with(&format!("get {path} ")),
                "{resource}: {}",
                requests[0]
            );
            assert!(
                requests[1].starts_with(&format!("patch {path} ")),
                "{resource}: {}",
                requests[1]
            );
            let body: Value = serde_json::from_str(request_body(&requests[1])).unwrap();
            assert_eq!(body["label_ids"], expected, "{resource}");
        }
    }

    #[tokio::test]
    async fn label_edits_treat_null_as_no_labels() {
        let (address, server) = serve(vec![
            json_response(r#"{"data":{"id":5,"label_ids":null}}"#),
            json_response(r#"{"data":{"id":5}}"#),
        ]);
        let command = pipedrive_command(&["deals", "update", "5", "--add-label-ids", "3"]);

        dispatch(&ApiClient::new().unwrap(), &served_ctx(&address), command)
            .await
            .unwrap();
        let requests = server.join().unwrap();

        let body: Value = serde_json::from_str(request_body(&requests[1])).unwrap();
        assert_eq!(body["label_ids"], json!([3]));
    }

    #[tokio::test]
    async fn label_edits_refuse_a_record_without_label_ids() {
        // Reading a missing key as "no labels" would PATCH label_ids: [] and erase them.
        let (address, server) = serve(vec![json_response(r#"{"data":{"id":5}}"#)]);
        let command = pipedrive_command(&["deals", "update", "5", "--remove-label-ids", "2"]);

        let error = dispatch(&ApiClient::new().unwrap(), &served_ctx(&address), command)
            .await
            .unwrap_err();
        let requests = server.join().unwrap();

        assert_eq!(requests.len(), 1, "nothing may be written");
        assert!(error.message.contains("label_ids"), "{error}");
    }

    #[tokio::test]
    async fn files_download_writes_the_file_and_reports_it() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("notes/transcript.pdf");
        let (address, server) = serve(vec![
            b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\nConnection: close\r\n\r\npdf-bytes".to_vec(),
        ]);
        let command = pipedrive_command(&[
            "files",
            "download",
            "42",
            "--output",
            output.to_str().unwrap(),
        ]);

        let result = dispatch(&ApiClient::new().unwrap(), &served_ctx(&address), command)
            .await
            .unwrap();
        let requests = server.join().unwrap();

        assert!(
            requests[0].starts_with("get /v1/files/42/download "),
            "{}",
            requests[0]
        );
        assert!(requests[0].contains("x-api-token: pd-secret"));
        assert_eq!(std::fs::read(&output).unwrap(), b"pdf-bytes");
        assert_eq!(result["file_id"], "42");
        assert_eq!(result["bytes"], 9);
        assert_eq!(result["output"], output.to_str().unwrap());
    }

    #[tokio::test]
    async fn users_list_is_marked_complete() {
        let (address, server) = serve(vec![json_response(r#"{"success":true,"data":[{"id":1}]}"#)]);
        let command = pipedrive_command(&["users", "list"]);

        let result = dispatch(&ApiClient::new().unwrap(), &served_ctx(&address), command)
            .await
            .unwrap();
        server.join().unwrap();

        assert_eq!(
            crate::pagination::annotate(result, &[])["_aai"]["pagination"]["status"],
            "complete"
        );
    }

    fn file_list(
        deal_id: Option<&str>,
        person_id: Option<&str>,
        org_id: Option<&str>,
    ) -> PipedriveFileList {
        PipedriveFileList {
            limit: 50,
            deal_id: deal_id.map(str::to_string),
            person_id: person_id.map(str::to_string),
            org_id: org_id.map(str::to_string),
            sort: None,
        }
    }

    #[test]
    fn file_list_uses_the_record_scoped_endpoint() {
        assert_eq!(file_list_path(&file_list(None, None, None)), "/v1/files");
        assert_eq!(
            file_list_path(&file_list(Some("12"), None, None)),
            "/v1/deals/12/files"
        );
        assert_eq!(
            file_list_path(&file_list(None, Some("5"), None)),
            "/v1/persons/5/files"
        );
        assert_eq!(
            file_list_path(&file_list(None, None, Some("3"))),
            "/v1/organizations/3/files"
        );
    }

    #[test]
    fn label_changes_keep_existing_labels() {
        assert_eq!(
            merge_label_ids(vec![json!(5), json!(6)], &[json!(7), json!(5)], &[json!(6)]),
            vec![json!(5), json!(7)]
        );
        assert_eq!(
            merge_label_ids(
                vec![json!("b4b7ce80-9c69-11f1-99c7-b5ebf9228c75")],
                &[json!("586d25c0-9c79-11f1-8028-1583155158d7")],
                &[json!("b4b7ce80-9c69-11f1-99c7-b5ebf9228c75")],
            ),
            vec![json!("586d25c0-9c79-11f1-8028-1583155158d7")]
        );
    }

    #[test]
    fn note_body_links_records_and_keeps_lead_ids_as_strings() {
        let body = note_create_body(PipedriveNoteWrite {
            json: None,
            content: "Discovery call summary".to_string(),
            links: PipedriveNoteLinks {
                deal_id: Some("12".to_string()),
                person_id: None,
                org_id: Some("3".to_string()),
                lead_id: Some("1f6f1c7e-0000-4000-8000-000000000000".to_string()),
            },
        })
        .unwrap();
        assert_eq!(
            body,
            json!({
                "content": "Discovery call summary",
                "deal_id": 12,
                "org_id": 3,
                "lead_id": "1f6f1c7e-0000-4000-8000-000000000000"
            })
        );
    }

    #[test]
    fn activity_typed_flags_override_json_payload() {
        let body = activity_update_body(PipedriveActivityUpdate {
            id: "5".to_string(),
            json: Some(r#"{"subject":"old","done":false,"note":"keep"}"#.to_string()),
            subject: Some("Follow up".to_string()),
            fields: PipedriveActivityFields {
                activity_type: Some("call".to_string()),
                due_date: Some("2026-10-01".to_string()),
                due_time: None,
                duration: None,
                deal_id: Some("12".to_string()),
                lead_id: None,
                person_id: None,
                org_id: None,
                owner_id: Some("28".to_string()),
                note: None,
                done: Some(true),
            },
        })
        .unwrap();
        assert_eq!(
            body,
            json!({
                "subject": "Follow up",
                "type": "call",
                "due_date": "2026-10-01",
                "deal_id": 12,
                "owner_id": 28,
                "done": true,
                "note": "keep"
            })
        );
    }

    #[test]
    fn parse_csv_keeps_numeric_and_string_ids() {
        assert_eq!(
            parse_csv("1, abc,2").unwrap(),
            vec![json!(1), json!("abc"), json!(2)]
        );
    }

    #[test]
    fn typed_flags_override_json_payload() {
        let body = deal_update_body(PipedriveDealUpdate {
            id: "10".to_string(),
            json: Some(r#"{"title":"old","value":1,"label_ids":["old"]}"#.to_string()),
            title: Some("new".to_string()),
            person_id: Some("123".to_string()),
            org_id: None,
            value: Some(42.5),
            currency: Some("USD".to_string()),
            pipeline_id: None,
            stage_id: None,
            label_ids: Some("blue,7".to_string()),
            labels: PipedriveLabelEdits::default(),
        })
        .unwrap();

        assert_eq!(body["title"], "new");
        assert_eq!(body["person_id"], 123);
        assert_eq!(body["value"], 42.5);
        assert_eq!(body["currency"], "USD");
        assert_eq!(body["label_ids"], json!(["blue", 7]));
    }

    #[test]
    fn person_body_uses_pipedrive_contact_arrays() {
        let body = person_create_body(PipedrivePersonWrite {
            json: None,
            name: "Ada".to_string(),
            org_id: Some("5".to_string()),
            email: Some("ada@example.com".to_string()),
            phone: Some("+1 555 0100".to_string()),
            label_ids: None,
        })
        .unwrap();

        assert_eq!(body["name"], "Ada");
        assert_eq!(body["org_id"], 5);
        assert_eq!(body["emails"][0]["value"], "ada@example.com");
        assert_eq!(body["phones"][0]["value"], "+1 555 0100");
    }

    #[test]
    fn organization_address_flag_uses_pipedrive_address_array() {
        let body = organization_create_body(PipedriveOrganizationWrite {
            json: None,
            name: "Acme".to_string(),
            address: Some("1 Test Way".to_string()),
            label_ids: None,
        })
        .unwrap();

        assert_eq!(body["address"], json!([{ "value": "1 Test Way" }]));
    }

    #[test]
    fn aggregate_v2_cursor_shape_replaces_page_local_data() {
        let first = json!({
            "success": true,
            "data": [{"id": 1}],
            "additional_data": {"next_cursor": "abc"}
        });
        let response = aggregate_response(
            Some(first),
            &["data"],
            vec![json!({"id": 1}), json!({"id": 2})],
        );
        assert_eq!(response["data"], json!([{"id": 1}, {"id": 2}]));
        assert_eq!(response["additional_data"]["next_cursor"], "abc");
    }

    #[test]
    fn aggregate_search_items_shape() {
        let first = json!({
            "success": true,
            "data": {"items": [{"item": {"id": 1}}]},
            "additional_data": {"pagination": {"next_start": 100}}
        });
        let response = aggregate_response(
            Some(first),
            &["data", "items"],
            vec![json!({"item": {"id": 1}}), json!({"item": {"id": 2}})],
        );
        assert_eq!(response["data"]["items"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn optional_continuation_removes_stale_first_page_value() {
        let mut response = Some(json!({
            "additional_data": {"next_cursor": "stale"}
        }));
        set_optional_at(&mut response, &["additional_data", "next_cursor"], None);
        assert!(response.unwrap()["additional_data"]
            .get("next_cursor")
            .is_none());
    }

    #[test]
    fn page_limit_never_fetches_past_the_requested_limit() {
        // A 500-item page cut down to fit the limit would leave its tail behind the
        // continuation marker, so a follow-up call would skip those records.
        assert_eq!(page_limit(700, 500), 200);
        assert_eq!(page_limit(700, 0), 500);
        assert_eq!(page_limit(30, 0), 30);
    }

    #[test]
    fn exhausted_cursor_pagination_ends_with_a_null_cursor() {
        let mut response = Some(json!({"additional_data": {"next_cursor": "page-2"}}));
        finish_cursor_pagination(&mut response, None);
        assert_eq!(
            response.unwrap()["additional_data"]["next_cursor"],
            Value::Null
        );
    }

    #[test]
    fn exhausted_start_pagination_clears_the_first_pages_more_flag() {
        let mut response = Some(json!({
            "additional_data": {"pagination": {"next_start": 500, "more_items_in_collection": true}}
        }));
        finish_start_pagination(&mut response, None);
        let pagination = &response.unwrap()["additional_data"]["pagination"];
        assert!(pagination.get("next_start").is_none());
        assert_eq!(pagination["more_items_in_collection"], false);
    }

    #[test]
    fn extracts_label_options_from_field_metadata() {
        let metadata = json!({
            "data": [
                {"key": "title"},
                {"key": "label", "options": [{"id": 1, "label": "Hot"}]}
            ]
        });
        assert_eq!(
            extract_label_options(&metadata),
            vec![json!({"id": 1, "label": "Hot"})]
        );
    }
}
