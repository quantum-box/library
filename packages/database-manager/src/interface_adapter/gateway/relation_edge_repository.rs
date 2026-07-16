use std::sync::Arc;

use crate::domain::{
    DataId, RecordReference, RelationDefinition, RelationEdge,
    RelationEdgeRepository, RelationEdgeSet,
};
use persistence::Db;
use value_object::TenantId;

#[derive(Clone, Debug)]
pub struct RelationEdgeRepositoryImpl {
    db: Arc<Db>,
}

#[derive(Debug, sqlx::FromRow)]
struct RelationEdgeRow {
    tenant_id: String,
    source_database_id: String,
    source_data_id: String,
    relation_id: String,
    target_database_id: String,
    target_data_id: String,
}

impl RelationEdgeRepositoryImpl {
    pub fn new(db: Arc<Db>) -> Arc<Self> {
        Arc::new(Self { db })
    }

    fn ensure_tenant_scope(
        tenant_id: &TenantId,
        definition: &RelationDefinition,
    ) -> errors::Result<()> {
        if tenant_id != definition.tenant_id() {
            return Err(errors::Error::invalid(
                "RelationEdge query tenant does not match RelationDefinition",
            ));
        }
        Ok(())
    }

    fn restore_set(
        definition: &RelationDefinition,
        rows: Vec<RelationEdgeRow>,
    ) -> errors::Result<RelationEdgeSet> {
        let edges = rows
            .into_iter()
            .map(|row| {
                RelationEdge::restore(
                    &row.tenant_id.parse()?,
                    &row.relation_id.parse()?,
                    RecordReference::new(
                        &row.source_database_id.parse()?,
                        &row.source_data_id.parse()?,
                    ),
                    RecordReference::new(
                        &row.target_database_id.parse()?,
                        &row.target_data_id.parse()?,
                    ),
                    definition,
                )
            })
            .collect::<errors::Result<Vec<_>>>()?;
        RelationEdgeSet::new(definition, edges)
    }
}

#[async_trait::async_trait]
impl RelationEdgeRepository for RelationEdgeRepositoryImpl {
    async fn find_forward(
        &self,
        tenant_id: &TenantId,
        definition: &RelationDefinition,
        source_data_id: &DataId,
    ) -> errors::Result<RelationEdgeSet> {
        Self::ensure_tenant_scope(tenant_id, definition)?;
        let rows = sqlx::query_as::<_, RelationEdgeRow>(
            r#"
            SELECT tenant_id, source_database_id, source_data_id,
                   relation_id, target_database_id, target_data_id
            FROM relation_edges
            WHERE tenant_id = ?
              AND source_database_id = ?
              AND source_data_id = ?
              AND relation_id = ?
              AND target_database_id = ?
            ORDER BY target_database_id, target_data_id
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(definition.source_database_id().to_string())
        .bind(source_data_id.to_string())
        .bind(definition.id().to_string())
        .bind(definition.target_database_id().to_string())
        .fetch_all(self.db.pool().as_ref())
        .await?;
        Self::restore_set(definition, rows)
    }

    async fn find_backlinks(
        &self,
        tenant_id: &TenantId,
        definition: &RelationDefinition,
        target_data_id: &DataId,
    ) -> errors::Result<RelationEdgeSet> {
        Self::ensure_tenant_scope(tenant_id, definition)?;
        let rows = sqlx::query_as::<_, RelationEdgeRow>(
            r#"
            SELECT tenant_id, source_database_id, source_data_id,
                   relation_id, target_database_id, target_data_id
            FROM relation_edges
            WHERE tenant_id = ?
              AND target_database_id = ?
              AND target_data_id = ?
              AND relation_id = ?
              AND source_database_id = ?
            ORDER BY source_database_id, source_data_id
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(definition.target_database_id().to_string())
        .bind(target_data_id.to_string())
        .bind(definition.id().to_string())
        .bind(definition.source_database_id().to_string())
        .fetch_all(self.db.pool().as_ref())
        .await?;
        Self::restore_set(definition, rows)
    }
}
