use std::{fs, path::Path};

use serde_json::{json, Value};

use crate::{
    config::{Context, Profile},
    error::AppError,
};

pub(crate) trait CtxProfile {
    fn profile(&self) -> &Profile;
}

impl CtxProfile for Context {
    fn profile(&self) -> &Profile {
        &self.profile
    }
}

pub(crate) fn site_url(
    profile: &Profile,
    service: &'static str,
    operation: &'static str,
) -> Result<String, AppError> {
    profile
        .site_url
        .as_deref()
        .or(profile.base_url.as_deref())
        .map(trim_url)
        .ok_or_else(|| {
            AppError::service_config(
                service,
                operation,
                format!("{service}.{operation} requires profile.site_url or profile.base_url"),
            )
        })
}

pub(crate) fn provider<'a>(
    profile: &'a Profile,
    service: &'static str,
    operation: &'static str,
) -> Result<&'a str, AppError> {
    profile.provider.as_deref().ok_or_else(|| {
        AppError::service_config(
            service,
            operation,
            format!("{service}.{operation} requires profile.provider"),
        )
    })
}

pub(crate) fn workspace<'a>(
    profile: &'a Profile,
    operation: &'static str,
) -> Result<&'a str, AppError> {
    profile.workspace.as_deref().ok_or_else(|| {
        AppError::service_config(
            "bitbucket",
            operation,
            format!("bitbucket.{operation} requires profile.workspace"),
        )
    })
}

pub(crate) fn account_id<'a>(
    profile: &'a Profile,
    service: &'static str,
    operation: &'static str,
) -> Result<&'a str, AppError> {
    profile.account_id.as_deref().ok_or_else(|| {
        AppError::service_config(
            service,
            operation,
            format!("{service}.{operation} requires profile.account_id"),
        )
    })
}

pub(crate) fn bitbucket_repo<'a>(
    profile: &'a Profile,
    repo_arg: Option<&'a str>,
    operation: &'static str,
) -> Result<(&'a str, &'a str), AppError> {
    let repo = repo_arg.or(profile.repo.as_deref()).ok_or_else(|| {
        AppError::service_config(
            "bitbucket",
            operation,
            format!("bitbucket.{operation} requires --repo or profile.repo"),
        )
    })?;
    if let Some((workspace, repo_name)) = repo.split_once('/') {
        if !workspace.is_empty() && !repo_name.is_empty() {
            return Ok((workspace, repo_name));
        }
    }
    let workspace = workspace(profile, operation)?;
    Ok((workspace, repo))
}

pub(crate) fn github_repo<'a>(
    profile: &'a Profile,
    owner_arg: Option<&'a str>,
    repo_arg: Option<&'a str>,
    operation: &'static str,
) -> Result<(&'a str, &'a str), AppError> {
    let owner = owner_arg.or(profile.owner.as_deref()).ok_or_else(|| {
        AppError::service_config(
            "github",
            operation,
            format!("github.{operation} requires --owner or profile.owner"),
        )
    })?;
    let repo = repo_arg.or(profile.repo.as_deref()).ok_or_else(|| {
        AppError::service_config(
            "github",
            operation,
            format!("github.{operation} requires --repo or profile.repo"),
        )
    })?;
    Ok((owner, repo))
}

pub(crate) fn calendar_id<'a>(profile: &'a Profile, arg: Option<&'a str>) -> &'a str {
    arg.or(profile.calendar_id.as_deref()).unwrap_or("primary")
}

pub(crate) fn github_base(profile: &Profile) -> String {
    profile
        .base_url
        .as_deref()
        .map(trim_url)
        .unwrap_or_else(|| "https://api.github.com".to_string())
}

pub(crate) fn bitbucket_base(profile: &Profile) -> String {
    profile
        .base_url
        .as_deref()
        .map(trim_url)
        .unwrap_or_else(|| "https://api.bitbucket.org/2.0".to_string())
}

pub(crate) fn google_base(profile: &Profile) -> String {
    profile
        .base_url
        .as_deref()
        .map(trim_url)
        .unwrap_or_else(|| "https://www.googleapis.com".to_string())
}

