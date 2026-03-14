//! HubSpot data handler implementation for Library data operations.

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;

use inbound_sync_domain::{PropertyMapping, Transform, WebhookEndpoint};

use super::event_processor::HubSpotDataHandler;
use super::payload::{HubSpotObject, ObjectType};
use crate::providers::github::LibraryDataRepository;

/// Default implementation of HubSpotDataHandler.
#[derive(Debug)]
pub struct DefaultHubSpotDataHandler {
    data_repo: Arc<dyn LibraryDataRepository>,
}

impl DefaultHubSpotDataHandler {
    /// Create a new handler with a data repository.
    pub fn new(data_repo: Arc<dyn LibraryDataRepository>) -> Self {
        Self { data_repo }
    }

    /// Map HubSpot object properties to Library properties.
    fn map_object_properties(
        object_type: ObjectType,
        object: &HubSpotObject,
        mapping: Option<&PropertyMapping>,
    ) -> HashMap<String, JsonValue> {
        let mut properties = HashMap::new();

        // Get properties from HubSpot object
        let hs_props = if let Some(obj) = object.properties.as_object() {
            obj.clone()
        } else {
            serde_json::Map::new()
        };

        // Default mappings based on object type
        if mapping.is_none() {
            match object_type {
                ObjectType::Contact => {
                    // Contact default mappings
                    if let Some(v) = hs_props.get("firstname") {
                        properties
                            .insert("first_name".to_string(), v.clone());
                    }
                    if let Some(v) = hs_props.get("lastname") {
                        properties
                            .insert("last_name".to_string(), v.clone());
                    }
                    if let Some(v) = hs_props.get("email") {
                        properties.insert("email".to_string(), v.clone());
                    }
                    if let Some(v) = hs_props.get("phone") {
                        properties.insert("phone".to_string(), v.clone());
                    }
                    if let Some(v) = hs_props.get("company") {
                        properties.insert("company".to_string(), v.clone());
                    }
                    if let Some(v) = hs_props.get("jobtitle") {
                        properties
                            .insert("job_title".to_string(), v.clone());
                    }
                    if let Some(v) = hs_props.get("lifecyclestage") {
                        properties.insert(
                            "lifecycle_stage".to_string(),
                            v.clone(),
                        );
                    }
                    if let Some(v) = hs_props.get("hs_lead_status") {
                        properties
                            .insert("lead_status".to_string(), v.clone());
                    }
                }
                ObjectType::Company => {
                    // Company default mappings
                    if let Some(v) = hs_props.get("name") {
                        properties.insert("name".to_string(), v.clone());
                    }
                    if let Some(v) = hs_props.get("domain") {
                        properties.insert("domain".to_string(), v.clone());
                    }
                    if let Some(v) = hs_props.get("industry") {
                        properties
                            .insert("industry".to_string(), v.clone());
                    }
                    if let Some(v) = hs_props.get("numberofemployees") {
                        properties.insert(
                            "employee_count".to_string(),
                            v.clone(),
                        );
                    }
                    if let Some(v) = hs_props.get("annualrevenue") {
                        properties.insert(
                            "annual_revenue".to_string(),
                            v.clone(),
                        );
                    }
                    if let Some(v) = hs_props.get("city") {
                        properties.insert("city".to_string(), v.clone());
                    }
                    if let Some(v) = hs_props.get("country") {
                        properties.insert("country".to_string(), v.clone());
                    }
                }
                ObjectType::Deal => {
                    // Deal default mappings
                    if let Some(v) = hs_props.get("dealname") {
                        properties.insert("name".to_string(), v.clone());
                    }
                    if let Some(v) = hs_props.get("amount") {
                        properties.insert("amount".to_string(), v.clone());
                    }
                    if let Some(v) = hs_props.get("dealstage") {
                        properties.insert("stage".to_string(), v.clone());
                    }
                    if let Some(v) = hs_props.get("pipeline") {
                        properties
                            .insert("pipeline".to_string(), v.clone());
                    }
                    if let Some(v) = hs_props.get("closedate") {
                        properties
                            .insert("close_date".to_string(), v.clone());
                    }
                    if let Some(v) =
                        hs_props.get("hs_deal_stage_probability")
                    {
                        properties
                            .insert("probability".to_string(), v.clone());
                    }
                }
                ObjectType::Product => {
                    // Product default mappings
                    if let Some(v) = hs_props.get("name") {
                        properties.insert("name".to_string(), v.clone());
                    }
                    if let Some(v) = hs_props.get("description") {
                        properties
                            .insert("description".to_string(), v.clone());
                    }
                    if let Some(v) = hs_props.get("price") {
                        properties.insert("price".to_string(), v.clone());
                    }
                    if let Some(v) = hs_props.get("hs_sku") {
                        properties.insert("sku".to_string(), v.clone());
                    }
                    if let Some(v) = hs_props.get("hs_cost_of_goods_sold") {
                        properties.insert("cost".to_string(), v.clone());
                    }
                }
                ObjectType::Ticket => {
                    // Ticket default mappings
                    if let Some(v) = hs_props.get("subject") {
                        properties.insert("subject".to_string(), v.clone());
                    }
                    if let Some(v) = hs_props.get("content") {
                        properties.insert("content".to_string(), v.clone());
                    }
                    if let Some(v) = hs_props.get("hs_pipeline_stage") {
                        properties.insert("stage".to_string(), v.clone());
                    }
                    if let Some(v) = hs_props.get("hs_ticket_priority") {
                        properties
                            .insert("priority".to_string(), v.clone());
                    }
                }
                ObjectType::LineItem => {
                    // LineItem default mappings
                    if let Some(v) = hs_props.get("name") {
                        properties.insert("name".to_string(), v.clone());
                    }
                    if let Some(v) = hs_props.get("quantity") {
                        properties
                            .insert("quantity".to_string(), v.clone());
                    }
                    if let Some(v) = hs_props.get("price") {
                        properties.insert("price".to_string(), v.clone());
                    }
                    if let Some(v) = hs_props.get("amount") {
                        properties.insert("amount".to_string(), v.clone());
                    }
                }
            }

            // Add common metadata
            properties.insert(
                "hubspot_id".to_string(),
                JsonValue::String(object.id.clone()),
            );
            properties.insert(
                "object_type".to_string(),
                JsonValue::String(object_type.as_str().to_string()),
            );
            properties.insert(
                "created_at".to_string(),
                JsonValue::String(object.created_at.clone()),
            );
            properties.insert(
                "updated_at".to_string(),
                JsonValue::String(object.updated_at.clone()),
            );

            return properties;
        }

        // Apply custom mapping
        let mapping = mapping.unwrap();

        for field_mapping in &mapping.static_mappings {
            // Support hubspot property names with or without prefix
            let source_key = field_mapping
                .source_field
                .strip_prefix("properties.")
                .unwrap_or(&field_mapping.source_field);

            if let Some(value) = hs_props.get(source_key) {
                let transformed =
                    if let Some(transform) = &field_mapping.transform {
                        apply_transform(value, transform)
                    } else {
                        value.clone()
                    };
                properties.insert(
                    field_mapping.target_property.clone(),
                    transformed,
                );
            }
        }

        // Apply defaults
        for (key, value) in &mapping.defaults {
            if !properties.contains_key(key) {
                properties.insert(key.clone(), value.clone());
            }
        }

        properties
    }

