//! Sync configuration entity and repository.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use value_object::TenantId;

use crate::SyncTarget;

/// Sync configuration ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SyncConfigId(String);

impl SyncConfigId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn generate() -> Self {
        Self(value_object::Ulid::new().to_string().to_lowercase())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SyncConfigId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for SyncConfigId {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

/// Data ID for synchronization
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DataId(String);

impl DataId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DataId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for DataId {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

/// Synchronization status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStatus {
    /// Never synchronized
    NeverSynced,
    /// Successfully synchronized
    Synced,
    /// Synchronization pending
    Pending,
    /// Synchronization failed
    Failed(String),
}

impl SyncStatus {
    pub fn as_str(&self) -> &str {
        match self {
            SyncStatus::NeverSynced => "never_synced",
            SyncStatus::Synced => "synced",
            SyncStatus::Pending => "pending",
            SyncStatus::Failed(_) => "failed",
        }
    }

    pub fn error_message(&self) -> Option<&str> {
        match self {
            SyncStatus::Failed(msg) => Some(msg),
            _ => None,
        }
    }
}

impl std::fmt::Display for SyncStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncStatus::NeverSynced => write!(f, "never_synced"),
            SyncStatus::Synced => write!(f, "synced"),
            SyncStatus::Pending => write!(f, "pending"),
            SyncStatus::Failed(msg) => write!(f, "failed: {msg}"),
        }
    }
}

/// Sync configuration entity.
///
/// Represents a configured synchronization between a data item and an external provider.
#[allow(clippy::too_many_arguments)]
#[derive(Debug, Clone, Getters, new)]
pub struct SyncConfig {
    id: SyncConfigId,
    tenant_id: TenantId,
    data_id: DataId,
    provider: String,
    target: SyncTarget,
    status: SyncStatus,
    last_synced_at: Option<DateTime<Utc>>,
    last_result_id: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl SyncConfig {
    /// Create a new sync configuration
    pub fn create(
        tenant_id: TenantId,
        data_id: DataId,
        provider: impl Into<String>,
        target: SyncTarget,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: SyncConfigId::generate(),
            tenant_id,
            data_id,
            provider: provider.into(),
            target,
            status: SyncStatus::NeverSynced,
            last_synced_at: None,
            last_result_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Mark as successfully synced
    pub fn mark_synced(&mut self, result_id: impl Into<String>) {
        self.status = SyncStatus::Synced;
        self.last_synced_at = Some(Utc::now());
        self.last_result_id = Some(result_id.into());
        self.updated_at = Utc::now();
    }

    /// Mark as failed
    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.status = SyncStatus::Failed(error.into());
        self.updated_at = Utc::now();
    }

    /// Update target
    pub fn update_target(&mut self, target: SyncTarget) {
        self.target = target;
        self.updated_at = Utc::now();
    }
}

/// Repository for sync configurations.
#[async_trait]
pub trait SyncConfigRepository: Send + Sync + Debug {
    /// Save a sync configuration
    async fn save(&self, config: &SyncConfig) -> errors::Result<()>;

    /// Find by ID
    async fn find_by_id(
        &self,
        id: &SyncConfigId,
    ) -> errors::Result<Option<SyncConfig>>;

    /// Find by data ID
    async fn find_by_data_id(
        &self,
        data_id: &DataId,
    ) -> errors::Result<Option<SyncConfig>>;

    /// Find all configurations for a tenant and provider
    async fn find_by_tenant_and_provider(
        &self,
        tenant_id: &TenantId,
        provider: &str,
    ) -> errors::Result<Vec<SyncConfig>>;

    /// Find all configurations for a tenant
    async fn find_by_tenant(
        &self,
        tenant_id: &TenantId,
    ) -> errors::Result<Vec<SyncConfig>>;

    /// Delete a sync configuration
    async fn delete(&self, id: &SyncConfigId) -> errors::Result<()>;
}
