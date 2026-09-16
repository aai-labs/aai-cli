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

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Profile {
    pub provider: Option<String>,
    pub transport: Option<String>,
    pub auth_type: Option<String>,
    pub base_url: Option<String>,
    pub site_url: Option<String>,
    pub email: Option<String>,
    pub username: Option<String>,
    pub token: Option<String>,
    pub token_env: Option<String>,
    pub token_secret: Option<String>,
    pub refresh_token: Option<String>,
    pub refresh_token_env: Option<String>,
    pub refresh_token_secret: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub client_secret_env: Option<String>,
    pub client_secret_secret: Option<String>,
    /// OAuth tenant for tenant-scoped authorities (Microsoft identity platform).
    pub tenant_id: Option<String>,
    /// OAuth scope override. Microsoft app-only profiles default to the Graph `.default` scope.
    pub scope: Option<String>,
    pub api_token: Option<String>,
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
            toml::from_str(&text)
                .map_err(|err| AppError::config(format!("invalid config TOML: {err}")))?
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

fn key_path(key_arg: Option<&str>, config_value: Option<&str>) -> Result<PathBuf, AppError> {
    if let Some(path) = key_arg.or(config_value) {
        return Ok(PathBuf::from(path));
    }
    secrets::default_key_path()
}

fn apply_env_overrides(profile: &mut Profile, profile_name: &str) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

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
        crate::secrets::dispatch(
            &seed_ctx,
            crate::cli::SecretsCommand {
                action: crate::cli::SecretsAction::Set(crate::cli::SecretSet {
                    key: "github.token".to_string(),
                    value: Some("resolved-token".to_string()),
                }),
            },
        )
        .unwrap();

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

    fn display_path(path: &Path) -> String {
        path.display().to_string().replace('\\', "\\\\")
    }
}
