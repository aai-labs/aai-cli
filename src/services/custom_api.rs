use crate::{
    cli::{CustomApiAction, CustomApiCommand, CustomApiRequest},
    config::Profile,
    error::AppError,
    http::custom_api::{execute, refused, Request, RequestHeader},
    input,
};
use serde_json::{Map, Value};

pub async fn dispatch(profile: &Profile, command: CustomApiCommand) -> Result<Value, AppError> {
    let CustomApiAction::Request(args) = command.action;
    execute(profile, build_request(args)?).await
}

fn build_request(args: CustomApiRequest) -> Result<Request, AppError> {
    if let Some(path) = args.request_json {
        return serde_json::from_value(
            input::read_json_arg("custom_api", "request", Some(&path))
                .map_err(|_| refused("Invalid request JSON"))?,
        )
        .map_err(|_| refused("Invalid request object"));
    }
    let mut query = Map::new();
    for item in args.query {
        let (name, value) = item
            .split_once('=')
            .ok_or_else(|| refused("Query must use name=value"))?;
        if name.is_empty() {
            return Err(refused("Query name must not be empty"));
        }
        query.insert(name.to_string(), Value::String(value.to_string()));
    }
    let mut headers = Vec::new();
    for item in args.header {
        let (name, value) = item
            .split_once('=')
            .ok_or_else(|| refused("Header must use name=value"))?;
        headers.push(RequestHeader {
            name: name.into(),
            value: value.into(),
        });
    }
    Ok(Request {
        method: args.method,
        url: args.url.unwrap_or_default(),
        query,
        body: args
            .json
            .map(|path| {
                input::read_json_arg("custom_api", "request", Some(&path))
                    .map_err(|_| refused("Invalid body JSON"))
            })
            .transpose()?
            .unwrap_or(Value::Null),
        headers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Command};
    use clap::Parser;
    use serde_json::json;

    fn parse(args: &[&str]) -> Result<CustomApiRequest, clap::Error> {
        let cli = Cli::try_parse_from(
            ["aai-cli", "custom-api", "request"]
                .iter()
                .chain(args)
                .copied(),
        )?;
        let Command::CustomApi(command) = cli.command else {
            panic!("expected custom-api command");
        };
        let CustomApiAction::Request(args) = command.action;
        Ok(args)
    }

    fn request_from(args: &[&str]) -> Result<Request, AppError> {
        build_request(parse(args).expect("valid CLI arguments"))
    }

    #[test]
    fn cli_requires_url_or_request_json_and_keeps_them_exclusive() {
        assert!(parse(&[]).is_err());
        assert!(parse(&["--url", "https://a.example/v1", "--request-json", "-"]).is_err());
        for flag in ["--method=POST", "--query=a=1", "--header=A=1", "--json={}"] {
            assert!(parse(&[flag, "--request-json", "-"]).is_err(), "{flag}");
        }
    }

    #[test]
    fn flags_build_a_get_request_by_default() {
        let request = request_from(&["--url", "https://a.example/v1/items"]).unwrap();
        assert_eq!(
            serde_json::to_value(request).unwrap(),
            json!({"method":"GET","url":"https://a.example/v1/items","query":{},"body":null,"headers":[]})
        );
    }

    #[test]
    fn flags_build_query_headers_and_inline_json_body() {
        let request = request_from(&[
            "--url",
            "https://a.example/v1/items",
            "--method",
            "post",
            "--query",
            "q=a=b",
            "--query",
            "empty=",
            "--query",
            "q=last",
            "--header",
            "X-Trace=one=two",
            "--json",
            r#"{"name":"sample"}"#,
        ])
        .unwrap();
        assert_eq!(
            serde_json::to_value(request).unwrap(),
            json!({
                "method":"post",
                "url":"https://a.example/v1/items",
                "query":{"q":"last","empty":""},
                "body":{"name":"sample"},
                "headers":[{"name":"X-Trace","value":"one=two"}]
            })
        );
    }

    #[test]
    fn json_body_is_read_from_a_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("body.json");
        std::fs::write(&path, "[1,2]").unwrap();
        let request = request_from(&[
            "--url",
            "https://a.example/v1",
            "--json",
            path.to_str().unwrap(),
        ])
        .unwrap();
        assert_eq!(request.body, json!([1, 2]));
    }

    #[test]
    fn malformed_flags_are_invalid_input() {
        for (args, message) in [
            (&["--query", "novalue"][..], "Query must use name=value"),
            (&["--query", "=x"][..], "Query name must not be empty"),
            (&["--header", "X-Novalue"][..], "Header must use name=value"),
            (&["--json", "{not json"][..], "Invalid body JSON"),
            (
                &["--json", "/nonexistent/body.json"][..],
                "Invalid body JSON",
            ),
        ] {
            let mut full = vec!["--url", "https://a.example/v1"];
            full.extend_from_slice(args);
            let error = request_from(&full).unwrap_err();
            assert_eq!(error.code, "invalid_input", "{args:?}");
            assert_eq!(error.message, message, "{args:?}");
        }
    }

    #[test]
    fn request_json_accepts_complete_objects_and_defaults() {
        let request = request_from(&[
            "--request-json",
            r#"{"url":"https://a.example/v1","query":{"tag":["a","b"]},"body":"raw text","headers":[{"name":"Accept","value":"text/plain"}]}"#,
        ])
        .unwrap();
        assert_eq!(request.method, "GET");
        assert_eq!(request.query["tag"], json!(["a", "b"]));
        assert_eq!(request.body, json!("raw text"));
        assert_eq!(request.headers[0].value, "text/plain");
    }

    #[test]
    fn request_json_rejects_malformed_and_unknown_fields() {
        for (raw, message) in [
            ("{oops", "Invalid request JSON"),
            (r#"{"method":"GET"}"#, "Invalid request object"),
            (
                r#"{"url":"https://a.example","extra":1}"#,
                "Invalid request object",
            ),
            (
                r#"{"url":"https://a.example","headers":[{"name":"X","value_env":"HOME"}]}"#,
                "Invalid request object",
            ),
            (
                r#"{"url":"https://a.example","headers":[{"name":"X","value":"v","secret":true}]}"#,
                "Invalid request object",
            ),
        ] {
            let error = request_from(&["--request-json", raw]).unwrap_err();
            assert_eq!(error.message, message, "{raw}");
        }
    }

    #[tokio::test]
    async fn dispatch_refuses_non_custom_api_profiles_before_any_request() {
        let command = CustomApiCommand {
            action: CustomApiAction::Request(parse(&["--url", "https://a.example/v1"]).unwrap()),
        };
        let profile = Profile {
            provider: Some("github".into()),
            base_url: Some("https://a.example/v1".into()),
            token: Some("fixture-token".into()),
            ..Default::default()
        };
        let error = dispatch(&profile, command).await.unwrap_err();
        assert_eq!(error.message, "Select a custom_api profile");
    }
}
