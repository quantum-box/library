//! Gateway implementations for repository interfaces.

mod api_key_validator;
mod builtin_integrations;
mod noop_clients;
mod noop_data_handlers;
mod oauth_service;
mod sync_operation_repository;
mod sync_state_repository;
mod webhook_endpoint_repository;
mod webhook_event_repository;

pub use api_key_validator::*;
pub use builtin_integrations::*;
pub use noop_clients::*;
pub use noop_data_handlers::*;
pub use oauth_service::*;
pub use sync_operation_repository::*;
pub use sync_state_repository::*;
pub use webhook_endpoint_repository::*;
pub use webhook_event_repository::*;

// Repository implementations (moved from integration package)
mod connection_repository;
mod oauth_token_repository;

pub use connection_repository::SqlxConnectionRepository;
pub use oauth_token_repository::SqlxOAuthTokenRepository;
