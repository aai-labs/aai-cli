use std::{
    env,
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::Value;

fn cli(profile_env: &str, args: &[&str]) -> Option<Value> {
    let config = match env::var("AAI_E2E_CONFIG") {
        Ok(value) => value,
        Err(_) => {
            eprintln!("skipping live E2E: AAI_E2E_CONFIG is not set");
            return None;
        }
    };
    let profile = match env::var(profile_env) {
        Ok(value) => value,
        Err(_) => {
            eprintln!("skipping live E2E: {profile_env} is not set");
            return None;
        }
    };

    let mut command = Command::new(env!("CARGO_BIN_EXE_aai-cli"));
    command
        .arg("--config")
        .arg(config)
        .arg("--profile")
        .arg(profile);
    command.args(args);
    Some(parse_success(
        command.output().expect("failed to execute aai-cli"),
        args,
    ))
}

fn cli_required(profile_env: &str, args: &[&str]) -> Value {
    cli(profile_env, args).unwrap_or_else(|| panic!("missing required E2E env for {profile_env}"))
}

fn parse_success(output: Output, args: &[&str]) -> Value {
    if !output.status.success() {
        panic!(
            "command failed: {:?}\nstatus: {}\nstdout: {}\nstderr: {}",
            args,
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&stdout)
            .unwrap_or_else(|err| panic!("invalid JSON stdout for {:?}: {err}\n{stdout}", args))
    }
}

fn env_or_skip(name: &str) -> Option<String> {
    match env::var(name) {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => {
            eprintln!("skipping live E2E branch: {name} is not set");
            None
        }
    }
}

fn unique(prefix: &str) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_millis();
    format!("{prefix}-{millis}")
}

fn str_at<'a>(value: &'a Value, path: &[&str]) -> &'a str {
    let mut current = value;
    for segment in path {
        current = current
            .get(*segment)
            .unwrap_or_else(|| panic!("missing path {path:?} in {value:#}"));
    }
    current
        .as_str()
        .unwrap_or_else(|| panic!("path {path:?} is not string in {value:#}"))
}

fn u64_at(value: &Value, path: &[&str]) -> u64 {
    let mut current = value;
    for segment in path {
        current = current
            .get(*segment)
            .unwrap_or_else(|| panic!("missing path {path:?} in {value:#}"));
    }
    current
        .as_u64()
        .unwrap_or_else(|| panic!("path {path:?} is not u64 in {value:#}"))
}

fn find_string(value: &Value, keys: &[&str]) -> Option<String> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(found) = map.get(*key).and_then(Value::as_str) {
                    return Some(found.to_string());
                }
            }
            map.values().find_map(|value| find_string(value, keys))
        }
        Value::Array(values) => values.iter().find_map(|value| find_string(value, keys)),
        _ => None,
    }
}

fn find_u64(value: &Value, keys: &[&str]) -> Option<u64> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(found) = map.get(*key).and_then(Value::as_u64) {
                    return Some(found);
                }
            }
            map.values().find_map(|value| find_u64(value, keys))
        }
        Value::Array(values) => values.iter().find_map(|value| find_u64(value, keys)),
        _ => None,
    }
}

#[test]
#[ignore = "requires live Apollo credentials"]
fn apollo_read_only_health_profile_and_searches() {
    let health = cli_required("AAI_E2E_APOLLO_PROFILE", &["apollo", "health"]);
    assert_eq!(health.get("healthy").and_then(Value::as_bool), Some(true));

    let profile = cli_required("AAI_E2E_APOLLO_PROFILE", &["apollo", "users", "me"]);
    assert!(
        profile.is_object(),
        "expected Apollo user profile object: {profile}"
    );

    let people = cli_required(
        "AAI_E2E_APOLLO_PROFILE",
        &["apollo", "people", "search", "--limit", "1"],
    );
    assert!(
        people.get("_aai").is_some(),
        "expected pagination metadata on people search: {people}"
    );

    let contacts = cli_required(
        "AAI_E2E_APOLLO_PROFILE",
        &["apollo", "contacts", "search", "--limit", "1"],
    );
    assert!(
        contacts.get("_aai").is_some(),
        "expected pagination metadata on contacts search: {contacts}"
    );

    let accounts = cli_required(
        "AAI_E2E_APOLLO_PROFILE",
        &["apollo", "accounts", "search", "--limit", "1"],
    );
    assert!(
        accounts.get("_aai").is_some(),
        "expected pagination metadata on accounts search: {accounts}"
    );

    let deals = cli_required(
        "AAI_E2E_APOLLO_PROFILE",
        &["apollo", "deals", "list", "--limit", "1"],
    );
    assert!(
        deals.get("_aai").is_some(),
        "expected pagination metadata on deals list: {deals}"
    );

    let tasks = cli_required(
        "AAI_E2E_APOLLO_PROFILE",
        &["apollo", "tasks", "search", "--limit", "1"],
    );
    assert!(
        tasks.get("_aai").is_some(),
        "expected pagination metadata on tasks search: {tasks}"
    );
}

#[test]
#[ignore = "requires live Jira credentials and a disposable project"]
fn jira_issue_crud_and_projects() {
    let Some(project) = env_or_skip("AAI_E2E_JIRA_PROJECT") else {
        return;
    };
    let summary = unique("aai-e2e-jira");

    let created = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &[
            "jira",
            "issues",
            "create",
            "--project",
            &project,
            "--summary",
            &summary,
            "--description",
            "created by aai-cli live e2e",
        ],
    );
    let issue_key = str_at(&created, &["key"]).to_string();

    let _ = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &["jira", "issues", "list", "--project", &project],
    );
    let fetched = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &["jira", "issues", "get", &issue_key],
    );
    assert_eq!(str_at(&fetched, &["key"]), issue_key);
    let _ = cli_required("AAI_E2E_JIRA_PROFILE", &["jira", "projects", "list"]);
    let _ = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &["jira", "projects", "get", &project],
    );
    let _ = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &["jira", "issues", "delete", &issue_key],
    );
}

