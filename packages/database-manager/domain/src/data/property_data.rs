use super::*;

#[derive(Debug, Clone, Getters, Serialize)]
pub struct PropertyData {
    property_id: PropertyId,
    value: Option<PropertyDataValue>,
}
impl PropertyData {
    /// Build PropertyData from a command input.
    ///
    /// Relation input contains only target DataIds. Its target DatabaseId is
    /// derived from the Property's configured TypeRelation.
    pub fn new(property: &Property, value: String) -> errors::Result<Self> {
        let empty_value_is_none =
            !matches!(property.property_type(), PropertyType::Relation(_));
        Self::from_value(
            property,
            value,
            empty_value_is_none,
            PropertyDataValue::new,
        )
    }

    /// Restore PropertyData from the legacy `value0..value50` storage format.
    pub fn from_storage(
        property: &Property,
        value: String,
    ) -> errors::Result<Self> {
        Self::from_value(
            property,
            value,
            true,
            PropertyDataValue::from_storage,
        )
    }

    fn from_value(
        property: &Property,
        value: String,
        empty_value_is_none: bool,
        parse: fn(&str, &PropertyType) -> errors::Result<PropertyDataValue>,
    ) -> errors::Result<Self> {
        if empty_value_is_none && value.is_empty() {
            return Ok(Self {
                property_id: property.id().clone(),
                value: None,
            });
        }
        Ok(Self {
            property_id: property.id().clone(),
            value: Some(parse(&value, property.property_type())?),
        })
    }

    pub fn string_value(&self) -> String {
        match &self.value {
            Some(value) => value.string_value(),
            None => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_relation_command_keeps_the_configured_database() {
        let target_database = DatabaseId::default();
        let property = Property::new(
            &PropertyId::default(),
            &TenantId::default(),
            &DatabaseId::default(),
            "relation",
            &PropertyType::Relation(TypeRelation::new(
                target_database.clone(),
            )),
            false,
            0,
        );

        let data = PropertyData::new(&property, String::new())
            .expect("an empty Relation target set is valid");

        assert_eq!(data.string_value(), target_database.to_string());
    }

    #[test]
    fn empty_legacy_storage_remains_an_absent_value() {
        let property = Property::new(
            &PropertyId::default(),
            &TenantId::default(),
            &DatabaseId::default(),
            "relation",
            &PropertyType::Relation(TypeRelation::new(
                DatabaseId::default(),
            )),
            false,
            0,
        );

        let data = PropertyData::from_storage(&property, String::new())
            .expect("an empty legacy column represents no value");

        assert!(data.value().is_none());
    }
}
