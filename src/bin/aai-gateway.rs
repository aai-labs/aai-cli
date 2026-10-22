#![allow(dead_code, unused_imports)]

// The shared encrypted-secret module exposes CLI dispatch helpers in addition to the
// storage functions used by gateway profile resolution. Keep the gateway binary
// independent from the full aai-cli command tree by providing only those helper types.
mod cli {
    pub struct SecretsCommand {
        pub action: SecretsAction,
    }

    pub enum SecretsAction {
        Set(SecretSet),
        List,
        Remove(SecretRemove),
    }

    pub struct SecretSet {
        pub key: String,
        pub value: Option<String>,
    }

    pub struct SecretRemove {
        pub key: String,
    }
}

#[path = "../config.rs"]
mod config;
#[path = "../error.rs"]
mod error;
#[path = "../gateway_catalog.rs"]
mod gateway_catalog;
#[path = "../gateway_protocol.rs"]
mod gateway_protocol;
#[path = "../gateway_server.rs"]
mod gateway_server;
#[path = "../http.rs"]
mod http;
#[path = "../input.rs"]
mod input;
#[path = "../oauth.rs"]
mod oauth;
#[path = "../secrets.rs"]
mod secrets;
#[cfg(test)]
#[path = "../test_support.rs"]
mod test_support;

use std::{net::SocketAddr, path::PathBuf};

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "aai-gateway", about = "Managed credential gateway for aai-cli")]
struct GatewayCli {
    #[command(subcommand)]
    command: GatewayCommand,
}

#[derive(Debug, Subcommand)]
enum GatewayCommand {
    Serve(ServeArgs),
}

#[derive(Debug, clap::Args)]
struct ServeArgs {
    #[arg(long, default_value = "127.0.0.1:8787")]
    bind: SocketAddr,
    #[arg(
        long,
        env = "AAI_GATEWAY_STATE_FILE",
        default_value = "local/gateway-state.enc.json"
    )]
    state_file: PathBuf,
    #[arg(
        long,
        env = "AAI_GATEWAY_KEY_FILE",
        default_value = "local/gateway-state.key"
    )]
    key_file: PathBuf,
    #[arg(long, env = "AAI_GATEWAY_ADMIN_TOKEN", hide_env_values = true)]
    admin_token: String,
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let cli = GatewayCli::parse();
    let result = match cli.command {
        GatewayCommand::Serve(args) => {
            gateway_server::serve(args.bind, args.state_file, args.key_file, args.admin_token).await
        }
    };
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}", error.to_json_line());
            error.exit_code()
        }
    }
}
