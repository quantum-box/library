use database_manager::usecase::FindAllPropertiesInputData;

use crate::usecase::{UpdateDataInputData, UpdateDataInputPort};
use std::sync::Arc;
use tachyon_sdk::auth::{AuthApp, CheckPolicyInput};

use super::property_value_adapter::property_value_command;

#[derive(Clone)]
pub struct UpdateData {
    get_org_by_username:
        Arc<dyn crate::usecase::GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn crate::usecase::GetRepoByUsernameQuery>,
    auth: Arc<dyn AuthApp>,
    database: Arc<database_manager::App>,
}

impl std::fmt::Debug for UpdateData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpdateData").finish_non_exhaustive()
    }
}

impl UpdateData {
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
impl UpdateDataInputPort for UpdateData {
    #[tracing::instrument(name = "UpdateData::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: UpdateDataInputData<'a>,
    ) -> errors::Result<(
        database_manager::domain::Data,
        Vec<database_manager::domain::Property>,
    )> {
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

        self.auth
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:UpdateRepo",
            })
            .await?;

        let database_id = repo
            .databases()
            .first()
            .ok_or_else(|| {
                errors::Error::application_logic_error(
                    "Repository has no associated database",
                )
            })?
            .clone();

        let properties = self
            .database
            .find_all_properties()
            .execute(FindAllPropertiesInputData {
                tenant_id: org.id().clone(),
                database_id: database_id.clone(),
            })
            .await?;
        let property_data = input
            .property_data
            .iter()
            .map(|data| {
                let property = properties
                    .iter()
                    .find(|property| {
                        property.id().as_str() == data.property_id
                    })
                    .ok_or_else(|| errors::Error::not_found("property"))?;
                Ok(database_manager::PropertyDataInputData {
                    property_id: property.id().clone(),
                    value: property_value_command(property, &data.value)?,
                })
            })
            .collect::<errors::Result<Vec<_>>>()?;

        let data = self
            .database
            .update_data_usecase()
            .execute(database_manager::UpdateDataInputData {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                tenant_id: org.id(),
                database_id: &database_id,
                data_id: &input.data_id.parse()?,
                name: input.data_name,
                data: property_data,
            })
            .await?;

        Ok((data, properties))
    }
}

#[cfg(test)]
mod architecture_tests {
    #[test]
    fn generic_data_commands_have_no_github_writeback_dependency() {
        for source in
            [include_str!("add_data.rs"), include_str!("update_data.rs")]
        {
            let implementation =
                source.split("#[cfg(test)]").next().unwrap_or(source);

            assert!(!implementation.contains("outbound_sync"));
            assert!(!implementation.contains("ext_github"));
        }
    }
}
