//! `library auth` — sign in and manage the credentials saved on this
//! machine. Browser sign-in is for MCP; `--api-key` saves a credential
//! for REST, GraphQL, MCP, CI, and service accounts.

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
    /// Sign in to MCP through the browser, or save an API key with `--api-key`
    Login {
        /// Save a Library API key (`pk_…`) instead of signing in through
        /// the browser. For CI and service accounts.
        #[arg(long, value_name = "KEY")]
        api_key: Option<String>,
        /// Print the sign-in URL; open it in a browser on this machine
        #[arg(long)]
        no_browser: bool,
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
    /// Sign out and delete the local profile
    Logout,
}

pub async fn run(
    command: AuthCommand,
    overrides: &ConfigOverrides,
    format: Format,
) -> Result<()> {
    match command {
        AuthCommand::Login {
            api_key: Some(api_key),
            api_url,
            operator_id,
            no_verify,
            ..
        } => login(api_key, api_url, operator_id, no_verify, format).await,
        AuthCommand::Login {
            api_key: None,
            no_browser,
            api_url,
            operator_id,
            ..
        } => {
            browser_login(
                overrides,
                api_url,
                operator_id,
                no_browser,
                format,
            )
            .await
        }
        AuthCommand::Status => status(overrides, format),
        AuthCommand::Logout => logout(format).await,
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

    // Verify the requested credential before taking the profile lock. A
    // concurrent refresh may finish while the API is being checked.
    let mut candidate = config::load_stored().unwrap_or_default();
    candidate.api_key = Some(api_key.clone());
    candidate.oauth = None;
    if let Some(api_url) = api_url.as_ref() {
        candidate.api_base_url = Some(api_url.clone());
    }
    if let Some(operator_id) = operator_id.as_ref() {
        candidate.operator_id = Some(operator_id.clone());
    }
    let verified_base_url = candidate
        .api_base_url
        .clone()
        .unwrap_or_else(|| DEFAULT_API_BASE_URL.to_string());
    if !no_verify {
        verify(&candidate).await?;
    }

    // Serialize credential changes with refresh and logout, then apply the
    // requested login to the latest profile rather than a stale snapshot.
    let _profile_lock = config::acquire_refresh_lock().await?;
    let mut stored = config::load_stored().unwrap_or_default();
    stored.api_key = Some(api_key.clone());
    stored.oauth = None;
    if let Some(api_url) = api_url {
        stored.api_base_url = Some(api_url);
    }
    if let Some(operator_id) = operator_id {
        stored.operator_id = Some(operator_id);
    }

    let base_url = stored
        .api_base_url
        .clone()
        .unwrap_or_else(|| DEFAULT_API_BASE_URL.to_string());
    if !no_verify && base_url != verified_base_url {
        verify(&stored).await?;
    }

    let path = config::save_stored(&stored)?;

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

async fn browser_login(
    overrides: &ConfigOverrides,
    api_url: Option<String>,
    operator_id: Option<String>,
    no_browser: bool,
    format: Format,
) -> Result<()> {
    let mut candidate = config::load_stored().unwrap_or_default();
    if let Some(api_url) = api_url.as_ref() {
        candidate.api_base_url = Some(api_url.clone());
    }
    if let Some(operator_id) = operator_id.as_ref() {
        candidate.operator_id = Some(operator_id.clone());
    }
    // Sign in against the URL this login will save, unless a flag or the
    // environment points this one command elsewhere.
    let base_url = config::merge(
        &ConfigOverrides {
            api_base_url: overrides.api_base_url.clone(),
            ..Default::default()
        },
        &config::EnvConfig::from_process(),
        &candidate,
    )
    .api_base_url;

    let session =
        crate::oauth::browser_login(&base_url, !no_browser).await?;

    // Do not hold this lock while the user completes the browser flow. Once
    // it completes, reload and serialize the save with refresh and logout.
    let _profile_lock = config::acquire_refresh_lock().await?;
    let mut stored = config::load_stored().unwrap_or_default();
    if let Some(api_url) = api_url {
        stored.api_base_url = Some(api_url);
    }
    if let Some(operator_id) = operator_id {
        stored.operator_id = Some(operator_id);
    }
    let current_base_url = config::merge(
        &ConfigOverrides {
            api_base_url: overrides.api_base_url.clone(),
            ..Default::default()
        },
        &config::EnvConfig::from_process(),
        &stored,
    )
    .api_base_url;
    if current_base_url != base_url {
        bail!(
            "the saved API URL changed while browser sign-in was in progress; run library auth login again"
        );
    }

    stored.api_key = None;
    stored.oauth = Some(session.clone());
    let path = config::save_stored(&stored)?;

    match format {
        Format::Json => print_json(&json!({
            "saved_to": path.display().to_string(),
            "api_base_url": base_url,
            "signed_in_with": session.issuer,
            "expires_at": session.expires_at,
        })),
        Format::Text => {
            println!("Signed in to MCP. Saved to {}", path.display());
            println!("  API URL:   {base_url}");
            println!("  Signed in: {}", session.issuer);
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
    let browser_session =
        session_in_use(key_source, &stored, &resolved.api_base_url);

    match format {
        Format::Json => print_json(&json!({
            "config_path": path.display().to_string(),
            "config_exists": path.exists(),
            "api_base_url": resolved.api_base_url,
            "api_base_url_source": url_source,
            "authenticated": resolved.api_key.is_some()
                || browser_session.is_some(),
            "credential": credential_kind(
                key_source,
                browser_session.is_some(),
            ),
            "api_key": api_key_shown(key_source, &resolved),
            "api_key_source": key_source,
            "session_expires_at": browser_session
                .and_then(|session| session.expires_at),
            "operator_id": resolved.operator_id,
        })),
        Format::Text => {
            println!("Config file: {}", path.display());
            println!(
                "API URL:     {} ({url_source})",
                resolved.api_base_url
            );
            match (browser_session, resolved.api_key.as_deref()) {
                (Some(session), _) => println!(
                    "MCP signed in: {} (browser sign-in{})",
                    session.issuer,
                    if session.refresh_token.is_some() {
                        ", renews automatically"
                    } else {
                        ""
                    }
                ),
                (None, Some(key)) => println!(
                    "API key:     {} ({key_source})",
                    config::redact_key(key)
                ),
                (None, None) => {
                    println!("Signed in:   no — run `library auth login`")
                }
            }
            if let Some(operator_id) = resolved.operator_id {
                println!("Operator id: {operator_id}");
            }
        }
    }

    Ok(())
}

/// The saved browser sign-in, when it is the credential in use.
fn session_in_use<'a>(
    key_source: &str,
    stored: &'a StoredConfig,
    api_base_url: &str,
) -> Option<&'a crate::oauth::OAuthSession> {
    if key_source != "default" {
        return None;
    }
    let resource = format!("{}/mcp", api_base_url.trim_end_matches('/'));
    let now = crate::oauth::now_unix();
    stored.oauth.as_ref().filter(|session| {
        session.resource.trim_end_matches('/') == resource
            && (!session.needs_refresh(now)
                || session.refresh_token.is_some())
    })
}

fn credential_kind(
    key_source: &str,
    has_browser_session: bool,
) -> &'static str {
    match (key_source, has_browser_session) {
        ("default", true) => "browser",
        ("default", false) => "none",
        _ => "api_key",
    }
}

/// A token from browser sign-in is never shown, even redacted.
fn api_key_shown(
    key_source: &str,
    resolved: &config::ResolvedConfig,
) -> Option<String> {
    if key_source == "default" {
        return None;
    }
    resolved.api_key.as_deref().map(config::redact_key)
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

async fn logout(format: Format) -> Result<()> {
    let _refresh_lock = config::acquire_refresh_lock().await?;
    if let Some(session) = config::load_stored().ok().and_then(|s| s.oauth)
    {
        crate::oauth::revoke(&session).await;
    }
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
