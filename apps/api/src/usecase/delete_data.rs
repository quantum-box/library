use tachyon_sdk::auth::{AuthApp, CheckPolicyInput};

use crate::usecase::{DeleteDataInputData, DeleteDataInputPort};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct DeleteData {
    get_org_by_username:
        Arc<dyn crate::usecase::GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn crate::usecase::GetRepoByUsernameQuery>,
    auth_app: Arc<dyn AuthApp>,
    database: Arc<database_manager::App>,
}

impl DeleteData {
    pub fn new(
        get_org_by_username: Arc<
            dyn crate::usecase::GetOrganizationByUsernameQuery,
        >,
        get_repo_by_username: Arc<
            dyn crate::usecase::GetRepoByUsernameQuery,
        >,
        auth_app: Arc<dyn AuthApp>,
        database: Arc<database_manager::App>,
    ) -> Arc<Self> {
        Arc::new(Self {
            get_org_by_username,
            get_repo_by_username,
            auth_app,
            database,
        })
    }
}

#[async_trait::async_trait]
impl DeleteDataInputPort for DeleteData {
    #[tracing::instrument(name = "DeleteData::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: DeleteDataInputData<'a>,
    ) -> errors::Result<()> {
        self.auth_app
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:DeleteData",
            })
            .await?;

        let org = self
            .get_org_by_username
            .execute(&input.org_username.parse()?)
            .await?
            .ok_or(errors::not_found!(
                "organization not found in delete data"
            ))?;
        let repo = self
            .get_repo_by_username
            .execute(org.username(), &input.repo_username.parse()?)
            .await?
            .ok_or(errors::not_found!("repo not found in delete data"))?;

        self.database
            .delete_data_usecase()
            .execute(&database_manager::usecase::DeleteDataInputData {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                tenant_id: org.id(),
                database_id: repo.databases().first().unwrap().as_ref(),
                data_id: &input.data_id,
            })
            .await?;

        Ok(())
    }
}
