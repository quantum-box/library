//! Stripe webhook payload types.
//!
//! Stripe sends webhook events for various resources like products, prices,
//! customers, subscriptions, etc.
//!
//! Reference: https://stripe.com/docs/webhooks

use serde::{Deserialize, Serialize};

/// Stripe webhook event wrapper.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StripeEvent {
    /// Event ID (e.g., "evt_...")
    pub id: String,
    /// Object type (always "event")
    pub object: String,
    /// API version
    pub api_version: String,
    /// Created timestamp (Unix)
    pub created: i64,
    /// Event data
    pub data: EventData,
    /// Whether this is a test mode event
    pub livemode: bool,
    /// Number of pending webhooks
    pub pending_webhooks: i32,
    /// Request information
    #[serde(default)]
    pub request: Option<RequestInfo>,
    /// Event type (e.g., "product.created")
    #[serde(rename = "type")]
    pub event_type: String,
}

impl StripeEvent {
    /// Parse the event type to extract object type and action.
    ///
    /// Examples:
    /// - "product.created" -> ("product", "created")
    /// - "price.updated" -> ("price", "updated")
    /// - "customer.subscription.created" -> ("subscription", "created")
    pub fn parse_event_type(
        &self,
    ) -> Option<(StripeObjectType, EventAction)> {
        let parts: Vec<&str> = self.event_type.split('.').collect();
        if parts.len() < 2 {
            return None;
        }

        // Handle nested types like "customer.subscription.created"
        let (object_part, action_part) = if parts.len() == 2 {
            (parts[0], parts[1])
        } else {
            // Take last two parts (e.g., "subscription", "created")
            (parts[parts.len() - 2], parts[parts.len() - 1])
        };

        let object_type = StripeObjectType::parse_str(object_part)?;
        let action = EventAction::parse_str(action_part)?;

        Some((object_type, action))
    }

    /// Get the Stripe object from the event data.
    pub fn get_object(&self) -> &serde_json::Value {
        &self.data.object
    }

    /// Get the object ID from the event data.
    pub fn get_object_id(&self) -> Option<&str> {
        self.data.object.get("id").and_then(|v| v.as_str())
    }
}

/// Event data wrapper.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EventData {
    /// The object that was changed
    pub object: serde_json::Value,
    /// Previous attributes (for update events)
    #[serde(default)]
    pub previous_attributes: Option<serde_json::Value>,
}

/// Request information.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RequestInfo {
    /// Request ID
    #[serde(default)]
    pub id: Option<String>,
    /// Idempotency key
    #[serde(default)]
    pub idempotency_key: Option<String>,
}

/// Stripe object types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StripeObjectType {
    Product,
    Price,
    Customer,
    Subscription,
    Invoice,
    PaymentIntent,
    Charge,
    Coupon,
}

impl StripeObjectType {
    pub fn parse_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "product" => Some(StripeObjectType::Product),
            "price" => Some(StripeObjectType::Price),
            "customer" => Some(StripeObjectType::Customer),
            "subscription" => Some(StripeObjectType::Subscription),
            "invoice" => Some(StripeObjectType::Invoice),
            "payment_intent" | "paymentintent" => {
                Some(StripeObjectType::PaymentIntent)
            }
            "charge" => Some(StripeObjectType::Charge),
            "coupon" => Some(StripeObjectType::Coupon),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            StripeObjectType::Product => "product",
            StripeObjectType::Price => "price",
            StripeObjectType::Customer => "customer",
            StripeObjectType::Subscription => "subscription",
            StripeObjectType::Invoice => "invoice",
            StripeObjectType::PaymentIntent => "payment_intent",
            StripeObjectType::Charge => "charge",
            StripeObjectType::Coupon => "coupon",
        }
    }
}

/// Stripe webhook event actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventAction {
    Created,
    Updated,
    Deleted,
}

impl EventAction {
    pub fn parse_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "created" => Some(EventAction::Created),
            "updated" => Some(EventAction::Updated),
            "deleted" => Some(EventAction::Deleted),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            EventAction::Created => "created",
            EventAction::Updated => "updated",
            EventAction::Deleted => "deleted",
        }
    }
}

