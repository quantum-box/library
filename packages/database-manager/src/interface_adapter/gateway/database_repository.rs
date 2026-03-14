use super::*;

#[derive(Clone, Debug)]
pub struct DatabaseRepositoryImpl {
    pub db: Arc<Db>,
}

impl DatabaseRepositoryImpl {
    pub fn new(db: Arc<Db>) -> Arc<Self> {
        Arc::new(Self { db })
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
