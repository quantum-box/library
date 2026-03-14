use std::sync::Arc;

use super::input::{GetMarkdownPreviewsInput, ListGitHubDirectoryInput};
use super::model::*;
use crate::app::LibraryApp;
use crate::sdk_auth::SdkAuthApp;
use crate::usecase::{
    self, FindSourcesInputData, GetMarkdownPreviewsInputData,
    GetRepoMembersInputPort, GetRepoPoliciesInputPort, GetSourceInputData,
    ListGitHubDirectoryInputData,
};
use async_graphql::{
    ComplexObject, Context, ErrorExtensions, Object, Result,
};
use inbound_sync::providers::linear::LinearClient;
use tachyon_sdk::auth::ExecutorAction;
use tachyon_sdk::auth::MultiTenancyAction;
use value_object;

#[derive(Default)]
pub struct LibraryQuery;

fn err_test() -> errors::Result<String> {
    Err(errors::permission_denied!("test-message"))
}

#[Object]
impl LibraryQuery {
    async fn err_test(&self, _ctx: &Context<'_>) -> Result<String> {
        err_test()?;
        Ok("test".to_string())
    }

    #[tracing::instrument(name = "library_me", skip(self, ctx))]
    async fn me(&self, ctx: &Context<'_>) -> Result<User> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let sdk = ctx.data::<Arc<SdkAuthApp>>()?;

        // Resolve operator_id from multi_tenancy
        let operator_id =
            multi_tenancy.get_operator_id().map_err(|e| e.extend())?;

        let user_id = executor.get_id();
        let user = sdk
            .get_user_by_id_full(&operator_id, user_id)
            .await
            .map_err(|e| {
                tracing::error!("error: {:?}", e);
                e.extend()
            })?
            .ok_or_else(|| async_graphql::Error::new("User not found"))?;

        Ok(user.into())
    }