pub(crate) fn sheets_base() -> &'static str {
    "https://sheets.googleapis.com"
}

pub(crate) fn zoho_mail_base(profile: &Profile) -> String {
    profile
        .base_url
        .as_deref()
        .map(trim_url)
        .unwrap_or_else(|| "https://mail.zoho.com".to_string())
}

pub(crate) fn zoho_calendar_base(profile: &Profile) -> String {
    profile
        .base_url
        .as_deref()
        .map(trim_url)
        .unwrap_or_else(|| "https://calendar.zoho.com".to_string())
}

pub(crate) fn pipedrive_base(profile: &Profile) -> String {
    profile
        .base_url
        .as_deref()
        .map(trim_url)
        .unwrap_or_else(|| "https://api.pipedrive.com".to_string())
}

pub(crate) fn apollo_base(profile: &Profile) -> String {
    profile
        .base_url
        .as_deref()
        .map(trim_url)
        .unwrap_or_else(|| "https://api.apollo.io/api/v1".to_string())
}

pub(crate) fn hubspot_base(profile: &Profile) -> String {
    profile
        .base_url
        .as_deref()
        .map(trim_url)
        .unwrap_or_else(|| "https://api.hubapi.com".to_string())
}

pub(crate) fn slack_base(profile: &Profile) -> String {
    profile
        .base_url
        .as_deref()
        .map(trim_url)
        .unwrap_or_else(|| "https://slack.com/api".to_string())
}

pub(crate) fn openpanel_base(profile: &Profile) -> String {
    profile
        .base_url
        .as_deref()
        .map(trim_url)
        .unwrap_or_else(|| "https://api.openpanel.dev".to_string())
}

pub(crate) fn graph_base(profile: &Profile) -> String {
    profile
        .base_url
        .as_deref()
        .map(trim_url)
        .unwrap_or_else(|| "https://graph.microsoft.com/v1.0".to_string())
}

pub(crate) fn posthog_base(profile: &Profile) -> String {
    profile
        .base_url
        .as_deref()
        .map(trim_url)
        .unwrap_or_else(|| "https://us.i.posthog.com".to_string())
}

pub(crate) fn openpanel_project_id<'a>(
    profile: &'a Profile,
    arg: Option<&'a str>,
    operation: &'static str,
) -> Result<&'a str, AppError> {
    arg.or(profile.project_id.as_deref()).ok_or_else(|| {
        AppError::invalid_input(
            "openpanel",
            operation,
            format!("openpanel.{operation} requires --project-id or profile.project_id"),
        )
    })
}

pub(crate) fn posthog_project_id<'a>(
    profile: &'a Profile,
    arg: Option<&'a str>,
    operation: &'static str,
) -> Result<&'a str, AppError> {
    arg.or(profile.project_id.as_deref()).ok_or_else(|| {
        AppError::service_config(
            "posthog",
            operation,
            format!("posthog.{operation} requires --project-id or profile.project_id (or POSTHOG_PROJECT_ID)"),
        )
    })
}

pub(crate) fn trim_url(value: &str) -> String {
    value.trim_end_matches('/').to_string()
}

pub(crate) fn enc(value: &str) -> String {
    urlencoding::encode(value).into_owned()
}

pub(crate) fn pick(src: &Value, keys: &[&str]) -> Value {
    let mut out = serde_json::Map::new();
    if let Some(obj) = src.as_object() {
        for k in keys {
            if let Some(v) = obj.get(*k) {
                out.insert((*k).to_string(), v.clone());
            }
        }
    }
    Value::Object(out)
}

pub(crate) fn write_download(
    service: &'static str,
    operation: &'static str,
    output: &str,
    bytes: &[u8],
) -> Result<Value, AppError> {
    let path = Path::new(output);
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|err| {
            AppError::invalid_input(
                service,
                operation,
                format!(
                    "failed to create output directory {}: {err}",
                    parent.display()
                ),
            )
        })?;
    }
    fs::write(path, bytes).map_err(|err| {
        AppError::invalid_input(
            service,
            operation,
            format!("failed to write output file {}: {err}", path.display()),
        )
    })?;
    Ok(json!({
        "output": output,
        "bytes": bytes.len()
    }))
}
