use std::time::{Duration, Instant};

use reqwest::Method;
use serde_json::{json, Value};

use crate::{
    cli::*,
    config::Context,
    error::AppError,
    http::ApiClient,
    services::{
        generic_request,
        shared::{enc, merkys_base, CtxProfile},
    },
};

const SERVICE: &str = "merkys";

/// Upper bound on message pages followed to find the newest assistant answer.
const MAX_MESSAGE_PAGES: usize = 40;

pub(crate) async fn dispatch(
    client: &ApiClient,
    ctx: &Context,
    command: MerkysCommand,
) -> Result<Value, AppError> {
    match command.resource {
        MerkysResource::Ask(args) => ask(client, ctx, args).await,
        MerkysResource::Research(command) => research(client, ctx, command).await,
        MerkysResource::Conversations(command) => conversations(client, ctx, command).await,
        MerkysResource::Documents(command) => match command.action {
            MerkysDocumentsAction::List(args) => {
                let mut query = Query::new();
                query.push("limit", Some(&args.limit.to_string()));
                get(client, ctx, "documents.list", "/api/documents", query).await
            }
        },
        MerkysResource::Sources(command) => match command.action {
            MerkysSourcesAction::List(args) => {
                let mut query = Query::new();
                query.push("limit", Some(&args.limit.to_string()));
                query.push("offset", Some(&args.offset.to_string()));
                get(client, ctx, "sources.list", "/api/sources/", query).await
            }
        },
        MerkysResource::Assistants(command) => match command.action {
            MerkysAssistantsAction::List => {
                get(
                    client,
                    ctx,
                    "assistants.list",
                    "/api/custom-assistants/",
                    Query::new(),
                )
                .await
            }
        },
        MerkysResource::Request(args) => {
            let base = merkys_base(ctx.profile(), "request")?;
            generic_request::dispatch(client, ctx, SERVICE, base, args).await
        }
    }
}

// --- ask ---

/// Ask one question: make (or reuse) a conversation, send the message, then read the
/// answer back from the conversation. The chat endpoint returns only the answer text;
/// the cited sources live on the stored assistant message.
async fn ask(client: &ApiClient, ctx: &Context, args: MerkysAsk) -> Result<Value, AppError> {
    let operation = "ask";
    let question = non_empty(&args.question, operation, "question")?;
    let conversation_id = match args.conversation_id {
        Some(id) => id,
        None => create_conversation(client, ctx, operation).await?,
    };

    let chat = send(
        client,
        ctx,
        operation,
        Method::POST,
        &format!("/api/chat/{}", enc(&conversation_id)),
        Some(json!({ "message": question })),
    )
    .await?;

    // The answer is already paid for, so a failure to load sources must not hide it.
    let (assistant, warning) = match last_assistant_message(client, ctx, &conversation_id).await {
        Ok(message) => (message, None),
        Err(error) => (
            None,
            Some(format!("could not load sources: {}", error.message)),
        ),
    };
    Ok(shape_ask(
        &conversation_id,
        &chat,
        assistant.as_ref(),
        warning.as_deref(),
    ))
}

async fn create_conversation(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
) -> Result<String, AppError> {
    let created = send(
        client,
        ctx,
        operation,
        Method::POST,
        "/api/conversation/",
        Some(json!({})),
    )
    .await?;
    created
        .pointer("/conversation/id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            AppError::internal(
                SERVICE,
                operation,
                "conversation create response had no conversation.id",
            )
        })
}

/// Follow message pages to the end and return the newest assistant message.
async fn last_assistant_message(
    client: &ApiClient,
    ctx: &Context,
    conversation_id: &str,
) -> Result<Option<Value>, AppError> {
    let mut newest = None;
    let mut page_token: Option<String> = None;
    for _ in 0..MAX_MESSAGE_PAGES {
        let mut query = Query::new();
        query.push("limit", Some("50"));
        query.push("page_token", page_token.as_deref());
        let page = get(
            client,
            ctx,
            "ask",
            &format!("/api/conversation/{}/messages", enc(conversation_id)),
            query,
        )
        .await?;
        if let Some(message) = newest_assistant(&page) {
            newest = Some(message.clone());
        }
        page_token = page
            .get("next_page_token")
            .and_then(Value::as_str)
            .filter(|token| !token.is_empty())
            .map(str::to_string);
        if page_token.is_none() {
            break;
        }
    }
    Ok(newest)
}

fn newest_assistant(page: &Value) -> Option<&Value> {
    page.get("items")?
        .as_array()?
        .iter()
        .rev()
        .find(|item| item.get("role").and_then(Value::as_str) == Some("assistant"))
}