    /// Get name for the object based on type.
    fn get_object_name(
        object_type: ObjectType,
        object: &HubSpotObject,
    ) -> String {
        let props = object.properties.as_object();

        match object_type {
            ObjectType::Contact => {
                if let Some(props) = props {
                    let first = props
                        .get("firstname")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let last = props
                        .get("lastname")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if !first.is_empty() || !last.is_empty() {
                        return format!("{first} {last}")
                            .trim()
                            .to_string();
                    }
                    if let Some(email) =
                        props.get("email").and_then(|v| v.as_str())
                    {
                        return email.to_string();
                    }
                }
                format!("Contact {}", object.id)
            }
            ObjectType::Company => {
                if let Some(props) = props {
                    if let Some(name) =
                        props.get("name").and_then(|v| v.as_str())
                    {
                        return name.to_string();
                    }
                }
                format!("Company {}", object.id)
            }
            ObjectType::Deal => {
                if let Some(props) = props {
                    if let Some(name) =
                        props.get("dealname").and_then(|v| v.as_str())
                    {
                        return name.to_string();
                    }
                }
                format!("Deal {}", object.id)
            }
            ObjectType::Product => {
                if let Some(props) = props {
                    if let Some(name) =
                        props.get("name").and_then(|v| v.as_str())
                    {
                        return name.to_string();
                    }
                }
                format!("Product {}", object.id)
            }
            ObjectType::Ticket => {
                if let Some(props) = props {
                    if let Some(subject) =
                        props.get("subject").and_then(|v| v.as_str())
                    {
                        return subject.to_string();
                    }
                }
                format!("Ticket {}", object.id)
            }
            ObjectType::LineItem => {
                if let Some(props) = props {
                    if let Some(name) =
                        props.get("name").and_then(|v| v.as_str())
                    {
                        return name.to_string();
                    }
                }
                format!("LineItem {}", object.id)
            }
        }
    }
}

