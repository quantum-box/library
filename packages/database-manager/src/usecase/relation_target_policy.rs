use std::{fmt::Debug, sync::Arc};

use crate::domain::{
    DataRepository, Database, DatabaseId, PropertyData, PropertyDataValue,
};
use crate::usecase::database_scope::DatabaseScope;
use value_object::{RepositoryV1, TenantId};

/// Application boundary for validating Relation values before persistence.
#[async_trait::async_trait]
pub trait RelationTargetValidationPort:
    Debug + Send + Sync + 'static
{
    async fn validate(
        &self,
        tenant_id: &TenantId,
        property_data: &PropertyData,
    ) -> errors::Result<()>;
}

/// Enforces that Relation targets stay inside the caller's tenant and point
/// to records in the configured target database.
#[derive(Debug, Clone)]
pub struct RelationTargetPolicy {
    database_repo: Arc<dyn RepositoryV1<DatabaseId, Database>>,
    data_repo: Arc<dyn DataRepository>,
}

impl RelationTargetPolicy {
    pub fn new(
        database_repo: Arc<dyn RepositoryV1<DatabaseId, Database>>,
        data_repo: Arc<dyn DataRepository>,
    ) -> Arc<Self> {
        Arc::new(Self {
            database_repo,
            data_repo,
        })
    }
}

#[async_trait::async_trait]
impl RelationTargetValidationPort for RelationTargetPolicy {
    async fn validate(
        &self,
        tenant_id: &TenantId,
        property_data: &PropertyData,
    ) -> errors::Result<()> {
        let Some(PropertyDataValue::Relation(
            target_database_id,
            target_data_ids,
        )) = property_data.value()
        else {
            return Ok(());
        };

        let target_scope =
            DatabaseScope::new(tenant_id, target_database_id);
        target_scope
            .require_database(self.database_repo.as_ref())
            .await?;

        for target_data_id in target_data_ids {
            target_scope
                .require_data(self.data_repo.as_ref(), target_data_id)
                .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        Data, DataCollection, DataId, Property, PropertyId, PropertyType,
        TypeRelation,
    };
    use chrono::Utc;
    use value_object::OffsetPaginator;

    #[derive(Debug)]
    struct StubDatabaseRepository {
        databases: Vec<Database>,
    }

    #[async_trait::async_trait]
    impl RepositoryV1<DatabaseId, Database> for StubDatabaseRepository {
        async fn save(&self, _entity: &Database) -> errors::Result<()> {
            unreachable!("not used by RelationTargetPolicy")
        }

        async fn delete(
            &self,
            _tenant_id: &TenantId,
            _id: &DatabaseId,
        ) -> errors::Result<()> {
            unreachable!("not used by RelationTargetPolicy")
        }

        async fn get_by_id(
            &self,
            tenant_id: &TenantId,
            id: &DatabaseId,
        ) -> errors::Result<Option<Database>> {
            Ok(self
                .databases
                .iter()
                .find(|database| {
                    database.tenant_id() == tenant_id && database.id() == id
                })
                .cloned())
        }

        async fn find_all(
            &self,
            _tenant_id: &TenantId,
        ) -> errors::Result<Vec<Database>> {
            unreachable!("not used by RelationTargetPolicy")
        }
    }

    #[derive(Debug)]
    struct StubDataRepository {
        data: Vec<Data>,
    }

    #[async_trait::async_trait]
    impl DataRepository for StubDataRepository {
        async fn create(&self, _data: &Data) -> errors::Result<()> {
            unreachable!("not used by RelationTargetPolicy")
        }

        async fn update(&self, _data: &Data) -> errors::Result<()> {
            unreachable!("not used by RelationTargetPolicy")
        }

        async fn update_all(
            &self,
            _data: &DataCollection,
        ) -> errors::Result<()> {
            unreachable!("not used by RelationTargetPolicy")
        }

        async fn find_by_id(
            &self,
            id: &DataId,
            database_id: &DatabaseId,
            tenant_id: &TenantId,
        ) -> errors::Result<Option<Data>> {
            Ok(self
                .data
                .iter()
                .find(|data| {
                    data.id() == id
                        && data.database_id() == database_id
                        && data.tenant_id() == tenant_id
                })
                .cloned())
        }

        async fn find_all(
            &self,
            _id: &DatabaseId,
            _tenant_id: &TenantId,
        ) -> errors::Result<DataCollection> {
            unreachable!("not used by RelationTargetPolicy")
        }

        async fn delete(
            &self,
            _tenant_id: &TenantId,
            _database_id: &DatabaseId,
            _id: &DataId,
        ) -> errors::Result<()> {
            unreachable!("not used by RelationTargetPolicy")
        }

        async fn delete_all(
            &self,
            _tenant_id: &TenantId,
            _database_id: &DatabaseId,
        ) -> errors::Result<()> {
            unreachable!("not used by RelationTargetPolicy")
        }

        async fn find_all_with_paging(
            &self,
            _tenant_id: &TenantId,
            _database_id: &DatabaseId,
            _page: u32,
            _page_size: u32,
        ) -> errors::Result<(DataCollection, OffsetPaginator)> {
            unreachable!("not used by RelationTargetPolicy")
        }
    }

    fn database(
        tenant_id: &TenantId,
        database_id: &DatabaseId,
    ) -> Database {
        Database::new(database_id, tenant_id, "database")
    }

    fn data(
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        data_id: &DataId,
    ) -> Data {
        Data::new(
            data_id,
            tenant_id,
            database_id,
            "record",
            vec![],
            Utc::now(),
            Utc::now(),
        )
        .expect("valid test Data")
    }

    fn relation_value(
        tenant_id: &TenantId,
        source_database_id: &DatabaseId,
        target_database_id: &DatabaseId,
        target_data_ids: &[DataId],
    ) -> PropertyData {
        let property = Property::new(
            &PropertyId::default(),
            tenant_id,
            source_database_id,
            "relation",
            &PropertyType::Relation(TypeRelation::new(
                target_database_id.clone(),
            )),
            false,
            0,
        );
        let command_value = target_data_ids
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",");

        PropertyData::new(&property, command_value)
            .expect("valid Relation command value")
    }

