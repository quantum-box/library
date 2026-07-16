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

/// Dedicated versioned creation boundary.
///
/// The adapter owns idempotency, Relation target/cardinality validation,
/// initial Record persistence, canonical projections, and Outbox insertion in
/// one transaction. Keeping this separate from the compatibility
/// `RecordUnitOfWork` prevents a legacy creator from appearing to satisfy the
/// versioned decision contract.
#[async_trait::async_trait]
pub trait VersionedRecordCreationUnitOfWork:
    Debug + Send + Sync + 'static
{
    async fn decide_create_atomically(
        &self,
        command: &DecideRecordCreateCommand,
    ) -> errors::Result<RecordMutationDecision>;
}

/// Dedicated versioned deletion boundary.
///
/// Keeping DELETE separate from the PATCH port lets the adapter claim the
/// journal with mutation kind DELETE and execute Relation target-deletion
/// policy without widening the already released PATCH contract.
#[async_trait::async_trait]
pub trait VersionedRecordDeletionUnitOfWork:
    Debug + Send + Sync + 'static
{
    async fn decide_delete_atomically(
        &self,
        command: &DecideRecordDeleteCommand,
    ) -> errors::Result<RecordMutationDecision>;
}
