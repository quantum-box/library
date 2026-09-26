//! COM-860: turn draft records into a validated, immutable release snapshot.
//!
//! Drafts are the editable Library records (ingredients, nutrient
//! definitions and values). Publishing validates all of them together and
//! freezes the result. After that, edits to the drafts cannot change what a
//! published release returns.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{NormalizedDecimal, NutrientValueStatus};

/// Version of the snapshot layout. Bump it when a field's meaning
/// changes, so hashes from different layouts are never compared.
pub const SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// Report at most this many validation errors, so one broken import does
/// not produce a response the size of the whole table.
const MAX_REPORTED_ERRORS: usize = 50;
const MAX_ALIASES: usize = 64;
const MAX_TEXT_LEN: usize = 255;

/// One ingredient record as read from the draft repo.
#[derive(Debug, Clone, Default)]
pub struct DraftIngredient {
    /// Where the draft came from (Library data ID), for error messages.
    pub record_ref: String,
    pub ingredient_key: String,
    pub source_food_code: String,
    pub original_name: String,
    pub standard_name: Option<String>,
    pub reading: Option<String>,
    /// One alias per line.
    pub aliases: Option<String>,
    pub category_code: Option<String>,
    pub category_name: Option<String>,
    pub part: Option<String>,
    pub cooking_state: Option<String>,
    pub skin_bone: Option<String>,
    pub refuse_rate: Option<String>,
    pub attribute_review_status: Option<String>,
}

/// One nutrient definition as read from the draft repo.
#[derive(Debug, Clone, Default)]
pub struct DraftNutrient {
    pub record_ref: String,
    pub nutrient_key: String,
    pub name: String,
    pub unit: String,
    pub basis: String,
    pub method: Option<String>,
    pub display_order: Option<String>,
    pub default_display: Option<String>,
}

/// One ingredient × nutrient value as read from the draft repo.
#[derive(Debug, Clone, Default)]
pub struct DraftValue {
    pub record_ref: String,
    pub ingredient_key: String,
    pub nutrient_key: String,
    pub value_status: String,
    pub amount: Option<String>,
    pub raw_notation: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct DraftCatalog {
    pub ingredients: Vec<DraftIngredient>,
    pub nutrients: Vec<DraftNutrient>,
    pub values: Vec<DraftValue>,
}

/// Whether parsed attributes (standard name, part, state, aliases) were
/// checked by a person. Source values are always as published.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AttributeReviewStatus {
    Unreviewed,
    Reviewed,
}

