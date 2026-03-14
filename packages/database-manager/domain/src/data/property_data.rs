use super::*;

#[derive(Debug, Clone, Getters, Serialize)]
pub struct PropertyData {
    property_id: PropertyId,
    value: Option<PropertyDataValue>,
}
impl PropertyData {
    pub fn new(property: &Property, value: String) -> errors::Result<Self> {
        if value.is_empty() {
            return Ok(Self {
                property_id: property.id().clone(),
                value: None,
            });
        }
        Ok(Self {
            property_id: property.id().clone(),
            value: Some(PropertyDataValue::new(
                &value,
                property.property_type(),
            )?),
        })
    }

    pub fn string_value(&self) -> String {
        match &self.value {
            Some(value) => value.string_value(),
            None => String::new(),
        }
    }
}
