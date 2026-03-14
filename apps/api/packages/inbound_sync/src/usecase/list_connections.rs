//! List tenant connections use case.

use std::sync::Arc;

use derive_new::new;
use inbound_sync_domain::{Connection, ConnectionRepository};
use tachyon_sdk::auth::{
    AuthApp, CheckPolicyInput, ExecutorAction, MultiTenancyAction,
};

/// Input data for listing connections.
#[derive(Debug)]
pub struct ListConnectionsInputData<'a> {
    /// Executor (user or service account)
    pub executor: &'a dyn ExecutorAction,
    /// Multi-tenancy context
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    /// Only return active connections
    pub active_only: bool,
}

/// Output data for list connections.
#[derive(Debug)]
pub struct ListConnectionsOutputData {
    /// List of connections for the tenant
    pub connections: Vec<Connection>,
}

/// Input port for list connections use case.
#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait]
pub trait ListConnectionsInputPort: std::fmt::Debug + Send + Sync {
    /// Returns the policy action name for this use case.
    fn policy(&self) -> &'static str {
        "inbound_sync:ListConnections"
    }

    /// Execute the use case.
    async fn execute<'a>(
        &self,
        input: ListConnectionsInputData<'a>,
    ) -> errors::Result<ListConnectionsOutputData>;
}

/// List all connections for a tenant.
///
/// This use case retrieves all integration connections for a specific tenant.
/// Connections represent active or past integrations with external services
/// like GitHub, Linear, HubSpot, etc.
#[derive(Debug, Clone, new)]
pub struct ListConnections {
    auth: Arc<dyn AuthApp>,
    repository: Arc<dyn ConnectionRepository>,
}

