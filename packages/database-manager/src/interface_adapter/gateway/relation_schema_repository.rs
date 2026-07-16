use sqlx::{MySql, Transaction};

use super::{
    encoded_definition, map_schema_insert_error, FieldRow,
    PropertyRepositoryImpl, PropertyValueRow, RelationDefinitionRow,
};
use crate::domain::{
    DataId, DeleteRelationDefinitionCommand, InversePropertyMutation,
    Property, PropertyData, PropertyDataValue, PropertyDefinition,
    PropertyId, PropertyValue, ReconfigureRelationDefinitionCommand,
    RecordReference, RelationDefinition, RelationEdge, RelationEdgeSet,
    RelationGeneration, RelationSchema, RelationSchemaMutationPort,
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

#[derive(sqlx::FromRow)]
struct RelationEdgeScopeRow {
    source_data_id: String,
    target_data_id: String,
}

#[derive(sqlx::FromRow)]
struct LegacyRelationValueRow {
    data_id: String,
    legacy_value: Option<String>,
}

async fn lock_relation_index_definitions(
    transaction: &mut Transaction<'_, MySql>,
    definition: &RelationDefinition,
) -> errors::Result<()> {
    sqlx::query_scalar::<_, String>(
        r#"
        SELECT id
        FROM index_definitions
        WHERE tenant_id = ? AND database_id = ? AND relation_id = ?
        ORDER BY id
        FOR UPDATE
        "#,
    )
    .bind(definition.tenant_id().to_string())
    .bind(definition.source_database_id().to_string())
    .bind(definition.id().to_string())
    .fetch_all(&mut **transaction)
    .await?;
    Ok(())
}

async fn lock_and_validate_relation_edges(
    transaction: &mut Transaction<'_, MySql>,
    current: &RelationDefinition,
    planned: &RelationDefinition,
) -> errors::Result<RelationEdgeSet> {
    let rows = sqlx::query_as::<_, RelationEdgeScopeRow>(
        r#"
        SELECT source_data_id, target_data_id
        FROM relation_edges
        WHERE tenant_id = ?
          AND source_database_id = ?
          AND relation_id = ?
          AND target_database_id = ?
        ORDER BY source_data_id, target_data_id
        FOR UPDATE
        "#,
    )
    .bind(current.tenant_id().to_string())
    .bind(current.source_database_id().to_string())
    .bind(current.id().to_string())
    .bind(current.target_database_id().to_string())
    .fetch_all(&mut **transaction)
    .await?;

    let edges = rows
        .into_iter()
        .map(|row| {
            let source_data_id = row.source_data_id.parse::<DataId>()?;
            let target_data_id = row.target_data_id.parse::<DataId>()?;
            RelationEdge::restore(
                current.tenant_id(),
                current.id(),
                RecordReference::new(
                    current.source_database_id(),
                    &source_data_id,
                ),
                RecordReference::new(
                    current.target_database_id(),
                    &target_data_id,
                ),
                current,
            )
        })
        .collect::<errors::Result<Vec<_>>>()?;

    // Rebuild the complete persisted set against the candidate definition.
    // This makes both forward and reverse Many -> One narrowing fail before
    // generation or inverse Property mutations can be persisted.
    RelationEdgeSet::new(planned, edges)
}

fn narrows_cardinality(
    current: &RelationDefinition,
    planned: &RelationDefinition,
) -> bool {
    (*current.forward_cardinality()
        == crate::domain::RelationCardinality::Many
        && *planned.forward_cardinality()
            == crate::domain::RelationCardinality::One)
        || (*current.reverse_cardinality()
            == crate::domain::RelationCardinality::Many
            && *planned.reverse_cardinality()
                == crate::domain::RelationCardinality::One)
}

fn relation_targets(
    value: Option<&PropertyDataValue>,
) -> errors::Result<Option<Vec<DataId>>> {
    match value {
        None => Ok(None),
        Some(PropertyDataValue::Relation(_, target_ids)) => {
            Ok(Some(target_ids.clone()))
        }
        Some(_) => Err(errors::Error::conflict(
            "Relation Property storage decoded as a different Property type",
        )),
    }
}

async fn lock_and_validate_relation_values(
    transaction: &mut Transaction<'_, MySql>,
    source_property: &PropertyDefinition,
    planned: &RelationDefinition,
    persisted_edges: &RelationEdgeSet,
) -> errors::Result<()> {
    let property = source_property.to_property()?;
    let legacy_sql = format!(
        r#"
        SELECT id AS data_id, value{} AS legacy_value
        FROM data
        WHERE tenant_id = ? AND object_id = ?
        ORDER BY id
        FOR UPDATE
        "#,
        source_property.property_num(),
    );
    let legacy_rows =
        sqlx::query_as::<_, LegacyRelationValueRow>(&legacy_sql)
            .bind(planned.tenant_id().to_string())
            .bind(planned.source_database_id().to_string())
            .fetch_all(&mut **transaction)
            .await?;

    let mut legacy_targets = std::collections::BTreeMap::new();
    let mut legacy_edges = Vec::new();
    for row in legacy_rows {
        let source_data_id = row.data_id.parse::<DataId>()?;
        let targets = row
            .legacy_value
            .map(|value| PropertyData::from_storage(&property, value))
            .transpose()?
            .map(|value| relation_targets(value.value().as_ref()))
            .transpose()?
            .flatten();
        if let Some(targets) = &targets {
            for target_data_id in targets {
                legacy_edges.push(RelationEdge::new(
                    planned,
                    RecordReference::new(
                        planned.source_database_id(),
                        &source_data_id,
                    ),
                    RecordReference::new(
                        planned.target_database_id(),
                        target_data_id,
                    ),
                )?);
            }
        }
        legacy_targets.insert(row.data_id, targets);
    }
    let legacy_set = RelationEdgeSet::new(planned, legacy_edges)?;

    let legacy_identities = legacy_set
        .edges()
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    if persisted_edges
        .edges()
        .iter()
        .any(|edge| !legacy_identities.contains(edge))
    {
        return Err(errors::Error::conflict(
            "RelationEdge projection contains an edge absent from legacy Relation storage",
        ));
    }

    let canonical_rows = sqlx::query_as::<_, PropertyValueRow>(
        r#"
        SELECT tenant_id, database_id, data_id, property_id, type_key,
               type_version, value_encoding_version, value
        FROM property_values
        WHERE tenant_id = ? AND database_id = ? AND property_id = ?
        ORDER BY data_id
        FOR UPDATE
        "#,
    )
    .bind(planned.tenant_id().to_string())
    .bind(planned.source_database_id().to_string())
    .bind(source_property.id().to_string())
    .fetch_all(&mut **transaction)
    .await?;

    let mut canonical_edges = Vec::new();
    for row in canonical_rows {
        let value = PropertyData::from_definition_envelope(
            source_property,
            row.envelope()?,
        )?;
        if !matches!(value.envelope(), Some(PropertyValue::Known(_))) {
            return Err(errors::Error::conflict(
                "Relation cardinality cannot narrow while a canonical value is opaque",
            ));
        }
        let canonical_targets = relation_targets(value.value().as_ref())?
            .ok_or_else(|| {
            errors::Error::conflict(
                "canonical Relation value is unexpectedly absent",
            )
        })?;
        let Some(Some(expected_targets)) = legacy_targets.get(&row.data_id)
        else {
            return Err(errors::Error::conflict(
                "canonical Relation value has no matching legacy value",
            ));
        };
        let mut expected_targets = expected_targets.clone();
        expected_targets.sort();
        let mut canonical_targets_sorted = canonical_targets.clone();
        canonical_targets_sorted.sort();
        if expected_targets != canonical_targets_sorted {
            return Err(errors::Error::conflict(
                "legacy and canonical Relation values do not match",
            ));
        }

        let source_data_id = row.data_id.parse::<DataId>()?;
        for target_data_id in canonical_targets {
            canonical_edges.push(RelationEdge::new(
                planned,
                RecordReference::new(
                    planned.source_database_id(),
                    &source_data_id,
                ),
                RecordReference::new(
                    planned.target_database_id(),
                    &target_data_id,
                ),
            )?);
        }
    }
    RelationEdgeSet::new(planned, canonical_edges)?;
    Ok(())
}

async fn delete_relation_edges(
    transaction: &mut Transaction<'_, MySql>,
    definition: &RelationDefinition,
) -> errors::Result<()> {
    sqlx::query(
        r#"
        DELETE FROM relation_edges
        WHERE tenant_id = ?
          AND source_database_id = ?
          AND relation_id = ?
          AND target_database_id = ?
        "#,
    )
    .bind(definition.tenant_id().to_string())
    .bind(definition.source_database_id().to_string())
    .bind(definition.id().to_string())
    .bind(definition.target_database_id().to_string())
    .execute(&mut **transaction)
    .await?;
    Ok(())
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
    lock_relation_index_definitions(&mut transaction, &definition).await?;
    delete_relation_edges(&mut transaction, &definition).await?;
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

        lock_relation_index_definitions(&mut transaction, &definition)
            .await?;
        let persisted_edges = lock_and_validate_relation_edges(
            &mut transaction,
            &current.definition,
            &definition,
        )
        .await?;
        if narrows_cardinality(&current.definition, &definition) {
            lock_and_validate_relation_values(
                &mut transaction,
                &current.source_property,
                &definition,
                &persisted_edges,
            )
            .await?;
        }

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
