use std::sync::Arc;

use crate::domain::{Source, SourceRepository};
use tachyon_sdk::auth::AuthApp;

use super::{
    GetOrganizationByUsernameQuery, GetRepoByUsernameQuery,
    UpdateSourceInputData, UpdateSourceInputPort,
};

#[derive(Debug, Clone)]
pub struct UpdateSource {
    source_repository: Arc<dyn SourceRepository>,
    auth: Arc<dyn AuthApp>,
    get_org_by_username: Arc<dyn GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn GetRepoByUsernameQuery>,
}

impl UpdateSource {
    pub fn new(
        source_repository: Arc<dyn SourceRepository>,
        auth: Arc<dyn AuthApp>,
        get_org_by_username: Arc<dyn GetOrganizationByUsernameQuery>,
        get_repo_by_username: Arc<dyn GetRepoByUsernameQuery>,
    ) -> Self {
        Self {
            source_repository,
            auth,
            get_org_by_username,
            get_repo_by_username,
        }
    }
}

#[async_trait::async_trait]
impl UpdateSourceInputPort for UpdateSource {
    #[tracing::instrument(name = "UpdateSource::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: UpdateSourceInputData<'a>,
    ) -> errors::Result<Source> {
        // TODO: add English comment
        self.auth
            .check_policy(&tachyon_sdk::auth::CheckPolicyInput::<'a> {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:UpdateSource",
            })
            .await?;

        let org_username =
            input.org_username.parse::<value_object::Identifier>()?;
        let _org = self
            .get_org_by_username
            .execute(&org_username)
            .await?
            .ok_or(errors::Error::not_found("organization"))?;

        let repo_username =
            input.repo_username.parse::<value_object::Identifier>()?;
        let repo = self
            .get_repo_by_username
            .execute(&org_username, &repo_username)
            .await?
            .ok_or(errors::Error::not_found("repo"))?;

        super::ensure_scope_targets_org(
            input.multi_tenancy,
            repo.organization_id(),
        )?;

        // The policy above is checked for the named organization, so the
        // source must belong to the named repository. Otherwise a caller
        // with UpdateSource in their own organization could rewrite another
        // organization's source by id. Report it as missing, as get_source
        // does, so ids of other repositories are not confirmed.
        let source = self
            .source_repository
            .get_by_id(input.source_id)
            .await?
            .filter(|source| source.repo_id() == repo.id())
            .ok_or_else(|| errors::Error::not_found("source"))?;

        // TODO: add English comment
        let updated_source = source.update(input.name, input.url);

        // TODO: add English comment
        self.source_repository.save(&updated_source).await?;

        Ok(updated_source)
    }
}

