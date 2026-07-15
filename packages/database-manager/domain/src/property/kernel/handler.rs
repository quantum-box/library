use std::cmp::Ordering;
use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use value_object::Location;

use crate::{
    DataId, DatabaseId, PropertyDataValue, SelectItemId, TypeMultiSelect,
    TypeSelect,
};

use super::{
    ConversionPolicy, IndexCapabilities, PropertyConfig, PropertyKind,
    PropertyReference, PropertyTypeRef, StorageClass,
    VALUE_ENCODING_VERSION_V1, ValueEncodingVersion,
};

/// Domain operations supplied by one compile-time built-in Property Type.
#[derive(Debug, Clone, Copy)]
pub struct BuiltinPropertyTypeHandler {
    kind: PropertyKind,
}

impl BuiltinPropertyTypeHandler {
    pub const fn new(kind: PropertyKind) -> Self {
        Self { kind }
    }

    pub const fn kind(&self) -> PropertyKind {
        self.kind
    }

    pub fn type_ref(&self) -> PropertyTypeRef {
        self.kind.type_ref()
    }

    pub const fn value_encoding_version(&self) -> ValueEncodingVersion {
        VALUE_ENCODING_VERSION_V1
    }

    pub const fn storage_class(&self) -> StorageClass {
        match self.kind {
            PropertyKind::String
            | PropertyKind::Html
            | PropertyKind::Markdown
            | PropertyKind::Id
            | PropertyKind::Image => StorageClass::Text,
            PropertyKind::Integer => StorageClass::Integer,
            PropertyKind::Relation | PropertyKind::MultiSelect => {
                StorageClass::MultiReference
            }
            PropertyKind::Select => StorageClass::Reference,
            PropertyKind::Location => StorageClass::Location,
            PropertyKind::Date => StorageClass::Date,
        }
    }

    pub const fn index_capabilities(&self) -> IndexCapabilities {
        match self.kind {
            PropertyKind::String => IndexCapabilities {
                exact: true,
                range: false,
                full_text: true,
                sortable: true,
                unique: true,
                multi_value: false,
            },
            PropertyKind::Integer | PropertyKind::Date => {
                IndexCapabilities::scalar(true, false, true, true)
            }
            PropertyKind::Html | PropertyKind::Markdown => {
                IndexCapabilities::content()
            }
            PropertyKind::Relation | PropertyKind::MultiSelect => {
                IndexCapabilities::multi_reference()
            }
            PropertyKind::Select => IndexCapabilities {
                exact: true,
                range: false,
                full_text: false,
                sortable: true,
                unique: true,
                multi_value: false,
            },
            PropertyKind::Id => IndexCapabilities {
                exact: true,
                range: false,
                full_text: false,
                sortable: true,
                unique: true,
                multi_value: false,
            },
            PropertyKind::Location => IndexCapabilities {
                exact: true,
                range: false,
                full_text: false,
                sortable: false,
                unique: false,
                multi_value: false,
            },
            PropertyKind::Image => IndexCapabilities {
                exact: true,
                range: false,
                full_text: false,
                sortable: false,
                unique: false,
                multi_value: false,
            },
        }
    }

