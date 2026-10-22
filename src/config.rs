use std::{collections::HashMap, env, fs, path::PathBuf};

use serde::Deserialize;

use crate::{error::AppError, secrets};

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    pub default_profile: Option<String>,
    pub secrets_file: Option<String>,
    pub key_file: Option<String>,
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
}

#[derive(Debug, Deserialize, serde::Serialize, Clone, Default)]
pub struct Profile {
    #[serde(default)]
    pub headers: Vec<CustomHeader>,
    pub provider: Option<String>,
    pub transport: Option<String>,
    pub credential_source: Option<String>,
    pub gateway_url: Option<String>,
    pub gateway_profile_id: Option<String>,
    pub gateway_token: Option<String>,
    pub gateway_token_env: Option<String>,
    pub gateway_token_secret: Option<String>,
    pub auth_type: Option<String>,
    pub base_url: Option<String>,
    pub site_url: Option<String>,
    pub email: Option<String>,
    pub username: Option<String>,
    pub username_env: Option<String>,
    pub username_secret: Option<String>,
    pub token: Option<String>,
    pub token_env: Option<String>,
    pub token_secret: Option<String>,
    pub refresh_token: Option<String>,
    pub refresh_token_env: Option<String>,
    pub refresh_token_secret: Option<String>,
    pub client_id: Option<String>,
    pub tenant_id: Option<String>,
    pub scope: Option<String>,
    pub token_url: Option<String>,
    pub client_secret: Option<String>,
    pub client_secret_env: Option<String>,
    pub client_secret_secret: Option<String>,
    pub api_token: Option<String>,
    /// Header that carries `api_token` for `auth_type = "api_key_header"`.
    pub api_key_header: Option<String>,
    pub api_token_env: Option<String>,
    pub api_token_secret: Option<String>,
    pub password: Option<String>,
    pub password_env: Option<String>,
    pub password_secret: Option<String>,
    pub workspace: Option<String>,
    pub owner: Option<String>,
    pub repo: Option<String>,
    pub org: Option<String>,
    pub user_id: Option<String>,
    pub account_id: Option<String>,
    pub calendar_id: Option<String>,
    pub project_id: Option<String>,
    pub from_address: Option<String>,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<u16>,
    pub imap_host: Option<String>,
    pub imap_port: Option<u16>,
    pub mail_folder: Option<String>,
    pub sent_folder: Option<String>,
    pub caldav_url: Option<String>,
    #[serde(skip)]
    pub runtime_secrets_file: Option<PathBuf>,
    #[serde(skip)]
    pub runtime_key_file: Option<PathBuf>,
}

#[derive(Debug, Deserialize, serde::Serialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct CustomHeader {
    pub name: String,
    pub value: Option<String>,
    pub value_env: Option<String>,
    pub value_secret: Option<String>,
    #[serde(default)]
    pub secret: bool,
}

pub struct Context {
    pub profile: Profile,
    pub secrets_file: PathBuf,
    pub key_file: PathBuf,
}

impl Context {
    pub fn load(
        config_arg: Option<&str>,
        profile_arg: Option<&str>,
        secrets_arg: Option<&str>,
        key_arg: Option<&str>,
    ) -> Result<Self, AppError> {
        let config_path = config_path(config_arg)?;
        let text = if config_path.exists() {
            fs::read_to_string(&config_path).map_err(|err| {
                AppError::config(format!("failed to read {}: {err}", config_path.display()))
            })?
        } else {
            String::new()
        };
        let config: Config = if text.trim().is_empty() {
            Config::default()
        } else {
            toml::from_str(&text).map_err(|err| toml_error(&text, &err))?
        };

        let secrets_file = secrets_path(secrets_arg, config.secrets_file.as_deref())?;
        let key_file = key_path(key_arg, config.key_file.as_deref())?;

        let profile_name = profile_arg
            .map(str::to_string)
            .or_else(|| env::var("AAI_PROFILE").ok())
            .or(config.default_profile)
            .unwrap_or_else(|| "default".to_string());

        let mut profile = config
            .profiles
            .get(&profile_name)
            .cloned()
            .unwrap_or_default();
        apply_env_overrides(&mut profile, &profile_name);
        let mut ctx = Self {
            profile,
            secrets_file,
            key_file,
        };
        apply_secret_overrides(&mut ctx)?;
        validate_gateway_profile(&ctx.profile)?;
        ctx.profile.runtime_secrets_file = Some(ctx.secrets_file.clone());
        ctx.profile.runtime_key_file = Some(ctx.key_file.clone());

        Ok(ctx)
    }
}