/// Apply a transform function to a value.
fn apply_transform(value: &JsonValue, transform: &Transform) -> JsonValue {
    match transform {
        Transform::SplitComma => {
            if let Some(s) = value.as_str() {
                let parts: Vec<JsonValue> = s
                    .split(',')
                    .map(|p| JsonValue::String(p.trim().to_string()))
                    .collect();
                JsonValue::Array(parts)
            } else {
                value.clone()
            }
        }
        Transform::Lowercase => {
            if let Some(s) = value.as_str() {
                JsonValue::String(s.to_lowercase())
            } else {
                value.clone()
            }
        }
        Transform::Uppercase => {
            if let Some(s) = value.as_str() {
                JsonValue::String(s.to_uppercase())
            } else {
                value.clone()
            }
        }
        Transform::ToNumber => {
            if let Some(s) = value.as_str() {
                if let Ok(n) = s.parse::<i64>() {
                    JsonValue::Number(n.into())
                } else if let Ok(f) = s.parse::<f64>() {
                    serde_json::Number::from_f64(f)
                        .map(JsonValue::Number)
                        .unwrap_or_else(|| value.clone())
                } else {
                    value.clone()
                }
            } else {
                value.clone()
            }
        }
        _ => value.clone(),
    }
}

#[async_trait]
impl HubSpotDataHandler for DefaultHubSpotDataHandler {
    async fn upsert_object(
        &self,
        endpoint: &WebhookEndpoint,
        object_type: ObjectType,
        object: &HubSpotObject,
        mapping: Option<&PropertyMapping>,
    ) -> errors::Result<String> {
        let properties =
            Self::map_object_properties(object_type, object, mapping);
        let name = Self::get_object_name(object_type, object);

        // Use description or empty content
        let content = object
            .properties
            .as_object()
            .and_then(|p| p.get("description"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Check for existing data
        let external_id =
            format!("hubspot:{}:{}", object_type.as_str(), object.id);
        let existing = self
            .data_repo
            .find_by_external_id(endpoint, &external_id)
            .await?;

        if let Some(data_id) = existing {
            self.data_repo
                .update_data(
                    endpoint, &data_id, &name, &content, properties,
                )
                .await?;
            Ok(data_id)
        } else {
            self.data_repo
                .create_data(endpoint, &name, &content, properties)
                .await
        }
    }

    async fn delete_object(
        &self,
        endpoint: &WebhookEndpoint,
        _object_type: ObjectType,
        data_id: &str,
    ) -> errors::Result<()> {
        self.data_repo.delete_data(endpoint, data_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_contact() -> HubSpotObject {
        HubSpotObject {
            id: "123".to_string(),
            properties: serde_json::json!({
                "firstname": "John",
                "lastname": "Doe",
                "email": "john@example.com",
                "phone": "+1234567890",
                "company": "Acme Inc",
                "lifecyclestage": "customer"
            }),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-02T00:00:00Z".to_string(),
            archived: false,
        }
    }

    #[test]
    fn test_map_contact_properties_default() {
        let contact = create_test_contact();
        let properties = DefaultHubSpotDataHandler::map_object_properties(
            ObjectType::Contact,
            &contact,
            None,
        );

        assert_eq!(
            properties.get("first_name").unwrap().as_str().unwrap(),
            "John"
        );
        assert_eq!(
            properties.get("last_name").unwrap().as_str().unwrap(),
            "Doe"
        );
        assert_eq!(
            properties.get("email").unwrap().as_str().unwrap(),
            "john@example.com"
        );
        assert_eq!(
            properties.get("lifecycle_stage").unwrap().as_str().unwrap(),
            "customer"
        );
    }

    #[test]
    fn test_get_contact_name() {
        let contact = create_test_contact();
        let name = DefaultHubSpotDataHandler::get_object_name(
            ObjectType::Contact,
            &contact,
        );
        assert_eq!(name, "John Doe");
    }

    #[test]
    fn test_get_deal_name() {
        let deal = HubSpotObject {
            id: "456".to_string(),
            properties: serde_json::json!({
                "dealname": "Big Sale",
                "amount": "10000"
            }),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-02T00:00:00Z".to_string(),
            archived: false,
        };

        let name = DefaultHubSpotDataHandler::get_object_name(
            ObjectType::Deal,
            &deal,
        );
        assert_eq!(name, "Big Sale");
    }
}
