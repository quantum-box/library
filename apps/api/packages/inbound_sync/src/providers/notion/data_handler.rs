//! Notion data handler implementation for Library data operations.

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;

use inbound_sync_domain::{PropertyMapping, WebhookEndpoint};

use super::event_processor::NotionDataHandler;
use crate::providers::github::LibraryDataRepository;

/// Default Notion data handler.
#[derive(Debug)]
pub struct DefaultNotionDataHandler {
    #[allow(dead_code)]
    data_repo: Arc<dyn LibraryDataRepository>,
}

impl DefaultNotionDataHandler {
    /// Create a new Notion data handler.
    pub fn new(data_repo: Arc<dyn LibraryDataRepository>) -> Self {
        Self { data_repo }
    }

    /// Map page properties to Library properties.
    fn map_page_properties(
        page: &JsonValue,
        mapping: Option<&PropertyMapping>,
    ) -> HashMap<String, JsonValue> {
        let mut properties = HashMap::new();

        // Get Notion properties
        let notion_props =
            page.get("properties").and_then(|p| p.as_object());

        if let Some(custom_mapping) = mapping {
            // Apply custom mapping
            for field_mapping in &custom_mapping.static_mappings {
                if let Some(value) = get_notion_property_value(
                    notion_props,
                    &field_mapping.source_field,
                ) {
                    let transformed = if let Some(transform) =
                        &field_mapping.transform
                    {
                        crate::providers::github::data_handler::apply_transform(
                            &value, transform,
                        )
                    } else {
                        value
                    };
                    properties.insert(
                        field_mapping.target_property.clone(),
                        transformed,
                    );
                }
            }

            // Apply defaults
            for (key, value) in &custom_mapping.defaults {
                if !properties.contains_key(key) {
                    properties.insert(key.clone(), value.clone());
                }
            }
        } else {
            // Apply default mapping
            Self::map_page_default(page, &mut properties);
        }

        properties
    }

    /// Default page property mapping.
    fn map_page_default(
        page: &JsonValue,
        properties: &mut HashMap<String, JsonValue>,
    ) {
        // Extract page ID
        if let Some(id) = page.get("id").and_then(|v| v.as_str()) {
            properties.insert(
                "notion_id".to_string(),
                JsonValue::String(id.to_string()),
            );
        }

        // Extract URL
        if let Some(url) = page.get("url").and_then(|v| v.as_str()) {
            properties.insert(
                "notion_url".to_string(),
                JsonValue::String(url.to_string()),
            );
        }

        // Extract timestamps
        if let Some(created) =
            page.get("created_time").and_then(|v| v.as_str())
        {
            properties.insert(
                "created_time".to_string(),
                JsonValue::String(created.to_string()),
            );
        }
        if let Some(edited) =
            page.get("last_edited_time").and_then(|v| v.as_str())
        {
            properties.insert(
                "last_edited_time".to_string(),
                JsonValue::String(edited.to_string()),
            );
        }

        // Extract archived status
        if let Some(archived) =
            page.get("archived").and_then(|v| v.as_bool())
        {
            properties
                .insert("archived".to_string(), JsonValue::Bool(archived));
        }

        // Extract icon
        if let Some(icon) = page.get("icon") {
            if let Some(emoji) = icon.get("emoji").and_then(|v| v.as_str())
            {
                properties.insert(
                    "icon".to_string(),
                    JsonValue::String(emoji.to_string()),
                );
            }
        }

        // Extract properties
        if let Some(notion_props) =
            page.get("properties").and_then(|p| p.as_object())
        {
            for (key, value) in notion_props {
                if let Some(extracted) =
                    extract_notion_property_value(value)
                {
                    let target_key = key.to_lowercase().replace(' ', "_");
                    properties.insert(target_key, extracted);
                }
            }
        }
    }

    /// Map database to Library properties.
    fn map_database_properties(
        database: &JsonValue,
        mapping: Option<&PropertyMapping>,
    ) -> HashMap<String, JsonValue> {
        let mut properties = HashMap::new();

        if let Some(custom_mapping) = mapping {
            // Apply custom mapping for databases
            for field_mapping in &custom_mapping.static_mappings {
                if let Some(value) =
                    get_nested_value(database, &field_mapping.source_field)
                {
                    let transformed = if let Some(transform) =
                        &field_mapping.transform
                    {
                        crate::providers::github::data_handler::apply_transform(
                            &value, transform,
                        )
                    } else {
                        value
                    };
                    properties.insert(
                        field_mapping.target_property.clone(),
                        transformed,
                    );
                }
            }

            // Apply defaults
            for (key, value) in &custom_mapping.defaults {
                if !properties.contains_key(key) {
                    properties.insert(key.clone(), value.clone());
                }
            }
        } else {
            // Default database mapping
            Self::map_database_default(database, &mut properties);
        }

        properties
    }

