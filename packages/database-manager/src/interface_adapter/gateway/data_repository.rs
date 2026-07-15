use super::*;
use crate::property_value_rollout::PropertyValueStorageMode;
use std::collections::HashSet;

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
    pub property_value_mode: PropertyValueStorageMode,
}

impl DataRepositoryImpl {
    pub fn new(db: Arc<Db>) -> Arc<Self> {
        Self::new_with_property_value_mode(db, Default::default())
    }

    pub fn new_with_property_value_mode(
        db: Arc<Db>,
        property_value_mode: PropertyValueStorageMode,
    ) -> Arc<Self> {
        Arc::new(Self {
            db,
            property_value_mode,
        })
    }
}

impl DataRepositoryImpl {
    async fn lock_database_and_fields(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::MySql>,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
    ) -> errors::Result<Vec<FieldRow>> {
        let database = sqlx::query_scalar::<_, String>(
            r#"
            SELECT id FROM objects
            WHERE tenant_id = ? AND id = ?
            FOR SHARE
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .fetch_optional(&mut **transaction)
        .await?;
        if database.is_none() {
            return Err(errors::Error::not_found("resource not found"));
        }

        Ok(sqlx::query_as::<_, FieldRow>(
            r#"
            SELECT id, tenant_id, object_id, field_name, datatype,
                   datatype_meta, is_indexed, field_num, meta_json
            FROM fields
            WHERE tenant_id = ? AND object_id = ?
            ORDER BY field_num ASC
            FOR SHARE
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .fetch_all(&mut **transaction)
        .await?)
    }

    fn persisted_property(
        fields: &[FieldRow],
        requested: &Property,
    ) -> errors::Result<(Property, u32)> {
        let field = fields
            .iter()
            .find(|field| *requested.id() == field.id)
            .ok_or_else(|| {
                errors::Error::not_found("resource not found")
            })?;
        let persisted: Property = field.clone().into();
        if persisted.tenant_id() != requested.tenant_id()
            || persisted.database_id() != requested.database_id()
            || persisted.property_type().canonical_type_ref()
                != requested.property_type().canonical_type_ref()
        {
            return Err(errors::Error::conflict(
                "Property definition changed during record mutation",
            ));
        }
        Ok((persisted, field.field_num))
    }

    async fn ensure_existing_canonical_is_writable(
        transaction: &mut sqlx::Transaction<'_, sqlx::MySql>,
        record: &Data,
        property: &Property,
    ) -> errors::Result<()> {
        let row = sqlx::query_as::<_, PropertyValueRow>(
            r#"
            SELECT tenant_id, database_id, data_id, property_id, type_key,
                   type_version, value_encoding_version, value
            FROM property_values
            WHERE tenant_id = ? AND database_id = ?
              AND data_id = ? AND property_id = ?
            FOR UPDATE
            "#,
        )
        .bind(record.tenant_id().to_string())
        .bind(record.database_id().to_string())
        .bind(record.id().to_string())
        .bind(property.id().to_string())
        .fetch_optional(&mut **transaction)
        .await?;

        if let Some(row) = row {
            let config = ResolvedPropertyConfig::Known(
                property.property_type().canonical_config(),
            );
            BUILTIN_PROPERTY_TYPE_REGISTRY
                .decode_envelope(&config, row.envelope()?)?
                .ensure_writable()?;
        }
        Ok(())
    }

    async fn apply_change(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::MySql>,
        record: &Data,
        fields: &[FieldRow],
        change: &PropertyValueChange,
    ) -> errors::Result<()> {
        let (property, field_num) =
            Self::persisted_property(fields, change.property())?;
        if self.property_value_mode.writes_canonical() {
            Self::ensure_existing_canonical_is_writable(
                transaction,
                record,
                &property,
            )
            .await?;
        }

        match change {
            PropertyValueChange::Set { value, .. } => {
                value.ensure_writable()?;
                if value.type_ref()
                    != &property.property_type().canonical_type_ref()
                {
                    return Err(errors::Error::invalid(
                        "PropertyValue type does not match Property",
                    ));
                }
                let config = property.property_type().canonical_config();
                let envelope = BUILTIN_PROPERTY_TYPE_REGISTRY
                    .encode_envelope(&config, value.value())?;

                if self.property_value_mode.writes_canonical() {
                    sqlx::query(
                        r#"
                        INSERT INTO property_values
                            (tenant_id, database_id, data_id, property_id,
                             type_key, type_version,
                             value_encoding_version, value)
                        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                        ON DUPLICATE KEY UPDATE
                            type_key = VALUES(type_key),
                            type_version = VALUES(type_version),
                            value_encoding_version =
                                VALUES(value_encoding_version),
                            value = VALUES(value)
                        "#,
                    )
                    .bind(record.tenant_id().to_string())
                    .bind(record.database_id().to_string())
                    .bind(record.id().to_string())
                    .bind(property.id().to_string())
                    .bind(envelope.type_ref.key.as_str())
                    .bind(envelope.type_ref.version.get())
                    .bind(envelope.encoding_version.get())
                    .bind(
                        serde_json::to_string(&envelope.raw_value)
                            .map_err(errors::Error::invalid)?,
                    )
                    .execute(&mut **transaction)
                    .await?;
                }

                let legacy = LegacyPropertyValueCodec::encode(
                    value.value(),
                    property.property_type(),
                )?;
                sqlx::query(&format!(
                    "UPDATE data SET value{field_num} = ? \
                     WHERE tenant_id = ? AND object_id = ? AND id = ?"
                ))
                .bind(legacy)
                .bind(record.tenant_id().to_string())
                .bind(record.database_id().to_string())
                .bind(record.id().to_string())
                .execute(&mut **transaction)
                .await?;
            }
            PropertyValueChange::Clear { .. } => {
                if self.property_value_mode.writes_canonical() {
                    sqlx::query(
                        r#"
                        DELETE FROM property_values
                        WHERE tenant_id = ? AND database_id = ?
                          AND data_id = ? AND property_id = ?
                        "#,
                    )
                    .bind(record.tenant_id().to_string())
                    .bind(record.database_id().to_string())
                    .bind(record.id().to_string())
                    .bind(property.id().to_string())
                    .execute(&mut **transaction)
                    .await?;
                }
                sqlx::query(&format!(
                    "UPDATE data SET value{field_num} = NULL \
                     WHERE tenant_id = ? AND object_id = ? AND id = ?"
                ))
                .bind(record.tenant_id().to_string())
                .bind(record.database_id().to_string())
                .bind(record.id().to_string())
                .execute(&mut **transaction)
                .await?;
            }
        }
        Ok(())
    }

    fn validate_changes(
        changes: &[PropertyValueChange],
    ) -> errors::Result<()> {
        let mut property_ids = HashSet::with_capacity(changes.len());
        for change in changes {
            if !property_ids.insert(change.property().id().to_string()) {
                return Err(errors::Error::invalid(
                    "record mutation contains a duplicate Property",
                ));
            }
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl RecordUnitOfWork for DataRepositoryImpl {
    async fn create_atomically(
        &self,
        command: &CreateRecordCommand,
    ) -> errors::Result<()> {
        Self::validate_changes(&command.changes)?;
        let record = &command.record;
        let mut transaction = self.db.pool().begin().await?;
        let fields = self
            .lock_database_and_fields(
                &mut transaction,
                record.tenant_id(),
                record.database_id(),
            )
            .await?;
        let result = sqlx::query(CREATE_DATA_SCOPED_SQL)
            .bind(record.id().to_string())
            .bind(record.tenant_id().to_string())
            .bind(record.name().to_string())
            .bind(record.created_at())
            .bind(record.updated_at())
            .bind(record.tenant_id().to_string())
            .bind(record.database_id().to_string())
            .execute(&mut *transaction)
            .await;
        let result = match result {
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
        if result.rows_affected() == 0 {
            return Err(errors::Error::not_found("resource not found"));
        }
        for change in &command.changes {
            self.apply_change(&mut transaction, record, &fields, change)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    async fn patch_atomically(
        &self,
        command: &PatchRecordCommand,
    ) -> errors::Result<()> {
        Self::validate_changes(&command.changes)?;
        let record = &command.record;
        let mut transaction = self.db.pool().begin().await?;
        let fields = self
            .lock_database_and_fields(
                &mut transaction,
                record.tenant_id(),
                record.database_id(),
            )
            .await?;
        let locked = sqlx::query_scalar::<_, String>(
            r#"
            SELECT id FROM data
            WHERE tenant_id = ? AND object_id = ? AND id = ?
            FOR UPDATE
            "#,
        )
        .bind(record.tenant_id().to_string())
        .bind(record.database_id().to_string())
        .bind(record.id().to_string())
        .fetch_optional(&mut *transaction)
        .await?;
        if locked.is_none() {
            return Err(errors::Error::not_found("resource not found"));
        }
        sqlx::query(
            r#"
            UPDATE data SET name = ?, updated_at = ?
            WHERE tenant_id = ? AND object_id = ? AND id = ?
            "#,
        )
        .bind(record.name().to_string())
        .bind(record.updated_at())
        .bind(record.tenant_id().to_string())
        .bind(record.database_id().to_string())
        .bind(record.id().to_string())
        .execute(&mut *transaction)
        .await?;
        for change in &command.changes {
            self.apply_change(&mut transaction, record, &fields, change)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    async fn delete_atomically(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        data_id: &DataId,
    ) -> errors::Result<()> {
        let result = sqlx::query(
            r#"
            DELETE FROM data
            WHERE tenant_id = ? AND object_id = ? AND id = ?
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .bind(data_id.to_string())
        .execute(self.db.pool().as_ref())
        .await?;
        if result.rows_affected() == 0 {
            return Err(errors::Error::not_found("resource not found"));
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl DataRepository for DataRepositoryImpl {
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
        let Some(row) = row else {
            return Ok(None);
        };
        let canonical = load_canonical_values_for_mode(
            self.db.as_ref(),
            tenant_id,
            database_id,
            std::slice::from_ref(&row.id),
            self.property_value_mode,
        )
        .await?;
        Ok(Some(hydrate_data_row(
            row,
            &fields,
            &canonical,
            self.property_value_mode,
        )?))
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
        let data_ids = data_rows
            .iter()
            .map(|row| row.id.clone())
            .collect::<Vec<_>>();
        let canonical = load_canonical_values_for_mode(
            self.db.as_ref(),
            tenant_id,
            id,
            &data_ids,
            self.property_value_mode,
        )
        .await?;
        let mut data_vec = vec![];
        for data_row in data_rows {
            let data = hydrate_data_row(
                data_row,
                &fields,
                &canonical,
                self.property_value_mode,
            )?;
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
        page: OffsetPage,
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
        .bind(page.items_per_page())
        .bind(page.offset())
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
        let data_ids = data_rows
            .iter()
            .map(|row| row.id.clone())
            .collect::<Vec<_>>();
        let canonical = load_canonical_values_for_mode(
            self.db.as_ref(),
            tenant_id,
            database_id,
            &data_ids,
            self.property_value_mode,
        )
        .await?;
        let mut data_vec = vec![];
        for data_row in data_rows {
            let data = hydrate_data_row(
                data_row,
                &fields,
                &canonical,
                self.property_value_mode,
            )?;
            data_vec.push(data);
        }
        let total = u32::try_from(total).map_err(|_| {
            errors::Error::internal_server_error(
                "data count exceeds the supported pagination range",
            )
        })?;
        let paginator = OffsetPaginator::new(page, total);
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
