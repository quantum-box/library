use std::str::FromStr;

use serde_json::Value;

use crate::{
    DataId, DatabaseId, PropertyDataValue, PropertyType, TypeRelation,
};

use super::PropertyKind;

/// Isolated reader for the pre-kernel SCREAMING_SNAKE_CASE type names and
/// Relation CSV representation.
///
/// New persistence code must use the versioned registry. Keeping legacy
/// parsing here prevents the historical wire shape from becoming the kernel
/// contract.
pub struct LegacyPropertyTypeReader;

impl LegacyPropertyTypeReader {
    pub fn decode_type(
        legacy_type: &str,
        legacy_config: Value,
    ) -> errors::Result<PropertyType> {
        let kind = match legacy_type {
            "STRING" => PropertyKind::String,
            "INTEGER" => PropertyKind::Integer,
            "HTML" => PropertyKind::Html,
            "MARKDOWN" => PropertyKind::Markdown,
            "RELATION" => PropertyKind::Relation,
            "SELECT" => PropertyKind::Select,
            "MULTI_SELECT" => PropertyKind::MultiSelect,
            "ID" => PropertyKind::Id,
            "LOCATION" => PropertyKind::Location,
            "DATE" => PropertyKind::Date,
            "IMAGE" => PropertyKind::Image,
            "RICH_TEXT" => PropertyKind::RichText,
            "BOOLEAN" => PropertyKind::Boolean,
            _ => {
                return Err(errors::Error::invalid(format!(
                    "unknown legacy property type {legacy_type}"
                )));
            }
        };

        // Legacy rows predate kernel validation. Reads must therefore retain
        // the historical behavior: unit-like variants ignore metadata and
        // configured variants are deserialized without applying new-write
        // invariants. Canonical encode/normalize paths remain strict.
        match kind {
            PropertyKind::String => Ok(PropertyType::String),
            PropertyKind::Integer => Ok(PropertyType::Integer),
            PropertyKind::Html => Ok(PropertyType::Html),
            PropertyKind::Markdown => Ok(PropertyType::Markdown),
            PropertyKind::Relation => Ok(PropertyType::Relation(
                serde_json::from_value(legacy_config)
                    .map_err(errors::Error::invalid)?,
            )),
            PropertyKind::Select => Ok(PropertyType::Select(
                serde_json::from_value(legacy_config)
                    .map_err(errors::Error::invalid)?,
            )),
            PropertyKind::MultiSelect => Ok(PropertyType::MultiSelect(
                serde_json::from_value(legacy_config)
                    .map_err(errors::Error::invalid)?,
            )),
            PropertyKind::Id => Ok(PropertyType::Id(
                serde_json::from_value(legacy_config)
                    .map_err(errors::Error::invalid)?,
            )),
            PropertyKind::Location => Ok(PropertyType::Location(
                serde_json::from_value(legacy_config)
                    .map_err(errors::Error::invalid)?,
            )),
            PropertyKind::Date => Ok(PropertyType::Date),
            PropertyKind::Image => Ok(PropertyType::Image),
            PropertyKind::RichText => Ok(PropertyType::RichText),
            PropertyKind::Boolean => Ok(PropertyType::Boolean),
        }
    }

    pub fn decode_value(
        legacy_value: &str,
        property_type: &PropertyType,
    ) -> errors::Result<PropertyDataValue> {
        match property_type {
            PropertyType::Relation(relation) => {
                Self::decode_relation_csv(legacy_value, relation)
            }
            _ => PropertyDataValue::new(legacy_value, property_type),
        }
    }

    fn decode_relation_csv(
        legacy_value: &str,
        relation: &TypeRelation,
    ) -> errors::Result<PropertyDataValue> {
        let mut parts = legacy_value.split(',');
        let database_id = parts
            .next()
            .filter(|id| !id.is_empty())
            .ok_or_else(invalid_relation_value)
            .and_then(|id| {
                DatabaseId::from_str(id)
                    .map_err(|_| invalid_relation_value())
            })?;

        if database_id != relation.database_id {
            return Err(invalid_relation_value());
        }

        let data_ids = parts
            .map(|id| {
                if id.is_empty() {
                    return Err(invalid_relation_value());
                }
                DataId::from_str(id).map_err(|_| invalid_relation_value())
            })
            .collect::<errors::Result<Vec<_>>>()?;

        Ok(PropertyDataValue::Relation(database_id, data_ids))
    }
}

/// Symmetric codec for the temporary `value0..value50` representation.
/// New adapters use this type instead of duplicating CSV/string formatting.
pub struct LegacyPropertyValueCodec;

impl LegacyPropertyValueCodec {
    pub fn decode(
        legacy_value: &str,
        property_type: &PropertyType,
    ) -> errors::Result<PropertyDataValue> {
        LegacyPropertyTypeReader::decode_value(legacy_value, property_type)
    }

    pub fn encode(
        value: &PropertyDataValue,
        property_type: &PropertyType,
    ) -> errors::Result<String> {
        let config = property_type.canonical_config();
        config.handler().validate_value(&config, value)?;

        Ok(match value {
            PropertyDataValue::String(value)
            | PropertyDataValue::Html(value)
            | PropertyDataValue::Markdown(value)
            | PropertyDataValue::Id(value)
            | PropertyDataValue::Date(value)
            | PropertyDataValue::Image(value) => value.clone(),
            PropertyDataValue::Integer(value) => value.to_string(),
            PropertyDataValue::Boolean(value) => value.to_string(),
            PropertyDataValue::Relation(database_id, data_ids) => {
                std::iter::once(database_id.to_string())
                    .chain(data_ids.iter().map(ToString::to_string))
                    .collect::<Vec<_>>()
                    .join(",")
            }
            PropertyDataValue::Location(value) => value.to_string(),
            PropertyDataValue::Select(value) => value.to_string(),
            PropertyDataValue::MultiSelect(values) => values
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(","),
            // The legacy column is LONGTEXT, so the document rides in it as
            // JSON text. Object keys serialize in a stable order, so an
            // unchanged document never produces a spurious row diff.
            PropertyDataValue::RichText(document) => {
                serde_json::to_string(document)
                    .map_err(errors::Error::invalid)?
            }
        })
    }
}

fn invalid_relation_value() -> errors::Error {
    errors::Error::invalid("relation value")
}
