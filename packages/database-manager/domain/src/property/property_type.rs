use crate::DatabaseId;
use derive_getters::Getters;
use derive_new::new;
use serde::Deserialize;
use serde::Serialize;
use strum::{Display, EnumString};
use util::def_id;
use util::macros::*;
use value_object::{Identifier, Text};

use super::{LegacyPropertyTypeReader, PropertyConfig, PropertyTypeRef};

#[derive(Debug, Clone, Default, EnumString, Display)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum PropertyType {
    #[default]
    String,
    Integer,
    Html,
    Markdown,
    Relation(TypeRelation),
    Select(TypeSelect),
    MultiSelect(TypeMultiSelect),
    Id(TypeId),
    Location(TypeLocation),
    Date,
    Image,
}

impl PropertyType {
    pub fn from_meta(
        typ: &str,
        meta: serde_json::Value,
    ) -> errors::Result<Self> {
        LegacyPropertyTypeReader::decode_type(typ, meta)
    }

    pub fn get_meta(&self) -> errors::Result<serde_json::Value> {
        let config = self.canonical_config();
        config.handler().encode_config(&config)
    }

    /// The stable type identity for new storage and API envelopes.
    pub fn canonical_type_ref(&self) -> PropertyTypeRef {
        self.canonical_config().type_ref()
    }

    /// The typed kernel config represented by this compatibility enum.
    pub fn canonical_config(&self) -> PropertyConfig {
        self.into()
    }
}

impl From<&PropertyType> for PropertyConfig {
    fn from(value: &PropertyType) -> Self {
        match value {
            PropertyType::String => Self::String,
            PropertyType::Integer => Self::Integer,
            PropertyType::Html => Self::Html,
            PropertyType::Markdown => Self::Markdown,
            PropertyType::Relation(value) => Self::Relation(value.clone()),
            PropertyType::Select(value) => Self::Select(value.clone()),
            PropertyType::MultiSelect(value) => {
                Self::MultiSelect(value.clone())
            }
            PropertyType::Id(value) => Self::Id(value.clone()),
            PropertyType::Location(value) => Self::Location(value.clone()),
            PropertyType::Date => Self::Date,
            PropertyType::Image => Self::Image,
        }
    }
}

impl From<&PropertyConfig> for PropertyType {
    fn from(value: &PropertyConfig) -> Self {
        match value {
            PropertyConfig::String => Self::String,
            PropertyConfig::Integer => Self::Integer,
            PropertyConfig::Html => Self::Html,
            PropertyConfig::Markdown => Self::Markdown,
            PropertyConfig::Relation(value) => {
                Self::Relation(value.clone())
            }
            PropertyConfig::Select(value) => Self::Select(value.clone()),
            PropertyConfig::MultiSelect(value) => {
                Self::MultiSelect(value.clone())
            }
            PropertyConfig::Id(value) => Self::Id(value.clone()),
            PropertyConfig::Location(value) => {
                Self::Location(value.clone())
            }
            PropertyConfig::Date => Self::Date,
            PropertyConfig::Image => Self::Image,
        }
    }
}

def_id!(SelectItemId, "op_");

#[derive(Debug, Clone, Getters, Serialize, Deserialize, new)]
pub struct SelectItem {
    id: SelectItemId,
    key: Identifier,
    name: Text,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, new)]
pub struct TypeRelation {
    pub database_id: DatabaseId,
}

#[derive(Debug, Clone, Getters, Default, Serialize, Deserialize, new)]
pub struct TypeSelect {
    pub items: Vec<SelectItem>,
}

#[derive(Debug, Clone, Getters, Default, Serialize, Deserialize, new)]
pub struct TypeMultiSelect {
    pub items: Vec<SelectItem>,
}

#[derive(Debug, Clone, Getters, Default, Serialize, Deserialize, new)]
pub struct TypeId {
    /// Generate the canonical DataId when AddData accepts a new record. The
    /// caller may omit the value or send an empty value, but cannot override
    /// it. Adding the property does not backfill existing records.
    pub auto_generate: bool,
}

#[derive(Debug, Clone, Getters, Default, Serialize, Deserialize, new)]
pub struct TypeLocation {
    /// Default latitude value (optional)
    pub default_latitude: Option<f64>,
    /// Default longitude value (optional)
    pub default_longitude: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest]
    #[case(PropertyType::default(), "STRING")]
    #[case(PropertyType::String, "STRING")]
    #[case(PropertyType::Integer, "INTEGER")]
    #[case(PropertyType::Html, "HTML")]
    #[case(PropertyType::Markdown, "MARKDOWN")]
    #[case(
        PropertyType::Relation(TypeRelation::new(DatabaseId::default())),
        "RELATION"
    )]
    #[case(PropertyType::Select(TypeSelect::default()), "SELECT")]
    #[case(
        PropertyType::MultiSelect(TypeMultiSelect::default()),
        "MULTI_SELECT"
    )]
    #[case(PropertyType::Id(TypeId::default()), "ID")]
    #[case(PropertyType::Location(TypeLocation::default()), "LOCATION")]
    #[case(PropertyType::Date, "DATE")]
    #[case(PropertyType::Image, "IMAGE")]
    fn test_property_type_to_string(
        #[case] property_type: PropertyType,
        #[case] expected: &str,
    ) {
        assert_eq!(property_type.to_string(), expected.to_string());
    }
}
