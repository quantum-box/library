//! Initial sync use case for pulling all existing data from external provider.

use std::sync::Arc;

use tachyon_sdk::auth::{
    AuthApp, CheckPolicyInput, ExecutorAction, MultiTenancyAction,
};

use inbound_sync_domain::{
    SyncOperation, SyncOperationId, SyncOperationRepository,
    SyncOperationType, SyncStateRepository, WebhookEndpointId,
    WebhookEndpointRepository,
};

use super::ApiPullProcessorRegistry;

/// Input data for initial sync use case.
#[derive(Debug)]
pub struct InitialSyncInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub endpoint_id: WebhookEndpointId,
}

/// Output data for initial sync use case.
pub struct InitialSyncOutputData {
    pub operation_id: SyncOperationId,
}

/// Initial sync use case.
///
/// Performs initial full synchronization of all existing data from an external
/// provider when a new connection is established.
pub struct InitialSync {
    auth_app: Arc<dyn AuthApp>,
    endpoint_repo: Arc<dyn WebhookEndpointRepository>,
    operation_repo: Arc<dyn SyncOperationRepository>,
    sync_state_repo: Arc<dyn SyncStateRepository>,
    processor_registry: Arc<ApiPullProcessorRegistry>,
}

impl InitialSync {
    pub fn new(
        auth_app: Arc<dyn AuthApp>,
        endpoint_repo: Arc<dyn WebhookEndpointRepository>,
        operation_repo: Arc<dyn SyncOperationRepository>,
        sync_state_repo: Arc<dyn SyncStateRepository>,
        processor_registry: Arc<ApiPullProcessorRegistry>,
    ) -> Self {
        Self {
            auth_app,
            endpoint_repo,
            operation_repo,
            sync_state_repo,
            processor_registry,
        }
    }

    pub async fn execute(
        &self,
        input: InitialSyncInputData<'_>,
    ) -> errors::Result<InitialSyncOutputData> {
        // Policy check
        self.auth_app
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "inbound_sync:InitialSync",
            })
            .await?;

        // Get endpoint
        let endpoint = self
            .endpoint_repo
            .find_by_id(&input.endpoint_id)
            .await?
            .ok_or_else(|| errors::Error::not_found("Webhook endpoint"))?;

        // Create sync operation
        let mut operation = SyncOperation::create(
            input.endpoint_id.clone(),
            SyncOperationType::InitialSync,
        );
        self.operation_repo.save(&operation).await?;

        // Get processor for provider
        let processor = self
            .processor_registry
            .get(*endpoint.provider())
            .ok_or_else(|| {
            errors::Error::invalid(format!(
                "No API pull processor for provider: {}",
                endpoint.provider()
            ))
        })?;

        let operation_id = operation.id().clone();
        let operation_id_for_spawn = operation_id.clone();
        let endpoint_clone = endpoint.clone();
        let operation_repo_clone = self.operation_repo.clone();
        let sync_state_repo_clone = self.sync_state_repo.clone();

        // Spawn background task for processing
        tokio::spawn(async move {
            let operation_id = operation_id_for_spawn;
            operation.mark_running();
            let _ = operation_repo_clone.save(&operation).await;

            match processor
                .pull_all(
                    &endpoint_clone,
                    &sync_state_repo_clone,
                    &mut operation,
                )
                .await
            {
                Ok(stats) => {
                    tracing::info!(
                        operation_id = %operation_id,
                        created = stats.created,
                        updated = stats.updated,
                        deleted = stats.deleted,
                        skipped = stats.skipped,
                        "Initial sync completed successfully"
                    );
                    operation.mark_completed(stats);
                    let _ = operation_repo_clone.save(&operation).await;
                }
                Err(e) => {
                    tracing::error!(
                        operation_id = %operation_id,
                        error = %e,
                        "Initial sync failed"
                    );
                    operation.mark_failed(e.to_string());
                    let _ = operation_repo_clone.save(&operation).await;
                }
            }
        });

        Ok(InitialSyncOutputData { operation_id })
    }
}