#[async_trait::async_trait]
impl ListConnectionsInputPort for ListConnections {
    #[tracing::instrument(
        name = "inbound_sync::ListConnections::execute",
        skip(self, input),
        fields(active_only = input.active_only)
    )]
    async fn execute<'a>(
        &self,
        input: ListConnectionsInputData<'a>,
    ) -> errors::Result<ListConnectionsOutputData> {
        // 1. Policy check
        self.auth
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: self.policy(),
            })
            .await?;

        // 2. Get tenant ID from multi-tenancy context
        let tenant_id = input.multi_tenancy.get_operator_id()?;

        // 3. Fetch connections based on filter
        let connections = if input.active_only {
            self.repository.find_active_by_tenant(&tenant_id).await?
        } else {
            self.repository.find_by_tenant(&tenant_id).await?
        };

        tracing::debug!(
            tenant_id = %tenant_id,
            count = connections.len(),
            active_only = input.active_only,
            "Retrieved connections for tenant"
        );

        Ok(ListConnectionsOutputData { connections })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use inbound_sync_domain::{
        ConnectionId, ConnectionStatus, IntegrationId, OAuthProvider,
    };
    use tachyon_sdk::auth::MockAuthApp;
    use value_object::TenantId;

    #[derive(Debug)]
    struct MockConnectionRepository {
        connections: Vec<Connection>,
    }

    impl MockConnectionRepository {
        fn new() -> Self {
            let tenant1 =
                TenantId::new("tn_01hjryxysgey07h5jz5wagqj0m").unwrap();
            let now = Utc::now();

            let connections = vec![
                Connection::new(
                    ConnectionId::new("conn_1"),
                    tenant1.clone(),
                    IntegrationId::new("int_github"),
                    OAuthProvider::Github,
                    ConnectionStatus::Active,
                    None,
                    None,
                    None,
                    None,
                    None,
                    now,
                    now,
                    std::collections::HashMap::new(),
                ),
                Connection::new(
                    ConnectionId::new("conn_2"),
                    tenant1,
                    IntegrationId::new("int_linear"),
                    OAuthProvider::Linear,
                    ConnectionStatus::Active,
                    None,
                    None,
                    None,
                    None,
                    None,
                    now,
                    now,
                    std::collections::HashMap::new(),
                )
                .with_external_account(
                    "acct_123",
                    Some("My Account".to_string()),
                ),
            ];

            Self { connections }
        }
    }

    #[async_trait::async_trait]
    impl ConnectionRepository for MockConnectionRepository {
        async fn save(
            &self,
            _connection: &Connection,
        ) -> errors::Result<()> {
            Ok(())
        }

        async fn find_by_id(
            &self,
            id: &ConnectionId,
        ) -> errors::Result<Option<Connection>> {
            Ok(self.connections.iter().find(|c| c.id() == id).cloned())
        }

        async fn find_by_tenant(
            &self,
            tenant_id: &TenantId,
        ) -> errors::Result<Vec<Connection>> {
            Ok(self
                .connections
                .iter()
                .filter(|c| c.tenant_id() == tenant_id)
                .cloned()
                .collect())
        }

        async fn find_active_by_tenant(
            &self,
            tenant_id: &TenantId,
        ) -> errors::Result<Vec<Connection>> {
            Ok(self
                .connections
                .iter()
                .filter(|c| {
                    c.tenant_id() == tenant_id
                        && c.status() == ConnectionStatus::Active
                })
                .cloned()
                .collect())
        }

        async fn find_by_tenant_and_integration(
            &self,
            tenant_id: &TenantId,
            integration_id: &IntegrationId,
        ) -> errors::Result<Option<Connection>> {
            Ok(self
                .connections
                .iter()
                .find(|c| {
                    c.tenant_id() == tenant_id
                        && c.integration_id() == integration_id
                })
                .cloned())
        }

        async fn find_by_tenant_and_provider(
            &self,
            tenant_id: &TenantId,
            provider: OAuthProvider,
        ) -> errors::Result<Option<Connection>> {
            Ok(self
                .connections
                .iter()
                .find(|c| {
                    c.tenant_id() == tenant_id && c.provider() == provider
                })
                .cloned())
        }

        async fn find_by_provider_and_external_account_id(
            &self,
            provider: OAuthProvider,
            external_account_id: &str,
        ) -> errors::Result<Option<Connection>> {
            Ok(self
                .connections
                .iter()
                .find(|c| {
                    c.provider() == provider
                        && c.external_account_id()
                            .map(|id| id == external_account_id)
                            .unwrap_or(false)
                })
                .cloned())
        }

        async fn delete(&self, _id: &ConnectionId) -> errors::Result<()> {
            Ok(())
        }
    }

    fn create_mock_auth() -> Arc<MockAuthApp> {
        let mut mock = MockAuthApp::new();
        mock.expect_check_policy()
            .returning(|_| Box::pin(async { Ok(()) }));
        Arc::new(mock)
    }

    mod list_all {
        use super::*;
        use tachyon_sdk::auth::test_helper::{
            create_test_executor, create_test_multi_tenancy,
        };

        #[tokio::test]
        async fn test_list_all_connections_for_tenant() {
            let auth = create_mock_auth();
            let repository = Arc::new(MockConnectionRepository::new());
            let usecase = ListConnections::new(auth, repository);

            let executor = create_test_executor();
            let multi_tenancy = create_test_multi_tenancy();

            let result = usecase
                .execute(ListConnectionsInputData {
                    executor: &executor,
                    multi_tenancy: &multi_tenancy,
                    active_only: false,
                })
                .await
                .unwrap();

            assert_eq!(result.connections.len(), 2);
        }
    }

    mod list_active {
        use super::*;
        use tachyon_sdk::auth::test_helper::{
            create_test_executor, create_test_multi_tenancy,
        };

        #[tokio::test]
        async fn test_list_active_connections_only() {
            let auth = create_mock_auth();
            let repository = Arc::new(MockConnectionRepository::new());
            let usecase = ListConnections::new(auth, repository);

            let executor = create_test_executor();
            let multi_tenancy = create_test_multi_tenancy();

            let result = usecase
                .execute(ListConnectionsInputData {
                    executor: &executor,
                    multi_tenancy: &multi_tenancy,
                    active_only: true,
                })
                .await
                .unwrap();

            assert!(result
                .connections
                .iter()
                .all(|c| c.status() == ConnectionStatus::Active));
        }
    }
}