pub(crate) fn config_path(config_arg: Option<&str>) -> Result<PathBuf, AppError> {
    if let Some(path) = config_arg {
        return Ok(PathBuf::from(path));
    }
    if let Ok(path) = env::var("AAI_CONFIG") {
        return Ok(PathBuf::from(path));
    }
    let config_dir =
        dirs::config_dir().ok_or_else(|| AppError::config("could not locate config directory"))?;
    Ok(config_dir.join("aai-cli").join("config.toml"))
}

fn secrets_path(
    secrets_arg: Option<&str>,
    config_value: Option<&str>,
) -> Result<PathBuf, AppError> {
    if let Some(path) = secrets_arg.or(config_value) {
        return Ok(PathBuf::from(path));
    }
    secrets::default_secrets_path()
}

/// Reports only the location: the parser's message can echo config values, including secrets.
pub(crate) fn toml_error(text: &str, err: &toml::de::Error) -> AppError {
    match err.span() {
        Some(span) => AppError::config(format!(
            "invalid config TOML at line {}",
            text[..span.start].matches('\n').count() + 1
        )),
        None => AppError::config("invalid config TOML"),
    }
}

fn key_path(key_arg: Option<&str>, config_value: Option<&str>) -> Result<PathBuf, AppError> {
    if let Some(path) = key_arg.or(config_value) {
        return Ok(PathBuf::from(path));
    }
    secrets::default_key_path()
}

fn apply_env_overrides(profile: &mut Profile, profile_name: &str) {
    if profile.username.is_none() {
        profile.username = profile.username_env.as_deref().and_then(env_value);
    }
    if profile.token.is_none() {
        profile.token = profile.token_env.as_deref().and_then(env_value);
    }
    if profile.refresh_token.is_none() {
        profile.refresh_token = profile.refresh_token_env.as_deref().and_then(env_value);
    }
    if profile.client_secret.is_none() {
        profile.client_secret = profile.client_secret_env.as_deref().and_then(env_value);
    }
    if profile.api_token.is_none() {
        profile.api_token = profile.api_token_env.as_deref().and_then(env_value);
    }
    if profile.password.is_none() {
        profile.password = profile.password_env.as_deref().and_then(env_value);
    }

    if profile.gateway_token.is_none() {
        profile.gateway_token = profile.gateway_token_env.as_deref().and_then(env_value);
    }
    for header in &mut profile.headers {
        if header.value.is_none() {
            header.value = header.value_env.as_deref().and_then(env_value);
        }
    }
    if profile.provider.as_deref() == Some("custom_api") {
        return;
    }

    let normalized = profile_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>();

    if profile.token.is_none() {
        profile.token = env_value(&format!("AAI_{}_TOKEN", normalized))
            .or_else(|| env_value("AAI_TOKEN"))
            .or_else(|| env_value("POSTHOG_API_KEY"))
            .or_else(|| env_value("POSTHOG_PERSONAL_API_KEY"));
    }
    if profile.api_token.is_none() {
        profile.api_token = env_value(&format!("AAI_{}_API_TOKEN", normalized))
            .or_else(|| env_value("AAI_API_TOKEN"))
            .or_else(|| env_value("POSTHOG_API_KEY"))
            .or_else(|| env_value("POSTHOG_PERSONAL_API_KEY"));
    }
    if profile.password.is_none() {
        profile.password = env_value(&format!("AAI_{}_PASSWORD", normalized))
            .or_else(|| env_value("AAI_PASSWORD"));
    }
    if profile.gateway_token.is_none() {
        profile.gateway_token = env_value(&format!("AAI_{}_GATEWAY_TOKEN", normalized))
            .or_else(|| env_value("AAI_GATEWAY_TOKEN"));
    }
    if profile.project_id.is_none() {
        profile.project_id = env_value(&format!("AAI_{}_PROJECT_ID", normalized))
            .or_else(|| env_value("AAI_PROJECT_ID"))
            .or_else(|| env_value("POSTHOG_PROJECT_ID"));
    }
    if profile.base_url.is_none() {
        profile.base_url = env_value(&format!("AAI_{}_BASE_URL", normalized))
            .or_else(|| env_value("AAI_BASE_URL"))
            .or_else(|| env_value("POSTHOG_HOST"))
            .or_else(|| env_value("POSTHOG_BASE_URL"));
    }
}

