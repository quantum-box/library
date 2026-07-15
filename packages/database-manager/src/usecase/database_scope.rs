use crate::domain::{Data, DataId, DataRepository, Database, DatabaseId};
use value_object::{RepositoryV1, TenantId};

const SCOPED_RESOURCE_NOT_FOUND: &str = "resource not found";

/// Tenant and database boundary shared by record use cases.
///
/// Repositories must apply the same scope in their queries, while this policy
/// keeps the application response independent from which part of the scope was
/// missing. That prevents callers from probing database or record ownership.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DatabaseScope<'a> {
    tenant_id: &'a TenantId,
    database_id: &'a DatabaseId,
}

impl<'a> DatabaseScope<'a> {
    pub(crate) fn new(
        tenant_id: &'a TenantId,
        database_id: &'a DatabaseId,
    ) -> Self {
        Self {
            tenant_id,
            database_id,
        }
    }

    pub(crate) async fn require_database<R>(
        &self,
        repository: &R,
    ) -> errors::Result<Database>
    where
        R: RepositoryV1<DatabaseId, Database> + ?Sized,
    {
        repository
            .get_by_id(self.tenant_id, self.database_id)
            .await?
            .filter(|database| {
                database.tenant_id() == self.tenant_id
                    && database.id() == self.database_id
            })
            .ok_or_else(Self::not_found)
    }

    pub(crate) async fn require_data<R>(
        &self,
        repository: &R,
        data_id: &DataId,
    ) -> errors::Result<Data>
    where
        R: DataRepository + ?Sized,
    {
        repository
            .find_by_id(data_id, self.database_id, self.tenant_id)
            .await?
            .filter(|data| {
                data.tenant_id() == self.tenant_id
                    && data.database_id() == self.database_id
            })
            .ok_or_else(Self::not_found)
    }

