//! Auditable lifecycle for provider-neutral inbound changes and outbound
//! deliveries.

use std::{fmt::Debug, str::FromStr};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use value_object::TenantId;

use crate::{ExternalSyncBindingId, LibraryDataId};

macro_rules! generated_id {
    ($name:ident, $prefix:literal) => {
        #[derive(
            Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn generate() -> Self {
                Self(format!(
                    "{}{}",
                    $prefix,
                    ulid::Ulid::new().to_string().to_lowercase()
                ))
            }

            pub fn parse(value: impl Into<String>) -> errors::Result<Self> {
                let value = value.into();
                if !value.starts_with($prefix)
                    || value.len() <= $prefix.len()
                {
                    return Err(errors::Error::invalid(format!(
                        "invalid {}: expected a {} identifier",
                        stringify!($name),
                        $prefix
                    )));
                }
                Ok(Self(value.to_lowercase()))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::generate()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(
                &self,
                formatter: &mut std::fmt::Formatter<'_>,
            ) -> std::fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl FromStr for $name {
            type Err = errors::Error;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::parse(value)
            }
        }
    };
}

generated_id!(InboundChangeSetId, "ics_");
generated_id!(OutboundDeliveryId, "odl_");

macro_rules! string_enum {
    ($name:ident { $($variant:ident => $value:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }

        impl $name {
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $value),+ }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = errors::Error;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    $($value => Ok(Self::$variant),)+
                    _ => Err(errors::Error::invalid(format!(
                        "invalid {}: {value}", stringify!($name)
                    ))),
                }
            }
        }
    };
}

string_enum!(ExternalChangeType {
    Upsert => "upsert",
    Tombstone => "tombstone",
    Rename => "rename",
});

string_enum!(InboundChangeSetStatus {
    Pending => "pending",
    Accepted => "accepted",
    Rejected => "rejected",
    Conflict => "conflict",
});

string_enum!(OutboundDeliveryStatus {
    Pending => "pending",
    Retrying => "retrying",
    Delivered => "delivered",
    Conflict => "conflict",
    Failed => "failed",
});

