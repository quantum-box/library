//! HubSpot webhook payload types.
//!
//! HubSpot sends webhooks as an array of subscription events.
//! Each event contains information about the change to a CRM object.
//!
//! Reference: https://developers.hubspot.com/docs/api/webhooks

use serde::{Deserialize, Serialize};

/// HubSpot webhook payload (array of events).
pub type HubSpotWebhookPayload = Vec<HubSpotEvent>;

/// Individual HubSpot webhook event.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HubSpotEvent {
    /// Event ID
    pub event_id: i64,
    /// Subscription ID
    pub subscription_id: i64,
    /// Portal ID (HubSpot account ID)
    pub portal_id: i64,
    /// App ID
    pub app_id: i64,
    /// Timestamp when event occurred (milliseconds since epoch)
    pub occurred_at: i64,
    /// Event type (e.g., "contact.creation", "deal.propertyChange")
    pub subscription_type: String,
    /// Attempt number for this delivery
    pub attempt_number: i32,
    /// Object ID that was changed
    pub object_id: i64,
    /// Changed property name (for propertyChange events)
    #[serde(default)]
    pub property_name: Option<String>,
    /// Changed property value (for propertyChange events)
    #[serde(default)]
    pub property_value: Option<String>,
    /// Change source (e.g., "API", "FORM", "IMPORT")
    #[serde(default)]
    pub change_source: Option<String>,
    /// Source ID (for identifying the source of the change)
    #[serde(default)]
    pub source_id: Option<String>,
    /// Message ID (for timeline events)
    #[serde(default)]
    pub message_id: Option<String>,
    /// Message type (for timeline events)
    #[serde(default)]
    pub message_type: Option<String>,
}

impl HubSpotEvent {
    /// Parse the subscription type to extract object type and action.
    ///
    /// Examples:
    /// - "contact.creation" -> ("contact", "creation")
    /// - "deal.propertyChange" -> ("deal", "propertyChange")
    /// - "company.deletion" -> ("company", "deletion")
    pub fn parse_subscription_type(
        &self,
    ) -> Option<(ObjectType, EventAction)> {
        let parts: Vec<&str> = self.subscription_type.split('.').collect();
        if parts.len() != 2 {
            return None;
        }

        let object_type = ObjectType::parse_str(parts[0])?;
        let action = EventAction::parse_str(parts[1])?;

        Some((object_type, action))
    }

    /// Get the object type from the subscription type.
    pub fn object_type(&self) -> Option<ObjectType> {
        self.parse_subscription_type().map(|(t, _)| t)
    }

    /// Get the event action from the subscription type.
    pub fn event_action(&self) -> Option<EventAction> {
        self.parse_subscription_type().map(|(_, a)| a)
    }
}

/// HubSpot CRM object types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectType {
    Contact,
    Company,
    Deal,
    Product,
    Ticket,
    LineItem,
}

impl ObjectType {
    pub fn parse_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "contact" => Some(ObjectType::Contact),
            "company" => Some(ObjectType::Company),
            "deal" => Some(ObjectType::Deal),
            "product" => Some(ObjectType::Product),
            "ticket" => Some(ObjectType::Ticket),
            "line_item" | "lineitem" => Some(ObjectType::LineItem),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ObjectType::Contact => "contact",
            ObjectType::Company => "company",
            ObjectType::Deal => "deal",
            ObjectType::Product => "product",
            ObjectType::Ticket => "ticket",
            ObjectType::LineItem => "line_item",
        }
    }
}

/// HubSpot webhook event actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventAction {
    Creation,
    Deletion,
    PropertyChange,
    Merge,
    Restore,
    AssociationChange,
}

impl EventAction {
    pub fn parse_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "creation" => Some(EventAction::Creation),
            "deletion" => Some(EventAction::Deletion),
            "propertychange" => Some(EventAction::PropertyChange),
            "merge" => Some(EventAction::Merge),
            "restore" => Some(EventAction::Restore),
            "associationchange" => Some(EventAction::AssociationChange),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            EventAction::Creation => "creation",
            EventAction::Deletion => "deletion",
            EventAction::PropertyChange => "propertyChange",
            EventAction::Merge => "merge",
            EventAction::Restore => "restore",
            EventAction::AssociationChange => "associationChange",
        }
    }
}

/// HubSpot CRM object (generic structure fetched from API).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HubSpotObject {
    /// Object ID
    pub id: String,
    /// Object properties
    pub properties: serde_json::Value,
    /// Creation timestamp
    #[serde(rename = "createdAt")]
    pub created_at: String,
    /// Update timestamp
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    /// Whether the object is archived
    #[serde(default)]
    pub archived: bool,
}

impl HubSpotObject {
    /// Get a property value as a string.
    pub fn get_property(&self, name: &str) -> Option<&str> {
        self.properties.get(name).and_then(|v| v.as_str())
    }

    /// Get a property value as an i64.
    pub fn get_property_i64(&self, name: &str) -> Option<i64> {
        self.properties
            .get(name)
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
    }

    /// Get a property value as an f64.
    pub fn get_property_f64(&self, name: &str) -> Option<f64> {
        self.properties
            .get(name)
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_subscription_type() {
        let event = HubSpotEvent {
            event_id: 1,
            subscription_id: 1,
            portal_id: 12345,
            app_id: 1,
            occurred_at: 1234567890000,
            subscription_type: "contact.creation".to_string(),
            attempt_number: 1,
            object_id: 123,
            property_name: None,
            property_value: None,
            change_source: None,
            source_id: None,
            message_id: None,
            message_type: None,
        };

        let (object_type, action) =
            event.parse_subscription_type().unwrap();
        assert_eq!(object_type, ObjectType::Contact);
        assert_eq!(action, EventAction::Creation);
    }

    #[test]
    fn test_object_type_from_str() {
        assert_eq!(
            ObjectType::parse_str("contact"),
            Some(ObjectType::Contact)
        );
        assert_eq!(
            ObjectType::parse_str("COMPANY"),
            Some(ObjectType::Company)
        );
        assert_eq!(ObjectType::parse_str("deal"), Some(ObjectType::Deal));
        assert_eq!(
            ObjectType::parse_str("product"),
            Some(ObjectType::Product)
        );
        assert_eq!(ObjectType::parse_str("unknown"), None);
    }

    #[test]
    fn test_event_action_from_str() {
        assert_eq!(
            EventAction::parse_str("creation"),
            Some(EventAction::Creation)
        );
        assert_eq!(
            EventAction::parse_str("propertyChange"),
            Some(EventAction::PropertyChange)
        );
        assert_eq!(
            EventAction::parse_str("deletion"),
            Some(EventAction::Deletion)
        );
    }
}
