use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{json, Map as JsonMap, Value as JsonValue};
use toml::{map::Map as TomlMap, Value as TomlValue};

use crate::{
    cli::{
        ConfigCommand, ConfigDefaultProfileAction, ConfigProfileName, ConfigProfileSet,
        ConfigProfilesAction, ConfigResource,
    },
    config::{self, Profile},
    error::AppError,
    input,
};

const DIRECT_SECRET_FIELDS: &[&str] = &[
    "token",
    "token_env",
    "api_token",
    "api_token_env",
    "password",
    "password_env",
];

const ALLOWED_PROFILE_FIELDS: &[&str] = &[
    "provider",
    "transport",
    "auth_type",
    "base_url",
    "site_url",
    "email",
    "username",
    "token_secret",
    "api_token_secret",
    "password_secret",
    "workspace",
    "owner",
    "repo",
    "org",
    "user_id",
    "account_id",
    "calendar_id",
    "tenant_id",
    "scope",
    "from_address",
    "smtp_host",
    "smtp_port",
    "imap_host",
    "imap_port",
    "mail_folder",
    "sent_folder",
    "caldav_url",
];

pub(crate) fn dispatch(
    config_arg: Option<&str>,
    command: ConfigCommand,
) -> Result<JsonValue, AppError> {
    let path = config::config_path(config_arg)?;
    match command.resource {
        ConfigResource::Profiles(command) => match command.action {
            ConfigProfilesAction::List => profiles_list(&path),
            ConfigProfilesAction::Get(args) => profile_get(&path, args),
            ConfigProfilesAction::Set(args) => profile_set(&path, args),
            ConfigProfilesAction::Remove(args) => profile_remove(&path, args),
            ConfigProfilesAction::Validate(args) => profile_validate(&path, args),
        },
        ConfigResource::DefaultProfile(command) => match command.action {
            ConfigDefaultProfileAction::Get => default_profile_get(&path),
            ConfigDefaultProfileAction::Set(args) => default_profile_set(&path, args),
        },
    }
}

