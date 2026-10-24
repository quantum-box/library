//! Private, online-only PM projections. No provider payload enters the
//! database-manager record/search/export/offline paths.
pub mod migration;
mod migration_store;
pub mod store;
pub mod wire;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(&'static str),
    #[error("{0}")]
    Conflict(&'static str),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(deny_unknown_fields)]
pub struct ResourceRef {
    pub provider: String,
    pub workspace_id: String,
    pub kind: String,
    pub external_id: String,
}
impl ResourceRef {
    pub fn validate(&self) -> Result<()> {
        if self.provider != "linear"
            || !["initiative", "project"].contains(&self.kind.as_str())
            || self.workspace_id.is_empty()
            || self.external_id.is_empty()
            || self.workspace_id.len() > 255
            || self.external_id.len() > 255
        {
            return Err(Error::Invalid("invalid PM resource reference"));
        }
        Ok(())
    }
    pub fn key(&self) -> String {
        // Tuple encoding, not delimiter concatenation: IDs may contain delimiters.
        hash(
            &serde_json::to_vec(self)
                .expect("resource reference serialization"),
        )
    }
}
pub fn hash(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ResourceOwner {
    pub external_id: String,
    pub name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Projection {
    pub resource_ref: ResourceRef,
    pub name: String,
    pub url: Option<String>,
    pub archived: bool,
    pub status: Option<String>,
    pub owner: Option<ResourceOwner>,
    pub target_date: Option<String>,
    pub source_updated_at: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Relation {
    pub source: ResourceRef,
    pub target: ResourceRef,
    pub removed: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Upsert,
    Archived,
    Deleted,
    Inaccessible,
    RelationChange,
    AccessRevoked,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    pub subscription_id: String,
    pub generation: u64,
    pub sequence: u64,
    pub event_id: String,
    pub resource_ref: ResourceRef,
    pub observed_revision: u64,
    pub operation: Operation,
    pub projection: Option<Projection>,
    pub relation: Option<Relation>,
    pub schema_version: u32,
    pub observed_at: String,
}
impl Change {
    pub fn validate(&self) -> Result<()> {
        self.resource_ref.validate()?;
        if self.schema_version != 1
            || self.event_id.is_empty()
            || self.event_id.len() > 255
            || self.sequence == 0
            || (self.observed_revision == 0
                && self.operation != Operation::AccessRevoked)
        {
            return Err(Error::Invalid("invalid PM change envelope"));
        }
        if matches!(self.operation, Operation::Upsert | Operation::Archived)
            && self.projection.is_none()
        {
            return Err(Error::Invalid(
                "upsert/archive requires projection",
            ));
        }
        if matches!(self.operation, Operation::RelationChange) {
            let relation = self
                .relation
                .as_ref()
                .ok_or(Error::Invalid("missing relation"))?;
            relation.source.validate()?;
            relation.target.validate()?;
            if relation.source.kind != "initiative"
                || relation.target.kind != "project"
                || relation.source.workspace_id
                    != relation.target.workspace_id
                || relation.source.workspace_id
                    != self.resource_ref.workspace_id
            {
                return Err(Error::Invalid(
                    "invalid initiative-project relation",
                ));
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePage {
    pub generation: u64,
    pub changes: Vec<Change>,
    pub next_cursor: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionRecord {
    pub id: String,
    pub resource_ref: ResourceRef,
    pub projection: Projection,
    pub observed_at: String,
    pub observed_revision: u64,
    pub lifecycle: String,
    /// Library-owned text. Never overwritten by a provider event.
    pub note: String,
    pub document_ids: Vec<String>,
    pub legacy_data_id: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct ProjectionList {
    pub sync_pending: bool,
    pub records: Vec<ProjectionRecord>,
    pub relations: Vec<Relation>,
    pub pending_relations: usize,
    pub next_after: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn reference(kind: &str) -> ResourceRef {
        ResourceRef {
            provider: "linear".into(),
            workspace_id: "ws".into(),
            kind: kind.into(),
            external_id: "id".into(),
        }
    }
    fn change() -> Change {
        Change {
            subscription_id: "sub".into(),
            generation: 1,
            sequence: 1,
            event_id: "ev".into(),
            resource_ref: reference("project"),
            observed_revision: 1,
            operation: Operation::Upsert,
            projection: Some(Projection {
                resource_ref: reference("project"),
                name: "project".into(),
                url: None,
                status: None,
                owner: None,
                source_updated_at: None,
                target_date: None,
                archived: false,
            }),
            relation: None,
            schema_version: 1,
            observed_at: "2026-10-24T00:00:00Z".into(),
        }
    }
    #[test]
    fn identity_is_kind_and_workspace_scoped() {
        assert_ne!(
            reference("project").key(),
            reference("initiative").key()
        );
        let mut r = reference("project");
        r.workspace_id = "ws2".into();
        assert_ne!(r.key(), reference("project").key());
    }
    #[test]
    fn rejects_missing_payload() {
        let mut c = change();
        c.projection = None;
        assert!(c.validate().is_err());
        c.operation = Operation::Inaccessible;
        assert!(c.validate().is_ok());
    }
    #[test]
    fn validates_relation_scope() {
        let mut c = change();
        c.operation = Operation::RelationChange;
        c.relation = Some(Relation {
            source: reference("initiative"),
            target: reference("project"),
            removed: false,
        });
        assert!(c.validate().is_ok());
        c.relation.as_mut().unwrap().target.workspace_id = "other".into();
        assert!(c.validate().is_err());
    }
    #[test]
    fn revocation_before_first_sync_accepts_zero_revision() {
        let mut c = change();
        c.operation = Operation::AccessRevoked;
        c.projection = None;
        c.observed_revision = 0;
        assert!(c.validate().is_ok());
        c.operation = Operation::Inaccessible;
        assert!(c.validate().is_err());
    }
    #[test]
    fn unknown_schema_is_not_accepted() {
        let mut c = change();
        c.schema_version = 2;
        assert!(c.validate().is_err());
    }
    #[test]
    fn provider_payload_cannot_smuggle_local_text() {
        let mut p =
            serde_json::to_value(change().projection.unwrap()).unwrap();
        p["note"] = "secret".into();
        assert!(serde_json::from_value::<Projection>(p).is_err());
    }
}
