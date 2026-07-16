use super::*;

#[derive(Clone, Debug)]
pub struct PropertyRepositoryImpl {
    pub db: Arc<Db>,
    pub definition_mode:
        crate::property_definition_rollout::PropertyDefinitionStorageMode,
}

impl PropertyRepositoryImpl {
    pub fn new(db: Arc<Db>) -> Arc<Self> {
        Self::new_with_definition_mode(db, Default::default())
    }

    pub fn new_with_definition_mode(
        db: Arc<Db>,
        definition_mode: crate::property_definition_rollout::PropertyDefinitionStorageMode,
    ) -> Arc<Self> {
        Arc::new(Self {
            db,
            definition_mode,
        })
    }
}

pub(super) fn encoded_definition(
    definition: &PropertyDefinition,
) -> errors::Result<(Property, String, u16, String)> {
    definition.config().ensure_writable()?;
    let property = definition.to_property()?;
    Ok((
        property,
        definition.type_ref().key.as_str().to_string(),
        definition.type_ref().version.get(),
        serde_json::to_string(&definition.raw_config()?)
            .map_err(errors::Error::invalid)?,
    ))
}

pub(super) fn map_schema_insert_error(
    error: sqlx::Error,
    property_type: &PropertyType,
) -> errors::Error {
    if let sqlx::Error::Database(database_error) = &error {
        if database_error.is_unique_violation() {
            let message = if matches!(property_type, PropertyType::Id(_)) {
                ID_PROPERTY_ALREADY_EXISTS
            } else {
                "Property slot already exists"
            };
            return errors::Error::conflict(message);
        }
    }

    error.into()
}

