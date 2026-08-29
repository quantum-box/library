//! `library` — a command line client for Library.
//!
//! The CLI exists so an agent (or a person) can read and change a Library
//! repository without a browser. It talks to the same REST API the web
//! and desktop clients use, authenticating with a `pk_…` API key, and it
//! can also call the MCP tools directly so the two surfaces can be
//! compared without standing up an MCP client.

mod client;
mod commands;
mod config;
mod output;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::config::ConfigOverrides;
use crate::output::Format;

#[derive(Parser)]
#[command(
    name = "library",
    version,
    about = "Read and change Library repositories from the command line",
    long_about = "Read and change Library repositories from the command \
                  line.\n\nCredentials come from --api-key, then \
                  LIBRARY_API_KEY, then the profile saved by `library \
                  auth login`. Pass --json for machine-readable output; \
                  the table form is for humans and is not stable."
)]
struct Cli {
    /// Base URL of the Library API. Overrides LIBRARY_API_BASE_URL and
    /// the saved profile.
    #[arg(long, global = true, value_name = "URL")]
    api_url: Option<String>,

    /// Library API key (`pk_…`). Overrides LIBRARY_API_KEY and the saved
    /// profile.
    #[arg(long, global = true, value_name = "KEY")]
    api_key: Option<String>,

    /// Tenant id sent as `x-operator-id`, for the few endpoints that do
    /// not name an organization in their path.
    #[arg(long, global = true, value_name = "ID")]
    operator_id: Option<String>,

    /// Print raw JSON instead of a table.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Save, inspect, and clear the API key this machine uses
    #[command(subcommand)]
    Auth(commands::auth::AuthCommand),
    /// Organizations
    #[command(subcommand)]
    Org(commands::org::OrgCommand),
    /// Repositories
    #[command(subcommand)]
    Repo(commands::repo::RepoCommand),
    /// Data records
    #[command(subcommand)]
    Data(commands::data::DataCommand),
    /// Property definitions
    #[command(subcommand)]
    Property(commands::property::PropertyCommand),
    /// Sources attached to a repository
    #[command(subcommand)]
    Source(commands::source::SourceCommand),
    /// Talk to the Library MCP server directly
    #[command(subcommand)]
    Mcp(commands::mcp::McpCommand),
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    let overrides = ConfigOverrides {
        api_base_url: cli.api_url.clone(),
        api_key: cli.api_key.clone(),
        operator_id: cli.operator_id.clone(),
    };
    let format = if cli.json { Format::Json } else { Format::Text };

    match cli.command {
        // `auth` writes the profile the other commands read, so it takes
        // the overrides rather than an already-built client.
        Command::Auth(command) => {
            commands::auth::run(command, &overrides, format).await
        }
        Command::Org(command) => {
            let client = build_client(&overrides)?;
            commands::org::run(command, &client, format).await
        }
        Command::Repo(command) => {
            let client = build_client(&overrides)?;
            commands::repo::run(command, &client, format).await
        }
        Command::Data(command) => {
            let client = build_client(&overrides)?;
            commands::data::run(command, &client, format).await
        }
        Command::Property(command) => {
            let client = build_client(&overrides)?;
            commands::property::run(command, &client, format).await
        }
        Command::Source(command) => {
            let client = build_client(&overrides)?;
            commands::source::run(command, &client, format).await
        }
        Command::Mcp(command) => {
            let client = build_client(&overrides)?;
            commands::mcp::run(command, &client, format).await
        }
    }
}

fn build_client(
    overrides: &ConfigOverrides,
) -> Result<client::LibraryClient> {
    client::LibraryClient::new(config::resolve(overrides)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn the_command_tree_is_well_formed() {
        Cli::command().debug_assert();
    }
}
