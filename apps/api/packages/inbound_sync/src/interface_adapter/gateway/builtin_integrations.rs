//! Built-in integrations registry.
//!
//! Provides static integration definitions for all supported providers.
//! These integrations are always available in the marketplace.

use std::collections::HashMap;
use std::sync::LazyLock;

use inbound_sync_domain::{
    ConnectionRepository, Integration, IntegrationCategory, IntegrationId,
    IntegrationRepository, OAuthConfig, OAuthProvider, SyncCapability,
};

/// Built-in integrations registry.
///
/// This is an in-memory implementation that provides static integration
/// definitions for all supported providers.
#[derive(Debug, Clone)]
pub struct BuiltinIntegrationRegistry {
    integrations: HashMap<IntegrationId, Integration>,
}

impl Default for BuiltinIntegrationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl BuiltinIntegrationRegistry {
    /// Create a new registry with all built-in integrations.
    pub fn new() -> Self {
        let integrations = BUILTIN_INTEGRATIONS
            .iter()
            .map(|i| (i.id().clone(), i.clone()))
            .collect();

        Self { integrations }
    }
}

#[async_trait::async_trait]
impl IntegrationRepository for BuiltinIntegrationRegistry {
    async fn find_all(&self) -> errors::Result<Vec<Integration>> {
        Ok(self.integrations.values().cloned().collect())
    }

    async fn find_enabled(&self) -> errors::Result<Vec<Integration>> {
        Ok(self
            .integrations
            .values()
            .filter(|i| i.is_enabled())
            .cloned()
            .collect())
    }

    async fn find_by_category(
        &self,
        category: IntegrationCategory,
    ) -> errors::Result<Vec<Integration>> {
        Ok(self
            .integrations
            .values()
            .filter(|i| i.category() == category)
            .cloned()
            .collect())
    }

    async fn find_featured(&self) -> errors::Result<Vec<Integration>> {
        Ok(self
            .integrations
            .values()
            .filter(|i| i.is_featured())
            .cloned()
            .collect())
    }

    async fn find_by_id(
        &self,
        id: &IntegrationId,
    ) -> errors::Result<Option<Integration>> {
        Ok(self.integrations.get(id).cloned())
    }

    async fn find_by_provider(
        &self,
        provider: OAuthProvider,
    ) -> errors::Result<Option<Integration>> {
        Ok(self
            .integrations
            .values()
            .find(|i| i.provider() == provider)
            .cloned())
    }
}

