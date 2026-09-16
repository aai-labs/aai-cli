use serde_json::{json, Value};

const COLLECTION_KEYS: &[&str] = &[
    "value",
    "values",
    "results",
    "issues",
    "comments",
    "messages",
    "items",
    "data",
    "files",
    "jobs",
    "workflow_runs",
    "channels",
    "links",
    "bookmarks",
    "fields",
    "drives",
    "permissions",
];

pub(crate) fn annotate(value: Value, command_args: &[String]) -> Value {
    let analysis = analyze(&value, command_args);
    let metadata = json!({ "pagination": analysis });
    match value {
        Value::Object(mut object) => {
            object.remove("_aai_provider_next_url");
            object.insert("_aai".to_string(), metadata);
            Value::Object(object)
        }
        Value::Array(results) => json!({
            "results": results,
            "_aai": metadata,
        }),
        other => json!({
            "result": other,
            "_aai": metadata,
        }),
    }
}

fn analyze(value: &Value, command_args: &[String]) -> Value {
    let returned_count = returned_count(value);
    let continuation = continuation(value);
    let truncated = value
        .get("truncated")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let explicitly_more = explicit_more(value);
    let explicitly_complete = explicit_completion(value);
    let collection = returned_count.is_some() || looks_like_collection_command(command_args);
    let next_command = next_command(command_args, continuation.as_ref(), collection);

    let (status, has_more, instruction) = if continuation.is_some() || truncated || explicitly_more
    {
        (
            "more_available",
            Some(true),
            next_command
                .as_ref()
                .map(|_| "Run next_command to retrieve more results.")
                .unwrap_or(
                    "Use the continuation parameters with the same endpoint to retrieve the next batch.",
                ),
        )
    } else if explicitly_complete == Some(true) {
        (
            "complete",
            Some(false),
            "The provider indicates there are no more results.",
        )
    } else if collection {
        (
            "unknown",
            None,
            next_command
                .as_ref()
                .map(|_| {
                    "The provider returned no continuation marker. Run next_command with a larger limit to check for more results."
                })
                .unwrap_or(
                    "The provider returned no continuation marker. Use a larger limit or the provider's pagination parameters to check for more results.",
                ),
        )
    } else {
        (
            "not_applicable",
            Some(false),
            "This response does not appear to be a paginated result collection.",
        )
    };

    json!({
        "status": status,
        "has_more": has_more,
        "returned_count": returned_count,
        "continuation": continuation.map(|value| value.to_json()),
        "next_command": next_command,
        "instruction": instruction,
    })
}

#[derive(Debug)]
struct Continuation {
    source: &'static str,
    parameters: Vec<(String, String)>,
    next_url: Option<String>,
}

impl Continuation {
    fn to_json(&self) -> Value {
        json!({
            "source": self.source,
            "parameters": self
                .parameters
                .iter()
                .map(|(key, value)| json!({ "key": key, "value": value }))
                .collect::<Vec<_>>(),
            "next_url": self.next_url,
        })
    }
}

fn continuation(value: &Value) -> Option<Continuation> {
    for (pointer, key, source) in [
        ("/nextPageToken", "pageToken", "nextPageToken"),
        ("/next_page_token", "page_token", "next_page_token"),
        (
            "/additional_data/next_cursor",
            "cursor",
            "additional_data.next_cursor",
        ),
        (
            "/response_metadata/next_cursor",
            "cursor",
            "response_metadata.next_cursor",
        ),
        (
            "/additional_data/pagination/next_start",
            "start",
            "additional_data.pagination.next_start",
        ),
        ("/next_cursor", "cursor", "next_cursor"),
        ("/next_start", "start", "next_start"),
    ] {
        if let Some(next) = scalar_string(value.pointer(pointer)) {
            return Some(Continuation {
                source,
                parameters: vec![(key.to_string(), next)],
                next_url: None,
            });
        }
    }

    for (pointer, source) in [("/next", "next"), ("/_links/next", "_links.next")] {
        if let Some(next_url) = value.pointer(pointer).and_then(Value::as_str) {
            if !next_url.is_empty() {
                return Some(Continuation {
                    source,
                    parameters: query_parameters(next_url),
                    next_url: Some(next_url.to_string()),
                });
            }
        }
    }

    if let Some(next_url) = value.get("_aai_provider_next_url").and_then(Value::as_str) {
        return Some(Continuation {
            source: "Link rel=next",
            parameters: query_parameters(next_url),
            next_url: Some(next_url.to_string()),
        });
    }

    if let Some(next_url) = value.get("@odata.nextLink").and_then(Value::as_str) {
        if !next_url.is_empty() {
            return Some(Continuation {
                source: "@odata.nextLink",
                parameters: query_parameters(next_url),
                next_url: Some(next_url.to_string()),
            });
        }
    }

    offset_continuation(value)
}