    #[tracing::instrument(name = "library_organization", skip(self, ctx))]
    async fn organization(
        &self,
        ctx: &Context<'_>,
        username: String,
    ) -> Result<Organization> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let ctx = ctx.data::<Arc<LibraryApp>>()?;
        let output = ctx
            .view_org
            .execute(&crate::usecase::ViewOrgInputData {
                executor,
                multi_tenancy,
                organization_username: username,
            })
            .await
            .map_err(|e| {
                tracing::error!("error: {:?}", e);
                e.extend()
            })?;
        let org = output.organization.into();
        Ok(Organization {
            repos: output.repos.into_iter().map(|r| r.into()).collect(),
            ..org
        })
    }

    #[tracing::instrument(name = "library_repo", skip(self, ctx))]
    async fn repo(
        &self,
        ctx: &Context<'_>,
        org_username: String,
        repo_username: String,
    ) -> Result<Repo> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let ctx = ctx.data::<Arc<LibraryApp>>()?;
        let output = ctx
            .view_repo
            .execute(&crate::usecase::ViewRepoInputData {
                executor,
                multi_tenancy,
                organization_username: org_username,
                repo_username,
            })
            .await
            .map_err(|e| {
                tracing::error!("error: {:?}", e);
                e.extend()
            })?;
        Ok(output.repo.into())
    }

    #[tracing::instrument(name = "library_data", skip(self, ctx))]
    async fn data(
        &self,
        ctx: &Context<'_>,
        org_username: String,
        repo_username: String,
        data_id: String,
    ) -> Result<Data> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let ctx = ctx.data::<Arc<LibraryApp>>()?;
        let output = ctx
            .view_data
            .execute(&crate::usecase::ViewDataInputData {
                executor,
                multi_tenancy,
                org_username,
                repo_username,
                data_id,
            })
            .await
            .map_err(|e| {
                tracing::error!("error: {:?}", e);
                e.extend()
            })?;
        Ok(output.0.into())
    }

    #[tracing::instrument(name = "library_data_list", skip(self, ctx))]
    async fn data_list(
        &self,
        ctx: &Context<'_>,
        org_username: String,
        repo_username: String,
        page_size: Option<u32>,
        page: Option<u32>,
    ) -> Result<DataList> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let ctx = ctx.data::<Arc<LibraryApp>>()?;
        let output = ctx
            .view_data_list
            .execute(&crate::usecase::ViewDataListInputData {
                executor,
                multi_tenancy,
                org_username,
                repo_username,
                page_size,
                page,
            })
            .await
            .map_err(|e| {
                tracing::error!("error: {:?}", e);
                e.extend()
            })?;
        Ok(DataList {
            items: output.0.into_iter().map(|d| d.into()).collect(),
            paginator: output.2.into(),
        })
    }

    #[tracing::instrument(name = "library_properties", skip(self, ctx))]
    async fn properties(
        &self,
        ctx: &Context<'_>,
        org_username: String,
        repo_username: String,
    ) -> Result<Vec<Property>> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let ctx = ctx.data::<Arc<LibraryApp>>()?;
        Ok(ctx
            .get_properties
            .execute(crate::usecase::GetPropertiesInputData {
                executor,
                multi_tenancy,
                org_username,
                repo_username,
            })
            .await
            .map_err(|e| {
                tracing::error!("error: {:?}", e);
                e.extend()
            })?
            .into_iter()
            .map(|p| p.into())
            .collect())
    }

    #[tracing::instrument(name = "library_source", skip(self, ctx))]
    async fn source(
        &self,
        ctx: &Context<'_>,
        org_username: String,
        repo_username: String,
        source_id: String,
    ) -> Result<Source> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

        // TODO: add English comment
        let source = app
            .get_source
            .execute(GetSourceInputData {
                executor,
                multi_tenancy,
                source_id: &source_id.parse()?,
                org_username,
                repo_username,
            })
            .await
            .map_err(|e| {
                tracing::error!("error: {:?}", e);
                e.extend()
            })?
            .ok_or(errors::not_found!("Source not found"))?;

        Ok(source.into())
    }

    #[tracing::instrument(name = "library_api_keys", skip(self, ctx))]
    async fn api_keys(
        &self,
        ctx: &Context<'_>,
        org_username: String,
    ) -> Result<Vec<PublicApiKey>> {
        let executor = ctx.data_unchecked::<tachyon_sdk::auth::Executor>();
        let multi_tenancy = ctx.data_unchecked::<tachyon_sdk::auth::MultiTenancy>();
        let app = ctx.data_unchecked::<Arc<LibraryApp>>();

        // TODO: add English comment
        let api_keys = app
            .list_api_keys
            .execute(&crate::usecase::ListApiKeysInputData {
                executor,
                multi_tenancy,
                org_name: &org_username.parse()?,
            })
            .await
            .map_err(|e| {
                tracing::error!("error: {:?}", e);
                e.extend()
            })?;

        Ok(api_keys.into_iter().map(PublicApiKey::from).collect())
    }

    /// [LIBRARY-API] Get GitHub connection status
    #[tracing::instrument(name = "github_connection", skip(self, ctx))]
    async fn github_connection(
        &self,
        ctx: &Context<'_>,
    ) -> Result<GitHubConnection> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let auth_app =
            ctx.data::<Arc<dyn tachyon_sdk::auth::AuthApp>>()?;

        let token = auth_app
            .get_oauth_token_by_provider(
                &tachyon_sdk::auth::GetOAuthTokenByProviderInput {
                    executor,
                    multi_tenancy,
                    provider: "github",
                },
            )
            .await
            .map_err(|e| {
                tracing::error!("error: {:?}", e);
                e.extend()
            })?;

        match token {
            Some(t) => Ok(GitHubConnection {
                connected: true,
                username: Some(t.provider_user_id.clone()),
                // Note: connected_at is not stored in the database, so we return None
                // The token record doesn't have a created_at field to determine actual connection time
                connected_at: None,
                expires_at: Some(t.expires_at),
            }),
            None => Ok(GitHubConnection {
                connected: false,
                username: None,
                connected_at: None,
                expires_at: None,
            }),
        }
    }

    /// [LIBRARY-API] List GitHub repositories accessible to the user
    #[tracing::instrument(
        name = "github_list_repositories",
        skip(self, ctx)
    )]
    async fn github_list_repositories(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Search query to filter repositories")]
        search: Option<String>,
        #[graphql(desc = "Number of repositories per page (max 100)")]
        per_page: Option<i32>,
        #[graphql(desc = "Page number (1-indexed)")] page: Option<i32>,
    ) -> Result<Vec<GitHubRepository>> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let auth_app =
            ctx.data::<Arc<dyn tachyon_sdk::auth::AuthApp>>()?;

        // Get the OAuth token
        let token = auth_app
            .get_oauth_token_by_provider(
                &tachyon_sdk::auth::GetOAuthTokenByProviderInput {
                    executor,
                    multi_tenancy,
                    provider: "github",
                },
            )
            .await
            .map_err(|e| {
                tracing::error!("error: {:?}", e);
                e.extend()
            })?;

        let token = token.ok_or_else(|| {
            errors::Error::unauthorized(
                "GitHub is not connected. Please connect your GitHub account first.",
            )
        })?;

        // Validate pagination parameters to prevent integer underflow
        let per_page_value = per_page.unwrap_or(30).clamp(1, 100) as u32;
        let page_value = page.unwrap_or(1).max(1) as u32;

        // List repositories using the GitHub API
        let repos = github_provider::GitHub::list_repositories(
            &token.access_token,
            search.as_deref(),
            per_page_value,
            page_value,
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to list GitHub repositories: {:?}", e);
            e.extend()
        })?;

        Ok(repos.into_iter().map(GitHubRepository::from).collect())
    }

    /// [LIBRARY-API] List directory contents from a GitHub repository
    #[tracing::instrument(
        name = "github_list_directory_contents",
        skip(self, ctx)
    )]
    async fn github_list_directory_contents(
        &self,
        ctx: &Context<'_>,
        input: ListGitHubDirectoryInput,
    ) -> Result<GitHubDirectoryContents> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

        let output = app
            .list_github_directory
            .execute(ListGitHubDirectoryInputData {
                executor,
                multi_tenancy,
                github_repo: input.github_repo,
                path: input.path,
                ref_name: input.ref_name,
            })
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to list GitHub directory contents: {:?}",
                    e
                );
                e.extend()
            })?;

        let files: Vec<GitHubFileInfo> = output
            .files
            .into_iter()
            .map(|f| GitHubFileInfo {
                name: f.name,
                path: f.path,
                sha: f.sha,
                size: f.size,
                file_type: f.file_type,
                html_url: f.html_url,
            })
            .collect();

        Ok(GitHubDirectoryContents {
            files,
            truncated: output.truncated,
        })
    }

    /// [LIBRARY-API] Get previews of Markdown files for import
    #[tracing::instrument(
        name = "github_get_markdown_previews",
        skip(self, ctx)
    )]
    async fn github_get_markdown_previews(
        &self,
        ctx: &Context<'_>,
        input: GetMarkdownPreviewsInput,
    ) -> Result<Vec<MarkdownImportPreview>> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

        let previews = app
            .get_markdown_previews
            .execute(GetMarkdownPreviewsInputData {
                executor,
                multi_tenancy,
                github_repo: input.github_repo,
                paths: input.paths,
                ref_name: input.ref_name,
            })
            .await
            .map_err(|e| {
                tracing::error!("Failed to get markdown previews: {:?}", e);
                e.extend()
            })?;

        Ok(previews
            .into_iter()
            .map(|p| MarkdownImportPreview {
                path: p.path,
                sha: p.sha,
                frontmatter_json: p.frontmatter_json,
                frontmatter_keys: p.frontmatter_keys,
                suggested_name: p.suggested_name,
                body_preview: p.body_preview,
                parse_error: p.parse_error,
            })
            .collect())
    }

    /// [LIBRARY-API] Analyze frontmatter across multiple files
    #[tracing::instrument(
        name = "github_analyze_frontmatter",
        skip(self, ctx)
    )]
    async fn github_analyze_frontmatter(
        &self,
        ctx: &Context<'_>,
        input: GetMarkdownPreviewsInput,
    ) -> Result<FrontmatterAnalysis> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

        let analysis = app
            .analyze_frontmatter
            .execute(GetMarkdownPreviewsInputData {
                executor,
                multi_tenancy,
                github_repo: input.github_repo,
                paths: input.paths,
                ref_name: input.ref_name,
            })
            .await
            .map_err(|e| {
                tracing::error!("Failed to analyze frontmatter: {:?}", e);
                e.extend()
            })?;

        let properties: Vec<SuggestedProperty> = analysis
            .properties
            .into_iter()
            .map(|p| {
                let suggested_type = match p.suggested_type.as_str() {
                    "SELECT" => PropertyType::Select,
                    "INTEGER" => PropertyType::Integer,
                    "MARKDOWN" => PropertyType::Markdown,
                    _ => PropertyType::String,
                };
                SuggestedProperty {
                    key: p.key,
                    suggested_type,
                    unique_values: p.unique_values,
                    suggest_select: p.suggest_select,
                }
            })
            .collect();

        Ok(FrontmatterAnalysis {
            properties,
            total_files: analysis.total_files,
            valid_files: analysis.valid_files,
        })
    }

    /// [LIBRARY-API] List all available integrations in the marketplace
    #[tracing::instrument(name = "list_integrations", skip(self, ctx))]
    async fn integrations(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<Integration>> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let database_app = ctx.data::<Arc<database_manager::App>>()?;

        let output = database_app
            .list_integrations()
            .execute(inbound_sync::usecase::ListIntegrationsInputData {
                executor,
                multi_tenancy,
                filter: inbound_sync::usecase::IntegrationFilter::All,
            })
            .await
            .map_err(|e| {
                tracing::error!("Failed to list integrations: {:?}", e);
                e.extend()
            })?;

        Ok(output
            .integrations
            .into_iter()
            .map(|i| Integration {
                id: i.id().as_str().to_string(),
                provider: i.provider().to_string(),
                name: i.name().to_string(),
                description: i.description().to_string(),
                icon: i.icon().map(|s| s.to_string()),
                category: format!("{:?}", i.category()),
                sync_capability: format!("{:?}", i.sync_capability()),
                supported_objects: i.supported_objects().to_vec(),
                requires_oauth: i.oauth_config().is_some(),
                is_enabled: i.is_enabled(),
                is_featured: i.is_featured(),
            })
            .collect())
    }

    /// [LIBRARY-API] List tenant's connections to integrations
    #[tracing::instrument(name = "list_connections", skip(self, ctx))]
    async fn connections(
        &self,
        ctx: &Context<'_>,
        #[graphql(
            desc = "Tenant ID (deprecated, uses multi_tenancy context)"
        )]
        _tenant_id: String,
    ) -> Result<Vec<Connection>> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let database_app = ctx.data::<Arc<database_manager::App>>()?;

        let output = database_app
            .list_connections()
            .execute(inbound_sync::usecase::ListConnectionsInputData {
                executor,
                multi_tenancy,
                active_only: false,
            })
            .await
            .map_err(|e| {
                tracing::error!("Failed to list connections: {:?}", e);
                e.extend()
            })?;

        Ok(output
            .connections
            .into_iter()
            .map(|c| Connection {
                id: c.id().as_str().to_string(),
                integration_id: c.integration_id().as_str().to_string(),
                provider: c.provider().to_string(),
                status: match c.status() {
                    inbound_sync::ConnectionStatus::Active => {
                        ConnectionStatus::Active
                    }
                    inbound_sync::ConnectionStatus::Expired => {
                        ConnectionStatus::Expired
                    }
                    inbound_sync::ConnectionStatus::Paused => {
                        ConnectionStatus::Paused
                    }
                    inbound_sync::ConnectionStatus::Error => {
                        ConnectionStatus::Error
                    }
                    inbound_sync::ConnectionStatus::Disconnected => {
                        ConnectionStatus::Disconnected
                    }
                },
                external_account_id: c
                    .external_account_id()
                    .map(|s| s.to_string()),
                external_account_name: c
                    .external_account_name()
                    .map(|s| s.to_string()),
                connected_at: c.connected_at(),
                last_synced_at: c.last_synced_at(),
                error_message: c.error_message().map(|s| s.to_string()),
            })
            .collect())
    }

    /// [LIBRARY-API] List Linear teams
    #[tracing::instrument(name = "linear_list_teams", skip(self, ctx))]
    async fn linear_list_teams(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<LinearTeam>> {
        let multi_tenancy = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let oauth_token_repo = ctx
            .data::<Arc<dyn inbound_sync_domain::OAuthTokenRepository>>()?;

        let tenant_id = multi_tenancy.operator_id().ok_or_else(|| {
            async_graphql::Error::new(
                "Operator ID required for Linear teams list",
            )
        })?;

        // Get the OAuth token
        let token = oauth_token_repo
            .find_by_tenant_and_provider(
                &tenant_id,
                inbound_sync_domain::OAuthProvider::Linear,
            )
            .await
            .map_err(|e| {
                tracing::error!("error: {:?}", e);
                e.extend()
            })?;

        let token = token.ok_or_else(|| {
            errors::Error::unauthorized(
                "Linear is not connected. Please connect your Linear account first.",
            )
        })?;

        // Create LinearApiClient
        use inbound_sync::providers::linear::LinearApiClient;
        let linear_client =
            LinearApiClient::new(format!("Bearer {}", token.access_token));

        // List teams using the Linear API
        let teams =
            linear_client.list_teams(&tenant_id).await.map_err(|e| {
                tracing::error!("Failed to list Linear teams: {:?}", e);
                e.extend()
            })?;

        Ok(teams
            .into_iter()
            .map(|t| LinearTeam {
                id: t.id,
                name: t.name,
                key: t.key,
            })
            .collect())
    }

    /// [LIBRARY-API] List Linear projects
    #[tracing::instrument(name = "linear_list_projects", skip(self, ctx))]
    async fn linear_list_projects(
        &self,
        ctx: &Context<'_>,
        team_id: Option<String>,
    ) -> Result<Vec<LinearProject>> {
        let multi_tenancy = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let oauth_token_repo = ctx
            .data::<Arc<dyn inbound_sync_domain::OAuthTokenRepository>>()?;

        let tenant_id = multi_tenancy.operator_id().ok_or_else(|| {
            async_graphql::Error::new(
                "Operator ID required for Linear projects list",
            )
        })?;

        let token = oauth_token_repo
            .find_by_tenant_and_provider(
                &tenant_id,
                inbound_sync_domain::OAuthProvider::Linear,
            )
            .await
            .map_err(|e| {
                tracing::error!("error: {:?}", e);
                e.extend()
            })?;

        let token = token.ok_or_else(|| {
            errors::Error::unauthorized(
                "Linear is not connected. Please connect your Linear account first.",
            )
        })?;

        use inbound_sync::providers::linear::LinearApiClient;
        let linear_client =
            LinearApiClient::new(format!("Bearer {}", token.access_token));

        let projects = linear_client
            .list_projects(&tenant_id, team_id.as_deref())
            .await
            .map_err(|e| {
                tracing::error!("Failed to list Linear projects: {:?}", e);
                e.extend()
            })?;

        Ok(projects
            .into_iter()
            .map(|p| LinearProject {
                id: p.id,
                name: p.name,
            })
            .collect())
    }

    /// [LIBRARY-API] List Linear issues
    #[tracing::instrument(name = "linear_list_issues", skip(self, ctx))]
    async fn linear_list_issues(
        &self,
        ctx: &Context<'_>,
        team_id: Option<String>,
        project_id: Option<String>,
    ) -> Result<Vec<LinearIssue>> {
        let multi_tenancy = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let oauth_token_repo = ctx
            .data::<Arc<dyn inbound_sync_domain::OAuthTokenRepository>>()?;

        let tenant_id = multi_tenancy.operator_id().ok_or_else(|| {
            async_graphql::Error::new(
                "Operator ID required for Linear issues list",
            )
        })?;

        let token = oauth_token_repo
            .find_by_tenant_and_provider(
                &tenant_id,
                inbound_sync_domain::OAuthProvider::Linear,
            )
            .await
            .map_err(|e| {
                tracing::error!("error: {:?}", e);
                e.extend()
            })?;

        let token = token.ok_or_else(|| {
            errors::Error::unauthorized(
                "Linear is not connected. Please connect your Linear account first.",
            )
        })?;

        use inbound_sync::providers::linear::LinearApiClient;
        let linear_client =
            LinearApiClient::new(format!("Bearer {}", token.access_token));

        let issues = linear_client
            .list_issues(
                &tenant_id,
                team_id.as_deref(),
                project_id.as_deref(),
            )
            .await
            .map_err(|e| {
                tracing::error!("Failed to list Linear issues: {:?}", e);
                e.extend()
            })?;

        Ok(issues
            .into_iter()
            .map(|issue| {
                let state_name = issue.state.map(|state| state.name);
                let assignee_name =
                    issue.assignee.map(|assignee| assignee.name);
                LinearIssue {
                    id: issue.id,
                    identifier: issue.identifier,
                    title: issue.title,
                    state_name,
                    assignee_name,
                    url: issue.url,
                }
            })
            .collect())
    }
}

