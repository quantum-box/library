use super::*;

#[derive(Clone, Debug)]
pub struct PropertyRepositoryImpl {
    pub db: Arc<Db>,
}

impl PropertyRepositoryImpl {
    pub fn new(db: Arc<Db>) -> Arc<Self> {
        Arc::new(Self { db })
    }
}

fn map_schema_insert_error(
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
                   datatype_meta, is_indexed, field_num, meta_json
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
        .map(Property::from)
        .collect::<Vec<_>>();

        let mutation =
            PropertySchema::plan_addition(&existing_properties, command)?;
        let (property, relation) = mutation.into_parts();

        let field_insert = sqlx::query(
            r#"
            INSERT INTO fields
                (id, tenant_id, object_id, field_name, datatype,
                 datatype_meta, is_indexed, field_num, meta_json)
            VALUES
                (?, ?, ?, ?, ?, ?, ?, ?, ?);
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
        .execute(&mut *transaction)
        .await;
        if let Err(error) = field_insert {
            return Err(map_schema_insert_error(
                error,
                property.property_type(),
            ));
        }

        if let Some(relation) = relation {
            sqlx::query(
                r#"
                INSERT INTO relationships
                    (id, tenant_id, object_id, field_id, relation_id,
                     target_object_id)
                VALUES
                    (?, ?, ?, ?, ?, ?);
                "#,
            )
            .bind(relation.id().to_string())
            .bind(relation.tenant_id().to_string())
            .bind(relation.database_id().to_string())
            .bind(relation.property_id().to_string())
            .bind(*relation.relation_id() as u32)
            .bind(relation.target_database_id().to_string())
            .execute(&mut *transaction)
            .await?;
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
                   datatype_meta, is_indexed, field_num, meta_json
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
        let field_num = row.field_num;
        let property: Property = row.into();

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
    async fn update(&self, property: &Property) -> errors::Result<()> {
        sqlx::query!(
            "
            UPDATE tachyon_apps_database_manager.fields
            SET field_name = ?, datatype = ?, is_indexed = ?, datatype_meta = ?, meta_json = ?
            WHERE tenant_id = ? AND object_id = ? AND id = ?;
            ",
            property.name(),
            property.property_type().to_string(),
            property.is_indexed(),
            property.property_type().get_meta()?,
            property.meta_json(),
            property.tenant_id().to_string(),
            property.database_id().to_string(),
            property.id().to_string(),
        )
        .execute(self.db.pool().as_ref())
        .await?;
        Ok(())
    }
    async fn find_by_id(
        &self,
        id: &PropertyId,
        database_id: &DatabaseId,
        tenant_id: &TenantId,
    ) -> errors::Result<Option<Property>> {
        Ok(sqlx::query_as::<_, FieldRow>(
            "SELECT * 
            FROM fields
            WHERE tenant_id = ? AND object_id = ? AND id = ? LIMIT 1;
            ",
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .bind(id.to_string())
        .fetch_optional(self.db.pool().as_ref())
        .await?
        .map(|row| row.into()))
    }
    async fn find_all(
        &self,
        database_id: &DatabaseId,
        tenant_id: &TenantId,
    ) -> errors::Result<Vec<Property>> {
        Ok(sqlx::query_as::<_, FieldRow>(
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
        .map(|row| row.into())
        .collect::<Vec<Property>>())
    }

    async fn delete_all(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
    ) -> errors::Result<()> {
        sqlx::query(
            "
            DELETE FROM fields
            WHERE tenant_id = ? AND object_id = ?;
            ",
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .execute(self.db.pool().as_ref())
        .await?;
        Ok(())
    }

    async fn delete(
        &self,
        tenant_id: &TenantId,
        id: &PropertyId,
    ) -> errors::Result<()> {
        sqlx::query(
            "
            DELETE FROM fields
            WHERE tenant_id = ? AND id = ?;
            ",
        )
        .bind(tenant_id.to_string())
        .bind(id.to_string())
        .execute(self.db.pool().as_ref())
        .await?;
        Ok(())
    }
}
