use std::sync::Arc;

use crate::domain::{
    Data, DataRepository, Database, DatabaseId, PatchRecordCommand,
    PropertyData, PropertyRepository, PropertyType, PropertyValueChange,
    RecordUnitOfWork,
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
    command: &crate::domain::PropertyValueCommand,
) -> errors::Result<()> {
    if !matches!(
        property.property_type(),
        PropertyType::Id(type_id) if type_id.auto_generate
    ) {
        return Ok(());
    }

    let crate::domain::PropertyValueCommand::Id(value) = command else {
        return Err(errors::Error::business_logic(
            "Auto-generated Id property is immutable",
        ));
    };
    if value != data_id.as_str()
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
    record_uow: Arc<dyn RecordUnitOfWork>,
    relation_target_validator: Arc<dyn RelationTargetValidationPort>,
}

impl UpdateDataInteractorImpl {
    pub fn new(
        database_repo: Arc<dyn RepositoryV1<DatabaseId, Database>>,
        property_repo: Arc<dyn PropertyRepository>,
        data_repo: Arc<dyn DataRepository>,
        record_uow: Arc<dyn RecordUnitOfWork>,
    ) -> Arc<Self> {
        let relation_target_validator = RelationTargetPolicy::new(
            database_repo.clone(),
            data_repo.clone(),
        );
        Self::new_with_relation_target_validator(
            database_repo,
            property_repo,
            data_repo,
            record_uow,
            relation_target_validator,
        )
    }

    pub fn new_with_relation_target_validator(
        database_repo: Arc<dyn RepositoryV1<DatabaseId, Database>>,
        property_repo: Arc<dyn PropertyRepository>,
        data_repo: Arc<dyn DataRepository>,
        record_uow: Arc<dyn RecordUnitOfWork>,
        relation_target_validator: Arc<dyn RelationTargetValidationPort>,
    ) -> Arc<Self> {
        Arc::new(Self {
            database_repo,
            property_repo,
            data_repo,
            record_uow,
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
        let mut data = scope
            .require_data(self.data_repo.as_ref(), input.data_id)
            .await?;

        data.update_name(&input.name.parse()?);
        let mut changes = Vec::with_capacity(input.data.len());
        for d in input.data {
            // Resolve only the requested Property. A newer opaque sibling is
            // still hydrated losslessly but must not block an unrelated
            // record patch through this older compatibility boundary.
            let property = self
                .property_repo
                .find_by_id(
                    &d.property_id,
                    database.id(),
                    database.tenant_id(),
                )
                .await?
                .ok_or_else(DatabaseScope::not_found)?;
            validate_auto_generated_id_update(
                &property,
                input.data_id,
                data.get_property_data(property.id()),
                &d.value,
            )?;
            let property_data =
                PropertyData::from_command(&property, d.value)?;
            self.relation_target_validator
                .validate(input.tenant_id, &property_data)
                .await?;
            data.update_property_data(&property_data)?;
            changes.push(PropertyValueChange::from_property_data(
                &property,
                &property_data,
            )?);
        }
        self.record_uow
            .patch_atomically(&PatchRecordCommand {
                record: data.clone(),
                changes,
            })
            .await?;

        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::PropertyId;
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
            &crate::domain::PropertyValueCommand::Id(data_id.to_string()),
        )
        .expect("the canonical value must remain valid");
    }

    #[test]
    fn auto_generated_id_cannot_be_changed() {
        let error = validate_auto_generated_id_update(
            &auto_generated_id_property(),
            &crate::domain::DataId::default(),
            None,
            &crate::domain::PropertyValueCommand::Id(
                "replacement".to_string(),
            ),
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
            &crate::domain::PropertyValueCommand::Id(
                "legacy-id".to_string(),
            ),
        )
        .expect(
            "an existing legacy value must remain readable and immutable",
        );
    }
}