fn profiles_list(path: &Path) -> Result<JsonValue, AppError> {
    let document = load_document(path)?;
    let profiles = profiles_table(&document);
    let values = profiles
        .map(|profiles| {
            profiles
                .iter()
                .map(|(name, value)| {
                    json!({
                        "name": name,
                        "profile": sanitize_profile(value),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Ok(json!({ "profiles": values }))
}

fn profile_get(path: &Path, args: ConfigProfileName) -> Result<JsonValue, AppError> {
    let document = load_document(path)?;
    let profile = find_profile(&document, &args.name, "profiles.get")?;
    Ok(json!({
        "name": args.name,
        "profile": sanitize_profile(profile),
    }))
}

fn profile_set(path: &Path, args: ConfigProfileSet) -> Result<JsonValue, AppError> {
    validate_profile_name(&args.name, "profiles.set")?;
    let name = args.name.clone();
    let patch = profile_patch(args)?;
    if patch.is_empty() {
        return Err(AppError::invalid_input(
            "config",
            "profiles.set",
            "provide at least one profile field or --json",
        ));
    }

    let mut document = load_document(path)?;
    let profile = ensure_profile_table(&mut document, &name, "profiles.set")?;
    for (key, value) in &patch {
        profile.insert(key.clone(), value.clone());
    }
    validate_profile_table(profile, "profiles.set")?;
    let changed = sanitize_table(&patch);
    let sanitized_profile = sanitize_table(profile);
    save_document(path, &document)?;

    Ok(json!({
        "name": name,
        "changed": changed,
        "profile": sanitized_profile,
    }))
}

fn profile_remove(path: &Path, args: ConfigProfileName) -> Result<JsonValue, AppError> {
    validate_profile_name(&args.name, "profiles.remove")?;
    let mut document = load_document(path)?;
    let root = document
        .as_table_mut()
        .expect("config document is always a table");
    let removed = root
        .get_mut("profiles")
        .and_then(TomlValue::as_table_mut)
        .and_then(|profiles| profiles.remove(&args.name))
        .is_some();
    let default_profile_cleared = removed
        && root
            .get("default_profile")
            .and_then(TomlValue::as_str)
            .is_some_and(|name| name == args.name);
    if default_profile_cleared {
        root.remove("default_profile");
    }
    if removed {
        save_document(path, &document)?;
    }
    Ok(json!({
        "name": args.name,
        "removed": removed,
        "default_profile_cleared": default_profile_cleared,
    }))
}

fn profile_validate(path: &Path, args: ConfigProfileName) -> Result<JsonValue, AppError> {
    let document = load_document(path)?;
    let profile = find_profile(&document, &args.name, "profiles.validate")?;
    let table = profile
        .as_table()
        .ok_or_else(|| AppError::config(format!("profile {} must be a TOML table", args.name)))?;
    validate_profile_table(table, "profiles.validate")?;
    Ok(json!({
        "name": args.name,
        "valid": true,
        "profile": sanitize_table(table),
    }))
}

fn default_profile_get(path: &Path) -> Result<JsonValue, AppError> {
    let document = load_document(path)?;
    let default_profile = document
        .as_table()
        .and_then(|root| root.get("default_profile"))
        .and_then(TomlValue::as_str);
    Ok(json!({ "default_profile": default_profile }))
}

fn default_profile_set(path: &Path, args: ConfigProfileName) -> Result<JsonValue, AppError> {
    validate_profile_name(&args.name, "default-profile.set")?;
    let mut document = load_document(path)?;
    find_profile(&document, &args.name, "default-profile.set")?;
    document
        .as_table_mut()
        .expect("config document is always a table")
        .insert(
            "default_profile".to_string(),
            TomlValue::String(args.name.clone()),
        );
    save_document(path, &document)?;
    Ok(json!({ "default_profile": args.name, "changed": true }))
}

fn profile_patch(args: ConfigProfileSet) -> Result<TomlMap<String, TomlValue>, AppError> {
    let mut patch = match input::read_json_arg("config", "profiles.set", args.json.as_deref())? {
        JsonValue::Object(values) => json_profile_patch(values)?,
        _ => {
            return Err(AppError::invalid_input(
                "config",
                "profiles.set",
                "profile JSON must be an object",
            ))
        }
    };
    set_string(&mut patch, "provider", args.provider);
    set_string(&mut patch, "auth_type", args.auth_type);
    set_string(&mut patch, "base_url", args.base_url);
    set_string(&mut patch, "api_token_secret", args.api_token_secret);
    set_string(&mut patch, "token_secret", args.token_secret);
    set_string(&mut patch, "password_secret", args.password_secret);
    Ok(patch)
}

fn json_profile_patch(
    values: JsonMap<String, JsonValue>,
) -> Result<TomlMap<String, TomlValue>, AppError> {
    let mut patch = TomlMap::new();
    for (key, value) in values {
        validate_profile_field(&key)?;
        let value = json_to_toml(value, &key)?;
        patch.insert(key, value);
    }
    Ok(patch)
}

fn json_to_toml(value: JsonValue, field: &str) -> Result<TomlValue, AppError> {
    match value {
        JsonValue::String(value) => Ok(TomlValue::String(value)),
        JsonValue::Number(value) => value
            .as_i64()
            .map(TomlValue::Integer)
            .ok_or_else(|| invalid_field(field, "must be a string, integer, or boolean")),
        JsonValue::Bool(value) => Ok(TomlValue::Boolean(value)),
        _ => Err(invalid_field(
            field,
            "must be a string, integer, or boolean",
        )),
    }
}

fn validate_profile_field(field: &str) -> Result<(), AppError> {
    if DIRECT_SECRET_FIELDS.contains(&field) {
        return Err(invalid_field(
            field,
            "direct and environment-backed credentials are forbidden; use a *_secret reference",
        ));
    }
    if !ALLOWED_PROFILE_FIELDS.contains(&field) {
        return Err(invalid_field(field, "is not a supported profile field"));
    }
    Ok(())
}

fn invalid_field(field: &str, message: &str) -> AppError {
    AppError::invalid_input("config", "profiles.set", format!("field {field} {message}"))
}

fn set_string(table: &mut TomlMap<String, TomlValue>, key: &str, value: Option<String>) {
    if let Some(value) = value {
        table.insert(key.to_string(), TomlValue::String(value));
    }
}

fn validate_profile_table(
    profile: &TomlMap<String, TomlValue>,
    operation: &'static str,
) -> Result<(), AppError> {
    for field in DIRECT_SECRET_FIELDS {
        if profile.contains_key(*field) {
            return Err(AppError::invalid_input(
                "config",
                operation,
                format!(
                    "profile contains forbidden credential field {field}; use a *_secret reference"
                ),
            ));
        }
    }

    let _: Profile = TomlValue::Table(profile.clone()).try_into().map_err(|_| {
        AppError::invalid_input(
            "config",
            operation,
            "profile contains an invalid value type",
        )
    })?;
    let provider = required_string(profile, "provider", operation)?;
    let auth_type = required_string(profile, "auth_type", operation)?;
    match provider {
        "pipedrive" => require_auth(
            profile,
            operation,
            auth_type,
            "pipedrive_personal_token",
            "api_token_secret",
        ),
        "apollo" => require_auth(
            profile,
            operation,
            auth_type,
            "apollo_api_key",
            "api_token_secret",
        ),
        "github" => require_auth(
            profile,
            operation,
            auth_type,
            "bearer_token",
            "token_secret",
        ),
        "hubspot" => require_one_of_auth(
            profile,
            operation,
            auth_type,
            &["hubspot_service_key", "hubspot_legacy_private_app"],
            "token_secret",
        ),
        "slack" => require_auth(
            profile,
            operation,
            auth_type,
            "bearer_token",
            "token_secret",
        ),
        "sharepoint" => require_auth(
            profile,
            operation,
            auth_type,
            "microsoft_client_credentials",
            "client_secret_secret",
        ),
        "jira" | "confluence" | "bitbucket" => require_auth(
            profile,
            operation,
            auth_type,
            "basic_api_token",
            "api_token_secret",
        ),
        _ => Ok(()),
    }
}

fn require_auth(
    profile: &TomlMap<String, TomlValue>,
    operation: &'static str,
    actual_auth: &str,
    expected_auth: &str,
    secret_field: &str,
) -> Result<(), AppError> {
    if actual_auth != expected_auth {
        return Err(AppError::invalid_input(
            "config",
            operation,
            format!("auth_type must be {expected_auth} for this provider"),
        ));
    }
    required_string(profile, secret_field, operation)?;
    Ok(())
}

fn require_one_of_auth(
    profile: &TomlMap<String, TomlValue>,
    operation: &'static str,
    actual_auth: &str,
    expected_auth: &[&str],
    secret_field: &str,
) -> Result<(), AppError> {
    if !expected_auth.contains(&actual_auth) {
        return Err(AppError::invalid_input(
            "config",
            operation,
            format!(
                "auth_type must be one of {} for this provider",
                expected_auth.join(", ")
            ),
        ));
    }
    required_string(profile, secret_field, operation)?;
    Ok(())
}

fn required_string<'a>(
    profile: &'a TomlMap<String, TomlValue>,
    field: &str,
    operation: &'static str,
) -> Result<&'a str, AppError> {
    profile
        .get(field)
        .and_then(TomlValue::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            AppError::invalid_input(
                "config",
                operation,
                format!("profile requires non-empty {field}"),
            )
        })
}

fn validate_profile_name(name: &str, operation: &'static str) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::invalid_input(
            "config",
            operation,
            "profile name must not be empty",
        ));
    }
    Ok(())
}

