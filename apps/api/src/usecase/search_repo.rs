#![allow(dead_code)]

use super::{
    GetOrganizationByUsernameQuery, SearchRepoInputData,
    SearchRepoInputPort,
};
use crate::domain::Repo;
use std::{fmt::Debug, sync::Arc};
use value_object::TenantId;

#[derive(Debug, Clone)]
pub struct AllRepoQuerySearchInOrganizationQueryData {
    pub organization_id: String,
    pub name: Option<String>,
    pub limit: Option<i64>,
}

#[async_trait::async_trait]
pub trait AllRepoQuery: Debug + Send + Sync {
    async fn search_in_organization(
        &self,
        query: &AllRepoQuerySearchInOrganizationQueryData,
    ) -> errors::Result<Vec<Repo>>;
}

#[derive(Debug, Clone)]
pub struct SearchRepo {
    all_repo_query: Arc<dyn AllRepoQuery>,
    get_organization_by_username: Arc<dyn GetOrganizationByUsernameQuery>,
}

impl SearchRepo {
    pub fn new(
        all_repo_query: Arc<dyn AllRepoQuery>,
        get_organization_by_username: Arc<
            dyn GetOrganizationByUsernameQuery,
        >,
    ) -> Arc<Self> {
        Arc::new(Self {
            all_repo_query,
            get_organization_by_username,
        })
    }

    /// Resolve the single organization a search may look inside, or `None`
    /// when the caller has no claim to one.
    ///
    /// Repository search must never double as a directory of the whole
    /// tenant, so anonymous callers and callers asking about an organization
    /// they do not belong to both resolve to `None`.
    async fn searchable_organization(
        &self,
        input: &SearchRepoInputData<'_>,
    ) -> errors::Result<Option<TenantId>> {
        if input.executor.is_none() {
            return Ok(None);
        }

        let organization_id = match &input.org_username {
            Some(org_username) => self
                .get_organization_by_username
                .execute(&org_username.parse()?)
                .await?
                .map(|org| org.id().clone()),
            None => input.multi_tenancy.operator_id(),
        };

        Ok(organization_id.filter(|id| input.executor.has_tenant_id(id)))
    }
}

