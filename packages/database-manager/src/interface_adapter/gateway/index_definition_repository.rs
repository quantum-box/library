use std::sync::Arc;

use crate::domain::{
    DatabaseId, IndexDefinition, IndexDefinitionId,
    IndexDefinitionRepository, IndexDefinitionVersion, IndexGeneration,
    IndexPolicy, IndexProjectionState, IndexTarget, PropertyId, RelationId,
};
use persistence::Db;
use value_object::TenantId;

const INDEX_DEFINITION_COLUMNS: &str = r#"
    id, tenant_id, database_id, property_id, relation_id, policy,
    is_unique, definition_version, generation, projection_state
"#;

#[derive(Clone, Debug, sqlx::FromRow)]
struct IndexDefinitionRow {
    id: String,
    tenant_id: String,
    database_id: String,
    property_id: Option<String>,
    relation_id: Option<String>,
    policy: String,
    is_unique: bool,
    definition_version: u16,
    generation: u64,
    projection_state: String,
}

#[derive(Clone, Debug)]
pub struct IndexDefinitionRepositoryImpl {
    db: Arc<Db>,
}

impl IndexDefinitionRepositoryImpl {
    pub fn new(db: Arc<Db>) -> Arc<Self> {
        Arc::new(Self { db })
    }

    fn target_columns(
        target: &IndexTarget,
    ) -> (Option<String>, Option<String>) {
        match target {
            IndexTarget::Property(id) => (Some(id.to_string()), None),
            IndexTarget::Relation(id) => (None, Some(id.to_string())),
        }
    }

    fn restore_all(
        rows: Vec<IndexDefinitionRow>,
    ) -> errors::Result<Vec<IndexDefinition>> {
        rows.into_iter().map(IndexDefinition::try_from).collect()
    }

    fn map_insert_error(error: sqlx::Error) -> errors::Error {
        match error {
            sqlx::Error::Database(database_error)
                if database_error.is_unique_violation() =>
            {
                errors::Error::conflict(
                    "IndexDefinition already exists for target",
                )
            }
            error => error.into(),
        }
    }

    fn cas_conflict() -> errors::Error {
        errors::Error::conflict("IndexDefinition generation changed")
    }
}

#[async_trait::async_trait]
impl IndexDefinitionRepository for IndexDefinitionRepositoryImpl {
    async fn insert(
        &self,
        definition: &IndexDefinition,
    ) -> errors::Result<()> {
        definition.ensure_writable()?;
        let (property_id, relation_id) =
            Self::target_columns(definition.target());
        sqlx::query(
            r#"
            INSERT INTO index_definitions
                (id, tenant_id, database_id, property_id, relation_id,
                 policy, is_unique, definition_version, generation,
                 projection_state)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(definition.id().to_string())
        .bind(definition.tenant_id().to_string())
        .bind(definition.database_id().to_string())
        .bind(property_id)
        .bind(relation_id)
        .bind(definition.policy().to_string())
        .bind(*definition.unique())
        .bind(definition.definition_version().get())
        .bind(definition.generation().get())
        .bind(definition.projection_state().to_string())
        .execute(self.db.pool().as_ref())
        .await
        .map_err(Self::map_insert_error)?;
        Ok(())
    }

    async fn replace_if_generation(
        &self,
        definition: &IndexDefinition,
        expected_generation: IndexGeneration,
    ) -> errors::Result<()> {
        definition.ensure_writable()?;
        let expected_next = expected_generation.next()?;
        if definition.generation() != &expected_next {
            return Err(errors::Error::invalid(
                "replacement IndexDefinition must increment generation once",
            ));
        }
        let (property_id, relation_id) =
            Self::target_columns(definition.target());
        let result = sqlx::query(
            r#"
            UPDATE index_definitions
            SET policy = ?, is_unique = ?, definition_version = ?,
                generation = ?, projection_state = ?
            WHERE tenant_id = ? AND database_id = ? AND id = ?
              AND generation = ?
              AND property_id <=> ? AND relation_id <=> ?
            "#,
        )
        .bind(definition.policy().to_string())
        .bind(*definition.unique())
        .bind(definition.definition_version().get())
        .bind(definition.generation().get())
        .bind(definition.projection_state().to_string())
        .bind(definition.tenant_id().to_string())
        .bind(definition.database_id().to_string())
        .bind(definition.id().to_string())
        .bind(expected_generation.get())
        .bind(property_id)
        .bind(relation_id)
        .execute(self.db.pool().as_ref())
        .await?;
        if result.rows_affected() != 1 {
            return Err(Self::cas_conflict());
        }
        Ok(())
    }

    async fn transition_projection_if_generation(
        &self,
        definition: &IndexDefinition,
        expected_generation: IndexGeneration,
        expected_state: IndexProjectionState,
    ) -> errors::Result<()> {
        definition.ensure_writable()?;
        let expected_result_generation = if definition.projection_state()
            == &IndexProjectionState::Building
        {
            expected_generation.next()?
        } else {
            expected_generation
        };
        if definition.generation() != &expected_result_generation {
            return Err(errors::Error::invalid(
                "projection transition has an invalid generation",
            ));
        }
        let (property_id, relation_id) =
            Self::target_columns(definition.target());
        let result = sqlx::query(
            r#"
            UPDATE index_definitions
            SET generation = ?, projection_state = ?
            WHERE tenant_id = ? AND database_id = ? AND id = ?
              AND generation = ? AND projection_state = ?
              AND property_id <=> ? AND relation_id <=> ?
            "#,
        )
        .bind(definition.generation().get())
        .bind(definition.projection_state().to_string())
        .bind(definition.tenant_id().to_string())
        .bind(definition.database_id().to_string())
        .bind(definition.id().to_string())
        .bind(expected_generation.get())
        .bind(expected_state.to_string())
        .bind(property_id)
        .bind(relation_id)
        .execute(self.db.pool().as_ref())
        .await?;
        if result.rows_affected() != 1 {
            return Err(Self::cas_conflict());
        }
        Ok(())
    }

    async fn find_by_id(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        index_definition_id: &IndexDefinitionId,
    ) -> errors::Result<Option<IndexDefinition>> {
        let sql = format!(
            r#"
            SELECT {INDEX_DEFINITION_COLUMNS}
            FROM index_definitions
            WHERE tenant_id = ? AND database_id = ? AND id = ?
            LIMIT 1
            "#,
        );
        sqlx::query_as::<_, IndexDefinitionRow>(&sql)
            .bind(tenant_id.to_string())
            .bind(database_id.to_string())
            .bind(index_definition_id.to_string())
            .fetch_optional(self.db.pool().as_ref())
            .await?
            .map(IndexDefinition::try_from)
            .transpose()
    }

    async fn find_by_target(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        target: &IndexTarget,
    ) -> errors::Result<Option<IndexDefinition>> {
        let (column, id) = match target {
            IndexTarget::Property(id) => ("property_id", id.to_string()),
            IndexTarget::Relation(id) => ("relation_id", id.to_string()),
        };
        let sql = format!(
            r#"
            SELECT {INDEX_DEFINITION_COLUMNS}
            FROM index_definitions
            WHERE tenant_id = ? AND database_id = ? AND {column} = ?
            LIMIT 1
            "#,
        );
        sqlx::query_as::<_, IndexDefinitionRow>(&sql)
            .bind(tenant_id.to_string())
            .bind(database_id.to_string())
            .bind(id)
            .fetch_optional(self.db.pool().as_ref())
            .await?
            .map(IndexDefinition::try_from)
            .transpose()
    }

    async fn find_all_by_database(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
    ) -> errors::Result<Vec<IndexDefinition>> {
        let sql = format!(
            r#"
            SELECT {INDEX_DEFINITION_COLUMNS}
            FROM index_definitions
            WHERE tenant_id = ? AND database_id = ?
            ORDER BY id
            "#,
        );
        let rows = sqlx::query_as::<_, IndexDefinitionRow>(&sql)
            .bind(tenant_id.to_string())
            .bind(database_id.to_string())
            .fetch_all(self.db.pool().as_ref())
            .await?;
        Self::restore_all(rows)
    }
}

impl TryFrom<IndexDefinitionRow> for IndexDefinition {
    type Error = errors::Error;