#[ComplexObject]
impl Organization {
    #[tracing::instrument(skip(ctx))]
    async fn users(&self, ctx: &Context<'_>) -> Result<Vec<User>> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        if executor.is_none() {
            return Ok(vec![]);
        }
        let auth_app =
            ctx.data::<Arc<dyn tachyon_sdk::auth::AuthApp>>()?;
        let tenant_id: value_object::TenantId =
            value_object::TenantId::new(&self.id)
                .map_err(|e| e.extend())?;
        let users = tachyon_sdk::auth::AuthApp::find_users_by_tenant(
            auth_app.as_ref(),
            &tachyon_sdk::auth::FindUsersByTenantInput {
                executor,
                multi_tenancy,
                tenant_id: &tenant_id,
            },
        )
        .await
        .map_err(|e| {
            tracing::error!("error: {:?}", e);
            e.extend()
        })?;

        Ok(users.into_iter().map(Into::into).collect())
    }
}

#[ComplexObject]
impl Repo {
    async fn data_list(
        &self,
        ctx: &Context<'_>,
        page_size: Option<u32>,
        page: Option<u32>,
    ) -> Result<DataList> {
        let org_username = self.org_username.clone();
        let repo_username = self.username.clone();
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let ctx = ctx.data::<Arc<LibraryApp>>()?;
        let output = ctx
            .view_data_list
            .execute(&crate::usecase::ViewDataListInputData {
                executor,
                multi_tenancy,
                org_username,
                repo_username,
                page_size,
                page,
            })
            .await
            .map_err(|e| {
                tracing::error!("error: {:?}", e);
                e.extend()
            })?;
        Ok(DataList {
            items: output.0.into_iter().map(|d| d.into()).collect(),
            paginator: output.2.into(),
        })
    }