    pub fn validate_config(
        &self,
        config: &PropertyConfig,
    ) -> errors::Result<()> {
        if config.kind() != self.kind {
            return Err(errors::Error::invalid(format!(
                "property config {} does not belong to handler {}",
                config.type_ref(),
                self.type_ref()
            )));
        }

        match config {
            PropertyConfig::Select(config) => validate_select_items(config),
            PropertyConfig::MultiSelect(config) => {
                validate_multi_select_items(config)
            }
            PropertyConfig::Location(config) => {
                if let Some(latitude) = config.default_latitude {
                    Location::new(latitude, 0.0)?;
                }
                if let Some(longitude) = config.default_longitude {
                    Location::new(0.0, longitude)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn validate_value(
        &self,
        config: &PropertyConfig,
        value: &PropertyDataValue,
    ) -> errors::Result<()> {
        self.validate_config(config)?;

        match (config, value) {
            (PropertyConfig::String, PropertyDataValue::String(_))
            | (PropertyConfig::Integer, PropertyDataValue::Integer(_))
            | (PropertyConfig::Id(_), PropertyDataValue::Id(_)) => Ok(()),
            (PropertyConfig::Html, PropertyDataValue::Html(value)) => {
                validate_max_bytes(value, 3_145_728, "HTML")
            }
            (
                PropertyConfig::Markdown,
                PropertyDataValue::Markdown(value),
            ) => validate_max_bytes(value, 65_535, "Markdown"),
            (PropertyConfig::Date, PropertyDataValue::Date(value)) => {
                validate_date(value)
            }
            (PropertyConfig::Image, PropertyDataValue::Image(value)) => {
                validate_max_bytes(value, 2_048, "Image URL")
            }
            (
                PropertyConfig::Relation(relation),
                PropertyDataValue::Relation(database_id, _),
            ) if database_id == &relation.database_id => Ok(()),
            (
                PropertyConfig::Select(select),
                PropertyDataValue::Select(id),
            ) => validate_selected_id(select, id),
            (
                PropertyConfig::MultiSelect(select),
                PropertyDataValue::MultiSelect(ids),
            ) => {
                let mut unique = HashSet::new();
                for id in ids {
                    validate_multi_selected_id(select, id)?;
                    if !unique.insert(id.as_str()) {
                        return Err(errors::Error::invalid(format!(
                            "duplicate multi-select option id {id}"
                        )));
                    }
                }
                Ok(())
            }
            (
                PropertyConfig::Location(_),
                PropertyDataValue::Location(location),
            ) => {
                Location::new(location.latitude(), location.longitude())?;
                Ok(())
            }
            _ => Err(errors::Error::invalid(format!(
                "property value does not match handler {}",
                self.type_ref()
            ))),
        }
    }

    pub fn encode_config(
        &self,
        config: &PropertyConfig,
    ) -> errors::Result<Value> {
        self.validate_config(config)?;
        match config {
            PropertyConfig::String
            | PropertyConfig::Integer
            | PropertyConfig::Html
            | PropertyConfig::Markdown
            | PropertyConfig::Date
            | PropertyConfig::Image => Ok(Value::Null),
            PropertyConfig::Relation(value) => to_json(value),
            PropertyConfig::Select(value) => to_json(value),
            PropertyConfig::MultiSelect(value) => to_json(value),
            PropertyConfig::Id(value) => to_json(value),
            PropertyConfig::Location(value) => to_json(value),
        }
    }

    pub fn decode_config(
        &self,
        raw: Value,
    ) -> errors::Result<PropertyConfig> {
        let config = match self.kind {
            PropertyKind::String => {
                require_empty_config(&raw)?;
                PropertyConfig::String
            }
            PropertyKind::Integer => {
                require_empty_config(&raw)?;
                PropertyConfig::Integer
            }
            PropertyKind::Html => {
                require_empty_config(&raw)?;
                PropertyConfig::Html
            }
            PropertyKind::Markdown => {
                require_empty_config(&raw)?;
                PropertyConfig::Markdown
            }
            PropertyKind::Relation => {
                PropertyConfig::Relation(from_json(raw)?)
            }
            PropertyKind::Select => PropertyConfig::Select(from_json(raw)?),
            PropertyKind::MultiSelect => {
                PropertyConfig::MultiSelect(from_json(raw)?)
            }
            PropertyKind::Id => PropertyConfig::Id(from_json(raw)?),
            PropertyKind::Location => {
                PropertyConfig::Location(from_json(raw)?)
            }
            PropertyKind::Date => {
                require_empty_config(&raw)?;
                PropertyConfig::Date
            }
            PropertyKind::Image => {
                require_empty_config(&raw)?;
                PropertyConfig::Image
            }
        };
        self.validate_config(&config)?;
        Ok(config)
    }

    pub fn encode_value(
        &self,
        config: &PropertyConfig,
        value: &PropertyDataValue,
    ) -> errors::Result<Value> {
        self.validate_value(config, value)?;
        match value {
            PropertyDataValue::String(value)
            | PropertyDataValue::Html(value)
            | PropertyDataValue::Markdown(value)
            | PropertyDataValue::Id(value)
            | PropertyDataValue::Date(value)
            | PropertyDataValue::Image(value) => Ok(json!(value)),
            PropertyDataValue::Integer(value) => Ok(json!(value)),
            PropertyDataValue::Relation(database_id, data_ids) => {
                to_json(&RelationValue {
                    database_id: database_id.clone(),
                    data_ids: data_ids.clone(),
                })
            }
            PropertyDataValue::Select(value) => to_json(value),
            PropertyDataValue::MultiSelect(value) => to_json(value),
            PropertyDataValue::Location(value) => to_json(value),
        }
    }

    pub fn decode_value(
        &self,
        config: &PropertyConfig,
        raw: Value,
    ) -> errors::Result<PropertyDataValue> {
        self.validate_config(config)?;
        let value = match self.kind {
            PropertyKind::String => {
                PropertyDataValue::String(from_json(raw)?)
            }
            PropertyKind::Integer => {
                PropertyDataValue::Integer(from_json(raw)?)
            }
            PropertyKind::Html => PropertyDataValue::Html(from_json(raw)?),
            PropertyKind::Markdown => {
                PropertyDataValue::Markdown(from_json(raw)?)
            }
            PropertyKind::Relation => {
                let relation: RelationValue = from_json(raw)?;
                PropertyDataValue::Relation(
                    relation.database_id,
                    relation.data_ids,
                )
            }
            PropertyKind::Select => {
                PropertyDataValue::Select(from_json(raw)?)
            }
            PropertyKind::MultiSelect => {
                PropertyDataValue::MultiSelect(from_json(raw)?)
            }
            PropertyKind::Id => PropertyDataValue::Id(from_json(raw)?),
            PropertyKind::Location => {
                PropertyDataValue::Location(from_json(raw)?)
            }
            PropertyKind::Date => PropertyDataValue::Date(from_json(raw)?),
            PropertyKind::Image => {
                PropertyDataValue::Image(from_json(raw)?)
            }
        };
        self.validate_value(config, &value)?;
        Ok(value)
    }

    pub fn compare(
        &self,
        config: &PropertyConfig,
        left: &PropertyDataValue,
        right: &PropertyDataValue,
    ) -> errors::Result<Ordering> {
        self.validate_value(config, left)?;
        self.validate_value(config, right)?;

        let ordering = match (left, right) {
            (
                PropertyDataValue::String(left),
                PropertyDataValue::String(right),
            )
            | (
                PropertyDataValue::Html(left),
                PropertyDataValue::Html(right),
            )
            | (
                PropertyDataValue::Markdown(left),
                PropertyDataValue::Markdown(right),
            )
            | (PropertyDataValue::Id(left), PropertyDataValue::Id(right))
            | (
                PropertyDataValue::Date(left),
                PropertyDataValue::Date(right),
            )
            | (
                PropertyDataValue::Image(left),
                PropertyDataValue::Image(right),
            ) => left.cmp(right),
            (
                PropertyDataValue::Integer(left),
                PropertyDataValue::Integer(right),
            ) => left.cmp(right),
            (
                PropertyDataValue::Relation(left_database, left_ids),
                PropertyDataValue::Relation(right_database, right_ids),
            ) => left_database
                .cmp(right_database)
                .then_with(|| left_ids.cmp(right_ids)),
            (
                PropertyDataValue::Select(left),
                PropertyDataValue::Select(right),
            ) => left.cmp(right),
            (
                PropertyDataValue::MultiSelect(left),
                PropertyDataValue::MultiSelect(right),
            ) => left.cmp(right),
            (
                PropertyDataValue::Location(left),
                PropertyDataValue::Location(right),
            ) => left.latitude().total_cmp(&right.latitude()).then_with(
                || left.longitude().total_cmp(&right.longitude()),
            ),
            _ => unreachable!("validated values match the same handler"),
        };
        Ok(ordering)
    }

    pub fn extract_references(
        &self,
        config: &PropertyConfig,
        value: &PropertyDataValue,
    ) -> errors::Result<Vec<PropertyReference>> {
        self.validate_value(config, value)?;
        let references = match value {
            PropertyDataValue::Relation(database_id, data_ids) => data_ids
                .iter()
                .cloned()
                .map(|data_id| PropertyReference::Data {
                    database_id: database_id.clone(),
                    data_id,
                })
                .collect(),
            PropertyDataValue::Select(id) => {
                vec![PropertyReference::SelectOption(id.clone())]
            }
            PropertyDataValue::MultiSelect(ids) => ids
                .iter()
                .cloned()
                .map(PropertyReference::SelectOption)
                .collect(),
            PropertyDataValue::Image(url) if !url.is_empty() => {
                vec![PropertyReference::ExternalUrl(url.clone())]
            }
            _ => vec![],
        };
        Ok(references)
    }

    pub fn conversion_policy(
        &self,
        target: &PropertyTypeRef,
    ) -> ConversionPolicy {
        let Some(target_kind) = PropertyKind::from_key(&target.key) else {
            return ConversionPolicy::Forbidden;
        };
        if target.version != super::PROPERTY_TYPE_VERSION_V1 {
            return ConversionPolicy::Forbidden;
        }
        if target_kind == self.kind {
            return ConversionPolicy::Identity;
        }
        match (self.kind, target_kind) {
            (_, PropertyKind::String) => ConversionPolicy::ExplicitLossless,
            (PropertyKind::String, _) => {
                ConversionPolicy::ExplicitValidated
            }
            (PropertyKind::Html, PropertyKind::Markdown)
            | (PropertyKind::Markdown, PropertyKind::Html) => {
                ConversionPolicy::ExplicitLossy
            }
            (PropertyKind::Select, PropertyKind::MultiSelect)
            | (PropertyKind::MultiSelect, PropertyKind::Select) => {
                ConversionPolicy::ExplicitValidated
            }
            _ => ConversionPolicy::Forbidden,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RelationValue {
    database_id: DatabaseId,
    data_ids: Vec<DataId>,
}

fn validate_select_items(config: &TypeSelect) -> errors::Result<()> {
    validate_option_uniqueness(config.items())
}

fn validate_max_bytes(
    value: &str,
    maximum: usize,
    label: &str,
) -> errors::Result<()> {
    if value.len() > maximum {
        Err(errors::Error::invalid(format!(
            "{label} exceeds the maximum size of {maximum} bytes"
        )))
    } else {
        Ok(())
    }
}

fn validate_date(value: &str) -> errors::Result<()> {
    if value.len() != 10
        || value.as_bytes().get(4) != Some(&b'-')
        || value.as_bytes().get(7) != Some(&b'-')
    {
        return Err(errors::Error::invalid(
            "date must use the canonical YYYY-MM-DD format",
        ));
    }
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(errors::Error::invalid)?;
    Ok(())
}

fn validate_multi_select_items(
    config: &TypeMultiSelect,
) -> errors::Result<()> {
    validate_option_uniqueness(config.items())
}

fn validate_option_uniqueness(
    items: &[crate::SelectItem],
) -> errors::Result<()> {
    let mut ids = HashSet::new();
    let mut keys = HashSet::new();
    for item in items {
        if !ids.insert(item.id().as_str()) {
            return Err(errors::Error::invalid(format!(
                "duplicate select option id {}",
                item.id()
            )));
        }
        if !keys.insert(item.key().to_string()) {
            return Err(errors::Error::invalid(format!(
                "duplicate select option key {}",
                item.key()
            )));
        }
    }
    Ok(())
}

fn validate_selected_id(
    config: &TypeSelect,
    id: &SelectItemId,
) -> errors::Result<()> {
    if config.items().iter().any(|item| item.id() == id) {
        Ok(())
    } else {
        Err(errors::Error::invalid(format!(
            "unknown select option id {id}"
        )))
    }
}

fn validate_multi_selected_id(
    config: &TypeMultiSelect,
    id: &SelectItemId,
) -> errors::Result<()> {
    if config.items().iter().any(|item| item.id() == id) {
        Ok(())
    } else {
        Err(errors::Error::invalid(format!(
            "unknown multi-select option id {id}"
        )))
    }
}

fn require_empty_config(raw: &Value) -> errors::Result<()> {
    if raw.is_null() {
        Ok(())
    } else {
        Err(errors::Error::invalid(
            "this property type requires a null config",
        ))
    }
}

fn to_json(value: &impl Serialize) -> errors::Result<Value> {
    serde_json::to_value(value).map_err(errors::Error::invalid)
}

fn from_json<T: for<'de> Deserialize<'de>>(
    raw: Value,
) -> errors::Result<T> {
    serde_json::from_value(raw).map_err(errors::Error::invalid)
}
