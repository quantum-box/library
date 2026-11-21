//! Building `https://api.github.com/repos/{owner}/{repo}/...` URLs from
//! values that come from outside.
//!
//! Repository names, refs and file paths reach these URLs from endpoint
//! settings, webhook payloads, repository trees and API arguments.
//! Formatting them into a string lets the URL parser read `%2e%2e` (or a
//! literal `..`) as a parent segment and normalise it away, so a file path
//! could leave `/repos/{owner}/{repo}/contents/` and make the caller's token
//! read any repository it can see; a `?` or `#` would cut off the query.
//! Every piece is therefore checked first and then added as its own
//! percent-encoded path segment or query pair.

use reqwest::Url;

/// Base of the GitHub REST API.
pub const GITHUB_API_BASE: &str = "https://api.github.com";

/// `https://api.github.com/repos/{owner}/{repo}/{tail...}`.
pub fn repo_url(
    owner: &str,
    repo: &str,
    tail: &[&str],
) -> errors::Result<Url> {
    validate_repository_part(owner)?;
    validate_repository_part(repo)?;
    let mut url =
        Url::parse(GITHUB_API_BASE).expect("GitHub API base URL is valid");
    url.path_segments_mut()
        .expect("GitHub API base URL can have a path")
        .pop_if_empty()
        .extend(["repos", owner, repo])
        .extend(tail);
    Ok(url)
}

/// Like [`repo_url`] for an `owner/repo` full name.
pub fn full_name_url(
    full_name: &str,
    tail: &[&str],
) -> errors::Result<Url> {
    let (owner, repo) = split_repository(full_name)?;
    repo_url(owner, repo, tail)
}

/// `.../repos/{owner}/{repo}/contents/{path}`, with `?ref=` when a ref is
/// given. An empty path names the repository root.
pub fn contents_url(
    owner: &str,
    repo: &str,
    path: &str,
    ref_name: Option<&str>,
) -> errors::Result<Url> {
    let mut tail = vec!["contents"];
    if !path.is_empty() {
        tail.extend(file_path_segments(path)?);
    }
    let mut url = repo_url(owner, repo, &tail)?;
    if let Some(ref_name) = ref_name {
        validate_ref(ref_name)?;
        url.query_pairs_mut().append_pair("ref", ref_name);
    }
    Ok(url)
}

/// Split `owner/repo`, accepting only the characters GitHub allows in
/// account and repository names.
pub fn split_repository(full_name: &str) -> errors::Result<(&str, &str)> {
    let (owner, repo) = full_name.split_once('/').ok_or_else(|| {
        errors::Error::invalid(format!(
            "Invalid GitHub repository name: {full_name:?}"
        ))
    })?;
    validate_repository_part(owner)?;
    validate_repository_part(repo)?;
    Ok((owner, repo))
}

fn validate_repository_part(part: &str) -> errors::Result<()> {
    if part.is_empty()
        || part == "."
        || part == ".."
        || !part.bytes().all(|b| {
            b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.')
        })
    {
        return Err(errors::Error::invalid(format!(
            "Invalid GitHub repository name: {part:?}"
        )));
    }
    Ok(())
}

/// Check a branch or ref name against git's ref-name rules, plus `%` and `#`
/// so that the name cannot be read as an encoded or cut-off URL part.
pub fn validate_ref(name: &str) -> errors::Result<()> {
    let bad_char = |c: char| {
        c.is_control()
            || matches!(
                c,
                ' ' | '~' | '^' | ':' | '?' | '*' | '[' | '\\' | '%' | '#'
            )
    };
    if name.is_empty()
        || name.contains("..")
        || name.contains("@{")
        || name.ends_with('.')
        || name.ends_with(".lock")
        || name.chars().any(bad_char)
        || name
            .split('/')
            .any(|segment| segment.is_empty() || segment.starts_with('.'))
    {
        return Err(errors::Error::invalid(format!(
            "Invalid GitHub ref name: {name:?}"
        )));
    }
    Ok(())
}

/// Validate a ref and split it into path segments (for `commits/{ref}` and
/// `git/trees/{ref}`, where GitHub accepts a ref containing `/`).
pub fn ref_segments(name: &str) -> errors::Result<Vec<&str>> {
    validate_ref(name)?;
    Ok(name.split('/').collect())
}

