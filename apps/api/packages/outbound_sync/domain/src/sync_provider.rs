//! Sync provider trait and related types.
//!
//! This module defines the abstraction for synchronization providers,
//! allowing uniform access to various backends like GitHub, GitLab, S3, and CRM systems.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Provider type classification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderType {
    /// Git-based repositories (GitHub, GitLab, Bitbucket)
    GitRepository,
    /// Object storage services (S3, GCS, Azure Blob)
    ObjectStorage,
    /// CRM systems (HubSpot, Salesforce)
    Crm,
    /// Other provider types
    Other,
}

/// Authentication credentials for sync providers.
///
/// Different providers may use different fields:
/// - OAuth providers: `access_token` and optionally `refresh_token`
/// - API key providers: `api_key`
#[derive(Debug, Clone)]
pub struct SyncAuth {
    /// OAuth access token
    pub access_token: String,
    /// OAuth refresh token (if available)
    pub refresh_token: Option<String>,
    /// API key for non-OAuth providers
    pub api_key: Option<String>,
}

impl SyncAuth {
    /// Create a new SyncAuth with OAuth tokens
    pub fn oauth(access_token: impl Into<String>) -> Self {
        Self {
            access_token: access_token.into(),
            refresh_token: None,
            api_key: None,
        }
    }

    /// Create a new SyncAuth with OAuth tokens including refresh token
    pub fn oauth_with_refresh(
        access_token: impl Into<String>,
        refresh_token: impl Into<String>,
    ) -> Self {
        Self {
            access_token: access_token.into(),
            refresh_token: Some(refresh_token.into()),
            api_key: None,
        }
    }

    /// Create a new SyncAuth with API key
    pub fn api_key(api_key: impl Into<String>) -> Self {
        Self {
            access_token: String::new(),
            refresh_token: None,
            api_key: Some(api_key.into()),
        }
    }
}

/// Synchronization target specification.
///
/// Interpretation varies by provider:
/// - Git: `container` = "owner/repo", `resource` = "path/to/file.md"
/// - S3: `container` = "bucket-name", `resource` = "object/key"
/// - CRM: `container` = "contacts"/"deals", `resource` = record_id
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncTarget {
    /// Container name (repository, bucket, object type)
    pub container: String,
    /// Resource path or ID (file path, object key, record ID)
    pub resource: Option<String>,
    /// Version/branch/tag (optional)
    pub version: Option<String>,
}

impl SyncTarget {
    /// Create a new SyncTarget for Git-based providers
    pub fn git(repo: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            container: repo.into(),
            resource: Some(path.into()),
            version: None,
        }
    }

    /// Create a new SyncTarget for Git-based providers with branch
    pub fn git_with_branch(
        repo: impl Into<String>,
        path: impl Into<String>,
        branch: impl Into<String>,
    ) -> Self {
        Self {
            container: repo.into(),
            resource: Some(path.into()),
            version: Some(branch.into()),
        }
    }

    /// Create a new SyncTarget for CRM providers (existing record)
    pub fn crm(
        object_type: impl Into<String>,
        record_id: impl Into<String>,
    ) -> Self {
        Self {
            container: object_type.into(),
            resource: Some(record_id.into()),
            version: None,
        }
    }

    /// Create a new SyncTarget for CRM providers (new record)
    pub fn crm_new(object_type: impl Into<String>) -> Self {
        Self {
            container: object_type.into(),
            resource: None,
            version: None,
        }
    }
}

/// Synchronization payload containing content and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPayload {
    /// Content to synchronize
    pub content: String,
    /// Content type (e.g., "text/markdown", "application/json")
    pub content_type: String,
    /// Additional metadata
    pub metadata: SyncMetadata,
}

