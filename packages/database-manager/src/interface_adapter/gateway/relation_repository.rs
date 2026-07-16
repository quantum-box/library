use std::sync::Arc;

use crate::domain::{
    DatabaseId, PropertyId, Relation, RelationCardinality,
    RelationDefinition, RelationDefinitionRepository,
    RelationDefinitionVersion, RelationGeneration, RelationOnDelete,
    RelationRepository,
};
use persistence::Db;
use value_object::TenantId;

use super::{RelationDefinitionRow, RelationshipRow};

const RELATION_DEFINITION_COLUMNS: &str = r#"
    id, tenant_id, object_id, field_id, target_object_id,
    forward_cardinality, reverse_cardinality, inverse_field_id,
    inverse_owned, on_target_delete, definition_version, generation
"#;

#[derive(Clone, Debug)]
pub struct RelationDefinitionRepositoryImpl {
    db: Arc<Db>,
}

/// Compatibility adapter name retained for existing constructors and callers.
pub type RelationRepositoryImpl = RelationDefinitionRepositoryImpl;

impl RelationDefinitionRepositoryImpl {
    pub fn new(db: Arc<Db>) -> Arc<Self> {
        Arc::new(Self { db })
    }

    fn restore_all(
        rows: Vec<RelationDefinitionRow>,
    ) -> errors::Result<Vec<RelationDefinition>> {
        rows.into_iter().map(RelationDefinition::try_from).collect()
    }
}

#[async_trait::async_trait]
impl RelationDefinitionRepository for RelationDefinitionRepositoryImpl {
    async fn find_by_id(
        &self,
        tenant_id: &TenantId,
        source_database_id: &DatabaseId,
        relation_id: &crate::domain::RelationId,
    ) -> errors::Result<Option<RelationDefinition>> {
        let sql = format!(
            r#"
            SELECT {RELATION_DEFINITION_COLUMNS}
            FROM relationships
            WHERE tenant_id = ? AND object_id = ? AND id = ?
            LIMIT 1;
            "#,
        );
        sqlx::query_as::<_, RelationDefinitionRow>(&sql)
            .bind(tenant_id.to_string())
            .bind(source_database_id.to_string())
            .bind(relation_id.to_string())
            .fetch_optional(self.db.pool().as_ref())
            .await?
            .map(RelationDefinition::try_from)
            .transpose()
    }

    async fn find_by_source_property(
        &self,
        tenant_id: &TenantId,
        source_database_id: &DatabaseId,
        source_property_id: &PropertyId,
    ) -> errors::Result<Option<RelationDefinition>> {
        let sql = format!(
            r#"
            SELECT {RELATION_DEFINITION_COLUMNS}
            FROM relationships
            WHERE tenant_id = ? AND object_id = ? AND field_id = ?
            LIMIT 1;
            "#,
        );
        sqlx::query_as::<_, RelationDefinitionRow>(&sql)
            .bind(tenant_id.to_string())
            .bind(source_database_id.to_string())
            .bind(source_property_id.to_string())
            .fetch_optional(self.db.pool().as_ref())
            .await?
            .map(RelationDefinition::try_from)
            .transpose()
    }

    async fn find_all_by_source_database(
        &self,
        tenant_id: &TenantId,
        source_database_id: &DatabaseId,
    ) -> errors::Result<Vec<RelationDefinition>> {
        let sql = format!(
            r#"
            SELECT {RELATION_DEFINITION_COLUMNS}
            FROM relationships
            WHERE tenant_id = ? AND object_id = ?
            ORDER BY id;
            "#,
        );
        let rows = sqlx::query_as::<_, RelationDefinitionRow>(&sql)
            .bind(tenant_id.to_string())
            .bind(source_database_id.to_string())
            .fetch_all(self.db.pool().as_ref())
            .await?;
        Self::restore_all(rows)
    }

    async fn find_all_by_target_database(
        &self,
        tenant_id: &TenantId,
        target_database_id: &DatabaseId,
    ) -> errors::Result<Vec<RelationDefinition>> {
        let sql = format!(
            r#"
            SELECT {RELATION_DEFINITION_COLUMNS}
            FROM relationships
            WHERE tenant_id = ? AND target_object_id = ?
            ORDER BY id;
            "#,
        );
        let rows = sqlx::query_as::<_, RelationDefinitionRow>(&sql)
            .bind(tenant_id.to_string())
            .bind(target_database_id.to_string())
            .fetch_all(self.db.pool().as_ref())
            .await?;
        Self::restore_all(rows)
    }
}