#[test]
#[ignore = "requires live Jira credentials and a Product Discovery project"]
fn jira_idea_crud_and_field_discovery() {
    let Some(project) = env_or_skip("AAI_E2E_JIRA_JPD_PROJECT") else {
        return;
    };
    let summary = unique("aai-e2e-jira-idea");

    let projects = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &[
            "jira",
            "projects",
            "list",
            "--type",
            "product_discovery",
            "--limit",
            "100",
        ],
    );
    let discovery_keys: Vec<String> = projects
        .get("values")
        .and_then(Value::as_array)
        .expect("projects list returns values")
        .iter()
        .filter_map(|value| value.get("key").and_then(Value::as_str))
        .map(str::to_string)
        .collect();
    assert!(
        discovery_keys.contains(&project),
        "expected {project} in product_discovery projects: {discovery_keys:?}"
    );

    let fields = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &["jira", "ideas", "fields", &project],
    );
    let idea_type = str_at(&fields, &["issueType", "name"]).to_string();
    let field_list = fields
        .get("fields")
        .and_then(Value::as_array)
        .expect("ideas fields returns a fields array");
    assert!(
        !field_list.is_empty(),
        "expected at least one idea field: {fields:#}"
    );
    assert!(
        field_list
            .iter()
            .any(|field| field.get("fieldId").and_then(Value::as_str) == Some("summary")),
        "expected summary in idea fields: {fields:#}"
    );
    assert!(
        field_list.iter().any(|field| {
            field
                .get("fieldId")
                .and_then(Value::as_str)
                .is_some_and(|id| id.starts_with("customfield_"))
        }),
        "expected at least one Product Discovery custom field: {fields:#}"
    );

    let listed = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &["jira", "ideas", "list", "--project", &project],
    );
    assert!(
        listed.get("issues").and_then(Value::as_array).is_some(),
        "expected issues array from ideas list: {listed:#}"
    );

    // The mutating stage creates a disposable idea and must be able to clean it
    // up, so it requires create, edit, and delete permission on the project.
    // Product Discovery contributor seats typically only have create.
    let permissions = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &[
            "jira",
            "request",
            "get",
            "/rest/api/3/mypermissions",
            "--query",
            &format!("projectKey={project}"),
            "--query",
            "permissions=CREATE_ISSUES,EDIT_ISSUES,DELETE_ISSUES",
        ],
    );
    let has_permission = |name: &str| {
        permissions
            .pointer(&format!("/permissions/{name}/havePermission"))
            .and_then(Value::as_bool)
            == Some(true)
    };
    if !(has_permission("CREATE_ISSUES")
        && has_permission("EDIT_ISSUES")
        && has_permission("DELETE_ISSUES"))
    {
        eprintln!(
            "skipping idea CRUD stage: account lacks create/edit/delete on {project} \
             (needs a Product Discovery creator seat)"
        );
        return;
    }

    let created = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &[
            "jira",
            "ideas",
            "create",
            "--project",
            &project,
            "--summary",
            &summary,
            "--description",
            "created by aai-cli live e2e",
        ],
    );
    let idea_key = str_at(&created, &["key"]).to_string();

    let fetched = cli_required("AAI_E2E_JIRA_PROFILE", &["jira", "ideas", "get", &idea_key]);
    assert_eq!(str_at(&fetched, &["key"]), idea_key);
    assert_eq!(str_at(&fetched, &["fields", "summary"]), summary);
    assert_eq!(
        str_at(&fetched, &["fields", "issuetype", "name"]),
        idea_type
    );
    assert_eq!(
        str_at(&fetched, &["fields", "project", "projectTypeKey"]),
        "product_discovery"
    );

    let transitions = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &["jira", "ideas", "transitions", "list", &idea_key],
    );
    assert!(
        transitions
            .get("transitions")
            .and_then(Value::as_array)
            .is_some(),
        "expected available idea transitions array: {transitions:#}"
    );

    let updated_summary = format!("{summary}-updated");
    let _ = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &[
            "jira",
            "ideas",
            "update",
            &idea_key,
            "--summary",
            &updated_summary,
        ],
    );
    let refetched = cli_required("AAI_E2E_JIRA_PROFILE", &["jira", "ideas", "get", &idea_key]);
    assert_eq!(str_at(&refetched, &["fields", "summary"]), updated_summary);

    let _ = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &["jira", "issues", "delete", &idea_key],
    );
}

#[test]
#[ignore = "requires live Jira credentials and a disposable project"]
fn jira_issue_comments_crud() {
    let Some(project) = env_or_skip("AAI_E2E_JIRA_PROJECT") else {
        return;
    };
    let summary = unique("aai-e2e-jira-comments");

    let created = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &[
            "jira",
            "issues",
            "create",
            "--project",
            &project,
            "--summary",
            &summary,
            "--description",
            "comments crud fixture created by aai-cli live e2e",
        ],
    );
    let issue_key = str_at(&created, &["key"]).to_string();

    let empty = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &["jira", "issues", "comments", "list", &issue_key],
    );
    assert_eq!(
        empty
            .get("comments")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(0),
        "expected zero comments on freshly created issue: {empty}"
    );

    let created_comment = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &[
            "jira",
            "issues",
            "comments",
            "create",
            &issue_key,
            "--body",
            "agent comment from aai-cli live e2e",
        ],
    );
    let comment_id = str_at(&created_comment, &["id"]).to_string();

    let listed = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &["jira", "issues", "comments", "list", &issue_key],
    );
    assert_eq!(
        listed
            .get("comments")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1),
        "expected one comment after create: {listed}"
    );

    let fetched_comment = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &["jira", "issues", "comments", "get", &issue_key, &comment_id],
    );
    assert_eq!(str_at(&fetched_comment, &["id"]), comment_id);

    let _ = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &["jira", "issues", "delete", &issue_key],
    );
}

#[test]
#[ignore = "requires live Jira credentials and a disposable Agile board"]
fn jira_sprint_crud() {
    let Some(board) = env_or_skip("AAI_E2E_JIRA_BOARD") else {
        return;
    };
    let name = unique("aai-e2e-sprint");

    let created = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &[
            "jira", "sprints", "create", "--board", &board, "--name", &name,
        ],
    );
    let sprint_id = u64_at(&created, &["id"]).to_string();

    let fetched = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &["jira", "sprints", "get", &sprint_id],
    );
    assert_eq!(str_at(&fetched, &["name"]), name);

    let listed = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &[
            "jira", "sprints", "list", "--board", &board, "--state", "future",
        ],
    );
    let values = listed
        .get("values")
        .and_then(Value::as_array)
        .expect("values array");
    assert!(
        values.iter().any(
            |v| v.get("id").and_then(Value::as_u64).map(|n| n.to_string())
                == Some(sprint_id.clone())
        ),
        "created sprint {sprint_id} not found in list: {listed}"
    );

    // No `delete sprint` command in this slice; the disposable sprint is left in `future` state on the test board.
    // The update slice will add a state=closed path and full teardown.
}

#[test]
#[ignore = "requires live Jira credentials, a disposable project, and a Scrum board"]
fn jira_sprint_issues_add() {
    let Some(project) = env_or_skip("AAI_E2E_JIRA_PROJECT") else {
        return;
    };
    let Some(board) = env_or_skip("AAI_E2E_JIRA_BOARD") else {
        return;
    };

    let issue = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &[
            "jira",
            "issues",
            "create",
            "--project",
            &project,
            "--summary",
            &unique("aai-e2e-sprint-issue"),
            "--description",
            "added to sprint by aai-cli live e2e",
        ],
    );
    let issue_key = str_at(&issue, &["key"]).to_string();

    let sprint = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &[
            "jira",
            "sprints",
            "create",
            "--board",
            &board,
            "--name",
            &unique("aai-e2e-sprint"),
        ],
    );
    let sprint_id = u64_at(&sprint, &["id"]).to_string();

    let _ = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &[
            "jira", "sprints", "issues", "add", &sprint_id, "--issues", &issue_key,
        ],
    );

    let _ = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &["jira", "issues", "delete", &issue_key],
    );
}

#[test]
#[ignore = "requires live Jira credentials and a disposable Agile board"]
fn jira_board_read() {
    let Some(board) = env_or_skip("AAI_E2E_JIRA_BOARD") else {
        return;
    };

    let fetched = cli_required("AAI_E2E_JIRA_PROFILE", &["jira", "boards", "get", &board]);
    assert_eq!(u64_at(&fetched, &["id"]).to_string(), board);

    let listed = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &["jira", "boards", "list", "--limit", "100"],
    );
    let values = listed
        .get("values")
        .and_then(Value::as_array)
        .expect("values array");
    assert!(
        values.iter().any(
            |v| v.get("id").and_then(Value::as_u64).map(|n| n.to_string()) == Some(board.clone())
        ),
        "board {board} not found in list: {listed}"
    );
}

