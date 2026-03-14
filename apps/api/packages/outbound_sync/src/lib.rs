//! Database synchronization engine.
//!
//! This crate provides a unified interface for synchronizing database content
//! to various external providers such as GitHub, GitLab, S3, and CRM systems.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐
//! │  Application    │
//! │  (library-api)  │
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │  SyncData       │  Usecase
//! │  Usecase        │
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │  SyncProvider   │  Domain Trait
//! │  Registry       │
//! └────────┬────────┘
//!          │
//!     ┌────┴────┬─────────┐
//!     ▼         ▼         ▼
//! ┌───────┐ ┌───────┐ ┌───────┐
//! │GitHub │ │GitLab │ │  S3   │  Providers
//! └───────┘ └───────┘ └───────┘
//! ```
//!
//! # Example
//!
//! ```ignore
//! use outbound_sync::{SyncData, SyncDataInputData, SyncTarget, SyncPayload};
//!
//! let sync_data = SyncData::new(oauth_repo, config_repo, registry);
//!
//! let result = sync_data.execute(&SyncDataInputData {
//!     executor: &executor,
//!     multi_tenancy: &multi_tenancy,
//!     data_id: "data_123".to_string(),
//!     provider: "github".to_string(),
//!     target: SyncTarget::git("owner/repo", "docs/article.md"),
//!     payload: SyncPayload::markdown("# Hello World"),
//!     dry_run: false,
//! }).await?;
//! ```

pub mod interface_adapter;
pub mod providers;
pub mod usecase;

// Re-export domain types
pub use outbound_sync_domain::*;

// Re-export usecase types
pub use usecase::sync_data::{
    DeleteDataInputData, SyncData, SyncDataInputData, SyncDataInputPort,
    SyncDataResult,
};

// Re-export provider types
pub use providers::{build_default_registry, SyncProviderRegistry};