/// Static list of built-in integrations.
static BUILTIN_INTEGRATIONS: LazyLock<Vec<Integration>> = LazyLock::new(
    || {
        vec![
        // GitHub Integration
        Integration::new(
            IntegrationId::new("int_github"),
            OAuthProvider::Github,
            "GitHub",
            "Connect your GitHub repositories to sync issues, pull requests, \
             and code changes with Library.",
            IntegrationCategory::CodeManagement,
            SyncCapability::Inbound,
        )
        .with_icon("github")
        .with_objects(vec![
            "repository".to_string(),
            "issue".to_string(),
            "pull_request".to_string(),
            "commit".to_string(),
        ])
        .with_oauth(OAuthConfig {
            scopes: vec![
                "repo".to_string(),
                "read:org".to_string(),
                "read:user".to_string(),
            ],
            auth_url: "https://github.com/login/oauth/authorize".to_string(),
            token_url: "https://github.com/login/oauth/access_token".to_string(),
            supports_refresh: false,
        })
        .as_featured(),
        // Linear Integration
        Integration::new(
            IntegrationId::new("int_linear"),
            OAuthProvider::Linear,
            "Linear",
            "Sync issues, projects, and cycles from Linear to keep your \
             team's work organized in Library.",
            IntegrationCategory::ProjectManagement,
            SyncCapability::Inbound,
        )
        .with_icon("linear")
        .with_objects(vec![
            "issue".to_string(),
            "project".to_string(),
            "cycle".to_string(),
            "comment".to_string(),
        ])
        .with_oauth(OAuthConfig {
            scopes: vec!["read".to_string(), "write".to_string()],
            auth_url: "https://linear.app/oauth/authorize".to_string(),
            token_url: "https://api.linear.app/oauth/token".to_string(),
            supports_refresh: true,
        })
        .as_featured(),
        // HubSpot Integration
        Integration::new(
            IntegrationId::new("int_hubspot"),
            OAuthProvider::Hubspot,
            "HubSpot",
            "Connect HubSpot CRM to sync contacts, companies, deals, and \
             products with Library.",
            IntegrationCategory::Crm,
            SyncCapability::Bidirectional,
        )
        .with_icon("hubspot")
        .with_objects(vec![
            "contact".to_string(),
            "company".to_string(),
            "deal".to_string(),
            "product".to_string(),
        ])
        .with_oauth(OAuthConfig {
            scopes: vec![
                "crm.objects.contacts.read".to_string(),
                "crm.objects.companies.read".to_string(),
                "crm.objects.deals.read".to_string(),
            ],
            auth_url: "https://app.hubspot.com/oauth/authorize".to_string(),
            token_url: "https://api.hubapi.com/oauth/v1/token".to_string(),
            supports_refresh: true,
        })
        .as_featured(),
        // Stripe Integration
        Integration::new(
            IntegrationId::new("int_stripe"),
            OAuthProvider::Stripe,
            "Stripe",
            "Sync payments, customers, products, and subscriptions from \
             Stripe to manage billing data in Library.",
            IntegrationCategory::Payments,
            SyncCapability::Inbound,
        )
        .with_icon("stripe")
        .with_objects(vec![
            "customer".to_string(),
            "product".to_string(),
            "price".to_string(),
            "subscription".to_string(),
            "invoice".to_string(),
            "payment_intent".to_string(),
        ])
        .as_featured(),
        // Square Integration
        Integration::new(
            IntegrationId::new("int_square"),
            OAuthProvider::Square,
            "Square",
            "Connect Square to sync catalog items, customers, orders, and \
             inventory with Library.",
            IntegrationCategory::Ecommerce,
            SyncCapability::Bidirectional,
        )
        .with_icon("square")
        .with_objects(vec![
            "catalog_item".to_string(),
            "customer".to_string(),
            "order".to_string(),
            "inventory".to_string(),
            "payment".to_string(),
        ])
        .with_oauth(OAuthConfig {
            scopes: vec![
                "ITEMS_READ".to_string(),
                "CUSTOMERS_READ".to_string(),
                "ORDERS_READ".to_string(),
                "INVENTORY_READ".to_string(),
                "PAYMENTS_READ".to_string(),
            ],
            auth_url: "https://connect.squareup.com/oauth2/authorize".to_string(),
            token_url: "https://connect.squareup.com/oauth2/token".to_string(),
            supports_refresh: true,
        }),
        // Notion Integration
        Integration::new(
            IntegrationId::new("int_notion"),
            OAuthProvider::Notion,
            "Notion",
            "Sync pages and databases from Notion to centralize your \
             documentation in Library.",
            IntegrationCategory::ContentManagement,
            SyncCapability::Bidirectional,
        )
        .with_icon("notion")
        .with_objects(vec![
            "page".to_string(),
            "database".to_string(),
            "block".to_string(),
        ])
        .with_oauth(OAuthConfig {
            scopes: vec!["read_content".to_string(), "update_content".to_string()],
            auth_url: "https://api.notion.com/v1/oauth/authorize".to_string(),
            token_url: "https://api.notion.com/v1/oauth/token".to_string(),
            supports_refresh: false,
        }),
        // Airtable Integration
        Integration::new(
            IntegrationId::new("int_airtable"),
            OAuthProvider::Airtable,
            "Airtable",
            "Connect Airtable bases and tables to sync structured data \
             with Library.",
            IntegrationCategory::ContentManagement,
            SyncCapability::Bidirectional,
        )
        .with_icon("airtable")
        .with_objects(vec![
            "base".to_string(),
            "table".to_string(),
            "record".to_string(),
        ])
        .with_oauth(OAuthConfig {
            scopes: vec![
                "data.records:read".to_string(),
                "data.records:write".to_string(),
                "schema.bases:read".to_string(),
            ],
            auth_url: "https://airtable.com/oauth2/v1/authorize".to_string(),
            token_url: "https://airtable.com/oauth2/v1/token".to_string(),
            supports_refresh: true,
        }),
    ]
    },
);

