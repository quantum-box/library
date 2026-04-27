//! PLT-942: SQLx implementation of GlobalIdMappingRepository.
//!
//! Tenant-scoped: every query carries `WHERE tenant_id = ?`. Cross-tenant
//! `id` lookups silently miss (no information leak).

use std::sync::Arc;

use chrono::{DateTime, Utc};
use sqlx::FromRow;

use crate::domain::{
    GlobalId, GlobalIdMapping, GlobalIdMappingId, GlobalIdMappingRepository,
};
use persistence;
use value_object::{TenantId, Text};

#[derive(Debug, FromRow)]
struct GlobalIdMappingRow {
    id: String,
    tenant_id: String,
    global_id: String,
    system: String,
    system_code: String,
    name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<GlobalIdMappingRow> for GlobalIdMapping {
    type Error = errors::Error;

    fn try_from(row: GlobalIdMappingRow) -> Result<Self, Self::Error> {
        let id: GlobalIdMappingId =
            row.id.parse().map_err(|e: errors::ParseIdError| {
                errors::Error::invalid(e.to_string())
            })?;
        let tenant_id: TenantId =
            row.tenant_id.parse().map_err(|e: errors::ParseIdError| {
                errors::Error::invalid(e.to_string())
            })?;
        let global_id: GlobalId =
            row.global_id.parse().map_err(|e: errors::ParseIdError| {
                errors::Error::invalid(e.to_string())
            })?;
        let system = row
            .system
            .parse()
            .map_err(|e: anyhow::Error| errors::Error::invalid(e.to_string()))?;
        let system_code = row.system_code.parse().map_err(
            |e: anyhow::Error| errors::Error::invalid(e.to_string()),
        )?;
        let name = row
            .name
            .parse()
            .map_err(|e: anyhow::Error| errors::Error::invalid(e.to_string()))?;

        Ok(GlobalIdMapping::new(
            &id,
            &tenant_id,
            &global_id,
            &system,
            &system_code,
            &name,
            row.created_at,
            row.updated_at,
        ))
    }
}

#[derive(Debug)]
pub struct GlobalIdMappingRepositoryImpl {
    db: Arc<persistence::Db>,
}

impl GlobalIdMappingRepositoryImpl {
    pub fn new(db: Arc<persistence::Db>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl GlobalIdMappingRepository for GlobalIdMappingRepositoryImpl {
    async fn insert(&self, entity: &GlobalIdMapping) -> errors::Result<()> {
        let result = sqlx::query(
            r#"
            INSERT INTO `global_id_mapping`
                (`id`, `tenant_id`, `global_id`, `system`, `system_code`,
                 `name`, `created_at`, `updated_at`)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(entity.id().to_string())
        .bind(entity.tenant_id().to_string())
        .bind(entity.global_id().to_string())
        .bind(entity.system().to_string())
        .bind(entity.system_code().to_string())
        .bind(entity.name().to_string())
        .bind(entity.created_at())
        .bind(entity.updated_at())
        .execute(self.db.pool().as_ref())
        .await;

        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(db_err))
                if db_err.is_unique_violation() =>
            {
                Err(errors::Error::conflict(format!(
                    "global_id_mapping unique constraint violated: {}",
                    db_err
                )))
            }
            Err(e) => Err(errors::Error::internal_server_error(e)),
        }
    }

    async fn update_name(
        &self,
        tenant_id: &TenantId,
        id: &GlobalIdMappingId,
        name: &Text,
    ) -> errors::Result<()> {
        let affected = sqlx::query(
            r#"
            UPDATE `global_id_mapping`
            SET `name` = ?, `updated_at` = ?
            WHERE `tenant_id` = ? AND `id` = ?
            "#,
        )
        .bind(name.to_string())
        .bind(Utc::now())
        .bind(tenant_id.to_string())
        .bind(id.to_string())
        .execute(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?
        .rows_affected();

        if affected == 0 {
            return Err(errors::Error::not_found(format!(
                "global_id_mapping {} not found",
                id
            )));
        }
        Ok(())
    }

    async fn get_by_id(
        &self,
        tenant_id: &TenantId,
        id: &GlobalIdMappingId,
    ) -> errors::Result<Option<GlobalIdMapping>> {
        let row: Option<GlobalIdMappingRow> = sqlx::query_as(
            r#"
            SELECT `id`, `tenant_id`, `global_id`, `system`, `system_code`,
                   `name`, `created_at`, `updated_at`
            FROM `global_id_mapping`
            WHERE `tenant_id` = ? AND `id` = ?
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(id.to_string())
        .fetch_optional(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?;

        row.map(GlobalIdMapping::try_from).transpose()
    }

    async fn find_by_system_code(
        &self,
        tenant_id: &TenantId,
        system: &str,
        system_code: &str,
    ) -> errors::Result<Option<GlobalIdMapping>> {
        let row: Option<GlobalIdMappingRow> = sqlx::query_as(
            r#"
            SELECT `id`, `tenant_id`, `global_id`, `system`, `system_code`,
                   `name`, `created_at`, `updated_at`
            FROM `global_id_mapping`
            WHERE `tenant_id` = ? AND `system` = ? AND `system_code` = ?
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(system)
        .bind(system_code)
        .fetch_optional(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?;

        row.map(GlobalIdMapping::try_from).transpose()
    }

    async fn find_all(
        &self,
        tenant_id: &TenantId,
        system: Option<&str>,
    ) -> errors::Result<Vec<GlobalIdMapping>> {
        let rows: Vec<GlobalIdMappingRow> = match system {
            Some(s) => {
                sqlx::query_as(
                    r#"
                    SELECT `id`, `tenant_id`, `global_id`, `system`,
                           `system_code`, `name`, `created_at`, `updated_at`
                    FROM `global_id_mapping`
                    WHERE `tenant_id` = ? AND `system` = ?
                    ORDER BY `created_at` DESC
                    "#,
                )
                .bind(tenant_id.to_string())
                .bind(s)
                .fetch_all(self.db.pool().as_ref())
                .await
            }
            None => {
                sqlx::query_as(
                    r#"
                    SELECT `id`, `tenant_id`, `global_id`, `system`,
                           `system_code`, `name`, `created_at`, `updated_at`
                    FROM `global_id_mapping`
                    WHERE `tenant_id` = ?
                    ORDER BY `created_at` DESC
                    "#,
                )
                .bind(tenant_id.to_string())
                .fetch_all(self.db.pool().as_ref())
                .await
            }
        }
        .map_err(errors::Error::internal_server_error)?;

        rows.into_iter()
            .map(GlobalIdMapping::try_from)
            .collect::<Result<Vec<_>, _>>()
    }
}
