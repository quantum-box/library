//! List GitHub directory contents usecase

use std::sync::Arc;

use tachyon_sdk::auth::{AuthApp, GetOAuthTokenByProviderInput};

use crate::usecase::{
    GitHubFileInfo, ListGitHubDirectoryInputData,
    ListGitHubDirectoryInputPort, ListGitHubDirectoryOutputData,
};

#[derive(Clone)]
pub struct ListGitHubDirectory {
    auth: Arc<dyn AuthApp>,
}

impl std::fmt::Debug for ListGitHubDirectory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListGitHubDirectory")
            .finish_non_exhaustive()
    }
}

impl ListGitHubDirectory {
    pub fn new(auth: Arc<dyn AuthApp>) -> Arc<Self> {
        Arc::new(Self { auth })
    }
}

#[async_trait::async_trait]
impl ListGitHubDirectoryInputPort for ListGitHubDirectory {
    #[tracing::instrument(
        name = "ListGitHubDirectory::execute",
        skip(self)
    )]
    async fn execute<'a>(
        &self,
        input: ListGitHubDirectoryInputData<'a>,
    ) -> errors::Result<ListGitHubDirectoryOutputData> {
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

        // List directory contents
        let listing = github_provider::GitHub::list_directory_contents(
            &token.access_token,
            owner,
            repo,
            &input.path,
            input.ref_name.as_deref(),
        )
        .await?;

        // Filter to only markdown files and directories
        let files: Vec<GitHubFileInfo> = listing
            .contents
            .into_iter()
            .filter(|c| {
                c.content_type == "dir"
                    || c.name.ends_with(".md")
                    || c.name.ends_with(".mdx")
            })
            .map(|c| GitHubFileInfo {
                name: c.name,
                path: c.path,
                sha: c.sha,
                size: c.size,
                file_type: c.content_type,
                html_url: c.html_url,
            })
            .collect();

        Ok(ListGitHubDirectoryOutputData {
            files,
            truncated: listing.truncated,
        })
    }
}
