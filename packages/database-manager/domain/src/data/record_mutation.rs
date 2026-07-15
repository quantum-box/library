use super::*;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub enum PropertyValueChange {
    Set {
        property: Property,
        value: KnownPropertyValue,
    },
    Clear {
        property: Property,
    },
}

impl PropertyValueChange {
    pub fn from_property_data(
        property: &Property,
        property_data: &PropertyData,
    ) -> errors::Result<Self> {
        if property.id() != property_data.property_id() {
            return Err(errors::Error::invalid(
                "PropertyData does not belong to Property",
            ));
        }

        match property_data.envelope() {
            Some(PropertyValue::Known(value)) => Ok(Self::Set {
                property: property.clone(),
                value: value.clone(),
            }),
            Some(PropertyValue::Opaque(value)) => {
                value.ensure_writable()?;
                unreachable!("opaque values are never writable")
            }
            None => Ok(Self::Clear {
                property: property.clone(),
            }),
        }
    }

    pub fn property(&self) -> &Property {
        match self {
            Self::Set { property, .. } | Self::Clear { property } => {
                property
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateRecordCommand {
    pub record: Data,
    pub changes: Vec<PropertyValueChange>,
}

#[derive(Debug, Clone)]
pub struct PatchRecordCommand {
    pub record: Data,
    pub changes: Vec<PropertyValueChange>,
}

#[async_trait::async_trait]
pub trait RecordUnitOfWork: Debug + Send + Sync + 'static {
    async fn create_atomically(
        &self,
        command: &CreateRecordCommand,
    ) -> errors::Result<()>;

    async fn patch_atomically(
        &self,
        command: &PatchRecordCommand,
    ) -> errors::Result<()>;

    async fn delete_atomically(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        data_id: &DataId,
    ) -> errors::Result<()>;
}
