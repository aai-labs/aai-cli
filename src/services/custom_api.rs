use crate::{
    cli::{CustomApiAction, CustomApiCommand},
    config::Profile,
    error::AppError,
    http::custom_api::{execute, refused, Request},
    input,
};
use serde_json::{Map, Value};

pub async fn dispatch(profile: &Profile, command: CustomApiCommand) -> Result<Value, AppError> {
    let CustomApiAction::Request(args) = command.action;
    let request = if let Some(path) = args.request_json {
        serde_json::from_value(
            input::read_json_arg("custom_api", "request", Some(&path))
                .map_err(|_| refused("Invalid request JSON"))?,
        )
        .map_err(|_| refused("Invalid request object"))?
    } else {
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
            headers.push(crate::config::CustomHeader {
                name: name.into(),
                value: Some(value.into()),
                ..Default::default()
            });
        }
        Request {
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
        }
    };
    execute(profile, request).await
}