fn hash_parts(parts: &[&str]) -> String {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update((part.len() as u64).to_be_bytes());
        digest.update(part.as_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn validate_non_empty(name: &str, value: &str) -> errors::Result<()> {
    if value.trim().is_empty() {
        return Err(errors::Error::invalid(format!(
            "{name} must not be empty"
        )));
    }
    Ok(())
}

/// A provider change waiting for an explicit Library decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InboundChangeSet {
    id: InboundChangeSetId,
    tenant_id: TenantId,
    binding_id: ExternalSyncBindingId,
    data_id: Option<LibraryDataId>,
    external_object_id: String,
    external_revision: String,
    base_external_revision: Option<String>,
    change_type: ExternalChangeType,
    payload: Value,
    idempotency_key: String,
    status: InboundChangeSetStatus,
    decision_note: Option<String>,
    created_at: DateTime<Utc>,
    decided_at: Option<DateTime<Utc>>,
}

impl InboundChangeSet {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: InboundChangeSetId,
        tenant_id: TenantId,
        binding_id: ExternalSyncBindingId,
        data_id: Option<LibraryDataId>,
        external_object_id: impl Into<String>,
        external_revision: impl Into<String>,
        base_external_revision: Option<String>,
        change_type: ExternalChangeType,
        payload: Value,
        idempotency_key: impl Into<String>,
        status: InboundChangeSetStatus,
        decision_note: Option<String>,
        created_at: DateTime<Utc>,
        decided_at: Option<DateTime<Utc>>,
    ) -> errors::Result<Self> {
        let external_object_id = external_object_id.into();
        let external_revision = external_revision.into();
        let idempotency_key = idempotency_key.into();
        validate_non_empty("external_object_id", &external_object_id)?;
        validate_non_empty("external_revision", &external_revision)?;
        if !payload.is_object() {
            return Err(errors::Error::invalid(
                "inbound change payload must be a JSON object",
            ));
        }
        if idempotency_key.len() != 64
            || !idempotency_key.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(errors::Error::invalid(
                "inbound idempotency_key must be a SHA-256 hex digest",
            ));
        }
        if matches!(status, InboundChangeSetStatus::Pending)
            != decided_at.is_none()
        {
            return Err(errors::Error::invalid(
                "pending inbound changes must be undecided and terminal changes must have decided_at",
            ));
        }
        Ok(Self {
            id,
            tenant_id,
            binding_id,
            data_id,
            external_object_id,
            external_revision,
            base_external_revision,
            change_type,
            payload,
            idempotency_key,
            status,
            decision_note,
            created_at,
            decided_at,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create(
        tenant_id: TenantId,
        binding_id: ExternalSyncBindingId,
        data_id: Option<LibraryDataId>,
        external_object_id: impl Into<String>,
        external_revision: impl Into<String>,
        base_external_revision: Option<String>,
        change_type: ExternalChangeType,
        payload: Value,
    ) -> errors::Result<Self> {
        let external_object_id = external_object_id.into();
        let external_revision = external_revision.into();
        let idempotency_key = hash_parts(&[
            binding_id.as_str(),
            &external_revision,
            &external_object_id,
            change_type.as_str(),
        ]);
        Self::new(
            InboundChangeSetId::generate(),
            tenant_id,
            binding_id,
            data_id,
            external_object_id,
            external_revision,
            base_external_revision,
            change_type,
            payload,
            idempotency_key,
            InboundChangeSetStatus::Pending,
            None,
            Utc::now(),
            None,
        )
    }

    pub fn accept(&mut self, note: Option<String>) -> errors::Result<()> {
        self.decide(InboundChangeSetStatus::Accepted, note)
    }

    pub fn reject(&mut self, note: Option<String>) -> errors::Result<()> {
        self.decide(InboundChangeSetStatus::Rejected, note)
    }

    pub fn mark_conflict(
        &mut self,
        note: impl Into<String>,
    ) -> errors::Result<()> {
        self.decide(InboundChangeSetStatus::Conflict, Some(note.into()))
    }

    fn decide(
        &mut self,
        status: InboundChangeSetStatus,
        note: Option<String>,
    ) -> errors::Result<()> {
        if matches!(
            self.status,
            InboundChangeSetStatus::Accepted
                | InboundChangeSetStatus::Rejected
        ) {
            return Err(errors::Error::conflict(
                "inbound change set already has a terminal decision",
            ));
        }
        self.status = status;
        self.decision_note = note;
        self.decided_at = Some(Utc::now());
        Ok(())
    }

    pub fn id(&self) -> &InboundChangeSetId {
        &self.id
    }
    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }
    pub fn binding_id(&self) -> &ExternalSyncBindingId {
        &self.binding_id
    }
    pub fn data_id(&self) -> Option<&LibraryDataId> {
        self.data_id.as_ref()
    }
    pub fn external_object_id(&self) -> &str {
        &self.external_object_id
    }
    pub fn external_revision(&self) -> &str {
        &self.external_revision
    }
    pub fn base_external_revision(&self) -> Option<&str> {
        self.base_external_revision.as_deref()
    }
    pub fn change_type(&self) -> ExternalChangeType {
        self.change_type
    }
    pub fn payload(&self) -> &Value {
        &self.payload
    }
    pub fn idempotency_key(&self) -> &str {
        &self.idempotency_key
    }
    pub fn status(&self) -> InboundChangeSetStatus {
        self.status
    }
    pub fn decision_note(&self) -> Option<&str> {
        self.decision_note.as_deref()
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn decided_at(&self) -> Option<DateTime<Utc>> {
        self.decided_at
    }
}

/// Delivery of one accepted Library revision to one linked external object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutboundDelivery {
    id: OutboundDeliveryId,
    tenant_id: TenantId,
    binding_id: ExternalSyncBindingId,
    data_id: LibraryDataId,
    external_object_id: String,
    library_revision: String,
    base_external_revision: Option<String>,
    payload: Value,
    idempotency_key: String,
    status: OutboundDeliveryStatus,
    attempt_count: u32,
    next_attempt_at: DateTime<Utc>,
    last_error_category: Option<String>,
    remote_revision: Option<String>,
    delivery_url: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl OutboundDelivery {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: OutboundDeliveryId,
        tenant_id: TenantId,
        binding_id: ExternalSyncBindingId,
        data_id: LibraryDataId,
        external_object_id: impl Into<String>,
        library_revision: impl Into<String>,
        base_external_revision: Option<String>,
        payload: Value,
        idempotency_key: impl Into<String>,
        status: OutboundDeliveryStatus,
        attempt_count: u32,
        next_attempt_at: DateTime<Utc>,
        last_error_category: Option<String>,
        remote_revision: Option<String>,
        delivery_url: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> errors::Result<Self> {
        let external_object_id = external_object_id.into();
        let library_revision = library_revision.into();
        let idempotency_key = idempotency_key.into();
        validate_non_empty("external_object_id", &external_object_id)?;
        validate_non_empty("library_revision", &library_revision)?;
        if !payload.is_object() {
            return Err(errors::Error::invalid(
                "outbound delivery payload must be a JSON object",
            ));
        }
        if idempotency_key.len() != 64
            || !idempotency_key.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(errors::Error::invalid(
                "outbound idempotency_key must be a SHA-256 hex digest",
            ));
        }
        if updated_at < created_at {
            return Err(errors::Error::invalid(
                "updated_at must not precede created_at",
            ));
        }
        Ok(Self {
            id,
            tenant_id,
            binding_id,
            data_id,
            external_object_id,
            library_revision,
            base_external_revision,
            payload,
            idempotency_key,
            status,
            attempt_count,
            next_attempt_at,
            last_error_category,
            remote_revision,
            delivery_url,
            created_at,
            updated_at,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create(
        tenant_id: TenantId,
        binding_id: ExternalSyncBindingId,
        data_id: LibraryDataId,
        external_object_id: impl Into<String>,
        library_revision: impl Into<String>,
        base_external_revision: Option<String>,
        payload: Value,
    ) -> errors::Result<Self> {
        let external_object_id = external_object_id.into();
        let library_revision = library_revision.into();
        let idempotency_key = hash_parts(&[
            binding_id.as_str(),
            &library_revision,
            &external_object_id,
        ]);
        // The same accepted revision may be observed more than once by an
        // at-least-once outbox consumer. A deterministic ID lets every retry
        // address the same lifecycle row before any provider call is made.
        let id = OutboundDeliveryId::parse(format!(
            "odl_{}",
            &idempotency_key[..26]
        ))?;
        let now = Utc::now();
        Self::new(
            id,
            tenant_id,
            binding_id,
            data_id,
            external_object_id,
            library_revision,
            base_external_revision,
            payload,
            idempotency_key,
            OutboundDeliveryStatus::Pending,
            0,
            now,
            None,
            None,
            None,
            now,
            now,
        )
    }

    pub fn mark_attempt(&mut self, now: DateTime<Utc>) {
        self.attempt_count = self.attempt_count.saturating_add(1);
        self.status = OutboundDeliveryStatus::Retrying;
        self.updated_at = now;
    }

    pub fn mark_delivered(
        &mut self,
        remote_revision: impl Into<String>,
        delivery_url: Option<String>,
        now: DateTime<Utc>,
    ) {
        self.status = OutboundDeliveryStatus::Delivered;
        self.remote_revision = Some(remote_revision.into());
        self.delivery_url = delivery_url;
        self.last_error_category = None;
        self.updated_at = now;
    }

    pub fn mark_retry(
        &mut self,
        error_category: impl Into<String>,
        now: DateTime<Utc>,
    ) {
        self.status = OutboundDeliveryStatus::Retrying;
        self.last_error_category = Some(error_category.into());
        let exponent = self.attempt_count.min(8);
        self.next_attempt_at =
            now + chrono::Duration::seconds(2_i64.pow(exponent));
        self.updated_at = now;
    }

    pub fn mark_conflict(
        &mut self,
        remote_revision: Option<String>,
        now: DateTime<Utc>,
    ) {
        self.status = OutboundDeliveryStatus::Conflict;
        self.remote_revision = remote_revision;
        self.last_error_category = Some("remote_revision_conflict".into());
        self.updated_at = now;
    }

    pub fn mark_failed(
        &mut self,
        error_category: impl Into<String>,
        now: DateTime<Utc>,
    ) {
        self.status = OutboundDeliveryStatus::Failed;
        self.last_error_category = Some(error_category.into());
        self.updated_at = now;
    }

    pub fn request_retry(
        &mut self,
        now: DateTime<Utc>,
    ) -> errors::Result<()> {
        if !matches!(
            self.status,
            OutboundDeliveryStatus::Pending
                | OutboundDeliveryStatus::Failed
                | OutboundDeliveryStatus::Conflict
                | OutboundDeliveryStatus::Retrying
        ) {
            return Err(errors::Error::conflict(
                "delivered deliveries cannot be retried",
            ));
        }
        self.status = OutboundDeliveryStatus::Pending;
        self.next_attempt_at = now;
        self.last_error_category = None;
        self.updated_at = now;
        Ok(())
    }

    pub fn request_retry_from_base(
        &mut self,
        accepted_external_revision: Option<String>,
        now: DateTime<Utc>,
    ) -> errors::Result<()> {
        self.request_retry(now)?;
        self.base_external_revision = accepted_external_revision;
        Ok(())
    }

    pub fn id(&self) -> &OutboundDeliveryId {
        &self.id
    }
    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }
    pub fn binding_id(&self) -> &ExternalSyncBindingId {
        &self.binding_id
    }
    pub fn data_id(&self) -> &LibraryDataId {
        &self.data_id
    }
    pub fn external_object_id(&self) -> &str {
        &self.external_object_id
    }
    pub fn library_revision(&self) -> &str {
        &self.library_revision
    }
    pub fn base_external_revision(&self) -> Option<&str> {
        self.base_external_revision.as_deref()
    }
    pub fn payload(&self) -> &Value {
        &self.payload
    }
    pub fn idempotency_key(&self) -> &str {
        &self.idempotency_key
    }
    pub fn status(&self) -> OutboundDeliveryStatus {
        self.status
    }
    pub fn attempt_count(&self) -> u32 {
        self.attempt_count
    }
    pub fn next_attempt_at(&self) -> DateTime<Utc> {
        self.next_attempt_at
    }
    pub fn last_error_category(&self) -> Option<&str> {
        self.last_error_category.as_deref()
    }
    pub fn remote_revision(&self) -> Option<&str> {
        self.remote_revision.as_deref()
    }
    pub fn delivery_url(&self) -> Option<&str> {
        self.delivery_url.as_deref()
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

#[async_trait]
pub trait InboundChangeSetRepository: Send + Sync + Debug {
    async fn save(
        &self,
        change_set: &InboundChangeSet,
    ) -> errors::Result<()>;
    async fn find_by_id(
        &self,
        tenant_id: &TenantId,
        id: &InboundChangeSetId,
    ) -> errors::Result<Option<InboundChangeSet>>;
    async fn find_by_binding(
        &self,
        tenant_id: &TenantId,
        binding_id: &ExternalSyncBindingId,
        status: Option<InboundChangeSetStatus>,
        limit: u32,
    ) -> errors::Result<Vec<InboundChangeSet>>;
}

#[async_trait]
pub trait OutboundDeliveryRepository: Send + Sync + Debug {
    async fn save(&self, delivery: &OutboundDelivery)
        -> errors::Result<()>;
    async fn find_by_id(
        &self,
        tenant_id: &TenantId,
        id: &OutboundDeliveryId,
    ) -> errors::Result<Option<OutboundDelivery>>;
    async fn find_by_binding(
        &self,
        tenant_id: &TenantId,
        binding_id: &ExternalSyncBindingId,
        status: Option<OutboundDeliveryStatus>,
        limit: u32,
    ) -> errors::Result<Vec<OutboundDelivery>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tenant() -> TenantId {
        "tn_01j91h09tpj5ehwbwfwfxpak2b".parse().unwrap()
    }

    fn binding() -> ExternalSyncBindingId {
        ExternalSyncBindingId::parse("esb_01j91h09tpj5ehwbwfwfxpak2b")
            .unwrap()
    }

    #[test]
    fn inbound_identity_is_stable_and_decision_is_terminal() {
        let mut first = InboundChangeSet::create(
            tenant(),
            binding(),
            None,
            "docs/readme.md",
            "commit-a",
            None,
            ExternalChangeType::Upsert,
            json!({"content":"hello"}),
        )
        .unwrap();
        let second = InboundChangeSet::create(
            tenant(),
            binding(),
            None,
            "docs/readme.md",
            "commit-a",
            None,
            ExternalChangeType::Upsert,
            json!({"content":"different transport copy"}),
        )
        .unwrap();
        assert_eq!(first.idempotency_key(), second.idempotency_key());
        first.accept(None).unwrap();
        assert!(first.reject(None).is_err());
    }

    #[test]
    fn inbound_conflict_can_be_explicitly_resolved() {
        let mut change = InboundChangeSet::create(
            tenant(),
            binding(),
            None,
            "docs/readme.md",
            "commit-b",
            Some("commit-a".into()),
            ExternalChangeType::Upsert,
            json!({"content":"external"}),
        )
        .unwrap();
        change.mark_conflict("base moved").unwrap();
        change.accept(Some("prefer external".into())).unwrap();
        assert_eq!(change.status(), InboundChangeSetStatus::Accepted);
    }

    #[test]
    fn outbound_identity_and_retry_are_deterministic() {
        let now = Utc::now();
        let mut delivery = OutboundDelivery::create(
            tenant(),
            binding(),
            LibraryDataId::parse("data_01j91h09tpj5ehwbwfwfxpak2b")
                .unwrap(),
            "docs/readme.md",
            "42",
            Some("commit-a".into()),
            json!({"content":"hello"}),
        )
        .unwrap();
        let duplicate = OutboundDelivery::create(
            tenant(),
            binding(),
            LibraryDataId::parse("data_01j91h09tpj5ehwbwfwfxpak2b")
                .unwrap(),
            "docs/readme.md",
            "42",
            Some("commit-a".into()),
            json!({"content":"same accepted revision"}),
        )
        .unwrap();
        assert_eq!(delivery.idempotency_key().len(), 64);
        assert_eq!(delivery.id(), duplicate.id());
        delivery.mark_attempt(now);
        delivery.mark_retry("rate_limited", now);
        assert!(delivery.next_attempt_at() > now);
        assert_eq!(delivery.attempt_count(), 1);
    }
}
