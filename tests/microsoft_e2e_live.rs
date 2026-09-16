use std::{
    env,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{json, Value};

fn parse_success(output: Output, args: &[&str]) -> Value {
    if !output.status.success() {
        panic!(
            "command failed: {args:?}\nstatus: {}\nstdout: {}\nstderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    serde_json::from_slice(&output.stdout).expect("aai-cli JSON output")
}

fn run(config: &str, profile: &str, args: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_aai-cli"))
        .args(["--config", config, "--profile", profile])
        .args(args)
        .output()
        .expect("execute aai-cli");
    parse_success(output, args)
}

fn output(config: &str, profile: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aai-cli"))
        .args(["--config", config, "--profile", profile])
        .args(args)
        .output()
        .expect("execute aai-cli")
}

fn assert_not_found(config: &str, profile: &str, args: &[&str]) {
    let output = output(config, profile, args);
    assert_eq!(
        output.status.code(),
        Some(5),
        "expected not_found: {args:?}"
    );
    let error: Value = serde_json::from_slice(&output.stderr).expect("structured JSON error");
    assert_eq!(error["code"], "not_found");
}

fn field<'a>(value: &'a Value, name: &str) -> &'a str {
    value
        .get(name)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("missing string field {name}: {value:#}"))
}

fn unique(prefix: &str) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_millis();
    format!("{prefix}-{millis}")
}

fn write_minimal_docx(path: &Path, marker: &str) {
    let file = File::create(path).expect("create docx");
    let mut zip = zip::ZipWriter::new(file);
    let options: zip::write::SimpleFileOptions = Default::default();
    zip.start_file("[Content_Types].xml", options).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#).unwrap();
    zip.start_file("_rels/.rels", options).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#).unwrap();
    zip.start_file("word/document.xml", options).unwrap();
    write!(zip, r#"<?xml version="1.0" encoding="UTF-8"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>{marker}</w:t></w:r></w:p><w:sectPr/></w:body></w:document>"#).unwrap();
    zip.finish().unwrap();
}

fn docx_contains(path: &Path, marker: &str) -> bool {
    let file = File::open(path).expect("open downloaded docx");
    let mut archive = zip::ZipArchive::new(file).expect("parse downloaded docx");
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .expect("Word document part")
        .read_to_string(&mut xml)
        .expect("read Word document part");
    xml.contains(marker)
}

struct Cleanup {
    config: String,
    profile: String,
    drive_id: String,
    remote_name: String,
    local_paths: Vec<PathBuf>,
    active: bool,
}

impl Cleanup {
    fn run(&mut self) {
        for args in [
            vec![
                "microsoft",
                "sharepoint",
                "files",
                "delete",
                self.remote_name.as_str(),
                "--drive-id",
                self.drive_id.as_str(),
            ],
            vec!["microsoft", "files", "delete", self.remote_name.as_str()],
        ] {
            let _ = Command::new(env!("CARGO_BIN_EXE_aai-cli"))
                .args([
                    "--config",
                    self.config.as_str(),
                    "--profile",
                    self.profile.as_str(),
                ])
                .args(args)
                .output();
        }
        for path in &self.local_paths {
            let _ = std::fs::remove_file(path);
        }
        self.active = false;
    }
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        if self.active {
            self.run();
        }
    }
}

struct CommandCleanup {
    config: String,
    profile: String,
    commands: Vec<Vec<String>>,
    active: bool,
}

impl CommandCleanup {
    fn push(&mut self, args: &[&str]) {
        self.commands
            .push(args.iter().map(|value| value.to_string()).collect());
    }

    fn disarm(&mut self) {
        self.active = false;
    }
}

impl Drop for CommandCleanup {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        for args in self.commands.iter().rev() {
            let _ = Command::new(env!("CARGO_BIN_EXE_aai-cli"))
                .args([
                    "--config",
                    self.config.as_str(),
                    "--profile",
                    self.profile.as_str(),
                ])
                .args(args)
                .output();
        }
    }
}

fn microsoft_env(profile_key: &str) -> Option<(String, String)> {
    let config = env::var("AAI_E2E_CONFIG").ok()?;
    let profile = env::var(profile_key).ok()?;
    Some((config, profile))
}