#[async_trait::async_trait]
impl PropertySchemaMutationPort for PropertyRepositoryImpl {
    async fn add_property_atomically(
        &self,
        command: &AddPropertyCommand,
    ) -> errors::Result<Property> {
        let pool = self.db.pool();
        let mut transaction = pool.begin().await?;

        // Database rows are the serialization boundary for schema changes.
        // Relation writers lock both endpoints in the same primary-key order
        // so opposite A -> B and B -> A additions cannot deadlock while the
        // relationship foreign keys are checked.
        let source_database_id = command.database_id().to_string();
        let target_database_id = match command.property_type() {
            PropertyType::Relation(relation) => {
                relation.database_id.to_string()
            }
            _ => source_database_id.clone(),
        };
        let mut endpoint_ids =
            [source_database_id.clone(), target_database_id];
        endpoint_ids.sort();
        let locked_database_ids = sqlx::query_scalar::<_, String>(
            r#"
            SELECT id
            FROM objects
            WHERE tenant_id = ? AND id IN (?, ?)
            ORDER BY id
            FOR UPDATE;
            "#,
        )
        .bind(command.tenant_id().to_string())
        .bind(&endpoint_ids[0])
        .bind(&endpoint_ids[1])
        .fetch_all(&mut *transaction)
        .await?;
        if !locked_database_ids.contains(&source_database_id) {
            return Err(errors::Error::not_found("resource not found"));
        }

        // Every writer reads fields only after acquiring the endpoint locks,
        // so the domain evaluates slot and singleton invariants against the
        // latest schema.

        let existing_properties = sqlx::query_as::<_, FieldRow>(
            r#"
            SELECT id, tenant_id, object_id, field_name, datatype,
                   datatype_meta, is_indexed, field_num, meta_json,
                   type_key, type_version, type_config
            FROM fields
            WHERE tenant_id = ? AND object_id = ?
            FOR UPDATE;
            "#,
        )
        .bind(command.tenant_id().to_string())
        .bind(command.database_id().to_string())
        .fetch_all(&mut *transaction)
        .await?
        .into_iter()
        .map(|row| row.definition(self.definition_mode))
        .collect::<errors::Result<Vec<_>>>()?;

        let mutation =
            PropertySchema::plan_addition(&existing_properties, command)?;
        let (definition, relation_definition) = mutation.into_parts();
        let (property, type_key, type_version, type_config) =
            encoded_definition(&definition)?;

        let field_insert = sqlx::query(
            r#"
            INSERT INTO fields
                (id, tenant_id, object_id, field_name, datatype,
                 datatype_meta, is_indexed, field_num, meta_json,
                 type_key, type_version, type_config)
            VALUES
                (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);
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
        .execute(&mut *transaction)
        .await;
        if let Err(error) = field_insert {
            return Err(map_schema_insert_error(
                error,
                property.property_type(),
            ));
        }

        if let Some(definition) = relation_definition {
            sqlx::query(
                r#"
                INSERT INTO relationships
                    (id, tenant_id, object_id, field_id, relation_id,
                     target_object_id, forward_cardinality,
                     reverse_cardinality, inverse_field_id,
                     inverse_owned, on_target_delete,
                     definition_version, generation)
                VALUES
                    (?, ?, ?, ?, 0, ?, ?, ?, ?, ?, ?, ?, ?);
                "#,
            )
            .bind(definition.id().to_string())
            .bind(definition.tenant_id().to_string())
            .bind(definition.source_database_id().to_string())
            .bind(definition.source_property_id().to_string())
            .bind(definition.target_database_id().to_string())
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
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;
        Ok(property)
    }

    async fn update_property_atomically(
        &self,
        command: &UpdatePropertyCommand,
    ) -> errors::Result<Property> {
        let mut transaction = self.db.pool().begin().await?;
        let database = sqlx::query_scalar::<_, String>(
            r#"
            SELECT id FROM objects
            WHERE tenant_id = ? AND id = ?
            FOR UPDATE
            "#,
        )
        .bind(command.tenant_id().to_string())
        .bind(command.database_id().to_string())
        .fetch_optional(&mut *transaction)
        .await?;
        if database.is_none() {
            return Err(errors::Error::not_found("resource not found"));
        }

        let row = sqlx::query_as::<_, FieldRow>(
            r#"
            SELECT id, tenant_id, object_id, field_name, datatype,
                   datatype_meta, is_indexed, field_num, meta_json,
                   type_key, type_version, type_config
            FROM fields
            WHERE tenant_id = ? AND object_id = ? AND id = ?
            FOR UPDATE
            "#,
        )
        .bind(command.tenant_id().to_string())
        .bind(command.database_id().to_string())
        .bind(command.property_id().to_string())
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or_else(|| errors::Error::not_found("resource not found"))?;

        let generated_inverse_owner = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM relationships
                WHERE tenant_id = ? AND target_object_id = ?
                  AND inverse_field_id = ? AND inverse_owned = TRUE
            )
            "#,
        )
        .bind(command.tenant_id().to_string())
        .bind(command.database_id().to_string())
        .bind(command.property_id().to_string())
        .fetch_one(&mut *transaction)
        .await?;
        if generated_inverse_owner {
            return Err(errors::Error::conflict(
                "generated inverse Properties are read-only; mutate their RelationDefinition",
            ));
        }

        let relation_target = sqlx::query_scalar::<_, String>(
            r#"
            SELECT target_object_id
            FROM relationships
            WHERE tenant_id = ? AND object_id = ? AND field_id = ?
            LIMIT 1
            "#,
        )
        .bind(command.tenant_id().to_string())
        .bind(command.database_id().to_string())
        .bind(command.property_id().to_string())
        .fetch_optional(&mut *transaction)
        .await?;
        if relation_target.is_some() && command.property_type().is_some() {
            return Err(errors::Error::conflict(
                "Relation source type/config is owned by its RelationDefinition",
            ));
        }

        // Even in legacy-read mode, an unknown canonical envelope is owned by
        // a newer binary and must never be overwritten by this writer.
        let current = if let Some(target_database_id) = relation_target {
            let definition =
                row.definition_for_schema_write(self.definition_mode)?;
            let property = definition.to_property()?;
            let PropertyType::Relation(relation) = property.property_type()
            else {
                return Err(errors::Error::conflict(
                    "RelationDefinition source is not a Relation Property",
                ));
            };
            if relation.database_id.as_str() != target_database_id {
                return Err(errors::Error::conflict(
                    "RelationDefinition target does not match its source Property",
                ));
            }
            definition
        } else {
            row.ensure_canonical_definition_writable()?;
            row.definition(self.definition_mode)?
        };
        let updated = command.apply(&current)?;
        let (property, type_key, type_version, type_config) =
            encoded_definition(&updated)?;

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
        .execute(&mut *transaction)
        .await?;
        if result.rows_affected() == 0 {
            return Err(errors::Error::not_found("resource not found"));
        }
        transaction.commit().await?;
        Ok(property)
    }

