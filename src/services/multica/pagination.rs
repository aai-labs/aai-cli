use super::{
    endpoint::{Endpoint, Paging},
    SERVICE,
};
use crate::{config::Profile, error::AppError, http::ApiClient};
use clap::ArgMatches;
use reqwest::{header::HeaderMap, Method, Url};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};

type Parameters = BTreeMap<String, String>;

fn header(headers: &HeaderMap, key: &str) -> Option<String> {
    headers
        .get(key)
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
}
fn continuation(headers: &HeaderMap, body: &Value) -> Parameters {
    let mut next = Parameters::new();
    if let Some(cursor) = header(headers, "x-agent-tasks-next-cursor") {
        next.insert("before".into(), cursor);
    }
    if let (Some(before), Some(id)) = (
        header(headers, "x-multica-next-before"),
        header(headers, "x-multica-next-before-id"),
    ) {
        next.insert("before".into(), before);
        next.insert("before_id".into(), id);
    }
    if let Some(cursor) = body
        .get("next_cursor")
        .and_then(Value::as_str)
        .filter(|v| !v.is_empty())
    {
        next.insert("cursor".into(), cursor.to_owned());
    }
    next
}
fn truncated(headers: &HeaderMap) -> bool {
    [
        "x-comments-truncated",
        "x-timeline-truncated",
        "x-task-runs-truncated",
    ]
    .iter()
    .any(|key| header(headers, key).is_some_and(|v| v != "false" && v != "0"))
}
fn set_query(url: &mut Url, params: &Parameters) {
    let mut query: Parameters = url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    query.extend(params.clone());
    url.set_query(None);
    if !query.is_empty() {
        url.query_pairs_mut().extend_pairs(query);
    }
}
fn cursor_params(url: &Url) -> Parameters {
    url.query_pairs()
        .filter(|(k, _)| matches!(k.as_ref(), "offset" | "cursor" | "before" | "before_id"))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect()
}
fn array<'a>(body: &'a Value, key: &str) -> Option<&'a Vec<Value>> {
    if key.is_empty() {
        body.as_array()
    } else {
        body.get(key)?.as_array()
    }
}
fn finish(
    mut body: Value,
    key: &str,
    records: Vec<Value>,
    next: Parameters,
    incomplete: bool,
) -> Value {
    let count = records.len();
    if key.is_empty() {
        body = json!({"results": records});
    } else {
        body[key] = Value::Array(records);
    }
    body["_aai_collection"] =
        json!({"returned_count":count,"parameters":next,"incomplete":incomplete});
    body
}

pub(super) async fn collect(
    client: &ApiClient,
    profile: &Profile,
    e: &Endpoint,
    args: &ArgMatches,
    mut url: Url,
) -> Result<Value, AppError> {
    let limit = *args.get_one::<usize>("limit").expect("collection default");
    let mut skip = *args
        .get_one::<usize>("page_skip")
        .expect("collection default");
    if limit == 0 {
        return Ok(finish(
            json!({}),
            e.key,
            Vec::new(),
            Parameters::new(),
            true,
        ));
    }
    if matches!(
        e.paging,
        Paging::Offset | Paging::OffsetUntilShort | Paging::Cursor | Paging::AgentTasks
    ) {
        set_query(
            &mut url,
            &Parameters::from([("limit".into(), e.page_size.to_string())]),
        );
    }
    let mut records = Vec::new();
    let mut seen_ids = HashSet::new();
    let mut visited = HashSet::new();
    let mut incomplete = e.paging == Paging::Capped;
    loop {
        if !visited.insert(url.to_string()) {
            return Err(AppError::internal(
                SERVICE,
                e.command,
                "provider repeated a pagination cursor",
            ));
        }
        let (body, headers) = client
            .request_with_headers(
                SERVICE,
                e.command,
                profile,
                Method::GET,
                url.to_string(),
                None,
            )
            .await?;
        incomplete |= truncated(&headers);
        let key = if e.command == "issues timeline" && body.get("entries").is_some() {
            "entries"
        } else {
            e.key
        };
        let values = array(&body, key).ok_or_else(|| {
            AppError::internal(
                SERVICE,
                e.command,
                "provider returned an unexpected collection shape",
            )
        })?;
        let mut next = continuation(&headers, &body);
        if matches!(e.paging, Paging::Offset | Paging::OffsetUntilShort) {
            let offset = url
                .query_pairs()
                .find(|(k, _)| k == "offset")
                .and_then(|(_, v)| v.parse::<usize>().ok())
                .unwrap_or(0);
            let end = offset.saturating_add(values.len());
            let total = if e.paging == Paging::Offset {
                body.get("total").and_then(Value::as_u64)
            } else {
                None
            };
            if !values.is_empty()
                && total.map_or(values.len() == e.page_size, |total| (end as u64) < total)
            {
                next.insert("offset".into(), end.to_string());
            }
            if values.is_empty() && total.is_some_and(|n| n > end as u64) {
                incomplete = true;
            }
        }
        if next.is_empty()
            && (body.get("has_more").and_then(Value::as_bool) == Some(true)
                || body.get("has_more_before").and_then(Value::as_bool) == Some(true))
        {
            incomplete = true;
        }
        let mut consumed = skip.min(values.len());
        for value in values.iter().skip(skip) {
            if records.len() == limit {
                break;
            }
            consumed += 1;
            if let Some(id) = value.get("id").and_then(Value::as_str) {
                if !seen_ids.insert(id.to_owned()) {
                    continue;
                }
            }
            records.push(value.clone());
        }
        if e.paging == Paging::Comments {
            records.sort_by(|a, b| {
                a.get("created_at")
                    .and_then(Value::as_str)
                    .cmp(&b.get("created_at").and_then(Value::as_str))
                    .then_with(|| {
                        a.get("id")
                            .and_then(Value::as_str)
                            .cmp(&b.get("id").and_then(Value::as_str))
                    })
            });
        }
        if consumed < values.len() {
            next = cursor_params(&url);
            next.insert("page_skip".into(), consumed.to_string());
            return Ok(finish(body, key, records, next, incomplete));
        }
        if records.len() >= limit || next.is_empty() {
            return Ok(finish(body, key, records, next, incomplete));
        }
        if values.is_empty() {
            return Err(AppError::internal(
                SERVICE,
                e.command,
                "provider returned an empty page with a continuation cursor",
            ));
        }
        skip = 0;
        set_query(&mut url, &next);
    }
}

/// The generic request stays single-page, but must retain server continuation headers.
pub(super) fn raw(mut value: Value, headers: &HeaderMap) -> Value {
    let next = continuation(headers, &value);
    let incomplete = truncated(headers);
    if !next.is_empty() || incomplete {
        if value.is_array() {
            value = json!({"results":value});
        }
        if let Some(object) = value.as_object_mut() {
            object.insert(
                "_aai_collection".into(),
                json!({"parameters":next,"incomplete":incomplete}),
            );
        }
    }
    value
}