// TODO: add English comment

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::sync::{Arc, Mutex};

    use database_manager::domain::DatabaseId;
    use mockall::mock;
    use tachyon_sdk::auth::{
        test_helper::{create_test_executor, create_test_multi_tenancy},
        MockAuthApp,
    };
    use value_object::{Identifier, TenantId, Text};

    use super::*;
    use crate::domain::{Organization, Repo, RepoId, SourceId};
    use crate::usecase::{
        DeleteSource, DeleteSourceInputData, DeleteSourceInputPort,
    };

    mock! {
        #[derive(Debug)]
        Sources {}
        #[async_trait::async_trait]
        impl SourceRepository for Sources {
            async fn save(&self, entity: &Source) -> errors::Result<()>;
            async fn get_by_id(&self, id: &SourceId) -> errors::Result<Option<Source>>;
            async fn find_by_repo_id(&self, repo_id: &RepoId) -> errors::Result<Vec<Source>>;
            async fn delete(&self, id: &SourceId) -> errors::Result<()>;
        }
    }

    mock! {
        #[derive(Debug)]
        Orgs {}
        #[async_trait::async_trait]
        impl GetOrganizationByUsernameQuery for Orgs {
            async fn execute(&self, username: &Identifier) -> errors::Result<Option<Organization>>;
        }
    }

    mock! {
        #[derive(Debug)]
        Repos {}
        #[async_trait::async_trait]
        impl GetRepoByUsernameQuery for Repos {
            async fn execute(&self, org_username: &Identifier, repo_username: &Identifier) -> errors::Result<Option<Repo>>;
        }
    }

    const ORG: &str = "attacker-org";
    const REPO: &str = "attacker-repo";

    fn allow_all() -> MockAuthApp {
        let mut auth = MockAuthApp::new();
        auth.expect_check_policy()
            .returning(|_| Box::pin(async { Ok(()) }));
        auth
    }

    fn orgs() -> MockOrgs {
        let mut orgs = MockOrgs::new();
        orgs.expect_execute().returning(|_| {
            Ok(Some(Organization::new(
                &TenantId::new(
                    tachyon_sdk::auth::test_helper::TEST_TENANT_ID,
                )
                .unwrap(),
                &Text::new("Attacker").unwrap(),
                &Identifier::from_str(ORG).unwrap(),
                None,
                None,
            )))
        });
        orgs
    }

    fn repos(repo_id: RepoId) -> MockRepos {
        let mut repos = MockRepos::new();
        repos.expect_execute().returning(move |_, _| {
            Ok(Some(Repo::new(
                &repo_id,
                &TenantId::new(
                    tachyon_sdk::auth::test_helper::TEST_TENANT_ID,
                )
                .unwrap(),
                &Identifier::from_str(ORG).unwrap(),
                &Text::new("Attacker repo").unwrap(),
                &Identifier::from_str(REPO).unwrap(),
                true,
                None,
                vec![DatabaseId::default()],
                vec![],
            )))
        });
        repos
    }

    /// Source repository holding one source owned by `owner`, recording
    /// saves and deletes.
    fn sources(
        source: Source,
        writes: Arc<Mutex<Vec<&'static str>>>,
    ) -> MockSources {
        let mut sources = MockSources::new();
        sources
            .expect_get_by_id()
            .returning(move |_| Ok(Some(source.clone())));
        let saves = writes.clone();
        sources.expect_save().returning(move |_| {
            saves.lock().unwrap().push("save");
            Ok(())
        });
        sources.expect_delete().returning(move |_| {
            writes.lock().unwrap().push("delete");
            Ok(())
        });
        sources
    }

    fn source_in(repo_id: &RepoId) -> Source {
        Source::new(
            &SourceId::default(),
            repo_id,
            &Text::new("victim source").unwrap(),
            None,
        )
    }

    async fn update(
        named_repo: RepoId,
        source: Source,
    ) -> (errors::Result<Source>, Vec<&'static str>) {
        let writes = Arc::new(Mutex::new(Vec::new()));
        let usecase = UpdateSource::new(
            Arc::new(sources(source.clone(), writes.clone())),
            Arc::new(allow_all()),
            Arc::new(orgs()),
            Arc::new(repos(named_repo)),
        );
        let executor = create_test_executor();
        let multi_tenancy = create_test_multi_tenancy();
        let result = usecase
            .execute(UpdateSourceInputData {
                executor: &executor,
                multi_tenancy: &multi_tenancy,
                source_id: source.id(),
                org_username: ORG.into(),
                repo_username: REPO.into(),
                name: Some(Text::new("renamed").unwrap()),
                url: None,
            })
            .await;
        let writes = writes.lock().unwrap().clone();
        (result, writes)
    }

    async fn delete(
        named_repo: RepoId,
        source: Source,
    ) -> (errors::Result<()>, Vec<&'static str>) {
        let writes = Arc::new(Mutex::new(Vec::new()));
        let usecase = DeleteSource::new(
            Arc::new(sources(source.clone(), writes.clone())),
            Arc::new(allow_all()),
            Arc::new(orgs()),
            Arc::new(repos(named_repo)),
        );
        let executor = create_test_executor();
        let multi_tenancy = create_test_multi_tenancy();
        let result = usecase
            .execute(DeleteSourceInputData {
                executor: &executor,
                multi_tenancy: &multi_tenancy,
                source_id: source.id(),
                org_username: ORG.into(),
                repo_username: REPO.into(),
            })
            .await;
        let writes = writes.lock().unwrap().clone();
        (result, writes)
    }

    #[tokio::test]
    async fn update_rejects_a_source_of_another_repository() {
        let (result, writes) =
            update(RepoId::default(), source_in(&RepoId::default())).await;
        let err = result.unwrap_err();
        assert!(matches!(err, errors::Error::NotFound { .. }), "{err:?}");
        assert!(writes.is_empty(), "{writes:?}");
    }

    #[tokio::test]
    async fn update_saves_a_source_of_the_named_repository() {
        let repo_id = RepoId::default();
        let (result, writes) =
            update(repo_id.clone(), source_in(&repo_id)).await;
        assert_eq!(result.unwrap().name().to_string(), "renamed");
        assert_eq!(writes, vec!["save"]);
    }

    #[tokio::test]
    async fn delete_rejects_a_source_of_another_repository() {
        let (result, writes) =
            delete(RepoId::default(), source_in(&RepoId::default())).await;
        let err = result.unwrap_err();
        assert!(matches!(err, errors::Error::NotFound { .. }), "{err:?}");
        assert!(writes.is_empty(), "{writes:?}");
    }

    #[tokio::test]
    async fn delete_removes_a_source_of_the_named_repository() {
        let repo_id = RepoId::default();
        let (result, writes) =
            delete(repo_id.clone(), source_in(&repo_id)).await;
        result.unwrap();
        assert_eq!(writes, vec!["delete"]);
    }
}
