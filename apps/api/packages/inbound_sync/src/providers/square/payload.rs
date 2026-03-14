//! Square webhook payload types.
//!
//! Square sends webhook events for various resources like catalog items,
//! customers, orders, payments, inventory, etc.
//!
//! Reference: https://developer.squareup.com/docs/webhooks/overview

use serde::{Deserialize, Serialize};

/// Square webhook event wrapper.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SquareEvent {
    /// Merchant ID
    pub merchant_id: String,
    /// Event type (e.g., "catalog.version.updated")
    #[serde(rename = "type")]
    pub event_type: String,
    /// Event ID
    pub event_id: String,
    /// Created at timestamp (ISO 8601)
    pub created_at: String,
    /// Event data
    pub data: EventData,
}

impl SquareEvent {
    /// Parse the event type to extract object type and action.
    ///
    /// Examples:
    /// - "catalog.version.updated" -> (Catalog, VersionUpdated)
    /// - "customer.created" -> (Customer, Created)
    /// - "order.created" -> (Order, Created)
    pub fn parse_event_type(
        &self,
    ) -> Option<(SquareObjectType, EventAction)> {
        let parts: Vec<&str> = self.event_type.split('.').collect();
        if parts.len() < 2 {
            return None;
        }

        let object_type = SquareObjectType::parse_str(parts[0])?;

        // Handle special case: catalog.version.updated
        let action_str = if parts.len() >= 3 && parts[0] == "catalog" {
            // For catalog, we use "version.updated" as the action
            "version_updated"
        } else {
            parts.last()?
        };

        let action = EventAction::parse_str(action_str)?;
        Some((object_type, action))
    }

    /// Get the object ID from the event data.
    pub fn get_object_id(&self) -> Option<&str> {
        self.data.id.as_deref()
    }

    /// Get the object data from the event.
    pub fn get_object(&self) -> &serde_json::Value {
        &self.data.object
    }
}

/// Event data wrapper.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EventData {
    /// Object type
    #[serde(rename = "type")]
    pub data_type: Option<String>,
    /// Object ID
    pub id: Option<String>,
    /// The object that was changed
    #[serde(default)]
    pub object: serde_json::Value,
}

/// Square object types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SquareObjectType {
    Catalog,
    CatalogItem,
    CatalogCategory,
    CatalogItemVariation,
    CatalogModifier,
    CatalogTax,
    CatalogDiscount,
    Customer,
    Order,
    Payment,
    Inventory,
    Subscription,
    Invoice,
}

impl SquareObjectType {
    pub fn parse_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "catalog" => Some(SquareObjectType::Catalog),
            "catalog_item" | "catalogitem" | "item" => {
                Some(SquareObjectType::CatalogItem)
            }
            "catalog_category" | "catalogcategory" | "category" => {
                Some(SquareObjectType::CatalogCategory)
            }
            "catalog_item_variation"
            | "catalogitemvariation"
            | "item_variation" => {
                Some(SquareObjectType::CatalogItemVariation)
            }
            "catalog_modifier" | "catalogmodifier" | "modifier" => {
                Some(SquareObjectType::CatalogModifier)
            }
            "catalog_tax" | "catalogtax" | "tax" => {
                Some(SquareObjectType::CatalogTax)
            }
            "catalog_discount" | "catalogdiscount" | "discount" => {
                Some(SquareObjectType::CatalogDiscount)
            }
            "customer" => Some(SquareObjectType::Customer),
            "order" => Some(SquareObjectType::Order),
            "payment" => Some(SquareObjectType::Payment),
            "inventory" => Some(SquareObjectType::Inventory),
            "subscription" => Some(SquareObjectType::Subscription),
            "invoice" => Some(SquareObjectType::Invoice),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SquareObjectType::Catalog => "catalog",
            SquareObjectType::CatalogItem => "catalog_item",
            SquareObjectType::CatalogCategory => "catalog_category",
            SquareObjectType::CatalogItemVariation => {
                "catalog_item_variation"
            }
            SquareObjectType::CatalogModifier => "catalog_modifier",
            SquareObjectType::CatalogTax => "catalog_tax",
            SquareObjectType::CatalogDiscount => "catalog_discount",
            SquareObjectType::Customer => "customer",
            SquareObjectType::Order => "order",
            SquareObjectType::Payment => "payment",
            SquareObjectType::Inventory => "inventory",
            SquareObjectType::Subscription => "subscription",
            SquareObjectType::Invoice => "invoice",
        }
    }
}

