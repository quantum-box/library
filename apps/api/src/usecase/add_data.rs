use std::sync::Arc;

use super::property_value_adapter::property_value_command;
use super::{
    AddDataInputData, AddDataInputPort, GetOrganizationByUsernameQuery,
    GetRepoByUsernameQuery, PropertyDataValueInputData,
};

use database_manager::{
    domain::Data, domain::Property, domain::PropertyValueCommand,
    PropertyDataInputData,
};
use tachyon_sdk::auth::{AuthApp, CheckPolicyInput};
use value_object::Ulid;

fn property_value_for_database(
    property: &Property,
    value: &PropertyDataValueInputData,
) -> errors::Result<PropertyValueCommand> {
    match value {
        PropertyDataValueInputData::String(_)
            if property.name() == "id"
                && matches!(
                    property.property_type(),
                    database_manager::domain::PropertyType::String
                ) =>
        {
            // Legacy repositories model their generated ID as a String named
            // "id". Typed Id generation belongs to the Database BC instead.
            Ok(PropertyValueCommand::String(
                Ulid::new().to_string().to_lowercase(),
            ))
        }
        _ => property_value_command(property, value),
    }
}

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

            let value = property_value_for_database(property, &p.value)?;

            outputs.push(PropertyDataInputData {
                property_id: property.id().clone(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use database_manager::domain::{
        DatabaseId, PropertyId, PropertyType, TypeId,
    };
    use value_object::TenantId;

    fn property(name: &str, property_type: PropertyType) -> Property {
        Property::new(
            &PropertyId::default(),
            &TenantId::default(),
            &DatabaseId::default(),
            name,
            &property_type,
            false,
            0,
        )
    }

    #[test]
    fn legacy_string_id_generation_stays_in_the_library_adapter() {
        let property = property("id", PropertyType::String);

        let generated = property_value_for_database(
            &property,
            &PropertyDataValueInputData::String(String::new()),
        )
        .expect("command");

        assert!(matches!(
            generated,
            PropertyValueCommand::String(value) if !value.is_empty()
        ));
    }

    #[test]
    fn typed_id_generation_is_left_to_the_database_context() {
        let property = property("id", PropertyType::Id(TypeId::new(true)));

        let value = property_value_for_database(
            &property,
            &PropertyDataValueInputData::String(String::new()),
        )
        .expect("command");

        assert_eq!(value, PropertyValueCommand::Clear);
    }
}