    async fn delete_property_atomically(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        property_id: &PropertyId,
    ) -> errors::Result<Property> {
        if let Some(property) =
            super::relation_schema_repository::delete_relation_property_if_present(
                self,
                tenant_id,
                database_id,
                property_id,
            )
            .await?
        {
            return Ok(property);
        }

        let mut transaction = self.db.pool().begin().await?;
        let database = sqlx::query_scalar::<_, String>(
            r#"
            SELECT id FROM objects
            WHERE tenant_id = ? AND id = ?
            FOR UPDATE
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .fetch_optional(&mut *transaction)
        .await?;
        if database.is_none() {
            return Err(errors::Error::not_found("resource not found"));
        }
        let row = sqlx::query_as::<_, FieldRow>(
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
        .bind(database_id.to_string())
        .bind(property_id.to_string())
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or_else(|| errors::Error::not_found("resource not found"))?;
        let generated_inverse_owner = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM relationships
                WHERE tenant_id = ? AND target_object_id = ?
                  AND inverse_field_id = ? AND inverse_owned = TRUE
            )
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .bind(property_id.to_string())
        .fetch_one(&mut *transaction)
        .await?;
        if generated_inverse_owner {
            return Err(errors::Error::conflict(
                "generated inverse Properties are read-only; mutate their RelationDefinition",
            ));
        }
        row.ensure_canonical_definition_writable()?;
        let field_num = row.field_num;
        let property =
            row.definition(self.definition_mode)?.to_property()?;

        sqlx::query(
            r#"
            DELETE FROM relationships
            WHERE tenant_id = ? AND object_id = ? AND field_id = ?
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .bind(property_id.to_string())
        .execute(&mut *transaction)
        .await?;
        sqlx::query(&format!(
            "UPDATE data SET value{field_num} = NULL \
             WHERE tenant_id = ? AND object_id = ?"
        ))
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            r#"
            DELETE FROM fields
            WHERE tenant_id = ? AND object_id = ? AND id = ?
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .bind(property_id.to_string())
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(property)
    }
}

#[async_trait::async_trait]
impl PropertyRepository for PropertyRepositoryImpl {
    async fn find_by_id(
        &self,
        id: &PropertyId,
        database_id: &DatabaseId,
        tenant_id: &TenantId,
    ) -> errors::Result<Option<Property>> {
        let row = sqlx::query_as::<_, FieldRow>(
            "SELECT * 
            FROM fields
            WHERE tenant_id = ? AND object_id = ? AND id = ? LIMIT 1;
            ",
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .bind(id.to_string())
        .fetch_optional(self.db.pool().as_ref())
        .await?;
        row.map(|row| row.definition(self.definition_mode)?.to_property())
            .transpose()
    }
    async fn find_all(
        &self,
        database_id: &DatabaseId,
        tenant_id: &TenantId,
    ) -> errors::Result<Vec<Property>> {
        sqlx::query_as::<_, FieldRow>(
            "SELECT * 
            FROM fields
            WHERE tenant_id = ? AND object_id = ?;
            ",
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .fetch_all(self.db.pool().as_ref())
        .await?
        .into_iter()
        .map(|row| row.definition(self.definition_mode)?.to_property())
        .collect()
    }

    async fn delete_all(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
    ) -> errors::Result<()> {
        let mut transaction = self.db.pool().begin().await?;
        let database = sqlx::query_scalar::<_, String>(
            r#"
            SELECT id FROM objects
            WHERE tenant_id = ? AND id = ?
            FOR UPDATE
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .fetch_optional(&mut *transaction)
        .await?;
        if database.is_none() {
            return Err(errors::Error::not_found("resource not found"));
        }
        let owned_external_inverse = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM relationships
                WHERE tenant_id = ? AND inverse_owned = TRUE
                  AND (
                    (object_id = ? AND target_object_id <> ?)
                    OR (target_object_id = ? AND object_id <> ?)
                  )
            )
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .bind(database_id.to_string())
        .bind(database_id.to_string())
        .bind(database_id.to_string())
        .fetch_one(&mut *transaction)
        .await?;
        if owned_external_inverse {
            return Err(errors::Error::conflict(
                "bulk Property delete would orphan an owned inverse; delete Relation schemas first",
            ));
        }
        sqlx::query(
            r#"
            DELETE FROM relationships
            WHERE tenant_id = ? AND object_id = ?
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "
            DELETE FROM fields
            WHERE tenant_id = ? AND object_id = ?;
            ",
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn delete(
        &self,
        tenant_id: &TenantId,
        id: &PropertyId,
    ) -> errors::Result<()> {
        let database_id = sqlx::query_scalar::<_, String>(
            r#"
            SELECT object_id
            FROM fields
            WHERE tenant_id = ? AND id = ?
            LIMIT 1
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(id.to_string())
        .fetch_optional(self.db.pool().as_ref())
        .await?
        .ok_or_else(|| errors::Error::not_found("resource not found"))?
        .parse::<DatabaseId>()?;
        self.delete_property_atomically(tenant_id, &database_id, id)
            .await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl PropertyDefinitionRepository for PropertyRepositoryImpl {
    async fn find_canonical_definition_by_id(
        &self,
        id: &PropertyId,
        database_id: &DatabaseId,
        tenant_id: &TenantId,
    ) -> errors::Result<Option<PropertyDefinition>> {
        let row = sqlx::query_as::<_, FieldRow>(
            "SELECT * FROM fields
             WHERE tenant_id = ? AND object_id = ? AND id = ? LIMIT 1",
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .bind(id.to_string())
        .fetch_optional(self.db.pool().as_ref())
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let Some(canonical) = row.canonical_definition()? else {
            return Ok(None);
        };

        // Capability-driven consumers must not make schema decisions from an
        // envelope that this binary cannot safely write. During dual-write,
        // parity is also a precondition: accepting either side of a mismatch
        // would make index behavior depend on which read path a caller uses.
        canonical.config().ensure_writable()?;
        let legacy = row.legacy_definition()?;
        if canonical.type_ref() != legacy.type_ref()
            || canonical.raw_config()? != legacy.raw_config()?
        {
            return Err(errors::Error::invalid(
                "canonical PropertyDefinition does not match legacy projection",
            ));
        }

        Ok(Some(canonical))
    }

    async fn find_definition_by_id(
        &self,
        id: &PropertyId,
        database_id: &DatabaseId,
        tenant_id: &TenantId,
    ) -> errors::Result<Option<PropertyDefinition>> {
        let row = sqlx::query_as::<_, FieldRow>(
            "SELECT * FROM fields
             WHERE tenant_id = ? AND object_id = ? AND id = ? LIMIT 1",
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .bind(id.to_string())
        .fetch_optional(self.db.pool().as_ref())
        .await?;
        row.map(|row| row.definition(self.definition_mode))
            .transpose()
    }

    async fn find_all_definitions(
        &self,
        database_id: &DatabaseId,
        tenant_id: &TenantId,
    ) -> errors::Result<Vec<PropertyDefinition>> {
        sqlx::query_as::<_, FieldRow>(
            "SELECT * FROM fields
             WHERE tenant_id = ? AND object_id = ? ORDER BY field_num ASC",
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .fetch_all(self.db.pool().as_ref())
        .await?
        .into_iter()
        .map(|row| row.definition(self.definition_mode))
        .collect()
    }
}
