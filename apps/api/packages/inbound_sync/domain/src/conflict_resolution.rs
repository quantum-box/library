//! Conflict resolution strategies for bidirectional sync.
//!
//! When the same data is modified both locally (Library) and externally (SaaS),
//! a conflict occurs. This module provides strategies to resolve such conflicts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::SyncState;

/// Strategy for resolving conflicts when data is modified in both places.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolutionStrategy {
    /// The last write wins based on timestamp comparison.
    /// This is the default strategy.
    #[default]
    LastWriteWins,
    /// External (source) data always wins.
    SourceWins,
    /// Local (Library) data always wins.
    LocalWins,
    /// Conflict is flagged for manual resolution.
    Manual,
}

impl ConflictResolutionStrategy {
    /// Get strategy from string.
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "last_write_wins" => Some(Self::LastWriteWins),
            "source_wins" => Some(Self::SourceWins),
            "local_wins" => Some(Self::LocalWins),
            "manual" => Some(Self::Manual),
            _ => None,
        }
    }

    /// Convert to string.
    pub fn as_str(&self) -> &str {
        match self {
            Self::LastWriteWins => "last_write_wins",
            Self::SourceWins => "source_wins",
            Self::LocalWins => "local_wins",
            Self::Manual => "manual",
        }
    }
}

/// Result of conflict detection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictDetectionResult {
    /// No conflict - data can be safely updated.
    NoConflict,
    /// External data is newer.
    ExternalNewer,
    /// Local data is newer.
    LocalNewer,
    /// Both modified at the same time or timestamps unavailable.
    Indeterminate,
}

/// Information about a detected conflict.
#[derive(Debug, Clone)]
pub struct ConflictInfo {
    /// The sync state at the time of conflict.
    pub sync_state: SyncState,
    /// External version that caused the conflict.
    pub external_version: String,
    /// External timestamp (if available).
    pub external_updated_at: Option<DateTime<Utc>>,
    /// Local version at conflict time.
    pub local_version: Option<String>,
    /// Local timestamp at conflict time.
    pub local_updated_at: Option<DateTime<Utc>>,
    /// Detection result.
    pub detection_result: ConflictDetectionResult,
}

/// Resolution outcome after applying a strategy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictResolutionOutcome {
    /// Apply the external change (overwrite local).
    ApplyExternal,
    /// Keep local data (ignore external change).
    KeepLocal,
    /// Flag for manual review.
    RequiresManualReview,
    /// Create a backup version before overwriting.
    ApplyExternalWithBackup,
}

/// Conflict resolver that applies strategies to resolve conflicts.
#[derive(Debug, Clone)]
pub struct ConflictResolver {
    strategy: ConflictResolutionStrategy,
    /// Whether to create a backup version before overwriting.
    create_backup: bool,
}

impl Default for ConflictResolver {
    fn default() -> Self {
        Self::new(ConflictResolutionStrategy::LastWriteWins)
    }
}

impl ConflictResolver {
    /// Create a new conflict resolver with the given strategy.
    pub fn new(strategy: ConflictResolutionStrategy) -> Self {
        Self {
            strategy,
            create_backup: false,
        }
    }

    /// Enable backup creation before overwriting.
    pub fn with_backup(mut self) -> Self {
        self.create_backup = true;
        self
    }

    /// Get the current strategy.
    pub fn strategy(&self) -> ConflictResolutionStrategy {
        self.strategy
    }

    /// Check if backup creation is enabled.
    pub fn creates_backup(&self) -> bool {
        self.create_backup
    }

