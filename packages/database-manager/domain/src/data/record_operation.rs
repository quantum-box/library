use super::*;
use chrono::{DateTime, Utc};
use derive_getters::Getters;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use value_object::{TenantId, Text, Ulid};

pub const RECORD_REQUEST_FINGERPRINT_VERSION_V1: u16 = 1;
pub const RECORD_DECISION_VERSION_V1: u16 = 1;

/// A client-stable idempotency key for one Record mutation.
///
/// The value is deliberately opaque. UUIDs, ULIDs, and Photon operation IDs
/// can all cross this boundary without importing an external DTO into the
/// Database bounded context.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RecordOperationId(String);

impl RecordOperationId {
    pub fn new(value: impl Into<String>) -> errors::Result<Self> {
        let value = value.into();
        if value.is_empty() || value.len() > 64 || !value.is_ascii() {
            return Err(errors::Error::invalid(
                "record operation id must be 1..=64 ASCII bytes",
            ));
        }
        if !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'-' | b'_' | b'.' | b':')
        }) {
            return Err(errors::Error::invalid(
                "record operation id contains an unsupported character",
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for RecordOperationId {
    fn default() -> Self {
        Self(format!("rop_{}", Ulid::new().to_string().to_lowercase()))
    }
}

impl Display for RecordOperationId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, formatter)
    }
}

impl Serialize for RecordOperationId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for RecordOperationId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

util::def_id!(RecordEventId, "dbe_");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecordActorKind {
    User,
    ServiceAccount,
    System,
}

impl Display for RecordActorKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::User => "USER",
            Self::ServiceAccount => "SERVICE_ACCOUNT",
            Self::System => "SYSTEM",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Getters)]
pub struct RecordActor {
    kind: RecordActorKind,
    id: String,
}

impl RecordActor {
    pub fn new(
        kind: RecordActorKind,
        id: impl Into<String>,
    ) -> errors::Result<Self> {
        let id = id.into();
        if id.is_empty() || id.len() > 64 || !id.is_ascii() {
            return Err(errors::Error::invalid(
                "record actor id must be 1..=64 ASCII bytes",
            ));
        }
        Ok(Self { kind, id })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Getters)]
pub struct RecordPropertySnapshot {
    property_id: PropertyId,
    value: Option<EncodedPropertyValue>,
}

impl RecordPropertySnapshot {
    pub fn new(
        property_id: &PropertyId,
        value: Option<EncodedPropertyValue>,
    ) -> Self {
        Self {
            property_id: property_id.clone(),
            value,
        }
    }
}