    fn try_from(row: IndexDefinitionRow) -> errors::Result<Self> {
        let target = match (row.property_id, row.relation_id) {
            (Some(id), None) => {
                IndexTarget::Property(id.parse::<PropertyId>()?)
            }
            (None, Some(id)) => {
                IndexTarget::Relation(id.parse::<RelationId>()?)
            }
            _ => {
                return Err(errors::Error::invalid(
                    "stored IndexDefinition must have exactly one target",
                ));
            }
        };
        IndexDefinition::restore(
            &row.id.parse()?,
            &row.tenant_id.parse()?,
            &row.database_id.parse()?,
            target,
            row.policy.parse::<IndexPolicy>()?,
            row.is_unique,
            IndexDefinitionVersion::new(row.definition_version)?,
            IndexGeneration::new(row.generation)?,
            row.projection_state.parse::<IndexProjectionState>()?,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row() -> IndexDefinitionRow {
        IndexDefinitionRow {
            id: IndexDefinitionId::default().to_string(),
            tenant_id: TenantId::default().to_string(),
            database_id: DatabaseId::default().to_string(),
            property_id: Some(PropertyId::default().to_string()),
            relation_id: None,
            policy: "EXACT".to_string(),
            is_unique: true,
            definition_version: 1,
            generation: 3,
            projection_state: "READY".to_string(),
        }
    }

    #[test]
    fn restores_a_scoped_property_definition() {
        let definition =
            IndexDefinition::try_from(row()).expect("valid definition row");
        assert!(matches!(definition.target(), IndexTarget::Property(_)));
        assert_eq!(*definition.policy(), IndexPolicy::Exact);
        assert_eq!(definition.generation().get(), 3);
        assert_eq!(
            *definition.projection_state(),
            IndexProjectionState::Ready
        );
    }

    #[test]
    fn rejects_ambiguous_or_malformed_storage() {
        let mut ambiguous = row();
        ambiguous.relation_id = Some(RelationId::default().to_string());
        assert!(IndexDefinition::try_from(ambiguous).is_err());

        let mut malformed = row();
        malformed.projection_state = "MAGIC".to_string();
        assert!(IndexDefinition::try_from(malformed).is_err());
    }
}