    /// Default database property mapping.
    fn map_database_default(
        database: &JsonValue,
        properties: &mut HashMap<String, JsonValue>,
    ) {
        // Extract database ID
        if let Some(id) = database.get("id").and_then(|v| v.as_str()) {
            properties.insert(
                "notion_id".to_string(),
                JsonValue::String(id.to_string()),
            );
        }

        // Extract URL
        if let Some(url) = database.get("url").and_then(|v| v.as_str()) {
            properties.insert(
                "notion_url".to_string(),
                JsonValue::String(url.to_string()),
            );
        }

        // Extract title
        if let Some(title_arr) =
            database.get("title").and_then(|v| v.as_array())
        {
            let title: String = title_arr
                .iter()
                .filter_map(|t| {
                    t.get("plain_text").and_then(|v| v.as_str())
                })
                .collect::<Vec<_>>()
                .join("");
            if !title.is_empty() {
                properties
                    .insert("title".to_string(), JsonValue::String(title));
            }
        }

        // Extract description
        if let Some(desc_arr) =
            database.get("description").and_then(|v| v.as_array())
        {
            let description: String = desc_arr
                .iter()
                .filter_map(|t| {
                    t.get("plain_text").and_then(|v| v.as_str())
                })
                .collect::<Vec<_>>()
                .join("");
            if !description.is_empty() {
                properties.insert(
                    "description".to_string(),
                    JsonValue::String(description),
                );
            }
        }

        // Extract icon
        if let Some(icon) = database.get("icon") {
            if let Some(emoji) = icon.get("emoji").and_then(|v| v.as_str())
            {
                properties.insert(
                    "icon".to_string(),
                    JsonValue::String(emoji.to_string()),
                );
            }
        }

        // Extract timestamps
        if let Some(created) =
            database.get("created_time").and_then(|v| v.as_str())
        {
            properties.insert(
                "created_time".to_string(),
                JsonValue::String(created.to_string()),
            );
        }
        if let Some(edited) =
            database.get("last_edited_time").and_then(|v| v.as_str())
        {
            properties.insert(
                "last_edited_time".to_string(),
                JsonValue::String(edited.to_string()),
            );
        }
    }
}

/// Get a property value from Notion properties object.
fn get_notion_property_value(
    props: Option<&serde_json::Map<String, JsonValue>>,
    field_path: &str,
) -> Option<JsonValue> {
    let props = props?;

    // Handle nested paths like "properties.Name"
    let parts: Vec<&str> = field_path.split('.').collect();

    if parts.first() == Some(&"properties") && parts.len() > 1 {
        let prop_name = parts[1];
        let prop = props.get(prop_name)?;
        return extract_notion_property_value(prop);
    }

    // Direct property access
    props.get(field_path).cloned()
}

/// Extract the actual value from a Notion property.
fn extract_notion_property_value(prop: &JsonValue) -> Option<JsonValue> {
    let prop_type = prop.get("type").and_then(|t| t.as_str())?;

    match prop_type {
        "title" => {
            let arr = prop.get("title")?.as_array()?;
            let text: String = arr
                .iter()
                .filter_map(|t| {
                    t.get("plain_text").and_then(|v| v.as_str())
                })
                .collect::<Vec<_>>()
                .join("");
            Some(JsonValue::String(text))
        }
        "rich_text" => {
            let arr = prop.get("rich_text")?.as_array()?;
            let text: String = arr
                .iter()
                .filter_map(|t| {
                    t.get("plain_text").and_then(|v| v.as_str())
                })
                .collect::<Vec<_>>()
                .join("");
            Some(JsonValue::String(text))
        }
        "number" => prop.get("number").cloned(),
        "select" => prop.get("select").and_then(|s| s.get("name")).cloned(),
        "multi_select" => {
            let arr = prop.get("multi_select")?.as_array()?;
            let values: Vec<JsonValue> =
                arr.iter().filter_map(|s| s.get("name").cloned()).collect();
            Some(JsonValue::Array(values))
        }
        "status" => prop.get("status").and_then(|s| s.get("name")).cloned(),
        "date" => {
            let date = prop.get("date")?;
            if date.is_null() {
                return None;
            }
            date.get("start").cloned()
        }
        "checkbox" => prop.get("checkbox").cloned(),
        "url" => prop.get("url").cloned(),
        "email" => prop.get("email").cloned(),
        "phone_number" => prop.get("phone_number").cloned(),
        "people" => {
            let arr = prop.get("people")?.as_array()?;
            let names: Vec<JsonValue> =
                arr.iter().filter_map(|p| p.get("name").cloned()).collect();
            Some(JsonValue::Array(names))
        }
        "files" => {
            let arr = prop.get("files")?.as_array()?;
            let urls: Vec<JsonValue> = arr
                .iter()
                .filter_map(|f| {
                    f.get("file")
                        .or_else(|| f.get("external"))
                        .and_then(|u| u.get("url"))
                        .cloned()
                })
                .collect();
            Some(JsonValue::Array(urls))
        }
        "relation" => {
            let arr = prop.get("relation")?.as_array()?;
            let ids: Vec<JsonValue> =
                arr.iter().filter_map(|r| r.get("id").cloned()).collect();
            Some(JsonValue::Array(ids))
        }
        "created_time" => prop.get("created_time").cloned(),
        "created_by" => {
            prop.get("created_by").and_then(|u| u.get("name")).cloned()
        }
        "last_edited_time" => prop.get("last_edited_time").cloned(),
        "last_edited_by" => prop
            .get("last_edited_by")
            .and_then(|u| u.get("name"))
            .cloned(),
        "formula" => {
            let formula = prop.get("formula")?;
            let formula_type =
                formula.get("type").and_then(|t| t.as_str())?;
            formula.get(formula_type).cloned()
        }
        "rollup" => {
            let rollup = prop.get("rollup")?;
            let rollup_type =
                rollup.get("type").and_then(|t| t.as_str())?;
            rollup.get(rollup_type).cloned()
        }
        _ => None,
    }
}

