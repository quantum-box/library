//! Provider-level webhook secrets.

use std::collections::HashMap;

use inbound_sync_domain::Provider;

/// In-memory store for provider webhook secrets.
#[derive(Debug, Default)]
pub struct WebhookSecretStore {
    secrets: HashMap<Provider, String>,
}

impl WebhookSecretStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace a secret for a provider.
    pub fn insert(&mut self, provider: Provider, secret: String) {
        self.secrets.insert(provider, secret);
    }

    /// Get the secret for a provider.
    pub fn get(&self, provider: Provider) -> Option<&str> {
        self.secrets.get(&provider).map(|s| s.as_str())
    }
}
