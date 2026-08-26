//! Provider type definitions for webhook sources.

use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use crate::OAuthProvider;

/// Supported webhook provider types.
///
/// Each provider has different webhook payload formats, signature verification
/// methods, and API endpoints for fetching detailed data.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumString,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    /// GitHub - repository and code management
    /// Webhook: X-Hub-Signature-256 (HMAC-SHA256)
    /// API: GitHub REST/GraphQL API
    Github,

    /// Linear - issue and project tracking
    /// Webhook: Linear-Signature (HMAC-SHA256)
    /// API: Linear GraphQL API
    Linear,

    /// HubSpot - CRM (contacts, companies, deals, products)
    /// Webhook: X-HubSpot-Signature-v3 (HMAC-SHA256)
    /// API: HubSpot CRM API
    Hubspot,

    /// Stripe - payments, products, subscriptions
    /// Webhook: Stripe-Signature (HMAC-SHA256 with timestamp)
    /// API: Stripe API
    Stripe,

    /// Square - POS, catalog, payments, customers, orders, inventory
    /// Webhook: X-Square-Hmacsha256-Signature (HMAC-SHA256)
    /// API: Square API
    Square,

    /// Notion - pages and databases
    /// Webhook: Custom signature verification
    /// API: Notion API
    Notion,

    /// Airtable - spreadsheet-like databases
    /// Webhook: Custom verification
    /// API: Airtable API
    Airtable,

    /// Generic webhook provider for custom integrations
    /// Webhook: Configurable HMAC-SHA256
    /// API: User-defined endpoints
    Generic,
}

/// Provider release readiness for GA surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderReadiness {
    Ga,
    Experimental,
    NonGa,
}

impl From<OAuthProvider> for Provider {
    fn from(p: OAuthProvider) -> Self {
        match p {
            OAuthProvider::Github => Provider::Github,
            OAuthProvider::Linear => Provider::Linear,
            OAuthProvider::Hubspot => Provider::Hubspot,
            OAuthProvider::Stripe => Provider::Stripe,
            OAuthProvider::Square => Provider::Square,
            OAuthProvider::Notion => Provider::Notion,
            OAuthProvider::Airtable => Provider::Airtable,
            OAuthProvider::Slack => Provider::Generic,
            OAuthProvider::Discord => Provider::Generic,
            OAuthProvider::Generic => Provider::Generic,
            OAuthProvider::Custom => Provider::Generic,
        }
    }
}

impl From<Provider> for OAuthProvider {
    fn from(p: Provider) -> Self {
        match p {
            Provider::Github => OAuthProvider::Github,
            Provider::Linear => OAuthProvider::Linear,
            Provider::Hubspot => OAuthProvider::Hubspot,
            Provider::Stripe => OAuthProvider::Stripe,
            Provider::Square => OAuthProvider::Square,
            Provider::Notion => OAuthProvider::Notion,
            Provider::Airtable => OAuthProvider::Airtable,
            Provider::Generic => OAuthProvider::Generic,
        }
    }
}

impl Provider {
    pub fn readiness(&self) -> ProviderReadiness {
        match self {
            Provider::Github | Provider::Linear | Provider::Generic => {
                ProviderReadiness::Ga
            }
            Provider::Square => ProviderReadiness::Experimental,
            Provider::Hubspot
            | Provider::Stripe
            | Provider::Notion
            | Provider::Airtable => ProviderReadiness::NonGa,
        }
    }

