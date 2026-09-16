use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use base64::{engine::general_purpose, Engine as _};
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    XChaCha20Poly1305,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    cli::{SecretSet, SecretsAction, SecretsCommand},
    config::Context,
    error::AppError,
};

const KEY_LEN: usize = 32;

#[derive(Debug, Serialize, Deserialize)]
struct SecretFile {
    version: u8,
    nonce: String,
    ciphertext: String,
}

pub(crate) fn default_key_path() -> Result<PathBuf, AppError> {
    if let Ok(path) = std::env::var("AAI_SECRET_KEY_FILE") {
        return Ok(PathBuf::from(path));
    }
    let run_dir = PathBuf::from("/run/aai");
    if run_dir.exists() || fs::create_dir_all(&run_dir).is_ok() {
        return Ok(run_dir.join("key"));
    }
    let config_dir =
        dirs::config_dir().ok_or_else(|| AppError::config("could not locate config directory"))?;
    Ok(config_dir.join("aai-cli").join("key"))
}

pub(crate) fn default_secrets_path() -> Result<PathBuf, AppError> {
    if let Ok(path) = std::env::var("AAI_SECRETS_FILE") {
        return Ok(PathBuf::from(path));
    }
    let config_dir =
        dirs::config_dir().ok_or_else(|| AppError::config("could not locate config directory"))?;
    Ok(config_dir.join("aai-cli").join("secrets.enc.json"))
}

pub(crate) fn get(ctx: &Context, key: &str) -> Result<Option<String>, AppError> {
    let store = SecretStore::open(&ctx.secrets_file, &ctx.key_file)?;
    store.get(key)
}

pub(crate) fn set(ctx: &Context, key: &str, value: &str) -> Result<(), AppError> {
    set_at(&ctx.secrets_file, &ctx.key_file, key, value)
}

pub(crate) fn set_at(
    secrets_file: &Path,
    key_file: &Path,
    key: &str,
    value: &str,
) -> Result<(), AppError> {
    SecretStore::open(secrets_file, key_file)?.set(key, value)
}

pub(crate) fn dispatch(ctx: &Context, command: SecretsCommand) -> Result<Value, AppError> {
    let store = SecretStore::open(&ctx.secrets_file, &ctx.key_file)?;
    match command.action {
        SecretsAction::Set(args) => {
            let value = secret_value(args)?;
            store.set(&value.0, &value.1)?;
            Ok(json!({
                "key": value.0,
                "set": true,
                "secrets_file": ctx.secrets_file,
                "key_file": ctx.key_file,
            }))
        }
        SecretsAction::List => {
            let mut keys = store.keys()?;
            keys.sort();
            Ok(json!({ "keys": keys }))
        }
        SecretsAction::Remove(args) => {
            let removed = store.remove(&args.key)?;
            Ok(json!({ "key": args.key, "removed": removed }))
        }
    }
}

fn secret_value(args: SecretSet) -> Result<(String, String), AppError> {
    if args.key.trim().is_empty() {
        return Err(AppError::invalid_input(
            "secrets",
            "set",
            "secret key must not be empty",
        ));
    }
    let value = match args.value {
        Some(value) => value,
        None => {
            let mut value = String::new();
            io::stdin()
                .read_to_string(&mut value)
                .map_err(|err| AppError::internal("secrets", "set", err.to_string()))?;
            strip_trailing_newline(value)
        }
    };
    if value.is_empty() {
        return Err(AppError::invalid_input(
            "secrets",
            "set",
            "secret value must not be empty",
        ));
    }
    Ok((args.key, value))
}

fn strip_trailing_newline(mut value: String) -> String {
    if value.ends_with('\n') {
        value.pop();
        if value.ends_with('\r') {
            value.pop();
        }
    }
    value
}

struct SecretStore<'a> {
    secrets_file: &'a Path,
    key: [u8; KEY_LEN],
}

impl<'a> SecretStore<'a> {
    fn open(secrets_file: &'a Path, key_file: &Path) -> Result<Self, AppError> {
        let key = load_or_create_key(key_file)?;
        Ok(Self { secrets_file, key })
    }

    fn get(&self, key: &str) -> Result<Option<String>, AppError> {
        Ok(self.load_plaintext()?.remove(key))
    }

    fn keys(&self) -> Result<Vec<String>, AppError> {
        Ok(self.load_plaintext()?.into_keys().collect())
    }

    fn set(&self, key: &str, value: &str) -> Result<(), AppError> {
        let mut values = self.load_plaintext()?;
        values.insert(key.to_string(), value.to_string());
        self.save_plaintext(&values)
    }

    fn remove(&self, key: &str) -> Result<bool, AppError> {
        let mut values = self.load_plaintext()?;
        let removed = values.remove(key).is_some();
        if removed {
            self.save_plaintext(&values)?;
        }
        Ok(removed)
    }

