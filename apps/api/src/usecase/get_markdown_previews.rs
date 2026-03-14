//! Get Markdown previews usecase

use std::sync::Arc;

use tachyon_sdk::auth::{AuthApp, GetOAuthTokenByProviderInput};

use crate::usecase::{
    GetMarkdownPreviewsInputData, GetMarkdownPreviewsInputPort,
    MarkdownImportPreview,
};

#[derive(Clone)]
pub struct GetMarkdownPreviews {
    auth: Arc<dyn AuthApp>,
}

impl std::fmt::Debug for GetMarkdownPreviews {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GetMarkdownPreviews")
            .finish_non_exhaustive()
    }
}

impl GetMarkdownPreviews {
    pub fn new(auth: Arc<dyn AuthApp>) -> Arc<Self> {
        Arc::new(Self { auth })
    }
}

#[async_trait::async_trait]
impl GetMarkdownPreviewsInputPort for GetMarkdownPreviews {
    #[tracing::instrument(
        name = "GetMarkdownPreviews::execute",
        skip(self)
    )]
    async fn execute<'a>(
        &self,
        input: GetMarkdownPreviewsInputData<'a>,
    ) -> errors::Result<Vec<MarkdownImportPreview>> {
        // Get GitHub OAuth token
        let token = self
            .auth
            .get_oauth_token_by_provider(&GetOAuthTokenByProviderInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                provider: "github",
            })
            .await?
            .ok_or_else(|| {
                errors::Error::unauthorized(
                    "GitHub is not connected. Please connect your GitHub account first.",
                )
            })?;

        // Parse owner/repo format
        let parts: Vec<&str> = input.github_repo.split('/').collect();
        if parts.len() != 2 {
            return Err(errors::Error::bad_request(
                "Invalid github_repo format. Expected 'owner/repo'.",
            ));
        }
        let owner = parts[0];
        let repo = parts[1];

        let mut previews = Vec::new();

        for path in &input.paths {
            let preview = get_single_markdown_preview(
                &token.access_token,
                owner,
                repo,
                path,
                input.ref_name.as_deref(),
            )
            .await;
            previews.push(preview);
        }

        Ok(previews)
    }
}

/// Get preview for a single markdown file
async fn get_single_markdown_preview(
    access_token: &str,
    owner: &str,
    repo: &str,
    path: &str,
    ref_name: Option<&str>,
) -> MarkdownImportPreview {
    // Get file content
    let content_result = github_provider::GitHub::get_raw_file_content(
        access_token,
        owner,
        repo,
        path,
        ref_name,
    )
    .await;

    let content = match content_result {
        Ok(c) => c,
        Err(e) => {
            return MarkdownImportPreview {
                path: path.to_string(),
                sha: String::new(),
                frontmatter_json: None,
                frontmatter_keys: vec![],
                suggested_name: extract_filename(path),
                body_preview: String::new(),
                parse_error: Some(format!("Failed to fetch file: {e}")),
            };
        }
    };

    // Get file info for SHA
    let file_info_result =
        github_provider::GitHub::list_directory_contents(
            access_token,
            owner,
            repo,
            path,
            ref_name,
        )
        .await;

    let sha = file_info_result
        .ok()
        .and_then(|listing| listing.contents.first().map(|c| c.sha.clone()))
        .unwrap_or_default();

    // Parse frontmatter
    let (frontmatter, body) = parse_frontmatter(&content);

    let frontmatter_json = frontmatter.as_ref().map(|fm| fm.to_string());
    let frontmatter_keys = frontmatter
        .as_ref()
        .and_then(|fm| fm.as_object())
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default();

    // Extract suggested name
    let suggested_name = extract_title(&frontmatter, &body, path);

    // Create body preview (first 500 chars)
    let body_preview: String = body.chars().take(500).collect();

    MarkdownImportPreview {
        path: path.to_string(),
        sha,
        frontmatter_json,
        frontmatter_keys,
        suggested_name,
        body_preview,
        parse_error: None,
    }
}

/// Parse YAML frontmatter from markdown content
pub fn parse_frontmatter(
    content: &str,
) -> (Option<serde_json::Value>, String) {
    let content = content.trim();

    if !content.starts_with("---") {
        return (None, content.to_string());
    }

    // Find the closing ---
    let rest = &content[3..];
    if let Some(end_pos) = rest.find("\n---") {
        let frontmatter_str = &rest[..end_pos].trim();
        let body = &rest[end_pos + 4..].trim();

        // Parse YAML frontmatter
        match serde_yaml::from_str::<serde_json::Value>(frontmatter_str) {
            Ok(value) => (Some(value), body.to_string()),
            Err(_) => (None, content.to_string()),
        }
    } else {
        (None, content.to_string())
    }
}

/// Extract title from frontmatter, H1, or filename
pub fn extract_title(
    frontmatter: &Option<serde_json::Value>,
    body: &str,
    path: &str,
) -> String {
    // 1. Try frontmatter title
    if let Some(fm) = frontmatter {
        if let Some(title) = fm.get("title").and_then(|v| v.as_str()) {
            if !title.is_empty() {
                return title.to_string();
            }
        }
    }

    // 2. Try H1 from body
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(stripped) = trimmed.strip_prefix("# ") {
            let title = stripped.trim();
            if !title.is_empty() {
                return title.to_string();
            }
        }
    }

    // 3. Fall back to filename
    extract_filename(path)
}

/// Extract filename without extension
pub fn extract_filename(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string()
}
