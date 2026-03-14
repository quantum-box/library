//! Domain models for Library inbound webhook synchronization.
//!
//! This module provides abstractions for receiving webhooks from external SaaS
//! (GitHub, Linear, HubSpot, Stripe, Notion, etc.) and synchronizing their data
//! into Library.
//!
//! The marketplace module provides integration app store functionality,
//! allowing tenants to discover, connect, and manage external service
//! integrations with OAuth support.

mod api_key;
mod conflict_resolution;
mod property_mapping;
mod provider;
mod sync_operation;
mod sync_state;
mod webhook_endpoint;
mod webhook_event;

pub use api_key::*;
pub use conflict_resolution::*;
pub use property_mapping::*;
pub use provider::*;
pub use sync_operation::*;
pub use sync_state::*;
pub use webhook_endpoint::*;
pub use webhook_event::*;

// Re-export OAuth types from integration_domain
pub use integration_domain::{
    ExchangeOAuthCodeInput, InitOAuthInput, InitOAuthOutput,
    OAuthClientCredentials, OAuthProvider, OAuthService,
    OAuthTokenRepository, OAuthTokenResponse, StoredOAuthToken,
};

// Re-export marketplace types from integration_domain
pub use integration_domain::{
    Connection, ConnectionId, ConnectionRepository, ConnectionStatus,
    Integration, IntegrationCategory, IntegrationId, IntegrationRepository,
    OAuthConfig, SyncCapability,
};
