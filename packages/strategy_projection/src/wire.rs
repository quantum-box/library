//! Tachyon PM sync v1 wire contract. Kept separate from the storage model;
//! no cross-checkout path dependency is required by Library builds.
use crate::*;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cursor {
    pub generation: u64,
    pub sequence: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum Payload {
    Upsert {
        projection: Projection,
    },
    Archived {
        projection: Projection,
    },
    Inaccessible {
        resource_ref: ResourceRef,
    },
    RelationChange {
        relation: WireRelation,
        removed: bool,
    },
    AccessRevoked,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireRelation {
    pub source: ResourceRef,
    pub target: ResourceRef,
    pub kind: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionChange {
    pub subscription_id: String,
    pub generation: u64,
    pub sequence: u64,
    pub event_id: String,
    pub observed_revision: u64,
    pub schema_version: u32,
    pub payload_hash: String,
    pub observed_at: String,
    pub run_id: String,
    pub scope_revision: String,
    #[serde(flatten)]
    pub payload: Payload,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub changes: Vec<ProjectionChange>,
    pub next_cursor: Cursor,
    pub has_more: bool,
}
impl Page {
    pub fn into_page(self, workspace: &str) -> Result<ChangePage> {
        let mut changes = Vec::new();
        for c in self.changes {
            if hash(&serde_json::to_vec(&c.payload)?) != c.payload_hash {
                return Err(Error::Invalid(
                    "provider payload hash mismatch",
                ));
            }
            let (reference, operation, projection, relation) =
                match c.payload {
                    Payload::Upsert { projection } => (
                        projection.resource_ref.clone(),
                        Operation::Upsert,
                        Some(projection),
                        None,
                    ),
                    Payload::Archived { projection } => (
                        projection.resource_ref.clone(),
                        Operation::Archived,
                        Some(projection),
                        None,
                    ),
                    Payload::Inaccessible { resource_ref } => {
                        (resource_ref, Operation::Inaccessible, None, None)
                    }
                    Payload::RelationChange { relation, removed } => {
                        if relation.kind != "initiative_project" {
                            return Err(Error::Invalid(
                                "unsupported relation kind",
                            ));
                        }
                        (
                            relation.source.clone(),
                            Operation::RelationChange,
                            None,
                            Some(Relation {
                                source: relation.source,
                                target: relation.target,
                                removed,
                            }),
                        )
                    }
                    Payload::AccessRevoked => (
                        ResourceRef {
                            provider: "linear".into(),
                            workspace_id: workspace.into(),
                            kind: "project".into(),
                            external_id: "_revoked".into(),
                        },
                        Operation::AccessRevoked,
                        None,
                        None,
                    ),
                };
            changes.push(Change {
                subscription_id: c.subscription_id,
                generation: c.generation,
                sequence: c.sequence,
                event_id: c.event_id,
                resource_ref: reference,
                observed_revision: c.observed_revision,
                operation,
                projection,
                relation,
                schema_version: c.schema_version,
                observed_at: c.observed_at,
            });
        }
        Ok(ChangePage {
            generation: self.next_cursor.generation,
            changes,
            next_cursor: Some(serde_json::to_string(&self.next_cursor)?),
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_tachyon_producer_golden_hashes_match() {
        let json =
            include_str!("../tests/fixtures/tachyon-change-page.json");
        let page: Page = serde_json::from_str(json).unwrap();
        let workspace = match &page.changes[0].payload {
            Payload::Upsert { projection } => {
                projection.resource_ref.workspace_id.clone()
            }
            _ => panic!("golden first operation"),
        };
        let accepted = page.into_page(&workspace).unwrap();
        assert_eq!(accepted.changes.len(), 4);
        for change in &accepted.changes {
            change.validate().unwrap();
        }
        assert!(matches!(accepted.changes[0].operation, Operation::Upsert));
        assert!(matches!(
            accepted.changes[1].operation,
            Operation::Archived
        ));
        assert!(matches!(
            accepted.changes[2].operation,
            Operation::RelationChange
        ));
        assert!(matches!(
            accepted.changes[3].operation,
            Operation::AccessRevoked
        ));
    }
    #[test]
    fn accepts_tachyon_flattened_change_and_cursor() {
        let payload = Payload::AccessRevoked;
        let page = Page {
            changes: vec![ProjectionChange {
                subscription_id: "sub".into(),
                generation: 2,
                sequence: 3,
                event_id: "e".into(),
                observed_revision: 3,
                schema_version: 1,
                payload_hash: hash(&serde_json::to_vec(&payload).unwrap()),
                observed_at: "2026-10-24T00:00:00Z".into(),
                run_id: "r".into(),
                scope_revision: "scope".into(),
                payload,
            }],
            next_cursor: Cursor {
                generation: 2,
                sequence: 3,
            },
            has_more: false,
        };
        let json = serde_json::to_string(&page).unwrap();
        let decoded: Page = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            decoded.into_page("ws").unwrap().changes[0].operation,
            Operation::AccessRevoked
        ));
    }
}