fn shape_ask(
    conversation_id: &str,
    chat: &Value,
    assistant: Option<&Value>,
    warning: Option<&str>,
) -> Value {
    let answer = chat
        .get("message")
        .or_else(|| assistant.and_then(|message| message.get("text")))
        .cloned()
        .unwrap_or(Value::Null);
    let field = |name: &str| {
        assistant
            .and_then(|message| message.get(name))
            .cloned()
            .unwrap_or(Value::Null)
    };
    let sources = if assistant.is_some() {
        match field("sources") {
            Value::Null => json!([]),
            sources => sources,
        }
    } else {
        Value::Null
    };
    let mut output = json!({
        "conversation_id": conversation_id,
        "message_id": field("id"),
        "answer": answer,
        "sources": sources,
        "detect_conflicts": chat.get("detect_conflicts").cloned().unwrap_or(Value::Null),
        "faithfulness": field("faithfulness"),
        "total_tokens": field("total_tokens"),
    });
    if let Some(warning) = warning {
        output["warning"] = json!(warning);
    }
    output
}

// --- deep research ---

async fn research(
    client: &ApiClient,
    ctx: &Context,
    command: MerkysResearchCommand,
) -> Result<Value, AppError> {
    match command.action {
        MerkysResearchAction::Start(args) => research_start(client, ctx, args).await,
        MerkysResearchAction::Get(args) => get_run(client, ctx, "research.get", &args.run_id).await,
        MerkysResearchAction::Stop(args) => {
            send(
                client,
                ctx,
                "research.stop",
                Method::POST,
                &format!("/v1/chat/deep-research/runs/{}/stop", enc(&args.run_id)),
                None,
            )
            .await
        }
    }
}

async fn research_start(
    client: &ApiClient,
    ctx: &Context,
    args: MerkysResearchStart,
) -> Result<Value, AppError> {
    let operation = "research.start";
    let question = non_empty(&args.question, operation, "question")?;
    let conversation_id = match args.conversation_id {
        Some(id) => id,
        None => create_conversation(client, ctx, operation).await?,
    };
    let started = send(
        client,
        ctx,
        operation,
        Method::POST,
        "/v1/chat/deep-research",
        Some(json!({ "question": question, "conversation_id": conversation_id })),
    )
    .await?;
    let run_id = started
        .get("run_id")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::internal(SERVICE, operation, "start response had no run_id"))?
        .to_string();

    if !args.wait {
        return Ok(json!({ "run_id": run_id, "conversation_id": conversation_id }));
    }

    let deadline = Instant::now() + Duration::from_secs(args.timeout_secs);
    let poll = Duration::from_secs(args.poll_secs.max(1));
    loop {
        let run = get_run(client, ctx, operation, &run_id).await?;
        if run_is_finished(&run) {
            return Ok(run);
        }
        if Instant::now() + poll > deadline {
            let mut error = AppError::internal(
                SERVICE,
                operation,
                format!(
                    "run {run_id} did not finish within {}s; it is still running. \
                     Use `merkys research get {run_id}` to check it later",
                    args.timeout_secs
                ),
            );
            error.code = "timeout";
            error.details = Some(json!({
                "run_id": run_id,
                "conversation_id": conversation_id,
                "status": run.get("status").cloned().unwrap_or(Value::Null),
            }));
            return Err(error);
        }
        tokio::time::sleep(poll).await;
    }
}

async fn get_run(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    run_id: &str,
) -> Result<Value, AppError> {
    get(
        client,
        ctx,
        operation,
        &format!("/v1/chat/deep-research/runs/{}", enc(run_id)),
        Query::new(),
    )
    .await
}

fn run_is_finished(run: &Value) -> bool {
    matches!(
        run.get("status").and_then(Value::as_str),
        Some("completed" | "failed")
    )
}

// --- conversations ---

async fn conversations(
    client: &ApiClient,
    ctx: &Context,
    command: MerkysConversationsCommand,
) -> Result<Value, AppError> {
    match command.action {
        MerkysConversationsAction::List(args) => {
            let query = page_query(&args.limit, args.page_token.as_deref());
            get(
                client,
                ctx,
                "conversations.list",
                "/api/conversation/",
                query,
            )
            .await
        }
        MerkysConversationsAction::Search(args) => {
            let mut query = page_query(&args.page.limit, args.page.page_token.as_deref());
            query.push("q", Some(&args.query));
            get(
                client,
                ctx,
                "conversations.search",
                "/api/conversation/search",
                query,
            )
            .await
        }
        MerkysConversationsAction::Messages(args) => {
            let query = page_query(&args.limit, args.page_token.as_deref());
            get(
                client,
                ctx,
                "conversations.messages",
                &format!("/api/conversation/{}/messages", enc(&args.conversation_id)),
                query,
            )
            .await
        }
    }
}