#[test]
#[ignore = "requires live Jira credentials"]
fn jira_user_read() {
    let Some(account_id) = env_or_skip("AAI_E2E_JIRA_ACCOUNT_ID") else {
        return;
    };

    let fetched = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &["jira", "users", "get", &account_id],
    );
    assert_eq!(str_at(&fetched, &["accountId"]), account_id);
}

#[test]
#[ignore = "requires live Confluence credentials and a disposable space"]
fn confluence_page_crud_and_spaces() {
    let Some(space_id) = env_or_skip("AAI_E2E_CONFLUENCE_SPACE_ID") else {
        return;
    };
    let title = unique("aai-e2e-page");

    let created = cli_required(
        "AAI_E2E_CONFLUENCE_PROFILE",
        &[
            "confluence",
            "pages",
            "create",
            "--space-id",
            &space_id,
            "--title",
            &title,
            "--body",
            "created by aai-cli live e2e",
        ],
    );
    let page_id = str_at(&created, &["id"]).to_string();

    let _ = cli_required(
        "AAI_E2E_CONFLUENCE_PROFILE",
        &["confluence", "pages", "list"],
    );
    let fetched = cli_required(
        "AAI_E2E_CONFLUENCE_PROFILE",
        &["confluence", "pages", "get", &page_id],
    );
    assert_eq!(str_at(&fetched, &["id"]), page_id);
    let _ = cli_required(
        "AAI_E2E_CONFLUENCE_PROFILE",
        &["confluence", "spaces", "list"],
    );
    let _ = cli_required(
        "AAI_E2E_CONFLUENCE_PROFILE",
        &["confluence", "spaces", "get", &space_id],
    );
    let _ = cli_required(
        "AAI_E2E_CONFLUENCE_PROFILE",
        &["confluence", "pages", "delete", &page_id],
    );
}

#[test]
#[ignore = "requires live Confluence credentials and a disposable page"]
fn confluence_page_comments_crud() {
    let Some(space_id) = env_or_skip("AAI_E2E_CONFLUENCE_SPACE_ID") else {
        return;
    };
    let title = unique("aai-e2e-comments");

    let created_page = cli_required(
        "AAI_E2E_CONFLUENCE_PROFILE",
        &[
            "confluence",
            "pages",
            "create",
            "--space-id",
            &space_id,
            "--title",
            &title,
            "--body",
            "page for comments e2e",
        ],
    );
    let page_id = str_at(&created_page, &["id"]).to_string();

    // list before any comments
    let empty = cli_required(
        "AAI_E2E_CONFLUENCE_PROFILE",
        &["confluence", "pages", "comments", "list", &page_id],
    );
    assert_eq!(empty["page_comments"].as_array().unwrap().len(), 0);

    // create top-level comment
    let top = cli_required(
        "AAI_E2E_CONFLUENCE_PROFILE",
        &[
            "confluence",
            "pages",
            "comments",
            "create",
            &page_id,
            "--body",
            "top-level comment from e2e",
        ],
    );
    let comment_id = str_at(&top, &["id"]).to_string();

    // create reply
    let reply = cli_required(
        "AAI_E2E_CONFLUENCE_PROFILE",
        &[
            "confluence",
            "pages",
            "comments",
            "create",
            &page_id,
            "--body",
            "reply from e2e",
            "--reply-to",
            &comment_id,
        ],
    );
    let reply_id = str_at(&reply, &["id"]).to_string();
    assert_eq!(str_at(&reply, &["parentCommentId"]), comment_id);

    // list should show top-level with nested reply
    let listed = cli_required(
        "AAI_E2E_CONFLUENCE_PROFILE",
        &["confluence", "pages", "comments", "list", &page_id],
    );
    let page_comments = listed["page_comments"].as_array().unwrap();
    assert_eq!(page_comments.len(), 1);
    let replies = page_comments[0]["replies"].as_array().unwrap();
    assert_eq!(replies.len(), 1);
    assert_eq!(str_at(&replies[0], &["id"]), reply_id);

    let _ = cli_required(
        "AAI_E2E_CONFLUENCE_PROFILE",
        &["confluence", "pages", "delete", &page_id],
    );
}

#[test]
#[ignore = "requires live Confluence credentials and a disposable space"]
fn confluence_page_attachments_crud() {
    let Some(space_id) = env_or_skip("AAI_E2E_CONFLUENCE_SPACE_ID") else {
        return;
    };
    let title = unique("aai-e2e-attachments");

    let created_page = cli_required(
        "AAI_E2E_CONFLUENCE_PROFILE",
        &[
            "confluence",
            "pages",
            "create",
            "--space-id",
            &space_id,
            "--title",
            &title,
            "--body",
            "page for attachments e2e",
        ],
    );
    let page_id = str_at(&created_page, &["id"]).to_string();

    // write a temp file to upload
    let tmp = std::env::temp_dir().join("aai_e2e_attachment.txt");
    std::fs::write(&tmp, b"hello from aai-cli e2e").unwrap();
    let tmp_path = tmp.to_str().unwrap();

    // list before any attachments — expect empty
    let empty = cli_required(
        "AAI_E2E_CONFLUENCE_PROFILE",
        &["confluence", "pages", "attachments", "list", &page_id],
    );
    assert_eq!(empty["results"].as_array().unwrap().len(), 0);

    // upload
    let uploaded = cli_required(
        "AAI_E2E_CONFLUENCE_PROFILE",
        &[
            "confluence",
            "pages",
            "attachments",
            "upload",
            &page_id,
            "--file",
            tmp_path,
            "--comment",
            "e2e upload",
        ],
    );
    let attachment_id = str_at(&uploaded, &["id"]).to_string();

    // list should show the uploaded attachment
    let listed = cli_required(
        "AAI_E2E_CONFLUENCE_PROFILE",
        &["confluence", "pages", "attachments", "list", &page_id],
    );
    let results = listed["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(str_at(&results[0], &["id"]), attachment_id);

    // download to a temp path and verify content
    let dl_path = std::env::temp_dir().join("aai_e2e_downloaded.txt");
    let dl_path_str = dl_path.to_str().unwrap();
    let _ = cli_required(
        "AAI_E2E_CONFLUENCE_PROFILE",
        &[
            "confluence",
            "pages",
            "attachments",
            "download",
            &page_id,
            &attachment_id,
            "--output",
            dl_path_str,
        ],
    );
    let content = std::fs::read(&dl_path).unwrap();
    assert!(!content.is_empty());

    let _ = cli_required(
        "AAI_E2E_CONFLUENCE_PROFILE",
        &["confluence", "pages", "delete", &page_id],
    );
    let _ = std::fs::remove_file(&tmp);
    let _ = std::fs::remove_file(&dl_path);
}

#[test]
#[ignore = "requires live Jira credentials and a disposable project"]
fn jira_issue_attachments_crud() {
    let Some(project) = env_or_skip("AAI_E2E_JIRA_PROJECT") else {
        return;
    };

    let created_issue = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &[
            "jira",
            "issues",
            "create",
            "--project",
            &project,
            "--summary",
            &unique("aai-e2e-attachments"),
            "--issue-type",
            "Task",
        ],
    );
    let issue_key = str_at(&created_issue, &["key"]).to_string();

    // write a temp file
    let tmp = std::env::temp_dir().join("aai_e2e_jira_attachment.txt");
    std::fs::write(&tmp, b"hello from jira e2e").unwrap();
    let tmp_path = tmp.to_str().unwrap();

    // list before upload — expect empty
    let empty = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &["jira", "issues", "attachments", "list", &issue_key],
    );
    assert_eq!(empty["total"].as_u64().unwrap(), 0);

    // upload
    let uploaded_arr = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &[
            "jira",
            "issues",
            "attachments",
            "upload",
            &issue_key,
            "--file",
            tmp_path,
        ],
    );
    // Jira returns an array on upload success
    let attachment_id = uploaded_arr
        .as_array()
        .and_then(|a| a.first())
        .and_then(|o| o.get("id"))
        .and_then(Value::as_str)
        .unwrap()
        .to_string();

    // list should show the uploaded attachment
    let listed = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &["jira", "issues", "attachments", "list", &issue_key],
    );
    assert_eq!(listed["total"].as_u64().unwrap(), 1);
    assert_eq!(str_at(&listed["attachments"][0], &["id"]), attachment_id);

    // download
    let dl_path = std::env::temp_dir().join("aai_e2e_jira_downloaded.txt");
    let dl_path_str = dl_path.to_str().unwrap();
    let _ = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &[
            "jira",
            "issues",
            "attachments",
            "download",
            &attachment_id,
            "--output",
            dl_path_str,
        ],
    );
    let content = std::fs::read(&dl_path).unwrap();
    assert!(!content.is_empty());

    let _ = cli_required(
        "AAI_E2E_JIRA_PROFILE",
        &["jira", "issues", "delete", &issue_key],
    );
    let _ = std::fs::remove_file(&tmp);
    let _ = std::fs::remove_file(&dl_path);
}

