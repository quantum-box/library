use super::*;

#[derive(Clone, Debug)]
pub struct PropertyRepositoryImpl {
    pub db: Arc<Db>,
}

impl PropertyRepositoryImpl {
    pub fn new(db: Arc<Db>) -> Arc<Self> {
        Arc::new(Self { db })
    }
}

#[async_trait::async_trait]
impl PropertyRepository for PropertyRepositoryImpl {
    async fn create(&self, property: &Property) -> errors::Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO tachyon_apps_database_manager.fields 
                (id, tenant_id, object_id, field_name, datatype, datatype_meta, is_indexed, field_num, meta_json)
            VALUES
                (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                field_name = VALUES(field_name),
                datatype = VALUES(datatype),
                datatype_meta = VALUES(datatype_meta),
                is_indexed = VALUES(is_indexed),
                field_num = VALUES(field_num),
                meta_json = VALUES(meta_json);
            "#,
            property.id().to_string(),
            property.tenant_id().to_string(),
            property.database_id().to_string(),
            property.name(),
            property.property_type().to_string(),
            property.property_type().get_meta()?,
            property.is_indexed(),
            property.property_num(),
            property.meta_json(),
        )
        .execute(self.db.pool().as_ref())
        .await?;
        Ok(())
    }

    async fn update(&self, property: &Property) -> errors::Result<()> {
        sqlx::query!(
            "
            UPDATE tachyon_apps_database_manager.fields
            SET field_name = ?, datatype = ?, is_indexed = ?, datatype_meta = ?, meta_json = ?
            WHERE tenant_id = ? AND object_id = ? AND id = ?;
            ",
            property.name(),
            property.property_type().to_string(),
            property.is_indexed(),
            property.property_type().get_meta()?,
            property.meta_json(),
            property.tenant_id().to_string(),
            property.database_id().to_string(),
            property.id().to_string(),
        )
        .execute(self.db.pool().as_ref())
        .await?;
        Ok(())
    }
    async fn find_by_id(
        &self,
        id: &PropertyId,
        database_id: &DatabaseId,
        tenant_id: &TenantId,
    ) -> errors::Result<Option<Property>> {
        Ok(sqlx::query_as::<_, FieldRow>(
            "SELECT * 
            FROM fields
            WHERE tenant_id = ? AND object_id = ? AND id = ? LIMIT 1;
            ",
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .bind(id.to_string())
        .fetch_optional(self.db.pool().as_ref())
        .await?
        .map(|row| row.into()))
    }
    async fn find_all(
        &self,
        database_id: &DatabaseId,
        tenant_id: &TenantId,
    ) -> errors::Result<Vec<Property>> {
        Ok(sqlx::query_as::<_, FieldRow>(
            "SELECT * 
            FROM fields
            WHERE tenant_id = ? AND object_id = ?;
            ",
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .fetch_all(self.db.pool().as_ref())
        .await?
        .into_iter()
        .map(|row| row.into())
        .collect::<Vec<Property>>())
    }

    async fn delete_all(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
    ) -> errors::Result<()> {
        sqlx::query(
            "
            DELETE FROM fields
            WHERE tenant_id = ? AND object_id = ?;
            ",
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .execute(self.db.pool().as_ref())
        .await?;
        Ok(())
    }

    async fn delete(
        &self,
        tenant_id: &TenantId,
        id: &PropertyId,
    ) -> errors::Result<()> {
        sqlx::query(
            "
            DELETE FROM fields
            WHERE tenant_id = ? AND id = ?;
            ",
        )
        .bind(tenant_id.to_string())
        .bind(id.to_string())
        .execute(self.db.pool().as_ref())
        .await?;
        Ok(())
    }
}