impl AttributeReviewStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unreviewed => "unreviewed",
            Self::Reviewed => "reviewed",
        }
    }

    pub fn parse(raw: &str) -> errors::Result<Self> {
        match raw.trim() {
            "unreviewed" => Ok(Self::Unreviewed),
            "reviewed" => Ok(Self::Reviewed),
            other => Err(errors::Error::invalid(format!(
                "unknown attribute_review_status: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReleasedIngredient {
    pub ingredient_key: String,
    pub source_food_code: String,
    pub original_name: String,
    pub standard_name: Option<String>,
    pub reading: Option<String>,
    pub aliases: Vec<String>,
    pub category_code: Option<String>,
    pub category_name: Option<String>,
    pub part: Option<String>,
    pub cooking_state: Option<String>,
    pub skin_bone: Option<String>,
    #[serde(serialize_with = "serialize_decimal_opt")]
    pub refuse_rate: Option<NormalizedDecimal>,
    pub attribute_review_status: AttributeReviewStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReleasedNutrient {
    pub nutrient_key: String,
    pub name: String,
    pub unit: String,
    pub basis: String,
    pub method: Option<String>,
    pub display_order: i32,
    pub default_display: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReleasedValue {
    pub ingredient_key: String,
    pub nutrient_key: String,
    #[serde(serialize_with = "serialize_status")]
    pub status: NutrientValueStatus,
    #[serde(serialize_with = "serialize_decimal_opt")]
    pub amount: Option<NormalizedDecimal>,
    pub raw_notation: Option<String>,
}

/// A validated release. Every list is sorted by key, so the same drafts
/// always produce the same snapshot and hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseSnapshot {
    ingredients: Vec<ReleasedIngredient>,
    nutrients: Vec<ReleasedNutrient>,
    values: Vec<ReleasedValue>,
    content_hash: String,
}

#[derive(Serialize)]
struct HashedContent<'a> {
    schema_version: u32,
    ingredients: &'a [ReleasedIngredient],
    nutrients: &'a [ReleasedNutrient],
    values: &'a [ReleasedValue],
}

fn serialize_decimal_opt<S: serde::Serializer>(
    value: &Option<NormalizedDecimal>,
    s: S,
) -> Result<S::Ok, S::Error> {
    match value {
        Some(v) => s.serialize_some(v.as_str()),
        None => s.serialize_none(),
    }
}

fn serialize_status<S: serde::Serializer>(
    value: &NutrientValueStatus,
    s: S,
) -> Result<S::Ok, S::Error> {
    s.serialize_str(value.as_str())
}

impl ReleaseSnapshot {
    pub fn ingredients(&self) -> &[ReleasedIngredient] {
        &self.ingredients
    }

    pub fn nutrients(&self) -> &[ReleasedNutrient] {
        &self.nutrients
    }

    pub fn values(&self) -> &[ReleasedValue] {
        &self.values
    }

    /// `sha256:<hex>` over the canonical JSON of the snapshot.
    pub fn content_hash(&self) -> &str {
        &self.content_hash
    }

    /// Recompute the hash of stored rows, e.g. to check that a published
    /// release was not modified after it was written.
    pub fn compute_hash(
        schema_version: u32,
        ingredients: &[ReleasedIngredient],
        nutrients: &[ReleasedNutrient],
        values: &[ReleasedValue],
    ) -> String {
        let content = HashedContent {
            schema_version,
            ingredients,
            nutrients,
            values,
        };
        let json = serde_json::to_vec(&content)
            .expect("snapshot content always serializes");
        format!("sha256:{:x}", Sha256::digest(json))
    }

    /// Validate every draft and build the snapshot, or report what is
    /// wrong. Nothing is fixed up silently: a value that cannot be read
    /// fails the whole release.
    pub fn build(draft: &DraftCatalog) -> errors::Result<Self> {
        let mut errs = Errors::default();

        let mut nutrients = BTreeMap::new();
        for d in &draft.nutrients {
            if let Some(n) =
                errs.collect(&d.record_ref, validate_nutrient(d))
            {
                if nutrients.contains_key(&n.nutrient_key) {
                    errs.push(
                        &d.record_ref,
                        format!(
                            "duplicate nutrient_key {}",
                            n.nutrient_key
                        ),
                    );
                } else {
                    nutrients.insert(n.nutrient_key.clone(), n);
                }
            }
        }

        let mut ingredients = BTreeMap::new();
        let mut food_codes = BTreeSet::new();
        for d in &draft.ingredients {
            if let Some(i) =
                errs.collect(&d.record_ref, validate_ingredient(d))
            {
                if ingredients.contains_key(&i.ingredient_key) {
                    errs.push(
                        &d.record_ref,
                        format!(
                            "duplicate ingredient_key {}",
                            i.ingredient_key
                        ),
                    );
                } else if !food_codes.insert(i.source_food_code.clone()) {
                    errs.push(
                        &d.record_ref,
                        format!(
                            "duplicate source_food_code {}",
                            i.source_food_code
                        ),
                    );
                } else {
                    ingredients.insert(i.ingredient_key.clone(), i);
                }
            }
        }

        let mut values = BTreeMap::new();
        for d in &draft.values {
            let Some(v) = errs.collect(&d.record_ref, validate_value(d))
            else {
                continue;
            };
            if !ingredients.contains_key(&v.ingredient_key) {
                errs.push(
                    &d.record_ref,
                    format!("unknown ingredient_key {}", v.ingredient_key),
                );
                continue;
            }
            if !nutrients.contains_key(&v.nutrient_key) {
                errs.push(
                    &d.record_ref,
                    format!("unknown nutrient_key {}", v.nutrient_key),
                );
                continue;
            }
            let key = (v.ingredient_key.clone(), v.nutrient_key.clone());
            if values.contains_key(&key) {
                errs.push(
                    &d.record_ref,
                    format!(
                        "duplicate value for {} / {}",
                        v.ingredient_key, v.nutrient_key
                    ),
                );
                continue;
            }
            values.insert(key, v);
        }

        if ingredients.is_empty() && errs.is_empty() {
            errs.push("catalog", "release has no ingredients".to_string());
        }
        if nutrients.is_empty() && errs.is_empty() {
            errs.push("catalog", "release has no nutrients".to_string());
        }
        errs.into_result()?;

        let ingredients: Vec<_> = ingredients.into_values().collect();
        let nutrients: Vec<_> = nutrients.into_values().collect();
        let values: Vec<_> = values.into_values().collect();
        let content_hash = Self::compute_hash(
            SNAPSHOT_SCHEMA_VERSION,
            &ingredients,
            &nutrients,
            &values,
        );

        Ok(Self {
            ingredients,
            nutrients,
            values,
            content_hash,
        })
    }
}

#[derive(Default)]
struct Errors {
    messages: Vec<String>,
    total: usize,
}

impl Errors {
    fn push(&mut self, record_ref: &str, message: String) {
        self.total += 1;
        if self.messages.len() < MAX_REPORTED_ERRORS {
            self.messages.push(format!("{record_ref}: {message}"));
        }
    }

    fn collect<T>(
        &mut self,
        record_ref: &str,
        result: errors::Result<T>,
    ) -> Option<T> {
        match result {
            Ok(v) => Some(v),
            Err(e) => {
                self.push(record_ref, e.to_string());
                None
            }
        }
    }

    fn is_empty(&self) -> bool {
        self.total == 0
    }

    fn into_result(self) -> errors::Result<()> {
        if self.total == 0 {
            return Ok(());
        }
        let omitted = self.total - self.messages.len();
        let mut message = format!(
            "ingredient release validation failed ({} errors): {}",
            self.total,
            self.messages.join("; ")
        );
        if omitted > 0 {
            message.push_str(&format!("; and {omitted} more"));
        }
        Err(errors::Error::invalid(message))
    }
}

/// Stable identifiers: letters, digits, `.`, `_` and `-`, at most 64.
fn validate_key(field: &str, raw: &str) -> errors::Result<String> {
    let s = raw.trim();
    let valid = !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b));
    if !valid {
        return Err(errors::Error::invalid(format!(
            "{field} must be 1-64 characters of [A-Za-z0-9._-]: {raw:?}"
        )));
    }
    Ok(s.to_string())
}

fn required_text(field: &str, raw: &str) -> errors::Result<String> {
    optional_text(field, Some(raw))?.ok_or_else(|| {
        errors::Error::invalid(format!("{field} is required"))
    })
}

fn optional_text(
    field: &str,
    raw: Option<&str>,
) -> errors::Result<Option<String>> {
    let Some(s) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    if s.chars().count() > MAX_TEXT_LEN {
        return Err(errors::Error::invalid(format!(
            "{field} is longer than {MAX_TEXT_LEN} characters"
        )));
    }
    Ok(Some(s.to_string()))
}

/// Cooking or processing state code, e.g. `raw`, `boiled`, `dried`.
fn optional_state(raw: Option<&str>) -> errors::Result<Option<String>> {
    let Some(s) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let valid = s.len() <= 32
        && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_');
    if !valid {
        return Err(errors::Error::invalid(format!(
            "cooking_state must be 1-32 characters of [a-z_]: {s:?}"
        )));
    }
    Ok(Some(s.to_string()))
}

fn validate_ingredient(
    d: &DraftIngredient,
) -> errors::Result<ReleasedIngredient> {
    // Food numbers keep their leading zeros, so they stay text.
    let source_food_code =
        validate_key("source_food_code", &d.source_food_code)?;

    let mut aliases: Vec<String> = d
        .aliases
        .as_deref()
        .unwrap_or("")
        .lines()
        .map(|l| optional_text("alias", Some(l)))
        .collect::<errors::Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();
    aliases.sort();
    aliases.dedup();
    if aliases.len() > MAX_ALIASES {
        return Err(errors::Error::invalid(format!(
            "more than {MAX_ALIASES} aliases"
        )));
    }

    Ok(ReleasedIngredient {
        ingredient_key: validate_key("ingredient_key", &d.ingredient_key)?,
        source_food_code,
        original_name: required_text("original_name", &d.original_name)?,
        standard_name: optional_text(
            "standard_name",
            d.standard_name.as_deref(),
        )?,
        reading: optional_text("reading", d.reading.as_deref())?,
        aliases,
        category_code: optional_text(
            "category_code",
            d.category_code.as_deref(),
        )?,
        category_name: optional_text(
            "category_name",
            d.category_name.as_deref(),
        )?,
        part: optional_text("part", d.part.as_deref())?,
        cooking_state: optional_state(d.cooking_state.as_deref())?,
        skin_bone: optional_text("skin_bone", d.skin_bone.as_deref())?,
        refuse_rate: d
            .refuse_rate
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(NormalizedDecimal::parse_percentage)
            .transpose()?,
        attribute_review_status: match d
            .attribute_review_status
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            Some(s) => AttributeReviewStatus::parse(s)?,
            None => AttributeReviewStatus::Unreviewed,
        },
    })
}

fn validate_nutrient(
    d: &DraftNutrient,
) -> errors::Result<ReleasedNutrient> {
    let display_order = match d
        .display_order
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(s) => s.parse::<i32>().map_err(|_| {
            errors::Error::invalid(format!(
                "display_order must be an integer: {s:?}"
            ))
        })?,
        None => 0,
    };
    let default_display = match d
        .default_display
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some("true") => true,
        Some("false") | None => false,
        Some(other) => {
            return Err(errors::Error::invalid(format!(
                "default_display must be true or false: {other:?}"
            )))
        }
    };

    Ok(ReleasedNutrient {
        nutrient_key: validate_key("nutrient_key", &d.nutrient_key)?,
        name: required_text("name", &d.name)?,
        unit: required_text("unit", &d.unit)?,
        basis: required_text("basis", &d.basis)?,
        method: optional_text("method", d.method.as_deref())?,
        display_order,
        default_display,
    })
}