/// In-memory connection repository for testing.
#[derive(Debug, Default)]
pub struct InMemoryConnectionRepository {
    connections: std::sync::RwLock<
        HashMap<
            inbound_sync_domain::ConnectionId,
            inbound_sync_domain::Connection,
        >,
    >,
}

#[async_trait::async_trait]
impl ConnectionRepository for InMemoryConnectionRepository {
    async fn save(
        &self,
        connection: &inbound_sync_domain::Connection,
    ) -> errors::Result<()> {
        let mut connections = self.connections.write().unwrap();
        connections.insert(connection.id().clone(), connection.clone());
        Ok(())
    }

    async fn find_by_id(
        &self,
        id: &inbound_sync_domain::ConnectionId,
    ) -> errors::Result<Option<inbound_sync_domain::Connection>> {
        let connections = self.connections.read().unwrap();
        Ok(connections.get(id).cloned())
    }

    async fn find_by_tenant(
        &self,
        tenant_id: &value_object::TenantId,
    ) -> errors::Result<Vec<inbound_sync_domain::Connection>> {
        let connections = self.connections.read().unwrap();
        Ok(connections
            .values()
            .filter(|c| c.tenant_id() == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_active_by_tenant(
        &self,
        tenant_id: &value_object::TenantId,
    ) -> errors::Result<Vec<inbound_sync_domain::Connection>> {
        let connections = self.connections.read().unwrap();
        Ok(connections
            .values()
            .filter(|c| c.tenant_id() == tenant_id && c.is_active())
            .cloned()
            .collect())
    }

    async fn find_by_tenant_and_integration(
        &self,
        tenant_id: &value_object::TenantId,
        integration_id: &IntegrationId,
    ) -> errors::Result<Option<inbound_sync_domain::Connection>> {
        let connections = self.connections.read().unwrap();
        Ok(connections
            .values()
            .find(|c| {
                c.tenant_id() == tenant_id
                    && c.integration_id() == integration_id
            })
            .cloned())
    }

    async fn find_by_tenant_and_provider(
        &self,
        tenant_id: &value_object::TenantId,
        provider: OAuthProvider,
    ) -> errors::Result<Option<inbound_sync_domain::Connection>> {
        let connections = self.connections.read().unwrap();
        Ok(connections
            .values()
            .find(|c| {
                c.tenant_id() == tenant_id && c.provider() == provider
            })
            .cloned())
    }

    async fn find_by_provider_and_external_account_id(
        &self,
        provider: OAuthProvider,
        external_account_id: &str,
    ) -> errors::Result<Option<inbound_sync_domain::Connection>> {
        let connections = self.connections.read().unwrap();
        Ok(connections
            .values()
            .find(|c| {
                c.provider() == provider
                    && c.external_account_id()
                        .map(|id| id == external_account_id)
                        .unwrap_or(false)
            })
            .cloned())
    }

    async fn delete(
        &self,
        id: &inbound_sync_domain::ConnectionId,
    ) -> errors::Result<()> {
        let mut connections = self.connections.write().unwrap();
        connections.remove(id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_builtin_integrations() {
        let registry = BuiltinIntegrationRegistry::new();

        let all = registry.find_all().await.unwrap();
        assert_eq!(all.len(), 7);

        let featured = registry.find_featured().await.unwrap();
        assert!(featured.len() >= 3);

        let github = registry
            .find_by_provider(OAuthProvider::Github)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(github.name(), "GitHub");
        assert!(github.requires_oauth());
    }

    #[tokio::test]
    async fn test_find_by_category() {
        let registry = BuiltinIntegrationRegistry::new();

        let payments = registry
            .find_by_category(IntegrationCategory::Payments)
            .await
            .unwrap();
        assert_eq!(payments.len(), 1);
        assert_eq!(payments[0].name(), "Stripe");
    }
}
