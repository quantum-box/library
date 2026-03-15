//! Sync state entity for tracking synchronization between external and local data.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use ulid::Ulid;
use util::{def_id, def_id_serde_impls};

use crate::WebhookEndpointId;

def_id!(SyncStateId, "sst_");

/// Direction of synchronization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncDirection {
    /// Data flows from external service to Library
    Inbound,
    /// Data flows from Library to external service
    Outbound,
    /// Bidirectional synchronization
    Both,
}

impl SyncDirection {
    pub fn as_str(&self) -> &str {
        match self {
            SyncDirection::Inbound => "inbound",
            SyncDirection::Outbound => "outbound",
            SyncDirection::Both => "both",
        }
    }
}

impl std::fmt::Display for SyncDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for SyncDirection {
    type Err = errors::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "inbound" => Ok(SyncDirection::Inbound),
            "outbound" => Ok(SyncDirection::Outbound),
            "both" => Ok(SyncDirection::Both),
            _ => Err(errors::Error::invalid(format!(
                "Invalid sync direction: {s}"
            ))),
        }
    }
}

/// Sync state entity.
///
/// Tracks the synchronization relationship between an external resource
/// and a Library data item. Used for:
/// - Mapping external IDs to Library data IDs
/// - Conflict detection via version tracking
/// - Preventing duplicate processing
#[derive(Debug, Clone, Getters, new)]
#[allow(clippy::too_many_arguments)]
pub struct SyncState {
    /// Unique identifier
    id: SyncStateId,
    /// Webhook endpoint this state belongs to
    endpoint_id: WebhookEndpointId,
    /// Library data ID
    data_id: String,
    /// External service resource ID
    external_id: String,
    /// External version/ETag (for conflict detection)
    #[getter(skip)]
    external_version: Option<String>,
    /// Local version (for conflict detection)
    #[getter(skip)]
    local_version: Option<String>,
    /// Synchronization direction
    sync_direction: SyncDirection,
    /// Last successful sync timestamp
    last_synced_at: DateTime<Utc>,
    /// Additional metadata
    #[getter(skip)]
    metadata: Option<serde_json::Value>,
}

impl SyncState {
    /// Create a new sync state.
    pub fn create(
        endpoint_id: WebhookEndpointId,
        data_id: impl Into<String>,
        external_id: impl Into<String>,
        direction: SyncDirection,
    ) -> Self {
        Self {
            id: SyncStateId::default(),
            endpoint_id,
            data_id: data_id.into(),
            external_id: external_id.into(),
            external_version: None,
            local_version: None,
            sync_direction: direction,
            last_synced_at: Utc::now(),
            metadata: None,
        }
    }

    /// Get external version.
    pub fn external_version(&self) -> Option<&str> {
        self.external_version.as_deref()
    }

    /// Get local version.
    pub fn local_version(&self) -> Option<&str> {
        self.local_version.as_deref()
    }

    /// Get metadata.
    pub fn metadata(&self) -> Option<&serde_json::Value> {
        self.metadata.as_ref()
    }

    /// Update after inbound sync (external → Library).
    pub fn update_inbound(
        &mut self,
        external_version: Option<String>,
        local_version: Option<String>,
    ) {
        self.external_version = external_version;
        self.local_version = local_version;
        self.last_synced_at = Utc::now();
    }

    /// Update after outbound sync (Library → external).
    pub fn update_outbound(
        &mut self,
        external_version: Option<String>,
        local_version: Option<String>,
    ) {
        self.external_version = external_version;
        self.local_version = local_version;
        self.last_synced_at = Utc::now();
    }

    /// Set metadata.
    pub fn set_metadata(&mut self, metadata: Option<serde_json::Value>) {
        self.metadata = metadata;
    }

    /// Check if external version has changed.
    pub fn has_external_changed(&self, new_version: &str) -> bool {
        match &self.external_version {
            Some(current) => current != new_version,
            None => true,
        }
    }

    /// Check if local version has changed.
    pub fn has_local_changed(&self, new_version: &str) -> bool {
        match &self.local_version {
            Some(current) => current != new_version,
            None => true,
        }
    }
}

/// Repository for sync states.
#[async_trait]
pub trait SyncStateRepository: Send + Sync + Debug {
    /// Save a sync state.
    async fn save(&self, state: &SyncState) -> errors::Result<()>;

    /// Find by ID.
    async fn find_by_id(
        &self,
        id: &SyncStateId,
    ) -> errors::Result<Option<SyncState>>;

    /// Find by endpoint and external ID.
    async fn find_by_external_id(
        &self,
        endpoint_id: &WebhookEndpointId,
        external_id: &str,
    ) -> errors::Result<Option<SyncState>>;

    /// Find by endpoint and data ID.
    async fn find_by_data_id(
        &self,
        endpoint_id: &WebhookEndpointId,
        data_id: &str,
    ) -> errors::Result<Option<SyncState>>;

    /// Find all sync states for an endpoint.
    async fn find_by_endpoint(
        &self,
        endpoint_id: &WebhookEndpointId,
    ) -> errors::Result<Vec<SyncState>>;

    /// Delete a sync state.
    async fn delete(&self, id: &SyncStateId) -> errors::Result<()>;

    /// Delete all sync states for an endpoint.
    async fn delete_by_endpoint(
        &self,
        endpoint_id: &WebhookEndpointId,
    ) -> errors::Result<u64>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_state_version_tracking() {
        let mut state = SyncState::create(
            WebhookEndpointId::default(),
            "data_123",
            "ext_456",
            SyncDirection::Inbound,
        );

        // Initial state - no versions
        assert!(state.has_external_changed("v1"));
        assert!(state.has_local_changed("v1"));

        // After sync
        state.update_inbound(
            Some("v1".to_string()),
            Some("local_v1".to_string()),
        );

        assert!(!state.has_external_changed("v1"));
        assert!(state.has_external_changed("v2"));
        assert!(!state.has_local_changed("local_v1"));
        assert!(state.has_local_changed("local_v2"));
    }

    #[test]
    fn test_sync_direction() {
        assert_eq!(SyncDirection::Inbound.as_str(), "inbound");
        assert_eq!(SyncDirection::Outbound.as_str(), "outbound");
        assert_eq!(SyncDirection::Both.as_str(), "both");
    }
}
