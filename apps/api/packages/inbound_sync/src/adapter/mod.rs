//! Adapters for external interfaces.

pub mod axum_handler;
pub mod graphql;
pub mod oauth_callback_handler;

pub use axum_handler::*;
pub use graphql::*;
pub use oauth_callback_handler::*;