#[test]
#[ignore = "requires live GitHub credentials and a disposable repo"]
fn github_issue_crud_repos_and_optional_prs() {
    let created = cli_required(
        "AAI_E2E_GITHUB_PROFILE",
        &[
            "github",
            "issues",
            "create",
            "--title",
            &unique("aai-e2e-github"),
            "--body",
            "created by aai-cli live e2e",
        ],
    );
    let number = u64_at(&created, &["number"]).to_string();

    let _ = cli_required("AAI_E2E_GITHUB_PROFILE", &["github", "issues", "list"]);
    let fetched = cli_required(
        "AAI_E2E_GITHUB_PROFILE",
        &["github", "issues", "get", &number],
    );
    assert_eq!(u64_at(&fetched, &["number"]).to_string(), number);
    let _ = cli_required("AAI_E2E_GITHUB_PROFILE", &["github", "repos", "list"]);
    let _ = cli_required("AAI_E2E_GITHUB_PROFILE", &["github", "repos", "get"]);
    let _ = cli_required(
        "AAI_E2E_GITHUB_PROFILE",
        &["github", "issues", "delete", &number],
    );

    let Some(head) = env_or_skip("AAI_E2E_GITHUB_PR_HEAD") else {
        return;
    };
    let Some(base) = env_or_skip("AAI_E2E_GITHUB_PR_BASE") else {
        return;
    };
    let created_pr = cli_required(
        "AAI_E2E_GITHUB_PROFILE",
        &[
            "github",
            "prs",
            "create",
            "--title",
            &unique("aai-e2e-pr"),
            "--head",
            &head,
            "--base",
            &base,
            "--body",
            "created by aai-cli live e2e",
        ],
    );
    let pr_number = u64_at(&created_pr, &["number"]).to_string();
    let head_sha = find_string(&created_pr, &["head", "sha"]);
    let _ = cli_required("AAI_E2E_GITHUB_PROFILE", &["github", "prs", "list"]);
    let _ = cli_required(
        "AAI_E2E_GITHUB_PROFILE",
        &["github", "prs", "get", &pr_number],
    );
    let comment = cli_required(
        "AAI_E2E_GITHUB_PROFILE",
        &[
            "github",
            "prs",
            "comments",
            "create",
            &pr_number,
            "--body",
            "created by aai-cli live e2e",
        ],
    );
    let comment_id = u64_at(&comment, &["id"]).to_string();
    let _ = cli_required(
        "AAI_E2E_GITHUB_PROFILE",
        &["github", "prs", "comments", "list", &pr_number],
    );
    let _ = cli_required(
        "AAI_E2E_GITHUB_PROFILE",
        &["github", "prs", "comments", "get", &pr_number, &comment_id],
    );
    let _ = cli_required(
        "AAI_E2E_GITHUB_PROFILE",
        &["github", "prs", "diff", &pr_number],
    );
    let _ = cli_required(
        "AAI_E2E_GITHUB_PROFILE",
        &["github", "prs", "files", &pr_number, "--limit", "10"],
    );
    let _ = cli_required(
        "AAI_E2E_GITHUB_PROFILE",
        &["github", "prs", "commits", &pr_number, "--limit", "10"],
    );
    let _ = cli_required(
        "AAI_E2E_GITHUB_PROFILE",
        &["github", "prs", "timeline", &pr_number, "--limit", "10"],
    );
    let _ = cli_required(
        "AAI_E2E_GITHUB_PROFILE",
        &["github", "prs", "reviews", "list", &pr_number],
    );
    let _ = cli_required(
        "AAI_E2E_GITHUB_PROFILE",
        &["github", "prs", "review-comments", "list", &pr_number],
    );
    let _ = cli_required(
        "AAI_E2E_GITHUB_PROFILE",
        &[
            "github",
            "prs",
            "reviews",
            "create",
            &pr_number,
            "--event",
            "COMMENT",
            "--body",
            "review summary from aai-cli live e2e",
        ],
    );
    if let (Some(commit_sha), Some(inline_path)) =
        (head_sha.clone(), env_or_skip("AAI_E2E_GITHUB_SOURCE_PATH"))
    {
        let inline = cli_required(
            "AAI_E2E_GITHUB_PROFILE",
            &[
                "github",
                "prs",
                "review-comments",
                "create",
                &pr_number,
                "--body",
                "inline review comment from aai-cli live e2e",
                "--path",
                &inline_path,
                "--line",
                "1",
                "--commit-id",
                &commit_sha,
            ],
        );
        let inline_id = u64_at(&inline, &["id"]).to_string();
        let _ = cli_required(
            "AAI_E2E_GITHUB_PROFILE",
            &[
                "github",
                "prs",
                "review-comments",
                "get",
                &pr_number,
                &inline_id,
            ],
        );
        let _ = cli_required(
            "AAI_E2E_GITHUB_PROFILE",
            &[
                "github",
                "prs",
                "review-comments",
                "delete",
                &pr_number,
                &inline_id,
            ],
        );
    }
    let _ = cli_required(
        "AAI_E2E_GITHUB_PROFILE",
        &["github", "prs", "close", &pr_number],
    );
}

