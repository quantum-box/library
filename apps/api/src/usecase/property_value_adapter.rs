use database_manager::domain::{
    DataId, Property, PropertyType, PropertyValueCommand, SelectItemId,
};

use super::PropertyDataValueInputData;

/// Translate the Library API input shape into the Database bounded context's
/// typed command. The legacy column representation is deliberately absent
/// from this boundary.
pub(crate) fn property_value_command(
    property: &Property,
    input: &PropertyDataValueInputData,
) -> errors::Result<PropertyValueCommand> {
    let command = match (property.property_type(), input) {
        (
            PropertyType::String,
            PropertyDataValueInputData::String(value),
        ) => string_or_clear(value, PropertyValueCommand::String),
        (
            PropertyType::Integer,
            PropertyDataValueInputData::Integer(value),
        ) => {
            if value.is_empty() {
                PropertyValueCommand::Clear
            } else {
                PropertyValueCommand::Integer(value.parse().map_err(
                    |_| {
                        errors::Error::invalid(
                            "Integer property must be an i32",
                        )
                    },
                )?)
            }
        }
        (PropertyType::Html, PropertyDataValueInputData::Html(value)) => {
            string_or_clear(value, PropertyValueCommand::Html)
        }
        (
            PropertyType::Markdown,
            PropertyDataValueInputData::Markdown(value),
        ) => string_or_clear(value, PropertyValueCommand::Markdown),
        (
            PropertyType::Relation(_),
            PropertyDataValueInputData::Relation(values),
        ) => PropertyValueCommand::Relation(
            values
                .iter()
                .map(|value| {
                    value.parse::<DataId>().map_err(|error| {
                        errors::Error::invalid(error.to_string())
                    })
                })
                .collect::<errors::Result<Vec<_>>>()?,
        ),
        (
            PropertyType::Select(_),
            PropertyDataValueInputData::Select(value),
        ) => {
            if value.is_empty() {
                PropertyValueCommand::Clear
            } else {
                PropertyValueCommand::Select(value.parse()?)
            }
        }
        (
            PropertyType::MultiSelect(_),
            PropertyDataValueInputData::MultiSelect(values),
        ) => {
            if values.is_empty() {
                PropertyValueCommand::Clear
            } else {
                PropertyValueCommand::MultiSelect(
                    values
                        .iter()
                        .map(|value| {
                            value.parse::<SelectItemId>().map_err(|error| {
                                errors::Error::invalid(error.to_string())
                            })
                        })
                        .collect::<errors::Result<Vec<_>>>()?,
                )
            }
        }
        (
            PropertyType::Id(_),
            PropertyDataValueInputData::String(value),
        ) => string_or_clear(value, PropertyValueCommand::Id),
        (
            PropertyType::Location(_),
            PropertyDataValueInputData::Location(value),
        ) => PropertyValueCommand::Location(value.clone()),
        (PropertyType::Date, PropertyDataValueInputData::Date(value)) => {
            string_or_clear(value, PropertyValueCommand::Date)
        }
        (PropertyType::Image, PropertyDataValueInputData::Image(value)) => {
            string_or_clear(value, PropertyValueCommand::Image)
        }
        _ => {
            return Err(errors::Error::invalid(format!(
                "property value input does not match Property {}",
                property.id()
            )));
        }
    };
    Ok(command)
}

fn string_or_clear(
    value: &str,
    command: impl FnOnce(String) -> PropertyValueCommand,
) -> PropertyValueCommand {
    if value.is_empty() {
        PropertyValueCommand::Clear
    } else {
        command(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use database_manager::domain::{DatabaseId, PropertyId, TypeRelation};
    use value_object::TenantId;

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
    fn empty_scalar_means_clear() {
        assert_eq!(
            property_value_command(
                &property(PropertyType::String),
                &PropertyDataValueInputData::String(String::new()),
            )
            .expect("command"),
            PropertyValueCommand::Clear
        );
    }

    #[test]
    fn empty_relation_remains_an_explicit_empty_set() {
        let command = property_value_command(
            &property(PropertyType::Relation(TypeRelation::new(
                DatabaseId::default(),
            ))),
            &PropertyDataValueInputData::Relation(vec![]),
        )
        .expect("command");

        assert_eq!(command, PropertyValueCommand::Relation(vec![]));
    }
}
