//! Sync data use case.
//!
//! This use case handles synchronizing data to external providers.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use integration_domain::{OAuthProvider, OAuthTokenRepository};
use tachyon_sdk::auth::{AuthApp, ExecutorAction, MultiTenancyAction};
use value_object::TenantId;

use outbound_sync_domain::{
    DataId, RemoteData, SyncAuth, SyncConfig, SyncConfigRepository,
    SyncPayload, SyncStatus, SyncTarget,
};

use crate::providers::SyncProviderRegistry;

/// Input data for sync operation
pub struct SyncDataInputData<'a> {
    /// Executor (user or service account)
    pub executor: &'a dyn ExecutorAction,
    /// Multi-tenancy context
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    /// Data ID to sync
    pub data_id: String,
    /// Provider name (github, gitlab, s3, etc.)
    pub provider: String,
    /// Sync target specification
    pub target: SyncTarget,
    /// Payload to sync
    pub payload: SyncPayload,
    /// If true, only calculate diff without actually syncing
    pub dry_run: bool,
}

/// Input data for delete operation
pub struct DeleteDataInputData<'a> {
    /// Executor (user or service account)
    pub executor: &'a dyn ExecutorAction,
    /// Multi-tenancy context
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    /// Provider name (github, gitlab, s3, etc.)
    pub provider: String,
    /// Sync target specification (path to delete)
    pub target: SyncTarget,
    /// Commit message for deletion
    pub message: Option<String>,
}

/// Result of sync operation
#[derive(Debug, Clone)]
pub struct SyncDataResult {
    /// Sync status
    pub status: SyncStatus,
    /// Result ID (commit SHA, record ID, etc.)
    pub result_id: Option<String>,
    /// URL to the synced resource
    pub url: Option<String>,
    /// Diff preview (for dry-run)
    pub diff: Option<String>,
}

/// Input port for sync data use case
#[async_trait]
pub trait SyncDataInputPort: Send + Sync {
    /// Execute the sync operation
    async fn execute<'a>(
        &self,
        input: &SyncDataInputData<'a>,
    ) -> errors::Result<SyncDataResult>;

    /// Delete data from the remote provider
    async fn delete<'a>(
        &self,
        input: &DeleteDataInputData<'a>,
    ) -> errors::Result<SyncDataResult>;
}

/// Provider token normalized across caller-authorized and background reads.
#[derive(Debug, Clone)]
pub struct SyncOAuthToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl SyncOAuthToken {
    fn is_expired(&self) -> bool {
        self.expires_at
            .is_some_and(|expires_at| expires_at <= Utc::now())
    }
}

/// Resolves a provider credential for one outbound attempt.
#[async_trait]
pub trait SyncOAuthTokenProvider: Send + Sync + std::fmt::Debug {
    async fn get_token(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        provider: &str,
    ) -> errors::Result<Option<SyncOAuthToken>>;
}

/// Caller-authorized token lookup retained by the public sync use case.
#[derive(Debug, Clone)]
struct AuthAppSyncOAuthTokenProvider {
    auth_app: Arc<dyn AuthApp>,
}

#[async_trait]
impl SyncOAuthTokenProvider for AuthAppSyncOAuthTokenProvider {
    async fn get_token(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        provider: &str,
    ) -> errors::Result<Option<SyncOAuthToken>> {
        Ok(self
            .auth_app
            .get_oauth_token_by_provider(
                &tachyon_sdk::auth::GetOAuthTokenByProviderInput {
                    executor,
                    multi_tenancy,
                    provider,
                },
            )
            .await?
            .map(|token| SyncOAuthToken {
                access_token: token.access_token,
                refresh_token: token.refresh_token,
                expires_at: Some(token.expires_at),
            }))
    }
}

/// Server-side token lookup for durable/background outbound work.
#[derive(Debug, Clone)]
pub struct RepositorySyncOAuthTokenProvider {
    repository: Arc<dyn OAuthTokenRepository>,
}