fn offset_continuation(value: &Value) -> Option<Continuation> {
    let start = value.get("startAt").and_then(Value::as_u64)?;
    let size = value
        .get("maxResults")
        .and_then(Value::as_u64)
        .or_else(|| returned_count(value).map(|count| count as u64))?;
    let total = value.get("total").and_then(Value::as_u64)?;
    let next = start.saturating_add(size);
    (next < total).then(|| Continuation {
        source: "startAt/maxResults/total",
        parameters: vec![("startAt".to_string(), next.to_string())],
        next_url: None,
    })
}

fn explicit_completion(value: &Value) -> Option<bool> {
    if value.get("isLast").and_then(Value::as_bool) == Some(true)
        || value
            .pointer("/pagination/more_items_in_collection")
            .and_then(Value::as_bool)
            == Some(false)
        || value
            .pointer("/additional_data/pagination/more_items_in_collection")
            .and_then(Value::as_bool)
            == Some(false)
        || value.get("has_more").and_then(Value::as_bool) == Some(false)
        || value.get("truncated").and_then(Value::as_bool) == Some(false)
    {
        return Some(true);
    }
    None
}

fn explicit_more(value: &Value) -> bool {
    value.get("isLast").and_then(Value::as_bool) == Some(false)
        || value
            .pointer("/pagination/more_items_in_collection")
            .and_then(Value::as_bool)
            == Some(true)
        || value
            .pointer("/additional_data/pagination/more_items_in_collection")
            .and_then(Value::as_bool)
            == Some(true)
        || value.get("has_more").and_then(Value::as_bool) == Some(true)
}

fn returned_count(value: &Value) -> Option<usize> {
    match value {
        Value::Array(values) => Some(values.len()),
        Value::Object(object) => COLLECTION_KEYS
            .iter()
            .find_map(|key| object.get(*key).and_then(array_count))
            .or_else(|| value.pointer("/data/items").and_then(array_count)),
        _ => None,
    }
}

fn array_count(value: &Value) -> Option<usize> {
    value.as_array().map(Vec::len)
}

fn scalar_string(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(value) if !value.is_empty() => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn query_parameters(url: &str) -> Vec<(String, String)> {
    reqwest::Url::parse(url)
        .ok()
        .map(|url| {
            url.query_pairs()
                .map(|(k, v)| (k.into(), v.into()))
                .collect()
        })
        .unwrap_or_default()
}

fn looks_like_collection_command(args: &[String]) -> bool {
    // Some resource names double as collection actions — `github prs files` is a
    // listing, but `drive files get` and `drive files download` address one file. An
    // explicit single-item action settles it, so it is checked first. This only
    // decides the *guess*; a response that actually contains a collection key is
    // still counted by `returned_count`.
    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "get" | "download" | "upload"))
    {
        return false;
    }
    args.iter().any(|arg| {
        matches!(
            arg.as_str(),
            "list"
                | "search"
                | "history"
                | "timeline"
                | "files"
                | "commits"
                | "activity"
                | "fields"
        )
    })
}

fn next_command(
    args: &[String],
    continuation: Option<&Continuation>,
    collection: bool,
) -> Option<String> {
    if contains_sensitive_input(args) {
        return None;
    }
    if is_generic_request(args) {
        if let Some(continuation) = continuation {
            if !continuation.parameters.is_empty() {
                return Some(generic_next_command(args, &continuation.parameters));
            }
        }
    }
    if collection {
        return larger_limit_command(args);
    }
    None
}

