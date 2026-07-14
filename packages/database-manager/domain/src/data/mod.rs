mod property_data;
mod property_data_value;

use chrono::{DateTime, Utc};
pub use property_data::*;
pub use property_data_value::*;
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
        let mut new_entity = Self {
            id: id.clone(),
            tenant_id: tenant_id.clone(),
            database_id: database_id.clone(),
            name: name.parse()?,
            property_data: vec![],
            created_at,
            updated_at,
        };
        for pd in property_data {
            new_entity.add_property_data(pd)?;
        }
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
    async fn create(&self, data: &Data) -> errors::Result<()>;
    async fn update(&self, data: &Data) -> errors::Result<()>;
    async fn update_all(&self, data: &DataCollection)
    -> errors::Result<()>;
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
        page: u32,
        page_size: u32,
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
                    PropertyData::new(&target, "remove".to_string()).unwrap(),
                    PropertyData::new(&retained, "first".to_string()).unwrap(),
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
        assert!(result
            .value()
            .iter()
            .all(|item| item.get_property_data(target.id()).is_none()));
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
