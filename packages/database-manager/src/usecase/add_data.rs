use std::sync::Arc;

use chrono::Utc;

use crate::domain::{
    Data, DataId, DataRepository, Database, DatabaseId, Property,
    PropertyData, PropertyRepository, PropertyType,
};
use crate::usecase::database_scope::DatabaseScope;
use crate::usecase::{
    AddDataInputData, AddDataInputPort, RelationTargetPolicy,
    RelationTargetValidationPort,
};
use value_object::RepositoryV1;

#[derive(Debug, Clone)]
pub struct AddDataInteractorImpl {
    database_repo: Arc<dyn RepositoryV1<DatabaseId, Database>>,
    property_repo: Arc<dyn PropertyRepository>,
    data_repo: Arc<dyn DataRepository>,
    relation_target_validator: Arc<dyn RelationTargetValidationPort>,
}

fn populate_auto_generated_ids(
    properties: &[Property],
    property_data: &mut Vec<PropertyData>,
    data_id: &DataId,
) -> errors::Result<()> {
    for property in properties.iter().filter(|property| {
        matches!(
            property.property_type(),
            PropertyType::Id(type_id) if type_id.auto_generate
        )
    }) {
        match property_data
            .iter()
            .position(|value| value.property_id() == property.id())
        {
            Some(index) if property_data[index].value().is_none() => {
                property_data[index] =
                    PropertyData::new(property, data_id.to_string())?;
            }
            Some(_) => {
                return Err(errors::Error::business_logic(
                    "Auto-generated Id property does not accept an explicit value",
                ));
            }
            None => property_data
                .push(PropertyData::new(property, data_id.to_string())?),
        }
    }

    Ok(())
}

impl AddDataInteractorImpl {
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
impl AddDataInputPort for AddDataInteractorImpl {
    async fn execute(
        &self,
        input: AddDataInputData<'_>,
    ) -> errors::Result<Data> {
        let scope = DatabaseScope::new(input.tenant_id, input.database_id);
        let database =
            scope.require_database(self.database_repo.as_ref()).await?;
        let properties = self
            .property_repo
            .find_all(database.id(), database.tenant_id())
            .await?;

        // Generate the canonical record ID before parsing property values so
        // an auto-generated Id property can store that exact identifier.
        let data_id = DataId::default();
        let mut property_data_list = Vec::new();
        for val in input.property_data.into_iter() {
            let property = properties
                .iter()
                .find(|x| x.id() == &val.property_id)
                .ok_or_else(DatabaseScope::not_found)?;
            let col = PropertyData::new(property, val.value)?;
            self.relation_target_validator
                .validate(input.tenant_id, &col)
                .await?;
            property_data_list.push(col);
        }
        populate_auto_generated_ids(
            &properties,
            &mut property_data_list,
            &data_id,
        )?;
        let data = Data::new(
            &data_id,
            database.tenant_id(),
            database.id(),
            input.name,
            property_data_list,
            Utc::now(),
            Utc::now(),
        )?;

        self.data_repo.create(&data).await?;
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{DatabaseId, PropertyId, TypeId};
    use value_object::TenantId;

    fn id_property(auto_generate: bool) -> Property {
        Property::new(
            &PropertyId::default(),
            &TenantId::default(),
            &DatabaseId::default(),
            "id",
            &PropertyType::Id(TypeId::new(auto_generate)),
            false,
            0,
        )
    }

    #[test]
    fn omitted_auto_generated_id_uses_the_canonical_data_id() {
        let property = id_property(true);
        let data_id = DataId::default();
        let mut property_data = vec![];

        populate_auto_generated_ids(
            &[property.clone()],
            &mut property_data,
            &data_id,
        )
        .expect("auto generation must succeed");

        assert_eq!(property_data.len(), 1);
        assert_eq!(property_data[0].property_id(), property.id());
        assert_eq!(property_data[0].string_value(), data_id.to_string());
    }

    #[test]
    fn empty_auto_generated_id_is_replaced() {
        let property = id_property(true);
        let data_id = DataId::default();
        let mut property_data =
            vec![PropertyData::new(&property, String::new())
                .expect("empty property data must be valid")];

        populate_auto_generated_ids(
            &[property],
            &mut property_data,
            &data_id,
        )
        .expect("auto generation must succeed");

        assert_eq!(property_data[0].string_value(), data_id.to_string());
    }

    #[test]
    fn explicit_auto_generated_id_value_is_rejected() {
        let property = id_property(true);
        let data_id = DataId::default();
        let mut property_data =
            vec![PropertyData::new(&property, "external-id".to_string())
                .expect("explicit Id value must be valid")];

        let error = populate_auto_generated_ids(
            &[property],
            &mut property_data,
            &data_id,
        )
        .expect_err("an auto-generated Id is server-owned");

        assert!(error.is_bad_request());
        assert!(error
            .to_string()
            .contains("does not accept an explicit value"));
    }

    #[test]
    fn manual_id_property_is_not_generated() {
        let property = id_property(false);
        let mut property_data = vec![];

        populate_auto_generated_ids(
            &[property],
            &mut property_data,
            &DataId::default(),
        )
        .expect("manual Id handling must succeed");

        assert!(property_data.is_empty());
    }
}
