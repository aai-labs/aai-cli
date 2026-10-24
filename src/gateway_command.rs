use std::{fs, path::Path};

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use reqwest::redirect::Policy;
use serde_json::Value;

use crate::{
    config::Profile,
    error::AppError,
    gateway_protocol::{
        CommandMetadata, CommandResult, CommandScope, GatewayErrorBody, COMMAND_EXECUTE_PATH,
        COMMAND_PROTOCOL_VERSION,
    },
};

const MAX_TRANSFER_BYTES: usize = 64 * 1024 * 1024;
const JSON_FLAGS: &[&str] = &[
    "--json",
    "--comments-json",
    "--values",
    "--formulas",
    "--number-format",
];

pub(crate) struct CommandPaths {
    pub input: Option<usize>,
    pub output: Option<usize>,
}

pub(crate) fn normalize_argv(mut argv: Vec<String>) -> Vec<String> {
    let mut index = 0;
    while index < argv.len() {
        if let Some((flag, value)) = argv[index].split_once('=') {
            if matches!(flag, "--file" | "--query-file" | "--output") || JSON_FLAGS.contains(&flag)
            {
                let flag = flag.to_string();
                let value = value.to_string();
                argv.splice(index..=index, [flag, value]);
            }
        }
        index += 1;
    }
    argv
}

pub(crate) fn command_paths(argv: &[String]) -> Result<CommandPaths, AppError> {
    let mut input = None;
    let mut output = None;
    for (index, arg) in argv.iter().enumerate() {
        if matches!(arg.as_str(), "--file" | "--query-file" | "--output") {
            let value_index = index + 1;
            if value_index >= argv.len() {
                return Err(AppError::invalid_input(
                    "gateway",
                    "execute",
                    "missing file path",
                ));
            }
            let target = if arg == "--output" {
                &mut output
            } else {
                &mut input
            };
            if target.replace(value_index).is_some() {
                return Err(AppError::invalid_input(
                    "gateway",
                    "execute",
                    "multiple input or output files are not supported",
                ));
            }
        }
    }
    for parts in [
        &["drive", "files", "upload"][..],
        &["microsoft", "files", "upload"][..],
        &["microsoft", "sharepoint", "files", "upload"][..],
    ] {
        if let Some(position) = argv
            .windows(parts.len())
            .position(|window| window.iter().map(String::as_str).eq(parts.iter().copied()))
        {
            let mut file_index = position + parts.len();
            while file_index < argv.len() && argv[file_index].starts_with("--") {
                file_index += if argv[file_index].contains('=') { 1 } else { 2 };
            }
            if file_index >= argv.len() || argv[file_index].starts_with('-') {
                return Err(AppError::invalid_input(
                    "gateway",
                    "execute",
                    "upload command is missing its local file",
                ));
            }
            if input.replace(file_index).is_some() {
                return Err(AppError::invalid_input(
                    "gateway",
                    "execute",
                    "multiple input files are not supported",
                ));
            }
        }
    }
    Ok(CommandPaths { input, output })
}

pub(crate) fn validate_inline_json(argv: &[String]) -> Result<(), AppError> {
    for (index, arg) in argv.iter().enumerate() {
        if JSON_FLAGS.contains(&arg.as_str()) {
            let value = argv.get(index + 1).ok_or_else(|| {
                AppError::invalid_input("gateway", "execute", "missing JSON argument")
            })?;
            serde_json::from_str::<Value>(value).map_err(|_| {
                AppError::invalid_input(
                    "gateway",
                    "execute",
                    "gateway JSON arguments must be supplied inline",
                )
            })?;
        }
    }
    Ok(())
}

fn inline_local_json(argv: &mut [String]) -> Result<(), AppError> {
    for index in 0..argv.len() {
        if JSON_FLAGS.contains(&argv[index].as_str()) {
            let value = argv.get(index + 1).ok_or_else(|| {
                AppError::invalid_input("gateway", "execute", "missing JSON argument")
            })?;
            let json = crate::input::read_json_arg("gateway", "execute", Some(value))?;
            argv[index + 1] = json.to_string();
        }
    }
    Ok(())
}

fn scope(profile: &Profile) -> CommandScope {
    CommandScope {
        owner: profile.owner.clone(),
        repo: profile.repo.clone(),
        org: profile.org.clone(),
        workspace: profile.workspace.clone(),
        project_id: profile.project_id.clone(),
        account_id: profile.account_id.clone(),
        user_id: profile.user_id.clone(),
        calendar_id: profile.calendar_id.clone(),
        from_address: profile.from_address.clone(),
    }
}

pub(crate) fn apply_scope(profile: &mut Profile, scope: CommandScope) {
    for (target, value) in [
        (&mut profile.owner, scope.owner),
        (&mut profile.repo, scope.repo),
        (&mut profile.org, scope.org),
        (&mut profile.workspace, scope.workspace),
        (&mut profile.project_id, scope.project_id),
        (&mut profile.account_id, scope.account_id),
        (&mut profile.user_id, scope.user_id),
        (&mut profile.calendar_id, scope.calendar_id),
        (&mut profile.from_address, scope.from_address),
    ] {
        if value.is_some() {
            *target = value;
        }
    }
}

