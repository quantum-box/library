use super::*;
use crate::usecase::DatabaseDeletionUnitOfWork;
use sqlx::{MySql, Transaction};

#[derive(Clone, Debug)]
pub struct DatabaseRepositoryImpl {
    pub db: Arc<Db>,
}

impl DatabaseRepositoryImpl {
    pub fn new(db: Arc<Db>) -> Arc<Self> {
        Arc::new(Self { db })
    }

    async fn delete_in_transaction(
        transaction: &mut Transaction<'_, MySql>,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
    ) -> errors::Result<Database> {
        // The Database row is the serialization boundary shared with
        // RelationDefinition writers. A writer locks both endpoints in
        // primary-key order; deleting one endpoint locks that row before the
        // inbound preflight, so a concurrent writer either commits before
        // this read or observes the Database deletion afterwards.
        let database = sqlx::query_as::<_, ObjectRow>(
            r#"
            SELECT id, tenant_id, object_name
            FROM objects
            WHERE tenant_id = ? AND id = ?
            FOR UPDATE;
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .fetch_optional(&mut **transaction)
        .await?
        .map(Database::from)
        .ok_or_else(|| errors::Error::not_found("resource not found"))?;

        // Self-relations belong to fields that are about to be deleted and
        // cascade with them. Any definition owned by another Database keeps
        // this target alive, irrespective of its future record-edge policy.
        let inbound_definition = sqlx::query_scalar::<_, String>(
            r#"
            SELECT id
            FROM relationships
            WHERE tenant_id = ?
              AND target_object_id = ?
              AND object_id <> ?
            ORDER BY object_id, id
            LIMIT 1
            FOR UPDATE;
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .bind(database_id.to_string())
        .fetch_optional(&mut **transaction)
        .await?;
        if inbound_definition.is_some() {
            return Err(errors::Error::conflict(
                "Database is referenced by an external RelationDefinition",
            ));
        }

        // A generated inverse is an owned schema child in another Database.
        // Cascading only the source field would orphan that child. Until a
        // multi-Database delete command is introduced, require callers to
        // delete these Relation schemas through their endpoint-ordered UoW.
        // Holding the source object lock closes the race with every writer.
        let external_owned_inverse = sqlx::query_scalar::<_, String>(
            r#"
            SELECT id
            FROM relationships
            WHERE tenant_id = ?
              AND object_id = ?
              AND target_object_id <> ?
              AND inverse_owned = TRUE
            ORDER BY target_object_id, id
            LIMIT 1
            FOR UPDATE;
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .bind(database_id.to_string())
        .fetch_optional(&mut **transaction)
        .await?;
        if external_owned_inverse.is_some() {
            return Err(errors::Error::conflict(
                "Database owns an external inverse Property; delete its Relation schema first",
            ));
        }

        // `indexes.object_id` is the legacy name of its referenced Data id.
        // That foreign key intentionally has no ON DELETE CASCADE, so remove
        // the Database-owned projection rows before deleting their records.
        // Joining through `data` keeps both tenant and Database ownership in
        // the predicate instead of trusting a globally unique Data id alone.
        sqlx::query(
            r#"
            DELETE indexes
            FROM indexes
            INNER JOIN data
                ON data.tenant_id = indexes.tenant_id
               AND data.id = indexes.object_id
            WHERE data.tenant_id = ? AND data.object_id = ?;
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .execute(&mut **transaction)
        .await?;

        sqlx::query(
            r#"
            DELETE FROM data
            WHERE tenant_id = ? AND object_id = ?;
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .execute(&mut **transaction)
        .await?;

        // RelationDefinition source ownership is RESTRICT rather than
        // cascading so mixed-fleet legacy deleters cannot orphan an owned
        // inverse. This UoW has completed every preflight and may remove the
        // Database-owned definitions explicitly before their Properties.
        sqlx::query(
            r#"
            DELETE FROM relationships
            WHERE tenant_id = ? AND object_id = ?;
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .execute(&mut **transaction)
        .await?;

        sqlx::query(
            r#"
            DELETE FROM fields
            WHERE tenant_id = ? AND object_id = ?;
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .execute(&mut **transaction)
        .await?;

        let deleted = sqlx::query(
            r#"
            DELETE FROM objects
            WHERE tenant_id = ? AND id = ?;
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .execute(&mut **transaction)
        .await?;
        if deleted.rows_affected() != 1 {
            return Err(errors::Error::internal_server_error(
                "locked Database disappeared during deletion",
            ));
        }

        Ok(database)
    }
}

#[async_trait::async_trait]
impl DatabaseDeletionUnitOfWork for DatabaseRepositoryImpl {
    async fn delete_atomically(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
    ) -> errors::Result<Database> {
        let pool = self.db.pool();
        let mut transaction = pool.begin().await?;
        match Self::delete_in_transaction(
            &mut transaction,
            tenant_id,
            database_id,
        )
        .await
        {
            Ok(database) => {
                transaction.commit().await?;
                Ok(database)
            }
            Err(error) => {
                transaction.rollback().await.map_err(|rollback_error| {
                    errors::Error::internal_server_error(format!(
                        "Database deletion rollback failed after {error}: {rollback_error}"
                    ))
                })?;
                Err(error)
            }
        }
    }
}

#[async_trait::async_trait]
impl RepositoryV1<DatabaseId, Database> for DatabaseRepositoryImpl {
    async fn save(&self, database: &Database) -> errors::Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO tachyon_apps_database_manager.objects (id, tenant_id, object_name) 
            VALUES (?, ?, ?)
            ON DUPLICATE KEY UPDATE
                object_name = VALUES(object_name);
            "#,
            database.id().to_string(),
            database.tenant_id().to_string(),
            database.name().to_string(),
        )
        .execute(self.db.pool().as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(format!("Fail to save database: {e}")))?;
        Ok(())
    }

    async fn get_by_id(
        &self,
        tenant_id: &TenantId,
        id: &DatabaseId,
    ) -> errors::Result<Option<Database>> {
        Ok(sqlx::query_as::<_, ObjectRow>(
            r#"
            SELECT id, tenant_id, object_name
            FROM objects
            WHERE id = ? AND tenant_id = ?;
            "#,
        )
        .bind(id.to_string())
        .bind(tenant_id.to_string())
        .fetch_optional(self.db.pool().as_ref())
        .await
        .map_err(|e| {
            errors::Error::internal_server_error(format!(
                "Fail to get database: {e}"
            ))
        })?
        .map(|row| row.into()))
    }

    async fn find_all(
        &self,
        tenant_id: &TenantId,
    ) -> errors::Result<Vec<Database>> {
        Ok(sqlx::query_as::<_, ObjectRow>(
            r#"
            SELECT id, tenant_id, object_name
            FROM objects
            WHERE tenant_id = ?;
            "#,
        )
        .bind(tenant_id.to_string())
        .fetch_all(self.db.pool().as_ref())
        .await
        .map_err(|e| {
            errors::Error::internal_server_error(format!(
                "Fail to get database: {e}"
            ))
        })?
        .into_iter()
        .map(|row| row.into())
        .collect())
    }

    async fn delete(
        &self,
        tenant_id: &TenantId,
        id: &DatabaseId,
    ) -> errors::Result<()> {
        sqlx::query(
            r#"
            DELETE FROM objects
            WHERE id = ? AND tenant_id = ?;
            "#,
        )
        .bind(id.to_string())
        .bind(tenant_id.to_string())
        .execute(self.db.pool().as_ref())
        .await
        .map_err(|e| {
            errors::Error::internal_server_error(format!(
                "Fail to delete database: {e}"
            ))
        })?;
        Ok(())
    }
}
