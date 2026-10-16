use super::*;
use crate::property_definition_rollout::PropertyDefinitionStorageMode;
use crate::property_value_rollout::PropertyValueStorageMode;
use sqlx::{MySql, QueryBuilder};
use std::collections::HashMap;

const CANONICAL_VALUE_DATA_ID_BATCH_SIZE: usize = 1_000;

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct PropertyValueRow {
    pub tenant_id: String,
    pub database_id: String,
    pub data_id: String,
    pub property_id: String,
    pub type_key: String,
    pub type_version: u16,
    pub value_encoding_version: u16,
    pub value: String,
}

impl PropertyValueRow {
    pub fn envelope(&self) -> errors::Result<EncodedPropertyValue> {
        Ok(EncodedPropertyValue {
            type_ref: PropertyTypeRef::new(
                PropertyTypeKey::new(self.type_key.clone())?,
                PropertyTypeVersion::new(self.type_version)?,
            ),
            encoding_version: ValueEncodingVersion::new(
                self.value_encoding_version,
            )?,
            raw_value: serde_json::from_str(&self.value)
                .map_err(errors::Error::invalid)?,
        })
    }
}

pub async fn load_canonical_values(
    db: &Db,
    tenant_id: &TenantId,
    database_id: &DatabaseId,
    data_ids: &[String],
) -> errors::Result<HashMap<(String, String), PropertyValueRow>> {
    if data_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut values = HashMap::new();
    for data_ids in data_ids.chunks(CANONICAL_VALUE_DATA_ID_BATCH_SIZE) {
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
        query.push(")");

        for row in query
            .build_query_as::<PropertyValueRow>()
            .fetch_all(db.pool().as_ref())
            .await?
        {
            values.insert(
                (row.data_id.clone(), row.property_id.clone()),
                row,
            );
        }
    }
    Ok(values)
}

/// Identity and version of one listed record, used to tell whether two
/// statements read the same state of a page.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PageRecord {
    pub id: String,
    pub record_version: u64,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<&DataRow> for PageRecord {
    fn from(row: &DataRow) -> Self {
        Self {
            id: row.id.clone(),
            record_version: row.record_version,
            updated_at: row.updated_at,
        }
    }
}

/// Canonical values of one `data` page, together with the page records the
/// same statement (and therefore the same snapshot) selected.
#[derive(Clone, Debug, Default)]
pub struct CanonicalPage {
    pub records: Vec<PageRecord>,
    pub values: HashMap<(String, String), PropertyValueRow>,
}

#[derive(sqlx::FromRow)]
struct CanonicalPageRow {
    page_id: String,
    page_record_version: u64,
    page_updated_at: chrono::DateTime<chrono::Utc>,
    tenant_id: Option<String>,
    database_id: Option<String>,
    data_id: Option<String>,
    property_id: Option<String>,
    type_key: Option<String>,
    type_version: Option<u16>,
    value_encoding_version: Option<u16>,
    value: Option<String>,
}

/// Load the canonical values of one `data` page without waiting for the page
/// itself, so the read can run concurrently with the legacy row query.
///
/// The page is selected with the same predicate, order and window as
/// `DataRepository::find_all_with_paging`, but in its own statement, which
/// may see a different snapshot. The selected records are returned so the
/// caller can pass the result through [`canonical_values_for_rows`] before
/// using it. Only legacy-first reads use this (for the parity shadow);
/// canonical-first reads load by the ids the page returned instead.
pub async fn load_canonical_values_for_page(
    db: &Db,
    tenant_id: &TenantId,
    database_id: &DatabaseId,
    page: OffsetPage,
    mode: PropertyValueStorageMode,
) -> errors::Result<CanonicalPage> {
    if !mode.reads_or_shadows_canonical() {
        return Ok(CanonicalPage::default());
    }
    let rows = sqlx::query_as::<_, CanonicalPageRow>(
        r#"
        SELECT
            page.id AS page_id,
            page.record_version AS page_record_version,
            page.updated_at AS page_updated_at,
            pv.tenant_id, pv.database_id, pv.data_id, pv.property_id,
            pv.type_key, pv.type_version, pv.value_encoding_version, pv.value
        FROM (
            SELECT id, record_version, updated_at
            FROM data
            WHERE object_id = ? and tenant_id = ?
            ORDER BY id ASC
            LIMIT ? OFFSET ?
        ) AS page
        LEFT JOIN property_values pv
            ON pv.tenant_id = ?
            AND pv.database_id = ?
            AND pv.data_id = page.id
        "#,
    )
    .bind(database_id.to_string())
    .bind(tenant_id.to_string())
    .bind(page.items_per_page())
    .bind(page.offset())
    .bind(tenant_id.to_string())
    .bind(database_id.to_string())
    .fetch_all(db.pool().as_ref())
    .await?;

    let mut canonical = CanonicalPage::default();
    for row in rows {
        canonical.records.push(PageRecord {
            id: row.page_id,
            record_version: row.page_record_version,
            updated_at: row.page_updated_at,
        });
        let (
            Some(tenant_id),
            Some(database_id),
            Some(data_id),
            Some(property_id),
            Some(type_key),
            Some(type_version),
            Some(value_encoding_version),
            Some(value),
        ) = (
            row.tenant_id,
            row.database_id,
            row.data_id,
            row.property_id,
            row.type_key,
            row.type_version,
            row.value_encoding_version,
            row.value,
        )
        else {
            // A page record without canonical values (LEFT JOIN miss).
            continue;
        };
        canonical.values.insert(
            (data_id.clone(), property_id.clone()),
            PropertyValueRow {
                tenant_id,
                database_id,
                data_id,
                property_id,
                type_key,
                type_version,
                value_encoding_version,
                value,
            },
        );
    }
    canonical.records.sort();
    canonical.records.dedup();
    Ok(canonical)
}

