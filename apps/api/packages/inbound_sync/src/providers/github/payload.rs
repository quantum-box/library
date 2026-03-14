//! GitHub webhook payload types.

use serde::{Deserialize, Serialize};

/// GitHub push event payload.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PushEvent {
    /// Git ref that was pushed (e.g., "refs/heads/main")
    #[serde(rename = "ref")]
    pub git_ref: String,
    /// Commit SHA before the push
    pub before: String,
    /// Commit SHA after the push
    pub after: String,
    /// Repository information
    pub repository: Repository,
    /// Person who pushed
    pub pusher: Pusher,
    /// List of commits in the push
    pub commits: Vec<Commit>,
    /// Head commit (most recent)
    pub head_commit: Option<Commit>,
    /// Whether this is a force push
    #[serde(default)]
    pub forced: bool,
    /// Whether this was a branch deletion
    #[serde(default)]
    pub deleted: bool,
    /// Whether this was a branch creation
    #[serde(default)]
    pub created: bool,
}

impl PushEvent {
    /// Get the branch name from the ref.
    pub fn branch(&self) -> Option<&str> {
        self.git_ref.strip_prefix("refs/heads/")
    }

    /// Get all added files across all commits.
    pub fn added_files(&self) -> Vec<&str> {
        self.commits
            .iter()
            .flat_map(|c| c.added.iter().map(|s| s.as_str()))
            .collect()
    }

    /// Get all modified files across all commits.
    pub fn modified_files(&self) -> Vec<&str> {
        self.commits
            .iter()
            .flat_map(|c| c.modified.iter().map(|s| s.as_str()))
            .collect()
    }

    /// Get all removed files across all commits.
    pub fn removed_files(&self) -> Vec<&str> {
        self.commits
            .iter()
            .flat_map(|c| c.removed.iter().map(|s| s.as_str()))
            .collect()
    }

    /// Get all changed files (added, modified, removed).
    pub fn all_changed_files(&self) -> Vec<ChangedFile> {
        let mut files = Vec::new();

        for file in self.added_files() {
            files.push(ChangedFile {
                path: file.to_string(),
                change_type: ChangeType::Added,
            });
        }
        for file in self.modified_files() {
            files.push(ChangedFile {
                path: file.to_string(),
                change_type: ChangeType::Modified,
            });
        }
        for file in self.removed_files() {
            files.push(ChangedFile {
                path: file.to_string(),
                change_type: ChangeType::Removed,
            });
        }

        files
    }
}

/// Repository information in push event.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Repository {
    /// Repository ID
    pub id: u64,
    /// Repository name
    pub name: String,
    /// Full repository name (owner/repo)
    pub full_name: String,
    /// Default branch
    pub default_branch: String,
    /// Repository URL
    pub html_url: String,
    /// Clone URL
    pub clone_url: String,
    /// Whether repository is private
    pub private: bool,
    /// Owner information
    pub owner: Owner,
}

/// Repository owner information.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Owner {
    /// Owner login
    pub login: String,
    /// Owner ID
    pub id: u64,
    /// Owner type (User or Organization)
    #[serde(rename = "type")]
    pub owner_type: String,
}

/// Person who pushed.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Pusher {
    /// Pusher name
    pub name: String,
    /// Pusher email
    pub email: String,
}

/// Commit information.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Commit {
    /// Commit SHA
    pub id: String,
    /// Short SHA (first 7 characters)
    pub tree_id: String,
    /// Commit message
    pub message: String,
    /// Commit timestamp
    pub timestamp: String,
    /// Commit URL
    pub url: String,
    /// Author information
    pub author: CommitAuthor,
    /// Committer information
    pub committer: CommitAuthor,
    /// Files added in this commit
    #[serde(default)]
    pub added: Vec<String>,
    /// Files modified in this commit
    #[serde(default)]
    pub modified: Vec<String>,
    /// Files removed in this commit
    #[serde(default)]
    pub removed: Vec<String>,
}