/// Stripe Product object.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StripeProduct {
    /// Product ID (e.g., "prod_...")
    pub id: String,
    /// Object type (always "product")
    pub object: String,
    /// Whether the product is active
    pub active: bool,
    /// Created timestamp (Unix)
    pub created: i64,
    /// Product description
    #[serde(default)]
    pub description: Option<String>,
    /// Product images
    #[serde(default)]
    pub images: Vec<String>,
    /// Whether this is a test mode product
    pub livemode: bool,
    /// Product metadata
    #[serde(default)]
    pub metadata: serde_json::Value,
    /// Product name
    pub name: String,
    /// Package dimensions
    #[serde(default)]
    pub package_dimensions: Option<serde_json::Value>,
    /// Whether the product is shippable
    #[serde(default)]
    pub shippable: Option<bool>,
    /// Tax code
    #[serde(default)]
    pub tax_code: Option<String>,
    /// Product type
    #[serde(rename = "type")]
    #[serde(default)]
    pub product_type: Option<String>,
    /// Unit label
    #[serde(default)]
    pub unit_label: Option<String>,
    /// Updated timestamp (Unix)
    pub updated: i64,
    /// URL
    #[serde(default)]
    pub url: Option<String>,
}

/// Stripe Price object.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StripePrice {
    /// Price ID (e.g., "price_...")
    pub id: String,
    /// Object type (always "price")
    pub object: String,
    /// Whether the price is active
    pub active: bool,
    /// Billing scheme
    pub billing_scheme: String,
    /// Created timestamp (Unix)
    pub created: i64,
    /// Currency
    pub currency: String,
    /// Whether this is a test mode price
    pub livemode: bool,
    /// Price metadata
    #[serde(default)]
    pub metadata: serde_json::Value,
    /// Nickname
    #[serde(default)]
    pub nickname: Option<String>,
    /// Product ID
    pub product: String,
    /// Recurring settings
    #[serde(default)]
    pub recurring: Option<RecurringSettings>,
    /// Price type (one_time or recurring)
    #[serde(rename = "type")]
    pub price_type: String,
    /// Unit amount in cents
    #[serde(default)]
    pub unit_amount: Option<i64>,
    /// Unit amount in decimal string
    #[serde(default)]
    pub unit_amount_decimal: Option<String>,
}

/// Recurring settings for a price.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RecurringSettings {
    /// Aggregate usage
    #[serde(default)]
    pub aggregate_usage: Option<String>,
    /// Billing interval
    pub interval: String,
    /// Number of intervals
    pub interval_count: i32,
    /// Usage type
    #[serde(default)]
    pub usage_type: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_event_type_simple() {
        let event = StripeEvent {
            id: "evt_test".to_string(),
            object: "event".to_string(),
            api_version: "2023-10-16".to_string(),
            created: 1234567890,
            data: EventData {
                object: serde_json::json!({}),
                previous_attributes: None,
            },
            livemode: false,
            pending_webhooks: 0,
            request: None,
            event_type: "product.created".to_string(),
        };

        let (object_type, action) = event.parse_event_type().unwrap();
        assert_eq!(object_type, StripeObjectType::Product);
        assert_eq!(action, EventAction::Created);
    }

    #[test]
    fn test_parse_event_type_nested() {
        let event = StripeEvent {
            id: "evt_test".to_string(),
            object: "event".to_string(),
            api_version: "2023-10-16".to_string(),
            created: 1234567890,
            data: EventData {
                object: serde_json::json!({}),
                previous_attributes: None,
            },
            livemode: false,
            pending_webhooks: 0,
            request: None,
            event_type: "customer.subscription.created".to_string(),
        };

        let (object_type, action) = event.parse_event_type().unwrap();
        assert_eq!(object_type, StripeObjectType::Subscription);
        assert_eq!(action, EventAction::Created);
    }

    #[test]
    fn test_stripe_object_type_from_str() {
        assert_eq!(
            StripeObjectType::parse_str("product"),
            Some(StripeObjectType::Product)
        );
        assert_eq!(
            StripeObjectType::parse_str("price"),
            Some(StripeObjectType::Price)
        );
        assert_eq!(
            StripeObjectType::parse_str("customer"),
            Some(StripeObjectType::Customer)
        );
        assert_eq!(StripeObjectType::parse_str("unknown"), None);
    }
}
