//! Integration domain models for external service management.
//!
//! This module provides domain models for the integration marketplace,
//! enabling tenants to discover, connect, and manage external service
//! integrations.

mod marketplace;
pub mod oauth;

pub use marketplace::*;

// Re-export OAuth types for convenience
pub use oauth::{
    ExchangeOAuthCodeInput, InitOAuthInput, InitOAuthOutput,
    OAuthClientCredentials, OAuthProvider, OAuthService,
    OAuthTokenResponse, StoredOAuthToken, StoredOAuthTokenRepository,
};

// Re-export StoredOAuthTokenRepository as OAuthTokenRepository
// for backward compatibility
pub use oauth::StoredOAuthTokenRepository as OAuthTokenRepository;
