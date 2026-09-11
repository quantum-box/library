//! SQLx persistence adapters for provider-neutral external synchronization.

use std::{str::FromStr, sync::Arc};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use integration_domain::{
    ExternalDeletePolicy, ExternalObjectLink, ExternalObjectLinkRepository,
    ExternalScope, ExternalSyncBinding, ExternalSyncBindingId,
    ExternalSyncBindingRepository, ExternalSyncBindingStatus,
    ExternalSyncPolicy, LibraryDataId, LibraryRepoId, OAuthProvider,
};
use sha2::{Digest, Sha256};
use sqlx::MySqlPool;
use value_object::TenantId;

#[derive(Debug, Clone)]
pub struct SqlxExternalSyncBindingRepository {
    pool: Arc<MySqlPool>,
}

impl SqlxExternalSyncBindingRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ExternalSyncBindingRow {
    id: String,
    tenant_id: String,
    repo_id: String,
    provider: String,
    connection_id: String,
    external_scope: serde_json::Value,
    external_scope_hash: String,
    object_type: String,
    mapping: serde_json::Value,
    inbound_policy: String,
    outbound_policy: String,
    delete_policy: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<ExternalSyncBindingRow> for ExternalSyncBinding {
    type Error = errors::Error;

    fn try_from(row: ExternalSyncBindingRow) -> Result<Self, Self::Error> {
        ExternalSyncBinding::new(
            ExternalSyncBindingId::parse(row.id)?,
            row.tenant_id.parse().map_err(
                |error: errors::ParseIdError| {
                    errors::Error::invalid(error.to_string())
                },
            )?,
            LibraryRepoId::parse(row.repo_id)?,
            OAuthProvider::from_str(&row.provider).map_err(|error| {
                errors::Error::invalid(error.to_string())
            })?,
            integration_domain::ConnectionId::new(row.connection_id),
            ExternalScope::from_stored(
                row.external_scope,
                row.external_scope_hash,
            )?,
            row.object_type,
            row.mapping,
            ExternalSyncPolicy::from_str(&row.inbound_policy)?,
            ExternalSyncPolicy::from_str(&row.outbound_policy)?,
            ExternalDeletePolicy::from_str(&row.delete_policy)?,
            ExternalSyncBindingStatus::from_str(&row.status)?,
            row.created_at,
            row.updated_at,
        )
    }
}

const BINDING_COLUMNS: &str = r#"
    id, tenant_id, repo_id, provider, connection_id,
    external_scope, external_scope_hash, object_type, mapping,
    inbound_policy, outbound_policy, delete_policy, status,
    created_at, updated_at
"#;

pub(super) fn persistence_error(error: sqlx::Error) -> errors::Error {
    match &error {
        sqlx::Error::Database(database_error)
            if database_error.is_unique_violation() =>
        {
            errors::Error::conflict(
                "an external synchronization mapping already exists",
            )
        }
        _ => errors::Error::internal_server_error(error.to_string()),
    }
}

#[async_trait]
impl ExternalSyncBindingRepository for SqlxExternalSyncBindingRepository {
    async fn save(
        &self,
        binding: &ExternalSyncBinding,
    ) -> errors::Result<()> {
        let mut transaction =
            self.pool.begin().await.map_err(persistence_error)?;
        let owns_scope: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM repos AS repo
                JOIN integration_connections AS connection
                  ON connection.id = ?
                WHERE repo.id = ?
                  AND repo.org_id = ?
                  AND connection.tenant_id = ?
                  AND connection.provider = ?
            )
            "#,
        )
        .bind(binding.connection_id().as_str())
        .bind(binding.library_repo_id().as_str())
        .bind(binding.tenant_id().to_string())
        .bind(binding.tenant_id().to_string())
        .bind(binding.provider().to_string())
        .fetch_one(&mut *transaction)
        .await
        .map_err(persistence_error)?;

        if !owns_scope {
            return Err(errors::Error::not_found(
                "repo and provider connection were not found in the binding tenant",
            ));
        }

        // Lock the canonical identity (or its unique-index gap) so two
        // concurrent creators cannot both claim different binding IDs for the
        // same external scope.
        let existing_id: Option<String> = sqlx::query_scalar(
            r#"
            SELECT id
            FROM external_sync_bindings
            WHERE tenant_id = ? AND repo_id = ? AND provider = ?
              AND external_scope_hash = ?
            FOR UPDATE
            "#,
        )
        .bind(binding.tenant_id().to_string())
        .bind(binding.library_repo_id().as_str())
        .bind(binding.provider().to_string())
        .bind(binding.external_scope().identity_hash())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(persistence_error)?;
        if existing_id
            .as_deref()
            .is_some_and(|id| id != binding.id().as_str())
        {
            return Err(errors::Error::conflict(
                "the external scope is already owned by another binding",
            ));
        }