/// Split a repository file path into URL segments, refusing any segment that
/// is, or percent-decodes to, an empty, `.` or `..` segment, and any segment
/// with `\`, `?`, `#` or a control character.
pub fn file_path_segments(path: &str) -> errors::Result<Vec<&str>> {
    let invalid = || {
        errors::Error::invalid(format!("Unsafe GitHub file path: {path:?}"))
    };
    let segments: Vec<&str> = path.split('/').collect();
    for segment in &segments {
        if !is_safe_path_segment(segment) {
            return Err(invalid());
        }
        let decoded =
            urlencoding::decode(segment).map_err(|_| invalid())?;
        if !is_safe_path_segment(&decoded) || decoded.contains('/') {
            return Err(invalid());
        }
    }
    Ok(segments)
}

/// Whether a repository file path can be fetched through the contents API.
pub fn is_fetchable_file_path(path: &str) -> bool {
    file_path_segments(path).is_ok()
}

fn is_safe_path_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment != "."
        && segment != ".."
        && !segment
            .chars()
            .any(|c| c.is_control() || matches!(c, '\\' | '?' | '#'))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Before, this path sent the token to `/repos/x/y/contents/z.md`.
    const TRAVERSAL: &str =
        "docs/%2e%2e/%2e%2e/%2e%2e/%2e%2e/%2e%2e/repos/x/y/contents/z.md";

    fn rejected(result: errors::Result<Url>) -> bool {
        matches!(result, Err(errors::Error::BadRequest { .. }))
    }

    #[test]
    fn contents_url_encodes_each_segment_and_the_ref() {
        let url = contents_url(
            "octo-org",
            "docs.site",
            "guides/a b/ü.md",
            Some("feature/x"),
        )
        .unwrap();
        assert_eq!(
            url.as_str(),
            "https://api.github.com/repos/octo-org/docs.site/contents/\
             guides/a%20b/%C3%BC.md?ref=feature%2Fx"
        );
        assert_eq!(
            contents_url("o", "r", "", None).unwrap().as_str(),
            "https://api.github.com/repos/o/r/contents"
        );
    }

    #[test]
    fn encoded_parent_segments_cannot_leave_the_repository() {
        for path in [
            TRAVERSAL,
            "docs/../secret.md",
            "docs/./a.md",
            "docs//a.md",
            "/a.md",
            "docs/",
            "docs/.%2e/a.md",
            "docs/%2E%2E/a.md",
            "docs/%2e/a.md",
            "docs/a%2fb.md",
            "docs/a%5c..md",
            "docs\\..\\a.md",
            "x.env?a=.md",
            "a.md#frag",
        ] {
            assert!(
                rejected(contents_url("o", "r", path, Some("main"))),
                "{path}"
            );
            assert!(!is_fetchable_file_path(path), "{path}");
        }
    }

    #[test]
    fn a_percent_in_a_name_stays_literal() {
        let url =
            contents_url("o", "r", "docs/100%41.md", Some("main")).unwrap();
        assert_eq!(url.path(), "/repos/o/r/contents/docs/100%2541.md");
        assert_eq!(url.query(), Some("ref=main"));
    }

    #[test]
    fn repository_and_ref_are_validated() {
        for repo in [
            "o", "o/r/x", "/r", "o/", "../r", "o/..", "o/%2e%2e", "o/r?x",
            "o r/x",
        ] {
            assert!(rejected(full_name_url(repo, &["contents"])), "{repo}");
        }
        for (owner, repo) in [
            ("..", "r"),
            ("o", ".."),
            ("o/x", "r"),
            ("o", "r#"),
            ("", "r"),
        ] {
            assert!(
                rejected(contents_url(owner, repo, "a.md", None)),
                "{owner}/{repo}"
            );
        }
        for branch in [
            "", "../x", "a/../b", "a//b", ".hidden", "a?b", "a#b", "a%2e",
            "a b", "x.lock", "a\\b",
        ] {
            assert!(
                rejected(contents_url("o", "r", "a.md", Some(branch))),
                "{branch}"
            );
            assert!(ref_segments(branch).is_err(), "{branch}");
        }
        assert_eq!(
            ref_segments("release/1.0").unwrap(),
            ["release", "1.0"]
        );
    }
}