fn contains_sensitive_input(args: &[String]) -> bool {
    args.iter().any(|arg| {
        matches!(
            arg.as_str(),
            "--json" | "--value" | "secrets" | "--api-token" | "--token" | "--password"
        ) || arg.starts_with("--json=")
            || arg.starts_with("--value=")
            || arg.starts_with("--api-token=")
            || arg.starts_with("--token=")
            || arg.starts_with("--password=")
    })
}

fn is_generic_request(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "request")
}

fn generic_next_command(args: &[String], parameters: &[(String, String)]) -> String {
    let mut output = Vec::new();
    let replacement_keys = parameters
        .iter()
        .map(|(key, _)| key.as_str())
        .collect::<Vec<_>>();
    let mut index = 0;
    while index < args.len() {
        if args[index] == "--query" {
            if let Some(query) = args.get(index + 1) {
                let key = query.split_once('=').map(|(key, _)| key).unwrap_or("");
                if !replacement_keys.contains(&key) {
                    output.push(args[index].clone());
                    output.push(query.clone());
                }
            }
            index += 2;
            continue;
        }
        if let Some(query) = args[index].strip_prefix("--query=") {
            let key = query.split_once('=').map(|(key, _)| key).unwrap_or("");
            if !replacement_keys.contains(&key) {
                output.push(args[index].clone());
            }
            index += 1;
            continue;
        }
        output.push(args[index].clone());
        index += 1;
    }
    for (key, value) in parameters {
        output.push("--query".to_string());
        output.push(format!("{key}={value}"));
    }
    shell_command(&output)
}

fn larger_limit_command(args: &[String]) -> Option<String> {
    let mut output = args.to_vec();
    let mut index = 0;
    while index < output.len() {
        if output[index] == "--limit" {
            let value = output.get(index + 1)?.parse::<u64>().ok()?;
            output[index + 1] = value.saturating_mul(2).max(value + 1).to_string();
            return Some(shell_command(&output));
        }
        if let Some(value) = output[index].strip_prefix("--limit=") {
            let value = value.parse::<u64>().ok()?;
            output[index] = format!("--limit={}", value.saturating_mul(2).max(value + 1));
            return Some(shell_command(&output));
        }
        index += 1;
    }
    None
}

