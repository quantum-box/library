use super::*;
use crate::property_definition_rollout::PropertyDefinitionStorageMode;
use crate::property_value_rollout::PropertyValueStorageMode;
use crate::relation_edge_rollout::RelationEdgeWriteMode;
use std::collections::HashSet;

const CREATE_DATA_SCOPED_SQL: &str = r#"
    INSERT INTO data (
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
    FROM objects AS scoped_database
    WHERE scoped_database.tenant_id = ?
      AND scoped_database.id = ?
"#;

const FIND_DATA_BY_ID_SQL: &str = r#"
    SELECT
        *
    FROM
        data
    WHERE
        tenant_id = ? and object_id = ? and id = ?
"#;

#[derive(Clone, Debug)]
pub struct DataRepositoryImpl {
    pub db: Arc<Db>,
    pub property_value_mode: PropertyValueStorageMode,
    pub property_definition_mode: PropertyDefinitionStorageMode,
    pub(super) relation_edge_mode: RelationEdgeWriteMode,
}

impl DataRepositoryImpl {
    pub fn new(db: Arc<Db>) -> Arc<Self> {
        Self::new_with_storage_modes(
            db,
            Default::default(),
            Default::default(),
        )
    }

    pub fn new_with_property_value_mode(
        db: Arc<Db>,
        property_value_mode: PropertyValueStorageMode,
    ) -> Arc<Self> {
        Self::new_with_storage_modes(
            db,
            property_value_mode,
            Default::default(),
        )
    }

    pub fn new_with_storage_modes(
        db: Arc<Db>,
        property_value_mode: PropertyValueStorageMode,
        property_definition_mode: PropertyDefinitionStorageMode,
    ) -> Arc<Self> {
        Arc::new(Self {
            db,
            property_value_mode,
            property_definition_mode,
            relation_edge_mode: Default::default(),
        })
    }

    /// Test/internal constructor for the dormant RelationEdge writer.
    ///
    /// Normal application factories intentionally use `new_with_storage_modes`
    /// and therefore cannot enable edge writes in this rollout slice.
    #[cfg(test)]
    pub(super) fn new_with_all_storage_modes(
        db: Arc<Db>,
        property_value_mode: PropertyValueStorageMode,
        property_definition_mode: PropertyDefinitionStorageMode,
        relation_edge_mode: RelationEdgeWriteMode,
    ) -> Arc<Self> {
        Arc::new(Self {
            db,
            property_value_mode,
            property_definition_mode,
            relation_edge_mode,
        })
    }
}

impl DataRepositoryImpl {
    pub(super) async fn lock_database_and_fields(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::MySql>,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
    ) -> errors::Result<Vec<FieldRow>> {
        // Record writes serialize with Property-schema writes at the Database
        // boundary. TiDB does not enforce shared locks by default, so a
        // `FOR SHARE` read could validate one definition and then persist a
        // value after a concurrent writer replaced it.
        let database = sqlx::query_scalar::<_, String>(
            r#"
            SELECT id FROM objects
            WHERE tenant_id = ? AND id = ?
            FOR UPDATE
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
            SELECT id, tenant_id, object_id, field_name, field_display_name, datatype,
                   datatype_meta, is_indexed, field_num, meta_json,
                   type_key, type_version, type_config
            FROM fields
            WHERE tenant_id = ? AND object_id = ?
            ORDER BY field_num ASC, id ASC
            FOR UPDATE
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .fetch_all(&mut **transaction)
        .await?)
    }

    fn persisted_property(
        &self,
        fields: &[FieldRow],
        requested: &Property,
    ) -> errors::Result<(Property, u32)> {
        let field = fields
            .iter()
            .find(|field| *requested.id() == field.id)
            .ok_or_else(|| {
                errors::Error::not_found("resource not found")
            })?;
        // A canonical definition owned by a newer binary is read-only even
        // while legacy definitions remain authoritative. Without this guard,
        // an older writer could replace its opaque canonical value with a
        // value encoded from the stale legacy type.
        field.ensure_canonical_definition_writable()?;
        let persisted = field
            .definition(self.property_definition_mode)?
            .to_property()?;
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

    pub(super) async fn ensure_existing_canonical_is_writable(
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

    pub(super) async fn apply_change(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::MySql>,
        record: &Data,
        fields: &[FieldRow],
        change: &PropertyValueChange,
    ) -> errors::Result<()> {
        let (property, field_num) =
            self.persisted_property(fields, change.property())?;
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

    /// Legacy last-writer-wins patch that still advances `record_version`.
    ///
    /// The caller's snapshot is not compared (this is not a CAS port), but
    /// every accepted write gets a fresh version so versioned readers can
    /// tell that the stored record moved. Without the bump an ordinary save
    /// changed the body while `/live/authorize` kept reporting the old
    /// version, and the Live room treated the new body as an unexplained
    /// divergence at the same version and refused every join.
    ///
    /// This path deliberately writes no `domain_outbox_events` row. Inbound
    /// provider sync writes through it so its changes cannot be re-derived
    /// as outbound deliveries and echo back to the provider; the versioned
    /// boundaries remain the only producers of Record events.
    async fn patch_atomically(
        &self,
        command: &PatchRecordCommand,
    ) -> errors::Result<RecordVersion> {
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
        // The base version comes from the locked row, not from the
        // caller's earlier read: another legacy writer may have advanced
        // it in between, and reusing a stale base would hand out a version
        // that already exists.
        let locked_version = sqlx::query_scalar::<_, u64>(
            r#"
            SELECT record_version FROM data
            WHERE tenant_id = ? AND object_id = ? AND id = ?
            FOR UPDATE
            "#,
        )
        .bind(record.tenant_id().to_string())
        .bind(record.database_id().to_string())
        .bind(record.id().to_string())
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or_else(|| errors::Error::not_found("resource not found"))?;
        let previous_version = RecordVersion::new(locked_version)?;
        // Exhaustion surfaces as the domain Conflict (409) before any value
        // is written, rather than as a wrapped version or a CHECK failure.
        let next_version = previous_version.checked_increment()?;
        // The row lock already excludes other writers; the CAS predicate
        // keeps the increment honest if that locking ever regresses, and
        // matches the versioned boundary's single-increment statement.
        let update = sqlx::query(
            r#"
            UPDATE data
            SET name = ?, updated_at = ?, record_version = ?
            WHERE tenant_id = ? AND object_id = ? AND id = ?
              AND record_version = ?
            "#,
        )
        .bind(record.name().to_string())
        .bind(record.updated_at())
        .bind(next_version.get())
        .bind(record.tenant_id().to_string())
        .bind(record.database_id().to_string())
        .bind(record.id().to_string())
        .bind(previous_version.get())
        .execute(&mut *transaction)
        .await?;
        if update.rows_affected() != 1 {
            return Err(errors::Error::internal_server_error(
                "locked Record version update affected an unexpected row count",
            ));
        }
        for change in &command.changes {
            self.apply_change(&mut transaction, record, &fields, change)
                .await?;
        }
        transaction.commit().await?;
        Ok(next_version)
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
                fields
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
            self.property_definition_mode,
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
                fields
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
                data
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
                self.property_definition_mode,
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
            DELETE FROM data
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
            DELETE FROM data
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
        // Fields are read before the page (see below); COUNT and the
        // legacy-first canonical shadow run concurrently with both. Every
        // read is still required, so a failure fails the list exactly as the
        // sequential version did.
        let pool = self.db.pool();
        let fields = async {
            Ok::<_, errors::Error>(
                sqlx::query_as::<_, FieldRow>(
                    r#"
                    SELECT
                        *
                    FROM
                        fields
                    WHERE
                        object_id = ? and tenant_id = ?
                    "#,
                )
                .bind(database_id.to_string())
                .bind(tenant_id.to_string())
                .fetch_all(pool.as_ref())
                .await?,
            )
        };
        let data_rows = async {
            Ok::<_, errors::Error>(
                sqlx::query_as::<_, DataRow>(
                    r#"
                    SELECT
                        *
                    FROM
                        data
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
                .fetch_all(pool.as_ref())
                .await?,
            )
        };
        // The SQL literal must stay byte-identical to the `.sqlx` cache entry.
        let total = async {
            let total: i64 = sqlx::query_scalar!(
                r#"
            SELECT
                COUNT(*)
            FROM
                data
            WHERE
                object_id = ? and tenant_id = ?
            "#,
                database_id.to_string(),
                tenant_id.to_string(),
            )
            .fetch_one(pool.as_ref())
            .await?;
            Ok::<_, errors::Error>(total)
        };
        // `valueN` slots are interpreted through `fields`. A property can be
        // deleted and its `field_num` reused by a new one, and nothing in a
        // data row says which property wrote a slot, so a page older than
        // the fields would show the old value under the new property. Keep
        // the original fields-then-page order for these two reads.
        let fields_then_rows = async {
            let fields = fields.await?;
            let data_rows = data_rows.await?;
            Ok::<_, errors::Error>((fields, data_rows))
        };
        let (fields, data_rows, total, canonical) = if self
            .property_value_mode
            .reads_canonical_first()
        {
            // Canonical values decide the response here, so read them
            // by the ids this page actually returned, chained after the
            // page (still concurrent with COUNT).
            let rows_with_canonical = async {
                let (fields, data_rows) = fields_then_rows.await?;
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
                Ok::<_, errors::Error>((fields, data_rows, canonical))
            };
            let ((fields, data_rows, canonical), total) =
                tokio::try_join!(rows_with_canonical, total)?;
            (fields, data_rows, total, canonical)
        } else {
            // Legacy-first reads only shadow canonical values for the
            // parity log, so they may select the page again concurrently.
            let canonical = load_canonical_values_for_page(
                self.db.as_ref(),
                tenant_id,
                database_id,
                page,
                self.property_value_mode,
            );
            let ((fields, data_rows), total, canonical) =
                tokio::try_join!(fields_then_rows, total, canonical)?;
            let page_records =
                data_rows.iter().map(PageRecord::from).collect::<Vec<_>>();
            let canonical = canonical_values_for_rows(
                self.db.as_ref(),
                tenant_id,
                database_id,
                &page_records,
                canonical,
                self.property_value_mode,
            )
            .await?;
            (fields, data_rows, total, canonical)
        };
        let mut data_vec = vec![];
        for data_row in data_rows {
            let data = hydrate_data_row(
                data_row,
                &fields,
                &canonical,
                self.property_value_mode,
                self.property_definition_mode,
            )?;
            data_vec.push(data);
        }
        let total = paged_total(total, page, data_vec.len())?;
        let paginator = OffsetPaginator::new(page, total);
        Ok((DataCollection::new(data_vec), paginator))
    }
}

/// COUNT runs concurrently with the page query and may see an older snapshot:
/// a record inserted between them can be listed while missing from the count.
/// A non-empty page proves at least `offset + returned` records exist, so the
/// total never goes below that. An empty page proves nothing and keeps COUNT.
fn paged_total(
    count: i64,
    page: OffsetPage,
    returned: usize,
) -> errors::Result<u32> {
    let out_of_range = || {
        errors::Error::internal_server_error(
            "data count exceeds the supported pagination range",
        )
    };
    let count = u32::try_from(count).map_err(|_| out_of_range())?;
    if returned == 0 {
        return Ok(count);
    }
    let returned = u32::try_from(returned).map_err(|_| out_of_range())?;
    let listed = page
        .offset()
        .checked_add(returned)
        .ok_or_else(out_of_range)?;
    Ok(count.max(listed))
}

#[cfg(test)]
mod paged_total_tests {
    use super::*;

    #[test]
    fn a_record_inserted_after_count_is_counted() {
        // Empty database: COUNT saw 0, then one record was inserted before
        // the page query returned it.
        let page = OffsetPage::new(1, 20).unwrap();
        let total = paged_total(0, page, 1).unwrap();
        assert_eq!(total, 1);
        let paginator = OffsetPaginator::new(page, total);
        assert_eq!(paginator.total_items, 1);
        assert_eq!(paginator.total_pages, 1);

        let page = OffsetPage::new(3, 10).unwrap();
        assert_eq!(paged_total(25, page, 7).unwrap(), 27);
    }

    #[test]
    fn a_consistent_count_is_kept() {
        let page = OffsetPage::new(2, 10).unwrap();
        assert_eq!(paged_total(31, page, 10).unwrap(), 31);
        assert_eq!(paged_total(15, page, 5).unwrap(), 15);
    }

    #[test]
    fn an_empty_page_keeps_the_count() {
        // Pages past the end (or an emptied database) must not invent rows.
        let page = OffsetPage::new(6, 7).unwrap();
        assert_eq!(paged_total(31, page, 0).unwrap(), 31);
        assert_eq!(
            paged_total(0, OffsetPage::new(1, 20).unwrap(), 0).unwrap(),
            0
        );
    }

    #[test]
    fn negative_or_oversized_counts_are_rejected() {
        let page = OffsetPage::new(1, 20).unwrap();
        assert!(paged_total(-1, page, 0).is_err());
        assert!(paged_total(i64::from(u32::MAX) + 1, page, 0).is_err());
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

        assert!(sql.contains("from objects as scoped_database"));
        assert!(sql.contains(
            "where scoped_database.tenant_id = ? and scoped_database.id = ?"
        ));
        assert!(!sql.contains("on duplicate key update"));
    }
}
