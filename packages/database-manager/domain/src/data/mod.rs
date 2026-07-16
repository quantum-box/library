mod property_data;
mod property_data_value;
mod property_value_command;
mod record_mutation;
mod record_operation;
mod record_version;

use chrono::{DateTime, Utc};
pub use property_data::*;
pub use property_data_value::*;
pub use property_value_command::*;
pub use record_mutation::*;
pub use record_operation::*;
pub use record_version::*;
use util::macros::*;

use super::*;
use serde::Serialize;
use std::fmt::Debug;

#[derive(Debug, Clone, Serialize, Getters)]
pub struct DataCollection {
    value: Vec<Data>,
}

impl DataCollection {
    pub fn new(value: Vec<Data>) -> Self {
        Self { value }
    }

    pub fn delete_property_data(&self, property_id: &PropertyId) -> Self {
        let value = self
            .value
            .clone()
            .into_iter()
            .map(|v| v.delete_property_data(property_id))
            .collect::<Vec<Data>>();
        Self { value }
    }
}

#[derive(Getters, Debug, Clone, Serialize)]
pub struct Data {
    id: DataId,
    tenant_id: TenantId,
    database_id: DatabaseId,
    name: Text,
    record_version: RecordVersion,
    property_data: Vec<PropertyData>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Data {
    pub fn new(
        id: &DataId,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        name: &str,
        property_data: Vec<PropertyData>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> anyhow::Result<Self> {
        Self::restore(
            id,
            tenant_id,
            database_id,
            name,
            RecordVersion::INITIAL,
            property_data,
            created_at,
            updated_at,
        )
    }

    /// Rehydrates a persisted record without replacing its storage version.
    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        id: &DataId,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        name: &str,
        record_version: RecordVersion,
        property_data: Vec<PropertyData>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> anyhow::Result<Self> {
        let mut new_entity = Self {
            id: id.clone(),
            tenant_id: tenant_id.clone(),
            database_id: database_id.clone(),
            name: name.parse()?,
            record_version,
            property_data: vec![],
            created_at,
            updated_at,
        };
        for pd in property_data {
            new_entity.add_property_data(pd)?;
        }
        // Entity validation must not replace the persisted timestamp.
        new_entity.updated_at = updated_at;
        Ok(new_entity)
    }

    pub fn get_property_data(
        &self,
        property_id: &PropertyId,
    ) -> Option<&PropertyData> {
        self.property_data
            .iter()
            .find(|pd| pd.property_id() == property_id)
    }

    pub fn add_property_data(
        &mut self,
        property_data: PropertyData,
    ) -> anyhow::Result<()> {
        // block duplicates
        if self
            .property_data
            .iter()
            .any(|pd| pd.property_id() == property_data.property_id())
        {
            anyhow::bail!(
                "PropertyData with property_id {} already exists",
                property_data.property_id()
            );
        }
        self.property_data.push(property_data);
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn update_name(&mut self, name: &Text) {
        self.name = name.clone();
        self.updated_at = Utc::now();
    }

    pub fn update_many_property_data(
        &mut self,
        data: Vec<PropertyData>,
    ) -> anyhow::Result<()> {
        for pd in data {
            self.update_property_data(&pd)?;
        }
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn update_property_data(
        &mut self,
        data: &PropertyData,
    ) -> anyhow::Result<()> {
        let index = self
            .property_data
            .iter()
            .position(|pd| pd.property_id() == data.property_id())
            .ok_or(anyhow::anyhow!(
                "PropertyData with property_id {} does not exist",
                data.property_id()
            ))?;
        self.property_data[index] = data.clone();
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn delete_property_data(&self, property_id: &PropertyId) -> Self {
        let property_data = self
            .clone()
            .property_data
            .into_iter()
            .filter(|v| !v.property_id().eq(property_id))
            .collect::<Vec<PropertyData>>();
        Self {
            property_data,
            updated_at: Utc::now(),
            ..self.clone()
        }
    }
}

def_id!(DataId, "data_");

#[async_trait::async_trait]
pub trait DataRepository: Debug + Send + Sync + 'static {
    async fn find_by_id(
        &self,
        id: &DataId,
        database_id: &DatabaseId,
        tenant_id: &TenantId,
    ) -> errors::Result<Option<Data>>;
    async fn find_all(
        &self,
        id: &DatabaseId,
        tenant_id: &TenantId,
    ) -> errors::Result<DataCollection>;
    async fn delete(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        id: &DataId,
    ) -> errors::Result<()>;
    async fn delete_all(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
    ) -> errors::Result<()>;
    async fn find_all_with_paging(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        page: OffsetPage,
    ) -> errors::Result<(DataCollection, OffsetPaginator)>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn property(
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        name: &str,
        property_num: u32,
    ) -> Property {
        Property::new(
            &PropertyId::default(),
            tenant_id,
            database_id,
            name,
            &PropertyType::String,
            false,
            property_num,
        )
    }

    fn data(
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        property_data: Vec<PropertyData>,
    ) -> Data {
        let now = Utc::now();
        Data::new(
            &DataId::default(),
            tenant_id,
            database_id,
            "fixture",
            property_data,
            now,
            now,
        )
        .unwrap()
    }

    #[test]
    fn new_records_start_at_v1_and_restore_keeps_persisted_version() {
        let tenant_id = TenantId::default();
        let database_id = DatabaseId::default();
        let data_id = DataId::default();
        let created_at = Utc::now();
        let updated_at = created_at + chrono::Duration::seconds(5);

        let new_record = Data::new(
            &data_id,
            &tenant_id,
            &database_id,
            "new",
            vec![],
            created_at,
            updated_at,
        )
        .expect("new record");
        assert_eq!(new_record.record_version().get(), 1);

        let restored = Data::restore(
            &data_id,
            &tenant_id,
            &database_id,
            "restored",
            RecordVersion::new(9).expect("persisted version"),
            vec![],
            created_at,
            updated_at,
        )
        .expect("restored record");
        assert_eq!(restored.record_version().get(), 9);
        assert_eq!(*restored.updated_at(), updated_at);
    }

    #[test]
    fn delete_property_data_removes_only_the_target_property() {
        let tenant_id = TenantId::default();
        let database_id = DatabaseId::default();
        let target = property(&tenant_id, &database_id, "target", 0);
        let retained = property(&tenant_id, &database_id, "retained", 1);
        let data = data(
            &tenant_id,
            &database_id,
            vec![
                PropertyData::new(&target, "remove".to_string()).unwrap(),
                PropertyData::new(&retained, "keep".to_string()).unwrap(),
            ],
        );

        let result = data.delete_property_data(target.id());

        assert!(result.get_property_data(target.id()).is_none());
        assert_eq!(
            result
                .get_property_data(retained.id())
                .expect("retained property data")
                .string_value(),
            "keep"
        );
    }

    #[test]
    fn delete_property_data_is_safe_for_collections_and_missing_values() {
        let tenant_id = TenantId::default();
        let database_id = DatabaseId::default();
        let target = property(&tenant_id, &database_id, "target", 0);
        let retained = property(&tenant_id, &database_id, "retained", 1);
        let collection = DataCollection::new(vec![
            data(
                &tenant_id,
                &database_id,
                vec![
                    PropertyData::new(&target, "remove".to_string())
                        .unwrap(),
                    PropertyData::new(&retained, "first".to_string())
                        .unwrap(),
                ],
            ),
            data(
                &tenant_id,
                &database_id,
                vec![
                    PropertyData::new(&retained, "second".to_string())
                        .unwrap(),
                ],
            ),
        ]);

        let result = collection.delete_property_data(target.id());

        assert_eq!(result.value().len(), 2);
        assert!(
            result
                .value()
                .iter()
                .all(|item| item.get_property_data(target.id()).is_none())
        );
        assert_eq!(
            result.value()[0]
                .get_property_data(retained.id())
                .expect("first retained property data")
                .string_value(),
            "first"
        );
        assert_eq!(
            result.value()[1]
                .get_property_data(retained.id())
                .expect("second retained property data")
                .string_value(),
            "second"
        );
    }
}
