//! Automatic GitHub writeback decorators for Data mutations.
//!
//! Wraps the `AddDataInputPort` / `UpdateDataInputPort` /
//! `UpsertDataInputPort` usecases so that
//! saving a Data item whose `ext_github` property has `enabled=true`
//! pushes the composed markdown back to the configured GitHub
//! repository. The push is best-effort: failures are logged and never
//! fail the user's save.
//!
//! Loop prevention:
//! - Inbound webhook upserts write through `database_manager` directly
//!   (`LibraryDataRepositoryImpl`), never through these ports, so an
//!   inbound sync can never trigger an outbound push.
//! - After a successful push, the resulting commit SHA is recorded in
//!   the inbound `SyncState` (`external_version`) so the webhook echo
//!   of our own push is skipped by `GitHubEventProcessor`.

use std::sync::Arc;

use database_manager::domain::{Data, Property};
use inbound_sync_domain::{
    Provider, ProviderConfig, SyncDirection, SyncState,
    SyncStateRepository, WebhookEndpointRepository,
};
use outbound_sync::{
    SyncDataInputData, SyncDataInputPort, SyncPayload, SyncTarget,
};
use tachyon_sdk::auth::{ExecutorAction, MultiTenancyAction};

use crate::usecase::ext_github_meta::ExtGithubMeta;
use crate::usecase::{
    AddDataInputData, AddDataInputPort, UpdateDataInputData,
    UpdateDataInputPort, UpsertDataInputData, UpsertDataInputPort,
};
use database_manager::usecase::UpsertOutcome;

/// Shared writeback logic used by the Add/Update decorators.
pub struct GithubWritebackDispatch {
    sync_data: Arc<dyn SyncDataInputPort>,
    webhook_endpoint_repo: Arc<dyn WebhookEndpointRepository>,
    sync_state_repo: Arc<dyn SyncStateRepository>,
}

impl std::fmt::Debug for GithubWritebackDispatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GithubWritebackDispatch")
            .finish_non_exhaustive()
    }
}

impl GithubWritebackDispatch {
    pub fn new(
        sync_data: Arc<dyn SyncDataInputPort>,
        webhook_endpoint_repo: Arc<dyn WebhookEndpointRepository>,
        sync_state_repo: Arc<dyn SyncStateRepository>,
    ) -> Arc<Self> {
        Arc::new(Self {
            sync_data,
            webhook_endpoint_repo,
            sync_state_repo,
        })
    }

    /// Extract sync-enabled `ext_github` metadata from a saved Data item.
    fn sync_enabled_meta(
        data: &Data,
        properties: &[Property],
    ) -> Option<ExtGithubMeta> {
        let prop = properties.iter().find(|p| p.name() == "ext_github")?;
        let value = data.get_property_data(prop.id())?.string_value();
        let meta = ExtGithubMeta::parse(&value)?;
        if !meta.enabled {
            return None;
        }
        Some(meta)
    }

    /// Push the Data item's markdown to GitHub. Best-effort: errors are
    /// logged, never propagated to the caller's save.
    async fn dispatch(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        data: &Data,
        properties: &[Property],
    ) {
        let Some(meta) = Self::sync_enabled_meta(data, properties) else {
            return;
        };

        let markdown = crate::usecase::markdown_composer::compose_markdown(
            data, properties,
        );
        let message =
            format!("chore(library): sync {}", data.name().as_str());

        let result = self
            .sync_data
            .execute(&SyncDataInputData {
                executor,
                multi_tenancy,
                data_id: data.id().to_string(),
                provider: "github".to_string(),
                target: SyncTarget::git_with_branch(
                    &meta.repo,
                    &meta.path,
                    meta.git_ref.clone(),
                ),
                payload: SyncPayload::markdown_with_message(
                    &markdown, &message,
                ),
                dry_run: false,
            })
            .await;

        match result {
            Ok(sync_result) => {
                tracing::info!(
                    data_id = %data.id(),
                    repo = %meta.repo,
                    path = %meta.path,
                    status = ?sync_result.status,
                    "Auto-synced data to GitHub"
                );
                if let Some(commit_sha) = sync_result.result_id {
                    self.record_outbound_commit(data, &meta, &commit_sha)
                        .await;
                }
            }
            Err(e) => {
                tracing::warn!(
                    data_id = %data.id(),
                    repo = %meta.repo,
                    path = %meta.path,
                    error = %e,
                    "Failed to auto-sync data to GitHub"
                );
            }
        }
    }