fn apply_secret_overrides(ctx: &mut Context) -> Result<(), AppError> {
    if ctx.profile.username.is_none() {
        if let Some(key) = ctx.profile.username_secret.clone() {
            ctx.profile.username = secrets::get(ctx, &key)?;
        }
    }
    if ctx.profile.gateway_token.is_none() {
        if let Some(key) = ctx.profile.gateway_token_secret.clone() {
            ctx.profile.gateway_token = secrets::get(ctx, &key)?;
        }
    }
    for index in 0..ctx.profile.headers.len() {
        if ctx.profile.headers[index].value.is_none() {
            if let Some(key) = ctx.profile.headers[index].value_secret.clone() {
                ctx.profile.headers[index].value = secrets::get(ctx, &key)?;
            }
        }
    }
    if ctx.profile.token.is_none() {
        if let Some(key) = ctx.profile.token_secret.clone() {
            ctx.profile.token = secrets::get(ctx, &key)?;
        }
    }
    if ctx.profile.refresh_token.is_none() {
        if let Some(key) = ctx.profile.refresh_token_secret.clone() {
            ctx.profile.refresh_token = secrets::get(ctx, &key)?;
        }
    }
    if ctx.profile.client_secret.is_none() {
        if let Some(key) = ctx.profile.client_secret_secret.clone() {
            ctx.profile.client_secret = secrets::get(ctx, &key)?;
        }
    }
    if ctx.profile.api_token.is_none() {
        if let Some(key) = ctx.profile.api_token_secret.clone() {
            ctx.profile.api_token = secrets::get(ctx, &key)?;
        }
    }
    if ctx.profile.password.is_none() {
        if let Some(key) = ctx.profile.password_secret.clone() {
            ctx.profile.password = secrets::get(ctx, &key)?;
        }
    }
    Ok(())
}