/// Get a nested value from JSON using dot notation.
fn get_nested_value(json: &JsonValue, path: &str) -> Option<JsonValue> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = json;

    for part in parts {
        current = current.get(part)?;
    }

    Some(current.clone())
}

#[async_trait]
impl NotionDataHandler for DefaultNotionDataHandler {
    async fn upsert_page(
        &self,
        endpoint: &WebhookEndpoint,
        page: &JsonValue,
        mapping: Option<&PropertyMapping>,
    ) -> errors::Result<()> {
        let properties = Self::map_page_properties(page, mapping);
        let page_id =
            page.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");

        tracing::info!(
            endpoint_id = %endpoint.id(),
            page_id = %page_id,
            property_count = properties.len(),
            "Upserting Notion page to Library"
        );

        // TODO: Actually save to Library database
        // self.data_repo.upsert_data(...).await?;

        Ok(())
    }

    async fn delete_page(
        &self,
        endpoint: &WebhookEndpoint,
        page_id: &str,
    ) -> errors::Result<()> {
        tracing::info!(
            endpoint_id = %endpoint.id(),
            page_id = %page_id,
            "Deleting Notion page from Library"
        );

        // TODO: Actually delete from Library database
        // self.data_repo.delete_data(...).await?;

        Ok(())
    }

    async fn upsert_database(
        &self,
        endpoint: &WebhookEndpoint,
        database: &JsonValue,
        mapping: Option<&PropertyMapping>,
    ) -> errors::Result<()> {
        let properties = Self::map_database_properties(database, mapping);
        let database_id = database
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        tracing::info!(
            endpoint_id = %endpoint.id(),
            database_id = %database_id,
            property_count = properties.len(),
            "Upserting Notion database to Library"
        );

        // TODO: Actually save to Library database
        // self.data_repo.upsert_data(...).await?;

        Ok(())
    }

    async fn delete_database(
        &self,
        endpoint: &WebhookEndpoint,
        database_id: &str,
    ) -> errors::Result<()> {
        tracing::info!(
            endpoint_id = %endpoint.id(),
            database_id = %database_id,
            "Deleting Notion database from Library"
        );

        // TODO: Actually delete from Library database
        // self.data_repo.delete_data(...).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_title_property() {
        let prop = serde_json::json!({
            "type": "title",
            "title": [
                {"plain_text": "Hello "},
                {"plain_text": "World"}
            ]
        });

        let result = extract_notion_property_value(&prop);
        assert_eq!(
            result,
            Some(JsonValue::String("Hello World".to_string()))
        );
    }

    #[test]
    fn test_extract_select_property() {
        let prop = serde_json::json!({
            "type": "select",
            "select": {
                "name": "Option 1",
                "color": "blue"
            }
        });

        let result = extract_notion_property_value(&prop);
        assert_eq!(result, Some(JsonValue::String("Option 1".to_string())));
    }

    #[test]
    fn test_extract_multi_select_property() {
        let prop = serde_json::json!({
            "type": "multi_select",
            "multi_select": [
                {"name": "Tag1"},
                {"name": "Tag2"}
            ]
        });

        let result = extract_notion_property_value(&prop);
        assert_eq!(
            result,
            Some(JsonValue::Array(vec![
                JsonValue::String("Tag1".to_string()),
                JsonValue::String("Tag2".to_string())
            ]))
        );
    }
}
