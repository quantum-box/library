use std::sync::Arc;

use crate::domain::{DatabaseId, Relation, RelationRepository};
use persistence::Db;
use value_object::TenantId;

use super::RelationshipRow;

#[derive(Clone, Debug)]
pub struct RelationRepositoryImpl {
    db: Arc<Db>,
}

impl RelationRepositoryImpl {
    pub fn new(db: Arc<Db>) -> Arc<Self> {
        Arc::new(Self { db })
    }
}

#[async_trait::async_trait]
impl RelationRepository for RelationRepositoryImpl {
    async fn find_all_by_database(
        &self,
        database_id: &DatabaseId,
        tenant_id: &TenantId,
    ) -> errors::Result<Vec<Relation>> {
        let rows = sqlx::query_as::<_, RelationshipRow>(
            r#"
            SELECT id, tenant_id, object_id, field_id, relation_id, target_object_id
            FROM relationships
            WHERE tenant_id = ? AND object_id = ?;
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .fetch_all(self.db.pool().as_ref())
        .await?;

        rows.into_iter().map(Relation::try_from).collect()
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

    fn relationship_row() -> RelationshipRow {
        RelationshipRow {
            id: crate::domain::RelationId::default().to_string(),
            tenant_id: TenantId::default().to_string(),
            object_id: DatabaseId::default().to_string(),
            field_id: crate::domain::PropertyId::default().to_string(),
            relation_id: 0,
            target_object_id: DatabaseId::default().to_string(),
        }
    }

    #[test]
    fn malformed_relationship_row_returns_a_domain_error() {
        let mut row = relationship_row();
        row.target_object_id = "not-a-database-id".to_string();

        let error = Relation::try_from(row)
            .expect_err("malformed stored ids must not panic");

        assert!(error.is_bad_request());
    }
}