impl RepositorySyncOAuthTokenProvider {
    pub fn new(repository: Arc<dyn OAuthTokenRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl SyncOAuthTokenProvider for RepositorySyncOAuthTokenProvider {
    async fn get_token(
        &self,
        _executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        provider: &str,
    ) -> errors::Result<Option<SyncOAuthToken>> {
        let tenant_id: TenantId =
            multi_tenancy.get_operator_id()?.to_string().parse()?;
        let provider = provider.parse::<OAuthProvider>().map_err(|_| {
            errors::Error::invalid(format!(
                "Unsupported OAuth provider: {provider}"
            ))
        })?;
        Ok(self
            .repository
            .find_by_tenant_and_provider(&tenant_id, provider)
            .await?
            .map(|token| SyncOAuthToken {
                access_token: token.access_token,
                refresh_token: token.refresh_token,
                expires_at: token.expires_at,
            }))
    }
}

/// Sync data use case implementation
pub struct SyncData {
    token_provider: Arc<dyn SyncOAuthTokenProvider>,
    sync_config_repo: Arc<dyn SyncConfigRepository>,
    provider_registry: Arc<SyncProviderRegistry>,
}

impl SyncData {
    pub fn new(
        auth_app: Arc<dyn AuthApp>,
        sync_config_repo: Arc<dyn SyncConfigRepository>,
        provider_registry: Arc<SyncProviderRegistry>,
    ) -> Self {
        Self::new_with_token_provider(
            Arc::new(AuthAppSyncOAuthTokenProvider { auth_app }),
            sync_config_repo,
            provider_registry,
        )
    }

    pub fn new_with_token_provider(
        token_provider: Arc<dyn SyncOAuthTokenProvider>,
        sync_config_repo: Arc<dyn SyncConfigRepository>,
        provider_registry: Arc<SyncProviderRegistry>,
    ) -> Self {
        Self {
            token_provider,
            sync_config_repo,
            provider_registry,
        }
    }
}

impl std::fmt::Debug for SyncData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyncData")
            .field("token_provider", &self.token_provider)
            .field("sync_config_repo", &self.sync_config_repo)
            .field("provider_registry", &self.provider_registry)
            .finish()
    }
}

#[async_trait]
impl SyncDataInputPort for SyncData {
    #[tracing::instrument(skip(self, input))]
    async fn execute<'a>(
        &self,
        input: &SyncDataInputData<'a>,
    ) -> errors::Result<SyncDataResult> {
        // 1. Get provider
        let provider = self
            .provider_registry
            .get(&input.provider)
            .ok_or_else(|| {
                errors::Error::not_found(format!(
                    "Sync provider '{}' not found",
                    input.provider
                ))
            })?;

        tracing::debug!(
            provider = %input.provider,
            provider_type = ?provider.provider_type(),
            "Using sync provider"
        );

        // 2. Get an OAuth token through the use-case-specific provider.
        let operator_id = input.multi_tenancy.get_operator_id()?;
        let token = self
            .token_provider
            .get_token(input.executor, input.multi_tenancy, &input.provider)
            .await?
            .ok_or_else(|| {
                errors::Error::not_found(format!(
                    "{} is not connected for this tenant",
                    input.provider
                ))
            })?;

        // Check if token is expired
        if token.is_expired() {
            return Err(errors::Error::unauthorized(format!(
                "{} token has expired. Please reconnect.",
                input.provider
            )));
        }

        let auth = SyncAuth {
            access_token: token.access_token.clone(),
            refresh_token: token.refresh_token.clone(),
            api_key: None,
        };

        // 3. Handle dry-run
        if input.dry_run {
            let existing = provider.get_data(&auth, &input.target).await?;
            let diff =
                calculate_diff(existing.as_ref(), &input.payload.content);

            tracing::debug!(
                has_existing = existing.is_some(),
                "Dry-run completed"
            );

            return Ok(SyncDataResult {
                status: SyncStatus::Pending,
                result_id: None,
                url: None,
                diff: Some(diff),
            });
        }

        // 4. Execute sync
        let result = provider
            .put_data(&auth, &input.target, &input.payload)
            .await?;

        if !result.success {
            tracing::error!("Sync failed for data_id={}", input.data_id);
            return Ok(SyncDataResult {
                status: SyncStatus::Failed(
                    "Sync operation failed".to_string(),
                ),
                result_id: None,
                url: None,
                diff: None,
            });
        }

        tracing::info!(
            data_id = %input.data_id,
            result_id = ?result.result_id,
            "Sync completed successfully"
        );

        // 5. Save/update sync config
        let data_id = DataId::new(&input.data_id);

        // Try to find existing config or create new one
        let mut config = self
            .sync_config_repo
            .find_by_data_id(&data_id)
            .await?
            .unwrap_or_else(|| {
                SyncConfig::create(
                    operator_id.clone(),
                    data_id,
                    &input.provider,
                    input.target.clone(),
                )
            });

        // Update config with sync result
        if let Some(ref result_id) = result.result_id {
            config.mark_synced(result_id);
        }
        config.update_target(input.target.clone());

        self.sync_config_repo.save(&config).await?;

        Ok(SyncDataResult {
            status: SyncStatus::Synced,
            result_id: result.result_id,
            url: result.url,
            diff: None,
        })
    }

    #[tracing::instrument(skip(self, input))]
    async fn delete<'a>(
        &self,
        input: &DeleteDataInputData<'a>,
    ) -> errors::Result<SyncDataResult> {
        // 1. Get provider
        let provider = self
            .provider_registry
            .get(&input.provider)
            .ok_or_else(|| {
                errors::Error::not_found(format!(
                    "Sync provider '{}' not found",
                    input.provider
                ))
            })?;

        tracing::debug!(
            provider = %input.provider,
            provider_type = ?provider.provider_type(),
            "Using sync provider for deletion"
        );

        // 2. Get an OAuth token through the use-case-specific provider.
        let token = self
            .token_provider
            .get_token(input.executor, input.multi_tenancy, &input.provider)
            .await?
            .ok_or_else(|| {
                errors::Error::not_found(format!(
                    "{} is not connected for this tenant",
                    input.provider
                ))
            })?;

        // Check if token is expired
        if token.is_expired() {
            return Err(errors::Error::unauthorized(format!(
                "{} token has expired. Please reconnect.",
                input.provider
            )));
        }

        let auth = SyncAuth {
            access_token: token.access_token.clone(),
            refresh_token: token.refresh_token.clone(),
            api_key: None,
        };

        // 3. Execute deletion
        let result = provider.delete_data(&auth, &input.target).await?;

        if !result.success {
            tracing::error!(
                target = ?input.target,
                "Delete operation failed"
            );
            return Ok(SyncDataResult {
                status: SyncStatus::Failed(
                    "Delete operation failed".to_string(),
                ),
                result_id: None,
                url: None,
                diff: None,
            });
        }

        tracing::info!(
            target = ?input.target,
            result_id = ?result.result_id,
            "Delete completed successfully"
        );

        Ok(SyncDataResult {
            status: SyncStatus::Synced,
            result_id: result.result_id,
            url: result.url,
            diff: None,
        })
    }
}

