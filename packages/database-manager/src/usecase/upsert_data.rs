//! Create-or-update a record at a caller-supplied id.
//!
//! `AddData` mints the id itself and `UpdateData` requires the record to exist,
//! so neither serves a caller that already holds an id for a record the server
//! may not have — which is every local-first client: it assigns the id offline
//! and pushes later, and its first push looks exactly like an edit. Sending
//! that as an update gets a 404 and the record is lost.
//!
//! The branch is taken here, next to the repository, rather than by the caller
//! reading first and then choosing: a read-then-write leaves a window in which
//! another writer creates the record, and the loser of that race would send a
//! create for an id that now exists.

use std::sync::Arc;

use chrono::Utc;

use crate::domain::{
    CreateRecordCommand, Data, DataRepository, Database, DatabaseId,
    PatchRecordCommand, PropertyData, PropertyRepository,
    PropertyValueChange, RecordUnitOfWork,
};
use crate::usecase::add_data::populate_auto_generated_ids;
use crate::usecase::database_scope::DatabaseScope;
use crate::usecase::update_data::validate_auto_generated_id_update;
use crate::usecase::{
    RelationTargetPolicy, RelationTargetValidationPort,
    UpsertDataInputData, UpsertDataInputPort, UpsertOutcome,
};
use value_object::RepositoryV1;

#[derive(Debug)]
pub struct UpsertDataInteractorImpl {
    database_repo: Arc<dyn RepositoryV1<DatabaseId, Database>>,
    property_repo: Arc<dyn PropertyRepository>,
    data_repo: Arc<dyn DataRepository>,
    record_uow: Arc<dyn RecordUnitOfWork>,
    relation_target_validator: Arc<dyn RelationTargetValidationPort>,
}

impl UpsertDataInteractorImpl {
    pub fn new(
        database_repo: Arc<dyn RepositoryV1<DatabaseId, Database>>,
        property_repo: Arc<dyn PropertyRepository>,
        data_repo: Arc<dyn DataRepository>,
        record_uow: Arc<dyn RecordUnitOfWork>,
    ) -> Arc<Self> {
        let relation_target_validator = RelationTargetPolicy::new(
            database_repo.clone(),
            data_repo.clone(),
        );
        Self::new_with_relation_target_validator(
            database_repo,
            property_repo,
            data_repo,
            record_uow,
            relation_target_validator,
        )
    }

    pub fn new_with_relation_target_validator(
        database_repo: Arc<dyn RepositoryV1<DatabaseId, Database>>,
        property_repo: Arc<dyn PropertyRepository>,
        data_repo: Arc<dyn DataRepository>,
        record_uow: Arc<dyn RecordUnitOfWork>,
        relation_target_validator: Arc<dyn RelationTargetValidationPort>,
    ) -> Arc<Self> {
        Arc::new(Self {
            database_repo,
            property_repo,
            data_repo,
            record_uow,
            relation_target_validator,
        })
    }

    /// The existing-record branch, byte-for-byte what `UpdateData` does.
    ///
    /// Keeping it identical is the point: an upsert must not become a second,
    /// subtly different way to edit a record.
    async fn update_existing(
        &self,
        database: &Database,
        input: UpsertDataInputData<'_>,
        mut data: Data,
    ) -> errors::Result<Data> {
        data.update_name(&input.name.parse()?);
        let mut changes = Vec::with_capacity(input.data.len());
        for d in input.data {
            let property = self
                .property_repo
                .find_by_id(
                    &d.property_id,
                    database.id(),
                    database.tenant_id(),
                )
                .await?
                .ok_or_else(DatabaseScope::not_found)?;
            validate_auto_generated_id_update(
                &property,
                input.data_id,
                data.get_property_data(property.id()),
                &d.value,
            )?;
            let property_data =
                PropertyData::from_command(&property, d.value)?;
            self.relation_target_validator
                .validate(input.tenant_id, &property_data)
                .await?;
            data.update_property_data(&property_data)?;
            changes.push(PropertyValueChange::from_property_data(
                &property,
                &property_data,
            )?);
        }
        self.record_uow
            .patch_atomically(&PatchRecordCommand {
                record: data.clone(),
                changes,
            })
            .await?;

        Ok(data)
    }