fn shell_command(args: &[String]) -> String {
    args.iter()
        .map(|arg| shell_quote(arg))
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_quote(value: &str) -> String {
    if !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_./:=,".contains(&byte))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annotates_cursor_with_generic_next_command() {
        let value = json!({
            "data": [{"id": 1}],
            "additional_data": {"next_cursor": "abc 123"}
        });
        let args = strings(&[
            "aai-cli",
            "pipedrive",
            "request",
            "get",
            "/api/v2/deals",
            "--query",
            "status=open",
        ]);
        let output = annotate(value, &args);

        assert_eq!(output["_aai"]["pagination"]["status"], "more_available");
        assert_eq!(output["_aai"]["pagination"]["has_more"], true);
        assert_eq!(
            output["_aai"]["pagination"]["next_command"],
            "aai-cli pipedrive request get /api/v2/deals --query status=open --query 'cursor=abc 123'"
        );
    }

    #[test]
    fn detects_http_link_next_url_metadata() {
        let value = json!({
            "results": [{"id": 1}],
            "_aai_provider_next_url": "https://api.github.com/items?state=open&page=2"
        });
        let args = strings(&[
            "aai-cli",
            "github",
            "request",
            "get",
            "/items",
            "--query",
            "state=open",
        ]);
        let output = annotate(value, &args);

        assert!(output.get("_aai_provider_next_url").is_none());
        assert_eq!(
            output["_aai"]["pagination"]["next_command"],
            "aai-cli github request get /items --query state=open --query page=2"
        );
    }

    #[test]
    fn detects_microsoft_odata_next_link_and_value_count() {
        let value = json!({
            "value": [{"id": "one"}, {"id": "two"}],
            "@odata.nextLink": "https://graph.microsoft.com/v1.0/users?$skiptoken=opaque"
        });
        let args = strings(&["aai-cli", "microsoft", "contacts", "list", "--limit", "2"]);
        let output = annotate(value, &args);

        assert_eq!(output["_aai"]["pagination"]["returned_count"], 2);
        assert_eq!(output["_aai"]["pagination"]["status"], "more_available");
        assert_eq!(
            output["_aai"]["pagination"]["continuation"]["source"],
            "@odata.nextLink"
        );
    }

    #[test]
    fn typed_truncated_result_suggests_larger_limit() {
        let value = json!({"values": [1, 2], "truncated": true});
        let args = strings(&["aai-cli", "github", "branches", "list", "--limit", "2"]);
        let output = annotate(value, &args);

        assert_eq!(output["_aai"]["pagination"]["has_more"], true);
        assert_eq!(
            output["_aai"]["pagination"]["next_command"],
            "aai-cli github branches list --limit 4"
        );
    }

    #[test]
    fn wraps_array_results_to_make_pagination_visible() {
        let output = annotate(
            json!([1, 2]),
            &strings(&["aai-cli", "github", "repos", "list"]),
        );
        assert_eq!(output["results"], json!([1, 2]));
        assert_eq!(output["_aai"]["pagination"]["status"], "unknown");
    }

    #[test]
    fn marks_non_collection_response_not_applicable() {
        let output = annotate(
            json!({"id": 1}),
            &strings(&["aai-cli", "github", "repos", "get"]),
        );
        assert_eq!(output["_aai"]["pagination"]["status"], "not_applicable");
        assert_eq!(output["_aai"]["pagination"]["has_more"], false);
    }

    #[test]
    fn a_single_file_action_is_not_mistaken_for_a_file_listing() {
        // "files" is a collection keyword for `github prs files`, but here it is the
        // resource name and `download` is the action.
        let output = annotate(
            json!({"output": "./report.docx", "bytes": 24713, "file_id": "1XyZ"}),
            &strings(&[
                "aai-cli",
                "drive",
                "files",
                "download",
                "1XyZ",
                "--output",
                "./report.docx",
            ]),
        );
        assert_eq!(output["_aai"]["pagination"]["status"], "not_applicable");
        assert_eq!(output["_aai"]["pagination"]["has_more"], false);
    }

    #[test]
    fn a_listing_that_shares_the_resource_name_is_still_a_collection() {
        let output = annotate(
            json!({"files": [{"id": "1XyZ"}], "nextPageToken": "next"}),
            &strings(&["aai-cli", "drive", "files", "list", "--limit", "1"]),
        );
        assert_eq!(output["_aai"]["pagination"]["status"], "more_available");
        assert_eq!(
            output["_aai"]["pagination"]["next_command"],
            "aai-cli drive files list --limit 2"
        );
    }

    #[test]
    fn nested_resource_get_is_not_mistaken_for_collection() {
        let output = annotate(
            json!({"id": 1}),
            &strings(&["aai-cli", "github", "prs", "comments", "get", "7"]),
        );
        assert_eq!(output["_aai"]["pagination"]["status"], "not_applicable");
    }

    #[test]
    fn non_truncated_aggregate_is_complete() {
        let output = annotate(
            json!({"values": [1, 2], "truncated": false}),
            &strings(&["aai-cli", "github", "branches", "list", "--limit", "10"]),
        );
        assert_eq!(output["_aai"]["pagination"]["status"], "complete");
        assert_eq!(output["_aai"]["pagination"]["has_more"], false);
    }

    #[test]
    fn does_not_echo_commands_with_json_bodies() {
        let value = json!({"data": [], "nextPageToken": "secret-next"});
        let args = strings(&[
            "aai-cli",
            "github",
            "request",
            "post",
            "/endpoint",
            "--json",
            r#"{"token":"secret"}"#,
        ]);
        let output = annotate(value, &args);
        assert!(output["_aai"]["pagination"]["next_command"].is_null());
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }
}