#[async_trait::async_trait]
impl RelationRepository for RelationDefinitionRepositoryImpl {
    async fn find_all_by_database(
        &self,
        database_id: &DatabaseId,
        tenant_id: &TenantId,
    ) -> errors::Result<Vec<Relation>> {
        let rows = sqlx::query_as::<_, RelationshipRow>(
            r#"
            SELECT id, tenant_id, object_id, field_id, relation_id,
                   target_object_id
            FROM relationships
            WHERE tenant_id = ? AND object_id = ?
            ORDER BY id;
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .fetch_all(self.db.pool().as_ref())
        .await?;

        rows.into_iter().map(Relation::try_from).collect()
    }
}

impl TryFrom<RelationDefinitionRow> for RelationDefinition {
    type Error = errors::Error;

    fn try_from(row: RelationDefinitionRow) -> errors::Result<Self> {
        let inverse_property_id = row
            .inverse_field_id
            .map(|id| id.parse::<PropertyId>())
            .transpose()?;
        Self::restore(
            &row.id.parse()?,
            &row.tenant_id.parse()?,
            &row.object_id.parse()?,
            &row.field_id.parse()?,
            &row.target_object_id.parse()?,
            row.forward_cardinality.parse::<RelationCardinality>()?,
            row.reverse_cardinality.parse::<RelationCardinality>()?,
            inverse_property_id.as_ref(),
            row.inverse_owned,
            row.on_target_delete.parse::<RelationOnDelete>()?,
            RelationDefinitionVersion::new(row.definition_version)?,
            RelationGeneration::new(row.generation)?,
        )
    }
}

impl TryFrom<RelationshipRow> for Relation {
    type Error = errors::Error;

    fn try_from(row: RelationshipRow) -> errors::Result<Self> {
        Ok(Self::new(
            &row.id.parse()?,
            &row.tenant_id.parse()?,
            &row.object_id.parse()?,
            &row.field_id.parse()?,
            row.relation_id as usize,
            &row.target_object_id.parse()?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::RelationId;

    fn relation_definition_row() -> RelationDefinitionRow {
        RelationDefinitionRow {
            id: RelationId::default().to_string(),
            tenant_id: TenantId::default().to_string(),
            object_id: DatabaseId::default().to_string(),
            field_id: PropertyId::default().to_string(),
            target_object_id: DatabaseId::default().to_string(),
            forward_cardinality: "ONE".to_string(),
            reverse_cardinality: "MANY".to_string(),
            inverse_field_id: Some(PropertyId::default().to_string()),
            inverse_owned: true,
            on_target_delete: "NULLIFY".to_string(),
            definition_version: 1,
            generation: 3,
        }
    }

    #[test]
    fn restores_the_canonical_definition_shape() {
        let definition =
            RelationDefinition::try_from(relation_definition_row())
                .expect("valid Relation definition row");

        assert_eq!(
            *definition.forward_cardinality(),
            RelationCardinality::One
        );
        assert_eq!(
            *definition.reverse_cardinality(),
            RelationCardinality::Many
        );
        assert_eq!(
            *definition.on_target_delete(),
            RelationOnDelete::Nullify
        );
        assert!(definition.inverse_property_id().is_some());
        assert!(*definition.inverse_property_owned());
        assert_eq!(definition.generation().get(), 3);
    }

    #[test]
    fn malformed_definition_policy_returns_a_domain_error() {
        let mut row = relation_definition_row();
        row.on_target_delete = "DELETE_EVERYTHING".to_string();

        let error = RelationDefinition::try_from(row)
            .expect_err("unknown lifecycle policy must not be restored");

        assert!(error.is_bad_request());
    }

    #[test]
    fn malformed_legacy_projection_returns_a_domain_error() {
        let row = RelationshipRow {
            id: RelationId::default().to_string(),
            tenant_id: TenantId::default().to_string(),
            object_id: DatabaseId::default().to_string(),
            field_id: PropertyId::default().to_string(),
            relation_id: 0,
            target_object_id: "not-a-database-id".to_string(),
        };

        let error = Relation::try_from(row)
            .expect_err("malformed stored ids must not panic");

        assert!(error.is_bad_request());
    }
}
