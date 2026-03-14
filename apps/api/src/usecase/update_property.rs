use crate::usecase::{UpdatePropertyInputData, UpdatePropertyInputPort};
use database_manager::domain::Property;
use std::sync::Arc;
use tachyon_sdk::auth::{AuthApp, CheckPolicyInput};

#[derive(Debug, Clone)]
pub struct UpdateProperty {
    get_org_by_username:
        Arc<dyn crate::usecase::GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn crate::usecase::GetRepoByUsernameQuery>,
    auth: Arc<dyn AuthApp>,
    database: Arc<database_manager::App>,
}

impl UpdateProperty {
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
impl UpdatePropertyInputPort for UpdateProperty {
    #[tracing::instrument(name = "UpdateProperty::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: UpdatePropertyInputData<'a>,
    ) -> errors::Result<Property> {
        self.auth
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "database:UpdateProperty",
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

        let property = self
            .database
            .update_property()
            .execute(database_manager::usecase::UpdatePropertyInputData {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                tenant_id: org.id(),
                database_id: &repo.databases().first().unwrap().clone(),
                property_id: &input.property_id.parse()?,
                name: input.property_name.as_deref(),
                property_type: input.property_type,
                meta_json: input.meta_json,
            })
            .await?;
        Ok(property)
    }
}