#[async_trait::async_trait]
impl SearchRepoInputPort for SearchRepo {
    /// TODO: add English documentation
    #[tracing::instrument(name = "SearchRepo::execute", skip_all)]
    async fn execute<'a>(
        &self,
        input: &SearchRepoInputData<'a>,
    ) -> errors::Result<Vec<Repo>> {
        let Some(organization_id) =
            self.searchable_organization(input).await?
        else {
            return Ok(vec![]);
        };

        self.all_repo_query
            .search_in_organization(
                &AllRepoQuerySearchInOrganizationQueryData {
                    organization_id: organization_id.to_string(),
                    name: input.name.clone(),
                    limit: input.limit,
                },
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Organization;
    use std::sync::Mutex;
    use tachyon_sdk::auth::{ExecutorAction, MultiTenancy};
    use value_object::Identifier;

    #[derive(Debug, Default)]
    struct RecordingAllRepoQuery {
        calls: Mutex<Vec<String>>,
    }

    #[async_trait::async_trait]
    impl AllRepoQuery for RecordingAllRepoQuery {
        async fn search_in_organization(
            &self,
            query: &AllRepoQuerySearchInOrganizationQueryData,
        ) -> errors::Result<Vec<Repo>> {
            self.calls.lock().unwrap().push(format!(
                "{}:{:?}:{:?}",
                query.organization_id, query.name, query.limit
            ));
            Ok(vec![])
        }
    }

    #[derive(Debug)]
    struct StubOrganizationQuery {
        organization: Option<Organization>,
    }

    #[async_trait::async_trait]
    impl GetOrganizationByUsernameQuery for StubOrganizationQuery {
        async fn execute(
            &self,
            _username: &Identifier,
        ) -> errors::Result<Option<Organization>> {
            Ok(self.organization.clone())
        }
    }

    /// Executor that belongs to exactly the tenants it is built with.
    #[derive(Debug)]
    struct TestExecutor {
        tenants: Vec<TenantId>,
    }

    impl ExecutorAction for TestExecutor {
        fn get_id(&self) -> &str {
            "01hjjn348rn3t49zz6hvmfq67p"
        }

        fn has_tenant_id(&self, tenant_id: &TenantId) -> bool {
            self.tenants.contains(tenant_id)
        }

        fn is_system_user(&self) -> bool {
            false
        }

        fn is_user(&self) -> bool {
            !self.tenants.is_empty()
        }

        fn is_service_account(&self) -> bool {
            false
        }

        fn is_none(&self) -> bool {
            self.tenants.is_empty()
        }
    }

    fn organization(id: &TenantId, username: &str) -> Organization {
        Organization::new(
            id,
            &"Acme".parse().unwrap(),
            &username.parse().unwrap(),
            None,
            None,
        )
    }

    fn member_of(tenant_id: &TenantId) -> TestExecutor {
        TestExecutor {
            tenants: vec![tenant_id.clone()],
        }
    }

    fn anonymous() -> TestExecutor {
        TestExecutor { tenants: vec![] }
    }

    fn usecase(
        query: Arc<RecordingAllRepoQuery>,
        organization: Option<Organization>,
    ) -> Arc<SearchRepo> {
        SearchRepo::new(
            query,
            Arc::new(StubOrganizationQuery { organization }),
        )
    }

    #[tokio::test]
    async fn execute_searches_the_callers_own_organization() {
        let tenant_id = TenantId::default();
        let query = Arc::new(RecordingAllRepoQuery::default());
        let usecase = usecase(query.clone(), None);
        let executor = member_of(&tenant_id);
        let multi_tenancy =
            MultiTenancy::new(None, Some(tenant_id.clone()));

        let repos = usecase
            .execute(&SearchRepoInputData {
                executor: &executor,
                multi_tenancy: &multi_tenancy,
                org_username: None,
                name: Some("docs".to_string()),
                limit: Some(7),
            })
            .await
            .unwrap();

        assert!(repos.is_empty());
        assert_eq!(
            query.calls.lock().unwrap().as_slice(),
            &[format!("{tenant_id}:Some(\"docs\"):Some(7)")]
        );
    }

    #[tokio::test]
    async fn execute_searches_a_named_organization_the_caller_belongs_to() {
        let tenant_id = TenantId::default();
        let query = Arc::new(RecordingAllRepoQuery::default());
        let usecase =
            usecase(query.clone(), Some(organization(&tenant_id, "acme")));
        let executor = member_of(&tenant_id);
        let multi_tenancy = MultiTenancy::new(None, None);

        usecase
            .execute(&SearchRepoInputData {
                executor: &executor,
                multi_tenancy: &multi_tenancy,
                org_username: Some("acme".to_string()),
                name: None,
                limit: None,
            })
            .await
            .unwrap();

        assert_eq!(
            query.calls.lock().unwrap().as_slice(),
            &[format!("{tenant_id}:None:None")]
        );
    }

    #[tokio::test]
    async fn execute_lists_nothing_for_anonymous_callers() {
        let tenant_id = TenantId::default();
        let query = Arc::new(RecordingAllRepoQuery::default());
        let usecase =
            usecase(query.clone(), Some(organization(&tenant_id, "acme")));
        let executor = anonymous();
        let multi_tenancy =
            MultiTenancy::new(None, Some(tenant_id.clone()));

        let repos = usecase
            .execute(&SearchRepoInputData {
                executor: &executor,
                multi_tenancy: &multi_tenancy,
                org_username: None,
                name: None,
                limit: None,
            })
            .await
            .unwrap();

        assert!(repos.is_empty());
        assert!(query.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn execute_lists_nothing_for_an_organization_the_caller_is_not_in(
    ) {
        let tenant_id = TenantId::default();
        let other_tenant_id = TenantId::default();
        let query = Arc::new(RecordingAllRepoQuery::default());
        let usecase = usecase(
            query.clone(),
            Some(organization(&other_tenant_id, "someone-else")),
        );
        let executor = member_of(&tenant_id);
        let multi_tenancy = MultiTenancy::new(None, None);

        let repos = usecase
            .execute(&SearchRepoInputData {
                executor: &executor,
                multi_tenancy: &multi_tenancy,
                org_username: Some("someone-else".to_string()),
                name: None,
                limit: None,
            })
            .await
            .unwrap();

        assert!(repos.is_empty());
        assert!(query.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn execute_lists_nothing_without_an_organization_to_scope_to() {
        let tenant_id = TenantId::default();
        let query = Arc::new(RecordingAllRepoQuery::default());
        let usecase = usecase(query.clone(), None);
        let executor = member_of(&tenant_id);
        let multi_tenancy = MultiTenancy::new(None, None);

        let repos = usecase
            .execute(&SearchRepoInputData {
                executor: &executor,
                multi_tenancy: &multi_tenancy,
                org_username: None,
                name: None,
                limit: None,
            })
            .await
            .unwrap();

        assert!(repos.is_empty());
        assert!(query.calls.lock().unwrap().is_empty());
    }
}