/// Calculate diff between existing content and new content
fn calculate_diff(
    existing: Option<&RemoteData>,
    new_content: &str,
) -> String {
    match existing {
        Some(remote) => {
            // Use similar crate for unified diff
            let diff = similar::TextDiff::from_lines(
                remote.content.as_str(),
                new_content,
            );
            diff.unified_diff()
                .context_radius(3)
                .header("existing", "new")
                .to_string()
        }
        None => {
            // New file - show all as additions
            let lines: Vec<&str> = new_content.lines().collect();
            let mut result =
                String::from("--- /dev/null\n+++ new\n@@ -0,0 +1,");
            result.push_str(&lines.len().to_string());
            result.push_str(" @@\n");
            for line in lines {
                result.push('+');
                result.push_str(line);
                result.push('\n');
            }
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use integration_domain::{OAuthTokenResponse, StoredOAuthToken};
    use tachyon_sdk::auth::{Executor, MultiTenancy, OperatorId};

    #[derive(Debug)]
    struct StaticTokenRepository {
        token: StoredOAuthToken,
    }

    #[async_trait]
    impl OAuthTokenRepository for StaticTokenRepository {
        async fn save(
            &self,
            _token: &StoredOAuthToken,
        ) -> errors::Result<()> {
            Ok(())
        }

        async fn find_by_tenant_and_provider(
            &self,
            tenant_id: &TenantId,
            provider: OAuthProvider,
        ) -> errors::Result<Option<StoredOAuthToken>> {
            Ok((self.token.tenant_id == *tenant_id
                && self.token.provider == provider)
                .then(|| self.token.clone()))
        }

        async fn delete(
            &self,
            _tenant_id: &TenantId,
            _provider: OAuthProvider,
        ) -> errors::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_calculate_diff_new_file() {
        let content = "# Hello\n\nWorld!";
        let diff = calculate_diff(None, content);

        assert!(diff.contains("--- /dev/null"));
        assert!(diff.contains("+++ new"));
        assert!(diff.contains("+# Hello"));
        assert!(diff.contains("+World!"));
    }

    #[test]
    fn test_calculate_diff_existing_file() {
        let existing = RemoteData {
            id: "sha123".to_string(),
            content: "# Hello\n\nWorld!".to_string(),
            content_type: Some("text/markdown".to_string()),
            size: Some(15),
            updated_at: None,
        };

        let new_content = "# Hello\n\nNew World!";
        let diff = calculate_diff(Some(&existing), new_content);

        assert!(diff.contains("--- existing"));
        assert!(diff.contains("+++ new"));
        assert!(diff.contains("-World!"));
        assert!(diff.contains("+New World!"));
    }

    #[tokio::test]
    async fn repository_provider_reads_background_broker_token() {
        let tenant_id = TenantId::default();
        let token = StoredOAuthToken::from_response(
            "oauth-test".to_string(),
            tenant_id.clone(),
            OAuthProvider::Github,
            OAuthTokenResponse {
                access_token: "github-access".to_string(),
                refresh_token: Some("github-refresh".to_string()),
                token_type: "Bearer".to_string(),
                expires_in: None,
                scope: None,
            },
        );
        let provider = RepositorySyncOAuthTokenProvider::new(Arc::new(
            StaticTokenRepository { token },
        ));
        let operator_id: OperatorId =
            tenant_id.to_string().parse().unwrap();
        let multi_tenancy = MultiTenancy::new_operator(operator_id);

        let resolved = provider
            .get_token(&Executor::SystemUser, &multi_tenancy, "github")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(resolved.access_token, "github-access");
        assert_eq!(
            resolved.refresh_token.as_deref(),
            Some("github-refresh")
        );
        assert!(resolved.expires_at.is_none());
    }
}