    /// Record the pushed commit SHA in the inbound sync state so the
    /// webhook echo of our own push is skipped. Best-effort.
    async fn record_outbound_commit(
        &self,
        data: &Data,
        meta: &ExtGithubMeta,
        commit_sha: &str,
    ) {
        let endpoints = match self
            .webhook_endpoint_repo
            .find_by_tenant_and_provider(data.tenant_id(), Provider::Github)
            .await
        {
            Ok(endpoints) => endpoints,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to look up GitHub webhook endpoints for echo suppression"
                );
                return;
            }
        };

        let external_id = format!("{}:{}", meta.repo, meta.path);

        for endpoint in endpoints {
            let matches_repo = matches!(
                endpoint.config(),
                ProviderConfig::Github { repository, .. }
                    if repository == &meta.repo
            );
            if !matches_repo {
                continue;
            }

            let mut state = match self
                .sync_state_repo
                .find_by_external_id(endpoint.id(), &external_id)
                .await
            {
                Ok(Some(state)) => state,
                Ok(None) => SyncState::create(
                    endpoint.id().clone(),
                    data.id().to_string(),
                    &external_id,
                    SyncDirection::Both,
                ),
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Failed to load sync state for echo suppression"
                    );
                    continue;
                }
            };

            state.update_outbound(
                Some(commit_sha.to_string()),
                Some(data.record_version().to_string()),
            );

            if let Err(e) = self.sync_state_repo.save(&state).await {
                tracing::warn!(
                    error = %e,
                    "Failed to save sync state for echo suppression"
                );
            }
        }
    }
}

/// `AddDataInputPort` decorator that auto-pushes sync-enabled Data.
pub struct AddDataWithGithubWriteback {
    inner: Arc<dyn AddDataInputPort>,
    writeback: Arc<GithubWritebackDispatch>,
}

impl AddDataWithGithubWriteback {
    pub fn new(
        inner: Arc<dyn AddDataInputPort>,
        writeback: Arc<GithubWritebackDispatch>,
    ) -> Arc<Self> {
        Arc::new(Self { inner, writeback })
    }
}

impl std::fmt::Debug for AddDataWithGithubWriteback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AddDataWithGithubWriteback")
            .finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
impl AddDataInputPort for AddDataWithGithubWriteback {
    async fn execute<'a>(
        &self,
        input: AddDataInputData<'a>,
    ) -> errors::Result<(Data, Vec<Property>)> {
        let executor = input.executor;
        let multi_tenancy = input.multi_tenancy;
        let (data, properties) = self.inner.execute(input).await?;
        self.writeback
            .dispatch(executor, multi_tenancy, &data, &properties)
            .await;
        Ok((data, properties))
    }
}

/// `UpdateDataInputPort` decorator that auto-pushes sync-enabled Data.
pub struct UpdateDataWithGithubWriteback {
    inner: Arc<dyn UpdateDataInputPort>,
    writeback: Arc<GithubWritebackDispatch>,
}

impl UpdateDataWithGithubWriteback {
    pub fn new(
        inner: Arc<dyn UpdateDataInputPort>,
        writeback: Arc<GithubWritebackDispatch>,
    ) -> Arc<Self> {
        Arc::new(Self { inner, writeback })
    }
}

impl std::fmt::Debug for UpdateDataWithGithubWriteback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpdateDataWithGithubWriteback")
            .finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
impl UpdateDataInputPort for UpdateDataWithGithubWriteback {
    async fn execute<'a>(
        &self,
        input: UpdateDataInputData<'a>,
    ) -> errors::Result<(Data, Vec<Property>)> {
        let executor = input.executor;
        let multi_tenancy = input.multi_tenancy;
        let (data, properties) = self.inner.execute(input).await?;
        self.writeback
            .dispatch(executor, multi_tenancy, &data, &properties)
            .await;
        Ok((data, properties))
    }
}