#[test]
#[ignore = "requires live GitHub credentials and read-only repo metadata"]
fn github_read_only_endpoints() {
    let branches = cli_required(
        "AAI_E2E_GITHUB_PROFILE",
        &["github", "branches", "list", "--limit", "10"],
    );
    let branch_name = env_or_skip("AAI_E2E_GITHUB_BRANCH").or_else(|| {
        branches
            .get("values")
            .and_then(Value::as_array)
            .and_then(|values| values.first())
            .and_then(|branch| branch.get("name"))
            .and_then(Value::as_str)
            .map(str::to_string)
    });
    let Some(branch_name) = branch_name else {
        eprintln!("skipping live E2E branch: no branch name available");
        return;
    };
    let branch = cli_required(
        "AAI_E2E_GITHUB_PROFILE",
        &["github", "branches", "get", &branch_name],
    );
    let commit_sha = env_or_skip("AAI_E2E_GITHUB_COMMIT_SHA")
        .or_else(|| find_string(&branch, &["sha"]))
        .unwrap_or_else(|| branch_name.clone());
    let _ = cli_required(
        "AAI_E2E_GITHUB_PROFILE",
        &["github", "branches", "list", "--name-prefix", &branch_name],
    );
    let Some(source_path) = env_or_skip("AAI_E2E_GITHUB_SOURCE_PATH") else {
        return;
    };
    let _ = cli_required(
        "AAI_E2E_GITHUB_PROFILE",
        &["github", "source", "get", &commit_sha, &source_path],
    );
    let _ = cli_required(
        "AAI_E2E_GITHUB_PROFILE",
        &[
            "github",
            "source",
            "get",
            &commit_sha,
            &source_path,
            "--meta",
        ],
    );
    let _ = cli_required(
        "AAI_E2E_GITHUB_PROFILE",
        &[
            "github",
            "source",
            "history",
            &commit_sha,
            &source_path,
            "--limit",
            "5",
        ],
    );
}

#[test]
#[ignore = "requires live Bitbucket credentials and a disposable repo"]
fn bitbucket_repos_and_optional_prs() {
    let Some(repo) = env_or_skip("AAI_E2E_BITBUCKET_REPO") else {
        return;
    };
    let _ = cli_required("AAI_E2E_BITBUCKET_PROFILE", &["bitbucket", "repos", "list"]);
    let _ = cli_required(
        "AAI_E2E_BITBUCKET_PROFILE",
        &["bitbucket", "repos", "get", &repo],
    );

    let Some(source) = env_or_skip("AAI_E2E_BITBUCKET_PR_SOURCE") else {
        return;
    };
    let Some(destination) = env_or_skip("AAI_E2E_BITBUCKET_PR_DESTINATION") else {
        return;
    };
    let created_pr = cli_required(
        "AAI_E2E_BITBUCKET_PROFILE",
        &[
            "bitbucket",
            "prs",
            "create",
            "--repo",
            &repo,
            "--title",
            &unique("aai-e2e-bb-pr"),
            "--source",
            &source,
            "--destination",
            &destination,
            "--body",
            "created by aai-cli live e2e",
        ],
    );
    let pr_number = find_u64(&created_pr, &["id"]).expect("Bitbucket PR response missing id");
    let pr_number = pr_number.to_string();
    let _ = cli_required(
        "AAI_E2E_BITBUCKET_PROFILE",
        &["bitbucket", "prs", "list", "--repo", &repo],
    );
    let _ = cli_required(
        "AAI_E2E_BITBUCKET_PROFILE",
        &["bitbucket", "prs", "get", &pr_number, "--repo", &repo],
    );
    let _ = cli_required(
        "AAI_E2E_BITBUCKET_PROFILE",
        &["bitbucket", "prs", "diff", &pr_number, "--repo", &repo],
    );
    let _ = cli_required(
        "AAI_E2E_BITBUCKET_PROFILE",
        &["bitbucket", "prs", "diffstat", &pr_number, "--repo", &repo],
    );
    let _ = cli_required(
        "AAI_E2E_BITBUCKET_PROFILE",
        &["bitbucket", "prs", "commits", &pr_number, "--repo", &repo],
    );
    let _ = cli_required(
        "AAI_E2E_BITBUCKET_PROFILE",
        &["bitbucket", "prs", "activity", &pr_number, "--repo", &repo],
    );
    let comment = cli_required(
        "AAI_E2E_BITBUCKET_PROFILE",
        &[
            "bitbucket",
            "prs",
            "comments",
            "create",
            &pr_number,
            "--repo",
            &repo,
            "--body",
            "created by aai-cli live e2e",
        ],
    );
    let comment_id = find_u64(&comment, &["id"]).expect("Bitbucket PR comment missing id");
    let comment_id = comment_id.to_string();
    let _ = cli_required(
        "AAI_E2E_BITBUCKET_PROFILE",
        &[
            "bitbucket",
            "prs",
            "comments",
            "list",
            &pr_number,
            "--repo",
            &repo,
        ],
    );
    let _ = cli_required(
        "AAI_E2E_BITBUCKET_PROFILE",
        &[
            "bitbucket",
            "prs",
            "comments",
            "get",
            &pr_number,
            &comment_id,
            "--repo",
            &repo,
        ],
    );
    if let Some(inline_path) = env_or_skip("AAI_E2E_BITBUCKET_SOURCE_PATH") {
        let _ = cli_required(
            "AAI_E2E_BITBUCKET_PROFILE",
            &[
                "bitbucket",
                "prs",
                "comments",
                "create",
                &pr_number,
                "--repo",
                &repo,
                "--body",
                "inline comment from aai-cli live e2e",
                "--inline-path",
                &inline_path,
                "--inline-to",
                "1",
            ],
        );
        let _ = cli_required(
            "AAI_E2E_BITBUCKET_PROFILE",
            &[
                "bitbucket",
                "prs",
                "comments",
                "list",
                &pr_number,
                "--repo",
                &repo,
                "--inline-only",
            ],
        );
    }
    let _ = cli_required(
        "AAI_E2E_BITBUCKET_PROFILE",
        &["bitbucket", "prs", "decline", &pr_number, "--repo", &repo],
    );
}

