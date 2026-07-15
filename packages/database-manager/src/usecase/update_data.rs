use std::sync::Arc;

use crate::domain::{
    Data, DataRepository, Database, DatabaseId, PropertyData, PropertyId,
    PropertyRepository, PropertyType,
};
use crate::usecase::{UpdateDataInputData, UpdateDataInputPort};
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

#[derive(Debug)]
pub struct UpdateDataInteractorImpl {
    database_repo: Arc<dyn RepositoryV1<DatabaseId, Database>>,
    property_repo: Arc<dyn PropertyRepository>,
    data_repo: Arc<dyn DataRepository>,
}

impl UpdateDataInteractorImpl {
    pub fn new(
        database_repo: Arc<dyn RepositoryV1<DatabaseId, Database>>,
        property_repo: Arc<dyn PropertyRepository>,
        data_repo: Arc<dyn DataRepository>,
    ) -> Arc<Self> {
        Arc::new(Self {
            database_repo,
            property_repo,
            data_repo,
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
        let database = self
            .database_repo
            .get_by_id(input.tenant_id, input.database_id)
            .await?
            .ok_or(errors::not_found!(
                "database is not found in update data"
            ))?;
        let property = self
            .property_repo
            .find_all(database.id(), database.tenant_id())
            .await?;
        let mut data = self
            .data_repo
            .find_by_id(input.data_id, database.id(), database.tenant_id())
            .await?
            .ok_or(errors::not_found!(
                "data is not found in update data"
            ))?;

        data.update_name(&input.name.parse()?);
        for d in input.data {
            let property = property
                .iter()
                .find(|p| {
                    PropertyId::new(&d.property_id).unwrap().eq(p.id())
                })
                .ok_or(errors::not_found!(
                    "property is not found in update data"
                ))?;
            validate_auto_generated_id_update(
                property,
                input.data_id,
                data.get_property_data(property.id()),
                &d.value,
            )?;
            data.update_property_data(&PropertyData::new(
                property,
                d.value.to_string(),
            )?)?;
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
}