impl SyncPayload {
    /// Create a new markdown payload
    pub fn markdown(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            content_type: "text/markdown".to_string(),
            metadata: SyncMetadata::default(),
        }
    }

    /// Create a new markdown payload with commit message
    pub fn markdown_with_message(
        content: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            content: content.into(),
            content_type: "text/markdown".to_string(),
            metadata: SyncMetadata {
                message: Some(message.into()),
                properties: None,
            },
        }
    }

    /// Create a new JSON payload for CRM
    pub fn json(properties: serde_json::Value) -> Self {
        Self {
            content: serde_json::to_string(&properties).unwrap_or_default(),
            content_type: "application/json".to_string(),
            metadata: SyncMetadata {
                message: None,
                properties: Some(properties),
            },
        }
    }
}

/// Metadata for synchronization operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncMetadata {
    /// Commit message for Git providers
    pub message: Option<String>,
    /// Properties for CRM providers
    pub properties: Option<serde_json::Value>,
}

/// Remote data retrieved from a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteData {
    /// Unique identifier (SHA, ETag, record ID)
    pub id: String,
    /// Content
    pub content: String,
    /// Content type
    pub content_type: Option<String>,
    /// Size in bytes
    pub size: Option<u64>,
    /// Last updated timestamp
    pub updated_at: Option<DateTime<Utc>>,
}

/// Result of a synchronization operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// Result identifier (commit SHA, record ID, etc.)
    pub result_id: Option<String>,
    /// URL to the synchronized resource (if available)
    pub url: Option<String>,
    /// Diff preview (for dry-run operations)
    pub diff: Option<String>,
}

impl SyncResult {
    /// Create a successful result
    pub fn success(result_id: impl Into<String>) -> Self {
        Self {
            success: true,
            result_id: Some(result_id.into()),
            url: None,
            diff: None,
        }
    }

    /// Create a successful result with URL
    pub fn success_with_url(
        result_id: impl Into<String>,
        url: impl Into<String>,
    ) -> Self {
        Self {
            success: true,
            result_id: Some(result_id.into()),
            url: Some(url.into()),
            diff: None,
        }
    }

    /// Create a dry-run result with diff
    pub fn dry_run(diff: impl Into<String>) -> Self {
        Self {
            success: true,
            result_id: None,
            url: None,
            diff: Some(diff.into()),
        }
    }

    /// Create a failure result
    pub fn failure() -> Self {
        Self {
            success: false,
            result_id: None,
            url: None,
            diff: None,
        }
    }
}

/// Trait for synchronization providers.
///
/// Implementations provide access to various backends like GitHub, GitLab, S3, and CRM systems
/// through a unified interface.
///
/// # Example
///
/// ```ignore
/// let provider = GitHubSyncProvider::new();
/// let auth = SyncAuth::oauth("token");
/// let target = SyncTarget::git("owner/repo", "docs/article.md");
/// let payload = SyncPayload::markdown("# Hello\n\nWorld!");
///
/// let result = provider.put_data(&auth, &target, &payload).await?;
/// ```
#[async_trait]
pub trait SyncProvider: Send + Sync + Debug {
    /// Returns the provider name (e.g., "github", "gitlab", "s3", "hubspot")
    fn provider_name(&self) -> &'static str;

    /// Returns the provider type classification
    fn provider_type(&self) -> ProviderType;

    /// Retrieves data from the provider.
    ///
    /// Returns `None` if the resource does not exist.
    async fn get_data(
        &self,
        auth: &SyncAuth,
        target: &SyncTarget,
    ) -> errors::Result<Option<RemoteData>>;

    /// Creates or updates data on the provider.
    async fn put_data(
        &self,
        auth: &SyncAuth,
        target: &SyncTarget,
        payload: &SyncPayload,
    ) -> errors::Result<SyncResult>;

    /// Deletes data from the provider.
    async fn delete_data(
        &self,
        auth: &SyncAuth,
        target: &SyncTarget,
    ) -> errors::Result<SyncResult>;

    /// Checks if a resource exists on the provider.
    async fn exists(
        &self,
        auth: &SyncAuth,
        target: &SyncTarget,
    ) -> errors::Result<bool> {
        Ok(self.get_data(auth, target).await?.is_some())
    }
}
