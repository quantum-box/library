//! Adapters for external interfaces.

pub mod axum_handler;
pub mod graphql;

pub use axum_handler::*;
pub use graphql::*;