    /// The missing-record branch: `AddData`, but at the id the caller chose.
    async fn create_at(
        &self,
        database: &Database,
        input: UpsertDataInputData<'_>,
    ) -> errors::Result<Data> {
        let properties = self
            .property_repo
            .find_all(database.id(), database.tenant_id())
            .await?;

        let mut property_data_list = Vec::new();
        for val in input.data.into_iter() {
            let property = properties
                .iter()
                .find(|x| x.id() == &val.property_id)
                .ok_or_else(DatabaseScope::not_found)?;
            let col = PropertyData::from_command(property, val.value)?;
            self.relation_target_validator
                .validate(input.tenant_id, &col)
                .await?;
            property_data_list.push(col);
        }
        populate_auto_generated_ids(
            &properties,
            &mut property_data_list,
            input.data_id,
        )?;

        let data = Data::new(
            input.data_id,
            database.tenant_id(),
            database.id(),
            input.name,
            property_data_list,
            Utc::now(),
            Utc::now(),
        )?;
        let changes = data
            .property_data()
            .iter()
            .map(|property_data| {
                let property = properties
                    .iter()
                    .find(|property| {
                        property.id() == property_data.property_id()
                    })
                    .ok_or_else(DatabaseScope::not_found)?;
                PropertyValueChange::from_property_data(
                    property,
                    property_data,
                )
            })
            .collect::<errors::Result<Vec<_>>>()?;
        self.record_uow
            .create_atomically(&CreateRecordCommand {
                record: data.clone(),
                changes,
            })
            .await?;

        Ok(data)
    }
}