        sqlx::query(
            r#"
            INSERT INTO external_sync_bindings (
                id, tenant_id, repo_id, provider, connection_id,
                external_scope, external_scope_hash, object_type, mapping,
                inbound_policy, outbound_policy, delete_policy, status,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                connection_id = VALUES(connection_id),
                external_scope = VALUES(external_scope),
                external_scope_hash = VALUES(external_scope_hash),
                object_type = VALUES(object_type),
                mapping = VALUES(mapping),
                inbound_policy = VALUES(inbound_policy),
                outbound_policy = VALUES(outbound_policy),
                delete_policy = VALUES(delete_policy),
                status = VALUES(status),
                updated_at = VALUES(updated_at)
            "#,
        )
        .bind(binding.id().as_str())
        .bind(binding.tenant_id().to_string())
        .bind(binding.library_repo_id().as_str())
        .bind(binding.provider().to_string())
        .bind(binding.connection_id().as_str())
        .bind(binding.external_scope().value())
        .bind(binding.external_scope().identity_hash())
        .bind(binding.object_type())
        .bind(binding.mapping())
        .bind(binding.inbound_policy().as_str())
        .bind(binding.outbound_policy().as_str())
        .bind(binding.delete_policy().as_str())
        .bind(binding.status().as_str())
        .bind(binding.created_at())
        .bind(binding.updated_at())
        .execute(&mut *transaction)
        .await
        .map_err(persistence_error)?;

        let canonical_id: String = sqlx::query_scalar(
            "SELECT id FROM external_sync_bindings \
             WHERE tenant_id = ? AND repo_id = ? AND provider = ? \
               AND external_scope_hash = ?",
        )
        .bind(binding.tenant_id().to_string())
        .bind(binding.library_repo_id().as_str())
        .bind(binding.provider().to_string())
        .bind(binding.external_scope().identity_hash())
        .fetch_one(&mut *transaction)
        .await
        .map_err(persistence_error)?;
        if canonical_id != binding.id().as_str() {
            return Err(errors::Error::conflict(
                "the external scope is already owned by another binding",
            ));
        }

        transaction.commit().await.map_err(persistence_error)?;

        Ok(())
    }

