//! Library Inbound Webhook Synchronization Engine.
//!
//! This crate provides functionality for receiving webhooks from external SaaS
//! services (GitHub, Linear, HubSpot, Stripe, Notion, Airtable) and synchronizing
//! their data into Library.
//!
//! # Architecture
//!
//! ```text
//! External SaaS ──Webhook──▶ Library API ──API Fetch──▶ External SaaS
//!                               │                           │
//!                               │ Queue Event               │ Get Details
//!                               ▼                           ▼
//!                          Event Processor ──────▶ Library Database
//! ```
//!
//! # Key Components
//!
//! - [`WebhookEndpoint`]: Configuration for receiving webhooks from a provider
//! - [`WebhookEvent`]: A received webhook event queued for processing
//! - [`SyncState`]: Tracks synchronization state between external and local data
//! - [`WebhookVerifier`]: Verifies webhook signatures for security
//!
//! # Example
//!
//! ```ignore
//! use library_sync::{WebhookEndpoint, WebhookVerifier, Provider};
//!
//! // Create a webhook endpoint
//! let endpoint = WebhookEndpoint::create(
//!     tenant_id,
//!     "GitHub Docs Sync",
//!     Provider::Github,
//!     ProviderConfig::Github {
//!         repository: "owner/repo".to_string(),
//!         branch: "main".to_string(),
//!         path_pattern: Some("docs/**/*.md".to_string()),
//!     },
//!     vec!["push".to_string()],
//!     secret_hash,
//! );
//!
//! // Verify incoming webhook signature
//! let verifier = GitHubWebhookVerifier::new();
//! let is_valid = verifier.verify(payload, signature, secret)?;
//! ```

pub mod adapter;
pub mod interface_adapter;
pub mod providers;
pub mod sdk;
pub mod usecase;
pub mod webhook_secret_store;
pub mod webhook_verifier;

// Re-export domain types
pub use inbound_sync_domain::*;

// Re-export key types
pub use interface_adapter::{
    BuiltinIntegrationRegistry, SqlxConnectionRepository,
};
pub use webhook_secret_store::WebhookSecretStore;
pub use webhook_verifier::{WebhookVerifier, WebhookVerifierRegistry};
