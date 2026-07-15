use super::*;
use std::str::FromStr;

const CREATE_DATA_SCOPED_SQL: &str = r#"
    INSERT INTO tachyon_apps_database_manager.data (
        id,
        tenant_id,
        object_id,
        name,
        created_at,
        updated_at
    )
    SELECT
        ?,
        ?,
        scoped_database.id,
        ?,
        ?,
        ?
    FROM tachyon_apps_database_manager.objects AS scoped_database
    WHERE scoped_database.tenant_id = ?
      AND scoped_database.id = ?
"#;

const FIND_DATA_BY_ID_SQL: &str = r#"
    SELECT
        *
    FROM
        tachyon_apps_database_manager.data
    WHERE
        tenant_id = ? and object_id = ? and id = ?
"#;

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
            let property_type = PropertyType::from_meta(
                &field.datatype,
                field.datatype_meta.clone(),
            )?;
            let data_field_value = project_property_value(
                &data_row.id,
                &property_type,
                data_row.get_field(field.field_num)?,
            )?;
            let property = Property::with_meta_json(
                &field.id.parse()?,
                &field.tenant_id.parse()?,
                &field.object_id.parse()?,
                &field.field_name,
                &property_type,
                field.is_indexed,
                field.field_num,
                field.meta_json.clone(),
            );
            let property_data =
                PropertyData::from_storage(&property, data_field_value)?;
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
        let insert_result = sqlx::query(CREATE_DATA_SCOPED_SQL)
            .bind(data.id().to_string())
            .bind(data.tenant_id().to_string())
            .bind(data.name().to_string())
            .bind(data.created_at())
            .bind(data.updated_at())
            .bind(data.tenant_id().to_string())
            .bind(data.database_id().to_string())
            .execute(&mut *tx)
            .await;
        let insert_result = match insert_result {
            Ok(result) => result,
            Err(sqlx::Error::Database(error))
                if error.is_unique_violation() =>
            {
                return Err(errors::Error::conflict(
                    "data id already exists",
                ));
            }
            Err(error) => return Err(error.into()),
        };
        if insert_result.rows_affected() == 0 {
            return Err(
                crate::usecase::database_scope::DatabaseScope::not_found(),
            );
        }
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
        let row = sqlx::query_as::<_, DataRow>(FIND_DATA_BY_ID_SQL)
            .bind(tenant_id.to_string())
            .bind(database_id.to_string())
            .bind(id.to_string())
            .fetch_optional(self.db.pool().as_ref())
            .await?;

        row.map(|row| self.convert_to_data(row, fields)).transpose()
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

#[cfg(test)]
mod scope_query_tests {
    use super::*;

    fn normalize(sql: &str) -> String {
        sql.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase()
    }

    #[test]
    fn find_by_id_requires_tenant_database_and_data_predicates() {
        assert!(normalize(FIND_DATA_BY_ID_SQL)
            .contains("where tenant_id = ? and object_id = ? and id = ?"));
    }

    #[test]
    fn create_is_scoped_and_never_upserts_an_existing_data_id() {
        let sql = normalize(CREATE_DATA_SCOPED_SQL);

        assert!(sql.contains(
            "from tachyon_apps_database_manager.objects as scoped_database"
        ));
        assert!(sql.contains(
            "where scoped_database.tenant_id = ? and scoped_database.id = ?"
        ));
        assert!(!sql.contains("on duplicate key update"));
    }
}
