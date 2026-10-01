mod cli;
mod config;
mod config_commands;
mod error;
mod http;
mod input;
mod oauth;
mod pagination;
mod secrets;
mod services;
mod skills;
#[cfg(test)]
mod test_support;

use std::process::ExitCode;

use clap::Parser;
use cli::Cli;
use error::AppError;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(value) => {
            match serde_json::to_string_pretty(&value) {
                Ok(rendered) => println!("{rendered}"),
                Err(err) => {
                    let app_err = AppError::internal("output", "serialize", err.to_string());
                    eprintln!("{}", app_err.to_json_line());
                    return app_err.exit_code();
                }
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{}", err.to_json_line());
            err.exit_code()
        }
    }
}

async fn run() -> Result<serde_json::Value, AppError> {
    let command_args = std::env::args().collect::<Vec<_>>();
    let cli = Cli::parse();
    match cli.command {
        // aai-cli's own tooling — these manage the CLI rather than doing work for an agent.
        cli::Command::Config(command) => config_commands::dispatch(cli.config.as_deref(), command),
        cli::Command::Skills(command) => skills::dispatch(command),
        // Capability services follow. Excel is one of them, but it reads local files, so it
        // is the only one needing neither a profile nor an HTTP client — hence the early
        // arm. It still goes out through the same response envelope as the rest.
        cli::Command::Excel(command) => Ok(pagination::annotate(
            services::excel::dispatch(command)?,
            &command_args,
        )),
        // URL parsing is deterministic local work. Keep it usable before an agent has
        // selected or configured the Jira profile it will use for the resulting target.
        cli::Command::Jira(cli::JiraCommand {
            resource:
                cli::JiraResource::Ideas(cli::JiraIdeasCommand {
                    action: cli::JiraIdeasAction::ParseUrl(args),
                }),
        }) => Ok(pagination::annotate(
            services::jira::parse_idea_url(&args.url)?,
            &command_args,
        )),
        command => {
            let ctx = config::Context::load(
                cli.config.as_deref(),
                cli.profile.as_deref(),
                cli.secrets_file.as_deref(),
                cli.key_file.as_deref(),
            )?;
            let value = services::dispatch(&ctx, command).await?;
            Ok(pagination::annotate(value, &command_args))
        }
    }
}
