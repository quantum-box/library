use database_manager::usecase::{
    FindAllPropertiesInputData, UpsertOutcome,
};

use crate::usecase::{UpsertDataInputData, UpsertDataInputPort};
use std::sync::Arc;
use tachyon_sdk::auth::{AuthApp, CheckPolicyInput};

use super::property_value_adapter::property_value_command;

/// Create-or-update a record at a caller-supplied id.
///
/// The org/repo resolution, the policy check and the Property mapping are
/// `UpdateData`'s; only the missing-record case differs, and that difference
/// lives in the Database bounded context. Authorization is deliberately the
/// same `library:UpdateRepo` this endpoint's neighbours use: an upsert is a
/// write to an existing repository either way, and giving it its own action
/// would let a caller hold one and not the other.
#[derive(Clone)]
pub struct UpsertData {
    get_org_by_username:
        Arc<dyn crate::usecase::GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn crate::usecase::GetRepoByUsernameQuery>,
    auth: Arc<dyn AuthApp>,
    database: Arc<database_manager::App>,
}

impl std::fmt::Debug for UpsertData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpsertData").finish_non_exhaustive()
    }
}

impl UpsertData {
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
impl UpsertDataInputPort for UpsertData {
    #[tracing::instrument(name = "UpsertData::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: UpsertDataInputData<'a>,
    ) -> errors::Result<(
        database_manager::domain::Data,
        Vec<database_manager::domain::Property>,
        UpsertOutcome,
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

        let (data, outcome) = self
            .database
            .upsert_data_usecase()
            .execute(database_manager::UpsertDataInputData {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                tenant_id: org.id(),
                database_id: &database_id,
                data_id: &input.data_id.parse()?,
                name: input.data_name,
                data: property_data,
            })
            .await?;

        Ok((data, properties, outcome))
    }
}

#[cfg(test)]
mod architecture_tests {
    /// The generic record commands must not reach for GitHub write-back
    /// themselves; that is the decorator's job, and mixing the two would make
    /// an upsert push to GitHub even where the caller composed it without one.
    #[test]
    fn upsert_data_has_no_github_writeback_dependency() {
        let source = include_str!("upsert_data.rs");
        let implementation =
            source.split("#[cfg(test)]").next().unwrap_or(source);

        assert!(!implementation.contains("outbound_sync"));
        assert!(!implementation.contains("ext_github"));
    }
}
