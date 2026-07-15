use std::sync::Arc;

use crate::domain::{
    Data, DataRepository, Database, DatabaseId, PropertyData, PropertyId,
    PropertyRepository, PropertyType,
};
use crate::usecase::database_scope::DatabaseScope;
use crate::usecase::{
    RelationTargetPolicy, RelationTargetValidationPort,
    UpdateDataInputData, UpdateDataInputPort,
};
use value_object::RepositoryV1;

fn validate_auto_generated_id_update(
    property: &crate::domain::Property,
    data_id: &crate::domain::DataId,
    existing_value: Option<&PropertyData>,
    value: &str,
) -> errors::Result<()> {
    if matches!(
        property.property_type(),
        PropertyType::Id(type_id) if type_id.auto_generate
    ) && value != data_id.as_str()
        && existing_value.map(PropertyData::string_value).as_deref()
            != Some(value)
    {
        return Err(errors::Error::business_logic(
            "Auto-generated Id property is immutable",
        ));
    }

    Ok(())
}

fn find_scoped_property<'a>(
    properties: &'a [crate::domain::Property],
    property_id: &str,
) -> errors::Result<&'a crate::domain::Property> {
    let property_id = property_id.parse::<PropertyId>()?;
    properties
        .iter()
        .find(|property| property.id() == &property_id)
        .ok_or_else(DatabaseScope::not_found)
}

#[derive(Debug)]
pub struct UpdateDataInteractorImpl {
    database_repo: Arc<dyn RepositoryV1<DatabaseId, Database>>,
    property_repo: Arc<dyn PropertyRepository>,
    data_repo: Arc<dyn DataRepository>,
    relation_target_validator: Arc<dyn RelationTargetValidationPort>,
}

impl UpdateDataInteractorImpl {
    pub fn new(
        database_repo: Arc<dyn RepositoryV1<DatabaseId, Database>>,
        property_repo: Arc<dyn PropertyRepository>,
        data_repo: Arc<dyn DataRepository>,
    ) -> Arc<Self> {
        let relation_target_validator = RelationTargetPolicy::new(
            database_repo.clone(),
            data_repo.clone(),
        );
        Self::new_with_relation_target_validator(
            database_repo,
            property_repo,
            data_repo,
            relation_target_validator,
        )
    }

    pub fn new_with_relation_target_validator(
        database_repo: Arc<dyn RepositoryV1<DatabaseId, Database>>,
        property_repo: Arc<dyn PropertyRepository>,
        data_repo: Arc<dyn DataRepository>,
        relation_target_validator: Arc<dyn RelationTargetValidationPort>,
    ) -> Arc<Self> {
        Arc::new(Self {
            database_repo,
            property_repo,
            data_repo,
            relation_target_validator,
        })
    }
}

#[async_trait::async_trait]
impl UpdateDataInputPort for UpdateDataInteractorImpl {
    #[tracing::instrument(skip(self))]
    async fn execute(
        &self,
        input: UpdateDataInputData<'_>,
    ) -> errors::Result<Data> {
        let scope = DatabaseScope::new(input.tenant_id, input.database_id);
        let database =
            scope.require_database(self.database_repo.as_ref()).await?;
        let property = self
            .property_repo
            .find_all(database.id(), database.tenant_id())
            .await?;
        let mut data = scope
            .require_data(self.data_repo.as_ref(), input.data_id)
            .await?;

        data.update_name(&input.name.parse()?);
        for d in input.data {
            let property = find_scoped_property(&property, &d.property_id)?;
            validate_auto_generated_id_update(
                property,
                input.data_id,
                data.get_property_data(property.id()),
                &d.value,
            )?;
            let property_data =
                PropertyData::new(property, d.value.to_string())?;
            self.relation_target_validator
                .validate(input.tenant_id, &property_data)
                .await?;
            data.update_property_data(&property_data)?;
        }
        self.data_repo.update(&data).await?;

        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Property, TypeId};
    use value_object::TenantId;

    fn auto_generated_id_property() -> Property {
        Property::new(
            &PropertyId::default(),
            &TenantId::default(),
            &DatabaseId::default(),
            "id",
            &PropertyType::Id(TypeId::new(true)),
            false,
            0,
        )
    }

    #[test]
    fn canonical_auto_generated_id_is_accepted_as_a_no_op() {
        let property = auto_generated_id_property();
        let data_id = crate::domain::DataId::default();

        validate_auto_generated_id_update(
            &property,
            &data_id,
            None,
            &data_id.to_string(),
        )
        .expect("the canonical value must remain valid");
    }

    #[test]
    fn auto_generated_id_cannot_be_changed() {
        let error = validate_auto_generated_id_update(
            &auto_generated_id_property(),
            &crate::domain::DataId::default(),
            None,
            "replacement",
        )
        .expect_err("an auto-generated Id is immutable");

        assert!(error.is_bad_request());
        assert!(error.to_string().contains("immutable"));
    }

    #[test]
    fn legacy_non_canonical_auto_generated_id_can_remain_unchanged() {
        let property = auto_generated_id_property();
        let existing =
            PropertyData::new(&property, "legacy-id".to_string())
                .expect("legacy value");

        validate_auto_generated_id_update(
            &property,
            &crate::domain::DataId::default(),
            Some(&existing),
            "legacy-id",
        )
        .expect(
            "an existing legacy value must remain readable and immutable",
        );
    }

    #[test]
    fn malformed_property_id_returns_a_domain_error() {
        let error = find_scoped_property(&[], "not-a-property-id")
            .expect_err("a malformed property id must not panic");

        assert!(error.is_bad_request());
        assert!(error.to_string().contains("PropertyId"));
    }
}