#[test]
#[ignore = "requires Microsoft app credentials and disposable Microsoft 365 resources"]
fn given_app_credentials_when_service_flows_run_then_outlook_sharepoint_and_teams_work() {
    // Given a saved app credential and the provisioned E2E workspace,
    // when typed CLI workflows exercise each app-capable service,
    // then CRUD, send/receive, collection, and collaboration reads succeed.
    outlook_and_sharepoint_crud_scenario();
    mail_send_to_self_scenario();
    teams_reads_scenario();
}

#[test]
#[ignore = "requires durable delegated/app credentials and disposable task resources"]
fn given_saved_credentials_when_task_flows_run_then_todo_and_planner_crud_work() {
    // Given durable delegated and app credentials,
    // when tasks are created, retrieved, updated, checked, and deleted,
    // then To Do and Planner both complete the idempotent behavioral flow.
    todo_and_planner_crud_scenario();
}

#[test]
#[ignore = "requires durable delegated credentials and disposable drive files"]
fn given_delegated_credentials_when_word_is_moved_then_both_drive_copies_are_verified_and_deleted()
{
    // Given a durable delegated credential and a provisioned SharePoint drive,
    // when a Word document moves through OneDrive and SharePoint,
    // then its content survives both downloads and both remote copies are deleted.
    word_onedrive_to_sharepoint_roundtrip_scenario();
}

