//! Stripe data handler implementation for Library data operations.

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;

use inbound_sync_domain::{PropertyMapping, Transform, WebhookEndpoint};

use super::event_processor::StripeDataHandler;
use super::payload::StripeObjectType;
use crate::providers::github::LibraryDataRepository;

/// Default implementation of StripeDataHandler.
#[derive(Debug)]
pub struct DefaultStripeDataHandler {
    data_repo: Arc<dyn LibraryDataRepository>,
}

impl DefaultStripeDataHandler {
    /// Create a new handler with a data repository.
    pub fn new(data_repo: Arc<dyn LibraryDataRepository>) -> Self {
        Self { data_repo }
    }

    /// Map Stripe object properties to Library properties.
    fn map_object_properties(
        object_type: StripeObjectType,
        object: &JsonValue,
        mapping: Option<&PropertyMapping>,
    ) -> HashMap<String, JsonValue> {
        let mut properties = HashMap::new();

        // Default mappings based on object type
        if mapping.is_none() {
            match object_type {
                StripeObjectType::Product => {
                    Self::map_product_default(object, &mut properties);
                }
                StripeObjectType::Price => {
                    Self::map_price_default(object, &mut properties);
                }
                StripeObjectType::Customer => {
                    Self::map_customer_default(object, &mut properties);
                }
                StripeObjectType::Subscription => {
                    Self::map_subscription_default(object, &mut properties);
                }
                StripeObjectType::Invoice => {
                    Self::map_invoice_default(object, &mut properties);
                }
                StripeObjectType::PaymentIntent => {
                    Self::map_payment_intent_default(
                        object,
                        &mut properties,
                    );
                }
                StripeObjectType::Charge => {
                    Self::map_charge_default(object, &mut properties);
                }
                StripeObjectType::Coupon => {
                    Self::map_coupon_default(object, &mut properties);
                }
            }

            // Add common metadata
            if let Some(id) = object.get("id").and_then(|v| v.as_str()) {
                properties.insert(
                    "stripe_id".to_string(),
                    JsonValue::String(id.to_string()),
                );
            }
            properties.insert(
                "object_type".to_string(),
                JsonValue::String(object_type.as_str().to_string()),
            );
            if let Some(created) =
                object.get("created").and_then(|v| v.as_i64())
            {
                properties.insert(
                    "created_at".to_string(),
                    JsonValue::Number(created.into()),
                );
            }
            if let Some(livemode) =
                object.get("livemode").and_then(|v| v.as_bool())
            {
                properties.insert(
                    "livemode".to_string(),
                    JsonValue::Bool(livemode),
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

    fn map_product_default(
        object: &JsonValue,
        properties: &mut HashMap<String, JsonValue>,
    ) {
        if let Some(v) = object.get("name") {
            properties.insert("name".to_string(), v.clone());
        }
        if let Some(v) = object.get("description") {
            properties.insert("description".to_string(), v.clone());
        }
        if let Some(v) = object.get("active") {
            properties.insert("active".to_string(), v.clone());
        }
        if let Some(v) = object.get("default_price") {
            if let Some(id) = v.as_str() {
                properties.insert(
                    "default_price_id".to_string(),
                    JsonValue::String(id.to_string()),
                );
            } else if let Some(obj) = v.as_object() {
                if let Some(id) = obj.get("id").and_then(|id| id.as_str()) {
                    properties.insert(
                        "default_price_id".to_string(),
                        JsonValue::String(id.to_string()),
                    );
                }
            }
        }
        if let Some(images) = object.get("images") {
            properties.insert("images".to_string(), images.clone());
        }
        if let Some(metadata) = object.get("metadata") {
            properties.insert("metadata".to_string(), metadata.clone());
        }
    }

    fn map_price_default(
        object: &JsonValue,
        properties: &mut HashMap<String, JsonValue>,
    ) {
        if let Some(v) = object.get("unit_amount") {
            properties.insert("unit_amount".to_string(), v.clone());
        }
        if let Some(v) = object.get("currency") {
            properties.insert("currency".to_string(), v.clone());
        }
        if let Some(v) = object.get("active") {
            properties.insert("active".to_string(), v.clone());
        }
        if let Some(v) = object.get("type") {
            properties.insert("price_type".to_string(), v.clone());
        }
        if let Some(v) = object.get("recurring") {
            if let Some(interval) = v.get("interval") {
                properties.insert(
                    "billing_interval".to_string(),
                    interval.clone(),
                );
            }
            if let Some(count) = v.get("interval_count") {
                properties
                    .insert("interval_count".to_string(), count.clone());
            }
        }
        if let Some(v) = object.get("product") {
            if let Some(id) = v.as_str() {
                properties.insert(
                    "product_id".to_string(),
                    JsonValue::String(id.to_string()),
                );
            }
        }
        if let Some(v) = object.get("nickname") {
            properties.insert("nickname".to_string(), v.clone());
        }
    }

    fn map_customer_default(
        object: &JsonValue,
        properties: &mut HashMap<String, JsonValue>,
    ) {
        if let Some(v) = object.get("name") {
            properties.insert("name".to_string(), v.clone());
        }
        if let Some(v) = object.get("email") {
            properties.insert("email".to_string(), v.clone());
        }
        if let Some(v) = object.get("phone") {
            properties.insert("phone".to_string(), v.clone());
        }
        if let Some(v) = object.get("description") {
            properties.insert("description".to_string(), v.clone());
        }
        if let Some(v) = object.get("balance") {
            properties.insert("balance".to_string(), v.clone());
        }
        if let Some(v) = object.get("currency") {
            properties.insert("currency".to_string(), v.clone());
        }
        if let Some(v) = object.get("delinquent") {
            properties.insert("delinquent".to_string(), v.clone());
        }
        if let Some(v) = object.get("default_source") {
            properties.insert("default_source".to_string(), v.clone());
        }
        if let Some(metadata) = object.get("metadata") {
            properties.insert("metadata".to_string(), metadata.clone());
        }
    }

    fn map_subscription_default(
        object: &JsonValue,
        properties: &mut HashMap<String, JsonValue>,
    ) {
        if let Some(v) = object.get("status") {
            properties.insert("status".to_string(), v.clone());
        }
        if let Some(v) = object.get("customer") {
            properties.insert("customer_id".to_string(), v.clone());
        }
        if let Some(v) = object.get("current_period_start") {
            properties
                .insert("current_period_start".to_string(), v.clone());
        }
        if let Some(v) = object.get("current_period_end") {
            properties.insert("current_period_end".to_string(), v.clone());
        }
        if let Some(v) = object.get("cancel_at_period_end") {
            properties
                .insert("cancel_at_period_end".to_string(), v.clone());
        }
        if let Some(v) = object.get("canceled_at") {
            properties.insert("canceled_at".to_string(), v.clone());
        }
        if let Some(v) = object.get("trial_start") {
            properties.insert("trial_start".to_string(), v.clone());
        }
        if let Some(v) = object.get("trial_end") {
            properties.insert("trial_end".to_string(), v.clone());
        }
        // Map items
        if let Some(items) = object.get("items").and_then(|i| i.get("data"))
        {
            if let Some(arr) = items.as_array() {
                let item_ids: Vec<JsonValue> = arr
                    .iter()
                    .filter_map(|item| {
                        item.get("price").and_then(|p| p.get("id"))
                    })
                    .cloned()
                    .collect();
                properties.insert(
                    "price_ids".to_string(),
                    JsonValue::Array(item_ids),
                );
            }
        }
    }

    fn map_invoice_default(
        object: &JsonValue,
        properties: &mut HashMap<String, JsonValue>,
    ) {
        if let Some(v) = object.get("number") {
            properties.insert("invoice_number".to_string(), v.clone());
        }
        if let Some(v) = object.get("status") {
            properties.insert("status".to_string(), v.clone());
        }
        if let Some(v) = object.get("customer") {
            properties.insert("customer_id".to_string(), v.clone());
        }
        if let Some(v) = object.get("subscription") {
            properties.insert("subscription_id".to_string(), v.clone());
        }
        if let Some(v) = object.get("amount_due") {
            properties.insert("amount_due".to_string(), v.clone());
        }
        if let Some(v) = object.get("amount_paid") {
            properties.insert("amount_paid".to_string(), v.clone());
        }
        if let Some(v) = object.get("currency") {
            properties.insert("currency".to_string(), v.clone());
        }
        if let Some(v) = object.get("due_date") {
            properties.insert("due_date".to_string(), v.clone());
        }
        if let Some(v) = object.get("hosted_invoice_url") {
            properties.insert("invoice_url".to_string(), v.clone());
        }
        if let Some(v) = object.get("invoice_pdf") {
            properties.insert("pdf_url".to_string(), v.clone());
        }
    }

    fn map_payment_intent_default(
        object: &JsonValue,
        properties: &mut HashMap<String, JsonValue>,
    ) {
        if let Some(v) = object.get("status") {
            properties.insert("status".to_string(), v.clone());
        }
        if let Some(v) = object.get("amount") {
            properties.insert("amount".to_string(), v.clone());
        }
        if let Some(v) = object.get("currency") {
            properties.insert("currency".to_string(), v.clone());
        }
        if let Some(v) = object.get("customer") {
            properties.insert("customer_id".to_string(), v.clone());
        }
        if let Some(v) = object.get("description") {
            properties.insert("description".to_string(), v.clone());
        }
        if let Some(v) = object.get("payment_method") {
            properties.insert("payment_method".to_string(), v.clone());
        }
        if let Some(v) = object.get("receipt_email") {
            properties.insert("receipt_email".to_string(), v.clone());
        }
        if let Some(metadata) = object.get("metadata") {
            properties.insert("metadata".to_string(), metadata.clone());
        }
    }

    fn map_charge_default(
        object: &JsonValue,
        properties: &mut HashMap<String, JsonValue>,
    ) {
        if let Some(v) = object.get("status") {
            properties.insert("status".to_string(), v.clone());
        }
        if let Some(v) = object.get("amount") {
            properties.insert("amount".to_string(), v.clone());
        }
        if let Some(v) = object.get("currency") {
            properties.insert("currency".to_string(), v.clone());
        }
        if let Some(v) = object.get("customer") {
            properties.insert("customer_id".to_string(), v.clone());
        }
        if let Some(v) = object.get("description") {
            properties.insert("description".to_string(), v.clone());
        }
        if let Some(v) = object.get("paid") {
            properties.insert("paid".to_string(), v.clone());
        }
        if let Some(v) = object.get("refunded") {
            properties.insert("refunded".to_string(), v.clone());
        }
        if let Some(v) = object.get("amount_refunded") {
            properties.insert("amount_refunded".to_string(), v.clone());
        }
        if let Some(v) = object.get("receipt_url") {
            properties.insert("receipt_url".to_string(), v.clone());
        }
        if let Some(v) = object.get("payment_intent") {
            properties.insert("payment_intent_id".to_string(), v.clone());
        }
    }

    fn map_coupon_default(
        object: &JsonValue,
        properties: &mut HashMap<String, JsonValue>,
    ) {
        if let Some(v) = object.get("name") {
            properties.insert("name".to_string(), v.clone());
        }
        if let Some(v) = object.get("percent_off") {
            properties.insert("percent_off".to_string(), v.clone());
        }
        if let Some(v) = object.get("amount_off") {
            properties.insert("amount_off".to_string(), v.clone());
        }
        if let Some(v) = object.get("currency") {
            properties.insert("currency".to_string(), v.clone());
        }
        if let Some(v) = object.get("duration") {
            properties.insert("duration".to_string(), v.clone());
        }
        if let Some(v) = object.get("duration_in_months") {
            properties.insert("duration_in_months".to_string(), v.clone());
        }
        if let Some(v) = object.get("max_redemptions") {
            properties.insert("max_redemptions".to_string(), v.clone());
        }
        if let Some(v) = object.get("times_redeemed") {
            properties.insert("times_redeemed".to_string(), v.clone());
        }
        if let Some(v) = object.get("valid") {
            properties.insert("valid".to_string(), v.clone());
        }
    }

    /// Get name for the object based on type.
    fn get_object_name(
        object_type: StripeObjectType,
        object: &JsonValue,
    ) -> String {
        match object_type {
            StripeObjectType::Product => object
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    format!(
                        "Product {}",
                        object
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                    )
                }),
            StripeObjectType::Price => {
                let nickname =
                    object.get("nickname").and_then(|v| v.as_str());
                let unit_amount =
                    object.get("unit_amount").and_then(|v| v.as_i64());
                let currency = object
                    .get("currency")
                    .and_then(|v| v.as_str())
                    .unwrap_or("usd");

                if let Some(name) = nickname {
                    name.to_string()
                } else if let Some(amount) = unit_amount {
                    format!(
                        "{:.2} {}",
                        amount as f64 / 100.0,
                        currency.to_uppercase()
                    )
                } else {
                    format!(
                        "Price {}",
                        object
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                    )
                }
            }
            StripeObjectType::Customer => {
                if let Some(name) =
                    object.get("name").and_then(|v| v.as_str())
                {
                    return name.to_string();
                }
                if let Some(email) =
                    object.get("email").and_then(|v| v.as_str())
                {
                    return email.to_string();
                }
                format!(
                    "Customer {}",
                    object
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                )
            }
            StripeObjectType::Subscription => {
                let status = object
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                format!(
                    "Subscription ({}) {}",
                    status,
                    object
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                )
            }
            StripeObjectType::Invoice => object
                .get("number")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    format!(
                        "Invoice {}",
                        object
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                    )
                }),
            StripeObjectType::PaymentIntent => {
                let amount = object
                    .get("amount")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let currency = object
                    .get("currency")
                    .and_then(|v| v.as_str())
                    .unwrap_or("usd");
                format!(
                    "{:.2} {} Payment",
                    amount as f64 / 100.0,
                    currency.to_uppercase()
                )
            }
            StripeObjectType::Charge => {
                let amount = object
                    .get("amount")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let currency = object
                    .get("currency")
                    .and_then(|v| v.as_str())
                    .unwrap_or("usd");
                format!(
                    "{:.2} {} Charge",
                    amount as f64 / 100.0,
                    currency.to_uppercase()
                )
            }
            StripeObjectType::Coupon => object
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    let percent =
                        object.get("percent_off").and_then(|v| v.as_f64());
                    let amount =
                        object.get("amount_off").and_then(|v| v.as_i64());
                    if let Some(p) = percent {
                        format!("{p}% off")
                    } else if let Some(a) = amount {
                        let currency = object
                            .get("currency")
                            .and_then(|v| v.as_str())
                            .unwrap_or("usd");
                        format!(
                            "{:.2} {} off",
                            a as f64 / 100.0,
                            currency.to_uppercase()
                        )
                    } else {
                        format!(
                            "Coupon {}",
                            object
                                .get("id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                        )
                    }
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
impl StripeDataHandler for DefaultStripeDataHandler {
    async fn upsert_object(
        &self,
        endpoint: &WebhookEndpoint,
        object_type: StripeObjectType,
        object: &JsonValue,
        mapping: Option<&PropertyMapping>,
    ) -> errors::Result<String> {
        let properties =
            Self::map_object_properties(object_type, object, mapping);
        let name = Self::get_object_name(object_type, object);

        // Use description or empty content
        let content = object
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Check for existing data
        let stripe_id = object
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let external_id =
            format!("stripe:{}:{}", object_type.as_str(), stripe_id);
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
        _object_type: StripeObjectType,
        data_id: &str,
    ) -> errors::Result<()> {
        self.data_repo.delete_data(endpoint, data_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_product_properties_default() {
        let product = serde_json::json!({
            "id": "prod_123",
            "name": "Premium Plan",
            "description": "Our best plan",
            "active": true,
            "default_price": "price_456",
            "created": 1704067200,
            "livemode": false
        });

        let properties = DefaultStripeDataHandler::map_object_properties(
            StripeObjectType::Product,
            &product,
            None,
        );

        assert_eq!(
            properties.get("name").unwrap().as_str().unwrap(),
            "Premium Plan"
        );
        assert_eq!(
            properties.get("description").unwrap().as_str().unwrap(),
            "Our best plan"
        );
        assert!(properties.get("active").unwrap().as_bool().unwrap());
        assert_eq!(
            properties
                .get("default_price_id")
                .unwrap()
                .as_str()
                .unwrap(),
            "price_456"
        );
    }

    #[test]
    fn test_map_subscription_properties_default() {
        let subscription = serde_json::json!({
            "id": "sub_123",
            "status": "active",
            "customer": "cus_456",
            "current_period_start": 1704067200,
            "current_period_end": 1706745600,
            "cancel_at_period_end": false,
            "items": {
                "data": [
                    {
                        "price": { "id": "price_abc" }
                    },
                    {
                        "price": { "id": "price_def" }
                    }
                ]
            }
        });

        let properties = DefaultStripeDataHandler::map_object_properties(
            StripeObjectType::Subscription,
            &subscription,
            None,
        );

        assert_eq!(
            properties.get("status").unwrap().as_str().unwrap(),
            "active"
        );
        assert_eq!(
            properties.get("customer_id").unwrap().as_str().unwrap(),
            "cus_456"
        );

        let price_ids =
            properties.get("price_ids").unwrap().as_array().unwrap();
        assert_eq!(price_ids.len(), 2);
    }

    #[test]
    fn test_get_product_name() {
        let product = serde_json::json!({
            "id": "prod_123",
            "name": "Premium Plan"
        });

        let name = DefaultStripeDataHandler::get_object_name(
            StripeObjectType::Product,
            &product,
        );
        assert_eq!(name, "Premium Plan");
    }

    #[test]
    fn test_get_customer_name_with_email_fallback() {
        let customer = serde_json::json!({
            "id": "cus_123",
            "email": "test@example.com"
        });

        let name = DefaultStripeDataHandler::get_object_name(
            StripeObjectType::Customer,
            &customer,
        );
        assert_eq!(name, "test@example.com");
    }

    #[test]
    fn test_cents_to_dollars_transform() {
        let value = JsonValue::Number(1500.into());
        let result = apply_transform(&value, &Transform::CentsToDollars);
        assert_eq!(result.as_f64().unwrap(), 15.0);
    }
}