#[test]
#[ignore = "requires live Bitbucket credentials and read-only repo metadata"]
fn bitbucket_read_only_endpoints() {
    let Some(repo) = env_or_skip("AAI_E2E_BITBUCKET_REPO") else {
        return;
    };
    let branches = cli_required(
        "AAI_E2E_BITBUCKET_PROFILE",
        &["bitbucket", "branches", "list", "--repo", &repo],
    );
    let branch_name = env_or_skip("AAI_E2E_BITBUCKET_BRANCH").or_else(|| {
        branches
            .get("values")
            .and_then(Value::as_array)
            .and_then(|values| values.first())
            .and_then(|branch| branch.get("name"))
            .and_then(Value::as_str)
            .map(str::to_string)
    });
    let Some(branch_name) = branch_name else {
        eprintln!("skipping live E2E branch: no branch name available");
        return;
    };
    let _ = cli_required(
        "AAI_E2E_BITBUCKET_PROFILE",
        &[
            "bitbucket",
            "branches",
            "get",
            &branch_name,
            "--repo",
            &repo,
        ],
    );
    let commits = cli_required(
        "AAI_E2E_BITBUCKET_PROFILE",
        &[
            "bitbucket",
            "commits",
            "list",
            "--repo",
            &repo,
            "--branch",
            &branch_name,
        ],
    );
    let commit_sha = env_or_skip("AAI_E2E_BITBUCKET_COMMIT_SHA").or_else(|| {
        commits
            .get("values")
            .and_then(Value::as_array)
            .and_then(|values| values.first())
            .and_then(|commit| commit.get("hash"))
            .and_then(Value::as_str)
            .map(str::to_string)
    });
    let Some(commit_sha) = commit_sha else {
        eprintln!("skipping live E2E branch: no commit SHA available");
        return;
    };
    let _ = cli_required(
        "AAI_E2E_BITBUCKET_PROFILE",
        &["bitbucket", "commits", "get", &commit_sha, "--repo", &repo],
    );
    let Some(source_path) = env_or_skip("AAI_E2E_BITBUCKET_SOURCE_PATH") else {
        return;
    };
    let _ = cli_required(
        "AAI_E2E_BITBUCKET_PROFILE",
        &[
            "bitbucket",
            "source",
            "get",
            &commit_sha,
            &source_path,
            "--repo",
            &repo,
        ],
    );
    let _ = cli_required(
        "AAI_E2E_BITBUCKET_PROFILE",
        &[
            "bitbucket",
            "source",
            "history",
            &commit_sha,
            &source_path,
            "--repo",
            &repo,
        ],
    );
}

#[test]
#[ignore = "requires live Google Gmail credentials and a test recipient"]
fn gmail_message_send_list_get_delete() {
    let Some(to) = env_or_skip("AAI_E2E_EMAIL_TO") else {
        return;
    };
    let sent = cli_required(
        "AAI_E2E_GMAIL_PROFILE",
        &[
            "email",
            "messages",
            "send",
            "--to",
            &to,
            "--subject",
            &unique("aai-e2e-gmail"),
            "--body",
            "created by aai-cli live e2e",
        ],
    );
    let id = str_at(&sent, &["id"]).to_string();
    let _ = cli_required("AAI_E2E_GMAIL_PROFILE", &["email", "messages", "list"]);
    let _ = cli_required("AAI_E2E_GMAIL_PROFILE", &["email", "messages", "get", &id]);
    let _ = cli_required(
        "AAI_E2E_GMAIL_PROFILE",
        &["email", "messages", "delete", &id],
    );
}

#[test]
#[ignore = "requires live Zoho Mail credentials and a test recipient"]
fn zoho_mail_message_send_list_get_delete() {
    let Some(to) = env_or_skip("AAI_E2E_EMAIL_TO") else {
        return;
    };
    let sent = cli_required(
        "AAI_E2E_ZOHO_MAIL_PROFILE",
        &[
            "email",
            "messages",
            "send",
            "--to",
            &to,
            "--subject",
            &unique("aai-e2e-zoho-mail"),
            "--body",
            "created by aai-cli live e2e",
        ],
    );
    let id = find_string(&sent, &["messageId", "messageIdString", "id"])
        .or_else(|| find_u64(&sent, &["id"]).map(|id| id.to_string()))
        .expect("Zoho Mail send response missing message id");
    let _ = cli_required("AAI_E2E_ZOHO_MAIL_PROFILE", &["email", "messages", "list"]);
    let _ = cli_required(
        "AAI_E2E_ZOHO_MAIL_PROFILE",
        &["email", "messages", "get", &id],
    );
    let _ = cli_required(
        "AAI_E2E_ZOHO_MAIL_PROFILE",
        &["email", "messages", "delete", &id],
    );
}

#[test]
#[ignore = "requires live Google Calendar credentials"]
fn google_calendar_event_crud() {
    let created = cli_required(
        "AAI_E2E_GOOGLE_CALENDAR_PROFILE",
        &[
            "calendar",
            "events",
            "create",
            "--summary",
            &unique("aai-e2e-google-calendar"),
            "--description",
            "created by aai-cli live e2e",
            "--start",
            "2030-01-01T10:00:00Z",
            "--end",
            "2030-01-01T10:30:00Z",
        ],
    );
    let id = str_at(&created, &["id"]).to_string();
    let _ = cli_required(
        "AAI_E2E_GOOGLE_CALENDAR_PROFILE",
        &["calendar", "events", "list"],
    );
    let _ = cli_required(
        "AAI_E2E_GOOGLE_CALENDAR_PROFILE",
        &["calendar", "events", "get", &id],
    );
    let _ = cli_required(
        "AAI_E2E_GOOGLE_CALENDAR_PROFILE",
        &["calendar", "events", "delete", &id],
    );
}

#[test]
#[ignore = "requires live Zoho Calendar credentials"]
fn zoho_calendar_event_crud() {
    let created = cli_required(
        "AAI_E2E_ZOHO_CALENDAR_PROFILE",
        &[
            "calendar",
            "events",
            "create",
            "--summary",
            &unique("aai-e2e-zoho-calendar"),
            "--description",
            "created by aai-cli live e2e",
            "--start",
            "20300101T100000Z",
            "--end",
            "20300101T103000Z",
        ],
    );
    let id = find_string(&created, &["eventId", "uid", "id"])
        .expect("Zoho Calendar response missing event id");
    let _ = cli_required(
        "AAI_E2E_ZOHO_CALENDAR_PROFILE",
        &["calendar", "events", "list"],
    );
    let _ = cli_required(
        "AAI_E2E_ZOHO_CALENDAR_PROFILE",
        &["calendar", "events", "get", &id],
    );
    let _ = cli_required(
        "AAI_E2E_ZOHO_CALENDAR_PROFILE",
        &["calendar", "events", "delete", &id],
    );
}