/// Square webhook event actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventAction {
    Created,
    Updated,
    Deleted,
    VersionUpdated,
    FulfillmentUpdated,
    PaymentMade,
    CountUpdated,
}

impl EventAction {
    pub fn parse_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "created" => Some(EventAction::Created),
            "updated" => Some(EventAction::Updated),
            "deleted" => Some(EventAction::Deleted),
            "version_updated" | "version.updated" => {
                Some(EventAction::VersionUpdated)
            }
            "fulfillment_updated" | "fulfillment.updated" => {
                Some(EventAction::FulfillmentUpdated)
            }
            "payment_made" | "payment.made" => {
                Some(EventAction::PaymentMade)
            }
            "count_updated" | "count.updated" => {
                Some(EventAction::CountUpdated)
            }
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            EventAction::Created => "created",
            EventAction::Updated => "updated",
            EventAction::Deleted => "deleted",
            EventAction::VersionUpdated => "version_updated",
            EventAction::FulfillmentUpdated => "fulfillment_updated",
            EventAction::PaymentMade => "payment_made",
            EventAction::CountUpdated => "count_updated",
        }
    }
}

/// Square Catalog object (from catalog.version.updated).
/// This contains the updated catalog version info.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CatalogVersionUpdated {
    /// Catalog version number
    pub catalog_version: Option<i64>,
    /// Updated at timestamp
    pub updated_at: Option<String>,
}

/// Square CatalogItem object.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SquareCatalogItem {
    /// Catalog object ID (e.g., "ITEM_...")
    pub id: String,
    /// Object type (always "ITEM")
    #[serde(rename = "type")]
    pub object_type: String,
    /// Updated at timestamp
    pub updated_at: Option<String>,
    /// Whether this object is deleted
    #[serde(default)]
    pub is_deleted: bool,
    /// Item data
    pub item_data: Option<ItemData>,
}

/// Square item data.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ItemData {
    /// Item name
    pub name: Option<String>,
    /// Item description
    pub description: Option<String>,
    /// Category ID
    pub category_id: Option<String>,
    /// Product type
    pub product_type: Option<String>,
    /// Visibility
    pub visibility: Option<String>,
    /// Variations
    #[serde(default)]
    pub variations: Vec<SquareItemVariation>,
    /// Image IDs
    #[serde(default)]
    pub image_ids: Vec<String>,
}

/// Square item variation.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SquareItemVariation {
    /// Variation ID
    pub id: String,
    /// Object type
    #[serde(rename = "type")]
    pub object_type: String,
    /// Variation data
    pub item_variation_data: Option<ItemVariationData>,
}

/// Square item variation data.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ItemVariationData {
    /// Item ID this variation belongs to
    pub item_id: Option<String>,
    /// Variation name
    pub name: Option<String>,
    /// SKU
    pub sku: Option<String>,
    /// Price money
    pub price_money: Option<Money>,
    /// Pricing type
    pub pricing_type: Option<String>,
}

/// Square Money object.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Money {
    /// Amount in smallest currency unit (cents)
    pub amount: Option<i64>,
    /// Currency code
    pub currency: Option<String>,
}

/// Square Customer object.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SquareCustomer {
    /// Customer ID
    pub id: String,
    /// Created at timestamp
    pub created_at: Option<String>,
    /// Updated at timestamp
    pub updated_at: Option<String>,
    /// Given name
    pub given_name: Option<String>,
    /// Family name
    pub family_name: Option<String>,
    /// Email address
    pub email_address: Option<String>,
    /// Phone number
    pub phone_number: Option<String>,
    /// Company name
    pub company_name: Option<String>,
    /// Nickname
    pub nickname: Option<String>,
    /// Address
    pub address: Option<Address>,
}