/// Commit author/committer information.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommitAuthor {
    /// Author name
    pub name: String,
    /// Author email
    pub email: String,
    /// Author username (optional)
    pub username: Option<String>,
}

/// Type of file change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    Added,
    Modified,
    Removed,
}

/// A changed file with its change type.
#[derive(Debug, Clone)]
pub struct ChangedFile {
    pub path: String,
    pub change_type: ChangeType,
}

impl ChangedFile {
    /// Check if this file matches a path pattern.
    ///
    /// Supports simple glob patterns:
    /// - `*` matches any sequence of characters except `/`
    /// - `**` matches any sequence of characters including `/`
    /// - `?` matches any single character
    pub fn matches_pattern(&self, pattern: &str) -> bool {
        glob_match(pattern, &self.path)
    }
}

/// Simple glob pattern matching.
pub fn glob_match(pattern: &str, path: &str) -> bool {
    let pattern_parts: Vec<&str> = pattern.split('/').collect();
    let path_parts: Vec<&str> = path.split('/').collect();

    glob_match_parts(&pattern_parts, &path_parts)
}

fn glob_match_parts(pattern: &[&str], path: &[&str]) -> bool {
    if pattern.is_empty() {
        return path.is_empty();
    }

    let p = pattern[0];

    if p == "**" {
        // Match zero or more path segments
        for i in 0..=path.len() {
            if glob_match_parts(&pattern[1..], &path[i..]) {
                return true;
            }
        }
        return false;
    }

    if path.is_empty() {
        return false;
    }

    if glob_match_segment(p, path[0]) {
        return glob_match_parts(&pattern[1..], &path[1..]);
    }

    false
}

fn glob_match_segment(pattern: &str, segment: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    let mut p_chars = pattern.chars().peekable();
    let mut s_chars = segment.chars().peekable();

    while let Some(pc) = p_chars.next() {
        match pc {
            '*' => {
                // Match any remaining characters
                if p_chars.peek().is_none() {
                    return true;
                }
                // Try matching rest of pattern at each position
                while s_chars.peek().is_some() {
                    let remaining_pattern: String =
                        p_chars.clone().collect();
                    let remaining_segment: String =
                        s_chars.clone().collect();
                    if glob_match_segment(
                        &remaining_pattern,
                        &remaining_segment,
                    ) {
                        return true;
                    }
                    s_chars.next();
                }
                return false;
            }
            '?' => {
                if s_chars.next().is_none() {
                    return false;
                }
            }
            c => {
                if s_chars.next() != Some(c) {
                    return false;
                }
            }
        }
    }

    s_chars.peek().is_none()
}

/// GitHub Pull Request event payload.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PullRequestEvent {
    /// Action type (opened, closed, synchronize, etc.)
    pub action: String,
    /// Pull request number
    pub number: u64,
    /// Pull request details
    pub pull_request: PullRequest,
    /// Repository information
    pub repository: Repository,
    /// Sender who triggered the event
    pub sender: User,
}

impl PullRequestEvent {
    /// Check if PR was merged.
    pub fn is_merged(&self) -> bool {
        self.action == "closed" && self.pull_request.merged
    }

    /// Check if this is a PR open event.
    pub fn is_opened(&self) -> bool {
        self.action == "opened"
    }

    /// Check if this is a synchronize event (new commits pushed).
    pub fn is_synchronized(&self) -> bool {
        self.action == "synchronize"
    }
}