#[test]
#[ignore = "requires live Pipedrive credentials and disposable CRM records"]
fn pipedrive_crm_crud_and_labels() {
    let label = cli_required(
        "AAI_E2E_PIPEDRIVE_PROFILE",
        &[
            "pipedrive",
            "labels",
            "leads",
            "create",
            "--name",
            &unique("aai-e2e-label"),
            "--color",
            "green",
        ],
    );
    let lead_label_id = find_string(&label, &["id"])
        .or_else(|| find_u64(&label, &["id"]).map(|id| id.to_string()))
        .expect("Pipedrive lead label response missing id");

    let org = cli_required(
        "AAI_E2E_PIPEDRIVE_PROFILE",
        &[
            "pipedrive",
            "organizations",
            "create",
            "--name",
            &unique("aai-e2e-org"),
            "--address",
            "1 Test Way",
        ],
    );
    let org_id = find_u64(&org, &["id"])
        .map(|id| id.to_string())
        .expect("Pipedrive organization response missing id");

    let person = cli_required(
        "AAI_E2E_PIPEDRIVE_PROFILE",
        &[
            "pipedrive",
            "persons",
            "create",
            "--name",
            &unique("aai-e2e-person"),
            "--org-id",
            &org_id,
            "--email",
            "aai-e2e@example.com",
        ],
    );
    let person_id = find_u64(&person, &["id"])
        .map(|id| id.to_string())
        .expect("Pipedrive person response missing id");

    let deal = cli_required(
        "AAI_E2E_PIPEDRIVE_PROFILE",
        &[
            "pipedrive",
            "deals",
            "create",
            "--title",
            &unique("aai-e2e-deal"),
            "--person-id",
            &person_id,
            "--org-id",
            &org_id,
            "--value",
            "10",
            "--currency",
            "USD",
        ],
    );
    let deal_id = find_u64(&deal, &["id"])
        .map(|id| id.to_string())
        .expect("Pipedrive deal response missing id");

    let lead = cli_required(
        "AAI_E2E_PIPEDRIVE_PROFILE",
        &[
            "pipedrive",
            "leads",
            "create",
            "--title",
            &unique("aai-e2e-lead"),
            "--person-id",
            &person_id,
            "--organization-id",
            &org_id,
            "--label-ids",
            &lead_label_id,
        ],
    );
    let lead_id = find_string(&lead, &["id"]).expect("Pipedrive lead response missing id");

    let _ = cli_required("AAI_E2E_PIPEDRIVE_PROFILE", &["pipedrive", "leads", "list"]);
    let _ = cli_required(
        "AAI_E2E_PIPEDRIVE_PROFILE",
        &["pipedrive", "leads", "get", &lead_id],
    );
    let _ = cli_required(
        "AAI_E2E_PIPEDRIVE_PROFILE",
        &["pipedrive", "persons", "get", &person_id],
    );
    let _ = cli_required(
        "AAI_E2E_PIPEDRIVE_PROFILE",
        &["pipedrive", "organizations", "get", &org_id],
    );
    let _ = cli_required(
        "AAI_E2E_PIPEDRIVE_PROFILE",
        &["pipedrive", "deals", "get", &deal_id],
    );
    let _ = cli_required(
        "AAI_E2E_PIPEDRIVE_PROFILE",
        &["pipedrive", "deals", "activities", &deal_id, "--limit", "1"],
    );
    let _ = cli_required(
        "AAI_E2E_PIPEDRIVE_PROFILE",
        &["pipedrive", "deals", "notes", &deal_id, "--limit", "1"],
    );
    let _ = cli_required(
        "AAI_E2E_PIPEDRIVE_PROFILE",
        &["pipedrive", "deals", "view", &deal_id, "--limit", "1"],
    );
    let _ = cli_required(
        "AAI_E2E_PIPEDRIVE_PROFILE",
        &["pipedrive", "deals", "flow", &deal_id, "--limit", "1"],
    );
    let _ = cli_required(
        "AAI_E2E_PIPEDRIVE_PROFILE",
        &["pipedrive", "activities", "list", "--deal-id", &deal_id],
    );
    let _ = cli_required(
        "AAI_E2E_PIPEDRIVE_PROFILE",
        &["pipedrive", "notes", "list", "--deal-id", &deal_id],
    );
    let _ = cli_required(
        "AAI_E2E_PIPEDRIVE_PROFILE",
        &["pipedrive", "labels", "leads", "list"],
    );

    let _ = cli_required(
        "AAI_E2E_PIPEDRIVE_PROFILE",
        &["pipedrive", "leads", "delete", &lead_id],
    );
    let _ = cli_required(
        "AAI_E2E_PIPEDRIVE_PROFILE",
        &["pipedrive", "deals", "delete", &deal_id],
    );
    let _ = cli_required(
        "AAI_E2E_PIPEDRIVE_PROFILE",
        &["pipedrive", "persons", "delete", &person_id],
    );
    let _ = cli_required(
        "AAI_E2E_PIPEDRIVE_PROFILE",
        &["pipedrive", "organizations", "delete", &org_id],
    );
    let _ = cli_required(
        "AAI_E2E_PIPEDRIVE_PROFILE",
        &["pipedrive", "labels", "leads", "delete", &lead_label_id],
    );
}

#[test]
#[ignore = "requires live Slack credentials and a seeded test channel"]
fn slack_channel_read_and_canvas_download() {
    let Some(channel_id) = env_or_skip("AAI_E2E_SLACK_CHANNEL") else {
        return;
    };

    let channels = cli_required(
        "AAI_E2E_SLACK_PROFILE",
        &["slack", "channels", "list", "--limit", "5"],
    );
    assert!(channels["channels"].as_array().is_some());

    let channel = cli_required(
        "AAI_E2E_SLACK_PROFILE",
        &["slack", "channels", "get", &channel_id],
    );
    assert_eq!(str_at(&channel, &["channel", "id"]), channel_id);

    let files = cli_required(
        "AAI_E2E_SLACK_PROFILE",
        &["slack", "files", "list", &channel_id],
    );
    assert!(files["files"].as_array().is_some());

    let first_file_id = files["files"][0]["id"].as_str().map(str::to_string);
    if let Some(file_id) = first_file_id {
        let dl_path = std::env::temp_dir().join(unique("aai_e2e_slack_file"));
        let dl_path_str = dl_path.to_str().unwrap();
        let downloaded = cli_required(
            "AAI_E2E_SLACK_PROFILE",
            &[
                "slack",
                "files",
                "download",
                &file_id,
                "--output",
                dl_path_str,
            ],
        );
        assert_eq!(str_at(&downloaded, &["file_id"]), file_id);
        let content = std::fs::read(&dl_path).unwrap();
        assert!(!content.is_empty());
        let _ = std::fs::remove_file(&dl_path);
    }

    let bookmarks = cli_required(
        "AAI_E2E_SLACK_PROFILE",
        &["slack", "bookmarks", "list", &channel_id],
    );
    assert!(bookmarks["bookmarks"].as_array().is_some());
    assert_eq!(bookmarks["has_more"], false);

    let links = cli_required(
        "AAI_E2E_SLACK_PROFILE",
        &["slack", "links", "list", &channel_id],
    );
    assert!(links["links"].as_array().is_some());

    let canvas_id = channel["channel"]["canvas_id"].as_str().map(str::to_string);
    if let Some(canvas_id) = canvas_id {
        let dl_path = std::env::temp_dir().join(unique("aai_e2e_slack_canvas"));
        let dl_path_str = dl_path.to_str().unwrap();
        let downloaded = cli_required(
            "AAI_E2E_SLACK_PROFILE",
            &[
                "slack",
                "canvas",
                "download",
                &channel_id,
                "--output",
                dl_path_str,
            ],
        );
        assert_eq!(str_at(&downloaded, &["canvas_id"]), canvas_id);
        let content = std::fs::read(&dl_path).unwrap();
        assert!(!content.is_empty());
        let _ = std::fs::remove_file(&dl_path);
    }
}