#[async_trait::async_trait]
impl UpsertDataInputPort for UpsertDataInteractorImpl {
    #[tracing::instrument(skip(self))]
    async fn execute(
        &self,
        input: UpsertDataInputData<'_>,
    ) -> errors::Result<(Data, UpsertOutcome)> {
        let scope = DatabaseScope::new(input.tenant_id, input.database_id);
        let database =
            scope.require_database(self.database_repo.as_ref()).await?;

        // Not `require_data`: a missing record is the create branch here, not
        // a 404. The tenant/database filter still has to match what that
        // helper applies, so a record outside the scope stays invisible rather
        // than being adopted into it.
        let existing = self
            .data_repo
            .find_by_id(input.data_id, database.id(), database.tenant_id())
            .await?
            .filter(|data| {
                data.tenant_id() == input.tenant_id
                    && data.database_id() == input.database_id
            });

        match existing {
            Some(data) => {
                let data =
                    self.update_existing(&database, input, data).await?;
                Ok((data, UpsertOutcome::Updated))
            }
            None => {
                let data = self.create_at(&database, input).await?;
                Ok((data, UpsertOutcome::Created))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        AddPropertyCommand, DataCollection, DataId, Property, PropertyId,
        PropertySchemaMutationPort, PropertyType, PropertyValueCommand,
        UpdatePropertyCommand,
    };
    use crate::usecase::PropertyDataInputData;
    use std::sync::Mutex;
    use tachyon_sdk::auth;
    use value_object::{OffsetPage, OffsetPaginator, TenantId};

    /// The records the stub repository serves, shared with the unit of work.
    ///
    /// A create has to become visible to the next `find_by_id`, otherwise the
    /// second call in the test below would take the create branch again and
    /// the test would pass for the wrong reason.
    #[derive(Debug, Default)]
    struct RecordStore {
        records: Mutex<Vec<Data>>,
    }

    impl RecordStore {
        fn put(&self, record: Data) {
            let mut records = self.records.lock().unwrap();
            match records.iter().position(|held| held.id() == record.id()) {
                Some(index) => records[index] = record,
                None => records.push(record),
            }
        }
    }

    #[derive(Debug)]
    struct StubDatabaseRepository {
        database: Database,
    }

    #[async_trait::async_trait]
    impl RepositoryV1<DatabaseId, Database> for StubDatabaseRepository {
        async fn save(&self, _entity: &Database) -> errors::Result<()> {
            unreachable!("not used by UpsertData")
        }

        async fn delete(
            &self,
            _tenant_id: &TenantId,
            _id: &DatabaseId,
        ) -> errors::Result<()> {
            unreachable!("not used by UpsertData")
        }

        async fn get_by_id(
            &self,
            tenant_id: &TenantId,
            id: &DatabaseId,
        ) -> errors::Result<Option<Database>> {
            Ok(Some(self.database.clone()).filter(|database| {
                database.tenant_id() == tenant_id && database.id() == id
            }))
        }

        async fn find_all(
            &self,
            _tenant_id: &TenantId,
        ) -> errors::Result<Vec<Database>> {
            unreachable!("not used by UpsertData")
        }
    }

    #[derive(Debug)]
    struct StubDataRepository {
        store: Arc<RecordStore>,
    }

    #[async_trait::async_trait]
    impl DataRepository for StubDataRepository {
        async fn find_by_id(
            &self,
            id: &DataId,
            database_id: &DatabaseId,
            tenant_id: &TenantId,
        ) -> errors::Result<Option<Data>> {
            Ok(self
                .store
                .records
                .lock()
                .unwrap()
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
            unreachable!("not used by UpsertData")
        }

        async fn delete(
            &self,
            _tenant_id: &TenantId,
            _database_id: &DatabaseId,
            _id: &DataId,
        ) -> errors::Result<()> {
            unreachable!("not used by UpsertData")
        }

        async fn delete_all(
            &self,
            _tenant_id: &TenantId,
            _database_id: &DatabaseId,
        ) -> errors::Result<()> {
            unreachable!("not used by UpsertData")
        }

        async fn find_all_with_paging(
            &self,
            _tenant_id: &TenantId,
            _database_id: &DatabaseId,
            _page: OffsetPage,
        ) -> errors::Result<(DataCollection, OffsetPaginator)> {
            unreachable!("not used by UpsertData")
        }
    }

    #[derive(Debug)]
    struct StubPropertyRepository {
        properties: Vec<Property>,
    }

    #[async_trait::async_trait]
    impl PropertySchemaMutationPort for StubPropertyRepository {
        async fn add_property_atomically(
            &self,
            _command: &AddPropertyCommand,
        ) -> errors::Result<Property> {
            unreachable!("not used by UpsertData")
        }

        async fn update_property_atomically(
            &self,
            _command: &UpdatePropertyCommand,
        ) -> errors::Result<Property> {
            unreachable!("not used by UpsertData")
        }

        async fn delete_property_atomically(
            &self,
            _tenant_id: &TenantId,
            _database_id: &DatabaseId,
            _property_id: &PropertyId,
        ) -> errors::Result<Property> {
            unreachable!("not used by UpsertData")
        }
    }

    #[async_trait::async_trait]
    impl PropertyRepository for StubPropertyRepository {
        async fn find_by_id(
            &self,
            id: &PropertyId,
            _database_id: &DatabaseId,
            _tenant_id: &TenantId,
        ) -> errors::Result<Option<Property>> {
            Ok(self
                .properties
                .iter()
                .find(|property| property.id() == id)
                .cloned())
        }

        async fn find_all(
            &self,
            _database_id: &DatabaseId,
            _tenant_id: &TenantId,
        ) -> errors::Result<Vec<Property>> {
            Ok(self.properties.clone())
        }

        async fn delete(
            &self,
            _tenant_id: &TenantId,
            _id: &PropertyId,
        ) -> errors::Result<()> {
            unreachable!("not used by UpsertData")
        }

        async fn delete_all(
            &self,
            _tenant_id: &TenantId,
            _database_id: &DatabaseId,
        ) -> errors::Result<()> {
            unreachable!("not used by UpsertData")
        }
    }

    /// Records which branch the interactor actually took.
    ///
    /// Asserting on the returned `UpsertOutcome` alone would not catch an
    /// implementation that reported `Created` while writing a patch.
    #[derive(Debug)]
    struct RecordingUnitOfWork {
        store: Arc<RecordStore>,
        creates: Mutex<Vec<DataId>>,
        patches: Mutex<Vec<DataId>>,
    }

    #[async_trait::async_trait]
    impl RecordUnitOfWork for RecordingUnitOfWork {
        async fn create_atomically(
            &self,
            command: &CreateRecordCommand,
        ) -> errors::Result<()> {
            self.creates
                .lock()
                .unwrap()
                .push(command.record.id().clone());
            self.store.put(command.record.clone());
            Ok(())
        }

        async fn patch_atomically(
            &self,
            command: &PatchRecordCommand,
        ) -> errors::Result<()> {
            self.patches
                .lock()
                .unwrap()
                .push(command.record.id().clone());
            self.store.put(command.record.clone());
            Ok(())
        }

        async fn delete_atomically(
            &self,
            _tenant_id: &TenantId,
            _database_id: &DatabaseId,
            _data_id: &DataId,
        ) -> errors::Result<()> {
            unreachable!("not used by UpsertData")
        }
    }

    #[derive(Debug)]
    struct AcceptAllRelationTargets;

    #[async_trait::async_trait]
    impl RelationTargetValidationPort for AcceptAllRelationTargets {
        async fn validate(
            &self,
            _tenant_id: &TenantId,
            _property_data: &PropertyData,
        ) -> errors::Result<()> {
            Ok(())
        }
    }

    struct Fixture {
        tenant_id: TenantId,
        database_id: DatabaseId,
        title: Property,
        interactor: Arc<UpsertDataInteractorImpl>,
        uow: Arc<RecordingUnitOfWork>,
        store: Arc<RecordStore>,
    }

    fn fixture() -> Fixture {
        let tenant_id = TenantId::default();
        let database_id = DatabaseId::default();
        let title = Property::new(
            &PropertyId::default(),
            &tenant_id,
            &database_id,
            "title",
            &PropertyType::String,
            false,
            0,
        );
        let store = Arc::new(RecordStore::default());
        let uow = Arc::new(RecordingUnitOfWork {
            store: store.clone(),
            creates: Mutex::new(Vec::new()),
            patches: Mutex::new(Vec::new()),
        });
        let interactor =
            UpsertDataInteractorImpl::new_with_relation_target_validator(
                Arc::new(StubDatabaseRepository {
                    database: Database::new(
                        &database_id,
                        &tenant_id,
                        "database",
                    ),
                }),
                Arc::new(StubPropertyRepository {
                    properties: vec![title.clone()],
                }),
                Arc::new(StubDataRepository {
                    store: store.clone(),
                }),
                uow.clone(),
                Arc::new(AcceptAllRelationTargets),
            );

        Fixture {
            tenant_id,
            database_id,
            title,
            interactor,
            uow,
            store,
        }
    }

    async fn upsert(
        fixture: &Fixture,
        data_id: &DataId,
        name: &str,
        title: &str,
    ) -> errors::Result<(Data, UpsertOutcome)> {
        let multi_tenancy =
            auth::MultiTenancy::new_operator(fixture.tenant_id.clone());
        fixture
            .interactor
            .execute(UpsertDataInputData {
                executor: &auth::Executor::SystemUser,
                multi_tenancy: &multi_tenancy,
                tenant_id: &fixture.tenant_id,
                database_id: &fixture.database_id,
                data_id,
                name,
                data: vec![PropertyDataInputData {
                    property_id: fixture.title.id().clone(),
                    value: PropertyValueCommand::String(title.to_string()),
                }],
            })
            .await
    }

    /// The case the endpoint exists for: a client pushes a record id the
    /// server has never seen. `UpdateData` answers that with a 404.
    #[tokio::test]
    async fn a_first_write_creates_the_record_instead_of_404ing() {
        let fixture = fixture();
        let data_id = DataId::default();

        let (data, outcome) =
            upsert(&fixture, &data_id, "first", "hello").await.expect(
                "an id the server has never seen must be created, not 404",
            );

        assert_eq!(outcome, UpsertOutcome::Created);
        assert_eq!(data.id(), &data_id);
        assert_eq!(
            data.get_property_data(fixture.title.id())
                .map(PropertyData::string_value),
            Some("hello".to_string())
        );
        assert_eq!(*fixture.uow.creates.lock().unwrap(), vec![data_id]);
        assert!(fixture.uow.patches.lock().unwrap().is_empty());
    }

    /// And the edit that follows it still behaves like an edit — no second
    /// record, and no create replayed over the first one.
    #[tokio::test]
    async fn a_later_write_to_the_same_id_updates_it() {
        let fixture = fixture();
        let data_id = DataId::default();

        upsert(&fixture, &data_id, "first", "hello")
            .await
            .expect("the first write creates");
        let (data, outcome) = upsert(&fixture, &data_id, "second", "world")
            .await
            .expect("the second write updates");

        assert_eq!(outcome, UpsertOutcome::Updated);
        assert_eq!(data.id(), &data_id);
        assert_eq!(data.name().to_string(), "second");
        assert_eq!(
            data.get_property_data(fixture.title.id())
                .map(PropertyData::string_value),
            Some("world".to_string())
        );
        assert_eq!(
            *fixture.uow.creates.lock().unwrap(),
            vec![data_id.clone()]
        );
        assert_eq!(*fixture.uow.patches.lock().unwrap(), vec![data_id]);
        assert_eq!(fixture.store.records.lock().unwrap().len(), 1);
    }
}