/// Accept a concurrently loaded [`CanonicalPage`] only if it saw exactly the
/// records, at exactly the versions, that the row query returned. Otherwise
/// the page shifted or a record was updated between the two statements, and
/// canonical-first reads would mix a missing (legacy fallback) or stale value
/// with the newer row, so re-read by the returned ids.
pub async fn canonical_values_for_rows(
    db: &Db,
    tenant_id: &TenantId,
    database_id: &DatabaseId,
    rows: &[PageRecord],
    canonical: CanonicalPage,
    mode: PropertyValueStorageMode,
) -> errors::Result<HashMap<(String, String), PropertyValueRow>> {
    if !mode.reads_or_shadows_canonical() {
        return Ok(HashMap::new());
    }
    let mut returned = rows.to_vec();
    returned.sort();
    returned.dedup();
    if returned == canonical.records {
        return Ok(canonical.values);
    }
    tracing::info!(
        tenant_id = %tenant_id,
        database_id = %database_id,
        "data page changed between concurrent reads; re-reading canonical values by id"
    );
    let data_ids =
        rows.iter().map(|row| row.id.clone()).collect::<Vec<_>>();
    load_canonical_values(db, tenant_id, database_id, &data_ids).await
}

pub async fn load_canonical_values_for_mode(
    db: &Db,
    tenant_id: &TenantId,
    database_id: &DatabaseId,
    data_ids: &[String],
    mode: PropertyValueStorageMode,
) -> errors::Result<HashMap<(String, String), PropertyValueRow>> {
    if !mode.reads_or_shadows_canonical() {
        return Ok(HashMap::new());
    }
    load_canonical_values(db, tenant_id, database_id, data_ids).await
}

fn legacy_property_data(
    data_row: &DataRow,
    field: &FieldRow,
    definition_mode: PropertyDefinitionStorageMode,
) -> errors::Result<PropertyData> {
    let legacy = field.legacy_definition()?;
    if definition_mode.reads_canonical_first() {
        if let Some(canonical) = field.canonical_definition()? {
            canonical.config().ensure_writable()?;
            if canonical.type_ref() != legacy.type_ref()
                || canonical.raw_config()? != legacy.raw_config()?
            {
                return Err(errors::Error::conflict(
                    "PropertyDefinition parity mismatch during legacy PropertyValue fallback",
                ));
            }
        }
    }

    let property = legacy.to_property()?;
    let raw = data_row.get_field(field.field_num)?;
    let raw = project_property_value(
        &data_row.id,
        property.property_type(),
        raw.unwrap_or_default(),
    )?;
    PropertyData::from_storage(&property, raw)
}

fn canonical_property_data(
    definition: &PropertyDefinition,
    row: &PropertyValueRow,
) -> errors::Result<PropertyData> {
    PropertyData::from_definition_envelope(definition, row.envelope()?)
}