/// Square Address object.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Address {
    pub address_line_1: Option<String>,
    pub address_line_2: Option<String>,
    pub locality: Option<String>,
    pub administrative_district_level_1: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
}

/// Square Order object.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SquareOrder {
    /// Order ID
    pub id: String,
    /// Location ID
    pub location_id: Option<String>,
    /// Customer ID
    pub customer_id: Option<String>,
    /// Order state
    pub state: Option<String>,
    /// Created at timestamp
    pub created_at: Option<String>,
    /// Updated at timestamp
    pub updated_at: Option<String>,
    /// Line items
    #[serde(default)]
    pub line_items: Vec<OrderLineItem>,
    /// Total money
    pub total_money: Option<Money>,
    /// Net amounts
    pub net_amounts: Option<NetAmounts>,
}

/// Square order line item.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OrderLineItem {
    pub uid: Option<String>,
    pub catalog_object_id: Option<String>,
    pub name: Option<String>,
    pub quantity: Option<String>,
    pub base_price_money: Option<Money>,
    pub total_money: Option<Money>,
}

/// Square net amounts.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NetAmounts {
    pub total_money: Option<Money>,
    pub tax_money: Option<Money>,
    pub discount_money: Option<Money>,
    pub tip_money: Option<Money>,
}

/// Square Payment object.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SquarePayment {
    /// Payment ID
    pub id: String,
    /// Order ID
    pub order_id: Option<String>,
    /// Customer ID
    pub customer_id: Option<String>,
    /// Location ID
    pub location_id: Option<String>,
    /// Payment status
    pub status: Option<String>,
    /// Amount money
    pub amount_money: Option<Money>,
    /// Source type
    pub source_type: Option<String>,
    /// Created at timestamp
    pub created_at: Option<String>,
    /// Updated at timestamp
    pub updated_at: Option<String>,
}

/// Square Inventory count.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SquareInventoryCount {
    /// Catalog object ID
    pub catalog_object_id: Option<String>,
    /// Location ID
    pub location_id: Option<String>,
    /// Quantity
    pub quantity: Option<String>,
    /// State
    pub state: Option<String>,
    /// Calculated at
    pub calculated_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_event_type_customer() {
        let event = SquareEvent {
            merchant_id: "MERCHANT123".to_string(),
            event_type: "customer.created".to_string(),
            event_id: "event123".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            data: EventData {
                data_type: Some("customer".to_string()),
                id: Some("cust123".to_string()),
                object: serde_json::json!({}),
            },
        };

        let (object_type, action) = event.parse_event_type().unwrap();
        assert_eq!(object_type, SquareObjectType::Customer);
        assert_eq!(action, EventAction::Created);
    }

    #[test]
    fn test_parse_event_type_catalog() {
        let event = SquareEvent {
            merchant_id: "MERCHANT123".to_string(),
            event_type: "catalog.version.updated".to_string(),
            event_id: "event123".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            data: EventData {
                data_type: Some("catalog".to_string()),
                id: None,
                object: serde_json::json!({"catalog_version": 123}),
            },
        };

        let (object_type, action) = event.parse_event_type().unwrap();
        assert_eq!(object_type, SquareObjectType::Catalog);
        assert_eq!(action, EventAction::VersionUpdated);
    }

    #[test]
    fn test_square_object_type_from_str() {
        assert_eq!(
            SquareObjectType::parse_str("customer"),
            Some(SquareObjectType::Customer)
        );
        assert_eq!(
            SquareObjectType::parse_str("order"),
            Some(SquareObjectType::Order)
        );
        assert_eq!(
            SquareObjectType::parse_str("payment"),
            Some(SquareObjectType::Payment)
        );
        assert_eq!(
            SquareObjectType::parse_str("catalog"),
            Some(SquareObjectType::Catalog)
        );
        assert_eq!(SquareObjectType::parse_str("unknown"), None);
    }
}
