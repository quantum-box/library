use crate::usecase::{AddPropertyInputData, AddPropertyInputPort};
use database_manager::domain::Property;
use std::sync::Arc;
use tachyon_sdk::auth::{AuthApp, CheckPolicyInput};

#[derive(Debug, Clone)]
pub struct AddProperty {
    get_org_by_username:
        Arc<dyn crate::usecase::GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn crate::usecase::GetRepoByUsernameQuery>,
    auth: Arc<dyn AuthApp>,
    database: Arc<database_manager::App>,
}

impl AddProperty {
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
impl AddPropertyInputPort for AddProperty {
    #[tracing::instrument(name = "AddProperty::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: AddPropertyInputData<'a>,
    ) -> errors::Result<Property> {
        self.auth
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "database:AddProperty",
            })
            .await?;

        let org = self
            .get_org_by_username
            .execute(&input.org_username.parse()?)
            .await
            .map_err(|e| {
                errors::Error::application_logic_error(e.to_string())
            })?
            .ok_or(errors::Error::not_found("organization"))?;
        let repo = self
            .get_repo_by_username
            .execute(org.username(), &input.repo_username.parse()?)
            .await
            .map_err(|e| {
                errors::Error::application_logic_error(e.to_string())
            })?
            .ok_or(errors::Error::not_found("repo"))?;

        // TODO: add English comment
        // let user = self
        //     .auth
        //     .authentication()
        //     .get_user(input.operator_id, &input.actor.get_id().parse()?)
        //     .await
        //     .map_err(|e| {
        //         errors::Error::application_logic_error(e.to_string())
        //     })?
        //     .ok_or(errors::Error::not_found("user"))?;
        // repo.can_write(user.id())?;

        let property = self
            .database
            .add_property()
            .execute(database_manager::usecase::AddPropertyInputData {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                tenant_id: org.id(),
                database_id: &repo.databases().first().unwrap().clone(),
                name: &input.property_name,
                property_type: input.property_type,
            })
            .await?;
        Ok(property)
    }
}
