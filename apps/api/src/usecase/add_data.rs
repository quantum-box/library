use std::sync::Arc;

use super::{
    AddDataInputData, AddDataInputPort, GetOrganizationByUsernameQuery,
    GetRepoByUsernameQuery, PropertyDataValueInputData,
};

use database_manager::{
    domain::Data, domain::Property, PropertyDataInputData,
};
use tachyon_sdk::auth::{AuthApp, CheckPolicyInput};
use value_object::Ulid;

#[derive(Debug, Clone)]
pub struct AddData {
    auth_ctx: Arc<dyn AuthApp>,
    get_repo_by_username: Arc<dyn GetRepoByUsernameQuery>,
    get_org_by_username: Arc<dyn GetOrganizationByUsernameQuery>,
    database_client: Arc<database_manager::App>,
}

impl AddData {
    pub fn new(
        auth_ctx: Arc<dyn AuthApp>,
        get_repo_by_username: Arc<dyn GetRepoByUsernameQuery>,
        get_org_by_username: Arc<dyn GetOrganizationByUsernameQuery>,
        database_client: Arc<database_manager::App>,
    ) -> Arc<Self> {
        Arc::new(Self {
            auth_ctx,
            get_repo_by_username,
            get_org_by_username,
            database_client,
        })
    }

    fn convert_property_data_value_input_data_to_property_data_input_dto(
        &self,
        input: &AddDataInputData,
        properties: &[Property],
    ) -> errors::Result<Vec<PropertyDataInputData>> {
        let mut outputs = Vec::with_capacity(input.property_data.len());

        for p in &input.property_data {
            let property = properties
                .iter()
                .find(|pr| p.property_id == **pr.id())
                .ok_or_else(|| errors::Error::not_found("property"))?;

            let value = match &p.value {
                PropertyDataValueInputData::String(s) => {
                    if property.name() == "id" {
                        Ulid::new().to_string().to_lowercase()
                    } else {
                        s.clone()
                    }
                }
                PropertyDataValueInputData::Integer(s) => s.clone(),
                PropertyDataValueInputData::Html(s) => s.clone(),
                PropertyDataValueInputData::Markdown(s) => s.clone(),
                PropertyDataValueInputData::Relation(s) => s.join(","),
                PropertyDataValueInputData::Select(s) => s.clone(),
                PropertyDataValueInputData::MultiSelect(s) => s.join(","),
                PropertyDataValueInputData::Location(l) => {
                    format!("{},{}", l.latitude(), l.longitude())
                }
                PropertyDataValueInputData::Date(s) => s.clone(),
                PropertyDataValueInputData::Image(s) => s.clone(),
            };

            outputs.push(PropertyDataInputData {
                property_id: p.property_id.clone(),
                value,
            });
        }

        Ok(outputs)
    }
}

#[async_trait::async_trait]
impl AddDataInputPort for AddData {
    /// TODO: add English documentation
    #[tracing::instrument(name = "SaveData::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: AddDataInputData<'a>,
    ) -> errors::Result<(Data, Vec<Property>)> {
        let org = self
            .get_org_by_username
            .execute(&input.org_username.parse()?)
            .await?
            .ok_or(errors::Error::not_found("organization"))?;
        let repo = self
            .get_repo_by_username
            .execute(org.username(), &input.repo_username.parse()?)
            .await?
            .ok_or(errors::Error::not_found("repo"))?;

        // TODO: add English comment
        self.auth_ctx
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:UpdateRepo",
            })
            .await?;

        let properties = self
            .database_client
            .find_all_properties()
            .execute(
                database_manager::usecase::FindAllPropertiesInputData {
                    tenant_id: org.id().clone(),
                    database_id: repo.databases().first().unwrap().clone(),
                },
            )
            .await?;
        // TODO: add English comment
        let property_data =
            self.convert_property_data_value_input_data_to_property_data_input_dto(&input, &properties)?;

        let data = self
            .database_client
            .add_data_usecase()
            .execute(database_manager::AddDataInputData {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                tenant_id: org.id(),
                database_id: repo.databases().first().unwrap(),
                name: input.data_name,
                property_data,
            })
            .await?;
        Ok((data, properties))
    }
}
