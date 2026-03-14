//! Sync provider implementations.

pub mod github;
mod registry;

pub use github::GitHubSyncProvider;
pub use registry::{build_default_registry, SyncProviderRegistry};
