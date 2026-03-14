use crate::usecase::{DeletePropertyInputData, DeletePropertyInputPort};
use database_manager::domain;
use std::sync::Arc;
use tachyon_sdk::auth::{AuthApp, CheckPolicyInput};

#[derive(Clone, Debug)]
pub struct DeleteProperty {
    get_org_by_username:
        Arc<dyn crate::usecase::GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn crate::usecase::GetRepoByUsernameQuery>,
    auth: Arc<dyn AuthApp>,
    database: Arc<database_manager::App>,
}

impl DeleteProperty {
    pub fn new(
        get_org_by_username: Arc<
            dyn crate::usecase::GetOrganizationByUsernameQuery,
        >,
        get_repo_by_username: Arc<
            dyn crate::usecase::GetRepoByUsernameQuery,
        >,
        auth: Arc<dyn AuthApp>,
        database: Arc<database_manager::App>,
    ) -> Arc<Self> {
        Arc::new(Self {
            get_org_by_username,
            get_repo_by_username,
            auth,
            database,
        })
    }
}

#[async_trait::async_trait]
impl DeletePropertyInputPort for DeleteProperty {
    #[tracing::instrument(name = "DeleteProperty::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: DeletePropertyInputData<'a>,
    ) -> errors::Result<domain::Property> {
        self.auth
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "database:DeleteProperty",
            })
            .await?;

        let org = self
            .get_org_by_username
            .execute(&input.org_username.parse()?)
            .await?
            .ok_or(errors::not_found!(
                "organization not found in delete property"
            ))?;
        let repo = self
            .get_repo_by_username
            .execute(org.username(), &input.repo_username.parse()?)
            .await?
            .ok_or(errors::not_found!(
                "repo not found in delete property"
            ))?;
        // TODO: add English comment
        // let user = self
        //     .auth
        //     .authentication()
        //     .get_user(input.tenant_id, &input.executor.get_id().parse()?)
        //     .await
        //     .map_err(|e| {
        //         errors::Error::application_logic_error(e.to_string())
        //     })?
        //     .ok_or(errors::Error::not_found("user"))?;

        // repo.can_delete(user.id())?;

        let property = self
            .database
            .delete_property_usecase()
            .execute(&database_manager::DeletePropertyInputData {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                tenant_id: org.id(),
                database_id: &repo.databases().first().unwrap().clone(),
                property_id: &input.property_id,
            })
            .await?;

        Ok(property)
    }
}
