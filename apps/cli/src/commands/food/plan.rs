//! Compare the target records with what a draft repo holds now.
//!
//! The result is the list of upserts to send, and it only ever contains
//! the fields an import may touch:
//!
//! - `Source` fields are overwritten when they differ;
//! - `Derived` fields too, unless a person marked the record reviewed;
//! - `Human` fields (standard name, reading, aliases, review status,
//!   nutrient display settings) are only written when the record is new.
//!
//! Because the upsert API patches the properties it is sent and keeps the
//! rest, leaving a field out is how a person's edit is preserved.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::catalog::{prop, Owner, RecordKind, TargetRecord};

/// A record as read back from a draft repo, by property name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ExistingRecord {
    pub id: String,
    pub name: String,
    pub fields: BTreeMap<String, String>,
}

impl ExistingRecord {
    fn get(&self, property: &str) -> &str {
        self.fields.get(property).map(|s| s.trim()).unwrap_or("")
    }

    fn business_key(&self, kind: RecordKind) -> Option<String> {
        let key = match kind {
            RecordKind::Ingredient => {
                self.get(prop::INGREDIENT_KEY).to_string()
            }
            RecordKind::Nutrient => {
                self.get(prop::NUTRIENT_KEY).to_string()
            }
            RecordKind::Value => {
                let (i, n) = (
                    self.get(prop::INGREDIENT_KEY),
                    self.get(prop::NUTRIENT_KEY),
                );
                if i.is_empty() || n.is_empty() {
                    String::new()
                } else {
                    format!("{i}/{n}")
                }
            }
        };
        (!key.is_empty()).then_some(key)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    Create,
    Update { changed: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlannedWrite {
    pub kind: RecordKind,
    pub data_id: String,
    pub business_key: String,
    pub name: String,
    /// Property name → value to send. `""` clears a source field that the
    /// source no longer prints.
    pub properties: BTreeMap<String, String>,
    #[serde(flatten)]
    pub action: Action,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KeyConflict {
    pub business_key: String,
    pub data_id: String,
    pub existing_ids: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeleteCandidate {
    pub id: String,
    pub name: String,
    pub business_key: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct RepoPlan {
    pub writes: Vec<PlannedWrite>,
    pub unchanged: usize,
    pub conflicts: Vec<KeyConflict>,
    /// Records in the repo that this import does not produce. Never
    /// deleted by the importer; listed for a person to decide.
    pub delete_candidates: Vec<DeleteCandidate>,
    /// Human-owned fields left alone on existing records, by property.
    pub preserved_human_fields: BTreeMap<String, usize>,
    /// Derived fields left alone because the record is reviewed.
    pub preserved_reviewed_fields: BTreeMap<String, usize>,
}

impl RepoPlan {
    pub fn creates(&self) -> usize {
        self.writes
            .iter()
            .filter(|w| w.action == Action::Create)
            .count()
    }

    pub fn updates(&self) -> usize {
        self.writes.len() - self.creates()
    }
}

/// `skip_properties`: properties the repo does not have (optional ones
/// only; required ones are checked before planning).
pub fn plan_repo(
    kind: RecordKind,
    targets: &[TargetRecord],
    existing: &[ExistingRecord],
    withheld_keys: &BTreeSet<String>,
    skip_properties: &BTreeSet<String>,
) -> RepoPlan {
    let mut plan = RepoPlan::default();
    let by_id: BTreeMap<&str, &ExistingRecord> =
        existing.iter().map(|e| (e.id.as_str(), e)).collect();
    let mut by_key: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for e in existing {
        if let Some(k) = e.business_key(kind) {
            by_key.entry(k).or_default().push(e.id.as_str());
        }
    }
    let target_ids: BTreeSet<&str> =
        targets.iter().map(|t| t.data_id.as_str()).collect();

    for t in targets {
        let others: Vec<String> = by_key
            .get(&t.business_key)
            .into_iter()
            .flatten()
            .filter(|id| **id != t.data_id)
            .map(|id| id.to_string())
            .collect();
        if !others.is_empty() {
            plan.conflicts.push(KeyConflict {
                business_key: t.business_key.clone(),
                data_id: t.data_id.clone(),
                existing_ids: others,
                reason: "the key already exists on a record the importer did not create; merge by hand".into(),
            });
            continue;
        }

        let fields = t
            .fields
            .iter()
            .filter(|f| !skip_properties.contains(f.property));

        match by_id.get(t.data_id.as_str()) {
            None => {
                let properties = fields
                    .filter_map(|f| {
                        f.value.clone().map(|v| (f.property.to_string(), v))
                    })
                    .collect();
                plan.writes.push(PlannedWrite {
                    kind,
                    data_id: t.data_id.clone(),
                    business_key: t.business_key.clone(),
                    name: t.name.clone(),
                    properties,
                    action: Action::Create,
                });
            }
            Some(e) => {
                let reviewed =
                    e.get(prop::ATTRIBUTE_REVIEW_STATUS) == "reviewed";
                let mut properties = BTreeMap::new();
                let mut changed = Vec::new();
                for f in fields {
                    let new = f.value.as_deref().unwrap_or("").trim();
                    let differs = e.get(f.property) != new;
                    match f.owner {
                        Owner::Human => {
                            if differs {
                                *plan
                                    .preserved_human_fields
                                    .entry(f.property.to_string())
                                    .or_default() += 1;
                            }
                            continue;
                        }
                        Owner::Derived if reviewed => {
                            if differs {
                                *plan
                                    .preserved_reviewed_fields
                                    .entry(f.property.to_string())
                                    .or_default() += 1;
                            }
                            continue;
                        }
                        Owner::Derived | Owner::Source => {}
                    }
                    if differs {
                        properties.insert(
                            f.property.to_string(),
                            new.to_string(),
                        );
                        changed.push(f.property.to_string());
                    }
                }
                let name = match t.name_owner {
                    Owner::Source => {
                        if e.name != t.name {
                            changed.insert(0, "name".into());
                        }
                        t.name.clone()
                    }
                    // Upsert always sets a name; send back the one there.
                    _ => e.name.clone(),
                };
                if changed.is_empty() {
                    plan.unchanged += 1;
                } else {
                    plan.writes.push(PlannedWrite {
                        kind,
                        data_id: t.data_id.clone(),
                        business_key: t.business_key.clone(),
                        name,
                        properties,
                        action: Action::Update { changed },
                    });
                }
            }
        }
    }

    for e in existing {
        if target_ids.contains(e.id.as_str()) {
            continue;
        }
        let key = e.business_key(kind);
        let withheld = key.as_ref().is_some_and(|k| {
            let ingredient = k.split('/').next().unwrap_or(k);
            withheld_keys.contains(ingredient)
        });
        // Records the importer could not rebuild this run (quarantined)
        // are kept, not offered for deletion.
        if withheld
            || plan
                .conflicts
                .iter()
                .any(|c| c.existing_ids.contains(&e.id))
        {
            continue;
        }
        plan.delete_candidates.push(DeleteCandidate {
            id: e.id.clone(),
            name: e.name.clone(),
            business_key: key,
        });
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::super::catalog::tests::fixture_catalog;
    use super::*;

    /// What a repo would hold right after an import of `t`.
    fn stored(t: &TargetRecord) -> ExistingRecord {
        ExistingRecord {
            id: t.data_id.clone(),
            name: t.name.clone(),
            fields: t
                .fields
                .iter()
                .filter_map(|f| {
                    f.value.clone().map(|v| (f.property.to_string(), v))
                })
                .collect(),
        }
    }

    fn no_skip() -> BTreeSet<String> {
        BTreeSet::new()
    }

    #[test]
    fn an_empty_repo_gets_every_record_created() {
        let c = fixture_catalog();
        let plan = plan_repo(
            RecordKind::Value,
            &c.values,
            &[],
            &c.withheld_keys,
            &no_skip(),
        );
        assert_eq!(plan.creates(), c.values.len());
        assert_eq!(plan.updates(), 0);
        let tr = plan
            .writes
            .iter()
            .find(|w| w.business_key == "mext-06154/VITK")
            .unwrap();
        assert_eq!(
            tr.properties.get("value_status").map(String::as_str),
            Some("trace")
        );
        assert!(
            !tr.properties.contains_key("amount"),
            "Tr must not carry an amount"
        );
    }

    #[test]
    fn a_second_run_on_the_same_data_writes_nothing() {
        let c = fixture_catalog();
        let existing: Vec<_> = c.ingredients.iter().map(stored).collect();
        let plan = plan_repo(
            RecordKind::Ingredient,
            &c.ingredients,
            &existing,
            &c.withheld_keys,
            &no_skip(),
        );
        assert!(plan.writes.is_empty(), "{:?}", plan.writes);
        assert_eq!(plan.unchanged, c.ingredients.len());
        assert!(plan.delete_candidates.is_empty());
    }

    #[test]
    fn human_fields_survive_and_source_fields_are_corrected() {
        let c = fixture_catalog();
        let mut existing: Vec<_> =
            c.ingredients.iter().map(stored).collect();
        let onion = existing
            .iter_mut()
            .find(|e| e.fields["ingredient_key"] == "mext-06153")
            .unwrap();
        onion
            .fields
            .insert("standard_name".into(), "たまねぎ".into());
        onion.fields.insert("reading".into(), "たまねぎ".into());
        onion
            .fields
            .insert("aliases".into(), "玉ねぎ\nオニオン\n新玉".into());
        onion
            .fields
            .insert("attribute_review_status".into(), "reviewed".into());
        onion.fields.insert("cooking_state".into(), "fresh".into());
        onion.fields.insert("refuse_rate".into(), "7".into());

        let plan = plan_repo(
            RecordKind::Ingredient,
            &c.ingredients,
            &existing,
            &c.withheld_keys,
            &no_skip(),
        );
        assert_eq!(plan.writes.len(), 1);
        let w = &plan.writes[0];
        assert_eq!(
            w.action,
            Action::Update {
                changed: vec!["refuse_rate".into()]
            }
        );
        assert_eq!(w.properties.len(), 1);
        assert_eq!(w.properties["refuse_rate"], "6");
        for human in [
            "standard_name",
            "reading",
            "aliases",
            "attribute_review_status",
        ] {
            assert!(
                !w.properties.contains_key(human),
                "{human} must not be sent"
            );
        }
        // Reviewed record: the person's cooking_state stays.
        assert!(!w.properties.contains_key("cooking_state"));
        assert_eq!(
            plan.preserved_reviewed_fields.get("cooking_state"),
            Some(&1)
        );
        assert!(plan.preserved_human_fields.contains_key("aliases"));
    }

    #[test]
    fn derived_fields_follow_the_source_until_reviewed() {
        let c = fixture_catalog();
        let mut existing: Vec<_> =
            c.ingredients.iter().map(stored).collect();
        existing[0]
            .fields
            .insert("cooking_state".into(), "boiled".into());
        let plan = plan_repo(
            RecordKind::Ingredient,
            &c.ingredients,
            &existing,
            &c.withheld_keys,
            &no_skip(),
        );
        assert_eq!(plan.writes.len(), 1);
        assert!(plan.writes[0].properties.contains_key("cooking_state"));
    }

    #[test]
    fn a_nutrient_keeps_its_edited_name_and_display_choice() {
        let c = fixture_catalog();
        let mut existing: Vec<_> = c.nutrients.iter().map(stored).collect();
        existing[0].name = "エネルギー".into();
        existing[0]
            .fields
            .insert("default_display".into(), "true".into());
        existing[0].fields.insert("unit".into(), "kcal".into());
        let plan = plan_repo(
            RecordKind::Nutrient,
            &c.nutrients,
            &existing,
            &c.withheld_keys,
            &no_skip(),
        );
        assert_eq!(plan.writes.len(), 1);
        let w = &plan.writes[0];
        assert_eq!(
            w.name, "エネルギー",
            "the edited name is sent back as is"
        );
        assert_eq!(w.properties.keys().collect::<Vec<_>>(), vec!["unit"]);
    }

    #[test]
    fn a_key_held_by_another_record_is_a_conflict_not_a_duplicate() {
        let c = fixture_catalog();
        let mut manual = stored(&c.ingredients[0]);
        manual.id = "data_01manualmanualmanualmanu".into();
        let plan = plan_repo(
            RecordKind::Ingredient,
            &c.ingredients,
            &[manual],
            &c.withheld_keys,
            &no_skip(),
        );
        assert_eq!(plan.conflicts.len(), 1);
        assert_eq!(plan.creates(), c.ingredients.len() - 1);
        assert!(plan.delete_candidates.is_empty());
    }

    #[test]
    fn stale_records_are_listed_but_withheld_ones_are_kept() {
        let c = fixture_catalog();
        let stale = ExistingRecord {
            id: "data_01stalestalestalestalesta".into(),
            name: "old".into(),
            fields: [(
                "ingredient_key".to_string(),
                "mext-99990".to_string(),
            )]
            .into(),
        };
        let withheld = ExistingRecord {
            id: "data_01keptkeptkeptkeptkeptk".into(),
            name: "quarantined".into(),
            fields: [(
                "ingredient_key".to_string(),
                "mext-18001".to_string(),
            )]
            .into(),
        };
        let plan = plan_repo(
            RecordKind::Ingredient,
            &c.ingredients,
            &[stale, withheld],
            &c.withheld_keys,
            &no_skip(),
        );
        let ids: Vec<_> = plan
            .delete_candidates
            .iter()
            .map(|d| d.id.as_str())
            .collect();
        assert_eq!(ids, vec!["data_01stalestalestalestalesta"]);
    }

    #[test]
    fn optional_properties_missing_from_the_repo_are_left_out() {
        let c = fixture_catalog();
        let skip: BTreeSet<String> = ["remarks".to_string()].into();
        let plan = plan_repo(
            RecordKind::Ingredient,
            &c.ingredients,
            &[],
            &c.withheld_keys,
            &skip,
        );
        assert!(plan
            .writes
            .iter()
            .all(|w| !w.properties.contains_key("remarks")));
    }
}
