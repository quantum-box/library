//! Provider registry for managing multiple sync providers.

use std::collections::HashMap;
use std::sync::Arc;

use outbound_sync_domain::SyncProvider;

use super::GitHubSyncProvider;

/// Registry for sync providers.
///
/// Allows registration and lookup of providers by name.
#[derive(Debug, Default)]
pub struct SyncProviderRegistry {
    providers: HashMap<String, Arc<dyn SyncProvider>>,
}

impl SyncProviderRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a provider
    pub fn register(&mut self, provider: Arc<dyn SyncProvider>) {
        self.providers
            .insert(provider.provider_name().to_string(), provider);
    }

    /// Get a provider by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn SyncProvider>> {
        self.providers.get(name).cloned()
    }

    /// Get all available provider names
    pub fn available_providers(&self) -> Vec<&str> {
        self.providers.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a provider is registered
    pub fn has_provider(&self, name: &str) -> bool {
        self.providers.contains_key(name)
    }
}

/// Build a registry with default providers.
///
/// Currently includes:
/// - GitHub
///
/// Future providers:
/// - GitLab
/// - S3
/// - HubSpot
pub fn build_default_registry() -> SyncProviderRegistry {
    let mut registry = SyncProviderRegistry::new();
    registry.register(Arc::new(GitHubSyncProvider::new()));
    // Future: registry.register(Arc::new(GitLabSyncProvider::new()));
    // Future: registry.register(Arc::new(S3SyncProvider::new()));
    // Future: registry.register(Arc::new(HubSpotSyncProvider::new()));
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry() {
        let registry = build_default_registry();

        assert!(registry.has_provider("github"));
        assert!(!registry.has_provider("gitlab"));

        let providers = registry.available_providers();
        assert!(providers.contains(&"github"));
    }
}