fn validate_value(d: &DraftValue) -> errors::Result<ReleasedValue> {
    let status: NutrientValueStatus = d.value_status.parse()?;
    let amount = status.validate_amount(d.amount.as_deref())?;
    Ok(ReleasedValue {
        ingredient_key: validate_key("ingredient_key", &d.ingredient_key)?,
        nutrient_key: validate_key("nutrient_key", &d.nutrient_key)?,
        status,
        amount,
        raw_notation: optional_text(
            "raw_notation",
            d.raw_notation.as_deref(),
        )?,
    })
}

#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;

    pub fn ingredient(
        key: &str,
        code: &str,
        name: &str,
    ) -> DraftIngredient {
        DraftIngredient {
            record_ref: format!("data:{key}"),
            ingredient_key: key.into(),
            source_food_code: code.into(),
            original_name: name.into(),
            ..Default::default()
        }
    }

    pub fn nutrient(key: &str, unit: &str) -> DraftNutrient {
        DraftNutrient {
            record_ref: format!("data:{key}"),
            nutrient_key: key.into(),
            name: key.into(),
            unit: unit.into(),
            basis: "可食部100g当たり".into(),
            ..Default::default()
        }
    }

    pub fn value(
        ingredient: &str,
        nutrient: &str,
        status: &str,
        amount: Option<&str>,
    ) -> DraftValue {
        DraftValue {
            record_ref: format!("data:{ingredient}/{nutrient}"),
            ingredient_key: ingredient.into(),
            nutrient_key: nutrient.into(),
            value_status: status.into(),
            amount: amount.map(Into::into),
            raw_notation: None,
        }
    }

    /// Two onions that differ only by state, plus the edge-case values
    /// the acceptance criteria call out.
    pub fn onion_catalog() -> DraftCatalog {
        let mut raw = ingredient(
            "onion-raw",
            "06153",
            "＜野菜類＞たまねぎ類 たまねぎ りん茎 生",
        );
        raw.standard_name = Some("たまねぎ".into());
        raw.aliases = Some("玉ねぎ\nオニオン\n玉ねぎ\n".into());
        raw.cooking_state = Some("raw".into());
        raw.refuse_rate = Some("6".into());
        let mut boiled = ingredient(
            "onion-boiled",
            "06154",
            "＜野菜類＞たまねぎ類 たまねぎ りん茎 ゆで",
        );
        boiled.standard_name = Some("たまねぎ".into());
        boiled.aliases = Some("玉ねぎ".into());
        boiled.cooking_state = Some("boiled".into());

        DraftCatalog {
            ingredients: vec![boiled, raw],
            nutrients: vec![
                nutrient("ENERC_KCAL", "kcal"),
                nutrient("PROT-", "g"),
                nutrient("NA", "mg"),
                nutrient("VITD", "µg"),
            ],
            values: vec![
                value("onion-raw", "ENERC_KCAL", "measured", Some("33")),
                value("onion-raw", "PROT-", "measured", Some("1.0")),
                value("onion-raw", "NA", "zero", Some("0")),
                value("onion-raw", "VITD", "not_measured", None),
                value("onion-boiled", "PROT-", "estimated", Some("0.10")),
                value("onion-boiled", "NA", "trace", None),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;

    #[test]
    fn builds_a_sorted_snapshot_with_normalized_values() {
        let snapshot = ReleaseSnapshot::build(&onion_catalog()).unwrap();

        let keys: Vec<_> = snapshot
            .ingredients()
            .iter()
            .map(|i| i.ingredient_key.as_str())
            .collect();
        assert_eq!(keys, ["onion-boiled", "onion-raw"]);

        let raw = &snapshot.ingredients()[1];
        assert_eq!(raw.source_food_code, "06153", "leading zero kept");
        assert_eq!(
            raw.aliases,
            ["オニオン", "玉ねぎ"],
            "sorted and deduped"
        );
        assert_eq!(raw.refuse_rate.as_ref().unwrap().as_str(), "6");

        let find = |i: &str, n: &str| {
            snapshot
                .values()
                .iter()
                .find(|v| v.ingredient_key == i && v.nutrient_key == n)
                .unwrap()
        };
        assert_eq!(
            find("onion-raw", "PROT-").amount.as_ref().unwrap().as_str(),
            "1"
        );
        assert_eq!(
            find("onion-boiled", "PROT-")
                .amount
                .as_ref()
                .unwrap()
                .as_str(),
            "0.1"
        );
        assert_eq!(
            find("onion-raw", "NA").status,
            NutrientValueStatus::Zero
        );
        assert_eq!(
            find("onion-boiled", "NA").status,
            NutrientValueStatus::Trace
        );
        assert_eq!(
            find("onion-boiled", "NA").amount,
            None,
            "trace is not 0"
        );
        assert_eq!(
            find("onion-raw", "VITD").amount,
            None,
            "not measured is not 0"
        );
    }

    #[test]
    fn hash_ignores_draft_order_but_tracks_content() {
        let a = ReleaseSnapshot::build(&onion_catalog()).unwrap();

        let mut reordered = onion_catalog();
        reordered.ingredients.reverse();
        reordered.values.reverse();
        let b = ReleaseSnapshot::build(&reordered).unwrap();
        assert_eq!(a.content_hash(), b.content_hash());
        assert!(a.content_hash().starts_with("sha256:"));

        let mut edited = onion_catalog();
        edited.values[1].amount = Some("1.1".into());
        let c = ReleaseSnapshot::build(&edited).unwrap();
        assert_ne!(a.content_hash(), c.content_hash());
    }

    #[test]
    fn same_value_in_different_notation_hashes_the_same() {
        let a = ReleaseSnapshot::build(&onion_catalog()).unwrap();
        let mut padded = onion_catalog();
        padded.values[1].amount = Some("01.00".into());
        let b = ReleaseSnapshot::build(&padded).unwrap();
        assert_eq!(a.content_hash(), b.content_hash());
    }

    #[test]
    fn compute_hash_matches_build() {
        let s = ReleaseSnapshot::build(&onion_catalog()).unwrap();
        assert_eq!(
            ReleaseSnapshot::compute_hash(
                SNAPSHOT_SCHEMA_VERSION,
                s.ingredients(),
                s.nutrients(),
                s.values()
            ),
            s.content_hash()
        );
    }

    #[test]
    fn rejects_dangling_and_duplicate_references() {
        let mut draft = onion_catalog();
        draft
            .values
            .push(value("leek", "PROT-", "measured", Some("1")));
        draft
            .values
            .push(value("onion-raw", "FAT", "measured", Some("1")));
        draft.values.push(value(
            "onion-raw",
            "PROT-",
            "measured",
            Some("2"),
        ));
        draft
            .ingredients
            .push(ingredient("onion-raw", "99999", "dup key"));
        draft
            .ingredients
            .push(ingredient("other", "06153", "dup code"));

        let err = ReleaseSnapshot::build(&draft).unwrap_err().to_string();
        assert!(err.contains("5 errors"), "{err}");
        assert!(err.contains("unknown ingredient_key leek"), "{err}");
        assert!(err.contains("unknown nutrient_key FAT"), "{err}");
        assert!(
            err.contains("duplicate value for onion-raw / PROT-"),
            "{err}"
        );
        assert!(
            err.contains("duplicate ingredient_key onion-raw"),
            "{err}"
        );
        assert!(err.contains("duplicate source_food_code 06153"), "{err}");
    }

    #[test]
    fn rejects_values_that_would_hide_their_meaning() {
        for (status, amount) in [
            ("trace", Some("0")),
            ("not_measured", Some("0")),
            ("measured", None),
            ("measured", Some("Tr")),
            ("zero", Some("0.1")),
            ("not_listed", None),
            ("unknown", Some("1")),
        ] {
            let mut draft = onion_catalog();
            draft.values[0].value_status = status.into();
            draft.values[0].amount = amount.map(Into::into);
            assert!(
                ReleaseSnapshot::build(&draft).is_err(),
                "{status} {amount:?} should be rejected"
            );
        }
    }

    #[test]
    fn rejects_bad_ingredient_fields() {
        let cases: Vec<fn(&mut DraftIngredient)> = vec![
            |i| i.ingredient_key = "has space".into(),
            |i| i.source_food_code = "".into(),
            |i| i.original_name = " ".into(),
            |i| i.cooking_state = Some("Raw".into()),
            |i| i.refuse_rate = Some("101".into()),
            |i| i.attribute_review_status = Some("approved".into()),
        ];
        for mutate in cases {
            let mut draft = onion_catalog();
            mutate(&mut draft.ingredients[0]);
            assert!(ReleaseSnapshot::build(&draft).is_err());
        }
    }

    #[test]
    fn rejects_an_empty_release() {
        assert!(ReleaseSnapshot::build(&DraftCatalog::default()).is_err());
    }

    #[test]
    fn caps_the_error_report() {
        let mut draft = onion_catalog();
        for n in 0..(MAX_REPORTED_ERRORS + 5) {
            draft.values.push(value(
                &format!("missing-{n}"),
                "NA",
                "trace",
                None,
            ));
        }
        let err = ReleaseSnapshot::build(&draft).unwrap_err().to_string();
        assert!(err.contains("and 5 more"), "{err}");
    }
}