/// `UpsertDataInputPort` decorator that auto-pushes sync-enabled Data.
///
/// An upsert reaches GitHub whichever branch it took: a record created at a
/// client-chosen id is as much a new document as one created through
/// `AddData`.
pub struct UpsertDataWithGithubWriteback {
    inner: Arc<dyn UpsertDataInputPort>,
    writeback: Arc<GithubWritebackDispatch>,
}

impl UpsertDataWithGithubWriteback {
    pub fn new(
        inner: Arc<dyn UpsertDataInputPort>,
        writeback: Arc<GithubWritebackDispatch>,
    ) -> Arc<Self> {
        Arc::new(Self { inner, writeback })
    }
}

impl std::fmt::Debug for UpsertDataWithGithubWriteback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpsertDataWithGithubWriteback")
            .finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
impl UpsertDataInputPort for UpsertDataWithGithubWriteback {
    async fn execute<'a>(
        &self,
        input: UpsertDataInputData<'a>,
    ) -> errors::Result<(Data, Vec<Property>, UpsertOutcome)> {
        let executor = input.executor;
        let multi_tenancy = input.multi_tenancy;
        let (data, properties, outcome) = self.inner.execute(input).await?;
        self.writeback
            .dispatch(executor, multi_tenancy, &data, &properties)
            .await;
        Ok((data, properties, outcome))
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use database_manager::domain::{
        Data, DataId, DatabaseId, Property, PropertyData, PropertyId,
        PropertyType,
    };
    use value_object::TenantId;

    use super::GithubWritebackDispatch;

    struct Fixture {
        tenant_id: TenantId,
        database_id: DatabaseId,
        properties: Vec<Property>,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                tenant_id: TenantId::default(),
                database_id: DatabaseId::default(),
                properties: vec![],
            }
        }

        fn property(
            &mut self,
            name: &str,
            property_type: PropertyType,
        ) -> Property {
            let property = Property::new(
                &PropertyId::default(),
                &self.tenant_id,
                &self.database_id,
                name,
                &property_type,
                false,
                self.properties.len() as u32,
            );
            self.properties.push(property.clone());
            property
        }

        fn data(
            &self,
            name: &str,
            property_data: Vec<PropertyData>,
        ) -> Data {
            Data::new(
                &DataId::default(),
                &self.tenant_id,
                &self.database_id,
                name,
                property_data,
                Utc::now(),
                Utc::now(),
            )
            .expect("fixture data should be valid")
        }
    }

    #[test]
    fn skips_data_without_ext_github_property() {
        let mut fixture = Fixture::new();
        let slug = fixture.property("slug", PropertyType::String);
        let data = fixture.data(
            "Plain page",
            vec![PropertyData::new(&slug, "plain".to_string()).unwrap()],
        );

        assert!(GithubWritebackDispatch::sync_enabled_meta(
            &data,
            &fixture.properties
        )
        .is_none());
    }

    #[test]
    fn skips_ext_github_with_sync_disabled() {
        let mut fixture = Fixture::new();
        let ext_github =
            fixture.property("ext_github", PropertyType::String);
        let data = fixture.data(
            "Imported page",
            vec![PropertyData::new(
                &ext_github,
                r#"{"repo":"owner/repo","path":"docs/a.md","enabled":false}"#
                    .to_string(),
            )
            .unwrap()],
        );

        assert!(GithubWritebackDispatch::sync_enabled_meta(
            &data,
            &fixture.properties
        )
        .is_none());
    }

    #[test]
    fn extracts_sync_enabled_ext_github_meta_with_branch() {
        let mut fixture = Fixture::new();
        let ext_github =
            fixture.property("ext_github", PropertyType::String);
        let data = fixture.data(
            "Synced page",
            vec![PropertyData::new(
                &ext_github,
                r#"{"repo":"owner/repo","path":"docs/a.md","ref":"develop","enabled":true,"sync_to_github":true}"#
                    .to_string(),
            )
            .unwrap()],
        );

        let meta = GithubWritebackDispatch::sync_enabled_meta(
            &data,
            &fixture.properties,
        )
        .expect("sync-enabled metadata should be extracted");

        assert_eq!(meta.repo, "owner/repo");
        assert_eq!(meta.path, "docs/a.md");
        assert_eq!(meta.git_ref, "develop");
    }
}