fn outlook_and_sharepoint_crud_scenario() {
    let Some((config, profile)) = microsoft_env("AAI_E2E_MS_APP_PROFILE") else {
        eprintln!("skipping live Microsoft E2E: app profile environment is not set");
        return;
    };
    let (Ok(site_id), Ok(list_id), Ok(user_upn)) = (
        env::var("AAI_E2E_MS_SITE_ID"),
        env::var("AAI_E2E_MS_LIST_ID"),
        env::var("AAI_E2E_MS_USER_UPN"),
    ) else {
        eprintln!("skipping live Microsoft E2E: resource environment is not set");
        return;
    };
    let marker = unique("aai-e2e-ms");
    let mut cleanup = CommandCleanup {
        config: config.clone(),
        profile: profile.clone(),
        commands: Vec::new(),
        active: true,
    };

    let draft_body = json!({
        "subject": format!("{marker} draft"),
        "body": {"contentType": "Text", "content": "created"},
        "toRecipients": [{"emailAddress": {"address": user_upn}}]
    })
    .to_string();
    let draft = run(
        &config,
        &profile,
        &[
            "microsoft",
            "mail",
            "messages",
            "create",
            "--json",
            &draft_body,
        ],
    );
    let draft_id = field(&draft, "id").to_string();
    cleanup.push(&["microsoft", "mail", "messages", "delete", &draft_id]);
    let fetched = run(
        &config,
        &profile,
        &["microsoft", "mail", "messages", "get", &draft_id],
    );
    assert_eq!(field(&fetched, "subject"), format!("{marker} draft"));
    let draft_update = json!({"subject": format!("{marker} draft updated")}).to_string();
    run(
        &config,
        &profile,
        &[
            "microsoft",
            "mail",
            "messages",
            "update",
            &draft_id,
            "--json",
            &draft_update,
        ],
    );
    let fetched = run(
        &config,
        &profile,
        &["microsoft", "mail", "messages", "get", &draft_id],
    );
    assert_eq!(
        field(&fetched, "subject"),
        format!("{marker} draft updated")
    );

    let event_body = json!({
        "subject": format!("{marker} event"),
        "start": {"dateTime": "2030-01-15T10:00:00", "timeZone": "UTC"},
        "end": {"dateTime": "2030-01-15T10:30:00", "timeZone": "UTC"}
    })
    .to_string();
    let event = run(
        &config,
        &profile,
        &[
            "microsoft",
            "calendar",
            "events",
            "create",
            "--json",
            &event_body,
        ],
    );
    let event_id = field(&event, "id").to_string();
    cleanup.push(&["microsoft", "calendar", "events", "delete", &event_id]);
    let event_update = json!({"subject": format!("{marker} event updated")}).to_string();
    run(
        &config,
        &profile,
        &[
            "microsoft",
            "calendar",
            "events",
            "update",
            &event_id,
            "--json",
            &event_update,
        ],
    );
    let fetched = run(
        &config,
        &profile,
        &["microsoft", "calendar", "events", "get", &event_id],
    );
    assert_eq!(
        field(&fetched, "subject"),
        format!("{marker} event updated")
    );

    let contact_body = json!({
        "givenName": marker.clone(),
        "surname": "Contact",
        "emailAddresses": [{"address": format!("{marker}@example.test"), "name": marker.clone()}]
    })
    .to_string();
    let contact = run(
        &config,
        &profile,
        &["microsoft", "contacts", "create", "--json", &contact_body],
    );
    let contact_id = field(&contact, "id").to_string();
    cleanup.push(&["microsoft", "contacts", "delete", &contact_id]);
    let contact_update = json!({"surname": "Updated"}).to_string();
    run(
        &config,
        &profile,
        &[
            "microsoft",
            "contacts",
            "update",
            &contact_id,
            "--json",
            &contact_update,
        ],
    );
    let fetched = run(
        &config,
        &profile,
        &["microsoft", "contacts", "get", &contact_id],
    );
    assert_eq!(field(&fetched, "surname"), "Updated");

    let item_body = json!({
        "fields": {"Title": marker.clone(), "Status": "New", "ExternalId": marker.clone()}
    })
    .to_string();
    let item = run(
        &config,
        &profile,
        &[
            "microsoft",
            "sharepoint",
            "items",
            "create",
            &site_id,
            &list_id,
            "--json",
            &item_body,
        ],
    );
    let item_id = field(&item, "id").to_string();
    cleanup.push(&[
        "microsoft",
        "sharepoint",
        "items",
        "delete",
        &site_id,
        &list_id,
        &item_id,
    ]);
    let item_update = json!({"Status": "Done"}).to_string();
    run(
        &config,
        &profile,
        &[
            "microsoft",
            "sharepoint",
            "items",
            "update",
            &site_id,
            &list_id,
            &item_id,
            "--json",
            &item_update,
        ],
    );
    let fetched = run(
        &config,
        &profile,
        &[
            "microsoft",
            "sharepoint",
            "items",
            "get",
            &site_id,
            &list_id,
            &item_id,
        ],
    );
    assert_eq!(fetched["fields"]["Status"], "Done");

    for args in [
        vec!["microsoft", "mail", "messages", "list", "--limit", "2"],
        vec!["microsoft", "calendar", "events", "list", "--limit", "2"],
        vec!["microsoft", "contacts", "list", "--limit", "2"],
        vec![
            "microsoft",
            "sharepoint",
            "lists",
            "list",
            &site_id,
            "--limit",
            "2",
        ],
        vec![
            "microsoft",
            "sharepoint",
            "items",
            "list",
            &site_id,
            &list_id,
            "--limit",
            "2",
        ],
    ] {
        let response = run(&config, &profile, &args);
        assert!(
            response["value"].is_array(),
            "expected Graph value[]: {response:#}"
        );
    }
    let configured_list = run(
        &config,
        &profile,
        &[
            "microsoft",
            "sharepoint",
            "lists",
            "get",
            &site_id,
            &list_id,
        ],
    );
    assert_eq!(field(&configured_list, "id"), list_id);

    run(
        &config,
        &profile,
        &["microsoft", "mail", "messages", "delete", &draft_id],
    );
    assert_not_found(
        &config,
        &profile,
        &["microsoft", "mail", "messages", "get", &draft_id],
    );
    run(
        &config,
        &profile,
        &["microsoft", "calendar", "events", "delete", &event_id],
    );
    assert_not_found(
        &config,
        &profile,
        &["microsoft", "calendar", "events", "get", &event_id],
    );
    run(
        &config,
        &profile,
        &["microsoft", "contacts", "delete", &contact_id],
    );
    assert_not_found(
        &config,
        &profile,
        &["microsoft", "contacts", "get", &contact_id],
    );
    run(
        &config,
        &profile,
        &[
            "microsoft",
            "sharepoint",
            "items",
            "delete",
            &site_id,
            &list_id,
            &item_id,
        ],
    );
    assert_not_found(
        &config,
        &profile,
        &[
            "microsoft",
            "sharepoint",
            "items",
            "get",
            &site_id,
            &list_id,
            &item_id,
        ],
    );
    cleanup.disarm();
}