/// Storage-neutral current Record returned by a CAS conflict.
///
/// Property values retain their type and encoding envelopes so a newer value
/// can be replayed without being retyped by an older binary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Getters)]
pub struct RecordSnapshot {
    tenant_id: TenantId,
    database_id: DatabaseId,
    data_id: DataId,
    name: String,
    #[serde(with = "record_version_string")]
    record_version: RecordVersion,
    properties: Vec<RecordPropertySnapshot>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[allow(clippy::too_many_arguments)]
impl RecordSnapshot {
    pub fn new(
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        data_id: &DataId,
        name: impl Into<String>,
        record_version: RecordVersion,
        mut properties: Vec<RecordPropertySnapshot>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        properties.sort_by(|left, right| {
            left.property_id().cmp(right.property_id())
        });
        Self {
            tenant_id: tenant_id.clone(),
            database_id: database_id.clone(),
            data_id: data_id.clone(),
            name: name.into(),
            record_version,
            properties,
            created_at,
            updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordPropertyPatch {
    property_id: PropertyId,
    value: PropertyValueCommand,
}

impl RecordPropertyPatch {
    pub fn new(
        property_id: &PropertyId,
        value: PropertyValueCommand,
    ) -> Self {
        Self {
            property_id: property_id.clone(),
            value,
        }
    }

    pub fn property_id(&self) -> &PropertyId {
        &self.property_id
    }

    pub fn value(&self) -> &PropertyValueCommand {
        &self.value
    }

    pub fn into_value(self) -> PropertyValueCommand {
        self.value
    }
}

#[derive(Debug, Clone, PartialEq, Getters)]
pub struct RecordPatch {
    name: Option<Text>,
    properties: Vec<RecordPropertyPatch>,
}

impl RecordPatch {
    pub fn new(
        name: Option<Text>,
        mut properties: Vec<RecordPropertyPatch>,
    ) -> Self {
        properties.sort_by(|left, right| {
            left.property_id().cmp(right.property_id())
        });
        Self { name, properties }
    }

    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.properties.is_empty()
    }

    pub fn has_duplicate_properties(&self) -> bool {
        let mut ids = HashSet::with_capacity(self.properties.len());
        self.properties
            .iter()
            .any(|patch| !ids.insert(patch.property_id().to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordRequestFingerprint {
    version: u16,
    digest: [u8; 32],
}

impl RecordRequestFingerprint {
    pub fn version(&self) -> u16 {
        self.version
    }

    pub fn digest(&self) -> &[u8; 32] {
        &self.digest
    }
}

#[derive(Debug, Clone, PartialEq, Getters)]
pub struct DecideRecordPatchCommand {
    tenant_id: TenantId,
    database_id: DatabaseId,
    data_id: DataId,
    operation_id: RecordOperationId,
    expected_version: RecordVersion,
    actor: RecordActor,
    patch: RecordPatch,
    fingerprint: RecordRequestFingerprint,
}

impl DecideRecordPatchCommand {
    pub fn new(
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        data_id: &DataId,
        operation_id: &RecordOperationId,
        expected_version: RecordVersion,
        actor: RecordActor,
        patch: RecordPatch,
    ) -> errors::Result<Self> {
        let fingerprint = fingerprint_v1(
            tenant_id,
            database_id,
            data_id,
            expected_version,
            &actor,
            &patch,
        )?;
        Ok(Self {
            tenant_id: tenant_id.clone(),
            database_id: database_id.clone(),
            data_id: data_id.clone(),
            operation_id: operation_id.clone(),
            expected_version,
            actor,
            patch,
            fingerprint,
        })
    }
}

fn command_value(value: &PropertyValueCommand) -> serde_json::Value {
    match value {
        PropertyValueCommand::Clear => serde_json::json!({"kind": "clear"}),
        PropertyValueCommand::String(value) => {
            serde_json::json!({"kind": "string", "value": value})
        }
        PropertyValueCommand::Integer(value) => {
            serde_json::json!({"kind": "integer", "value": value})
        }
        PropertyValueCommand::Html(value) => {
            serde_json::json!({"kind": "html", "value": value})
        }
        PropertyValueCommand::Markdown(value) => {
            serde_json::json!({"kind": "markdown", "value": value})
        }
        PropertyValueCommand::Relation(values) => {
            let mut values =
                values.iter().map(ToString::to_string).collect::<Vec<_>>();
            values.sort();
            serde_json::json!({"kind": "relation", "value": values})
        }
        PropertyValueCommand::Select(value) => serde_json::json!({
            "kind": "select",
            "value": value.to_string()
        }),
        PropertyValueCommand::MultiSelect(values) => {
            let mut values =
                values.iter().map(ToString::to_string).collect::<Vec<_>>();
            values.sort();
            serde_json::json!({"kind": "multi_select", "value": values})
        }
        PropertyValueCommand::Id(value) => {
            serde_json::json!({"kind": "id", "value": value})
        }
        PropertyValueCommand::Location(value) => serde_json::json!({
            "kind": "location",
            "latitude": value.latitude(),
            "longitude": value.longitude()
        }),
        PropertyValueCommand::Date(value) => {
            serde_json::json!({"kind": "date", "value": value})
        }
        PropertyValueCommand::Image(value) => {
            serde_json::json!({"kind": "image", "value": value})
        }
    }
}

fn fingerprint_v1(
    tenant_id: &TenantId,
    database_id: &DatabaseId,
    data_id: &DataId,
    expected_version: RecordVersion,
    actor: &RecordActor,
    patch: &RecordPatch,
) -> errors::Result<RecordRequestFingerprint> {
    let properties = patch
        .properties()
        .iter()
        .map(|property| {
            serde_json::json!({
                "property_id": property.property_id().to_string(),
                "value": command_value(property.value()),
            })
        })
        .collect::<Vec<_>>();
    let material = serde_json::json!({
        "fingerprint_version": RECORD_REQUEST_FINGERPRINT_VERSION_V1,
        "tenant_id": tenant_id.to_string(),
        "database_id": database_id.to_string(),
        "data_id": data_id.to_string(),
        "actor_kind": actor.kind().to_string(),
        "actor_id": actor.id(),
        "expected_version": expected_version.to_string(),
        "patch": {
            "name": patch.name().as_ref().map(ToString::to_string),
            "properties": properties,
        },
    });
    let bytes = serde_json::to_vec(&material)
        .map_err(errors::Error::internal_server_error)?;
    let digest: [u8; 32] = Sha256::digest(bytes).into();
    Ok(RecordRequestFingerprint {
        version: RECORD_REQUEST_FINGERPRINT_VERSION_V1,
        digest,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecordRejectionCode {
    EmptyPatch,
    DuplicateProperty,
    ResourceNotFound,
    InvalidPropertyValue,
    RelationProjectionRequired,
    RelationCardinalityExceeded,
    IndexProjectionRequired,
    IdempotencyKeyReuse,
    VersionExhausted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecordMutationDecision {
    Accepted {
        decision_version: u16,
        operation_id: RecordOperationId,
        #[serde(with = "record_version_string")]
        record_version: RecordVersion,
        event_ids: Vec<RecordEventId>,
    },
    Conflict {
        decision_version: u16,
        operation_id: RecordOperationId,
        current: RecordSnapshot,
    },
    Rejected {
        decision_version: u16,
        operation_id: RecordOperationId,
        code: RecordRejectionCode,
    },
}

impl RecordMutationDecision {
    pub fn accepted(
        operation_id: &RecordOperationId,
        record_version: RecordVersion,
        event_ids: Vec<RecordEventId>,
    ) -> Self {
        Self::Accepted {
            decision_version: RECORD_DECISION_VERSION_V1,
            operation_id: operation_id.clone(),
            record_version,
            event_ids,
        }
    }

    pub fn conflict(
        operation_id: &RecordOperationId,
        current: RecordSnapshot,
    ) -> Self {
        Self::Conflict {
            decision_version: RECORD_DECISION_VERSION_V1,
            operation_id: operation_id.clone(),
            current,
        }
    }

    pub fn rejected(
        operation_id: &RecordOperationId,
        code: RecordRejectionCode,
    ) -> Self {
        Self::Rejected {
            decision_version: RECORD_DECISION_VERSION_V1,
            operation_id: operation_id.clone(),
            code,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::Accepted { .. } => "ACCEPTED",
            Self::Conflict { .. } => "CONFLICT",
            Self::Rejected { .. } => "REJECTED",
        }
    }

    pub fn decision_version(&self) -> u16 {
        match self {
            Self::Accepted {
                decision_version, ..
            }
            | Self::Conflict {
                decision_version, ..
            }
            | Self::Rejected {
                decision_version, ..
            } => *decision_version,
        }
    }

    pub fn operation_id(&self) -> &RecordOperationId {
        match self {
            Self::Accepted { operation_id, .. }
            | Self::Conflict { operation_id, .. }
            | Self::Rejected { operation_id, .. } => operation_id,
        }
    }
}

mod record_version_string {
    use super::RecordVersion;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(
        value: &RecordVersion,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<RecordVersion, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let value =
            value.parse::<u64>().map_err(serde::de::Error::custom)?;
        RecordVersion::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecordPropertyDelta {
    Set {
        property_id: PropertyId,
        value: EncodedPropertyValue,
    },
    Clear {
        property_id: PropertyId,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordNameDelta {
    pub previous: String,
    pub current: String,
}

/// Library-owned v1 delta persisted in the Database BC transactional outbox.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordPatchedEventV1 {
    pub event_id: RecordEventId,
    pub event_type: String,
    pub operation_id: RecordOperationId,
    pub tenant_id: TenantId,
    pub database_id: DatabaseId,
    pub data_id: DataId,
    pub previous_version: String,
    pub record_version: String,
    pub actor: RecordActor,
    pub name: Option<RecordNameDelta>,
    pub properties: Vec<RecordPropertyDelta>,
    pub occurred_at: DateTime<Utc>,
}

#[async_trait::async_trait]
pub trait VersionedRecordMutationUnitOfWork:
    std::fmt::Debug + Send + Sync + 'static
{
    async fn decide_patch_atomically(
        &self,
        command: &DecideRecordPatchCommand,
    ) -> errors::Result<RecordMutationDecision>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(
        patches: Vec<RecordPropertyPatch>,
    ) -> DecideRecordPatchCommand {
        DecideRecordPatchCommand::new(
            &TenantId::default(),
            &DatabaseId::default(),
            &DataId::default(),
            &RecordOperationId::default(),
            RecordVersion::INITIAL,
            RecordActor::new(RecordActorKind::System, "system")
                .expect("actor"),
            RecordPatch::new(None, patches),
        )
        .expect("command")
    }

    #[test]
    fn operation_id_is_bounded_ascii_and_case_sensitive() {
        assert!(RecordOperationId::new("").is_err());
        assert!(RecordOperationId::new("x".repeat(65)).is_err());
        assert!(RecordOperationId::new("non-ascii-エ").is_err());
        assert_ne!(
            RecordOperationId::new("Operation-A").unwrap(),
            RecordOperationId::new("operation-a").unwrap()
        );
    }

    #[test]
    fn fingerprint_is_stable_for_property_and_set_order() {
        let first_id = PropertyId::default();
        let second_id = PropertyId::default();
        let select_a = SelectItemId::default();
        let select_b = SelectItemId::default();
        let left = command(vec![
            RecordPropertyPatch::new(
                &second_id,
                PropertyValueCommand::MultiSelect(vec![
                    select_b.clone(),
                    select_a.clone(),
                ]),
            ),
            RecordPropertyPatch::new(
                &first_id,
                PropertyValueCommand::String("value".to_string()),
            ),
        ]);
        let right = DecideRecordPatchCommand::new(
            left.tenant_id(),
            left.database_id(),
            left.data_id(),
            &RecordOperationId::default(),
            RecordVersion::INITIAL,
            left.actor().clone(),
            RecordPatch::new(
                None,
                vec![
                    RecordPropertyPatch::new(
                        &first_id,
                        PropertyValueCommand::String("value".to_string()),
                    ),
                    RecordPropertyPatch::new(
                        &second_id,
                        PropertyValueCommand::MultiSelect(vec![
                            select_a, select_b,
                        ]),
                    ),
                ],
            ),
        )
        .expect("command");

        // Operation IDs are deliberately excluded from request equivalence.
        assert_eq!(left.fingerprint(), right.fingerprint());
    }

    #[test]
    fn fingerprint_binds_actor_scope_version_and_payload() {
        let property_id = PropertyId::default();
        let base = command(vec![RecordPropertyPatch::new(
            &property_id,
            PropertyValueCommand::String("one".to_string()),
        )]);
        let changed = DecideRecordPatchCommand::new(
            base.tenant_id(),
            base.database_id(),
            base.data_id(),
            &RecordOperationId::default(),
            RecordVersion::new(2).unwrap(),
            base.actor().clone(),
            RecordPatch::new(
                None,
                vec![RecordPropertyPatch::new(
                    &property_id,
                    PropertyValueCommand::String("two".to_string()),
                )],
            ),
        )
        .unwrap();

        assert_ne!(base.fingerprint(), changed.fingerprint());
    }

    #[test]
    fn journal_decision_versions_are_decimal_strings_without_precision_loss()
     {
        let operation_id = RecordOperationId::default();
        let maximum =
            RecordVersion::new(u64::MAX).expect("maximum version");
        let decision = RecordMutationDecision::accepted(
            &operation_id,
            maximum,
            Vec::new(),
        );

        let json = serde_json::to_value(&decision).expect("serialize");
        assert_eq!(
            json.get("record_version"),
            Some(&serde_json::Value::String(u64::MAX.to_string()))
        );
        let restored: RecordMutationDecision =
            serde_json::from_value(json).expect("deserialize");
        assert_eq!(restored, decision);

        let snapshot = RecordSnapshot::new(
            &TenantId::default(),
            &DatabaseId::default(),
            &DataId::default(),
            "maximum",
            maximum,
            Vec::new(),
            Utc::now(),
            Utc::now(),
        );
        let conflict =
            RecordMutationDecision::conflict(&operation_id, snapshot);
        let json = serde_json::to_value(&conflict).expect("serialize");
        assert_eq!(
            json.pointer("/current/record_version"),
            Some(&serde_json::Value::String(u64::MAX.to_string()))
        );
        let restored: RecordMutationDecision =
            serde_json::from_value(json).expect("deserialize");
        assert_eq!(restored, conflict);
    }
}
