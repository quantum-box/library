use super::*;
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
    property: &Property,
    field_num: u32,
) -> errors::Result<PropertyData> {
    let raw = data_row.get_field(field_num)?;
    let raw = project_property_value(
        &data_row.id,
        property.property_type(),
        raw.unwrap_or_default(),
    )?;
    PropertyData::from_storage(property, raw)
}

fn canonical_property_data(
    property: &Property,
    row: &PropertyValueRow,
) -> errors::Result<PropertyData> {
    PropertyData::from_envelope(property, row.envelope()?)
}

pub fn hydrate_data_row(
    data_row: DataRow,
    fields: &[FieldRow],
    canonical: &HashMap<(String, String), PropertyValueRow>,
    mode: PropertyValueStorageMode,
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
        let property: Property = field.clone().into();
        let legacy =
            legacy_property_data(&data_row, &property, field.field_num);
        if !mode.reads_or_shadows_canonical() {
            data.add_property_data(legacy?)?;
            continue;
        }
        let canonical_row =
            canonical.get(&(data_row.id.clone(), field.id.clone()));
        let canonical_value = canonical_row
            .map(|row| canonical_property_data(&property, row))
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
        data.add_property_data(selected)?;
    }

    Ok(data)
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
            datatype: property_type.to_string(),
            datatype_meta: property_type.get_meta().expect("type metadata"),
            is_indexed: false,
            field_num: 0,
            meta_json: None,
        }
    }

    fn row(legacy: &str) -> DataRow {
        DataRow {
            id: DataId::default().to_string(),
            tenant_id: TenantId::default().to_string(),
            object_id: DatabaseId::default().to_string(),
            name: "record".to_string(),
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
        let field = field(PropertyType::String);
        let canonical = canonical(&row, &field, "String", "not-json");

        let data = hydrate_data_row(
            row,
            std::slice::from_ref(&field),
            &canonical,
            PropertyValueStorageMode::LegacyOnly,
        )
        .expect("legacy-only read must not depend on canonical storage");

        assert_eq!(
            data.get_property_data(&field.id.parse().expect("PropertyId"))
                .expect("value")
                .string_value(),
            "legacy"
        );
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
        )
        .expect("valid canonical row is authoritative");
        assert_eq!(
            data.get_property_data(&field.id.parse().expect("PropertyId"))
                .expect("value")
                .string_value(),
            "7"
        );

        let error = hydrate_data_row(
            row("not-an-integer"),
            &[field],
            &HashMap::new(),
            PropertyValueStorageMode::DualWriteCanonicalRead,
        )
        .expect_err(
            "missing canonical row must fall back to legacy decode",
        );
        assert!(error.is_bad_request());
    }
}