fn env_value(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

fn validate_gateway_profile(profile: &Profile) -> Result<(), AppError> {
    if profile.credential_source.as_deref() != Some("gateway") {
        return Ok(());
    }
    let gateway_url = profile
        .gateway_url
        .as_deref()
        .ok_or_else(|| AppError::config("gateway profile is missing gateway_url"))?;
    if !gateway_url_is_allowed(gateway_url) {
        return Err(AppError::config(
            "gateway_url must use https (loopback http is allowed for development)",
        ));
    }
    for (name, value) in [
        ("gateway_profile_id", profile.gateway_profile_id.as_deref()),
        ("gateway_token", profile.gateway_token.as_deref()),
    ] {
        if value.is_none() {
            return Err(AppError::config(format!(
                "gateway profile is missing {name}"
            )));
        }
    }
    Ok(())
}

pub(crate) fn gateway_url_is_allowed(value: &str) -> bool {
    let Ok(parsed) = url::Url::parse(value) else {
        return false;
    };
    let loopback = matches!(
        parsed.host_str(),
        Some("localhost" | "127.0.0.1" | "::1" | "[::1]")
    );
    parsed.scheme() == "https" || (parsed.scheme() == "http" && loopback)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn config_uses_default_profile() {
        let config: Config = toml::from_str(
            r#"
default_profile = "work"

[profiles.work]
provider = "github"
token = "abc"
"#,
        )
        .unwrap();
        assert_eq!(config.default_profile.as_deref(), Some("work"));
        assert_eq!(config.profiles["work"].provider.as_deref(), Some("github"));
    }

    #[test]
    fn config_resolves_secret_references() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("config.toml");
        let secrets_path = temp.path().join("secrets.enc.json");
        let key_path = temp.path().join("key");

        let seed_ctx = Context {
            profile: Profile::default(),
            secrets_file: secrets_path.clone(),
            key_file: key_path.clone(),
        };
        crate::secrets::set(&seed_ctx, "github.token", "resolved-token").unwrap();

        fs::write(
            &config_path,
            format!(
                r#"
default_profile = "work"
secrets_file = "{}"
key_file = "{}"

[profiles.work]
provider = "github"
auth_type = "bearer_token"
token_secret = "github.token"
"#,
                display_path(&secrets_path),
                display_path(&key_path)
            ),
        )
        .unwrap();

        let ctx =
            Context::load(Some(display_path(&config_path).as_str()), None, None, None).unwrap();
        assert_eq!(ctx.profile.token.as_deref(), Some("resolved-token"));
    }

    #[test]
    fn direct_profiles_do_not_require_gateway_fields() {
        validate_gateway_profile(&Profile {
            token: Some("provider-token".into()),
            ..Profile::default()
        })
        .unwrap();
    }

    #[test]
    fn gateway_profiles_require_loopback_http_or_https() {
        let profile = Profile {
            credential_source: Some("gateway".into()),
            gateway_url: Some("http://gateway.example.test".into()),
            gateway_profile_id: Some("profile".into()),
            gateway_token: Some("proxy-token".into()),
            ..Profile::default()
        };
        assert!(validate_gateway_profile(&profile).is_err());
        let mut loopback = profile;
        loopback.gateway_url = Some("http://127.0.0.1:8787".into());
        validate_gateway_profile(&loopback).unwrap();
        loopback.gateway_url = Some("http://localhost:8787".into());
        validate_gateway_profile(&loopback).unwrap();
        loopback.gateway_url = Some("http://[::1]:8787".into());
        validate_gateway_profile(&loopback).unwrap();
    }

    fn seed_secrets(temp: &Path, entries: &[(&str, &str)]) -> (PathBuf, PathBuf) {
        let secrets_path = temp.join("secrets.enc.json");
        let key_path = temp.join("key");
        let seed_ctx = Context {
            profile: Profile::default(),
            secrets_file: secrets_path.clone(),
            key_file: key_path.clone(),
        };
        for (key, value) in entries {
            crate::secrets::set(&seed_ctx, key, value).unwrap();
        }
        (secrets_path, key_path)
    }

    fn load_profile(temp: &Path, profile: &str, entries: &[(&str, &str)]) -> Profile {
        let (secrets_path, key_path) = seed_secrets(temp, entries);
        let config_path = temp.join("config.toml");
        fs::write(
            &config_path,
            format!(
                "default_profile = \"p\"\nsecrets_file = \"{}\"\nkey_file = \"{}\"\n\n[profiles.p]\n{profile}",
                display_path(&secrets_path),
                display_path(&key_path)
            ),
        )
        .unwrap();
        Context::load(Some(display_path(&config_path).as_str()), None, None, None)
            .unwrap()
            .profile
    }

    #[test]
    fn custom_api_credentials_resolve_from_secret_references() {
        let temp = tempfile::tempdir().unwrap();
        let profile = load_profile(
            temp.path(),
            r#"provider = "custom_api"
username_secret = "api.user"
gateway_token_secret = "api.gateway"

[[profiles.p.headers]]
name = "X-Store"
value_secret = "api.header"
secret = true

[[profiles.p.headers]]
name = "X-Plain"
value = "literal"
"#,
            &[
                ("api.user", "stored-user"),
                ("api.gateway", "stored-gateway"),
                ("api.header", "stored-header"),
            ],
        );
        assert_eq!(profile.username.as_deref(), Some("stored-user"));
        assert_eq!(profile.gateway_token.as_deref(), Some("stored-gateway"));
        assert_eq!(profile.headers[0].value.as_deref(), Some("stored-header"));
        assert_eq!(profile.headers[1].value.as_deref(), Some("literal"));
    }

    #[test]
    fn custom_api_credentials_prefer_literal_then_env_then_secret() {
        env::set_var("AAI_CONFIG_TEST_CUSTOM_USER", "env-user");
        env::set_var("AAI_CONFIG_TEST_CUSTOM_GATEWAY", "env-gateway");
        env::set_var("AAI_CONFIG_TEST_CUSTOM_HEADER", "env-header");
        let temp = tempfile::tempdir().unwrap();
        let profile = load_profile(
            temp.path(),
            r#"provider = "custom_api"
username_env = "AAI_CONFIG_TEST_CUSTOM_USER"
username_secret = "api.user"
gateway_token_env = "AAI_CONFIG_TEST_CUSTOM_GATEWAY"
gateway_token_secret = "api.gateway"

[[profiles.p.headers]]
name = "X-Env"
value_env = "AAI_CONFIG_TEST_CUSTOM_HEADER"
value_secret = "api.header"

[[profiles.p.headers]]
name = "X-Literal"
value = "literal"
value_env = "AAI_CONFIG_TEST_CUSTOM_HEADER"
value_secret = "api.header"

[[profiles.p.headers]]
name = "X-Missing-Env"
value_env = "AAI_CONFIG_TEST_CUSTOM_UNSET"
value_secret = "api.header"
"#,
            &[
                ("api.user", "stored-user"),
                ("api.gateway", "stored-gateway"),
                ("api.header", "stored-header"),
            ],
        );
        assert_eq!(profile.username.as_deref(), Some("env-user"));
        assert_eq!(profile.gateway_token.as_deref(), Some("env-gateway"));
        let values = profile
            .headers
            .iter()
            .map(|header| header.value.as_deref())
            .collect::<Vec<_>>();
        assert_eq!(
            values,
            [Some("env-header"), Some("literal"), Some("stored-header")]
        );
    }

    #[test]
    fn custom_api_profiles_ignore_ambient_token_fallbacks() {
        env::set_var("AAI_CUSTOMAPIAMBIENT_TOKEN", "ambient-token");
        env::set_var("AAI_CUSTOMAPIAMBIENT_API_TOKEN", "ambient-api-token");
        env::set_var("AAI_CUSTOMAPIAMBIENT_PASSWORD", "ambient-password");
        env::set_var("AAI_CUSTOMAPIAMBIENT_BASE_URL", "https://ambient.example");

        let mut custom = Profile {
            provider: Some("custom_api".into()),
            ..Default::default()
        };
        apply_env_overrides(&mut custom, "customapiambient");
        assert_eq!(custom.token, None);
        assert_eq!(custom.api_token, None);
        assert_eq!(custom.password, None);
        assert_eq!(custom.base_url, None);

        let mut other = Profile {
            provider: Some("github".into()),
            ..Default::default()
        };
        apply_env_overrides(&mut other, "customapiambient");
        assert_eq!(other.token.as_deref(), Some("ambient-token"));
        assert_eq!(other.base_url.as_deref(), Some("https://ambient.example"));
    }

    #[test]
    fn absent_custom_api_secret_reference_leaves_the_credential_unset() {
        let temp = tempfile::tempdir().unwrap();
        let profile = load_profile(
            temp.path(),
            "provider = \"custom_api\"\ngateway_token_secret = \"absent\"\n",
            &[("other", "x")],
        );
        assert_eq!(profile.gateway_token, None);
    }

    #[test]
    fn toml_errors_report_the_line_without_echoing_config_values() {
        let text = "a = 1\ntoken = \"fixture-secret\n";
        let error = toml::from_str::<toml::Value>(text).unwrap_err();
        let error = toml_error(text, &error);
        assert_eq!(error.code, "config_error");
        assert_eq!(error.message, "invalid config TOML at line 2");
    }

    #[test]
    fn custom_header_config_rejects_unknown_fields() {
        let result = toml::from_str::<Config>(
            "[profiles.p]\nprovider = \"custom_api\"\n[[profiles.p.headers]]\nname = \"X\"\nvalu = \"typo\"\n",
        );
        assert!(result.is_err());
    }

    fn display_path(path: &Path) -> String {
        path.display().to_string().replace('\\', "\\\\")
    }
}