    #[tracing::instrument(name = "library_properties", skip(self, ctx))]
    async fn properties(&self, ctx: &Context<'_>) -> Result<Vec<Property>> {
        let org_username = self.org_username.clone();
        let repo_username = self.username.clone();
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let ctx = ctx.data::<Arc<LibraryApp>>()?;
        Ok(ctx
            .get_properties
            .execute(crate::usecase::GetPropertiesInputData {
                executor,
                multi_tenancy,
                org_username,
                repo_username,
            })
            .await
            .map_err(|e| {
                tracing::error!("error: {:?}", e);
                e.extend()
            })?
            .into_iter()
            .map(|p| p.into())
            .collect())
    }

    #[tracing::instrument(name = "library_sources", skip(self, ctx))]
    async fn sources(&self, ctx: &Context<'_>) -> Result<Vec<Source>> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

        // TODO: add English comment
        let sources = app
            .find_sources
            .execute(FindSourcesInputData {
                executor,
                multi_tenancy,
                repo_id: &self.id.parse()?,
                org_username: self.org_username.clone(),
                repo_username: self.username.clone(),
            })
            .await
            .map_err(|e| {
                tracing::error!("error: {:?}", e);
                e.extend()
            })?;
        Ok(sources.into_iter().map(|s| s.into()).collect())
    }