    pub fn unavailable_reason(&self) -> Option<&'static str> {
        match self {
            Provider::Hubspot => Some(
                "HubSpot is not GA because the runtime still uses NoOp HubSpot client/data handlers.",
            ),
            Provider::Stripe => Some(
                "Stripe is not GA because the runtime still uses NoOp Stripe client/data handlers.",
            ),
            Provider::Notion => Some(
                "Notion is not GA because the runtime still uses NoOp Notion client/data handlers.",
            ),
            Provider::Square => Some(
                "Square is experimental and requires LIBRARY_ENABLE_EXPERIMENTAL_INTEGRATIONS=true plus SQUARE_API_KEY.",
            ),
            Provider::Airtable => Some(
                "Airtable is not GA because no runtime client, data handler, or API pull processor is wired.",
            ),
            Provider::Github | Provider::Linear | Provider::Generic => None,
        }
    }

    pub fn is_runtime_available(&self) -> bool {
        match self {
            Provider::Github | Provider::Linear | Provider::Generic => true,
            Provider::Square => {
                std::env::var("LIBRARY_ENABLE_EXPERIMENTAL_INTEGRATIONS")
                    .map(|value| value.eq_ignore_ascii_case("true"))
                    .unwrap_or(false)
                    && std::env::var("SQUARE_API_KEY")
                        .map(|value| !value.trim().is_empty())
                        .unwrap_or(false)
            }
            Provider::Hubspot
            | Provider::Stripe
            | Provider::Notion
            | Provider::Airtable => false,
        }
    }

    pub fn ensure_runtime_available(&self) -> errors::Result<()> {
        if self.is_runtime_available() {
            return Ok(());
        }

        Err(errors::Error::bad_request(
            self.unavailable_reason()
                .unwrap_or("Integration is not available in this runtime."),
        ))
    }

    /// Get the signature header name for this provider.
    pub fn signature_header(&self) -> &'static str {
        match self {
            Provider::Github => "x-hub-signature-256",
            Provider::Linear => "linear-signature",
            Provider::Hubspot => "x-hubspot-signature-v3",
            Provider::Stripe => "stripe-signature",
            Provider::Square => "x-square-hmacsha256-signature",
            Provider::Notion => "x-notion-signature",
            Provider::Airtable => "x-airtable-signature",
            Provider::Generic => "x-signature-256",
        }
    }

    /// Get the event type header name for this provider (if any).
    pub fn event_header(&self) -> Option<&'static str> {
        match self {
            Provider::Github => Some("x-github-event"),
            Provider::Linear => None, // Event type is in payload
            Provider::Hubspot => None,
            Provider::Stripe => None, // Event type is in payload
            Provider::Square => None, // Event type is in payload
            Provider::Notion => None,
            Provider::Airtable => None,
            Provider::Generic => Some("x-event-type"),
        }
    }

    /// Check if this provider uses timestamp in signature verification.
    pub fn uses_timestamp_signature(&self) -> bool {
        matches!(self, Provider::Stripe)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_provider_serialization() {
        assert_eq!(
            serde_json::to_string(&Provider::Github).unwrap(),
            "\"github\""
        );
        assert_eq!(
            serde_json::to_string(&Provider::Linear).unwrap(),
            "\"linear\""
        );
        assert_eq!(
            serde_json::to_string(&Provider::Hubspot).unwrap(),
            "\"hubspot\""
        );
        assert_eq!(
            serde_json::to_string(&Provider::Stripe).unwrap(),
            "\"stripe\""
        );
        assert_eq!(
            serde_json::to_string(&Provider::Square).unwrap(),
            "\"square\""
        );
    }

    #[test]
    fn test_provider_from_str() {
        assert_eq!(Provider::from_str("github").unwrap(), Provider::Github);
        assert_eq!(Provider::from_str("linear").unwrap(), Provider::Linear);
        assert_eq!(
            Provider::from_str("hubspot").unwrap(),
            Provider::Hubspot
        );
        assert_eq!(Provider::from_str("stripe").unwrap(), Provider::Stripe);
        assert_eq!(Provider::from_str("square").unwrap(), Provider::Square);
    }

    #[test]
    fn test_signature_headers() {
        assert_eq!(
            Provider::Github.signature_header(),
            "x-hub-signature-256"
        );
        assert_eq!(Provider::Stripe.signature_header(), "stripe-signature");
    }

    #[test]
    fn test_provider_readiness() {
        assert_eq!(Provider::Linear.readiness(), ProviderReadiness::Ga);
        assert_eq!(
            Provider::Square.readiness(),
            ProviderReadiness::Experimental
        );
        assert_eq!(Provider::Github.readiness(), ProviderReadiness::Ga);
        assert!(Provider::Github.unavailable_reason().is_none());
    }

    #[test]
    fn test_non_ga_providers_are_not_runtime_available() {
        assert!(Provider::Linear.is_runtime_available());
        assert!(Provider::Github.is_runtime_available());
        assert!(!Provider::Hubspot.is_runtime_available());
        assert!(!Provider::Stripe.is_runtime_available());
        assert!(!Provider::Notion.is_runtime_available());
        assert!(!Provider::Airtable.is_runtime_available());
    }
}