fn page_query(limit: &u32, page_token: Option<&str>) -> Query {
    let mut query = Query::new();
    query.push("limit", Some(&limit.to_string()));
    query.push("page_token", page_token);
    query
}

// --- plumbing ---

fn non_empty<'a>(value: &'a str, operation: &'static str, name: &str) -> Result<&'a str, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::invalid_input(
            SERVICE,
            operation,
            format!("{name} must not be empty"),
        ));
    }
    Ok(value)
}

async fn send(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> Result<Value, AppError> {
    let url = format!("{}{}", merkys_base(ctx.profile(), operation)?, path);
    client
        .request(SERVICE, operation, ctx.profile(), method, url, body)
        .await
}

async fn get(
    client: &ApiClient,
    ctx: &Context,
    operation: &'static str,
    path: &str,
    query: Query,
) -> Result<Value, AppError> {
    let mut url = format!("{}{}", merkys_base(ctx.profile(), operation)?, path);
    query.append_to(&mut url);
    client
        .request(SERVICE, operation, ctx.profile(), Method::GET, url, None)
        .await
}

#[derive(Clone, Debug, Default)]
struct Query(Vec<(String, String)>);

impl Query {
    fn new() -> Self {
        Self::default()
    }

    fn push(&mut self, key: &str, value: Option<&str>) {
        if let Some(value) = value.filter(|value| !value.is_empty()) {
            self.0.push((key.to_string(), value.to_string()));
        }
    }

    fn append_to(&self, url: &mut String) {
        for (index, (key, value)) in self.0.iter().enumerate() {
            url.push(if index == 0 { '?' } else { '&' });
            url.push_str(&enc(key));
            url.push('=');
            url.push_str(&enc(value));
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

    fn context(base_url: &str) -> Context {
        Context {
            profile: Profile {
                provider: Some("merkys".to_string()),
                auth_type: Some("api_key_header".to_string()),
                base_url: Some(base_url.to_string()),
                api_token: Some("cb_test".to_string()),
                ..Profile::default()
            },
            secrets_file: std::path::PathBuf::new(),
            key_file: std::path::PathBuf::new(),
        }
    }

    #[test]
    fn newest_assistant_picks_the_last_assistant_message() {
        let page = json!({ "items": [
            { "role": "user", "text": "q1" },
            { "role": "assistant", "text": "a1" },
            { "role": "user", "text": "q2" },
            { "role": "assistant", "text": "a2" },
        ]});
        assert_eq!(newest_assistant(&page).unwrap()["text"], "a2");
        assert!(newest_assistant(&json!({ "items": [] })).is_none());
        assert!(newest_assistant(&json!({})).is_none());
    }

    #[test]
    fn shape_ask_combines_chat_answer_with_stored_sources() {
        let chat =
            json!({ "status": "success", "message": "pong [[1]]", "detect_conflicts": null });
        let assistant = json!({
            "id": "m1",
            "text": "pong [[1]]",
            "sources": [{ "index": 1, "title": "a.pdf", "page_number": 3 }],
            "faithfulness": { "score": 1 },
            "total_tokens": 12,
        });
        let output = shape_ask("c1", &chat, Some(&assistant), None);
        assert_eq!(output["conversation_id"], "c1");
        assert_eq!(output["message_id"], "m1");
        assert_eq!(output["answer"], "pong [[1]]");
        assert_eq!(output["sources"][0]["title"], "a.pdf");
        assert_eq!(output["total_tokens"], 12);
        assert!(output.get("warning").is_none());
    }

    #[test]
    fn shape_ask_keeps_the_answer_when_sources_could_not_be_loaded() {
        let chat = json!({ "message": "hello" });
        let output = shape_ask("c1", &chat, None, Some("could not load sources: boom"));
        assert_eq!(output["answer"], "hello");
        assert!(output["sources"].is_null());
        assert_eq!(output["warning"], "could not load sources: boom");
    }

    #[test]
    fn shape_ask_reports_no_sources_as_an_empty_list() {
        let assistant = json!({ "id": "m1", "text": "hi", "sources": null });
        let output = shape_ask("c1", &json!({ "message": "hi" }), Some(&assistant), None);
        assert_eq!(output["sources"], json!([]));
    }

    #[test]
    fn run_is_finished_only_for_completed_and_failed() {
        for status in ["completed", "failed"] {
            assert!(run_is_finished(&json!({ "status": status })));
        }
        for status in ["pending", "running"] {
            assert!(!run_is_finished(&json!({ "status": status })));
        }
        assert!(!run_is_finished(&json!({})));
    }

    #[test]
    fn query_skips_empty_values_and_encodes() {
        let mut query = Query::new();
        query.push("q", Some("a b&c"));
        query.push("page_token", None);
        query.push("empty", Some(""));
        let mut url = "http://x/p".to_string();
        query.append_to(&mut url);
        assert_eq!(url, "http://x/p?q=a%20b%26c");
    }

    #[tokio::test]
    async fn ask_creates_a_conversation_sends_the_message_and_returns_sources() {
        let (address, server) = serve(vec![
            json_response(r#"{"status":"success","conversation":{"id":"c1"}}"#),
            json_response(
                r#"{"status":"success","message":"WebGLM is X [[1]]","detect_conflicts":null}"#,
            ),
            json_response(
                r#"{"status":"success","items":[
                    {"id":"u1","role":"user","text":"what is webglm"},
                    {"id":"a1","role":"assistant","text":"WebGLM is X [[1]]",
                     "sources":[{"index":1,"title":"webglm.pdf","page_number":1,"excerpt":"..."}],
                     "total_tokens":42}]}"#,
            ),
        ]);
        let client = ApiClient::new().unwrap();
        let ctx = context(&format!("http://{address}"));

        let output = ask(
            &client,
            &ctx,
            MerkysAsk {
                question: "what is webglm".to_string(),
                conversation_id: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(output["conversation_id"], "c1");
        assert_eq!(output["message_id"], "a1");
        assert_eq!(output["answer"], "WebGLM is X [[1]]");
        assert_eq!(output["sources"][0]["title"], "webglm.pdf");

        let requests = server.join().unwrap();
        assert!(requests[0].starts_with("post /api/conversation/ "));
        assert!(requests[0].contains("x-api-key: cb_test"));
        assert!(requests[1].starts_with("post /api/chat/c1 "));
        assert!(request_body(&requests[1]).contains(r#""message":"what is webglm""#));
        assert!(requests[2].starts_with("get /api/conversation/c1/messages?limit=50 "));
    }

    #[tokio::test]
    async fn research_with_wait_polls_until_the_run_completes() {
        let (address, server) = serve(vec![
            json_response(r#"{"run_id":"r1"}"#),
            json_response(r#"{"run_id":"r1","status":"running"}"#),
            json_response(r#"{"run_id":"r1","status":"completed","final_report":"done"}"#),
        ]);
        let client = ApiClient::new().unwrap();
        let ctx = context(&format!("http://{address}"));

        let run = research_start(
            &client,
            &ctx,
            MerkysResearchStart {
                question: "q".to_string(),
                conversation_id: Some("c1".to_string()),
                wait: true,
                timeout_secs: 30,
                poll_secs: 1,
            },
        )
        .await
        .unwrap();

        assert_eq!(run["status"], "completed");
        assert_eq!(run["final_report"], "done");
        let requests = server.join().unwrap();
        assert!(requests[0].starts_with("post /v1/chat/deep-research "));
        assert!(request_body(&requests[0]).contains(r#""conversation_id":"c1""#));
        assert!(requests[1].starts_with("get /v1/chat/deep-research/runs/r1 "));
    }

    #[tokio::test]
    async fn research_wait_times_out_with_the_run_id_in_the_error() {
        let (address, server) = serve(vec![
            json_response(r#"{"run_id":"r9"}"#),
            json_response(r#"{"run_id":"r9","status":"running"}"#),
        ]);
        let client = ApiClient::new().unwrap();
        let ctx = context(&format!("http://{address}"));

        let error = research_start(
            &client,
            &ctx,
            MerkysResearchStart {
                question: "q".to_string(),
                conversation_id: Some("c1".to_string()),
                wait: true,
                timeout_secs: 0,
                poll_secs: 1,
            },
        )
        .await
        .unwrap_err();

        assert_eq!(error.code, "timeout");
        assert_eq!(error.details.unwrap()["run_id"], "r9");
        server.join().unwrap();
    }

    #[tokio::test]
    async fn missing_base_url_is_a_config_error() {
        let client = ApiClient::new().unwrap();
        let mut ctx = context("http://unused");
        ctx.profile.base_url = None;
        let error = get(
            &client,
            &ctx,
            "documents.list",
            "/api/documents",
            Query::new(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, "config_error");
    }
}
