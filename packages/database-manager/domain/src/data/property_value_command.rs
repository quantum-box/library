use super::*;
use value_object::Location;

/// Typed command boundary for Property values.
///
/// This is deliberately different from both the public GraphQL shape and the
/// temporary legacy column encoding. In particular, Relation commands contain
/// only target DataIds; their DatabaseId comes from the Property definition.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValueCommand {
    Clear,
    String(String),
    Integer(i32),
    Html(String),
    Markdown(String),
    Relation(Vec<DataId>),
    Select(SelectItemId),
    MultiSelect(Vec<SelectItemId>),
    Id(String),
    Location(Location),
    Date(String),
    Image(String),
}

impl PropertyValueCommand {
    /// Compatibility adapter for callers that still receive a single textual
    /// value. New structured API boundaries should construct typed variants
    /// directly instead of using this function.
    pub fn from_legacy_command_text(
        property: &Property,
        value: &str,
    ) -> errors::Result<Self> {
        if value.is_empty()
            && !matches!(
                property.property_type(),
                PropertyType::Relation(_)
            )
        {
            return Ok(Self::Clear);
        }
        // This adapter receives the historical *command* representation.
        // Relation commands contain target DataIds only; the configured
        // Property owns the target DatabaseId. The legacy storage codec is
        // intentionally not used here because persisted Relation CSV starts
        // with that DatabaseId.
        let value =
            PropertyDataValue::new(value, property.property_type())?;
        Ok(Self::from_value(value))
    }

    pub fn from_value(value: PropertyDataValue) -> Self {
        match value {
            PropertyDataValue::String(value) => Self::String(value),
            PropertyDataValue::Integer(value) => Self::Integer(value),
            PropertyDataValue::Html(value) => Self::Html(value),
            PropertyDataValue::Markdown(value) => Self::Markdown(value),
            PropertyDataValue::Relation(_, values) => {
                Self::Relation(values)
            }
            PropertyDataValue::Select(value) => Self::Select(value),
            PropertyDataValue::MultiSelect(values) => {
                Self::MultiSelect(values)
            }
            PropertyDataValue::Id(value) => Self::Id(value),
            PropertyDataValue::Location(value) => Self::Location(value),
            PropertyDataValue::Date(value) => Self::Date(value),
            PropertyDataValue::Image(value) => Self::Image(value),
        }
    }

    pub fn into_value(
        self,
        property: &Property,
    ) -> errors::Result<Option<PropertyDataValue>> {
        let value = match (property.property_type(), self) {
            (_, Self::Clear) => return Ok(None),
            (PropertyType::String, Self::String(value)) => {
                PropertyDataValue::String(value)
            }
            (PropertyType::Integer, Self::Integer(value)) => {
                PropertyDataValue::Integer(value)
            }
            (PropertyType::Html, Self::Html(value)) => {
                PropertyDataValue::Html(value)
            }
            (PropertyType::Markdown, Self::Markdown(value)) => {
                PropertyDataValue::Markdown(value)
            }
            (PropertyType::Relation(config), Self::Relation(data_ids)) => {
                PropertyDataValue::Relation(
                    config.database_id.clone(),
                    data_ids,
                )
            }
            (PropertyType::Select(_), Self::Select(value)) => {
                PropertyDataValue::Select(value)
            }
            (PropertyType::MultiSelect(_), Self::MultiSelect(values)) => {
                PropertyDataValue::MultiSelect(values)
            }
            (PropertyType::Id(_), Self::Id(value)) => {
                PropertyDataValue::Id(value)
            }
            (PropertyType::Location(_), Self::Location(value)) => {
                PropertyDataValue::Location(value)
            }
            (PropertyType::Date, Self::Date(value)) => {
                PropertyDataValue::Date(value)
            }
            (PropertyType::Image, Self::Image(value)) => {
                PropertyDataValue::Image(value)
            }
            _ => {
                return Err(errors::Error::invalid(format!(
                    "property value command does not match Property {}",
                    property.id()
                )));
            }
        };

        // The legacy columns encode absence as an empty string. While both
        // stores are active, normalize values that have no distinct legacy
        // representation so legacy-read and canonical-read cannot disagree.
        // An empty Relation remains set because its legacy representation
        // contains the configured target DatabaseId.
        if value.string_value().is_empty() {
            return Ok(None);
        }

        let config = property.property_type().canonical_config();
        config.handler().validate_value(&config, &value)?;
        Ok(Some(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn property(property_type: PropertyType) -> Property {
        Property::new(
            &PropertyId::default(),
            &TenantId::default(),
            &DatabaseId::default(),
            "value",
            &property_type,
            false,
            0,
        )
    }

    #[test]
    fn relation_uses_the_configured_database() {
        let target = DatabaseId::default();
        let data_id = DataId::default();
        let property = property(PropertyType::Relation(TypeRelation::new(
            target.clone(),
        )));

        let value = PropertyValueCommand::Relation(vec![data_id.clone()])
            .into_value(&property)
            .expect("typed Relation command")
            .expect("set value");

        assert_eq!(
            value,
            PropertyDataValue::Relation(target, vec![data_id])
        );
    }

    #[test]
    fn clear_is_distinct_from_an_empty_relation() {
        let property = property(PropertyType::Relation(TypeRelation::new(
            DatabaseId::default(),
        )));

        assert!(
            PropertyValueCommand::Clear
                .into_value(&property)
                .expect("clear")
                .is_none()
        );
        assert!(matches!(
            PropertyValueCommand::Relation(vec![])
                .into_value(&property)
                .expect("empty Relation"),
            Some(PropertyDataValue::Relation(_, values)) if values.is_empty()
        ));
    }

    #[test]
    fn legacy_unrepresentable_empty_values_normalize_to_clear() {
        for (property_type, command) in [
            (
                PropertyType::String,
                PropertyValueCommand::String(String::new()),
            ),
            (
                PropertyType::Html,
                PropertyValueCommand::Html(String::new()),
            ),
            (
                PropertyType::Markdown,
                PropertyValueCommand::Markdown(String::new()),
            ),
            (
                PropertyType::MultiSelect(TypeMultiSelect::default()),
                PropertyValueCommand::MultiSelect(vec![]),
            ),
            (
                PropertyType::Id(TypeId::default()),
                PropertyValueCommand::Id(String::new()),
            ),
            (
                PropertyType::Date,
                PropertyValueCommand::Date(String::new()),
            ),
            (
                PropertyType::Image,
                PropertyValueCommand::Image(String::new()),
            ),
        ] {
            assert!(
                command
                    .into_value(&property(property_type))
                    .expect("empty legacy encoding is normalized")
                    .is_none()
            );
        }
    }

    #[test]
    fn legacy_command_text_relation_uses_the_command_representation() {
        let target_database = DatabaseId::default();
        let target_data = DataId::default();
        let property = property(PropertyType::Relation(TypeRelation::new(
            target_database,
        )));

        let command = PropertyValueCommand::from_legacy_command_text(
            &property,
            target_data.as_str(),
        )
        .expect("Relation command contains target DataIds only");

        assert_eq!(
            command,
            PropertyValueCommand::Relation(vec![target_data])
        );
    }

    #[test]
    fn mismatched_command_is_rejected() {
        let error = PropertyValueCommand::Integer(1)
            .into_value(&property(PropertyType::String))
            .expect_err("Integer cannot be written to String");

        assert!(error.is_bad_request());
    }
}
