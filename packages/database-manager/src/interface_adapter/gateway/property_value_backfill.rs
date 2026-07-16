use std::collections::HashMap;

use sha2::{Digest, Sha256};
use sqlx::{MySql, QueryBuilder};

use super::*;
use crate::usecase::{
    PropertyValueBackfillInputData, PropertyValueBackfillOutputPort,
    PropertyValueBackfillReport,
};

#[derive(Clone, Debug)]
pub struct PropertyValueBackfillGateway {
    db: Arc<Db>,
}

impl PropertyValueBackfillGateway {
    pub fn new(db: Arc<Db>) -> Arc<Self> {
        Arc::new(Self { db })
    }

    async fn load_canonical_values_for_update(
        transaction: &mut sqlx::Transaction<'_, MySql>,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        data_ids: &[String],
    ) -> errors::Result<HashMap<(String, String), PropertyValueRow>> {
        if data_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let mut query = QueryBuilder::<MySql>::new(
            "SELECT tenant_id, database_id, data_id, property_id, type_key, \
             type_version, value_encoding_version, value \
             FROM property_values WHERE tenant_id = ",
        );
        query.push_bind(tenant_id.to_string());
        query.push(" AND database_id = ");
        query.push_bind(database_id.to_string());
        query.push(" AND data_id IN (");
        {
            let mut separated = query.separated(", ");
            for data_id in data_ids {
                separated.push_bind(data_id);
            }
        }
        query.push(") FOR UPDATE");

        let rows = query
            .build_query_as::<PropertyValueRow>()
            .fetch_all(&mut **transaction)
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| {
                ((row.data_id.clone(), row.property_id.clone()), row)
            })
            .collect())
    }

    fn decode_property(field: &FieldRow) -> errors::Result<Property> {
        let legacy = field.legacy_definition()?;
        let Some(canonical) = field.canonical_definition()? else {
            return legacy.to_property();
        };

        // Backfill must never invent a canonical PropertyValue from stale
        // legacy metadata. A present canonical definition is owned by the
        // definition rollout and is usable only when this binary understands
        // it and both representations describe the same built-in config.
        canonical.config().ensure_writable()?;
        if canonical.type_ref() != legacy.type_ref()
            || canonical.raw_config()? != legacy.raw_config()?
        {
            return Err(errors::Error::conflict(
                "PropertyDefinition parity mismatch during PropertyValue backfill",
            ));
        }
        canonical.to_property()
    }

    fn expected_legacy_value(
        data: &DataRow,
        field: &FieldRow,
        property: &Property,
    ) -> errors::Result<Option<PropertyDataValue>> {
        let stored = data.get_field(field.field_num)?.unwrap_or_default();
        let projected = project_property_value(
            &data.id,
            property.property_type(),
            stored,
        )?;
        Ok(PropertyData::from_storage(property, projected)?
            .value()
            .clone())
    }

    fn add_checksum_entry(checksum: &mut [u8; 32], components: &[&[u8]]) {
        let mut digest = Sha256::new();
        for component in components {
            digest.update((component.len() as u64).to_be_bytes());
            digest.update(component);
        }
        for (target, source) in checksum.iter_mut().zip(digest.finalize()) {
            *target ^= source;
        }
    }

    fn checksum_hex(checksum: &[u8; 32]) -> String {
        let mut encoded = String::with_capacity(64);
        for byte in checksum {
            use std::fmt::Write as _;
            write!(&mut encoded, "{byte:02x}")
                .expect("writing to String cannot fail");
        }
        encoded
    }

    async fn execute_in_transaction(
        transaction: &mut sqlx::Transaction<'_, MySql>,
        input: &PropertyValueBackfillInputData<'_>,
    ) -> errors::Result<PropertyValueBackfillReport> {
        // TiDB ignores shared-lock reads unless shared lock promotion is
        // explicitly enabled. Every scan therefore takes exclusive locks on
        // the Database serialization row and every persisted Property
        // definition it decodes. This conflicts with both schema additions
        // (which lock the Database row) and updates/deletes (which write the
        // fields rows), so a report or envelope can never be produced from a
        // stale definition inside the transaction. Dry-runs roll these locks
        // back without mutating data.
        let lock = "FOR UPDATE";
        let scoped_database = sqlx::query_scalar::<_, String>(&format!(
            r#"
            SELECT id FROM objects
            WHERE tenant_id = ? AND id = ?
            {lock}
            "#
        ))
        .bind(input.tenant_id.to_string())
        .bind(input.database_id.to_string())
        .fetch_optional(&mut **transaction)
        .await?;
        if scoped_database.is_none() {
            return Err(errors::Error::not_found("resource not found"));
        }

        let fields = sqlx::query_as::<_, FieldRow>(&format!(
            r#"
            SELECT id, tenant_id, object_id, field_name, datatype,
                   datatype_meta, is_indexed, field_num, meta_json,
                   type_key, type_version, type_config
            FROM fields
            WHERE tenant_id = ? AND object_id = ?
            ORDER BY field_num ASC, id ASC
            {lock}
            "#
        ))
        .bind(input.tenant_id.to_string())
        .bind(input.database_id.to_string())
        .fetch_all(&mut **transaction)
        .await?;
        let properties = fields
            .iter()
            .map(Self::decode_property)
            .collect::<errors::Result<Vec<_>>>()?;

        let limit = u32::from(input.batch_size) + 1;
        let mut rows = if let Some(cursor) = input.after_data_id {
            sqlx::query_as::<_, DataRow>(&format!(
                "SELECT * FROM data \
                 WHERE tenant_id = ? AND object_id = ? AND id > ? \
                 ORDER BY id ASC LIMIT ? {lock}"
            ))
            .bind(input.tenant_id.to_string())
            .bind(input.database_id.to_string())
            .bind(cursor.to_string())
            .bind(limit)
            .fetch_all(&mut **transaction)
            .await?
        } else {
            sqlx::query_as::<_, DataRow>(&format!(
                "SELECT * FROM data \
                 WHERE tenant_id = ? AND object_id = ? \
                 ORDER BY id ASC LIMIT ? {lock}"
            ))
            .bind(input.tenant_id.to_string())
            .bind(input.database_id.to_string())
            .bind(limit)
            .fetch_all(&mut **transaction)
            .await?
        };
        let has_more = rows.len() > usize::from(input.batch_size);
        rows.truncate(usize::from(input.batch_size));

        let data_ids =
            rows.iter().map(|row| row.id.clone()).collect::<Vec<_>>();
        let canonical = Self::load_canonical_values_for_update(
            transaction,
            input.tenant_id,
            input.database_id,
            &data_ids,
        )
        .await?;

        let mut report = PropertyValueBackfillReport {
            scanned_records: rows.len() as u64,
            compared_values: 0,
            expected_values: 0,
            missing_values: 0,
            written_values: 0,
            matched_values: 0,
            absent_values: 0,
            opaque_values: 0,
            next_cursor: input.after_data_id.cloned(),
            complete: !has_more,
            parity_checksum: String::new(),
        };
        let mut checksum = input.checksum_seed;

        for data in &rows {
            for (field, property) in fields.iter().zip(&properties) {
                report.compared_values += 1;
                let expected =
                    Self::expected_legacy_value(data, field, property)?;
                let expected_envelope = expected
                    .as_ref()
                    .map(|value| {
                        BUILTIN_PROPERTY_TYPE_REGISTRY.encode_envelope(
                            &property.property_type().canonical_config(),
                            value,
                        )
                    })
                    .transpose()?;
                if expected_envelope.is_some() {
                    report.expected_values += 1;
                }

                let key = (data.id.clone(), field.id.clone());
                if let Some(row) = canonical.get(&key) {
                    let envelope = row.envelope()?;
                    let decoded = BUILTIN_PROPERTY_TYPE_REGISTRY
                        .decode_envelope(
                            &ResolvedPropertyConfig::Known(
                                property.property_type().canonical_config(),
                            ),
                            envelope,
                        )?;
                    match decoded {
                        PropertyValue::Opaque(_) => {
                            report.opaque_values += 1;
                            Self::add_checksum_entry(
                                &mut checksum,
                                &[
                                    data.id.as_bytes(),
                                    field.id.as_bytes(),
                                    b"opaque",
                                    row.type_key.as_bytes(),
                                    &row.type_version.to_be_bytes(),
                                    &row.value_encoding_version
                                        .to_be_bytes(),
                                    row.value.as_bytes(),
                                ],
                            );
                            // A future writer owns this row. It remains
                            // lossless and read-only to this binary.
                        }
                        PropertyValue::Known(value) => {
                            value.ensure_writable()?;
                            let matches =
                                expected.as_ref().is_some_and(|expected| {
                                    value.value() == expected
                                });
                            if !matches {
                                return Err(errors::Error::conflict(
                                    format!(
                                        "PropertyValue parity mismatch at data_id={} property_id={}",
                                        data.id, field.id
                                    ),
                                ));
                            }
                            report.matched_values += 1;
                            let encoded = serde_json::to_vec(
                                &expected_envelope
                                    .as_ref()
                                    .expect(
                                        "known match has expected value",
                                    )
                                    .raw_value,
                            )
                            .map_err(errors::Error::invalid)?;
                            Self::add_checksum_entry(
                                &mut checksum,
                                &[
                                    data.id.as_bytes(),
                                    field.id.as_bytes(),
                                    b"match",
                                    encoded.as_slice(),
                                ],
                            );
                        }
                    }
                    continue;
                }

                let Some(envelope) = expected_envelope else {
                    report.absent_values += 1;
                    Self::add_checksum_entry(
                        &mut checksum,
                        &[
                            data.id.as_bytes(),
                            field.id.as_bytes(),
                            b"absent",
                        ],
                    );
                    continue;
                };

                report.missing_values += 1;
                let encoded = serde_json::to_string(&envelope.raw_value)
                    .map_err(errors::Error::invalid)?;
                let state = if input.dry_run {
                    b"missing".as_slice()
                } else {
                    sqlx::query(
                        r#"
                        INSERT INTO property_values
                            (tenant_id, database_id, data_id, property_id,
                             type_key, type_version,
                             value_encoding_version, value)
                        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                        "#,
                    )
                    .bind(input.tenant_id.to_string())
                    .bind(input.database_id.to_string())
                    .bind(&data.id)
                    .bind(&field.id)
                    .bind(envelope.type_ref.key.as_str())
                    .bind(envelope.type_ref.version.get())
                    .bind(envelope.encoding_version.get())
                    .bind(&encoded)
                    .execute(&mut **transaction)
                    .await?;
                    report.written_values += 1;
                    b"match".as_slice()
                };
                Self::add_checksum_entry(
                    &mut checksum,
                    &[
                        data.id.as_bytes(),
                        field.id.as_bytes(),
                        state,
                        encoded.as_bytes(),
                    ],
                );
            }
            report.next_cursor = Some(data.id.parse::<DataId>()?);
        }

        report.parity_checksum = Self::checksum_hex(&checksum);
        Ok(report)
    }
}

#[async_trait::async_trait]
impl PropertyValueBackfillOutputPort for PropertyValueBackfillGateway {
    async fn execute_chunk(
        &self,
        input: &PropertyValueBackfillInputData<'_>,
    ) -> errors::Result<PropertyValueBackfillReport> {
        let mut transaction = self.db.pool().begin().await?;
        match Self::execute_in_transaction(&mut transaction, input).await {
            Ok(report) if input.dry_run => {
                transaction.rollback().await?;
                Ok(report)
            }
            Ok(report) => {
                transaction.commit().await?;
                Ok(report)
            }
            Err(error) => {
                transaction.rollback().await?;
                Err(error)
            }
        }
    }
}
