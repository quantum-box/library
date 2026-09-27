use crate::domain::{Data, DataRepository, Database, DatabaseId};
use crate::usecase::database_scope::DatabaseScope;
use value_object::{RepositoryV1, TenantId};

use std::fmt::Debug;
use std::sync::Arc;

/// A cheap summary of every record in one Database.
///
/// Two revisions compare equal only when the Database holds the same set of
/// records at the same versions, so a caller that keeps a derived copy of the
/// records (a search index, an export) can tell whether it is stale without
/// loading them. Each component catches a different kind of change:
///
/// - `record_count` catches creates and deletes.
/// - `id_checksum` catches a delete paired with a create, which leaves the
///   count unchanged.
/// - `version_sum` catches patches, which bump `record_version`.
/// - `last_updated_at` catches writes that change a stored value without
///   going through a versioned patch; `updated_at` moves with any change to
///   the row.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DataRevision {
    pub record_count: u64,
    pub id_checksum: u64,
    pub version_sum: u64,
    pub last_updated_at: Option<String>,
}

impl DataRevision {
    /// Opaque token suitable for cache keys and HTTP validators.
    pub fn token(&self) -> String {
        format!(
            "{}-{:x}-{}-{}",
            self.record_count,
            self.id_checksum,
            self.version_sum,
            self.last_updated_at.as_deref().unwrap_or("none"),
        )
    }
}

#[async_trait::async_trait]
pub trait DataRevisionQuery: Debug + Send + Sync + 'static {
    async fn data_revision(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
    ) -> errors::Result<DataRevision>;
}

/// Reads a whole Database for callers that derive their own view of it.
#[async_trait::async_trait]
pub trait DataSnapshotInputPort: Debug + Send + Sync + 'static {
    /// The Database's current revision. One aggregate query; no record
    /// values are read.
    async fn revision(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
    ) -> errors::Result<DataRevision>;

    /// Every record in the Database, hydrated. Callers should check
    /// `revision` first and load only when it has moved.
    async fn load_all(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
    ) -> errors::Result<Vec<Data>>;
}

#[derive(Debug, Clone)]
pub struct DataSnapshot {
    database_repo: Arc<dyn RepositoryV1<DatabaseId, Database>>,
    data_repo: Arc<dyn DataRepository>,
    revision_query: Arc<dyn DataRevisionQuery>,
}

impl DataSnapshot {
    pub fn new(
        database_repo: Arc<dyn RepositoryV1<DatabaseId, Database>>,
        data_repo: Arc<dyn DataRepository>,
        revision_query: Arc<dyn DataRevisionQuery>,
    ) -> Arc<Self> {
        Arc::new(Self {
            database_repo,
            data_repo,
            revision_query,
        })
    }
}

#[async_trait::async_trait]
impl DataSnapshotInputPort for DataSnapshot {
    #[tracing::instrument(skip(self))]
    async fn revision(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
    ) -> errors::Result<DataRevision> {
        let database = DatabaseScope::new(tenant_id, database_id)
            .require_database(self.database_repo.as_ref())
            .await?;
        self.revision_query
            .data_revision(database.tenant_id(), database.id())
            .await
    }

    #[tracing::instrument(skip(self))]
    async fn load_all(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
    ) -> errors::Result<Vec<Data>> {
        let database = DatabaseScope::new(tenant_id, database_id)
            .require_database(self.database_repo.as_ref())
            .await?;
        let data = self
            .data_repo
            .find_all(database.id(), database.tenant_id())
            .await?;
        Ok(data.value().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_changes_with_every_component() {
        let base = DataRevision {
            record_count: 2,
            id_checksum: 0xabc,
            version_sum: 5,
            last_updated_at: Some("2026-09-26 10:00:00".into()),
        };
        let variants = [
            DataRevision {
                record_count: 3,
                ..base.clone()
            },
            DataRevision {
                id_checksum: 0xabd,
                ..base.clone()
            },
            DataRevision {
                version_sum: 6,
                ..base.clone()
            },
            DataRevision {
                last_updated_at: Some("2026-09-26 10:00:01".into()),
                ..base.clone()
            },
        ];
        for variant in variants {
            assert_ne!(base.token(), variant.token());
        }
    }

    #[test]
    fn empty_database_has_a_stable_token() {
        let empty = DataRevision {
            record_count: 0,
            id_checksum: 0,
            version_sum: 0,
            last_updated_at: None,
        };
        assert_eq!(empty.token(), "0-0-0-none");
    }
}
