//! Where the CLI gets its API base URL and credentials.
//!
//! Three sources, in order: an explicit flag, the environment, then the
//! saved profile written by `library auth login`. The flag wins so a
//! single command can reach a different environment without disturbing
//! the saved profile, and the environment wins over the file so CI and
//! agent runners never need to write to a home directory at all.

use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

pub const DEFAULT_API_BASE_URL: &str = "http://localhost:50055";

/// The saved profile on disk. Nothing here is required: a file holding
/// only a base URL is valid, and so is one holding only a key.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct StoredConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Tenant id sent as `x-operator-id`. Only the endpoints that do not
    /// name an organization in their path need it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_id: Option<String>,
}

/// What a command actually runs with, after all three sources are merged.
#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub api_base_url: String,
    pub api_key: Option<String>,
    pub operator_id: Option<String>,
}

/// Overrides supplied on the command line.
#[derive(Debug, Default, Clone)]
pub struct ConfigOverrides {
    pub api_base_url: Option<String>,
    pub api_key: Option<String>,
    pub operator_id: Option<String>,
}

pub fn config_path() -> Result<PathBuf> {
    if let Ok(explicit) = std::env::var("LIBRARY_CONFIG") {
        if !explicit.is_empty() {
            return Ok(PathBuf::from(explicit));
        }
    }

    let base = match std::env::var("XDG_CONFIG_HOME") {
        Ok(value) if !value.is_empty() => PathBuf::from(value),
        _ => {
            let home = std::env::var("HOME").map_err(|_| {
                anyhow!(
                    "cannot locate a config directory: neither \
                     LIBRARY_CONFIG, XDG_CONFIG_HOME, nor HOME is set"
                )
            })?;
            PathBuf::from(home).join(".config")
        }
    };

    Ok(base.join("library").join("config.json"))
}

pub fn load_stored() -> Result<StoredConfig> {
    let path = config_path()?;
    match fs::read_to_string(&path) {
        Ok(contents) => {
            serde_json::from_str(&contents).with_context(|| {
                format!("{} is not valid CLI config JSON", path.display())
            })
        }
        // A missing profile is the normal state before the first login.
        Err(error) if error.kind() == ErrorKind::NotFound => {
            Ok(StoredConfig::default())
        }
        Err(error) => Err(error)
            .with_context(|| format!("failed to read {}", path.display())),
    }
}

pub fn save_stored(config: &StoredConfig) -> Result<PathBuf> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("failed to create {}", parent.display())
        })?;
    }

    let mut contents = serde_json::to_string_pretty(config)?;
    contents.push('\n');
    fs::write(&path, contents)
        .with_context(|| format!("failed to write {}", path.display()))?;
    restrict_to_owner(&path)?;

    Ok(path)
}

pub fn delete_stored() -> Result<Option<PathBuf>> {
    let path = config_path()?;
    match fs::remove_file(&path) {
        Ok(()) => Ok(Some(path)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| {
            format!("failed to remove {}", path.display())
        }),
    }
}

/// The config file holds an API key, so it must not be world-readable.
#[cfg(unix)]
fn restrict_to_owner(path: &PathBuf) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .with_context(|| {
            format!("failed to restrict permissions on {}", path.display())
        })
}

#[cfg(not(unix))]
fn restrict_to_owner(_path: &PathBuf) -> Result<()> {
    Ok(())
}

/// The environment half of the three sources, read once so the merge
/// itself stays a pure function that tests can drive directly.
#[derive(Debug, Default, Clone)]
pub struct EnvConfig {
    pub api_base_url: Option<String>,
    pub api_key: Option<String>,
    pub operator_id: Option<String>,
}

impl EnvConfig {
    pub fn from_process() -> Self {
        Self {
            api_base_url: non_empty_env("LIBRARY_API_BASE_URL"),
            api_key: non_empty_env("LIBRARY_API_KEY"),
            operator_id: non_empty_env("LIBRARY_OPERATOR_ID"),
        }
    }
}

pub fn resolve(overrides: &ConfigOverrides) -> Result<ResolvedConfig> {
    let stored = load_stored()?;
    Ok(merge(overrides, &EnvConfig::from_process(), &stored))
}

pub fn merge(
    overrides: &ConfigOverrides,
    env: &EnvConfig,
    stored: &StoredConfig,
) -> ResolvedConfig {
    let api_base_url = overrides
        .api_base_url
        .clone()
        .or_else(|| env.api_base_url.clone())
        .or_else(|| stored.api_base_url.clone())
        .unwrap_or_else(|| DEFAULT_API_BASE_URL.to_string());

    let api_key = overrides
        .api_key
        .clone()
        .or_else(|| env.api_key.clone())
        .or_else(|| stored.api_key.clone());

    let operator_id = overrides
        .operator_id
        .clone()
        .or_else(|| env.operator_id.clone())
        .or_else(|| stored.operator_id.clone());

    ResolvedConfig {
        api_base_url: api_base_url.trim_end_matches('/').to_string(),
        api_key,
        operator_id,
    }
}

fn non_empty_env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

/// Show enough of a key to tell two of them apart, and no more.
pub fn redact_key(key: &str) -> String {
    // `pk_` plus a few characters identifies a key without disclosing
    // anything usable.
    let visible = key.chars().take(7).collect::<String>();
    format!("{visible}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_url(value: &str) -> Option<String> {
        Some(value.to_string())
    }

    #[test]
    fn a_trailing_slash_on_the_base_url_is_dropped() {
        let overrides = ConfigOverrides {
            api_base_url: base_url("https://api.example.com/"),
            ..Default::default()
        };

        let resolved = merge(
            &overrides,
            &EnvConfig::default(),
            &StoredConfig::default(),
        );

        assert_eq!(resolved.api_base_url, "https://api.example.com");
    }

    #[test]
    fn a_flag_beats_the_environment_and_the_saved_profile() {
        let overrides = ConfigOverrides {
            api_base_url: base_url("https://flag.example.com"),
            ..Default::default()
        };
        let env = EnvConfig {
            api_base_url: base_url("https://env.example.com"),
            ..Default::default()
        };
        let stored = StoredConfig {
            api_base_url: base_url("https://stored.example.com"),
            ..Default::default()
        };

        let resolved = merge(&overrides, &env, &stored);

        assert_eq!(resolved.api_base_url, "https://flag.example.com");
    }

    #[test]
    fn the_environment_beats_the_saved_profile() {
        let env = EnvConfig {
            api_key: Some("pk_env".to_string()),
            ..Default::default()
        };
        let stored = StoredConfig {
            api_key: Some("pk_stored".to_string()),
            ..Default::default()
        };

        let resolved = merge(&ConfigOverrides::default(), &env, &stored);

        assert_eq!(resolved.api_key.as_deref(), Some("pk_env"));
    }

    #[test]
    fn nothing_configured_falls_back_to_the_local_api() {
        let resolved = merge(
            &ConfigOverrides::default(),
            &EnvConfig::default(),
            &StoredConfig::default(),
        );

        assert_eq!(resolved.api_base_url, DEFAULT_API_BASE_URL);
        assert!(resolved.api_key.is_none());
    }

    #[test]
    fn redacting_a_key_keeps_only_its_prefix() {
        assert_eq!(redact_key("pk_live_abcdefghijklmnop"), "pk_live…");
    }
}