    fn policy(
        databases: Vec<Database>,
        data: Vec<Data>,
    ) -> Arc<RelationTargetPolicy> {
        RelationTargetPolicy::new(
            Arc::new(StubDatabaseRepository { databases }),
            Arc::new(StubDataRepository { data }),
        )
    }

    #[tokio::test]
    async fn self_database_relations_are_allowed() {
        let tenant_id = TenantId::default();
        let database_id = DatabaseId::default();
        let target_data_id = DataId::default();
        let property_data = relation_value(
            &tenant_id,
            &database_id,
            &database_id,
            std::slice::from_ref(&target_data_id),
        );

        policy(
            vec![database(&tenant_id, &database_id)],
            vec![data(&tenant_id, &database_id, &target_data_id)],
        )
        .validate(&tenant_id, &property_data)
        .await
        .expect("a Relation may target its own database");
    }

    #[tokio::test]
    async fn a_foreign_or_missing_target_database_is_generic_not_found() {
        let tenant_id = TenantId::default();
        let foreign_tenant_id = TenantId::default();
        let source_database_id = DatabaseId::default();
        let target_database_id = DatabaseId::default();
        let property_data = relation_value(
            &tenant_id,
            &source_database_id,
            &target_database_id,
            &[],
        );

        let foreign_error = policy(
            vec![database(&foreign_tenant_id, &target_database_id)],
            vec![],
        )
        .validate(&tenant_id, &property_data)
        .await
        .expect_err("a foreign Relation database must remain hidden");
        let missing_error = policy(vec![], vec![])
            .validate(&tenant_id, &property_data)
            .await
            .expect_err("a missing Relation database must be not found");

        assert!(foreign_error.is_not_found());
        assert_eq!(foreign_error.to_string(), missing_error.to_string());
        assert_eq!(
            foreign_error.to_string(),
            "NotFoundError: resource not found"
        );
    }

    #[tokio::test]
    async fn a_target_data_id_must_belong_to_the_configured_database() {
        let tenant_id = TenantId::default();
        let source_database_id = DatabaseId::default();
        let target_database_id = DatabaseId::default();
        let other_database_id = DatabaseId::default();
        let existing_data_id = DataId::default();
        let invalid_data_id = DataId::default();
        let property_data = relation_value(
            &tenant_id,
            &source_database_id,
            &target_database_id,
            &[existing_data_id.clone(), invalid_data_id.clone()],
        );

        let error = policy(
            vec![database(&tenant_id, &target_database_id)],
            vec![
                data(&tenant_id, &target_database_id, &existing_data_id),
                data(&tenant_id, &other_database_id, &invalid_data_id),
            ],
        )
        .validate(&tenant_id, &property_data)
        .await
        .expect_err("a DataId from another database must be hidden");

        assert!(error.is_not_found());
        assert_eq!(error.to_string(), "NotFoundError: resource not found");
    }
}
