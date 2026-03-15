use crate::DatabaseId;
use derive_getters::Getters;
use derive_new::new;
use serde::Deserialize;
use serde::Serialize;
use strum::{Display, EnumString};
use util::def_id;
use util::macros::*;
use value_object::{Identifier, Text};

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
        let typ = typ.parse::<PropertyType>()?;
        match typ {
            PropertyType::String => Ok(PropertyType::String),
            PropertyType::Integer => Ok(PropertyType::Integer),
            PropertyType::Html => Ok(PropertyType::Html),
            PropertyType::Markdown => Ok(PropertyType::Markdown),
            PropertyType::Relation(_) => Ok(PropertyType::Relation(
                serde_json::from_value(meta)
                    .map_err(errors::Error::invalid)?,
            )),
            PropertyType::Select(_) => Ok(PropertyType::Select(
                serde_json::from_value(meta)
                    .map_err(errors::Error::invalid)?,
            )),
            PropertyType::MultiSelect(_) => Ok(PropertyType::MultiSelect(
                serde_json::from_value(meta)
                    .map_err(errors::Error::invalid)?,
            )),
            PropertyType::Id(_) => Ok(PropertyType::Id(
                serde_json::from_value(meta)
                    .map_err(errors::Error::invalid)?,
            )),
            PropertyType::Location(_) => Ok(PropertyType::Location(
                serde_json::from_value(meta)
                    .map_err(errors::Error::invalid)?,
            )),
            PropertyType::Date => Ok(PropertyType::Date),
            PropertyType::Image => Ok(PropertyType::Image),
        }
    }
    pub fn get_meta(&self) -> errors::Result<serde_json::Value> {
        match self {
            PropertyType::Relation(relation) => {
                Ok(serde_json::to_value(relation)
                    .map_err(errors::Error::invalid)?)
            }
            PropertyType::Select(select) => {
                Ok(serde_json::to_value(select)
                    .map_err(errors::Error::invalid)?)
            }
            PropertyType::MultiSelect(multi_select) => {
                Ok(serde_json::to_value(multi_select)
                    .map_err(errors::Error::invalid)?)
            }
            PropertyType::Id(id) => {
                Ok(serde_json::to_value(id)
                    .map_err(errors::Error::invalid)?)
            }
            PropertyType::Location(location) => {
                Ok(serde_json::to_value(location)
                    .map_err(errors::Error::invalid)?)
            }
            PropertyType::Date => Ok(serde_json::Value::Null),
            _ => Ok(serde_json::Value::Null),
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
    /// Whether to auto-generate ID
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