    /// Get policies for this repository
    ///
    /// Returns resource-based policies scoped to this repository.
    ///
    /// For authenticated requests the handler injects a
    /// request-scoped `AuthApp` carrying the caller's JWT,
    /// so we build a fresh `GetRepoPolicies` with it instead
    /// of using the schema-level one (which may hold a
    /// placeholder token that production tachyon-api rejects).
    #[tracing::instrument(name = "library_repo_policies", skip(self, ctx))]
    async fn policies(&self, ctx: &Context<'_>) -> Result<Vec<RepoPolicy>> {
        tracing::info!("[Repo::policies] Called for repo_id: {}", self.id);

        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

        // Build TRN for this repository
        let resource_trn = format!("trn:library:repo:{}", self.id);
        tracing::info!("[Repo::policies] Resource TRN: {}", resource_trn);

        // Use repository's organization_id as tenant_id
        // This is the operator ID that owns the repository
        let tenant_id = value_object::TenantId::new(&self.organization_id)
            .map_err(|e| {
                async_graphql::Error::new(format!(
                    "Invalid organization_id: {e}"
                ))
            })?;
        tracing::info!(
            "[Repo::policies] Tenant ID (from org_id): {}",
            tenant_id
        );

        // Use request-scoped AuthApp (carries caller's JWT) when
        // available; fall back to the schema-level one otherwise.
        let auth_app: Arc<dyn tachyon_sdk::auth::AuthApp> = ctx
            .data::<Arc<dyn tachyon_sdk::auth::AuthApp>>()
            .cloned()
            .unwrap_or_else(|_| {
                ctx.data::<Arc<SdkAuthApp>>()
                    .cloned()
                    .expect("SdkAuthApp must be in context")
            });

        let get_repo_policies = crate::usecase::GetRepoPolicies::new(
            app.user_policy_mapping_repo.clone(),
            auth_app,
        );

        // Get policies using usecase
        let policy_infos = get_repo_policies
            .execute(usecase::GetRepoPoliciesInputData {
                executor,
                multi_tenancy,
                resource_trn: &resource_trn,
                tenant_id: &tenant_id,
            })
            .await
            .map_err(|e| {
                tracing::error!("Failed to get repo policies: {:?}", e);
                e.extend()
            })?;

        // Convert to GraphQL model
        let policies: Vec<RepoPolicy> = policy_infos
            .into_iter()
            .map(|info| {
                let permission_source = match info.permission_source {
                    usecase::PermissionSource::Repo => {
                        PermissionSource::Repo
                    }
                    usecase::PermissionSource::Org => PermissionSource::Org,
                };
                RepoPolicy {
                    user_id: info.user_id,
                    role: info.role,
                    user: info.user.map(Into::into),
                    permission_source,
                }
            })
            .collect();

        Ok(policies)
    }

