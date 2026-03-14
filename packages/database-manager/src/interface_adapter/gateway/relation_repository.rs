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
    async fn insert(&self, entity: &Relation) -> errors::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO relationships 
                (id, tenant_id, object_id, field_id, relation_id, target_object_id)
            VALUES
                (?, ?, ?, ?, ?, ?);
            "#,
        )
        .bind(entity.id().to_string())
        .bind(entity.tenant_id().to_string())
        .bind(entity.database_id().to_string())
        .bind(entity.property_id().to_string())
        .bind(*entity.relation_id() as u32)
        .bind(entity.target_database_id().to_string())
        .execute(self.db.pool().as_ref())
        .await?;
        Ok(())
    }

    async fn find_all_by_database(
        &self,
        database_id: &DatabaseId,
        tenant_id: &TenantId,
    ) -> errors::Result<Vec<Relation>> {
        Ok(sqlx::query_as::<_, RelationshipRow>(
            r#"
            SELECT id, tenant_id, object_id, field_id, relation_id, target_object_id
            FROM relationships
            WHERE tenant_id = ? AND _id = ?;
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .fetch_all(self.db.pool().as_ref())
        .await?
        .into_iter()
        .map(|row| row.into())
        .collect())
    }
}

impl From<RelationshipRow> for Relation {
    fn from(row: RelationshipRow) -> Self {
        Self::new(
            &row.id.parse().unwrap(),
            &row.tenant_id.parse().unwrap(),
            &row.object_id.parse().unwrap(),
            &row.field_id.parse().unwrap(),
            row.relation_id as usize,
            &row.target_object_id.parse().unwrap(),
        )
    }
}
