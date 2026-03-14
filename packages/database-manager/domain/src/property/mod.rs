//! Property
//!
//! TODO: add English documentation

use super::*;
use std::fmt::Debug;
use util::macros::*;

mod property_type;
pub use property_type::*;

def_id!(PropertyId, "prop_");

#[derive(Getters, Debug, Clone)]
pub struct Property {
    id: PropertyId,
    tenant_id: TenantId,
    database_id: DatabaseId,
    name: String,
    property_type: PropertyType,
    is_indexed: bool,
    property_num: u32,
    /// JSON metadata for property configuration (e.g., ext_github repos)
    meta_json: Option<String>,
}

impl Property {
    pub fn new(
        id: &PropertyId,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        name: &str,
        property_type: &PropertyType,
        is_indexed: bool,
        property_num: u32,
    ) -> Self {
        Self::with_meta_json(
            id,
            tenant_id,
            database_id,
            name,
            property_type,
            is_indexed,
            property_num,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_meta_json(
        id: &PropertyId,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        name: &str,
        property_type: &PropertyType,
        is_indexed: bool,
        property_num: u32,
        meta_json: Option<String>,
    ) -> Self {
        let name: String = if name.is_empty() {
            format!("property{property_num}")
        } else {
            name.into()
        };
        Self {
            id: id.clone(),
            tenant_id: tenant_id.clone(),
            database_id: database_id.clone(),
            name: name.to_string(),
            property_type: property_type.clone(),
            is_indexed,
            property_num,
            meta_json,
        }
    }

    pub fn update(
        &self,
        name: Option<&str>,
        property_type: Option<&PropertyType>,
    ) -> errors::Result<Self> {
        self.update_with_meta_json(name, property_type, None)
    }

    pub fn update_with_meta_json(
        &self,
        name: Option<&str>,
        property_type: Option<&PropertyType>,
        meta_json: Option<Option<String>>,
    ) -> errors::Result<Self> {
        let property = self.update_property_type(property_type)?;

        Ok(Self {
            name: name
                .map(|s| s.to_string())
                .unwrap_or(property.name.clone()),
            meta_json: meta_json
                .unwrap_or_else(|| property.meta_json.clone()),
            ..property
        })
    }

    fn update_property_type(
        &self,
        property_type: Option<&PropertyType>,
    ) -> errors::Result<Self> {
        if let Some(property_type) = property_type {
            if self.property_type.to_string() != property_type.to_string() {
                // TODO: add English comment
                // TODO: add English comment
                return Err(errors::invalid!(
                    "Property type is not match. Cannot change property type currently."
                ));
            }
            return Ok(Self {
                property_type: property_type.clone(),
                ..self.clone()
            });
        }
        Ok(self.clone())
    }
}

#[async_trait::async_trait]
pub trait PropertyRepository: Debug + Send + Sync + 'static {
    async fn create(&self, property: &Property) -> errors::Result<()>;
    async fn update(&self, property: &Property) -> errors::Result<()>;
    async fn find_by_id(
        &self,
        id: &PropertyId,
        database_id: &DatabaseId,
        tenant_id: &TenantId,
    ) -> errors::Result<Option<Property>>;
    async fn find_all(
        &self,
        database_id: &DatabaseId,
        tenant_id: &TenantId,
    ) -> errors::Result<Vec<Property>>;
    async fn delete(
        &self,
        tenant_id: &TenantId,
        id: &PropertyId,
    ) -> errors::Result<()>;
    async fn delete_all(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
    ) -> errors::Result<()>;
}