    /// Get members who have access to this repository
    ///
    /// Returns users with resource-based policies scoped to this repository,
    /// including org owners who have implicit owner access.
    #[tracing::instrument(name = "library_repo_members", skip(self, ctx))]
    async fn members(&self, ctx: &Context<'_>) -> Result<Vec<RepoMember>> {
        use tachyon_sdk::auth::MultiTenancyAction;

        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

        let resource_trn = format!("trn:library:repo:{}", self.id);
        let tenant_id = multi_tenancy.operator_id().ok_or_else(|| {
            async_graphql::Error::new(
                "Operator ID required for member list",
            )
        })?;

        // Use request-scoped AuthApp for the same reason as
        // policies() above.
        let auth_app: Arc<dyn tachyon_sdk::auth::AuthApp> = ctx
            .data::<Arc<dyn tachyon_sdk::auth::AuthApp>>()
            .cloned()
            .unwrap_or_else(|_| {
                ctx.data::<Arc<SdkAuthApp>>()
                    .cloned()
                    .expect("SdkAuthApp must be in context")
            });

        let get_repo_members = crate::usecase::GetRepoMembers::new(
            app.user_policy_mapping_repo.clone(),
            auth_app,
        );

        let member_infos = get_repo_members
            .execute(usecase::GetRepoMembersInputData {
                executor,
                multi_tenancy,
                resource_trn: &resource_trn,
                tenant_id: &tenant_id,
            })
            .await
            .map_err(|e| {
                tracing::error!("Failed to get repo members: {:?}", e);
                e.extend()
            })?;

        let members = member_infos
            .into_iter()
            .map(|info| {
                let permission_source = match info.permission_source {
                    usecase::PermissionSource::Repo => {
                        PermissionSource::Repo
                    }
                    usecase::PermissionSource::Org => PermissionSource::Org,
                };
                RepoMember {
                    user_id: info.user_id.to_string(),
                    policy_id: info.policy_id,
                    policy_name: info.policy_name,
                    resource_scope: info.resource_scope,
                    assigned_at: info.assigned_at,
                    user: info.user.map(Into::into),
                    permission_source,
                }
            })
            .collect();

        Ok(members)
    }
}
