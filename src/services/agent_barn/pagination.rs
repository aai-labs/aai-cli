use super::{invalid, Endpoint, SERVICE};
use crate::{config::Profile, error::AppError, http::ApiClient};
use clap::ArgMatches;
use reqwest::{Method, Url};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};

type Parameters = BTreeMap<String, String>;
fn set_query(url: &mut Url, next: &Parameters) {
    let pairs: Vec<_> = url
        .query_pairs()
        .filter(|(k, _)| !next.contains_key(k.as_ref()))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    url.set_query(None);
    url.query_pairs_mut().extend_pairs(pairs).extend_pairs(next);
}
fn finish(
    mut body: Value,
    e: &Endpoint,
    records: Vec<Value>,
    next: Parameters,
    incomplete: bool,
) -> Value {
    let count = records.len();
    if e.key.is_empty() {
        body = json!({"results":records});
    } else {
        body[&e.key] = json!(records);
    }
    body["_aai_collection"] =
        json!({"source":SERVICE,"returned_count":count,"parameters":next,"incomplete":incomplete});
    body
}
fn next_page(e: &Endpoint, body: &Value, url: &Url, count: usize) -> Result<Parameters, AppError> {
    let mut next = Parameters::new();
    match e.paging.as_str() {
        "page" => {
            let page = body
                .get("page")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid("provider omitted page"))?;
            let size = body
                .get("page_size")
                .and_then(Value::as_u64)
                .filter(|s| *s > 0)
                .ok_or_else(|| invalid("provider omitted page_size"))?;
            let total = body
                .get("total")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid("provider omitted total"))?;
            if page.saturating_mul(size) < total {
                next.insert("page".into(), (page + 1).to_string());
            }
        }
        "cursor" if body.get("has_more").and_then(Value::as_bool) == Some(true) => {
            let cursor = body
                .get("next_cursor")
                .and_then(Value::as_object)
                .ok_or_else(|| invalid("provider omitted next_cursor despite has_more"))?;
            for key in ["before_occurred_at", "before_id"] {
                next.insert(
                    key.into(),
                    cursor
                        .get(key)
                        .and_then(Value::as_str)
                        .ok_or_else(|| {
                            invalid("provider returned an incomplete conversation cursor")
                        })?
                        .to_owned(),
                );
            }
        }
        "chat" if url.query_pairs().any(|(k, _)| k == "after_id") && count == 500 => {
            let id = body
                .as_array()
                .and_then(|rows| rows.last())
                .and_then(|v| v.get("id"))
                .and_then(Value::as_str)
                .ok_or_else(|| invalid("provider omitted chat message ID"))?;
            next.insert("after_id".into(), id.to_owned());
        }
        "snapshot" if body.get("has_more").and_then(Value::as_bool) == Some(true) => {
            next.insert(
                "snapshot_id".into(),
                body.get("next_snapshot_id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| invalid("provider omitted next_snapshot_id"))?
                    .to_owned(),
            );
        }
        _ => {}
    }
    if !next.is_empty() && count == 0 {
        return Err(invalid("provider returned an empty page with continuation"));
    }
    // Include page size and all cursor components in generated continuation commands.
    for (key, val) in url.query_pairs() {
        if key == "page_size" && !next.is_empty() {
            next.insert(key.into_owned(), val.into_owned());
        }
    }
    Ok(next)
}
pub(super) async fn collect(
    client: &ApiClient,
    profile: &Profile,
    e: &'static Endpoint,
    args: &ArgMatches,
    mut url: Url,
) -> Result<Value, AppError> {
    let limit = *args.get_one::<usize>("limit").expect("collection default");
    let mut skip = *args
        .get_one::<usize>("page_skip")
        .expect("collection default");
    if limit == 0 {
        return Ok(finish(json!({}), e, vec![], Parameters::new(), true));
    }
    // Use a fixed page size throughout a run, including exact within-page resumes.
    if e.query.iter().any(|q| q.name == "page_size")
        && !url.query_pairs().any(|(k, _)| k == "page_size")
    {
        let max = e
            .query
            .iter()
            .find(|q| q.name == "page_size")
            .and_then(|q| q.maximum)
            .unwrap_or(100.0) as usize;
        set_query(
            &mut url,
            &Parameters::from([("page_size".into(), max.min(100).to_string())]),
        );
    }
    if e.paging == "page" && !url.query_pairs().any(|(k, _)| k == "page") {
        set_query(&mut url, &Parameters::from([("page".into(), "1".into())]));
    }
    if e.paging == "members" {
        set_query(
            &mut url,
            &Parameters::from([("limit".into(), "200".into())]),
        );
    }
    let mut records = Vec::new();
    let mut incomplete = false;
    let mut visited = HashSet::new();
    loop {
        if !visited.insert(url.to_string()) {
            return Err(invalid("provider repeated a pagination cursor"));
        }
        let body = client
            .request_no_redirect(
                SERVICE,
                e.operation_id.as_str(),
                profile,
                Method::GET,
                url.to_string(),
                None,
            )
            .await?;
        let rows = if e.key.is_empty() {
            body.as_array()
        } else {
            body.get(&e.key).and_then(Value::as_array)
        }
        .ok_or_else(|| invalid("provider returned an unexpected collection shape"))?;
        if skip > rows.len() {
            return Err(invalid("--page-skip exceeds the provider page length"));
        }
        let mut next = next_page(e, &body, &url, rows.len())?;
        incomplete |= (e.paging == "members" && rows.len() >= 200)
            || (e.paging == "capped" && rows.len() >= 100)
            || (e.paging == "chat"
                && rows.len() >= 500
                && !url.query_pairs().any(|(k, _)| k == "after_id"));
        let take = (limit - records.len()).min(rows.len() - skip);
        records.extend_from_slice(&rows[skip..skip + take]);
        let consumed = skip + take;
        if consumed < rows.len() {
            next = url
                .query_pairs()
                .filter(|(k, _)| {
                    matches!(
                        k.as_ref(),
                        "page"
                            | "page_size"
                            | "before_occurred_at"
                            | "before_id"
                            | "snapshot_id"
                            | "after_id"
                    )
                })
                .map(|(k, v)| (k.into_owned(), v.into_owned()))
                .collect();
            next.insert("page_skip".into(), consumed.to_string());
            return Ok(finish(body, e, records, next, incomplete));
        }
        if records.len() == limit || next.is_empty() {
            // Explicitly clear a previous within-page offset when advancing.
            if !next.is_empty() {
                next.insert("page_skip".into(), "0".into());
            }
            return Ok(finish(body, e, records, next, incomplete));
        }
        skip = 0;
        set_query(&mut url, &next);
    }
}