fn load_document(path: &Path) -> Result<TomlValue, AppError> {
    if !path.exists() {
        return Ok(TomlValue::Table(TomlMap::new()));
    }
    let text = fs::read_to_string(path)
        .map_err(|err| AppError::config(format!("failed to read {}: {err}", path.display())))?;
    if text.trim().is_empty() {
        Ok(TomlValue::Table(TomlMap::new()))
    } else {
        toml::from_str(&text).map_err(|_| AppError::config("invalid config TOML"))
    }
}

fn save_document(path: &Path, document: &TomlValue) -> Result<(), AppError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|err| {
        AppError::config(format!(
            "failed to create config directory {}: {err}",
            parent.display()
        ))
    })?;
    let rendered = toml::to_string_pretty(document)
        .map_err(|err| AppError::internal("config", "save", err.to_string()))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or_default();
    let temp_path = parent.join(format!(
        ".{}.{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("config.toml"),
        std::process::id(),
        nonce
    ));
    let write_result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|err| {
                AppError::config(format!(
                    "failed to create temporary config {}: {err}",
                    temp_path.display()
                ))
            })?;
        if let Ok(metadata) = fs::metadata(path) {
            fs::set_permissions(&temp_path, metadata.permissions()).map_err(|err| {
                AppError::config(format!(
                    "failed to preserve config permissions on {}: {err}",
                    temp_path.display()
                ))
            })?;
        }
        file.write_all(rendered.as_bytes()).map_err(|err| {
            AppError::config(format!(
                "failed to write temporary config {}: {err}",
                temp_path.display()
            ))
        })?;
        file.sync_all().map_err(|err| {
            AppError::config(format!(
                "failed to sync temporary config {}: {err}",
                temp_path.display()
            ))
        })?;
        fs::rename(&temp_path, path).map_err(|err| {
            AppError::config(format!(
                "failed to atomically replace {}: {err}",
                path.display()
            ))
        })
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    write_result
}

