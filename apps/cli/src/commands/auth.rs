//! `library auth` — manage the API key saved on this machine.

use anyhow::{bail, Result};
use clap::Subcommand;
use serde_json::json;

use crate::client::LibraryClient;
use crate::config::{
    self, ConfigOverrides, StoredConfig, DEFAULT_API_BASE_URL,
};
use crate::output::{print_json, Format};

#[derive(Subcommand)]
pub enum AuthCommand {
    /// Save an API key (and optionally an API URL) to the local profile
    Login {
        /// Library API key. Create one in the Library client under API
        /// keys; it starts with `pk_`.
        #[arg(long, value_name = "KEY")]
        api_key: String,
        /// Base URL to save alongside the key
        #[arg(long, value_name = "URL")]
        api_url: Option<String>,
        /// Tenant id to send as `x-operator-id`
        #[arg(long, value_name = "ID")]
        operator_id: Option<String>,
        /// Save without checking the key against the API first
        #[arg(long)]
        no_verify: bool,
    },
    /// Show which credentials the CLI would use and where they came from
    Status,
    /// Delete the local profile
    Logout,
}

pub async fn run(
    command: AuthCommand,
    overrides: &ConfigOverrides,
    format: Format,
) -> Result<()> {
    match command {
        AuthCommand::Login {
            api_key,
            api_url,
            operator_id,
            no_verify,
        } => login(api_key, api_url, operator_id, no_verify, format).await,
        AuthCommand::Status => status(overrides, format),
        AuthCommand::Logout => logout(format),
    }
}

async fn login(
    api_key: String,
    api_url: Option<String>,
    operator_id: Option<String>,
    no_verify: bool,
    format: Format,
) -> Result<()> {
    if api_key.trim().is_empty() {
        bail!("--api-key cannot be empty");
    }
    if !api_key.starts_with("pk_") {
        // Cognito access tokens work too, but they expire in an hour, so
        // saving one to disk is almost never what the caller wanted.
        eprintln!(
            "warning: Library API keys start with `pk_`; saving this \
             value anyway"
        );
    }

    // Anything already saved survives a login that does not mention it,
    // so re-running with a rotated key does not clear the API URL.
    let mut stored = config::load_stored().unwrap_or_default();
    stored.api_key = Some(api_key.clone());
    if let Some(api_url) = api_url {
        stored.api_base_url = Some(api_url);
    }
    if let Some(operator_id) = operator_id {
        stored.operator_id = Some(operator_id);
    }

    if !no_verify {
        verify(&stored).await?;
    }

    let path = config::save_stored(&stored)?;
    let base_url = stored
        .api_base_url
        .clone()
        .unwrap_or_else(|| DEFAULT_API_BASE_URL.to_string());

    match format {
        Format::Json => print_json(&json!({
            "saved_to": path.display().to_string(),
            "api_base_url": base_url,
            "api_key": config::redact_key(&api_key),
            "verified": !no_verify,
        })),
        Format::Text => {
            println!("Saved credentials to {}", path.display());
            println!("  API URL: {base_url}");
            println!("  API key: {}", config::redact_key(&api_key));
            if no_verify {
                println!("  (not verified)");
            }
        }
    }

    Ok(())
}

/// Confirm the API answers before writing a key to disk, so a typo is
/// caught now rather than on the next command.
async fn verify(stored: &StoredConfig) -> Result<()> {
    let resolved = config::merge(
        &ConfigOverrides {
            api_base_url: stored.api_base_url.clone(),
            api_key: stored.api_key.clone(),
            operator_id: stored.operator_id.clone(),
        },
        &config::EnvConfig::default(),
        &StoredConfig::default(),
    );
    let client = LibraryClient::new(resolved)?;

    // `/health` is unauthenticated, so this proves the URL is a Library
    // API and reachable. Whether the key itself grants anything depends
    // on the organization, which login does not know yet.
    //
    // It answers `text/plain`, not JSON, so this must not go through the
    // JSON-decoding `get` — doing so failed every login that did not
    // pass `--no-verify`.
    client.get_text("/health").await.map_err(|error| {
        anyhow::anyhow!(
            "{error}\nhint: pass --no-verify to save the key without \
             reaching the API"
        )
    })?;

    Ok(())
}

fn status(overrides: &ConfigOverrides, format: Format) -> Result<()> {
    let path = config::config_path()?;
    let stored = config::load_stored()?;
    let resolved = config::resolve(overrides)?;

    let key_source = source_of(
        overrides.api_key.is_some(),
        std::env::var("LIBRARY_API_KEY")
            .ok()
            .filter(|value| !value.is_empty())
            .is_some(),
        stored.api_key.is_some(),
    );
    let url_source = source_of(
        overrides.api_base_url.is_some(),
        std::env::var("LIBRARY_API_BASE_URL")
            .ok()
            .filter(|value| !value.is_empty())
            .is_some(),
        stored.api_base_url.is_some(),
    );

    match format {
        Format::Json => print_json(&json!({
            "config_path": path.display().to_string(),
            "config_exists": path.exists(),
            "api_base_url": resolved.api_base_url,
            "api_base_url_source": url_source,
            "authenticated": resolved.api_key.is_some(),
            "api_key": resolved
                .api_key
                .as_deref()
                .map(config::redact_key),
            "api_key_source": key_source,
            "operator_id": resolved.operator_id,
        })),
        Format::Text => {
            println!("Config file: {}", path.display());
            println!(
                "API URL:     {} ({url_source})",
                resolved.api_base_url
            );
            match resolved.api_key.as_deref() {
                Some(key) => println!(
                    "API key:     {} ({key_source})",
                    config::redact_key(key)
                ),
                None => println!(
                    "API key:     none — run `library auth login \
                     --api-key pk_…`"
                ),
            }
            if let Some(operator_id) = resolved.operator_id {
                println!("Operator id: {operator_id}");
            }
        }
    }

    Ok(())
}

fn source_of(flag: bool, env: bool, stored: bool) -> &'static str {
    if flag {
        "flag"
    } else if env {
        "environment"
    } else if stored {
        "config file"
    } else {
        "default"
    }
}

fn logout(format: Format) -> Result<()> {
    let removed = config::delete_stored()?;

    match (format, removed) {
        (Format::Json, removed) => print_json(&json!({
            "removed": removed.is_some(),
            "config_path": removed.map(|path| path.display().to_string()),
        })),
        (Format::Text, Some(path)) => {
            println!("Removed {}", path.display());
        }
        (Format::Text, None) => {
            println!("No saved credentials to remove");
        }
    }

    Ok(())
}
