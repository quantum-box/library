//! Square data handler implementation for Library data operations.

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;

use inbound_sync_domain::{PropertyMapping, Transform, WebhookEndpoint};

use super::event_processor::SquareDataHandler;
use super::payload::SquareObjectType;
use crate::providers::github::LibraryDataRepository;

/// Default implementation of SquareDataHandler.
#[derive(Debug)]
pub struct DefaultSquareDataHandler {
    data_repo: Arc<dyn LibraryDataRepository>,
}

impl DefaultSquareDataHandler {
    /// Create a new handler with a data repository.
    pub fn new(data_repo: Arc<dyn LibraryDataRepository>) -> Self {
        Self { data_repo }
    }

    /// Map Square object properties to Library properties.
    fn map_object_properties(
        object_type: SquareObjectType,
        object: &JsonValue,
        mapping: Option<&PropertyMapping>,
    ) -> HashMap<String, JsonValue> {
        let mut properties = HashMap::new();

        // Default mappings based on object type
        if mapping.is_none() {
            match object_type {
                SquareObjectType::Catalog => {
                    Self::map_catalog_default(object, &mut properties);
                }
                SquareObjectType::CatalogItem => {
                    Self::map_catalog_item_default(object, &mut properties);
                }
                SquareObjectType::CatalogCategory => {
                    Self::map_catalog_category_default(
                        object,
                        &mut properties,
                    );
                }
                SquareObjectType::CatalogItemVariation => {
                    Self::map_catalog_variation_default(
                        object,
                        &mut properties,
                    );
                }
                SquareObjectType::CatalogModifier => {
                    Self::map_catalog_modifier_default(
                        object,
                        &mut properties,
                    );
                }
                SquareObjectType::CatalogTax => {
                    Self::map_catalog_tax_default(object, &mut properties);
                }
                SquareObjectType::CatalogDiscount => {
                    Self::map_catalog_discount_default(
                        object,
                        &mut properties,
                    );
                }
                SquareObjectType::Customer => {
                    Self::map_customer_default(object, &mut properties);
                }
                SquareObjectType::Order => {
                    Self::map_order_default(object, &mut properties);
                }
                SquareObjectType::Payment => {
                    Self::map_payment_default(object, &mut properties);
                }
                SquareObjectType::Inventory => {
                    Self::map_inventory_default(object, &mut properties);
                }
                SquareObjectType::Subscription => {
                    Self::map_subscription_default(object, &mut properties);
                }
                SquareObjectType::Invoice => {
                    Self::map_invoice_default(object, &mut properties);
                }
            }

            // Add common metadata
            if let Some(id) = object.get("id").and_then(|v| v.as_str()) {
                properties.insert(
                    "square_id".to_string(),
                    JsonValue::String(id.to_string()),
                );
            }
            properties.insert(
                "object_type".to_string(),
                JsonValue::String(object_type.as_str().to_string()),
            );
            if let Some(updated_at) =
                object.get("updated_at").and_then(|v| v.as_str())
            {
                properties.insert(
                    "updated_at".to_string(),
                    JsonValue::String(updated_at.to_string()),
                );
            }
            if let Some(created_at) =
                object.get("created_at").and_then(|v| v.as_str())
            {
                properties.insert(
                    "created_at".to_string(),
                    JsonValue::String(created_at.to_string()),
                );
            }

            return properties;
        }

        // Apply custom mapping
        let mapping = mapping.unwrap();

        for field_mapping in &mapping.static_mappings {
            if let Some(value) =
                get_nested_value(object, &field_mapping.source_field)
            {
                let transformed =
                    if let Some(transform) = &field_mapping.transform {
                        apply_transform(&value, transform)
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
        for (key, value) in &mapping.defaults {
            if !properties.contains_key(key) {
                properties.insert(key.clone(), value.clone());
            }
        }

        properties
    }

    fn map_catalog_default(
        object: &JsonValue,
        properties: &mut HashMap<String, JsonValue>,
    ) {
        if let Some(v) = object.get("catalog_version") {
            properties.insert("catalog_version".to_string(), v.clone());
        }
    }

    fn map_catalog_item_default(
        object: &JsonValue,
        properties: &mut HashMap<String, JsonValue>,
    ) {
        // Extract from item_data if present
        let item_data = object.get("item_data");

        if let Some(data) = item_data {
            if let Some(v) = data.get("name") {
                properties.insert("name".to_string(), v.clone());
            }
            if let Some(v) = data.get("description") {
                properties.insert("description".to_string(), v.clone());
            }
            if let Some(v) = data.get("category_id") {
                properties.insert("category_id".to_string(), v.clone());
            }
            if let Some(v) = data.get("product_type") {
                properties.insert("product_type".to_string(), v.clone());
            }
            if let Some(v) = data.get("visibility") {
                properties.insert("visibility".to_string(), v.clone());
            }
            if let Some(variations) = data.get("variations") {
                properties
                    .insert("variations".to_string(), variations.clone());
            }
            if let Some(image_ids) = data.get("image_ids") {
                properties
                    .insert("image_ids".to_string(), image_ids.clone());
            }
        }

        if let Some(v) = object.get("is_deleted") {
            properties.insert("is_deleted".to_string(), v.clone());
        }
    }

    fn map_catalog_category_default(
        object: &JsonValue,
        properties: &mut HashMap<String, JsonValue>,
    ) {
        let category_data = object.get("category_data");

        if let Some(data) = category_data {
            if let Some(v) = data.get("name") {
                properties.insert("name".to_string(), v.clone());
            }
        }

        if let Some(v) = object.get("is_deleted") {
            properties.insert("is_deleted".to_string(), v.clone());
        }
    }

    fn map_catalog_variation_default(
        object: &JsonValue,
        properties: &mut HashMap<String, JsonValue>,
    ) {
        let variation_data = object.get("item_variation_data");

        if let Some(data) = variation_data {
            if let Some(v) = data.get("name") {
                properties.insert("name".to_string(), v.clone());
            }
            if let Some(v) = data.get("sku") {
                properties.insert("sku".to_string(), v.clone());
            }
            if let Some(v) = data.get("item_id") {
                properties.insert("item_id".to_string(), v.clone());
            }
            if let Some(v) = data.get("pricing_type") {
                properties.insert("pricing_type".to_string(), v.clone());
            }
            if let Some(price) = data.get("price_money") {
                if let Some(amount) = price.get("amount") {
                    properties
                        .insert("price_amount".to_string(), amount.clone());
                }
                if let Some(currency) = price.get("currency") {
                    properties
                        .insert("currency".to_string(), currency.clone());
                }
            }
        }
    }

    fn map_catalog_modifier_default(
        object: &JsonValue,
        properties: &mut HashMap<String, JsonValue>,
    ) {
        let modifier_data = object.get("modifier_data");

        if let Some(data) = modifier_data {
            if let Some(v) = data.get("name") {
                properties.insert("name".to_string(), v.clone());
            }
            if let Some(price) = data.get("price_money") {
                if let Some(amount) = price.get("amount") {
                    properties
                        .insert("price_amount".to_string(), amount.clone());
                }
                if let Some(currency) = price.get("currency") {
                    properties
                        .insert("currency".to_string(), currency.clone());
                }
            }
        }
    }

    fn map_catalog_tax_default(
        object: &JsonValue,
        properties: &mut HashMap<String, JsonValue>,
    ) {
        let tax_data = object.get("tax_data");

        if let Some(data) = tax_data {
            if let Some(v) = data.get("name") {
                properties.insert("name".to_string(), v.clone());
            }
            if let Some(v) = data.get("calculation_phase") {
                properties
                    .insert("calculation_phase".to_string(), v.clone());
            }
            if let Some(v) = data.get("inclusion_type") {
                properties.insert("inclusion_type".to_string(), v.clone());
            }
            if let Some(v) = data.get("percentage") {
                properties.insert("percentage".to_string(), v.clone());
            }
        }
    }

    fn map_catalog_discount_default(
        object: &JsonValue,
        properties: &mut HashMap<String, JsonValue>,
    ) {
        let discount_data = object.get("discount_data");

        if let Some(data) = discount_data {
            if let Some(v) = data.get("name") {
                properties.insert("name".to_string(), v.clone());
            }
            if let Some(v) = data.get("discount_type") {
                properties.insert("discount_type".to_string(), v.clone());
            }
            if let Some(v) = data.get("percentage") {
                properties.insert("percentage".to_string(), v.clone());
            }
            if let Some(amount) = data.get("amount_money") {
                if let Some(amt) = amount.get("amount") {
                    properties.insert("amount".to_string(), amt.clone());
                }
                if let Some(currency) = amount.get("currency") {
                    properties
                        .insert("currency".to_string(), currency.clone());
                }
            }
        }
    }

    fn map_customer_default(
        object: &JsonValue,
        properties: &mut HashMap<String, JsonValue>,
    ) {
        if let Some(v) = object.get("given_name") {
            properties.insert("given_name".to_string(), v.clone());
        }
        if let Some(v) = object.get("family_name") {
            properties.insert("family_name".to_string(), v.clone());
        }
        if let Some(v) = object.get("email_address") {
            properties.insert("email".to_string(), v.clone());
        }
        if let Some(v) = object.get("phone_number") {
            properties.insert("phone".to_string(), v.clone());
        }
        if let Some(v) = object.get("company_name") {
            properties.insert("company_name".to_string(), v.clone());
        }
        if let Some(v) = object.get("nickname") {
            properties.insert("nickname".to_string(), v.clone());
        }
        if let Some(address) = object.get("address") {
            if let Some(line1) = address.get("address_line_1") {
                properties
                    .insert("address_line_1".to_string(), line1.clone());
            }
            if let Some(line2) = address.get("address_line_2") {
                properties
                    .insert("address_line_2".to_string(), line2.clone());
            }
            if let Some(city) = address.get("locality") {
                properties.insert("city".to_string(), city.clone());
            }
            if let Some(state) =
                address.get("administrative_district_level_1")
            {
                properties.insert("state".to_string(), state.clone());
            }
            if let Some(postal) = address.get("postal_code") {
                properties
                    .insert("postal_code".to_string(), postal.clone());
            }
            if let Some(country) = address.get("country") {
                properties.insert("country".to_string(), country.clone());
            }
        }
    }

    fn map_order_default(
        object: &JsonValue,
        properties: &mut HashMap<String, JsonValue>,
    ) {
        if let Some(v) = object.get("state") {
            properties.insert("state".to_string(), v.clone());
        }
        if let Some(v) = object.get("location_id") {
            properties.insert("location_id".to_string(), v.clone());
        }
        if let Some(v) = object.get("customer_id") {
            properties.insert("customer_id".to_string(), v.clone());
        }
        if let Some(line_items) = object.get("line_items") {
            properties.insert("line_items".to_string(), line_items.clone());
        }
        if let Some(total) = object.get("total_money") {
            if let Some(amount) = total.get("amount") {
                properties
                    .insert("total_amount".to_string(), amount.clone());
            }
            if let Some(currency) = total.get("currency") {
                properties.insert("currency".to_string(), currency.clone());
            }
        }
        if let Some(net) = object.get("net_amounts") {
            properties.insert("net_amounts".to_string(), net.clone());
        }
    }

    fn map_payment_default(
        object: &JsonValue,
        properties: &mut HashMap<String, JsonValue>,
    ) {
        if let Some(v) = object.get("status") {
            properties.insert("status".to_string(), v.clone());
        }
        if let Some(v) = object.get("order_id") {
            properties.insert("order_id".to_string(), v.clone());
        }
        if let Some(v) = object.get("customer_id") {
            properties.insert("customer_id".to_string(), v.clone());
        }
        if let Some(v) = object.get("location_id") {
            properties.insert("location_id".to_string(), v.clone());
        }
        if let Some(v) = object.get("source_type") {
            properties.insert("source_type".to_string(), v.clone());
        }
        if let Some(amount) = object.get("amount_money") {
            if let Some(amt) = amount.get("amount") {
                properties.insert("amount".to_string(), amt.clone());
            }
            if let Some(currency) = amount.get("currency") {
                properties.insert("currency".to_string(), currency.clone());
            }
        }
    }

    fn map_inventory_default(
        object: &JsonValue,
        properties: &mut HashMap<String, JsonValue>,
    ) {
        if let Some(v) = object.get("catalog_object_id") {
            properties.insert("catalog_object_id".to_string(), v.clone());
        }
        if let Some(v) = object.get("location_id") {
            properties.insert("location_id".to_string(), v.clone());
        }
        if let Some(v) = object.get("quantity") {
            properties.insert("quantity".to_string(), v.clone());
        }
        if let Some(v) = object.get("state") {
            properties.insert("state".to_string(), v.clone());
        }
        if let Some(v) = object.get("calculated_at") {
            properties.insert("calculated_at".to_string(), v.clone());
        }
    }

    fn map_subscription_default(
        object: &JsonValue,
        properties: &mut HashMap<String, JsonValue>,
    ) {
        if let Some(v) = object.get("status") {
            properties.insert("status".to_string(), v.clone());
        }
        if let Some(v) = object.get("customer_id") {
            properties.insert("customer_id".to_string(), v.clone());
        }
        if let Some(v) = object.get("location_id") {
            properties.insert("location_id".to_string(), v.clone());
        }
        if let Some(v) = object.get("plan_id") {
            properties.insert("plan_id".to_string(), v.clone());
        }
        if let Some(v) = object.get("start_date") {
            properties.insert("start_date".to_string(), v.clone());
        }
        if let Some(v) = object.get("canceled_date") {
            properties.insert("canceled_date".to_string(), v.clone());
        }
        if let Some(v) = object.get("charged_through_date") {
            properties
                .insert("charged_through_date".to_string(), v.clone());
        }
    }

    fn map_invoice_default(
        object: &JsonValue,
        properties: &mut HashMap<String, JsonValue>,
    ) {
        if let Some(v) = object.get("status") {
            properties.insert("status".to_string(), v.clone());
        }
        if let Some(v) = object.get("invoice_number") {
            properties.insert("invoice_number".to_string(), v.clone());
        }
        if let Some(v) = object.get("order_id") {
            properties.insert("order_id".to_string(), v.clone());
        }
        if let Some(v) = object.get("location_id") {
            properties.insert("location_id".to_string(), v.clone());
        }
        if let Some(primary) = object.get("primary_recipient") {
            if let Some(customer_id) = primary.get("customer_id") {
                properties
                    .insert("customer_id".to_string(), customer_id.clone());
            }
        }
        if let Some(v) = object.get("payment_requests") {
            properties.insert("payment_requests".to_string(), v.clone());
        }
        if let Some(v) = object.get("scheduled_at") {
            properties.insert("scheduled_at".to_string(), v.clone());
        }
        if let Some(v) = object.get("due_date") {
            properties.insert("due_date".to_string(), v.clone());
        }
        if let Some(v) = object.get("public_url") {
            properties.insert("public_url".to_string(), v.clone());
        }
    }

    /// Get name for the object based on type.
    fn get_object_name(
        object_type: SquareObjectType,
        object: &JsonValue,
    ) -> String {
        match object_type {
            SquareObjectType::Catalog => {
                let version = object
                    .get("catalog_version")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                format!("Catalog Version {version}")
            }
            SquareObjectType::CatalogItem => object
                .get("item_data")
                .and_then(|d| d.get("name"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    format!(
                        "Item {}",
                        object
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                    )
                }),
            SquareObjectType::CatalogCategory => object
                .get("category_data")
                .and_then(|d| d.get("name"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    format!(
                        "Category {}",
                        object
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                    )
                }),
            SquareObjectType::CatalogItemVariation => object
                .get("item_variation_data")
                .and_then(|d| d.get("name"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    format!(
                        "Variation {}",
                        object
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                    )
                }),
            SquareObjectType::CatalogModifier => object
                .get("modifier_data")
                .and_then(|d| d.get("name"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    format!(
                        "Modifier {}",
                        object
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                    )
                }),
            SquareObjectType::CatalogTax => object
                .get("tax_data")
                .and_then(|d| d.get("name"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    format!(
                        "Tax {}",
                        object
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                    )
                }),
            SquareObjectType::CatalogDiscount => object
                .get("discount_data")
                .and_then(|d| d.get("name"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    format!(
                        "Discount {}",
                        object
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                    )
                }),
            SquareObjectType::Customer => {
                let given = object
                    .get("given_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let family = object
                    .get("family_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if !given.is_empty() || !family.is_empty() {
                    format!("{given} {family}").trim().to_string()
                } else if let Some(email) =
                    object.get("email_address").and_then(|v| v.as_str())
                {
                    email.to_string()
                } else {
                    format!(
                        "Customer {}",
                        object
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                    )
                }
            }
            SquareObjectType::Order => {
                let state = object
                    .get("state")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UNKNOWN");
                format!(
                    "Order ({}) {}",
                    state,
                    object
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                )
            }
            SquareObjectType::Payment => {
                let amount = object
                    .get("amount_money")
                    .and_then(|a| a.get("amount"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let currency = object
                    .get("amount_money")
                    .and_then(|a| a.get("currency"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("USD");
                format!(
                    "{:.2} {} Payment",
                    amount as f64 / 100.0,
                    currency.to_uppercase()
                )
            }
            SquareObjectType::Inventory => {
                let catalog_id = object
                    .get("catalog_object_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let quantity = object
                    .get("quantity")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0");
                format!("Inventory {catalog_id} (qty: {quantity})")
            }
            SquareObjectType::Subscription => {
                let status = object
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UNKNOWN");
                format!(
                    "Subscription ({}) {}",
                    status,
                    object
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                )
            }
            SquareObjectType::Invoice => object
                .get("invoice_number")
                .and_then(|v| v.as_str())
                .map(|s| format!("Invoice {s}"))
                .unwrap_or_else(|| {
                    format!(
                        "Invoice {}",
                        object
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                    )
                }),
        }
    }
}

/// Get a nested value from JSON using dot notation.
fn get_nested_value(json: &JsonValue, path: &str) -> Option<JsonValue> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = json.clone();

    for part in parts {
        match current {
            JsonValue::Object(map) => {
                current = map.get(part)?.clone();
            }
            _ => return None,
        }
    }

    Some(current)
}

/// Apply a transform function to a value.
fn apply_transform(value: &JsonValue, transform: &Transform) -> JsonValue {
    match transform {
        Transform::ToNumber => {
            if let Some(s) = value.as_str() {
                if let Ok(n) = s.parse::<i64>() {
                    return JsonValue::Number(n.into());
                }
                if let Ok(f) = s.parse::<f64>() {
                    if let Some(num) = serde_json::Number::from_f64(f) {
                        return JsonValue::Number(num);
                    }
                }
            }
            value.clone()
        }
        Transform::ToBool => match value {
            JsonValue::String(s) => {
                let lower = s.to_lowercase();
                JsonValue::Bool(
                    lower == "true"
                        || lower == "yes"
                        || lower == "1"
                        || lower == "on",
                )
            }
            JsonValue::Number(n) => {
                JsonValue::Bool(n.as_i64().unwrap_or(0) != 0)
            }
            _ => value.clone(),
        },
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
        // Currency conversion: cents to dollars
        Transform::CentsToDollars => {
            if let Some(cents) = value.as_i64() {
                if let Some(num) =
                    serde_json::Number::from_f64(cents as f64 / 100.0)
                {
                    return JsonValue::Number(num);
                }
            }
            value.clone()
        }
        _ => value.clone(),
    }
}

#[async_trait]
impl SquareDataHandler for DefaultSquareDataHandler {
    async fn upsert_object(
        &self,
        endpoint: &WebhookEndpoint,
        object_type: SquareObjectType,
        object: &JsonValue,
        mapping: Option<&PropertyMapping>,
    ) -> errors::Result<String> {
        let properties =
            Self::map_object_properties(object_type, object, mapping);
        let name = Self::get_object_name(object_type, object);

        // Use description or empty content
        let content = match object_type {
            SquareObjectType::CatalogItem => object
                .get("item_data")
                .and_then(|d| d.get("description"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            _ => String::new(),
        };

        // Check for existing data
        let square_id = object
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let external_id =
            format!("square:{}:{}", object_type.as_str(), square_id);
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
        _object_type: SquareObjectType,
        data_id: &str,
    ) -> errors::Result<()> {
        self.data_repo.delete_data(endpoint, data_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_catalog_item_properties_default() {
        let item = serde_json::json!({
            "id": "ITEM_123",
            "type": "ITEM",
            "updated_at": "2024-01-01T00:00:00Z",
            "is_deleted": false,
            "item_data": {
                "name": "Coffee",
                "description": "Hot coffee",
                "category_id": "CAT_456",
                "product_type": "REGULAR",
                "visibility": "PUBLIC",
                "variations": [
                    {
                        "id": "VAR_789",
                        "type": "ITEM_VARIATION"
                    }
                ]
            }
        });

        let properties = DefaultSquareDataHandler::map_object_properties(
            SquareObjectType::CatalogItem,
            &item,
            None,
        );

        assert_eq!(
            properties.get("name").unwrap().as_str().unwrap(),
            "Coffee"
        );
        assert_eq!(
            properties.get("description").unwrap().as_str().unwrap(),
            "Hot coffee"
        );
        assert_eq!(
            properties.get("category_id").unwrap().as_str().unwrap(),
            "CAT_456"
        );
        assert!(!properties.get("is_deleted").unwrap().as_bool().unwrap());
    }

    #[test]
    fn test_map_customer_properties_default() {
        let customer = serde_json::json!({
            "id": "CUST_123",
            "given_name": "John",
            "family_name": "Doe",
            "email_address": "john@example.com",
            "phone_number": "+1234567890",
            "address": {
                "address_line_1": "123 Main St",
                "locality": "San Francisco",
                "administrative_district_level_1": "CA",
                "postal_code": "94102",
                "country": "US"
            }
        });

        let properties = DefaultSquareDataHandler::map_object_properties(
            SquareObjectType::Customer,
            &customer,
            None,
        );

        assert_eq!(
            properties.get("given_name").unwrap().as_str().unwrap(),
            "John"
        );
        assert_eq!(
            properties.get("family_name").unwrap().as_str().unwrap(),
            "Doe"
        );
        assert_eq!(
            properties.get("email").unwrap().as_str().unwrap(),
            "john@example.com"
        );
        assert_eq!(
            properties.get("city").unwrap().as_str().unwrap(),
            "San Francisco"
        );
    }

    #[test]
    fn test_get_catalog_item_name() {
        let item = serde_json::json!({
            "id": "ITEM_123",
            "item_data": {
                "name": "Espresso"
            }
        });

        let name = DefaultSquareDataHandler::get_object_name(
            SquareObjectType::CatalogItem,
            &item,
        );
        assert_eq!(name, "Espresso");
    }

    #[test]
    fn test_get_customer_name_with_name() {
        let customer = serde_json::json!({
            "id": "CUST_123",
            "given_name": "Jane",
            "family_name": "Smith"
        });

        let name = DefaultSquareDataHandler::get_object_name(
            SquareObjectType::Customer,
            &customer,
        );
        assert_eq!(name, "Jane Smith");
    }

    #[test]
    fn test_get_customer_name_email_fallback() {
        let customer = serde_json::json!({
            "id": "CUST_123",
            "email_address": "test@example.com"
        });

        let name = DefaultSquareDataHandler::get_object_name(
            SquareObjectType::Customer,
            &customer,
        );
        assert_eq!(name, "test@example.com");
    }

    #[test]
    fn test_get_payment_name() {
        let payment = serde_json::json!({
            "id": "PAY_123",
            "amount_money": {
                "amount": 1500,
                "currency": "USD"
            }
        });

        let name = DefaultSquareDataHandler::get_object_name(
            SquareObjectType::Payment,
            &payment,
        );
        assert_eq!(name, "15.00 USD Payment");
    }
}