fn profiles_table(document: &TomlValue) -> Option<&TomlMap<String, TomlValue>> {
    document
        .as_table()
        .and_then(|root| root.get("profiles"))
        .and_then(TomlValue::as_table)
}

fn find_profile<'a>(
    document: &'a TomlValue,
    name: &str,
    operation: &'static str,
) -> Result<&'a TomlValue, AppError> {
    profiles_table(document)
        .and_then(|profiles| profiles.get(name))
        .ok_or_else(|| {
            AppError::not_found("config", operation, format!("profile not found: {name}"))
        })
}

fn ensure_profile_table<'a>(
    document: &'a mut TomlValue,
    name: &str,
    operation: &'static str,
) -> Result<&'a mut TomlMap<String, TomlValue>, AppError> {
    let root = document
        .as_table_mut()
        .ok_or_else(|| AppError::config("config root must be a TOML table"))?;
    let profiles = root
        .entry("profiles".to_string())
        .or_insert_with(|| TomlValue::Table(TomlMap::new()))
        .as_table_mut()
        .ok_or_else(|| AppError::config("profiles must be a TOML table"))?;
    profiles
        .entry(name.to_string())
        .or_insert_with(|| TomlValue::Table(TomlMap::new()))
        .as_table_mut()
        .ok_or_else(|| {
            AppError::invalid_input(
                "config",
                operation,
                format!("profile {name} must be a TOML table"),
            )
        })
}

fn sanitize_profile(value: &TomlValue) -> JsonValue {
    value
        .as_table()
        .map(sanitize_table)
        .unwrap_or_else(|| JsonValue::Object(JsonMap::new()))
}

fn sanitize_table(table: &TomlMap<String, TomlValue>) -> JsonValue {
    let mut result = JsonMap::new();
    for (key, value) in table {
        if DIRECT_SECRET_FIELDS.contains(&key.as_str()) {
            continue;
        }
        result.insert(key.clone(), toml_to_json(value));
    }
    JsonValue::Object(result)
}

