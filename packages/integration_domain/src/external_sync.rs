//! Provider-neutral domain model for continuous external synchronization.
//!
//! A binding owns configuration at the Library repository boundary. An object
//! link only maps one Library data item to one provider object. Provider
//! cursors and credentials deliberately do not belong to either model.

use std::{collections::BTreeMap, fmt::Debug, str::FromStr};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use value_object::TenantId;

use crate::{ConnectionId, OAuthProvider};

macro_rules! opaque_library_id {
    ($name:ident, $prefix:literal) => {
        #[derive(
            Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn parse(value: impl Into<String>) -> errors::Result<Self> {
                let value = value.into();
                if !value.starts_with($prefix)
                    || value.len() <= $prefix.len()
                {
                    return Err(errors::Error::invalid(format!(
                        "invalid {}: expected a non-empty {} identifier",
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

        impl std::fmt::Display for $name {
            fn fmt(
                &self,
                f: &mut std::fmt::Formatter<'_>,
            ) -> std::fmt::Result {
                f.write_str(&self.0)
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

opaque_library_id!(LibraryRepoId, "rp_");
opaque_library_id!(LibraryDataId, "data_");

/// Unique ID of an external synchronization binding.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ExternalSyncBindingId(String);

impl ExternalSyncBindingId {
    pub fn generate() -> Self {
        Self(format!(
            "esb_{}",
            ulid::Ulid::new().to_string().to_lowercase()
        ))
    }

    pub fn parse(value: impl Into<String>) -> errors::Result<Self> {
        let value = value.into();
        if !value.starts_with("esb_") || value.len() <= 4 {
            return Err(errors::Error::invalid(
                "invalid ExternalSyncBindingId: expected an esb_ identifier",
            ));
        }
        Ok(Self(value.to_lowercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ExternalSyncBindingId {
    fn default() -> Self {
        Self::generate()
    }
}

impl std::fmt::Display for ExternalSyncBindingId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for ExternalSyncBindingId {
    type Err = errors::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

/// Provider-owned, non-secret configuration selecting an external scope.
///
/// The canonical hash is stable across JSON object key order and is used for
/// idempotent binding identity: `(tenant, repo, provider, scope_hash)`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExternalScope {
    value: Value,
    identity_hash: String,
}

impl<'de> Deserialize<'de> for ExternalScope {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct StoredScope {
            value: Value,
            identity_hash: String,
        }

        let stored = StoredScope::deserialize(deserializer)?;
        Self::from_stored(stored.value, stored.identity_hash)
            .map_err(serde::de::Error::custom)
    }
}

impl ExternalScope {
    pub fn new(value: Value) -> errors::Result<Self> {
        let object = value.as_object().ok_or_else(|| {
            errors::Error::invalid("external_scope must be a JSON object")
        })?;
        if object.is_empty() {
            return Err(errors::Error::invalid(
                "external_scope must not be empty",
            ));
        }
        reject_non_scope_state(&value)?;
        let identity_hash = hash_json(&value);
        Ok(Self {
            value,
            identity_hash,
        })
    }

    pub fn from_stored(
        value: Value,
        identity_hash: impl Into<String>,
    ) -> errors::Result<Self> {
        let scope = Self::new(value)?;
        let stored_hash = identity_hash.into();
        if scope.identity_hash != stored_hash {
            return Err(errors::Error::invalid(
                "external_scope hash does not match its canonical value",
            ));
        }
        Ok(scope)
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn identity_hash(&self) -> &str {
        &self.identity_hash
    }
}

fn reject_non_scope_state(value: &Value) -> errors::Result<()> {
    const FORBIDDEN_KEYS: &[&str] = &[
        "accesstoken",
        "apikey",
        "authorization",
        "clientsecret",
        "credential",
        "credentials",
        "cursor",
        "password",
        "privatekey",
        "refreshtoken",
        "secret",
        "token",
        "webhooksecret",
    ];

    match value {
        Value::Object(object) => {
            for (key, nested) in object {
                let normalized = key
                    .chars()
                    .filter(|character| character.is_ascii_alphanumeric())
                    .flat_map(char::to_lowercase)
                    .collect::<String>();
                if FORBIDDEN_KEYS.contains(&normalized.as_str()) {
                    return Err(errors::Error::invalid(format!(
                        "external_scope must not contain credential or cursor field `{key}`"
                    )));
                }
                reject_non_scope_state(nested)?;
            }
        }
        Value::Array(values) => {
            for nested in values {
                reject_non_scope_state(nested)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn canonical_json(value: &Value) -> String {
    match value {
        Value::Object(object) => {
            let sorted = object.iter().collect::<BTreeMap<_, _>>();
            let fields = sorted
                .into_iter()
                .map(|(key, value)| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(key)
                            .expect("JSON key serialization"),
                        canonical_json(value)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{fields}}}")
        }
        Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(canonical_json)
                .collect::<Vec<_>>()
                .join(",")
        ),
        _ => {
            serde_json::to_string(value).expect("JSON value serialization")
        }
    }
}

fn hash_json(value: &Value) -> String {
    format!("{:x}", Sha256::digest(canonical_json(value).as_bytes()))
}

fn hash_text(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

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
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
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

string_enum!(ExternalSyncPolicy {
    Review => "review",
    Disabled => "disabled",
});
string_enum!(ExternalDeletePolicy {
    ReviewTombstone => "review_tombstone",
});
string_enum!(ExternalSyncBindingStatus {
    Active => "active",
    Paused => "paused",
    ReauthorizationRequired => "reauthorization_required",
});

/// Repository-level synchronization configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalSyncBinding {
    id: ExternalSyncBindingId,
    tenant_id: TenantId,
    library_repo_id: LibraryRepoId,
    provider: OAuthProvider,
    connection_id: ConnectionId,
    external_scope: ExternalScope,
    object_type: String,
    mapping: Value,
    inbound_policy: ExternalSyncPolicy,
    outbound_policy: ExternalSyncPolicy,
    delete_policy: ExternalDeletePolicy,
    status: ExternalSyncBindingStatus,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ExternalSyncBinding {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: ExternalSyncBindingId,
        tenant_id: TenantId,
        library_repo_id: LibraryRepoId,
        provider: OAuthProvider,
        connection_id: ConnectionId,
        external_scope: ExternalScope,
        object_type: impl Into<String>,
        mapping: Value,
        inbound_policy: ExternalSyncPolicy,
        outbound_policy: ExternalSyncPolicy,
        delete_policy: ExternalDeletePolicy,
        status: ExternalSyncBindingStatus,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> errors::Result<Self> {
        let object_type = object_type.into();
        if object_type.trim().is_empty() {
            return Err(errors::Error::invalid(
                "object_type must not be empty",
            ));
        }
        if !mapping.is_object() {
            return Err(errors::Error::invalid(
                "mapping must be a JSON object",
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
            library_repo_id,
            provider,
            connection_id,
            external_scope,
            object_type,
            mapping,
            inbound_policy,
            outbound_policy,
            delete_policy,
            status,
            created_at,
            updated_at,
        })
    }

    pub fn create(
        tenant_id: TenantId,
        library_repo_id: LibraryRepoId,
        provider: OAuthProvider,
        connection_id: ConnectionId,
        external_scope: ExternalScope,
        object_type: impl Into<String>,
        mapping: Value,
    ) -> errors::Result<Self> {
        let now = Utc::now();
        Self::new(
            ExternalSyncBindingId::generate(),
            tenant_id,
            library_repo_id,
            provider,
            connection_id,
            external_scope,
            object_type,
            mapping,
            ExternalSyncPolicy::Review,
            ExternalSyncPolicy::Review,
            ExternalDeletePolicy::ReviewTombstone,
            ExternalSyncBindingStatus::Active,
            now,
            now,
        )
    }

    pub fn id(&self) -> &ExternalSyncBindingId {
        &self.id
    }
    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }
    pub fn library_repo_id(&self) -> &LibraryRepoId {
        &self.library_repo_id
    }
    pub fn provider(&self) -> OAuthProvider {
        self.provider
    }
    pub fn connection_id(&self) -> &ConnectionId {
        &self.connection_id
    }
    pub fn external_scope(&self) -> &ExternalScope {
        &self.external_scope
    }
    pub fn object_type(&self) -> &str {
        &self.object_type
    }
    pub fn mapping(&self) -> &Value {
        &self.mapping
    }
    pub fn inbound_policy(&self) -> ExternalSyncPolicy {
        self.inbound_policy
    }
    pub fn outbound_policy(&self) -> ExternalSyncPolicy {
        self.outbound_policy
    }
    pub fn delete_policy(&self) -> ExternalDeletePolicy {
        self.delete_policy
    }
    pub fn status(&self) -> ExternalSyncBindingStatus {
        self.status
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

/// Mapping between one Library data item and one external object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalObjectLink {
    binding_id: ExternalSyncBindingId,
    data_id: LibraryDataId,
    external_object_id: String,
    last_accepted_external_revision: Option<String>,
    last_delivered_library_revision: Option<String>,
    base_content_hash: Option<String>,
}

impl ExternalObjectLink {
    pub fn new(
        binding_id: ExternalSyncBindingId,
        data_id: LibraryDataId,
        external_object_id: impl Into<String>,
        last_accepted_external_revision: Option<String>,
        last_delivered_library_revision: Option<String>,
        base_content_hash: Option<String>,
    ) -> errors::Result<Self> {
        let external_object_id = external_object_id.into();
        if external_object_id.trim().is_empty() {
            return Err(errors::Error::invalid(
                "external_object_id must not be empty",
            ));
        }
        if base_content_hash.as_ref().is_some_and(|hash| {
            hash.len() != 64
                || !hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        }) {
            return Err(errors::Error::invalid(
                "base_content_hash must be a 64-character hexadecimal SHA-256",
            ));
        }
        Ok(Self {
            binding_id,
            data_id,
            external_object_id,
            last_accepted_external_revision,
            last_delivered_library_revision,
            base_content_hash,
        })
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
    pub fn external_object_id_hash(&self) -> String {
        hash_text(&self.external_object_id)
    }
    pub fn last_accepted_external_revision(&self) -> Option<&str> {
        self.last_accepted_external_revision.as_deref()
    }
    pub fn last_delivered_library_revision(&self) -> Option<&str> {
        self.last_delivered_library_revision.as_deref()
    }
    pub fn base_content_hash(&self) -> Option<&str> {
        self.base_content_hash.as_deref()
    }
}

#[async_trait]
pub trait ExternalSyncBindingRepository: Send + Sync + Debug {
    async fn save(
        &self,
        binding: &ExternalSyncBinding,
    ) -> errors::Result<()>;
    async fn find_by_id(
        &self,
        tenant_id: &TenantId,
        id: &ExternalSyncBindingId,
    ) -> errors::Result<Option<ExternalSyncBinding>>;
    async fn find_by_scope(
        &self,
        tenant_id: &TenantId,
        repo_id: &LibraryRepoId,
        provider: OAuthProvider,
        external_scope: &ExternalScope,
    ) -> errors::Result<Option<ExternalSyncBinding>>;
    async fn find_by_repo(
        &self,
        tenant_id: &TenantId,
        repo_id: &LibraryRepoId,
    ) -> errors::Result<Vec<ExternalSyncBinding>>;
}

#[async_trait]
pub trait ExternalObjectLinkRepository: Send + Sync + Debug {
    async fn save(
        &self,
        tenant_id: &TenantId,
        link: &ExternalObjectLink,
    ) -> errors::Result<()>;
    async fn find_by_data(
        &self,
        tenant_id: &TenantId,
        binding_id: &ExternalSyncBindingId,
        data_id: &LibraryDataId,
    ) -> errors::Result<Option<ExternalObjectLink>>;
    async fn find_by_external_object(
        &self,
        tenant_id: &TenantId,
        binding_id: &ExternalSyncBindingId,
        external_object_id: &str,
    ) -> errors::Result<Option<ExternalObjectLink>>;
    async fn find_by_binding(
        &self,
        tenant_id: &TenantId,
        binding_id: &ExternalSyncBindingId,
    ) -> errors::Result<Vec<ExternalObjectLink>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn scope_hash_is_stable_across_object_key_order() {
        let left = ExternalScope::new(json!({
            "github_repository": "quantum-box/library",
            "ref": "main",
            "filter": { "path": "docs/**/*.md", "enabled": true }
        }))
        .unwrap();
        let right = ExternalScope::new(json!({
            "filter": { "enabled": true, "path": "docs/**/*.md" },
            "ref": "main",
            "github_repository": "quantum-box/library"
        }))
        .unwrap();

        assert_eq!(left.identity_hash(), right.identity_hash());
        assert_eq!(left.identity_hash().len(), 64);
    }

    #[test]
    fn scope_rejects_credentials_and_cursor_state() {
        for value in [
            json!({"repository": "example/repo", "access_token": "nope"}),
            json!({"repository": "example/repo", "nested": {"cursor": "nope"}}),
        ] {
            assert!(ExternalScope::new(value).is_err());
        }
    }

    #[test]
    fn binding_defaults_to_review_policies() {
        let binding = ExternalSyncBinding::create(
            "tn_01j91h09tpj5ehwbwfwfxpak2b".parse().unwrap(),
            LibraryRepoId::parse("rp_01j91h09tpj5ehwbwfwfxpak2b").unwrap(),
            OAuthProvider::Github,
            ConnectionId::new("con_01j91h09tpj5ehwbwfwfxpak2b"),
            ExternalScope::new(json!({"repository": "example/docs"}))
                .unwrap(),
            "markdown_document",
            json!({"content": "body"}),
        )
        .unwrap();

        assert_eq!(binding.inbound_policy(), ExternalSyncPolicy::Review);
        assert_eq!(binding.outbound_policy(), ExternalSyncPolicy::Review);
        assert_eq!(
            binding.delete_policy(),
            ExternalDeletePolicy::ReviewTombstone
        );
    }

    #[test]
    fn stored_scope_detects_hash_mismatch() {
        let value = json!({"repository": "example/docs"});
        assert!(ExternalScope::from_stored(value, "0".repeat(64)).is_err());

        let serialized = json!({
            "value": {"repository": "example/docs"},
            "identity_hash": "0".repeat(64)
        });
        assert!(
            serde_json::from_value::<ExternalScope>(serialized).is_err()
        );
    }

    #[test]
    fn object_link_round_trips_through_json() {
        let link = ExternalObjectLink::new(
            ExternalSyncBindingId::generate(),
            LibraryDataId::parse("data_01j91h09tpj5ehwbwfwfxpak2b")
                .unwrap(),
            "docs/guide.md",
            Some("external-revision".into()),
            Some("library-revision".into()),
            Some("a".repeat(64)),
        )
        .unwrap();

        let stored = serde_json::to_value(&link).unwrap();
        let restored: ExternalObjectLink =
            serde_json::from_value(stored).unwrap();
        assert_eq!(restored, link);
        assert_eq!(restored.external_object_id_hash().len(), 64);
    }
}