pub(crate) async fn execute(profile: &Profile, argv: Vec<String>) -> Result<Value, AppError> {
    let gateway_url = profile
        .gateway_url
        .as_deref()
        .ok_or_else(|| AppError::config("gateway profile is missing gateway_url"))?;
    let profile_id = profile
        .gateway_profile_id
        .as_deref()
        .ok_or_else(|| AppError::config("gateway profile is missing gateway_profile_id"))?;
    let token = profile
        .gateway_token
        .as_deref()
        .ok_or_else(|| AppError::auth("gateway", "execute", "gateway token is missing"))?;
    let mut argv = normalize_argv(argv);
    inline_local_json(&mut argv)?;
    let paths = command_paths(&argv)?;
    let input = paths
        .input
        .map(|index| fs::read(&argv[index]))
        .transpose()
        .map_err(|err| AppError::invalid_input("gateway", "execute", err.to_string()))?;
    if input
        .as_ref()
        .is_some_and(|bytes| bytes.len() > MAX_TRANSFER_BYTES)
    {
        return Err(AppError::invalid_input(
            "gateway",
            "execute",
            "gateway input file exceeds 64 MiB",
        ));
    }
    let local_output = paths.output.map(|index| argv[index].clone());
    if local_output
        .as_deref()
        .is_some_and(|path| Path::new(path).exists())
    {
        return Err(AppError::invalid_input(
            "gateway",
            "execute",
            "output file already exists",
        ));
    }
    let metadata = CommandMetadata {
        protocol_version: COMMAND_PROTOCOL_VERSION,
        profile_id: profile_id.to_string(),
        argv,
        scope: scope(profile),
    };
    let client = reqwest::Client::builder()
        .redirect(Policy::none())
        .build()
        .map_err(|err| AppError::internal("gateway", "execute", err.to_string()))?;
    let mut form = reqwest::multipart::Form::new().text(
        "metadata",
        serde_json::to_string(&metadata)
            .map_err(|err| AppError::internal("gateway", "execute", err.to_string()))?,
    );
    if let Some(input) = input {
        form = form.part("input", reqwest::multipart::Part::bytes(input));
    }
    let response = client
        .post(format!(
            "{}{}",
            gateway_url.trim_end_matches('/'),
            COMMAND_EXECUTE_PATH
        ))
        .bearer_auth(token)
        .multipart(form)
        .send()
        .await
        .map_err(|err| {
            AppError::gateway(
                "gateway",
                "execute",
                reqwest::StatusCode::BAD_GATEWAY,
                err.to_string(),
                None,
            )
        })?;
    let status = response.status();
    if !status.is_success() {
        let error = response.json::<GatewayErrorBody>().await.ok();
        if let Some(error) = error {
            return Err(AppError::remote(
                &error.code,
                error.service,
                error.operation,
                error.status,
                error.message,
                error.details,
            ));
        }
        return Err(AppError::gateway(
            "gateway",
            "execute",
            status,
            "gateway command failed",
            None,
        ));
    }
    let mut result = response
        .json::<CommandResult>()
        .await
        .map_err(|err| AppError::internal("gateway", "execute", err.to_string()))?;
    if let Some(encoded) = result.output {
        let local_path = local_output.ok_or_else(|| {
            AppError::internal("gateway", "execute", "unexpected output file from gateway")
        })?;
        let bytes = B64
            .decode(encoded)
            .map_err(|_| AppError::internal("gateway", "execute", "invalid output encoding"))?;
        let path = Path::new(&local_path);
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        use std::io::Write;
        output.write_all(&bytes)?;
        if let Some(server_path) = result.server_output_path {
            let absolute = std::path::absolute(path)?;
            replace_path(
                &mut result.value,
                &server_path,
                absolute.to_string_lossy().as_ref(),
            );
        }
    }
    Ok(result.value)
}

fn replace_path(value: &mut Value, old: &str, new: &str) {
    match value {
        Value::String(text) if text == old => *text = new.to_string(),
        Value::Array(values) => {
            for value in values {
                replace_path(value, old, new);
            }
        }
        Value::Object(values) => {
            for value in values.values_mut() {
                replace_path(value, old, new);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positional_upload_file_is_found_after_value_flags() {
        for argv in [
            vec![
                "aai-cli",
                "drive",
                "files",
                "upload",
                "--name",
                "n",
                "local.bin",
            ],
            vec![
                "aai-cli",
                "drive",
                "files",
                "upload",
                "--name=n",
                "local.bin",
            ],
            vec![
                "aai-cli",
                "microsoft",
                "files",
                "upload",
                "--drive-id",
                "d",
                "local.bin",
                "remote.bin",
            ],
        ] {
            let argv = argv.into_iter().map(str::to_string).collect::<Vec<_>>();
            let paths = command_paths(&argv).unwrap();
            assert_eq!(argv[paths.input.unwrap()], "local.bin");
        }
    }
}