    async fn find_by_id(
        &self,
        tenant_id: &TenantId,
        id: &ExternalSyncBindingId,
    ) -> errors::Result<Option<ExternalSyncBinding>> {
        let query = format!(
            "SELECT {BINDING_COLUMNS} FROM external_sync_bindings \
             WHERE tenant_id = ? AND id = ?"
        );
        let row: Option<ExternalSyncBindingRow> = sqlx::query_as(&query)
            .bind(tenant_id.to_string())
            .bind(id.as_str())
            .fetch_optional(self.pool.as_ref())
            .await
            .map_err(persistence_error)?;
        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_scope(
        &self,
        tenant_id: &TenantId,
        repo_id: &LibraryRepoId,
        provider: OAuthProvider,
        external_scope: &ExternalScope,
    ) -> errors::Result<Option<ExternalSyncBinding>> {
        let query = format!(
            "SELECT {BINDING_COLUMNS} FROM external_sync_bindings \
             WHERE tenant_id = ? AND repo_id = ? AND provider = ? \
               AND external_scope_hash = ?"
        );
        let row: Option<ExternalSyncBindingRow> = sqlx::query_as(&query)
            .bind(tenant_id.to_string())
            .bind(repo_id.as_str())
            .bind(provider.to_string())
            .bind(external_scope.identity_hash())
            .fetch_optional(self.pool.as_ref())
            .await
            .map_err(persistence_error)?;
        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_repo(
        &self,
        tenant_id: &TenantId,
        repo_id: &LibraryRepoId,
    ) -> errors::Result<Vec<ExternalSyncBinding>> {
        let query = format!(
            "SELECT {BINDING_COLUMNS} FROM external_sync_bindings \
             WHERE tenant_id = ? AND repo_id = ? ORDER BY created_at, id"
        );
        let rows: Vec<ExternalSyncBindingRow> = sqlx::query_as(&query)
            .bind(tenant_id.to_string())
            .bind(repo_id.as_str())
            .fetch_all(self.pool.as_ref())
            .await
            .map_err(persistence_error)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }
}

#[derive(Debug, Clone)]
pub struct SqlxExternalObjectLinkRepository {
    pool: Arc<MySqlPool>,
}

impl SqlxExternalObjectLinkRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ExternalObjectLinkRow {
    binding_id: String,
    data_id: String,
    external_object_id: String,
    last_accepted_external_revision: Option<String>,
    last_delivered_library_revision: Option<String>,
    base_content_hash: Option<String>,
}

impl TryFrom<ExternalObjectLinkRow> for ExternalObjectLink {
    type Error = errors::Error;

    fn try_from(row: ExternalObjectLinkRow) -> Result<Self, Self::Error> {
        ExternalObjectLink::new(
            ExternalSyncBindingId::parse(row.binding_id)?,
            LibraryDataId::parse(row.data_id)?,
            row.external_object_id,
            row.last_accepted_external_revision,
            row.last_delivered_library_revision,
            row.base_content_hash,
        )
    }
}

const LINK_COLUMNS: &str = r#"
    link.binding_id, link.data_id, link.external_object_id,
    link.last_accepted_external_revision,
    link.last_delivered_library_revision, link.base_content_hash
"#;

fn hash_external_object_id(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

#[async_trait]
impl ExternalObjectLinkRepository for SqlxExternalObjectLinkRepository {
    async fn save(
        &self,
        tenant_id: &TenantId,
        link: &ExternalObjectLink,
    ) -> errors::Result<()> {
        let mut transaction =
            self.pool.begin().await.map_err(persistence_error)?;
        let binding_exists: Option<String> = sqlx::query_scalar(
            "SELECT id FROM external_sync_bindings \
             WHERE id = ? AND tenant_id = ? FOR UPDATE",
        )
        .bind(link.binding_id().as_str())
        .bind(tenant_id.to_string())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(persistence_error)?;
        if binding_exists.is_none() {
            return Err(errors::Error::not_found(
                "external synchronization binding was not found in the tenant",
            ));
        }

        // The hash lookup is the provider-object identity. Lock it (or its
        // unique-index gap) before the upsert so a duplicate object can never
        // be silently attached to a different Library data item.
        let existing_data_id: Option<String> = sqlx::query_scalar(
            "SELECT data_id FROM external_object_links \
             WHERE binding_id = ? AND external_object_id_hash = ? FOR UPDATE",
        )
        .bind(link.binding_id().as_str())
        .bind(link.external_object_id_hash())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(persistence_error)?;
        if existing_data_id
            .as_deref()
            .is_some_and(|data_id| data_id != link.data_id().as_str())
        {
            return Err(errors::Error::conflict(
                "the external object is already linked to another Library data item",
            ));
        }

        sqlx::query(
            r#"
            INSERT INTO external_object_links (
                binding_id, data_id, external_object_id,
                external_object_id_hash,
                last_accepted_external_revision,
                last_delivered_library_revision, base_content_hash
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                external_object_id = VALUES(external_object_id),
                external_object_id_hash = VALUES(external_object_id_hash),
                last_accepted_external_revision =
                    VALUES(last_accepted_external_revision),
                last_delivered_library_revision =
                    VALUES(last_delivered_library_revision),
                base_content_hash = VALUES(base_content_hash)
            "#,
        )
        .bind(link.binding_id().as_str())
        .bind(link.data_id().as_str())
        .bind(link.external_object_id())
        .bind(link.external_object_id_hash())
        .bind(link.last_accepted_external_revision())
        .bind(link.last_delivered_library_revision())
        .bind(link.base_content_hash())
        .execute(&mut *transaction)
        .await
        .map_err(persistence_error)?;

        let canonical_data_id: String = sqlx::query_scalar(
            "SELECT data_id FROM external_object_links \
             WHERE binding_id = ? AND external_object_id_hash = ?",
        )
        .bind(link.binding_id().as_str())
        .bind(link.external_object_id_hash())
        .fetch_one(&mut *transaction)
        .await
        .map_err(persistence_error)?;
        if canonical_data_id != link.data_id().as_str() {
            return Err(errors::Error::conflict(
                "the external object is already linked to another Library data item",
            ));
        }
        transaction.commit().await.map_err(persistence_error)?;
        Ok(())
    }

    async fn find_by_data(
        &self,
        tenant_id: &TenantId,
        binding_id: &ExternalSyncBindingId,
        data_id: &LibraryDataId,
    ) -> errors::Result<Option<ExternalObjectLink>> {
        let query = format!(
            "SELECT {LINK_COLUMNS} FROM external_object_links AS link \
             JOIN external_sync_bindings AS binding ON binding.id = link.binding_id \
             WHERE binding.tenant_id = ? AND link.binding_id = ? AND link.data_id = ?"
        );
        let row: Option<ExternalObjectLinkRow> = sqlx::query_as(&query)
            .bind(tenant_id.to_string())
            .bind(binding_id.as_str())
            .bind(data_id.as_str())
            .fetch_optional(self.pool.as_ref())
            .await
            .map_err(persistence_error)?;
        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_external_object(
        &self,
        tenant_id: &TenantId,
        binding_id: &ExternalSyncBindingId,
        external_object_id: &str,
    ) -> errors::Result<Option<ExternalObjectLink>> {
        let query = format!(
            "SELECT {LINK_COLUMNS} FROM external_object_links AS link \
             JOIN external_sync_bindings AS binding ON binding.id = link.binding_id \
             WHERE binding.tenant_id = ? AND link.binding_id = ? \
               AND link.external_object_id_hash = ? AND link.external_object_id = ?"
        );
        let row: Option<ExternalObjectLinkRow> = sqlx::query_as(&query)
            .bind(tenant_id.to_string())
            .bind(binding_id.as_str())
            .bind(hash_external_object_id(external_object_id))
            .bind(external_object_id)
            .fetch_optional(self.pool.as_ref())
            .await
            .map_err(persistence_error)?;
        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_binding(
        &self,
        tenant_id: &TenantId,
        binding_id: &ExternalSyncBindingId,
    ) -> errors::Result<Vec<ExternalObjectLink>> {
        let query = format!(
            "SELECT {LINK_COLUMNS} FROM external_object_links AS link \
             JOIN external_sync_bindings AS binding ON binding.id = link.binding_id \
             WHERE binding.tenant_id = ? AND link.binding_id = ? ORDER BY link.data_id"
        );
        let rows: Vec<ExternalObjectLinkRow> = sqlx::query_as(&query)
            .bind(tenant_id.to_string())
            .bind(binding_id.as_str())
            .fetch_all(self.pool.as_ref())
            .await
            .map_err(persistence_error)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn find_by_tenant_and_data(
        &self,
        tenant_id: &TenantId,
        data_id: &LibraryDataId,
    ) -> errors::Result<Vec<ExternalObjectLink>> {
        let query = format!(
            "SELECT {LINK_COLUMNS} FROM external_object_links AS link \
             JOIN external_sync_bindings AS binding ON binding.id = link.binding_id \
             WHERE binding.tenant_id = ? AND link.data_id = ? ORDER BY link.binding_id"
        );
        let rows: Vec<ExternalObjectLinkRow> = sqlx::query_as(&query)
            .bind(tenant_id.to_string())
            .bind(data_id.as_str())
            .fetch_all(self.pool.as_ref())
            .await
            .map_err(persistence_error)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn binding_row_round_trip_preserves_scope_and_policies() {
        let scope = ExternalScope::new(json!({
            "github_repository": "example/docs",
            "ref": "main"
        }))
        .unwrap();
        let now = Utc::now();
        let row = ExternalSyncBindingRow {
            id: "esb_01j91h09tpj5ehwbwfwfxpak2b".into(),
            tenant_id: "tn_01j91h09tpj5ehwbwfwfxpak2b".into(),
            repo_id: "rp_01j91h09tpj5ehwbwfwfxpak2b".into(),
            provider: "github".into(),
            connection_id: "con_01j91h09tpj5ehwbwfwfxpak2b".into(),
            external_scope: scope.value().clone(),
            external_scope_hash: scope.identity_hash().into(),
            object_type: "markdown_document".into(),
            mapping: json!({"content": "body"}),
            inbound_policy: "review".into(),
            outbound_policy: "disabled".into(),
            delete_policy: "review_tombstone".into(),
            status: "paused".into(),
            created_at: now,
            updated_at: now,
        };

        let binding = ExternalSyncBinding::try_from(row).unwrap();
        assert_eq!(binding.external_scope(), &scope);
        assert_eq!(binding.inbound_policy(), ExternalSyncPolicy::Review);
        assert_eq!(binding.outbound_policy(), ExternalSyncPolicy::Disabled);
        assert_eq!(binding.status(), ExternalSyncBindingStatus::Paused);
    }

    #[test]
    fn object_link_row_round_trip_preserves_revisions() {
        let row = ExternalObjectLinkRow {
            binding_id: "esb_01j91h09tpj5ehwbwfwfxpak2b".into(),
            data_id: "data_01j91h09tpj5ehwbwfwfxpak2b".into(),
            external_object_id: "docs/guide.md".into(),
            last_accepted_external_revision: Some("github-sha".into()),
            last_delivered_library_revision: Some("revision-id".into()),
            base_content_hash: Some("b".repeat(64)),
        };

        let link = ExternalObjectLink::try_from(row).unwrap();
        assert_eq!(link.external_object_id(), "docs/guide.md");
        assert_eq!(
            link.last_accepted_external_revision(),
            Some("github-sha")
        );
        assert_eq!(link.base_content_hash(), Some("b".repeat(64).as_str()));
    }
}