    fn load_plaintext(&self) -> Result<HashMap<String, String>, AppError> {
        if !self.secrets_file.exists() {
            return Ok(HashMap::new());
        }
        let text = fs::read_to_string(self.secrets_file).map_err(|err| {
            AppError::config(format!(
                "failed to read secrets file {}: {err}",
                self.secrets_file.display()
            ))
        })?;
        let file: SecretFile = serde_json::from_str(&text)
            .map_err(|err| AppError::config(format!("invalid secrets file: {err}")))?;
        if file.version != 1 {
            return Err(AppError::config(format!(
                "unsupported secrets file version {}",
                file.version
            )));
        }
        let nonce = general_purpose::STANDARD
            .decode(file.nonce)
            .map_err(|err| AppError::config(format!("invalid secrets nonce: {err}")))?;
        let ciphertext = general_purpose::STANDARD
            .decode(file.ciphertext)
            .map_err(|err| AppError::config(format!("invalid secrets ciphertext: {err}")))?;
        let cipher = XChaCha20Poly1305::new((&self.key).into());
        let plaintext = cipher
            .decrypt(nonce.as_slice().into(), ciphertext.as_ref())
            .map_err(|_| AppError::auth("secrets", "decrypt", "failed to decrypt secrets file"))?;
        serde_json::from_slice(&plaintext)
            .map_err(|err| AppError::config(format!("invalid decrypted secrets payload: {err}")))
    }

    fn save_plaintext(&self, values: &HashMap<String, String>) -> Result<(), AppError> {
        if let Some(parent) = self.secrets_file.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                AppError::config(format!(
                    "failed to create secrets directory {}: {err}",
                    parent.display()
                ))
            })?;
        }
        let plaintext = serde_json::to_vec(values)
            .map_err(|err| AppError::internal("secrets", "encrypt", err.to_string()))?;
        let cipher = XChaCha20Poly1305::new((&self.key).into());
        let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
        let ciphertext = cipher
            .encrypt(&nonce, plaintext.as_ref())
            .map_err(|_| AppError::internal("secrets", "encrypt", "encryption failed"))?;
        let file = SecretFile {
            version: 1,
            nonce: general_purpose::STANDARD.encode(nonce),
            ciphertext: general_purpose::STANDARD.encode(ciphertext),
        };
        let rendered = serde_json::to_string_pretty(&file)
            .map_err(|err| AppError::internal("secrets", "encrypt", err.to_string()))?;
        let parent = self.secrets_file.parent().unwrap_or_else(|| Path::new("."));
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_nanos())
            .unwrap_or_default();
        let temp_path = parent.join(format!(
            ".{}.{}.{}.tmp",
            self.secrets_file
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("secrets.enc.json"),
            std::process::id(),
            unique
        ));
        let write_result = (|| {
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp_path)
                .map_err(|err| AppError::internal("secrets", "encrypt", err.to_string()))?;
            output
                .write_all(rendered.as_bytes())
                .map_err(|err| AppError::internal("secrets", "encrypt", err.to_string()))?;
            output
                .sync_all()
                .map_err(|err| AppError::internal("secrets", "encrypt", err.to_string()))?;
            set_file_private(&temp_path)?;
            fs::rename(&temp_path, self.secrets_file)
                .map_err(|err| AppError::internal("secrets", "encrypt", err.to_string()))
        })();
        if write_result.is_err() {
            let _ = fs::remove_file(&temp_path);
        }
        write_result
    }
}

fn load_or_create_key(path: &Path) -> Result<[u8; KEY_LEN], AppError> {
    if path.exists() {
        let encoded = fs::read_to_string(path).map_err(|err| {
            AppError::config(format!("failed to read key file {}: {err}", path.display()))
        })?;
        let key = general_purpose::STANDARD
            .decode(encoded.trim())
            .map_err(|err| AppError::config(format!("invalid key file: {err}")))?;
        return key.try_into().map_err(|_| {
            AppError::config(format!(
                "invalid key length in {}; expected {KEY_LEN} bytes",
                path.display()
            ))
        });
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            AppError::config(format!(
                "failed to create key directory {}: {err}",
                parent.display()
            ))
        })?;
    }
    let key = XChaCha20Poly1305::generate_key(&mut OsRng);
    let encoded = general_purpose::STANDARD.encode(key);
    fs::write(path, format!("{encoded}\n")).map_err(|err| {
        AppError::config(format!(
            "failed to write key file {}: {err}",
            path.display()
        ))
    })?;
    set_file_private(path)?;
    Ok(key.into())
}

#[cfg_attr(not(unix), allow(unused_variables))]
fn set_file_private(path: &Path) -> Result<(), AppError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = fs::Permissions::from_mode(0o600);
        fs::set_permissions(path, permissions).map_err(|err| {
            AppError::config(format!(
                "failed to set private permissions on {}: {err}",
                path.display()
            ))
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_store_round_trips_values_without_plaintext_file() {
        let temp = tempfile::tempdir().unwrap();
        let secrets = temp.path().join("secrets.enc.json");
        let key = temp.path().join("key");
        let store = SecretStore::open(&secrets, &key).unwrap();

        store.set("github.token", "secret-value").unwrap();
        assert_eq!(
            store.get("github.token").unwrap().as_deref(),
            Some("secret-value")
        );
        assert!(!fs::read_to_string(&secrets)
            .unwrap()
            .contains("secret-value"));
    }
}