fn messages_with_subject(
    config: &str,
    profile: &str,
    user_id: &str,
    folder: &str,
    subject: &str,
) -> Vec<String> {
    let response = messages_query_output(config, profile, user_id, folder, subject);
    let response = parse_success(response, &["microsoft", "request", "get", "mail-query"]);
    response["value"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|message| {
            message
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .collect()
}

fn messages_query_output(
    config: &str,
    profile: &str,
    user_id: &str,
    folder: &str,
    subject: &str,
) -> Output {
    let filter = format!("$filter=subject eq '{subject}'");
    let select = "$select=id,subject";
    output(
        config,
        profile,
        &[
            "microsoft",
            "request",
            "get",
            &format!("/users/{user_id}/mailFolders/{folder}/messages"),
            "--query",
            &filter,
            "--query",
            select,
        ],
    )
}

struct MailCleanup {
    config: String,
    profile: String,
    user_id: String,
    subject: String,
}

impl Drop for MailCleanup {
    fn drop(&mut self) {
        for folder in ["inbox", "sentitems"] {
            let response = messages_query_output(
                &self.config,
                &self.profile,
                &self.user_id,
                folder,
                &self.subject,
            );
            if !response.status.success() {
                continue;
            }
            let Ok(body) = serde_json::from_slice::<Value>(&response.stdout) else {
                continue;
            };
            let ids = body["value"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|message| message.get("id").and_then(Value::as_str));
            for id in ids {
                let _ = output(
                    &self.config,
                    &self.profile,
                    &["microsoft", "mail", "messages", "delete", id],
                );
            }
        }
    }
}

fn mail_send_to_self_scenario() {
    let Some((config, profile)) = microsoft_env("AAI_E2E_MS_APP_PROFILE") else {
        eprintln!("skipping live Microsoft E2E: app profile environment is not set");
        return;
    };
    let (Ok(user_id), Ok(user_upn)) = (
        env::var("AAI_E2E_MS_USER_ID"),
        env::var("AAI_E2E_MS_USER_UPN"),
    ) else {
        eprintln!("skipping live Microsoft E2E: user environment is not set");
        return;
    };
    let subject = unique("aai-e2e-ms-send");
    let _cleanup = MailCleanup {
        config: config.clone(),
        profile: profile.clone(),
        user_id: user_id.clone(),
        subject: subject.clone(),
    };
    let body = json!({
        "message": {
            "subject": subject.clone(),
            "body": {"contentType": "Text", "content": "Disposable aai-cli Microsoft E2E message."},
            "toRecipients": [{"emailAddress": {"address": user_upn}}]
        },
        "saveToSentItems": true
    })
    .to_string();
    run(
        &config,
        &profile,
        &["microsoft", "mail", "send", "--json", &body],
    );

    let mut delivered = Vec::new();
    for _ in 0..15 {
        delivered = messages_with_subject(&config, &profile, &user_id, "inbox", &subject);
        if !delivered.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    assert_eq!(
        delivered.len(),
        1,
        "self-addressed message was not delivered"
    );
    let sent = messages_with_subject(&config, &profile, &user_id, "sentitems", &subject);
    assert_eq!(sent.len(), 1, "sent copy was not created");
}

struct PlannerCleanup {
    config: String,
    profile: String,
    task_id: Option<String>,
}

impl Drop for PlannerCleanup {
    fn drop(&mut self) {
        let Some(task_id) = self.task_id.as_deref() else {
            return;
        };
        let get_args = ["microsoft", "planner", "tasks", "get", task_id];
        let fetched = output(&self.config, &self.profile, &get_args);
        if !fetched.status.success() {
            return;
        }
        let Ok(task) = serde_json::from_slice::<Value>(&fetched.stdout) else {
            return;
        };
        let Some(etag) = task.get("@odata.etag").and_then(Value::as_str) else {
            return;
        };
        let _ = output(
            &self.config,
            &self.profile,
            &[
                "microsoft",
                "planner",
                "tasks",
                "delete",
                task_id,
                "--etag",
                etag,
            ],
        );
    }
}

fn todo_and_planner_crud_scenario() {
    let (Some((config, delegated_profile)), Some((_, app_profile))) = (
        microsoft_env("AAI_E2E_MS_DELEGATED_PROFILE"),
        microsoft_env("AAI_E2E_MS_APP_PROFILE"),
    ) else {
        eprintln!("skipping live Microsoft E2E: Microsoft profiles are not set");
        return;
    };
    let (Ok(plan_id), Ok(bucket_id)) = (
        env::var("AAI_E2E_MS_PLANNER_PLAN_ID"),
        env::var("AAI_E2E_MS_PLANNER_BUCKET_ID"),
    ) else {
        eprintln!("skipping live Microsoft E2E: Planner environment is not set");
        return;
    };
    let marker = unique("aai-e2e-ms-task");
    let mut todo_cleanup = CommandCleanup {
        config: config.clone(),
        profile: delegated_profile.clone(),
        commands: Vec::new(),
        active: true,
    };

    let list_body = json!({"displayName": format!("{marker} list")}).to_string();
    let list = run(
        &config,
        &delegated_profile,
        &["microsoft", "todo", "lists", "create", "--json", &list_body],
    );
    let list_id = field(&list, "id").to_string();
    todo_cleanup.push(&["microsoft", "todo", "lists", "delete", &list_id]);
    let list_update = json!({"displayName": format!("{marker} list updated")}).to_string();
    run(
        &config,
        &delegated_profile,
        &[
            "microsoft",
            "todo",
            "lists",
            "update",
            &list_id,
            "--json",
            &list_update,
        ],
    );
    let fetched = run(
        &config,
        &delegated_profile,
        &["microsoft", "todo", "lists", "get", &list_id],
    );
    assert_eq!(
        field(&fetched, "displayName"),
        format!("{marker} list updated")
    );
    let empty_tasks = run(
        &config,
        &delegated_profile,
        &[
            "microsoft",
            "todo",
            "tasks",
            "list",
            &list_id,
            "--limit",
            "10",
        ],
    );
    assert_eq!(empty_tasks["value"], json!([]));

    let task_body = json!({"title": format!("{marker} todo")}).to_string();
    let task = run(
        &config,
        &delegated_profile,
        &[
            "microsoft",
            "todo",
            "tasks",
            "create",
            &list_id,
            "--json",
            &task_body,
        ],
    );
    let task_id = field(&task, "id").to_string();
    let task_update = json!({"title": format!("{marker} todo updated")}).to_string();
    run(
        &config,
        &delegated_profile,
        &[
            "microsoft",
            "todo",
            "tasks",
            "update",
            &list_id,
            &task_id,
            "--json",
            &task_update,
        ],
    );
    let fetched = run(
        &config,
        &delegated_profile,
        &["microsoft", "todo", "tasks", "get", &list_id, &task_id],
    );
    assert_eq!(field(&fetched, "title"), format!("{marker} todo updated"));
    run(
        &config,
        &delegated_profile,
        &["microsoft", "todo", "tasks", "delete", &list_id, &task_id],
    );
    assert_not_found(
        &config,
        &delegated_profile,
        &["microsoft", "todo", "tasks", "get", &list_id, &task_id],
    );
    run(
        &config,
        &delegated_profile,
        &["microsoft", "todo", "lists", "delete", &list_id],
    );
    assert_not_found(
        &config,
        &delegated_profile,
        &["microsoft", "todo", "lists", "get", &list_id],
    );
    todo_cleanup.disarm();

    let planner_body = json!({
        "planId": plan_id,
        "bucketId": bucket_id,
        "title": format!("{marker} planner")
    })
    .to_string();
    let planner_task = run(
        &config,
        &app_profile,
        &[
            "microsoft",
            "planner",
            "tasks",
            "create",
            "--json",
            &planner_body,
        ],
    );
    let planner_task_id = field(&planner_task, "id").to_string();
    let mut planner_cleanup = PlannerCleanup {
        config: config.clone(),
        profile: app_profile.clone(),
        task_id: Some(planner_task_id.clone()),
    };
    let etag = field(&planner_task, "@odata.etag").to_string();
    let planner_update = json!({"title": format!("{marker} planner updated")}).to_string();
    run(
        &config,
        &app_profile,
        &[
            "microsoft",
            "planner",
            "tasks",
            "update",
            &planner_task_id,
            "--etag",
            &etag,
            "--json",
            &planner_update,
        ],
    );
    let fetched = run(
        &config,
        &app_profile,
        &["microsoft", "planner", "tasks", "get", &planner_task_id],
    );
    assert_eq!(
        field(&fetched, "title"),
        format!("{marker} planner updated")
    );
    let updated_etag = field(&fetched, "@odata.etag");
    run(
        &config,
        &app_profile,
        &[
            "microsoft",
            "planner",
            "tasks",
            "delete",
            &planner_task_id,
            "--etag",
            updated_etag,
        ],
    );
    assert_not_found(
        &config,
        &app_profile,
        &["microsoft", "planner", "tasks", "get", &planner_task_id],
    );
    planner_cleanup.task_id = None;
}

fn teams_reads_scenario() {
    let Some((config, profile)) = microsoft_env("AAI_E2E_MS_APP_PROFILE") else {
        eprintln!("skipping live Microsoft E2E: app profile environment is not set");
        return;
    };
    let (Ok(team_id), Ok(channel_id)) = (
        env::var("AAI_E2E_MS_TEAM_ID"),
        env::var("AAI_E2E_MS_CHANNEL_ID"),
    ) else {
        eprintln!("skipping live Microsoft E2E: Teams environment is not set");
        return;
    };
    let team = run(&config, &profile, &["microsoft", "teams", "get", &team_id]);
    assert_eq!(field(&team, "id"), team_id);
    let channel = run(
        &config,
        &profile,
        &["microsoft", "teams", "channel", &team_id, &channel_id],
    );
    assert_eq!(field(&channel, "id"), channel_id);
    for args in [
        vec!["microsoft", "teams", "channels", &team_id, "--limit", "10"],
        vec!["microsoft", "teams", "members", &team_id, "--limit", "10"],
        vec![
            "microsoft",
            "teams",
            "messages",
            &team_id,
            &channel_id,
            "--limit",
            "10",
        ],
        vec!["microsoft", "teams", "chats", "--limit", "10"],
    ] {
        let response = run(&config, &profile, &args);
        assert!(
            response["value"].is_array(),
            "expected Graph value[]: {response:#}"
        );
    }
}

fn word_onedrive_to_sharepoint_roundtrip_scenario() {
    let (Ok(config), Ok(profile), Ok(drive_id)) = (
        env::var("AAI_E2E_CONFIG"),
        env::var("AAI_E2E_MS_DELEGATED_PROFILE"),
        env::var("AAI_E2E_MS_DRIVE_ID"),
    ) else {
        eprintln!("skipping live Microsoft E2E: required environment is not set");
        return;
    };
    let remote_name = format!("{}.docx", unique("aai-e2e-ms-word"));
    let marker = format!("AAI Microsoft Word round trip {remote_name}");
    let source = env::temp_dir().join(format!("source-{remote_name}"));
    let from_onedrive = env::temp_dir().join(format!("onedrive-{remote_name}"));
    let from_sharepoint = env::temp_dir().join(format!("sharepoint-{remote_name}"));
    let mut cleanup = Cleanup {
        config: config.clone(),
        profile: profile.clone(),
        drive_id: drive_id.clone(),
        remote_name: remote_name.clone(),
        local_paths: vec![
            source.clone(),
            from_onedrive.clone(),
            from_sharepoint.clone(),
        ],
        active: true,
    };
    write_minimal_docx(&source, &marker);

    run(
        &config,
        &profile,
        &[
            "microsoft",
            "files",
            "upload",
            source.to_str().unwrap(),
            &remote_name,
            "--mime-type",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        ],
    );
    run(
        &config,
        &profile,
        &[
            "microsoft",
            "files",
            "download",
            &remote_name,
            "--output",
            from_onedrive.to_str().unwrap(),
        ],
    );
    assert!(docx_contains(&from_onedrive, &marker));

    run(
        &config,
        &profile,
        &[
            "microsoft",
            "sharepoint",
            "files",
            "upload",
            from_onedrive.to_str().unwrap(),
            &remote_name,
            "--drive-id",
            &drive_id,
            "--mime-type",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        ],
    );
    run(
        &config,
        &profile,
        &[
            "microsoft",
            "sharepoint",
            "files",
            "download",
            &remote_name,
            "--drive-id",
            &drive_id,
            "--output",
            from_sharepoint.to_str().unwrap(),
        ],
    );
    assert!(docx_contains(&from_sharepoint, &marker));

    run(
        &config,
        &profile,
        &[
            "microsoft",
            "sharepoint",
            "files",
            "delete",
            &remote_name,
            "--drive-id",
            &drive_id,
        ],
    );
    run(
        &config,
        &profile,
        &["microsoft", "files", "delete", &remote_name],
    );
    assert_not_found(
        &config,
        &profile,
        &[
            "microsoft",
            "sharepoint",
            "files",
            "download",
            &remote_name,
            "--drive-id",
            &drive_id,
            "--output",
            from_sharepoint.to_str().unwrap(),
        ],
    );
    assert_not_found(
        &config,
        &profile,
        &[
            "microsoft",
            "files",
            "download",
            &remote_name,
            "--output",
            from_onedrive.to_str().unwrap(),
        ],
    );
    cleanup.run();
}
