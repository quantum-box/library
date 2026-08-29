pub mod auth;
pub mod data;
pub mod mcp;
pub mod org;
pub mod property;
pub mod repo;
pub mod source;

use anyhow::{bail, Result};
use serde_json::json;

use crate::output::{print_json, Format};

/// Split the `org/repo` shorthand every repository command takes.
///
/// Both halves are Library usernames, which never contain a slash, so a
/// single split is unambiguous.
pub fn parse_repo_ref(value: &str) -> Result<(String, String)> {
    let mut parts = value.splitn(2, '/');
    let org = parts.next().unwrap_or_default().trim();
    let repo = parts.next().unwrap_or_default().trim();

    if org.is_empty() || repo.is_empty() {
        bail!("expected a repository as `org/repo`, got `{value}`");
    }

    Ok((org.to_string(), repo.to_string()))
}

/// Read a value that may point at a file. `@-` reads standard input, so
/// a long document can be piped in rather than quoted on the command
/// line; a literal leading `@` is escaped as `@@`.
pub fn read_possible_file(value: &str) -> Result<String> {
    use std::io::Read;

    if let Some(rest) = value.strip_prefix("@@") {
        return Ok(format!("@{rest}"));
    }
    let Some(path) = value.strip_prefix('@') else {
        return Ok(value.to_string());
    };

    if path == "-" {
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer)?;
        return Ok(buffer);
    }

    std::fs::read_to_string(path)
        .map_err(|error| anyhow::anyhow!("failed to read {path}: {error}"))
}

/// Split a `key=value` pair. The value may itself contain `=`.
pub fn parse_pair(raw: &str) -> Result<(String, String)> {
    let Some((key, value)) = raw.split_once('=') else {
        bail!("expected `key=value`, got `{raw}`");
    };
    let key = key.trim();
    if key.is_empty() {
        bail!("expected `key=value` with a non-empty key, got `{raw}`");
    }
    Ok((key.to_string(), value.to_string()))
}

/// Ask before a delete that cannot be undone.
///
/// A non-interactive caller — CI, or an agent — has no terminal to answer
/// from, so it must pass `--yes` rather than have the prompt silently
/// resolve itself.
pub fn confirm(already_confirmed: bool, question: &str) -> Result<()> {
    use std::io::{IsTerminal, Write};

    if already_confirmed {
        return Ok(());
    }
    if !std::io::stdin().is_terminal() {
        bail!(
            "{question}\nrefusing to continue without a terminal to \
             confirm on; pass --yes to proceed"
        );
    }

    print!("{question} [y/N] ");
    std::io::stdout().flush()?;

    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer)?;
    if !matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
        bail!("cancelled");
    }

    Ok(())
}

pub fn report_deleted(format: Format, kind: &str, id: &str) {
    match format {
        Format::Json => print_json(&json!({
            "deleted": true,
            "kind": kind,
            "id": id,
        })),
        Format::Text => println!("Deleted {kind} {id}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirming_up_front_skips_the_prompt() {
        assert!(confirm(true, "Delete everything?").is_ok());
    }

    #[test]
    fn a_repo_ref_splits_into_org_and_repo() {
        assert_eq!(
            parse_repo_ref("acme/docs").unwrap(),
            ("acme".to_string(), "docs".to_string())
        );
    }

    #[test]
    fn a_repo_ref_without_a_slash_is_rejected() {
        let error = parse_repo_ref("acme").unwrap_err().to_string();

        assert!(error.contains("org/repo"));
    }

    #[test]
    fn a_repo_ref_with_an_empty_half_is_rejected() {
        assert!(parse_repo_ref("acme/").is_err());
        assert!(parse_repo_ref("/docs").is_err());
    }

    #[test]
    fn a_pair_keeps_equals_signs_in_the_value() {
        assert_eq!(
            parse_pair("token=a=b").unwrap(),
            ("token".to_string(), "a=b".to_string())
        );
    }

    #[test]
    fn a_pair_without_an_equals_sign_is_rejected() {
        assert!(parse_pair("token").is_err());
    }

    #[test]
    fn a_doubled_at_sign_escapes_a_literal_at() {
        assert_eq!(read_possible_file("@@home").unwrap(), "@home");
    }

    #[test]
    fn a_plain_value_is_returned_untouched() {
        assert_eq!(read_possible_file("hello").unwrap(), "hello");
    }
}
