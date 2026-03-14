use super::*;
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct DataRepositoryImpl {
    pub db: Arc<Db>,
}

impl DataRepositoryImpl {
    pub fn new(db: Arc<Db>) -> Arc<Self> {
        Arc::new(Self { db })
    }

    fn convert_to_data(
        &self,
        data_row: DataRow,
        fields: Vec<FieldRow>,
    ) -> errors::Result<Data> {
        let tenant_id = TenantId::from_str(&data_row.tenant_id)?;
        let database_id = DatabaseId::from_str(&data_row.object_id)?;
        let mut data = Data::new(
            &data_row.id.parse()?,
            &tenant_id,
            &database_id,
            &data_row.name,
            vec![],
            data_row.created_at,
            data_row.updated_at,
        )?;
        for field in fields {
            let data_field_value = data_row.get_field(field.field_num)?;
            let property = Property::new(
                &field.id.parse()?,
                &field.tenant_id.parse()?,
                &field.object_id.parse()?,
                &field.field_name,
                &field.datatype.parse()?,
                field.is_indexed,
                field.field_num,
            );
            let property_data =
                PropertyData::new(&property, data_field_value)?;
            data.add_property_data(property_data)?;
        }
        Ok(data)
    }
}

#[async_trait::async_trait]
impl DataRepository for DataRepositoryImpl {
    #[tracing::instrument(skip(self))]
    async fn create(&self, data: &Data) -> errors::Result<()> {
        let fields = sqlx::query_as::<_, FieldRow>(
            r#"
            SELECT
                *
            FROM
                tachyon_apps_database_manager.fields
            WHERE
                object_id = ? and tenant_id = ?
            "#,
        )
        .bind(data.database_id().to_string())
        .bind(data.tenant_id().to_string())
        .fetch_all(self.db.pool().as_ref())
        .await?;

        let mut tx = self.db.pool().begin().await?;
        sqlx::query(
            r#"
            INSERT INTO tachyon_apps_database_manager.data (
                id,
                tenant_id,
                object_id,
                name,
                created_at,
                updated_at
            ) VALUES (
                ?,
                ?,
                ?,
                ?,
                ?,
                ?
            )
            ON DUPLICATE KEY UPDATE
                name = VALUES(name)
            "#,
        )
        .bind(data.id().to_string())
        .bind(data.tenant_id().to_string())
        .bind(data.database_id().to_string())
        .bind(data.name().to_string())
        .bind(data.created_at())
        .bind(data.updated_at())
        .execute(&mut *tx)
        .await?;
        for val in data.property_data().iter() {
            let field_num = fields
                .iter()
                .find(|f| *val.property_id() == f.id)
                .ok_or_else(|| {
                    errors::internal_server_error!(
                        "Property with id {} not found",
                        val.property_id().to_string()
                    )
                })?
                .field_num;
            sqlx::query(&format!(
                r#"
                update tachyon_apps_database_manager.data
                set value{field_num} = ?
                where object_id = ? and tenant_id = ? and id = ?
                "#
            ))
            .bind(val.string_value())
            .bind(data.database_id().to_string())
            .bind(data.tenant_id().to_string())
            .bind(data.id().to_string())
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn update(&self, data: &Data) -> errors::Result<()> {
        let fields = sqlx::query_as::<_, FieldRow>(
            r#"
            SELECT
                *
            FROM
                tachyon_apps_database_manager.fields
            WHERE
                object_id = ? and tenant_id = ?
            "#,
        )
        .bind(data.database_id().to_string())
        .bind(data.tenant_id().to_string())
        .fetch_all(self.db.pool().as_ref())
        .await?;
        let mut tx = self.db.pool().begin().await?;
        sqlx::query!(
            r#"
            update tachyon_apps_database_manager.data
            set name = ?,
                updated_at = ?
            where object_id = ? and tenant_id = ? and id = ?
            "#,
            data.name().to_string(),
            data.updated_at(),
            data.database_id().to_string(),
            data.tenant_id().to_string(),
            data.id().to_string(),
        )
        .execute(&mut *tx)
        .await?;
        for val in data.property_data().iter() {
            let field = fields
                .iter()
                .find(|f| *val.property_id() == f.id)
                .ok_or_else(|| {
                    errors::internal_server_error!(
                        "Property with id {} not found",
                        val.property_id().to_string()
                    )
                })?;
            sqlx::query(&format!(
                r#"
                update tachyon_apps_database_manager.data
                set value{} = ?
                where object_id = ? and tenant_id = ? and id = ?
                "#,
                field.field_num
            ))
            .bind(val.string_value())
            .bind(data.database_id().to_string())
            .bind(data.tenant_id().to_string())
            .bind(data.id().to_string())
            .execute(&mut *tx)
            .await?;
        }
        if fields.len() > data.property_data().len() {
            // delete a property_data for will delete property
            let current_properties: Vec<String> = data
                .property_data()
                .iter()
                .map(|v| v.property_id().to_string())
                .collect();
            let diff: Vec<_> = fields
                .iter()
                .filter(|v| !current_properties.contains(&v.id))
                .collect();
            for field in diff {
                sqlx::query(&format!(
                    r#"
                    update tachyon_apps_database_manager.data
                    set value{} = null
                    where object_id = ? and tenant_id = ? and id = ?
                    "#,
                    field.field_num
                ))
                .bind(data.database_id().to_string())
                .bind(data.tenant_id().to_string())
                .bind(data.id().to_string())
                .execute(&mut *tx)
                .await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn update_all(
        &self,
        data: &DataCollection,
    ) -> errors::Result<()> {
        for d in data.value() {
            self.update(d).await?;
        }
        Ok(())
    }

    async fn find_by_id(
        &self,
        id: &DataId,
        database_id: &DatabaseId,
        tenant_id: &TenantId,
    ) -> errors::Result<Option<Data>> {
        let fields = sqlx::query_as::<_, FieldRow>(
            r#"
            SELECT
                *
            FROM
                tachyon_apps_database_manager.fields
            WHERE
                object_id = ? and tenant_id = ?
            "#,
        )
        .bind(database_id.to_string())
        .bind(tenant_id.to_string())
        .fetch_all(self.db.pool().as_ref())
        .await?;
        Ok(sqlx::query_as::<_, DataRow>(
            r#"
            SELECT
                *
            FROM
                tachyon_apps_database_manager.data
            WHERE
                tenant_id = ? and id = ?
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(id.to_string())
        .fetch_optional(self.db.pool().as_ref())
        .await?
        .map(|row| self.convert_to_data(row, fields).unwrap()))
    }

    #[tracing::instrument(skip(self))]
    async fn find_all(
        &self,
        id: &DatabaseId,
        tenant_id: &TenantId,
    ) -> errors::Result<DataCollection> {
        let fields = sqlx::query_as::<_, FieldRow>(
            r#"
            SELECT
                *
            FROM
                tachyon_apps_database_manager.fields
            WHERE
                object_id = ? and tenant_id = ?
            ORDER BY
                field_num ASC
            "#,
        )
        .bind(id.to_string())
        .bind(tenant_id.to_string())
        .fetch_all(self.db.pool().as_ref())
        .await?;
        let data_rows = sqlx::query_as::<_, DataRow>(
            r#"
            SELECT
                *
            FROM
                tachyon_apps_database_manager.data
            WHERE
                object_id = ? and tenant_id = ?
            "#,
        )
        .bind(id.to_string())
        .bind(tenant_id.to_string())
        .fetch_all(self.db.pool().as_ref())
        .await?;
        let mut data_vec = vec![];
        for data_row in data_rows {
            let data = self.convert_to_data(data_row, fields.clone())?;
            data_vec.push(data);
        }
        Ok(DataCollection::new(data_vec))
    }

    #[tracing::instrument(skip(self))]
    async fn delete(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        id: &DataId,
    ) -> errors::Result<()> {
        sqlx::query(
            r#"
            DELETE FROM tachyon_apps_database_manager.data
            WHERE object_id = ? and tenant_id = ? and id = ?
            "#,
        )
        .bind(database_id.to_string())
        .bind(tenant_id.to_string())
        .bind(id.to_string())
        .execute(self.db.pool().as_ref())
        .await?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn delete_all(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
    ) -> errors::Result<()> {
        sqlx::query(
            r#"
            DELETE FROM tachyon_apps_database_manager.data
            WHERE object_id = ? and tenant_id = ?
            "#,
        )
        .bind(database_id.to_string())
        .bind(tenant_id.to_string())
        .execute(self.db.pool().as_ref())
        .await?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn find_all_with_paging(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        page: u32,
        page_size: u32,
    ) -> errors::Result<(DataCollection, OffsetPaginator)> {
        let fields = sqlx::query_as::<_, FieldRow>(
            r#"
            SELECT
                *
            FROM
                tachyon_apps_database_manager.fields
            WHERE
                object_id = ? and tenant_id = ?
            "#,
        )
        .bind(database_id.to_string())
        .bind(tenant_id.to_string())
        .fetch_all(self.db.pool().as_ref())
        .await?;
        let offset = (page - 1) * page_size;
        let data_rows = sqlx::query_as::<_, DataRow>(
            r#"
            SELECT
                *
            FROM
                tachyon_apps_database_manager.data
            WHERE
                object_id = ? and tenant_id = ?
            ORDER BY
                id ASC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(database_id.to_string())
        .bind(tenant_id.to_string())
        .bind(page_size)
        .bind(offset)
        .fetch_all(self.db.pool().as_ref())
        .await?;
        let total: i64 = sqlx::query_scalar!(
            r#"
            SELECT
                COUNT(*)
            FROM
                tachyon_apps_database_manager.data
            WHERE
                object_id = ? and tenant_id = ?
            "#,
            database_id.to_string(),
            tenant_id.to_string(),
        )
        .fetch_one(self.db.pool().as_ref())
        .await?;
        let mut data_vec = vec![];
        for data_row in data_rows {
            let data = self.convert_to_data(data_row, fields.clone())?;
            data_vec.push(data);
        }
        let paginator = OffsetPaginator::new(page, total as u32, page_size);
        Ok((DataCollection::new(data_vec), paginator))
    }
}
