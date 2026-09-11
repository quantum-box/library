//! Integration domain models for external service management.
//!
//! This module provides domain models for the integration marketplace,
//! enabling tenants to discover, connect, and manage external service
//! integrations.

mod external_sync;
mod external_sync_lifecycle;
mod marketplace;
pub mod oauth;

pub use external_sync::*;
pub use external_sync_lifecycle::*;
pub use marketplace::*;

// Re-export OAuth types for convenience
pub use oauth::{
    ExchangeOAuthCodeInput, InitOAuthInput, InitOAuthOutput,
    OAuthClientCredentials, OAuthCredentialsSource, OAuthProvider,
    OAuthService, OAuthTokenResponse, StoredOAuthToken,
    StoredOAuthTokenRepository,
};

// Re-export StoredOAuthTokenRepository as OAuthTokenRepository
// for backward compatibility
pub use oauth::StoredOAuthTokenRepository as OAuthTokenRepository;
