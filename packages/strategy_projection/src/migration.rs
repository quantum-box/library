//! Reviewed legacy adoption. No provider value is retained in the plan/journal.
use crate::{hash, Error, ResourceRef, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceAttestation {
    pub workspace_id: String,
    pub evidence: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    #[serde(default)]
    pub retire_only: bool,
    pub resource_ref: ResourceRef,
    pub endpoint_id: String,
    pub legacy_data_id: String,
    pub expected_record_version: String,
    /// Reviewed replacement for the provider-owned generic record title.
    pub local_name: String,
    pub clear_property_ids: Vec<String>,
    pub preserve_property_ids: Vec<String>,
    pub workspace_attestation: Option<WorkspaceAttestation>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub workspace_id: String,
    pub review_note: String,
    pub entries: Vec<Entry>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewedEntry {
    pub entry: Entry,
    pub schema_hash: String,
    pub operation_id: String,
    pub original_projection_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub repo_id: String,
    pub reviewed_by: String,
    pub digest: String,
    pub status: String,
    pub request: Request,
    pub entries: Vec<ReviewedEntry>,
}
#[derive(Debug, Clone, Serialize)]
pub struct PropertyClassification {
    pub id: String,
    pub relation: bool,
    pub type_key: String,
}
impl Request {
    pub fn validate(&self) -> Result<()> {
        if self.entries.is_empty()
            || self.entries.len() > 100
            || self.review_note.trim().len() < 10
        {
            return Err(Error::Invalid(
                "bounded entries and an explicit review note are required",
            ));
        }
        let mut resources = BTreeSet::new();
        let mut records = BTreeSet::new();
        for entry in &self.entries {
            if entry.retire_only && entry.resource_ref.kind == "issue" {
                if entry.resource_ref.provider != "linear"
                    || entry.resource_ref.external_id.is_empty()
                    || entry.resource_ref.workspace_id.is_empty()
                {
                    return Err(Error::Invalid(
                        "invalid legacy issue reference",
                    ));
                }
            } else {
                entry.resource_ref.validate()?;
            }
            if entry.resource_ref.workspace_id != self.workspace_id {
                return Err(Error::Invalid("workspace mismatch"));
            }
            if entry.legacy_data_id.is_empty()
                || entry.endpoint_id.is_empty()
                || entry.local_name.trim().is_empty()
                || entry.local_name.len() > 255
                || entry.expected_record_version.parse::<u64>().is_err()
            {
                return Err(Error::Invalid("invalid migration entry"));
            }
            if !resources.insert(entry.resource_ref.key())
                || !records.insert(&entry.legacy_data_id)
            {
                return Err(Error::Invalid(
                    "duplicate resource or legacy record",
                ));
            }
        }
        Ok(())
    }
}
pub fn validate_classifications(
    entry: &Entry,
    properties: &[PropertyClassification],
) -> Result<String> {
    let clear = entry
        .clear_property_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let preserve = entry
        .preserve_property_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if clear.len() != entry.clear_property_ids.len()
        || preserve.len() != entry.preserve_property_ids.len()
        || !clear.is_disjoint(&preserve)
    {
        return Err(Error::Invalid("duplicate property classification"));
    }
    let actual = properties
        .iter()
        .map(|p| p.id.clone())
        .collect::<BTreeSet<_>>();
    if clear.union(&preserve).cloned().collect::<BTreeSet<_>>() != actual {
        return Err(Error::Invalid(
            "every property must be explicitly classified",
        ));
    }
    if properties
        .iter()
        .any(|p| p.relation && clear.contains(&p.id))
    {
        return Err(Error::Invalid(
            "independent relations must be preserved",
        ));
    }
    Ok(hash(&serde_json::to_vec(properties)?))
}
pub fn verify_workspace(entry: &Entry, known: Option<&str>) -> Result<()> {
    if let Some(known) = known {
        if known != entry.resource_ref.workspace_id {
            return Err(Error::Invalid(
                "legacy workspace evidence contradicts mapping",
            ));
        }
        return Ok(());
    }
    let attestation=entry.workspace_attestation.as_ref().ok_or(Error::Invalid("unknown workspace provenance; explicit reviewed attestation required"))?;
    if attestation.workspace_id != entry.resource_ref.workspace_id
        || attestation.evidence.trim().len() < 10
    {
        return Err(Error::Invalid("invalid workspace evidence"));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn entry() -> Entry {
        Entry {
            retire_only: false,
            resource_ref: ResourceRef {
                provider: "linear".into(),
                workspace_id: "w".into(),
                kind: "project".into(),
                external_id: "p".into(),
            },
            endpoint_id: "e".into(),
            legacy_data_id: "d".into(),
            expected_record_version: "3".into(),
            local_name: "Local project notes".into(),
            clear_property_ids: vec!["provider".into()],
            preserve_property_ids: vec!["notes".into(), "relation".into()],
            workspace_attestation: None,
        }
    }
    #[test]
    fn unknown_provenance_and_conflicting_proof_are_rejected() {
        assert!(verify_workspace(&entry(), None).is_err());
        assert!(verify_workspace(&entry(), Some("other")).is_err());
        assert!(verify_workspace(&entry(), Some("w")).is_ok());
    }
    #[test]
    fn every_field_is_reviewed_and_relations_cannot_be_deleted() {
        let properties = vec![
            PropertyClassification {
                id: "provider".into(),
                relation: false,
                type_key: "string".into(),
            },
            PropertyClassification {
                id: "notes".into(),
                relation: false,
                type_key: "markdown".into(),
            },
            PropertyClassification {
                id: "relation".into(),
                relation: true,
                type_key: "relation".into(),
            },
        ];
        let mut e = entry();
        assert!(validate_classifications(&e, &properties).is_ok());
        e.preserve_property_ids.pop();
        assert!(validate_classifications(&e, &properties).is_err());
        e.clear_property_ids.push("relation".into());
        assert!(validate_classifications(&e, &properties).is_err());
    }
    #[test]
    fn duplicate_mapping_is_not_adoptable() {
        let e = entry();
        let p = Request {
            workspace_id: "w".into(),
            review_note: "reviewed actual workspace IDs".into(),
            entries: vec![e.clone(), e],
        };
        assert!(p.validate().is_err());
    }
}
