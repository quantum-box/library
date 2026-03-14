//! List all available integrations use case.

use std::sync::Arc;

use derive_new::new;
use inbound_sync_domain::{
    Integration, IntegrationCategory, IntegrationRepository,
};
use tachyon_sdk::auth::{
    AuthApp, CheckPolicyInput, ExecutorAction, MultiTenancyAction,
};

/// Filter options for listing integrations.
#[derive(Debug, Clone, Default)]
pub enum IntegrationFilter {
    /// Return all integrations
    #[default]
    All,
    /// Return only enabled integrations
    Enabled,
    /// Return only featured integrations
    Featured,
    /// Return integrations by category
    ByCategory(IntegrationCategory),
}

/// Input data for list integrations use case.
#[derive(Debug)]
pub struct ListIntegrationsInputData<'a> {
    /// Executor (user or service account)
    pub executor: &'a dyn ExecutorAction,
    /// Multi-tenancy context
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    /// Filter to apply
    pub filter: IntegrationFilter,
}

/// Output data for list integrations.
#[derive(Debug)]
pub struct ListIntegrationsOutputData {
    /// List of integrations matching the filter
    pub integrations: Vec<Integration>,
}

/// Input port for list integrations use case.
#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait]
pub trait ListIntegrationsInputPort: std::fmt::Debug + Send + Sync {
    /// Returns the policy action name for this use case.
    fn policy(&self) -> &'static str {
        "inbound_sync:ListIntegrations"
    }

    /// Execute the use case.
    async fn execute<'a>(
        &self,
        input: ListIntegrationsInputData<'a>,
    ) -> errors::Result<ListIntegrationsOutputData>;
}

/// List all available integrations in the marketplace.
///
/// This use case retrieves all built-in integrations that are available
/// for users to connect. Integrations include GitHub, Linear, HubSpot,
/// Stripe, Notion, and Square.
#[derive(Debug, Clone, new)]
pub struct ListIntegrations {
    auth: Arc<dyn AuthApp>,
    repository: Arc<dyn IntegrationRepository>,
}

#[async_trait::async_trait]
impl ListIntegrationsInputPort for ListIntegrations {
    #[tracing::instrument(
        name = "inbound_sync::ListIntegrations::execute",
        skip(self, input),
        fields(filter = ?input.filter)
    )]
    async fn execute<'a>(
        &self,
        input: ListIntegrationsInputData<'a>,
    ) -> errors::Result<ListIntegrationsOutputData> {
        // 1. Policy check
        self.auth
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: self.policy(),
            })
            .await?;

        // 2. Fetch integrations based on filter
        let integrations = match input.filter {
            IntegrationFilter::All => self.repository.find_all().await?,
            IntegrationFilter::Enabled => {
                self.repository.find_enabled().await?
            }
            IntegrationFilter::Featured => {
                self.repository.find_featured().await?
            }
            IntegrationFilter::ByCategory(category) => {
                self.repository.find_by_category(category).await?
            }
        };

        tracing::debug!(
            count = integrations.len(),
            filter = ?input.filter,
            "Retrieved integrations from marketplace"
        );

        Ok(ListIntegrationsOutputData { integrations })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inbound_sync_domain::{
        IntegrationId, OAuthProvider, SyncCapability,
    };
    use std::collections::HashMap;
    use tachyon_sdk::auth::MockAuthApp;

    #[derive(Debug)]
    struct MockIntegrationRepository {
        integrations: HashMap<IntegrationId, Integration>,
    }

    impl MockIntegrationRepository {
        fn new() -> Self {
            let github = Integration::new(
                IntegrationId::new("int_github"),
                OAuthProvider::Github,
                "GitHub",
                "GitHub integration",
                IntegrationCategory::CodeManagement,
                SyncCapability::Inbound,
            )
            .with_icon("github")
            .as_featured();

            let linear = Integration::new(
                IntegrationId::new("int_linear"),
                OAuthProvider::Linear,
                "Linear",
                "Linear integration",
                IntegrationCategory::ProjectManagement,
                SyncCapability::Inbound,
            )
            .with_icon("linear");

            let integrations = vec![github, linear]
                .into_iter()
                .map(|i| (i.id().clone(), i))
                .collect();

            Self { integrations }
        }
    }

    #[async_trait::async_trait]
    impl IntegrationRepository for MockIntegrationRepository {
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
        async fn test_list_all_integrations() {
            let auth = create_mock_auth();
            let repository = Arc::new(MockIntegrationRepository::new());
            let usecase = ListIntegrations::new(auth, repository);

            let executor = create_test_executor();
            let multi_tenancy = create_test_multi_tenancy();

            let result = usecase
                .execute(ListIntegrationsInputData {
                    executor: &executor,
                    multi_tenancy: &multi_tenancy,
                    filter: IntegrationFilter::All,
                })
                .await
                .unwrap();

            assert_eq!(result.integrations.len(), 2);
        }
    }

    mod list_featured {
        use super::*;
        use tachyon_sdk::auth::test_helper::{
            create_test_executor, create_test_multi_tenancy,
        };

        #[tokio::test]
        async fn test_list_featured_integrations() {
            let auth = create_mock_auth();
            let repository = Arc::new(MockIntegrationRepository::new());
            let usecase = ListIntegrations::new(auth, repository);

            let executor = create_test_executor();
            let multi_tenancy = create_test_multi_tenancy();

            let result = usecase
                .execute(ListIntegrationsInputData {
                    executor: &executor,
                    multi_tenancy: &multi_tenancy,
                    filter: IntegrationFilter::Featured,
                })
                .await
                .unwrap();

            assert_eq!(result.integrations.len(), 1);
            assert_eq!(
                result.integrations[0].provider(),
                OAuthProvider::Github
            );
        }
    }

    mod list_enabled {
        use super::*;
        use tachyon_sdk::auth::test_helper::{
            create_test_executor, create_test_multi_tenancy,
        };

        #[tokio::test]
        async fn test_list_enabled_integrations() {
            let auth = create_mock_auth();
            let repository = Arc::new(MockIntegrationRepository::new());
            let usecase = ListIntegrations::new(auth, repository);

            let executor = create_test_executor();
            let multi_tenancy = create_test_multi_tenancy();

            let result = usecase
                .execute(ListIntegrationsInputData {
                    executor: &executor,
                    multi_tenancy: &multi_tenancy,
                    filter: IntegrationFilter::Enabled,
                })
                .await
                .unwrap();

            assert_eq!(result.integrations.len(), 2);
        }
    }

    mod list_by_category {
        use super::*;
        use tachyon_sdk::auth::test_helper::{
            create_test_executor, create_test_multi_tenancy,
        };

        #[tokio::test]
        async fn test_list_by_category() {
            let auth = create_mock_auth();
            let repository = Arc::new(MockIntegrationRepository::new());
            let usecase = ListIntegrations::new(auth, repository);

            let executor = create_test_executor();
            let multi_tenancy = create_test_multi_tenancy();

            let result = usecase
                .execute(ListIntegrationsInputData {
                    executor: &executor,
                    multi_tenancy: &multi_tenancy,
                    filter: IntegrationFilter::ByCategory(
                        IntegrationCategory::CodeManagement,
                    ),
                })
                .await
                .unwrap();

            assert_eq!(result.integrations.len(), 1);
            assert_eq!(
                result.integrations[0].provider(),
                OAuthProvider::Github
            );
        }
    }
}
