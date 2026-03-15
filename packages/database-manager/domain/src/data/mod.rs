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
            .filter(|v| v.property_id().eq(property_id))
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
