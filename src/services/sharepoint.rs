use reqwest::Method;
use serde_json::Value;

use crate::{
    cli::*,
    config::Context,
    error::AppError,
    http::{ApiClient, BytesRequest},
    services::{
        generic_request,
        shared::{enc, graph_base, pick, write_download, CtxProfile},
    },
};

const SERVICE: &str = "sharepoint";

pub(crate) async fn dispatch(
    client: &ApiClient,
    ctx: &Context,
    command: SharepointCommand,
) -> Result<Value, AppError> {
    match command.resource {
        SharepointResource::Sites(command) => sites(client, ctx, command).await,
        SharepointResource::Drives(command) => drives(client, ctx, command).await,
        SharepointResource::Items(command) => items(client, ctx, command).await,
        SharepointResource::Request(args) => {
            generic_request::dispatch(client, ctx, SERVICE, graph_base(ctx.profile()), args).await
        }
    }
}

async fn sites(
    client: &ApiClient,
    ctx: &Context,
    command: SharepointSitesCommand,
) -> Result<Value, AppError> {
    let base = graph_base(ctx.profile());
    match command.action {
        SharepointSitesAction::List(args) => {
            let operation = "sites.list";
            let url = sites_search_url(&base, args.search.as_deref(), args.limit);
            let body = client
                .request(SERVICE, operation, ctx.profile(), Method::GET, url, None)
                .await?;
            Ok(trim_collection(body, trim_site))
        }
        SharepointSitesAction::Get(args) => {
            let operation = "sites.get";
            let url = format!("{base}/sites/{}", site_segment(&args.site, operation)?);
            client
                .request(SERVICE, operation, ctx.profile(), Method::GET, url, None)
                .await
        }
    }
}

async fn drives(
    client: &ApiClient,
    ctx: &Context,
    command: SharepointDrivesCommand,
) -> Result<Value, AppError> {
    let base = graph_base(ctx.profile());
    match command.action {
        SharepointDrivesAction::List(args) => {
            let operation = "drives.list";
            let url = format!(
                "{base}/sites/{}/drives",
                site_segment(&args.site, operation)?
            );
            let body = client
                .request(SERVICE, operation, ctx.profile(), Method::GET, url, None)
                .await?;
            Ok(trim_collection(body, trim_drive))
        }
    }
}

async fn items(
    client: &ApiClient,
    ctx: &Context,
    command: SharepointItemsCommand,
) -> Result<Value, AppError> {
    let base = graph_base(ctx.profile());
    match command.action {
        SharepointItemsAction::List(args) => {
            let operation = "items.list";
            let url = children_url(&base, &args.drive_id, args.path.as_deref(), args.limit);
            let body = client
                .request(SERVICE, operation, ctx.profile(), Method::GET, url, None)
                .await?;
            Ok(trim_collection(body, trim_item))
        }
        SharepointItemsAction::Get(args) => {
            let operation = "items.get";
            let url = format!(
                "{base}/drives/{}/items/{}",
                enc(&args.drive_id),
                enc(&args.item_id)
            );
            client
                .request(SERVICE, operation, ctx.profile(), Method::GET, url, None)
                .await
        }
        SharepointItemsAction::Download(args) => {
            let operation = "items.download";
            let url = format!(
                "{base}/drives/{}/items/{}/content",
                enc(&args.drive_id),
                enc(&args.item_id)
            );
            let bytes = client
                .download(SERVICE, operation, ctx.profile(), url)
                .await?;
            write_download(SERVICE, operation, &args.output, &bytes)
        }
        SharepointItemsAction::Upload(args) => {
            let operation = "items.upload";
            if !args.allow_write {
                return Err(AppError::invalid_input(
                    SERVICE,
                    operation,
                    "uploading replaces remote file content; pass --allow-write to confirm",
                ));
            }
            let body = std::fs::read(&args.file).map_err(|err| {
                AppError::invalid_input(
                    SERVICE,
                    operation,
                    format!("failed to read {}: {err}", args.file),
                )
            })?;
            let request = BytesRequest {
                method: Method::PUT,
                url: upload_url(&base, &args.drive_id, &args.path),
                content_type: "application/octet-stream".to_string(),
                headers: Vec::new(),
                body,
            };
            let response = client
                .request_bytes(SERVICE, operation, ctx.profile(), request)
                .await?;
            Ok(response.body)
        }
        SharepointItemsAction::Delta(args) => {
            let operation = "items.delta";
            let url = delta_url(&base, &args.drive_id, args.token.as_deref());
            let body = client
                .request(SERVICE, operation, ctx.profile(), Method::GET, url, None)
                .await?;
            Ok(trim_collection(body, trim_item))
        }
    }
}