fn toml_to_json(value: &TomlValue) -> JsonValue {
    match value {
        TomlValue::String(value) => json!(value),
        TomlValue::Integer(value) => json!(value),
        TomlValue::Float(value) => json!(value),
        TomlValue::Boolean(value) => json!(value),
        TomlValue::Datetime(value) => json!(value.to_string()),
        TomlValue::Array(values) => JsonValue::Array(values.iter().map(toml_to_json).collect()),
        TomlValue::Table(values) => sanitize_table(values),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{ConfigProfilesCommand, ConfigResource};

    fn profile_set_args(name: &str) -> ConfigProfileSet {
        ConfigProfileSet {
            name: name.to_string(),
            json: None,
            provider: None,
            auth_type: None,
            base_url: None,
            api_token_secret: None,
            token_secret: None,
            password_secret: None,
        }
    }

    fn profile_command(action: ConfigProfilesAction) -> ConfigCommand {
        ConfigCommand {
            resource: ConfigResource::Profiles(ConfigProfilesCommand { action }),
        }
    }

    #[test]
    fn set_patches_profile_and_preserves_unrelated_settings() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        fs::write(
            &path,
            r#"
custom_root = "keep"

[profiles.other]
provider = "custom"
auth_type = "custom_auth"
custom_field = "keep"

[profiles.work]
provider = "pipedrive"
auth_type = "pipedrive_personal_token"
api_token_secret = "old.secret"
owner = "keep"
"#,
        )
        .unwrap();
        let mut args = profile_set_args("work");
        args.api_token_secret = Some("new.secret".to_string());

        let result = dispatch(
            Some(path.to_str().unwrap()),
            profile_command(ConfigProfilesAction::Set(args)),
        )
        .unwrap();
        let saved = load_document(&path).unwrap();

        assert_eq!(result["changed"]["api_token_secret"], "new.secret");
        assert_eq!(result["profile"]["owner"], "keep");
        assert_eq!(saved["custom_root"].as_str(), Some("keep"));
        assert_eq!(
            saved["profiles"]["other"]["custom_field"].as_str(),
            Some("keep")
        );
        assert_eq!(
            saved["profiles"]["work"]["api_token_secret"].as_str(),
            Some("new.secret")
        );
        assert!(fs::read_dir(temp.path()).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp")));
    }

    #[test]
    fn get_never_prints_direct_or_environment_credentials() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        fs::write(
            &path,
            r#"
[profiles.legacy]
provider = "github"
auth_type = "bearer_token"
token = "direct"
token_env = "TOKEN_ENV"
token_secret = "github.token"
"#,
        )
        .unwrap();

        let result = dispatch(
            Some(path.to_str().unwrap()),
            profile_command(ConfigProfilesAction::Get(ConfigProfileName {
                name: "legacy".to_string(),
            })),
        )
        .unwrap();

        assert!(result["profile"].get("token").is_none());
        assert!(result["profile"].get("token_env").is_none());
        assert_eq!(result["profile"]["token_secret"], "github.token");
    }

    #[test]
    fn set_rejects_direct_and_environment_credentials_from_json() {
        for field in DIRECT_SECRET_FIELDS {
            let mut args = profile_set_args("work");
            args.json = Some(format!(r#"{{"{field}":"forbidden"}}"#));
            let error = profile_patch(args).unwrap_err();
            assert_eq!(error.code, "invalid_input");
            assert!(error.message.contains(field));
        }
    }

    #[test]
    fn validates_required_provider_auth_and_secret_combinations() {
        let cases = [
            ("pipedrive", "pipedrive_personal_token", "api_token_secret"),
            ("apollo", "apollo_api_key", "api_token_secret"),
            ("github", "bearer_token", "token_secret"),
            ("hubspot", "hubspot_service_key", "token_secret"),
            ("hubspot", "hubspot_legacy_private_app", "token_secret"),
            ("slack", "bearer_token", "token_secret"),
            (
                "sharepoint",
                "microsoft_client_credentials",
                "client_secret_secret",
            ),
            ("jira", "basic_api_token", "api_token_secret"),
            ("confluence", "basic_api_token", "api_token_secret"),
            ("bitbucket", "basic_api_token", "api_token_secret"),
        ];
        for (provider, auth_type, secret_field) in cases {
            let mut profile = TomlMap::new();
            profile.insert("provider".into(), TomlValue::String(provider.into()));
            profile.insert("auth_type".into(), TomlValue::String(auth_type.into()));
            assert!(validate_profile_table(&profile, "profiles.validate").is_err());
            profile.insert(secret_field.into(), TomlValue::String("reference".into()));
            validate_profile_table(&profile, "profiles.validate").unwrap();
        }
    }

    #[test]
    fn validation_rejects_invalid_known_field_types() {
        let mut profile = TomlMap::new();
        profile.insert("provider".into(), TomlValue::String("custom".into()));
        profile.insert("auth_type".into(), TomlValue::String("custom".into()));
        profile.insert("smtp_port".into(), TomlValue::String("465".into()));

        let error = validate_profile_table(&profile, "profiles.validate").unwrap_err();
        assert!(error.message.contains("invalid value type"));
    }

    #[test]
    fn default_profile_must_reference_existing_profile() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        let error = default_profile_set(
            &path,
            ConfigProfileName {
                name: "missing".to_string(),
            },
        )
        .unwrap_err();
        assert_eq!(error.code, "not_found");
        assert!(!path.exists());
    }

    #[test]
    fn removing_default_profile_clears_dangling_default() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        fs::write(
            &path,
            r#"
default_profile = "work"
custom_root = "keep"

[profiles.work]
provider = "github"
auth_type = "bearer_token"
token_secret = "github.token"
"#,
        )
        .unwrap();

        let result = profile_remove(
            &path,
            ConfigProfileName {
                name: "work".to_string(),
            },
        )
        .unwrap();
        let saved = load_document(&path).unwrap();

        assert_eq!(result["removed"], true);
        assert_eq!(result["default_profile_cleared"], true);
        assert!(saved.get("default_profile").is_none());
        assert_eq!(saved["custom_root"].as_str(), Some("keep"));
    }
}