    /// Detect if there's a conflict between external and local data.
    ///
    /// # Arguments
    /// * `sync_state` - Current sync state (may be None for new items)
    /// * `external_version` - Version/ETag from external source
    /// * `external_updated_at` - Timestamp from external source
    /// * `local_updated_at` - Local modification timestamp
    ///
    /// # Returns
    /// `ConflictDetectionResult` indicating the conflict state.
    pub fn detect_conflict(
        &self,
        sync_state: Option<&SyncState>,
        external_version: &str,
        external_updated_at: Option<DateTime<Utc>>,
        local_updated_at: Option<DateTime<Utc>>,
    ) -> ConflictDetectionResult {
        // If no sync state exists, this is a new item - no conflict
        let Some(state) = sync_state else {
            return ConflictDetectionResult::NoConflict;
        };

        // If external version hasn't changed since last sync, no conflict
        if !state.has_external_changed(external_version) {
            return ConflictDetectionResult::NoConflict;
        }

        // If local hasn't been modified since last sync, external is newer
        if let Some(_local_ver) = state.local_version() {
            if local_updated_at.is_none_or(|t| t <= *state.last_synced_at())
            {
                return ConflictDetectionResult::ExternalNewer;
            }
        } else {
            // No local version recorded - assume external is newer
            return ConflictDetectionResult::ExternalNewer;
        }

        // Both have been modified - compare timestamps if available
        match (external_updated_at, local_updated_at) {
            (Some(ext_time), Some(local_time)) => {
                if ext_time > local_time {
                    ConflictDetectionResult::ExternalNewer
                } else if local_time > ext_time {
                    ConflictDetectionResult::LocalNewer
                } else {
                    ConflictDetectionResult::Indeterminate
                }
            }
            _ => ConflictDetectionResult::Indeterminate,
        }
    }

    /// Resolve a conflict using the configured strategy.
    ///
    /// # Arguments
    /// * `detection_result` - Result from conflict detection
    ///
    /// # Returns
    /// `ConflictResolutionOutcome` indicating what action to take.
    pub fn resolve(
        &self,
        detection_result: &ConflictDetectionResult,
    ) -> ConflictResolutionOutcome {
        match detection_result {
            ConflictDetectionResult::NoConflict
            | ConflictDetectionResult::ExternalNewer => {
                if self.create_backup {
                    ConflictResolutionOutcome::ApplyExternalWithBackup
                } else {
                    ConflictResolutionOutcome::ApplyExternal
                }
            }
            ConflictDetectionResult::LocalNewer => match self.strategy {
                ConflictResolutionStrategy::LastWriteWins => {
                    ConflictResolutionOutcome::KeepLocal
                }
                ConflictResolutionStrategy::SourceWins => {
                    if self.create_backup {
                        ConflictResolutionOutcome::ApplyExternalWithBackup
                    } else {
                        ConflictResolutionOutcome::ApplyExternal
                    }
                }
                ConflictResolutionStrategy::LocalWins => {
                    ConflictResolutionOutcome::KeepLocal
                }
                ConflictResolutionStrategy::Manual => {
                    ConflictResolutionOutcome::RequiresManualReview
                }
            },
            ConflictDetectionResult::Indeterminate => match self.strategy {
                ConflictResolutionStrategy::LastWriteWins
                | ConflictResolutionStrategy::SourceWins => {
                    // Default to external when timestamps are equal or unavailable
                    if self.create_backup {
                        ConflictResolutionOutcome::ApplyExternalWithBackup
                    } else {
                        ConflictResolutionOutcome::ApplyExternal
                    }
                }
                ConflictResolutionStrategy::LocalWins => {
                    ConflictResolutionOutcome::KeepLocal
                }
                ConflictResolutionStrategy::Manual => {
                    ConflictResolutionOutcome::RequiresManualReview
                }
            },
        }
    }