/// Address a site for Graph.
///
/// Graph accepts both an opaque site id (`host,siteGuid,webGuid`) and the
/// `{host}:/{server-relative-path}` form on `/sites/{...}`, so only a SharePoint URL
/// needs converting. Anything that is not a URL is passed through untouched — an id
/// the caller already holds is not ours to reinterpret.
fn site_segment(site: &str, operation: &'static str) -> Result<String, AppError> {
    let site = site.trim();
    if site.is_empty() {
        return Err(AppError::invalid_input(
            SERVICE,
            operation,
            "site must not be empty",
        ));
    }
    let Some(rest) = site
        .strip_prefix("https://")
        .or_else(|| site.strip_prefix("http://"))
    else {
        return Ok(site.to_string());
    };
    let (host, path) = match rest.split_once('/') {
        Some((host, path)) => (host, path.trim_end_matches('/')),
        None => (rest, ""),
    };
    if host.is_empty() {
        return Err(AppError::invalid_input(
            SERVICE,
            operation,
            format!("not a SharePoint site URL: {site}"),
        ));
    }
    if path.is_empty() {
        Ok(host.to_string())
    } else {
        Ok(format!("{host}:/{path}"))
    }
}

/// Percent-encode each path segment while leaving the separators intact, so a
/// drive-relative path survives Graph's `root:/{path}:` addressing.
fn encode_path(path: &str) -> String {
    path.trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(enc)
        .collect::<Vec<_>>()
        .join("/")
}

fn sites_search_url(base: &str, search: Option<&str>, limit: u32) -> String {
    // Graph requires a search term on /sites; "*" is its documented match-all.
    let search = search
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("*");
    format!("{base}/sites?search={}&$top={limit}", enc(search))
}

fn children_url(base: &str, drive_id: &str, path: Option<&str>, limit: u32) -> String {
    let root = match path
        .map(str::trim)
        .filter(|p| !p.trim_matches('/').is_empty())
    {
        Some(path) => format!(
            "{base}/drives/{}/root:/{}:/children",
            enc(drive_id),
            encode_path(path)
        ),
        None => format!("{base}/drives/{}/root/children", enc(drive_id)),
    };
    format!("{root}?$top={limit}")
}

fn upload_url(base: &str, drive_id: &str, path: &str) -> String {
    format!(
        "{base}/drives/{}/root:/{}:/content",
        enc(drive_id),
        encode_path(path)
    )
}

fn delta_url(base: &str, drive_id: &str, token: Option<&str>) -> String {
    match token.map(str::trim).filter(|t| !t.is_empty()) {
        Some(token) => format!(
            "{base}/drives/{}/root/delta?token={}",
            enc(drive_id),
            enc(token)
        ),
        None => format!("{base}/drives/{}/root/delta", enc(drive_id)),
    }
}

/// Trim the page-local collection while preserving the envelope, so `@odata.nextLink`
/// and `@odata.deltaLink` survive for the pagination annotator.
fn trim_collection(mut body: Value, trim: fn(&Value) -> Value) -> Value {
    if let Some(items) = body.get("value").and_then(Value::as_array) {
        let trimmed: Vec<Value> = items.iter().map(trim).collect();
        body["value"] = Value::Array(trimmed);
    }
    body
}

fn trim_site(value: &Value) -> Value {
    pick(
        value,
        &[
            "id",
            "name",
            "displayName",
            "webUrl",
            "description",
            "createdDateTime",
            "lastModifiedDateTime",
        ],
    )
}

fn trim_drive(value: &Value) -> Value {
    pick(
        value,
        &[
            "id",
            "name",
            "driveType",
            "webUrl",
            "description",
            "lastModifiedDateTime",
        ],
    )
}

