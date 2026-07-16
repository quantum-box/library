use sqlx::{MySql, Transaction};

use super::{
    encoded_definition, map_schema_insert_error, FieldRow,
    PropertyRepositoryImpl, RelationDefinitionRow,
};
use crate::domain::{
    DeleteRelationDefinitionCommand, InversePropertyMutation, Property,
    PropertyDefinition, PropertyId, ReconfigureRelationDefinitionCommand,
    RelationDefinition, RelationGeneration, RelationSchema,
    RelationSchemaMutationPort,
};
use value_object::TenantId;

const RELATION_DEFINITION_COLUMNS: &str = r#"
    id, tenant_id, object_id, field_id, target_object_id,
    forward_cardinality, reverse_cardinality, inverse_field_id,
    inverse_owned, on_target_delete, definition_version, generation
"#;

struct LockedRelationSchema {
    definition: RelationDefinition,
    source_property: PropertyDefinition,
    current_inverse: Option<PropertyDefinition>,
    target_properties: Vec<PropertyDefinition>,
}

async fn relation_target(
    repository: &PropertyRepositoryImpl,
    tenant_id: &TenantId,
    source_database_id: &crate::domain::DatabaseId,
    source_property_id: &PropertyId,
) -> errors::Result<Option<crate::domain::DatabaseId>> {
    sqlx::query_scalar::<_, String>(
        r#"
        SELECT target_object_id
        FROM relationships
        WHERE tenant_id = ? AND object_id = ? AND field_id = ?
        LIMIT 1
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(source_database_id.to_string())
    .bind(source_property_id.to_string())
    .fetch_optional(repository.db.pool().as_ref())
    .await?
    .map(|id| crate::domain::DatabaseId::new(&id))
    .transpose()
}

async fn lock_endpoints(
    transaction: &mut Transaction<'_, MySql>,
    tenant_id: &TenantId,
    source_database_id: &crate::domain::DatabaseId,
    target_database_id: &crate::domain::DatabaseId,
) -> errors::Result<()> {
    let mut endpoint_ids = [
        source_database_id.to_string(),
        target_database_id.to_string(),
    ];
    endpoint_ids.sort();
    let locked = sqlx::query_scalar::<_, String>(
        r#"
        SELECT id
        FROM objects
        WHERE tenant_id = ? AND id IN (?, ?)
        ORDER BY id
        FOR UPDATE
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(&endpoint_ids[0])
    .bind(&endpoint_ids[1])
    .fetch_all(&mut **transaction)
    .await?;
    let expected_count = if source_database_id == target_database_id {
        1
    } else {
        2
    };
    if locked.len() != expected_count
        || !locked.contains(&source_database_id.to_string())
        || !locked.contains(&target_database_id.to_string())
    {
        return Err(errors::Error::not_found("resource not found"));
    }
    Ok(())
}

async fn load_locked_schema(
    repository: &PropertyRepositoryImpl,
    transaction: &mut Transaction<'_, MySql>,
    tenant_id: &TenantId,
    source_database_id: &crate::domain::DatabaseId,
    source_property_id: &PropertyId,
    expected_target_database_id: &crate::domain::DatabaseId,
) -> errors::Result<LockedRelationSchema> {
    let sql = format!(
        r#"
        SELECT {RELATION_DEFINITION_COLUMNS}
        FROM relationships
        WHERE tenant_id = ? AND object_id = ? AND field_id = ?
        FOR UPDATE
        "#,
    );
    let definition = sqlx::query_as::<_, RelationDefinitionRow>(&sql)
        .bind(tenant_id.to_string())
        .bind(source_database_id.to_string())
        .bind(source_property_id.to_string())
        .fetch_optional(&mut **transaction)
        .await?
        .ok_or_else(|| errors::Error::not_found("resource not found"))
        .and_then(RelationDefinition::try_from)?;
    if definition.target_database_id() != expected_target_database_id {
        return Err(errors::Error::conflict(
            "RelationDefinition target changed while acquiring endpoint locks",
        ));
    }

    let source_row = sqlx::query_as::<_, FieldRow>(
        r#"
        SELECT id, tenant_id, object_id, field_name, datatype,
               datatype_meta, is_indexed, field_num, meta_json,
               type_key, type_version, type_config
        FROM fields
        WHERE tenant_id = ? AND object_id = ? AND id = ?
        FOR UPDATE
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(source_database_id.to_string())
    .bind(source_property_id.to_string())
    .fetch_optional(&mut **transaction)
    .await?
    .ok_or_else(|| errors::Error::not_found("resource not found"))?;
    let source_property = source_row
        .definition_for_schema_write(repository.definition_mode)?;

    let target_rows = sqlx::query_as::<_, FieldRow>(
        r#"
        SELECT id, tenant_id, object_id, field_name, datatype,
               datatype_meta, is_indexed, field_num, meta_json,
               type_key, type_version, type_config
        FROM fields
        WHERE tenant_id = ? AND object_id = ?
        ORDER BY field_num, id
        FOR UPDATE
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(expected_target_database_id.to_string())
    .fetch_all(&mut **transaction)
    .await?;
    let target_properties = target_rows
        .iter()
        .map(|row| row.definition(repository.definition_mode))
        .collect::<errors::Result<Vec<_>>>()?;
    let current_inverse = definition
        .inverse_property_id()
        .as_ref()
        .map(|inverse_id| {
            target_rows
                .iter()
                .find(|row| row.id == inverse_id.as_str())
                .ok_or_else(|| {
                    errors::Error::conflict(
                        "RelationDefinition inverse Property is missing",
                    )
                })
                .and_then(|row| {
                    row.definition_for_schema_write(
                        repository.definition_mode,
                    )
                })
        })
        .transpose()?;

    Ok(LockedRelationSchema {
        definition,
        source_property,
        current_inverse,
        target_properties,
    })
}

async fn insert_property(
    transaction: &mut Transaction<'_, MySql>,
    definition: &PropertyDefinition,
) -> errors::Result<()> {
    let (property, type_key, type_version, type_config) =
        encoded_definition(definition)?;
    let result = sqlx::query(
        r#"
        INSERT INTO fields
            (id, tenant_id, object_id, field_name, datatype,
             datatype_meta, is_indexed, field_num, meta_json,
             type_key, type_version, type_config)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(property.id().to_string())
    .bind(property.tenant_id().to_string())
    .bind(property.database_id().to_string())
    .bind(property.name())
    .bind(property.property_type().to_string())
    .bind(property.property_type().get_meta()?)
    .bind(property.is_indexed())
    .bind(property.property_num())
    .bind(property.meta_json())
    .bind(type_key)
    .bind(type_version)
    .bind(type_config)
    .execute(&mut **transaction)
    .await;
    result
        .map_err(|error| {
            map_schema_insert_error(error, property.property_type())
        })
        .map(|_| ())
}

async fn replace_property(
    transaction: &mut Transaction<'_, MySql>,
    definition: &PropertyDefinition,
) -> errors::Result<()> {
    let (property, type_key, type_version, type_config) =
        encoded_definition(definition)?;
    let result = sqlx::query(
        r#"
        UPDATE fields
        SET field_name = ?, datatype = ?, datatype_meta = ?,
            is_indexed = ?, meta_json = ?, type_key = ?,
            type_version = ?, type_config = ?
        WHERE tenant_id = ? AND object_id = ? AND id = ?
        "#,
    )
    .bind(property.name())
    .bind(property.property_type().to_string())
    .bind(property.property_type().get_meta()?)
    .bind(property.is_indexed())
    .bind(property.meta_json())
    .bind(type_key)
    .bind(type_version)
    .bind(type_config)
    .bind(property.tenant_id().to_string())
    .bind(property.database_id().to_string())
    .bind(property.id().to_string())
    .execute(&mut **transaction)
    .await?;
    if result.rows_affected() != 1 {
        return Err(errors::Error::not_found("resource not found"));
    }
    Ok(())
}

async fn delete_property(
    transaction: &mut Transaction<'_, MySql>,
    definition: &PropertyDefinition,
) -> errors::Result<()> {
    let field_num = definition.property_num();
    sqlx::query(&format!(
        "UPDATE data SET value{field_num} = NULL \
         WHERE tenant_id = ? AND object_id = ?"
    ))
    .bind(definition.tenant_id().to_string())
    .bind(definition.database_id().to_string())
    .execute(&mut **transaction)
    .await?;
    let result = sqlx::query(
        r#"
        DELETE FROM fields
        WHERE tenant_id = ? AND object_id = ? AND id = ?
        "#,
    )
    .bind(definition.tenant_id().to_string())
    .bind(definition.database_id().to_string())
    .bind(definition.id().to_string())
    .execute(&mut **transaction)
    .await?;
    if result.rows_affected() != 1 {
        return Err(errors::Error::not_found("resource not found"));
    }
    Ok(())
}

async fn replace_relation_definition(
    transaction: &mut Transaction<'_, MySql>,
    definition: &RelationDefinition,
    expected_generation: RelationGeneration,
) -> errors::Result<()> {
    let expected_next = expected_generation.next()?;
    if definition.generation() != &expected_next {
        return Err(errors::Error::invalid(
            "replacement RelationDefinition must increment generation once",
        ));
    }
    let result = sqlx::query(
        r#"
        UPDATE relationships
        SET forward_cardinality = ?, reverse_cardinality = ?,
            inverse_field_id = ?, inverse_owned = ?,
            on_target_delete = ?, definition_version = ?, generation = ?
        WHERE tenant_id = ? AND object_id = ? AND field_id = ?
          AND id = ? AND generation = ?
        "#,
    )
    .bind(definition.forward_cardinality().to_string())
    .bind(definition.reverse_cardinality().to_string())
    .bind(
        definition
            .inverse_property_id()
            .as_ref()
            .map(ToString::to_string),
    )
    .bind(*definition.inverse_property_owned())
    .bind(definition.on_target_delete().to_string())
    .bind(definition.definition_version().get())
    .bind(definition.generation().get())
    .bind(definition.tenant_id().to_string())
    .bind(definition.source_database_id().to_string())
    .bind(definition.source_property_id().to_string())
    .bind(definition.id().to_string())
    .bind(expected_generation.get())
    .execute(&mut **transaction)
    .await?;
    if result.rows_affected() != 1 {
        return Err(errors::Error::conflict(
            "RelationDefinition generation changed",
        ));
    }
    Ok(())
}

async fn delete_relation_schema(
    repository: &PropertyRepositoryImpl,
    tenant_id: &TenantId,
    source_database_id: &crate::domain::DatabaseId,
    source_property_id: &PropertyId,
    expected_generation: Option<RelationGeneration>,
) -> errors::Result<Option<(RelationDefinition, Property)>> {
    let Some(target_database_id) = relation_target(
        repository,
        tenant_id,
        source_database_id,
        source_property_id,
    )
    .await?
    else {
        return Ok(None);
    };
    let mut transaction = repository.db.pool().begin().await?;
    lock_endpoints(
        &mut transaction,
        tenant_id,
        source_database_id,
        &target_database_id,
    )
    .await?;
    let current = load_locked_schema(
        repository,
        &mut transaction,
        tenant_id,
        source_database_id,
        source_property_id,
        &target_database_id,
    )
    .await?;
    let generation =
        expected_generation.unwrap_or(*current.definition.generation());
    let command = DeleteRelationDefinitionCommand::new(
        tenant_id,
        source_database_id,
        source_property_id,
        generation,
    );
    let deletion = RelationSchema::plan_deletion(
        &current.definition,
        &current.source_property,
        current.current_inverse.as_ref(),
        &command,
    )?;
    let (definition, source_property, owned_inverse) =
        deletion.into_parts();
    let result = sqlx::query(
        r#"
        DELETE FROM relationships
        WHERE tenant_id = ? AND object_id = ? AND field_id = ?
          AND id = ? AND generation = ?
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(source_database_id.to_string())
    .bind(source_property_id.to_string())
    .bind(definition.id().to_string())
    .bind(generation.get())
    .execute(&mut *transaction)
    .await?;
    if result.rows_affected() != 1 {
        return Err(errors::Error::conflict(
            "RelationDefinition generation changed",
        ));
    }
    if let Some(inverse) = owned_inverse.as_ref() {
        delete_property(&mut transaction, inverse).await?;
    }
    let source_projection = source_property.to_property()?;
    delete_property(&mut transaction, &source_property).await?;
    transaction.commit().await?;
    Ok(Some((definition, source_projection)))
}

/// Route the legacy Property delete surface through the Relation schema UoW
/// when the source Property owns a RelationDefinition.
pub(super) async fn delete_relation_property_if_present(
    repository: &PropertyRepositoryImpl,
    tenant_id: &TenantId,
    source_database_id: &crate::domain::DatabaseId,
    source_property_id: &PropertyId,
) -> errors::Result<Option<Property>> {
    Ok(delete_relation_schema(
        repository,
        tenant_id,
        source_database_id,
        source_property_id,
        None,
    )
    .await?
    .map(|(_, property)| property))
}

#[async_trait::async_trait]
impl RelationSchemaMutationPort for PropertyRepositoryImpl {
    async fn reconfigure_relation_atomically(
        &self,
        command: &ReconfigureRelationDefinitionCommand,
    ) -> errors::Result<RelationDefinition> {
        let target_database_id = relation_target(
            self,
            command.tenant_id(),
            command.source_database_id(),
            command.source_property_id(),
        )
        .await?
        .ok_or_else(|| errors::Error::not_found("resource not found"))?;
        let mut transaction = self.db.pool().begin().await?;
        lock_endpoints(
            &mut transaction,
            command.tenant_id(),
            command.source_database_id(),
            &target_database_id,
        )
        .await?;
        let current = load_locked_schema(
            self,
            &mut transaction,
            command.tenant_id(),
            command.source_database_id(),
            command.source_property_id(),
            &target_database_id,
        )
        .await?;
        let mutation = RelationSchema::plan_reconfiguration(
            &current.definition,
            &current.source_property,
            current.current_inverse.as_ref(),
            &current.target_properties,
            command,
        )?;
        let (definition, inverse_mutation) = mutation.into_parts();

        match &inverse_mutation {
            InversePropertyMutation::Insert(inverse) => {
                insert_property(&mut transaction, inverse).await?;
            }
            InversePropertyMutation::Replace(inverse) => {
                replace_property(&mut transaction, inverse).await?;
            }
            InversePropertyMutation::None
            | InversePropertyMutation::Delete(_) => {}
        }
        replace_relation_definition(
            &mut transaction,
            &definition,
            command.expected_generation(),
        )
        .await?;
        if let InversePropertyMutation::Delete(inverse) = &inverse_mutation
        {
            delete_property(&mut transaction, inverse).await?;
        }
        transaction.commit().await?;
        Ok(definition)
    }

    async fn delete_relation_atomically(
        &self,
        command: &DeleteRelationDefinitionCommand,
    ) -> errors::Result<RelationDefinition> {
        delete_relation_schema(
            self,
            command.tenant_id(),
            command.source_database_id(),
            command.source_property_id(),
            Some(command.expected_generation()),
        )
        .await?
        .map(|(definition, _)| definition)
        .ok_or_else(|| errors::Error::not_found("resource not found"))
    }
}