pub fn hydrate_data_row(
    data_row: DataRow,
    fields: &[FieldRow],
    canonical: &HashMap<(String, String), PropertyValueRow>,
    mode: PropertyValueStorageMode,
    definition_mode: PropertyDefinitionStorageMode,
) -> errors::Result<Data> {
    let tenant_id = TenantId::from_str(&data_row.tenant_id)?;
    let database_id = DatabaseId::from_str(&data_row.object_id)?;
    let data_id = data_row.id.parse()?;
    let record_version = RecordVersion::new(data_row.record_version)?;
    let mut property_data = Vec::with_capacity(fields.len());

    for field in fields {
        let definition = field.definition(definition_mode)?;
        let legacy =
            legacy_property_data(&data_row, field, definition_mode);
        if !mode.reads_or_shadows_canonical() {
            property_data.push(legacy?);
            continue;
        }
        let canonical_row =
            canonical.get(&(data_row.id.clone(), field.id.clone()));
        let canonical_value = canonical_row
            .map(|row| canonical_property_data(&definition, row))
            .transpose();

        let parity = match (&legacy, canonical_row, &canonical_value) {
            (Err(_), _, _) => "legacy_decode_failure",
            (Ok(_), None, _) => "missing_canonical",
            (Ok(_), Some(_), Err(_)) => "decode_failure",
            (Ok(_), Some(_), Ok(Some(value)))
                if matches!(
                    value.envelope(),
                    Some(PropertyValue::Opaque(_))
                ) =>
            {
                "opaque"
            }
            (Ok(legacy), Some(_), Ok(Some(value))) => {
                match (legacy.value(), value.value()) {
                    (None, Some(_)) => "missing_legacy",
                    (Some(left), Some(right)) if left == right => "match",
                    (None, None) => "match",
                    _ => "mismatch",
                }
            }
            (Ok(_), Some(_), Ok(None)) => "decode_failure",
        };
        tracing::debug!(
            tenant_id = %data_row.tenant_id,
            database_id = %data_row.object_id,
            data_id = %data_row.id,
            property_id = %field.id,
            parity,
            "PropertyValue dual-read parity"
        );
        if mode.reads_canonical_first()
            && canonical_row.is_some()
            && legacy.is_err()
        {
            tracing::warn!(
                tenant_id = %data_row.tenant_id,
                database_id = %data_row.object_id,
                data_id = %data_row.id,
                property_id = %field.id,
                "ignored legacy PropertyValue decode failure during canonical-first read"
            );
        }

        let selected = if mode.reads_canonical_first() {
            match (canonical_row, canonical_value) {
                (Some(_), Ok(Some(value))) => value,
                (Some(_), Ok(None)) => {
                    return Err(errors::Error::invalid(
                        "canonical PropertyValue row did not decode",
                    ));
                }
                (Some(_), Err(error)) => return Err(error),
                (None, _) => legacy?,
            }
        } else {
            if canonical_row.is_some() && canonical_value.is_err() {
                tracing::warn!(
                    tenant_id = %data_row.tenant_id,
                    database_id = %data_row.object_id,
                    data_id = %data_row.id,
                    property_id = %field.id,
                    "ignored canonical PropertyValue during legacy-first read"
                );
            }
            legacy?
        };
        property_data.push(selected);
    }

    Ok(Data::restore(
        &data_id,
        &tenant_id,
        &database_id,
        &data_row.name,
        record_version,
        property_data,
        data_row.created_at,
        data_row.updated_at,
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn field(property_type: PropertyType) -> FieldRow {
        FieldRow {
            id: PropertyId::default().to_string(),
            tenant_id: TenantId::default().to_string(),
            object_id: DatabaseId::default().to_string(),
            field_name: "value".to_string(),
            field_display_name: "Value".to_string(),
            datatype: property_type.to_string(),
            datatype_meta: property_type.get_meta().expect("type metadata"),
            is_indexed: false,
            field_num: 0,
            meta_json: None,
            type_key: None,
            type_version: None,
            type_config: None,
        }
    }

    fn row(legacy: &str) -> DataRow {
        DataRow {
            id: DataId::default().to_string(),
            tenant_id: TenantId::default().to_string(),
            object_id: DatabaseId::default().to_string(),
            name: "record".to_string(),
            record_version: 7,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            value0: Some(legacy.to_string()),
            ..Default::default()
        }
    }

    fn canonical(
        data: &DataRow,
        field: &FieldRow,
        type_key: &str,
        value: &str,
    ) -> HashMap<(String, String), PropertyValueRow> {
        HashMap::from([(
            (data.id.clone(), field.id.clone()),
            PropertyValueRow {
                tenant_id: data.tenant_id.clone(),
                database_id: data.object_id.clone(),
                data_id: data.id.clone(),
                property_id: field.id.clone(),
                type_key: type_key.to_string(),
                type_version: 1,
                value_encoding_version: 1,
                value: value.to_string(),
            },
        )])
    }

    #[test]
    fn legacy_only_does_not_decode_a_canonical_row() {
        let row = row("legacy");
        let created_at = row.created_at;
        let updated_at = row.updated_at;
        let field = field(PropertyType::String);
        let canonical = canonical(&row, &field, "String", "not-json");

        let data = hydrate_data_row(
            row,
            std::slice::from_ref(&field),
            &canonical,
            PropertyValueStorageMode::LegacyOnly,
            PropertyDefinitionStorageMode::DualWriteLegacyRead,
        )
        .expect("legacy-only read must not depend on canonical storage");

        assert_eq!(
            data.get_property_data(&field.id.parse().expect("PropertyId"))
                .expect("value")
                .string_value(),
            "legacy"
        );
        assert_eq!(data.record_version().get(), 7);
        assert_eq!(data.created_at(), &created_at);
        assert_eq!(data.updated_at(), &updated_at);
    }

    #[test]
    fn canonical_first_ignores_corrupt_legacy_only_when_a_row_exists() {
        let canonical_row = row("not-an-integer");
        let field = field(PropertyType::Integer);
        let canonical = canonical(&canonical_row, &field, "integer", "7");

        let data = hydrate_data_row(
            canonical_row,
            std::slice::from_ref(&field),
            &canonical,
            PropertyValueStorageMode::DualWriteCanonicalRead,
            PropertyDefinitionStorageMode::DualWriteLegacyRead,
        )
        .expect("valid canonical row is authoritative");
        assert_eq!(
            data.get_property_data(&field.id.parse().expect("PropertyId"))
                .expect("value")
                .string_value(),
            "7"
        );
        assert_eq!(data.record_version().get(), 7);

        let error = hydrate_data_row(
            row("not-an-integer"),
            &[field],
            &HashMap::new(),
            PropertyValueStorageMode::DualWriteCanonicalRead,
            PropertyDefinitionStorageMode::DualWriteLegacyRead,
        )
        .expect_err(
            "missing canonical row must fall back to legacy decode",
        );
        assert!(error.is_bad_request());
    }

    #[test]
    fn hydration_rejects_zero_record_version() {
        let mut data_row = row("legacy");
        data_row.record_version = 0;

        let error = hydrate_data_row(
            data_row,
            &[],
            &HashMap::new(),
            PropertyValueStorageMode::LegacyOnly,
            PropertyDefinitionStorageMode::DualWriteLegacyRead,
        )
        .expect_err("persisted record versions must be nonzero");

        assert!(error
            .to_string()
            .contains("record version must be greater than zero"));
    }

    #[test]
    fn canonical_definition_mismatch_never_retypes_a_legacy_fallback() {
        let mut field = field(PropertyType::String);
        field.type_key = Some("integer".to_string());
        field.type_version = Some(1);
        field.type_config = Some("null".to_string());

        let error = hydrate_data_row(
            row("123"),
            std::slice::from_ref(&field),
            &HashMap::new(),
            PropertyValueStorageMode::DualWriteCanonicalRead,
            PropertyDefinitionStorageMode::DualWriteCanonicalRead,
        )
        .expect_err("missing canonical value must not retype legacy bytes");
        assert!(error
            .to_string()
            .contains("PropertyDefinition parity mismatch"));

        let data_row = row("123");
        let canonical = canonical(&data_row, &field, "integer", "123");
        let data = hydrate_data_row(
            data_row,
            std::slice::from_ref(&field),
            &canonical,
            PropertyValueStorageMode::DualWriteCanonicalRead,
            PropertyDefinitionStorageMode::DualWriteCanonicalRead,
        )
        .expect("a present canonical value is authoritative");
        assert_eq!(
            data.get_property_data(&field.id.parse().expect("PropertyId"))
                .expect("canonical value")
                .string_value(),
            "123"
        );
    }

    #[test]
    fn canonical_read_preserves_unknown_definition_and_value_as_opaque() {
        let row = row("legacy-must-not-be-retyped");
        let mut field = field(PropertyType::String);
        field.type_key = Some("future_type".to_string());
        field.type_version = Some(1);
        field.type_config = Some(r#"{"future":true}"#.to_string());
        let canonical = canonical(
            &row,
            &field,
            "future_type",
            r#"{"payload":[1,2,3]}"#,
        );

        let data = hydrate_data_row(
            row,
            std::slice::from_ref(&field),
            &canonical,
            PropertyValueStorageMode::DualWriteCanonicalRead,
            PropertyDefinitionStorageMode::DualWriteCanonicalRead,
        )
        .expect("unknown canonical definition/value must remain readable");
        let value = data
            .get_property_data(&field.id.parse().expect("PropertyId"))
            .expect("opaque value projection");
        assert!(value.value().is_none());
        assert!(matches!(
            value.envelope(),
            Some(PropertyValue::Opaque(value))
                if value.raw_value == serde_json::json!({"payload": [1, 2, 3]})
        ));
    }
}