    pub(crate) fn not_found() -> errors::Error {
        errors::Error::not_found(SCOPED_RESOURCE_NOT_FOUND)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::sync::Mutex;
    use value_object::OffsetPaginator;

    #[derive(Debug)]
    struct StubDatabaseRepository {
        result: Option<Database>,
        requested_scope: Mutex<Option<(TenantId, DatabaseId)>>,
    }

    impl StubDatabaseRepository {
        fn new(result: Option<Database>) -> Self {
            Self {
                result,
                requested_scope: Mutex::new(None),
            }
        }
    }

    #[async_trait::async_trait]
    impl RepositoryV1<DatabaseId, Database> for StubDatabaseRepository {
        async fn save(&self, _entity: &Database) -> errors::Result<()> {
            unreachable!("not used by the scope policy")
        }

        async fn delete(
            &self,
            _tenant_id: &TenantId,
            _id: &DatabaseId,
        ) -> errors::Result<()> {
            unreachable!("not used by the scope policy")
        }

        async fn get_by_id(
            &self,
            tenant_id: &TenantId,
            id: &DatabaseId,
        ) -> errors::Result<Option<Database>> {
            *self.requested_scope.lock().unwrap() =
                Some((tenant_id.clone(), id.clone()));
            Ok(self.result.clone())
        }

        async fn find_all(
            &self,
            _tenant_id: &TenantId,
        ) -> errors::Result<Vec<Database>> {
            unreachable!("not used by the scope policy")
        }
    }

    #[derive(Debug)]
    struct StubDataRepository {
        result: Option<Data>,
        requested_scope: Mutex<Option<(TenantId, DatabaseId, DataId)>>,
    }

    impl StubDataRepository {
        fn new(result: Option<Data>) -> Self {
            Self {
                result,
                requested_scope: Mutex::new(None),
            }
        }
    }

    #[async_trait::async_trait]
    impl DataRepository for StubDataRepository {
        async fn create(&self, _data: &Data) -> errors::Result<()> {
            unreachable!("not used by the scope policy")
        }

        async fn update(&self, _data: &Data) -> errors::Result<()> {
            unreachable!("not used by the scope policy")
        }

        async fn update_all(
            &self,
            _data: &crate::domain::DataCollection,
        ) -> errors::Result<()> {
            unreachable!("not used by the scope policy")
        }

        async fn find_by_id(
            &self,
            id: &DataId,
            database_id: &DatabaseId,
            tenant_id: &TenantId,
        ) -> errors::Result<Option<Data>> {
            *self.requested_scope.lock().unwrap() =
                Some((tenant_id.clone(), database_id.clone(), id.clone()));
            Ok(self.result.clone())
        }

        async fn find_all(
            &self,
            _id: &DatabaseId,
            _tenant_id: &TenantId,
        ) -> errors::Result<crate::domain::DataCollection> {
            unreachable!("not used by the scope policy")
        }

        async fn delete(
            &self,
            _tenant_id: &TenantId,
            _database_id: &DatabaseId,
            _id: &DataId,
        ) -> errors::Result<()> {
            unreachable!("not used by the scope policy")
        }

        async fn delete_all(
            &self,
            _tenant_id: &TenantId,
            _database_id: &DatabaseId,
        ) -> errors::Result<()> {
            unreachable!("not used by the scope policy")
        }

        async fn find_all_with_paging(
            &self,
            _tenant_id: &TenantId,
            _database_id: &DatabaseId,
            _page: u32,
            _page_size: u32,
        ) -> errors::Result<(crate::domain::DataCollection, OffsetPaginator)>
        {
            unreachable!("not used by the scope policy")
        }
    }

    fn data(tenant_id: &TenantId, database_id: &DatabaseId) -> Data {
        Data::new(
            &DataId::default(),
            tenant_id,
            database_id,
            "record",
            vec![],
            Utc::now(),
            Utc::now(),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn database_lookup_uses_the_complete_scope() {
        let tenant_id = TenantId::default();
        let database_id = DatabaseId::default();
        let repository = StubDatabaseRepository::new(Some(Database::new(
            &database_id,
            &tenant_id,
            "database",
        )));

        let database = DatabaseScope::new(&tenant_id, &database_id)
            .require_database(&repository)
            .await
            .expect("database must be in scope");

        assert_eq!(database.id(), &database_id);
        assert_eq!(
            repository.requested_scope.lock().unwrap().as_ref(),
            Some(&(tenant_id, database_id))
        );
    }

    #[tokio::test]
    async fn a_foreign_database_is_the_same_generic_not_found_as_missing() {
        let tenant_id = TenantId::default();
        let database_id = DatabaseId::default();
        let foreign_tenant_id = TenantId::default();
        let foreign = Database::new(
            &database_id,
            &foreign_tenant_id,
            "foreign database",
        );

        let foreign_error = DatabaseScope::new(&tenant_id, &database_id)
            .require_database(&StubDatabaseRepository::new(Some(foreign)))
            .await
            .expect_err("a foreign database must remain hidden");
        let missing_error = DatabaseScope::new(&tenant_id, &database_id)
            .require_database(&StubDatabaseRepository::new(None))
            .await
            .expect_err("a missing database must be not found");

        assert!(foreign_error.is_not_found());
        assert_eq!(foreign_error.to_string(), missing_error.to_string());
        assert_eq!(
            foreign_error.to_string(),
            "NotFoundError: resource not found"
        );
    }

    #[tokio::test]
    async fn a_record_returned_from_another_scope_is_hidden() {
        let tenant_id = TenantId::default();
        let database_id = DatabaseId::default();
        let data_id = DataId::default();
        let foreign_database_id = DatabaseId::default();
        let repository = StubDataRepository::new(Some(data(
            &tenant_id,
            &foreign_database_id,
        )));

        let error = DatabaseScope::new(&tenant_id, &database_id)
            .require_data(&repository, &data_id)
            .await
            .expect_err(
                "a record from another database must remain hidden",
            );

        assert!(error.is_not_found());
        assert_eq!(error.to_string(), "NotFoundError: resource not found");
        assert_eq!(
            repository.requested_scope.lock().unwrap().as_ref(),
            Some(&(tenant_id, database_id, data_id))
        );
    }
}