#[test]
#[ignore = "requires live Google Drive credentials"]
fn google_drive_read_export_and_upload() {
    let profile = "AAI_E2E_DRIVE_PROFILE";

    let about = cli_required(profile, &["drive", "about", "get"]);
    assert!(about["storageQuota"].is_object(), "{about:#}");
    assert!(about["exportFormats"].is_object(), "{about:#}");

    // An account with no shared drives returns an empty array, not an error, so the
    // assertion is on the shape rather than on the account having one.
    let drives = cli_required(profile, &["drive", "drives", "list", "--limit", "5"]);
    assert!(drives["drives"].as_array().is_some(), "{drives:#}");

    let files = cli_required(profile, &["drive", "files", "list", "--limit", "5"]);
    assert!(files["files"].as_array().is_some(), "{files:#}");
    assert!(
        files["_aai"]["pagination"]["returned_count"].is_u64(),
        "{files:#}"
    );

    let folders = cli_required(profile, &["drive", "folders", "list", "--limit", "5"]);
    for folder in folders["files"].as_array().expect("folders array") {
        assert_eq!(
            folder["mimeType"], "application/vnd.google-apps.folder",
            "{folder:#}"
        );
    }

    // Google-native documents hold no bytes, so this branch proves the export path
    // rather than the alt=media one.
    if let Some(native_id) = env_or_skip("AAI_E2E_DRIVE_NATIVE_DOC") {
        let export_path = std::env::temp_dir().join(unique("aai-e2e-drive-export"));
        let exported = cli_required(
            profile,
            &[
                "drive",
                "files",
                "download",
                &native_id,
                "--output",
                export_path.to_str().unwrap(),
            ],
        );
        assert!(
            str_at(&exported, &["mime_type"]).starts_with("application/vnd.google-apps."),
            "{exported:#}"
        );
        assert!(exported["exported_mime_type"].is_string(), "{exported:#}");
        assert!(!std::fs::read(&export_path).unwrap().is_empty());
        let _ = std::fs::remove_file(&export_path);
    }

    // The one write. aai-cli has no Drive delete by design, so this branch only runs
    // against a folder the operator is willing to keep debris in: the uploaded file is
    // named with the shared `aai-e2e-` prefix and must be removed by hand or by a
    // folder-level cleanup.
    let Some(parent) = env_or_skip("AAI_E2E_DRIVE_UPLOAD_PARENT") else {
        return;
    };
    let body = "aai-cli drive live test\n";
    let name = format!("{}.txt", unique("aai-e2e-drive"));
    let source = std::env::temp_dir().join(&name);
    std::fs::write(&source, body).expect("write upload source");

    let uploaded = cli_required(
        profile,
        &[
            "drive",
            "files",
            "upload",
            source.to_str().unwrap(),
            "--parent",
            &parent,
        ],
    );
    let _ = std::fs::remove_file(&source);
    let file_id = str_at(&uploaded, &["id"]).to_string();
    assert_eq!(str_at(&uploaded, &["name"]), name);

    let fetched = cli_required(profile, &["drive", "files", "get", &file_id]);
    assert_eq!(str_at(&fetched, &["id"]), file_id);

    let download_path = std::env::temp_dir().join(unique("aai-e2e-drive-download"));
    let downloaded = cli_required(
        profile,
        &[
            "drive",
            "files",
            "download",
            &file_id,
            "--output",
            download_path.to_str().unwrap(),
        ],
    );
    assert_eq!(str_at(&downloaded, &["file_id"]), file_id);
    // An ordinary blob is streamed byte-for-byte, never exported.
    assert!(downloaded["exported_mime_type"].is_null(), "{downloaded:#}");
    let round_tripped = std::fs::read(&download_path).expect("read downloaded file");
    assert_eq!(String::from_utf8(round_tripped).unwrap(), body);
    let _ = std::fs::remove_file(&download_path);

    let permissions = cli_required(profile, &["drive", "permissions", "list", &file_id]);
    assert!(
        permissions["permissions"].as_array().is_some(),
        "{permissions:#}"
    );
}

#[test]
#[ignore = "requires live OpenPanel root/read client credentials and a seeded project"]
fn openpanel_projects_insights_and_profiles_read() {
    let Some(project_id) = env_or_skip("AAI_E2E_OPENPANEL_PROJECT") else {
        return;
    };

    let projects = cli_required(
        "AAI_E2E_OPENPANEL_ROOT_PROFILE",
        &["openpanel", "projects", "list"],
    );
    assert!(projects["data"].as_array().is_some());

    let project = cli_required(
        "AAI_E2E_OPENPANEL_ROOT_PROFILE",
        &["openpanel", "projects", "get", &project_id],
    );
    assert_eq!(str_at(&project, &["data", "id"]), project_id);

    let metrics = cli_required(
        "AAI_E2E_OPENPANEL_READ_PROFILE",
        &[
            "openpanel",
            "insights",
            "metrics",
            "--project-id",
            &project_id,
            "--range",
            "7d",
        ],
    );
    assert!(metrics.is_object());

    let pages = cli_required(
        "AAI_E2E_OPENPANEL_READ_PROFILE",
        &[
            "openpanel",
            "insights",
            "pages",
            "--project-id",
            &project_id,
            "--range",
            "7d",
            "--limit",
            "5",
        ],
    );
    assert!(pages.is_array() || pages.is_object());

    let events = cli_required(
        "AAI_E2E_OPENPANEL_READ_PROFILE",
        &[
            "openpanel",
            "events",
            "export",
            "--project-id",
            &project_id,
            "--limit",
            "5",
        ],
    );
    assert!(events["data"].as_array().is_some());

    let profiles = cli_required(
        "AAI_E2E_OPENPANEL_READ_PROFILE",
        &[
            "openpanel",
            "profiles",
            "list",
            "--project-id",
            &project_id,
            "--limit",
            "5",
        ],
    );
    assert!(profiles.is_array() || profiles.is_object());
}

#[test]
#[ignore = "requires live SharePoint credentials and a site the account can read"]
fn sharepoint_site_drive_and_item_reads() {
    let Some(site) = env_or_skip("AAI_E2E_SHAREPOINT_SITE") else {
        return;
    };

    let site_value = cli_required(
        "AAI_E2E_SHAREPOINT_PROFILE",
        &["sharepoint", "sites", "get", &site],
    );
    let site_id = str_at(&site_value, &["id"]).to_string();
    assert!(!site_id.is_empty());

    let drives = cli_required(
        "AAI_E2E_SHAREPOINT_PROFILE",
        &["sharepoint", "drives", "list", &site_id],
    );
    let drive_id = str_at(&drives, &["value", "0", "id"]).to_string();
    assert!(!drive_id.is_empty());

    let items = cli_required(
        "AAI_E2E_SHAREPOINT_PROFILE",
        &["sharepoint", "items", "list", &drive_id, "--limit", "5"],
    );
    assert!(items["value"].as_array().is_some());
    // the signed download url is trimmed out of listings on purpose
    assert!(items["value"][0]
        .get("@microsoft.graph.downloadUrl")
        .is_none());

    let delta = cli_required(
        "AAI_E2E_SHAREPOINT_PROFILE",
        &["sharepoint", "items", "delta", &drive_id],
    );
    // a completed delta enumeration carries a deltaLink and must NOT report more pages
    assert!(delta["@odata.deltaLink"].is_string() || delta["@odata.nextLink"].is_string());

    let Some(file_id) = items["value"]
        .as_array()
        .and_then(|values| values.iter().find(|item| item.get("file").is_some()))
        .and_then(|item| item.get("id"))
        .and_then(Value::as_str)
        .map(str::to_string)
    else {
        return;
    };

    let output = std::env::temp_dir().join(unique("aai_e2e_sharepoint_item"));
    let downloaded = cli_required(
        "AAI_E2E_SHAREPOINT_PROFILE",
        &[
            "sharepoint",
            "items",
            "download",
            &drive_id,
            &file_id,
            "--output",
            output.to_str().unwrap(),
        ],
    );
    assert!(downloaded["bytes"].as_u64().unwrap_or(0) > 0);
    let _ = std::fs::remove_file(&output);
}