/// Pull request details.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PullRequest {
    /// PR ID
    pub id: u64,
    /// PR number
    pub number: u64,
    /// PR state (open, closed)
    pub state: String,
    /// PR title
    pub title: String,
    /// PR body/description
    pub body: Option<String>,
    /// Whether PR was merged
    #[serde(default)]
    pub merged: bool,
    /// Merge commit SHA (if merged)
    pub merge_commit_sha: Option<String>,
    /// PR HTML URL
    pub html_url: String,
    /// Head branch info
    pub head: PullRequestRef,
    /// Base branch info
    pub base: PullRequestRef,
    /// User who created the PR
    pub user: User,
    /// Number of changed files
    #[serde(default)]
    pub changed_files: u32,
    /// Number of additions
    #[serde(default)]
    pub additions: u32,
    /// Number of deletions
    #[serde(default)]
    pub deletions: u32,
    /// Created at timestamp
    pub created_at: String,
    /// Updated at timestamp
    pub updated_at: String,
    /// Closed at timestamp (if closed)
    pub closed_at: Option<String>,
    /// Merged at timestamp (if merged)
    pub merged_at: Option<String>,
}

/// Pull request branch reference.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PullRequestRef {
    /// Branch name
    #[serde(rename = "ref")]
    pub branch_ref: String,
    /// Commit SHA
    pub sha: String,
    /// Repository info
    pub repo: Option<Repository>,
}

/// GitHub user information.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    /// User login
    pub login: String,
    /// User ID
    pub id: u64,
    /// Avatar URL
    pub avatar_url: Option<String>,
    /// User type
    #[serde(rename = "type")]
    pub user_type: String,
}

/// PR action types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullRequestAction {
    Opened,
    Closed,
    Reopened,
    Synchronize,
    Edited,
    Labeled,
    Unlabeled,
    ReadyForReview,
    Merged,
    Other,
}

impl PullRequestAction {
    pub fn parse_str(s: &str) -> Self {
        match s {
            "opened" => PullRequestAction::Opened,
            "closed" => PullRequestAction::Closed,
            "reopened" => PullRequestAction::Reopened,
            "synchronize" => PullRequestAction::Synchronize,
            "edited" => PullRequestAction::Edited,
            "labeled" => PullRequestAction::Labeled,
            "unlabeled" => PullRequestAction::Unlabeled,
            "ready_for_review" => PullRequestAction::ReadyForReview,
            _ => PullRequestAction::Other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match_exact() {
        assert!(glob_match("docs/article.md", "docs/article.md"));
        assert!(!glob_match("docs/article.md", "docs/other.md"));
    }

    #[test]
    fn test_glob_match_star() {
        assert!(glob_match("docs/*.md", "docs/article.md"));
        assert!(glob_match("docs/*.md", "docs/README.md"));
        assert!(!glob_match("docs/*.md", "docs/sub/article.md"));
        assert!(!glob_match("docs/*.md", "src/article.md"));
    }

    #[test]
    fn test_glob_match_double_star() {
        assert!(glob_match("docs/**/*.md", "docs/article.md"));
        assert!(glob_match("docs/**/*.md", "docs/sub/article.md"));
        assert!(glob_match("docs/**/*.md", "docs/a/b/c/article.md"));
        assert!(!glob_match("docs/**/*.md", "src/article.md"));
    }

    #[test]
    fn test_glob_match_question() {
        assert!(glob_match("doc?/article.md", "docs/article.md"));
        assert!(!glob_match("doc?/article.md", "document/article.md"));
    }

    #[test]
    fn test_push_event_branch() {
        let event = PushEvent {
            git_ref: "refs/heads/main".to_string(),
            before: "abc".to_string(),
            after: "def".to_string(),
            repository: Repository {
                id: 1,
                name: "repo".to_string(),
                full_name: "owner/repo".to_string(),
                default_branch: "main".to_string(),
                html_url: "https://github.com/owner/repo".to_string(),
                clone_url: "https://github.com/owner/repo.git".to_string(),
                private: false,
                owner: Owner {
                    login: "owner".to_string(),
                    id: 1,
                    owner_type: "User".to_string(),
                },
            },
            pusher: Pusher {
                name: "user".to_string(),
                email: "user@example.com".to_string(),
            },
            commits: vec![],
            head_commit: None,
            forced: false,
            deleted: false,
            created: false,
        };

        assert_eq!(event.branch(), Some("main"));
    }
}
