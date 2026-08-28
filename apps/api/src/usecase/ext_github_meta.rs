//! Shared wire format for the `ext_github` property value.
//!
//! The `ext_github` property stores a JSON object describing where a
//! Data item lives in GitHub and whether continuous sync is enabled:
//!
//! ```json
//! {
//!   "repo": "owner/repo",
//!   "path": "docs/article.md",
//!   "ref": "main",
//!   "enabled": true,
//!   "sync_to_github": true
//! }
//! ```
//!
//! Missing `enabled` / `sync_to_github` flags are treated as `false`
//! (default-deny), matching the web client's sync policy.

use serde::{Deserialize, Serialize};

fn default_ref() -> String {
    "main".to_string()
}

/// Parsed `ext_github` property value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtGithubMeta {
    /// GitHub repository in `owner/repo` format.
    pub repo: String,
    /// File path within the repository.
    pub path: String,
    /// Branch to sync with (defaults to "main").
    #[serde(rename = "ref", default = "default_ref")]
    pub git_ref: String,
    /// Whether continuous sync is enabled for this Data item.
    #[serde(default)]
    pub enabled: bool,
    /// Whether the `ext_github` metadata itself is included in the
    /// frontmatter when syncing back to GitHub.
    #[serde(default)]
    pub sync_to_github: bool,
}

impl ExtGithubMeta {
    /// Parse an `ext_github` property value.
    ///
    /// Returns `None` for empty or malformed values, or values missing
    /// `repo` / `path`.
    pub fn parse(value: &str) -> Option<Self> {
        if value.trim().is_empty() {
            return None;
        }
        let meta: Self = serde_json::from_str(value).ok()?;
        if meta.repo.trim().is_empty() || meta.path.trim().is_empty() {
            return None;
        }
        Some(meta)
    }
}

#[cfg(test)]
mod tests {
    use super::ExtGithubMeta;

    #[test]
    fn parses_full_metadata() {
        let meta = ExtGithubMeta::parse(
            r#"{"repo":"owner/repo","path":"docs/a.md","ref":"develop","enabled":true,"sync_to_github":true}"#,
        )
        .unwrap();

        assert_eq!(meta.repo, "owner/repo");
        assert_eq!(meta.path, "docs/a.md");
        assert_eq!(meta.git_ref, "develop");
        assert!(meta.enabled);
        assert!(meta.sync_to_github);
    }

    #[test]
    fn missing_flags_are_default_deny_and_ref_defaults_to_main() {
        let meta = ExtGithubMeta::parse(
            r#"{"repo":"owner/repo","path":"docs/a.md"}"#,
        )
        .unwrap();

        assert_eq!(meta.git_ref, "main");
        assert!(!meta.enabled);
        assert!(!meta.sync_to_github);
    }

    #[test]
    fn rejects_empty_and_malformed_values() {
        assert!(ExtGithubMeta::parse("").is_none());
        assert!(ExtGithubMeta::parse("{}").is_none());
        assert!(ExtGithubMeta::parse("not json").is_none());
        assert!(
            ExtGithubMeta::parse(r#"{"repo":"","path":"a.md"}"#).is_none()
        );
    }
}