    /// Convenience method to detect and resolve in one step.
    ///
    /// Returns the resolution outcome based on the current sync state and
    /// incoming external data.
    pub fn detect_and_resolve(
        &self,
        sync_state: Option<&SyncState>,
        external_version: &str,
        external_updated_at: Option<DateTime<Utc>>,
        local_updated_at: Option<DateTime<Utc>>,
    ) -> (ConflictDetectionResult, ConflictResolutionOutcome) {
        let detection = self.detect_conflict(
            sync_state,
            external_version,
            external_updated_at,
            local_updated_at,
        );
        let outcome = self.resolve(&detection);
        (detection, outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SyncDirection, SyncStateId, WebhookEndpointId};
    use chrono::Duration;

    fn create_sync_state(
        external_ver: Option<&str>,
        local_ver: Option<&str>,
    ) -> SyncState {
        SyncState::new(
            SyncStateId::default(),
            WebhookEndpointId::default(),
            "data_123".to_string(),
            "ext_456".to_string(),
            external_ver.map(String::from),
            local_ver.map(String::from),
            SyncDirection::Inbound,
            Utc::now() - Duration::hours(1),
            None,
        )
    }

    #[test]
    fn test_no_conflict_when_no_sync_state() {
        let resolver = ConflictResolver::default();
        let result =
            resolver.detect_conflict(None, "v1", Some(Utc::now()), None);
        assert_eq!(result, ConflictDetectionResult::NoConflict);
    }

    #[test]
    fn test_no_conflict_when_external_unchanged() {
        let resolver = ConflictResolver::default();
        let state = create_sync_state(Some("v1"), Some("local_v1"));

        let result = resolver.detect_conflict(
            Some(&state),
            "v1",
            Some(Utc::now()),
            None,
        );
        assert_eq!(result, ConflictDetectionResult::NoConflict);
    }

    #[test]
    fn test_external_newer_when_local_not_modified() {
        let resolver = ConflictResolver::default();
        let state = create_sync_state(Some("v1"), None);
        let now = Utc::now();

        let result =
            resolver.detect_conflict(Some(&state), "v2", Some(now), None);
        assert_eq!(result, ConflictDetectionResult::ExternalNewer);
    }

    #[test]
    fn test_last_write_wins_external_newer() {
        let resolver = ConflictResolver::new(
            ConflictResolutionStrategy::LastWriteWins,
        );
        let outcome =
            resolver.resolve(&ConflictDetectionResult::ExternalNewer);
        assert_eq!(outcome, ConflictResolutionOutcome::ApplyExternal);
    }

    #[test]
    fn test_last_write_wins_local_newer() {
        let resolver = ConflictResolver::new(
            ConflictResolutionStrategy::LastWriteWins,
        );
        let outcome =
            resolver.resolve(&ConflictDetectionResult::LocalNewer);
        assert_eq!(outcome, ConflictResolutionOutcome::KeepLocal);
    }

    #[test]
    fn test_source_wins_always() {
        let resolver =
            ConflictResolver::new(ConflictResolutionStrategy::SourceWins);

        let outcome =
            resolver.resolve(&ConflictDetectionResult::ExternalNewer);
        assert_eq!(outcome, ConflictResolutionOutcome::ApplyExternal);

        let outcome =
            resolver.resolve(&ConflictDetectionResult::LocalNewer);
        assert_eq!(outcome, ConflictResolutionOutcome::ApplyExternal);
    }

    #[test]
    fn test_local_wins_always() {
        let resolver =
            ConflictResolver::new(ConflictResolutionStrategy::LocalWins);

        let outcome =
            resolver.resolve(&ConflictDetectionResult::LocalNewer);
        assert_eq!(outcome, ConflictResolutionOutcome::KeepLocal);

        let outcome =
            resolver.resolve(&ConflictDetectionResult::Indeterminate);
        assert_eq!(outcome, ConflictResolutionOutcome::KeepLocal);
    }

    #[test]
    fn test_manual_strategy() {
        let resolver =
            ConflictResolver::new(ConflictResolutionStrategy::Manual);

        let outcome =
            resolver.resolve(&ConflictDetectionResult::LocalNewer);
        assert_eq!(
            outcome,
            ConflictResolutionOutcome::RequiresManualReview
        );

        let outcome =
            resolver.resolve(&ConflictDetectionResult::Indeterminate);
        assert_eq!(
            outcome,
            ConflictResolutionOutcome::RequiresManualReview
        );
    }

    #[test]
    fn test_backup_option() {
        let resolver = ConflictResolver::new(
            ConflictResolutionStrategy::LastWriteWins,
        )
        .with_backup();

        let outcome =
            resolver.resolve(&ConflictDetectionResult::ExternalNewer);
        assert_eq!(
            outcome,
            ConflictResolutionOutcome::ApplyExternalWithBackup
        );
    }

    #[test]
    fn test_strategy_from_str() {
        assert_eq!(
            ConflictResolutionStrategy::parse_str("last_write_wins"),
            Some(ConflictResolutionStrategy::LastWriteWins)
        );
        assert_eq!(
            ConflictResolutionStrategy::parse_str("source_wins"),
            Some(ConflictResolutionStrategy::SourceWins)
        );
        assert_eq!(
            ConflictResolutionStrategy::parse_str("local_wins"),
            Some(ConflictResolutionStrategy::LocalWins)
        );
        assert_eq!(
            ConflictResolutionStrategy::parse_str("manual"),
            Some(ConflictResolutionStrategy::Manual)
        );
        assert_eq!(ConflictResolutionStrategy::parse_str("invalid"), None);
    }
}