fn trim_item(value: &Value) -> Value {
    pick(
        value,
        &[
            "id",
            "name",
            "size",
            "webUrl",
            "eTag",
            "cTag",
            "createdDateTime",
            "lastModifiedDateTime",
            "file",
            "folder",
            "parentReference",
            "deleted",
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const BASE: &str = "https://graph.microsoft.com/v1.0";

    #[test]
    fn site_url_becomes_host_and_path() {
        assert_eq!(
            site_segment("https://contoso.sharepoint.com/sites/eng", "sites.get").unwrap(),
            "contoso.sharepoint.com:/sites/eng"
        );
    }

    #[test]
    fn site_url_trailing_slash_is_ignored() {
        assert_eq!(
            site_segment("https://contoso.sharepoint.com/sites/eng/", "sites.get").unwrap(),
            "contoso.sharepoint.com:/sites/eng"
        );
    }

    #[test]
    fn root_site_url_becomes_bare_host() {
        assert_eq!(
            site_segment("https://contoso.sharepoint.com", "sites.get").unwrap(),
            "contoso.sharepoint.com"
        );
    }

    #[test]
    fn composite_site_id_passes_through_untouched() {
        let id = "contoso.sharepoint.com,8c8e0f3b-1111,9d9f1a4c-2222";
        assert_eq!(site_segment(id, "sites.get").unwrap(), id);
    }

    #[test]
    fn empty_site_is_rejected() {
        assert!(site_segment("   ", "sites.get").is_err());
    }

    #[test]
    fn encodes_each_path_segment_but_keeps_separators() {
        assert_eq!(
            encode_path("/Shared Documents/Q3 Report.xlsx"),
            "Shared%20Documents/Q3%20Report.xlsx"
        );
    }

    #[test]
    fn children_url_uses_root_for_no_path() {
        assert_eq!(
            children_url(BASE, "drive1", None, 50),
            "https://graph.microsoft.com/v1.0/drives/drive1/root/children?$top=50"
        );
    }

    #[test]
    fn children_url_addresses_a_folder_path() {
        assert_eq!(
            children_url(BASE, "drive1", Some("Shared Documents"), 10),
            "https://graph.microsoft.com/v1.0/drives/drive1/root:/Shared%20Documents:/children?$top=10"
        );
    }

    #[test]
    fn children_url_treats_a_bare_slash_as_the_root() {
        assert_eq!(
            children_url(BASE, "drive1", Some("/"), 50),
            "https://graph.microsoft.com/v1.0/drives/drive1/root/children?$top=50"
        );
    }

    #[test]
    fn upload_url_targets_item_content() {
        assert_eq!(
            upload_url(BASE, "drive1", "Reports/q3.xlsx"),
            "https://graph.microsoft.com/v1.0/drives/drive1/root:/Reports/q3.xlsx:/content"
        );
    }

    #[test]
    fn delta_url_omits_an_absent_token() {
        assert_eq!(
            delta_url(BASE, "drive1", None),
            "https://graph.microsoft.com/v1.0/drives/drive1/root/delta"
        );
    }

    #[test]
    fn delta_url_carries_and_encodes_a_token() {
        assert_eq!(
            delta_url(BASE, "drive1", Some("aTok en+1")),
            "https://graph.microsoft.com/v1.0/drives/drive1/root/delta?token=aTok%20en%2B1"
        );
    }

    #[test]
    fn sites_search_defaults_to_match_all() {
        assert_eq!(
            sites_search_url(BASE, None, 25),
            // "*" is percent-encoded like any other search term; Graph decodes it back
            "https://graph.microsoft.com/v1.0/sites?search=%2A&$top=25"
        );
    }

    #[test]
    fn trim_collection_preserves_the_odata_envelope() {
        let body = json!({
            "@odata.context": "https://graph.microsoft.com/v1.0/$metadata#sites",
            "@odata.nextLink": "https://graph.microsoft.com/v1.0/sites?$skiptoken=X",
            "value": [{
                "id": "s1",
                "displayName": "Engineering",
                "webUrl": "https://contoso.sharepoint.com/sites/eng",
                "siteCollection": {"hostname": "contoso.sharepoint.com"}
            }]
        });
        let out = trim_collection(body, trim_site);

        assert_eq!(
            out["@odata.nextLink"],
            "https://graph.microsoft.com/v1.0/sites?$skiptoken=X"
        );
        assert_eq!(out["value"][0]["displayName"], "Engineering");
        // noise outside the allowlist is dropped
        assert!(out["value"][0].get("siteCollection").is_none());
    }

    #[test]
    fn trim_item_keeps_the_fields_an_agent_needs() {
        let item = json!({
            "id": "i1",
            "name": "report.xlsx",
            "size": 1024,
            "eTag": "\"{GUID},1\"",
            "file": {"mimeType": "application/vnd.ms-excel"},
            "@microsoft.graph.downloadUrl": "https://very-long-signed-url",
            "fileSystemInfo": {"createdDateTime": "2026-01-01T00:00:00Z"}
        });
        let out = trim_item(&item);

        assert_eq!(out["name"], "report.xlsx");
        assert_eq!(out["size"], 1024);
        assert_eq!(out["file"]["mimeType"], "application/vnd.ms-excel");
        // the signed download url is large and short-lived; downloads go through
        // items.download instead
        assert!(out.get("@microsoft.graph.downloadUrl").is_none());
        assert!(out.get("fileSystemInfo").is_none());
    }
}
