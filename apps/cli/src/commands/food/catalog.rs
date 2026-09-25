//! Turn the parsed (and corrected) table into COM-860 draft records.
//!
//! Every record gets a data ID derived from its business key, so running
//! the import again addresses the same records instead of adding copies.
//! Each field is tagged with who owns it, which decides whether a later
//! import may overwrite what a person changed.

use std::collections::{BTreeMap, BTreeSet};

use ingredient_notation::{
    validate_cooking_state, validate_key, NormalizedDecimal,
    NutrientValueStatus,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::errata::{strip_footnotes, ErrataOutcome, ErrataStatus};
use super::table::{expected_unit, SourceFood, SourceTable};

/// Property names, as in `apps/api/src/usecase/ingredient_catalog.rs`
/// `draft_schema`.
pub mod prop {
    pub const INGREDIENT_KEY: &str = "ingredient_key";
    pub const SOURCE_FOOD_CODE: &str = "source_food_code";
    pub const STANDARD_NAME: &str = "standard_name";
    pub const READING: &str = "reading";
    pub const ALIASES: &str = "aliases";
    pub const CATEGORY_CODE: &str = "category_code";
    pub const CATEGORY_NAME: &str = "category_name";
    pub const PART: &str = "part";
    pub const COOKING_STATE: &str = "cooking_state";
    pub const SKIN_BONE: &str = "skin_bone";
    pub const REFUSE_RATE: &str = "refuse_rate";
    pub const ATTRIBUTE_REVIEW_STATUS: &str = "attribute_review_status";
    /// Not read by publishing. Written only when the repo has it.
    pub const REMARKS: &str = "remarks";

    pub const NUTRIENT_KEY: &str = "nutrient_key";
    pub const UNIT: &str = "unit";
    pub const BASIS: &str = "basis";
    pub const METHOD: &str = "method";
    pub const DISPLAY_ORDER: &str = "display_order";
    pub const DEFAULT_DISPLAY: &str = "default_display";

    pub const VALUE_STATUS: &str = "value_status";
    pub const AMOUNT: &str = "amount";
    pub const RAW_NOTATION: &str = "raw_notation";

    /// Properties the importer may leave out when a repo lacks them.
    pub const OPTIONAL: [&str; 1] = [REMARKS];
}

/// Who may change a field once the record exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Owner {
    /// Printed in the source: every import overwrites it.
    Source,
    /// Parsed from the source by the importer (state, skin/bone). Updated
    /// until a person marks the record `attribute_review_status=reviewed`.
    Derived,
    /// Curated by people: only the first import sets it.
    Human,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordKind {
    Nutrient,
    Ingredient,
    Value,
}

impl RecordKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Nutrient => "nutrient",
            Self::Ingredient => "ingredient",
            Self::Value => "value",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TargetField {
    pub property: &'static str,
    pub value: Option<String>,
    pub owner: Owner,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TargetRecord {
    pub kind: RecordKind,
    pub data_id: String,
    /// `ingredient_key`, `nutrient_key`, or `ingredient_key/nutrient_key`.
    pub business_key: String,
    pub name: String,
    pub name_owner: Owner,
    pub fields: Vec<TargetField>,
}

impl TargetRecord {
    #[cfg(test)]
    pub fn field(&self, property: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|f| f.property == property)
            .and_then(|f| f.value.as_deref())
    }
}

/// A row or cell that was not turned into a record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Quarantined {
    /// `row` (the whole food), `cell` (one value) or `errata`.
    pub scope: &'static str,
    pub row: Option<usize>,
    pub food_code: Option<String>,
    pub nutrient_key: Option<String>,
    pub raw: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct UnmappedState {
    pub count: usize,
    /// Food numbers carrying the word, all of them, for the manual pass.
    pub food_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeferredCell {
    pub row: usize,
    pub food_code: String,
    pub nutrient_key: String,
    pub raw: String,
    pub remark: String,
}

/// `*` in a value cell meaning "see chapter 3", as the table uses for
/// iodine: the remark says `ヨウ素： 第3章参照` (sometimes `*ヨウ素：…`).
fn chapter3_remark(raw: &str, remarks: Option<&str>) -> Option<String> {
    if raw.trim() != "*" {
        return None;
    }
    remarks?
        .lines()
        .map(str::trim)
        .find(|l| l.contains("第3章参照"))
        .map(str::to_string)
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct TargetCatalog {
    pub nutrients: Vec<TargetRecord>,
    pub ingredients: Vec<TargetRecord>,
    pub values: Vec<TargetRecord>,
    pub quarantine: Vec<Quarantined>,
    /// value_status → count.
    pub status_counts: BTreeMap<String, usize>,
    /// Notation shape (`(9)`, `Tr`, `9†`) → count, digits folded to `9`.
    pub notation_counts: BTreeMap<String, usize>,
    /// Header unit vs the expected unit of the identifier.
    pub unit_mismatches: Vec<String>,
    /// Last name word that did not map to a cooking state → foods.
    pub unmapped_states: BTreeMap<String, UnmappedState>,
    /// Cells printed as `*` with a `第3章参照` remark (iodine). They have no
    /// value in this table, so no value record is written and a release
    /// reports `not_listed`.
    pub deferred_to_chapter3: Vec<DeferredCell>,
    /// Ingredient keys left out this run; their existing records are not
    /// delete candidates.
    pub withheld_keys: BTreeSet<String>,
}

#[derive(Debug, Clone)]
pub struct BuildOptions {
    /// e.g. `mext-sfct8-2023`. Part of every data ID.
    pub source_id: String,
    /// e.g. `mext-` → `mext-01001`.
    pub ingredient_key_prefix: String,
    pub basis: String,
    /// `org/repo` of each draft repo. Part of the data ID, because a data
    /// ID is unique across all of Library, not per repo.
    pub ingredient_repo: String,
    pub nutrient_repo: String,
    pub value_repo: String,
    /// Group code → group name.
    pub categories: BTreeMap<String, String>,
}

/// Nutrients shown by default on a new catalog: the five items of the
/// Japanese nutrition label. Only used when the record is created.
const DEFAULT_DISPLAY: [&str; 5] =
    ["ENERC_KCAL", "PROT-", "FAT-", "CHOCDF-", "NACL_EQ"];

/// Final name token → `cooking_state`. Tokens not listed stay blank for a
/// person to fill in; the importer does not guess.
pub const COOKING_STATES: [(&str, &str); 24] = [
    ("生", "raw"),
    ("ゆで", "boiled"),
    ("焼き", "grilled"),
    ("乾", "dried"),
    ("水煮", "simmered"),
    ("油いため", "stir_fried"),
    ("蒸し", "steamed"),
    ("冷凍", "frozen"),
    ("天ぷら", "tempura"),
    ("フライ", "breaded_fried"),
    ("素揚げ", "deep_fried"),
    ("から揚げ", "karaage"),
    ("ソテー", "sauteed"),
    ("電子レンジ調理", "microwaved"),
    ("いり", "roasted"),
    ("水戻し", "rehydrated"),
    ("塩抜き", "desalted"),
    ("素干し", "sun_dried"),
    ("煮干し", "boiled_dried"),
    ("缶詰", "canned"),
    ("水煮缶詰", "canned_in_water"),
    ("味付け缶詰", "canned_seasoned"),
    ("塩漬", "salted"),
    ("くん製", "smoked"),
];

const SKIN_BONE_TOKENS: [&str; 6] = [
    "皮つき",
    "皮なし",
    "皮下脂肪なし",
    "脂身つき",
    "骨つき",
    "骨なし",
];

/// Deterministic Library data ID for a business key.
///
/// `data_` + the first 128 bits of SHA-256, spelled as a lowercase ULID so
/// it has the shape of the IDs Library generates (31 characters).
pub fn data_id(
    repo: &str,
    source_id: &str,
    kind: RecordKind,
    key: &str,
) -> String {
    let mut h = Sha256::new();
    for part in [
        "library-food-import/v1",
        repo,
        source_id,
        kind.as_str(),
        key,
    ] {
        h.update(part.as_bytes());
        h.update([0u8]);
    }
    let digest = h.finalize();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    let ulid = ulid::Ulid::from(u128::from_be_bytes(bytes));
    format!("data_{}", ulid.to_string().to_lowercase())
}

fn name_tokens(name: &str) -> Vec<&str> {
    name.split(|c: char| c.is_whitespace())
        .filter(|t| !t.is_empty())
        .collect()
}

pub fn cooking_state(name: &str) -> Result<Option<&'static str>, String> {
    let tokens = name_tokens(name);
    let Some(last) = tokens.last() else {
        return Ok(None);
    };
    match COOKING_STATES.iter().find(|(t, _)| t == last) {
        Some((_, code)) => Ok(Some(code)),
        None => Err(last.to_string()),
    }
}

fn skin_bone(name: &str) -> Option<String> {
    let found: Vec<&str> = name_tokens(name)
        .into_iter()
        .filter(|t| SKIN_BONE_TOKENS.contains(t))
        .collect();
    (!found.is_empty()).then(|| found.join(" "))
}

/// `別名： 玉ねぎ、オニオン` lines of the remarks → one alias per line.
pub fn aliases_from_remarks(remarks: Option<&str>) -> Option<String> {
    let mut aliases: Vec<String> = Vec::new();
    for line in remarks.unwrap_or("").lines() {
        let line = line.trim();
        let Some(rest) =
            line.strip_prefix("別名").map(|r| r.trim_start()).and_then(
                |r| r.strip_prefix('：').or_else(|| r.strip_prefix(':')),
            )
        else {
            continue;
        };
        for alias in rest.split(['、', ',', '，']) {
            let alias = alias.trim();
            if !alias.is_empty() && !aliases.iter().any(|a| a == alias) {
                aliases.push(alias.to_string());
            }
        }
    }
    (!aliases.is_empty()).then(|| aliases.join("\n"))
}

fn notation_shape(raw: &str) -> String {
    let mut out = String::new();
    let mut in_number = false;
    for c in raw.trim().chars() {
        if c.is_ascii_digit() || c == '.' {
            if !in_number {
                out.push('9');
            }
            in_number = true;
        } else {
            in_number = false;
            out.push(c);
        }
    }
    out
}

fn field(
    property: &'static str,
    value: Option<String>,
    owner: Owner,
) -> TargetField {
    TargetField {
        property,
        value,
        owner,
    }
}

/// Build the draft records. Rows and cells that cannot be represented
/// faithfully go to quarantine instead of being coerced.
pub fn build_catalog(
    table: &SourceTable,
    foods: &[SourceFood],
    errata: &[ErrataOutcome],
    opts: &BuildOptions,
) -> TargetCatalog {
    let mut out = TargetCatalog::default();

    for issue in &table.row_issues {
        out.quarantine.push(Quarantined {
            scope: "row",
            row: Some(issue.row),
            food_code: issue.food_code.clone(),
            nutrient_key: None,
            raw: None,
            reason: issue.reason.clone(),
        });
        if let Some(code) = &issue.food_code {
            out.withheld_keys
                .insert(format!("{}{code}", opts.ingredient_key_prefix));
        }
    }

    // Errata the table disagrees with: the affected cell (or the whole
    // food, for its name) is not trusted this run.
    let mut conflict_cells: BTreeSet<(String, String)> = BTreeSet::new();
    let mut conflict_rows: BTreeSet<String> = BTreeSet::new();
    for o in errata {
        if o.status != ErrataStatus::Conflict {
            continue;
        }
        match o.field.as_str() {
            "remarks" => {} // kept as printed; reported as a conflict
            "name" | "refuse_rate" => {
                conflict_rows.insert(o.food_code.clone());
            }
            f => {
                let key = f.trim_end_matches("+marker").to_string();
                conflict_cells.insert((o.food_code.clone(), key));
            }
        }
    }

    for (order, n) in table.layout.nutrients.iter().enumerate() {
        if let Some(expected) = expected_unit(&n.key) {
            if expected != n.unit {
                out.unit_mismatches.push(format!(
                    "{}: header says {:?}, expected {:?}",
                    n.key, n.unit, expected
                ));
            }
        }
        out.nutrients.push(TargetRecord {
            kind: RecordKind::Nutrient,
            data_id: data_id(
                &opts.nutrient_repo,
                &opts.source_id,
                RecordKind::Nutrient,
                &n.key,
            ),
            business_key: n.key.clone(),
            name: n.name.clone(),
            // A display name people may prefer to word differently.
            name_owner: Owner::Human,
            fields: vec![
                field(
                    prop::NUTRIENT_KEY,
                    Some(n.key.clone()),
                    Owner::Source,
                ),
                field(prop::UNIT, Some(n.unit.clone()), Owner::Source),
                field(prop::BASIS, Some(opts.basis.clone()), Owner::Source),
                field(
                    prop::DISPLAY_ORDER,
                    Some(((order + 1) * 10).to_string()),
                    Owner::Source,
                ),
                field(prop::METHOD, None, Owner::Human),
                field(
                    prop::DEFAULT_DISPLAY,
                    Some(
                        DEFAULT_DISPLAY
                            .contains(&n.key.as_str())
                            .to_string(),
                    ),
                    Owner::Human,
                ),
            ],
        });
    }

    for food in foods {
        let quarantine_row = |out: &mut TargetCatalog, reason: String| {
            out.quarantine.push(Quarantined {
                scope: "row",
                row: Some(food.row),
                food_code: Some(food.food_code.clone()),
                nutrient_key: None,
                raw: None,
                reason,
            });
            out.withheld_keys.insert(format!(
                "{}{}",
                opts.ingredient_key_prefix, food.food_code
            ));
        };
        if conflict_rows.contains(&food.food_code) {
            quarantine_row(
                &mut out,
                "errata conflict on the food name or refuse rate".into(),
            );
            continue;
        }
        let key = match validate_key(
            "ingredient_key",
            &format!("{}{}", opts.ingredient_key_prefix, food.food_code),
        ) {
            Ok(k) => k,
            Err(e) => {
                quarantine_row(&mut out, e.to_string());
                continue;
            }
        };
        let refuse = match food
            .refuse
            .as_deref()
            .map(NormalizedDecimal::parse_percentage)
        {
            None => None,
            Some(Ok(v)) => Some(v.to_string()),
            Some(Err(e)) => {
                quarantine_row(
                    &mut out,
                    format!("refuse rate {:?}: {e}", food.refuse),
                );
                continue;
            }
        };
        let state = match cooking_state(&food.name) {
            Ok(s) => s.map(str::to_string),
            Err(token) => {
                let entry = out.unmapped_states.entry(token).or_default();
                entry.count += 1;
                entry.food_codes.push(food.food_code.clone());
                None
            }
        };
        let state =
            validate_cooking_state(state.as_deref()).unwrap_or(None);
        let category_name = food
            .group_code
            .as_ref()
            .and_then(|g| opts.categories.get(g))
            .cloned();

        out.ingredients.push(TargetRecord {
            kind: RecordKind::Ingredient,
            data_id: data_id(
                &opts.ingredient_repo,
                &opts.source_id,
                RecordKind::Ingredient,
                &food.food_code,
            ),
            business_key: key.clone(),
            name: food.name.clone(),
            name_owner: Owner::Source,
            fields: vec![
                field(
                    prop::INGREDIENT_KEY,
                    Some(key.clone()),
                    Owner::Source,
                ),
                field(
                    prop::SOURCE_FOOD_CODE,
                    Some(food.food_code.clone()),
                    Owner::Source,
                ),
                field(
                    prop::CATEGORY_CODE,
                    food.group_code.clone(),
                    Owner::Source,
                ),
                field(prop::CATEGORY_NAME, category_name, Owner::Source),
                field(prop::REFUSE_RATE, refuse, Owner::Source),
                field(prop::REMARKS, food.remarks.clone(), Owner::Source),
                field(prop::COOKING_STATE, state, Owner::Derived),
                field(
                    prop::SKIN_BONE,
                    skin_bone(&food.name),
                    Owner::Derived,
                ),
                field(prop::PART, None, Owner::Derived),
                field(prop::STANDARD_NAME, None, Owner::Human),
                field(prop::READING, None, Owner::Human),
                field(
                    prop::ALIASES,
                    aliases_from_remarks(food.remarks.as_deref()),
                    Owner::Human,
                ),
                field(
                    prop::ATTRIBUTE_REVIEW_STATUS,
                    Some("unreviewed".into()),
                    Owner::Human,
                ),
            ],
        });

        for n in &table.layout.nutrients {
            let cell = |reason: String, raw: Option<&String>| Quarantined {
                scope: "cell",
                row: Some(food.row),
                food_code: Some(food.food_code.clone()),
                nutrient_key: Some(n.key.clone()),
                raw: raw.cloned(),
                reason,
            };
            let raw = food.values.get(&n.key);
            if conflict_cells
                .contains(&(food.food_code.clone(), n.key.clone()))
            {
                out.quarantine.push(cell(
                    "errata conflict on this value".into(),
                    raw,
                ));
                continue;
            }
            let Some(raw) = raw else {
                out.quarantine.push(cell(
                    "empty cell (not a table notation)".into(),
                    None,
                ));
                continue;
            };
            if let Some(remark) =
                chapter3_remark(raw, food.remarks.as_deref())
            {
                out.deferred_to_chapter3.push(DeferredCell {
                    row: food.row,
                    food_code: food.food_code.clone(),
                    nutrient_key: n.key.clone(),
                    raw: raw.trim().to_string(),
                    remark,
                });
                continue;
            }
            let (status, amount) = match NutrientValueStatus::from_notation(
                strip_footnotes(raw),
            ) {
                Ok(v) => v,
                Err(e) => {
                    out.quarantine.push(cell(e.to_string(), Some(raw)));
                    continue;
                }
            };
            *out.status_counts.entry(status.to_string()).or_default() += 1;
            *out.notation_counts.entry(notation_shape(raw)).or_default() +=
                1;
            let business_key = format!("{key}/{}", n.key);
            out.values.push(TargetRecord {
                kind: RecordKind::Value,
                data_id: data_id(
                    &opts.value_repo,
                    &opts.source_id,
                    RecordKind::Value,
                    &format!("{}/{}", food.food_code, n.key),
                ),
                business_key,
                name: format!("{} {}", food.food_code, n.key),
                name_owner: Owner::Source,
                fields: vec![
                    field(
                        prop::INGREDIENT_KEY,
                        Some(key.clone()),
                        Owner::Source,
                    ),
                    field(
                        prop::NUTRIENT_KEY,
                        Some(n.key.clone()),
                        Owner::Source,
                    ),
                    field(
                        prop::VALUE_STATUS,
                        Some(status.to_string()),
                        Owner::Source,
                    ),
                    field(
                        prop::AMOUNT,
                        amount.map(|a| a.to_string()),
                        Owner::Source,
                    ),
                    field(
                        prop::RAW_NOTATION,
                        Some(raw.trim().to_string()),
                        Owner::Source,
                    ),
                ],
            });
        }
    }
    out
}

#[cfg(test)]
pub(super) mod tests {
    use super::super::errata::{apply_errata, parse_errata};
    use super::super::fixtures;
    use super::super::table::{categories_from_sheet_names, parse_table};
    use super::*;

    pub fn options() -> BuildOptions {
        BuildOptions {
            source_id: "mext-sfct8-2023".into(),
            ingredient_key_prefix: "mext-".into(),
            basis: "可食部100g当たり".into(),
            ingredient_repo: "library/food".into(),
            nutrient_repo: "library/food-nutrients".into(),
            value_repo: "library/food-nutrient-values".into(),
            categories: categories_from_sheet_names(&fixtures::table_book()),
        }
    }

    pub fn fixture_catalog() -> TargetCatalog {
        let table = parse_table(&fixtures::main_sheet()).unwrap();
        let mut foods = table.foods.clone();
        let book = parse_errata(&fixtures::errata_book(), &table.layout);
        let outcomes = apply_errata(&mut foods, &book);
        build_catalog(&table, &foods, &outcomes, &options())
    }

    fn value<'a>(
        c: &'a TargetCatalog,
        code: &str,
        key: &str,
    ) -> &'a TargetRecord {
        c.values
            .iter()
            .find(|v| v.business_key == format!("mext-{code}/{key}"))
            .unwrap_or_else(|| panic!("no value {code}/{key}"))
    }

    #[test]
    fn every_notation_keeps_its_meaning() {
        let c = fixture_catalog();
        let check = |code: &str,
                     key: &str,
                     status: &str,
                     amount: Option<&str>,
                     raw: &str| {
            let v = value(&c, code, key);
            assert_eq!(
                v.field(prop::VALUE_STATUS),
                Some(status),
                "{code}/{key}"
            );
            assert_eq!(v.field(prop::AMOUNT), amount, "{code}/{key}");
            assert_eq!(
                v.field(prop::RAW_NOTATION),
                Some(raw),
                "{code}/{key}"
            );
        };
        check("01001", "ENERC_KCAL", "measured", Some("343"), "343");
        check("01001", "PROTCAA", "estimated", Some("11.3"), "(11.3)");
        check("01001", "VITK", "estimated_zero", Some("0"), "(0)");
        check("06153", "VITK", "zero", Some("0"), "0");
        check("06153", "VITB12", "not_measured", None, "-");
        check("06153", "PROT-", "measured", Some("1"), "1.0");
        check("06154", "VITK", "trace", None, "Tr");
        check("06154", "NA", "estimated_trace", None, "(Tr)");
        check("06154", "CHOAVLM", "measured", Some("20.3"), "20.3†");
        // Errata applied: 33 → 31 kcal.
        check("06153", "ENERC_KCAL", "measured", Some("31"), "31");
        // Errata applied over a conflict: table 7, 誤 5, 正 `-`.
        check("10330", "VITK", "not_measured", None, "-");
    }

    #[test]
    fn chapter3_stars_are_deferred_not_quarantined() {
        let c = fixture_catalog();
        assert_eq!(
            c.deferred_to_chapter3,
            vec![DeferredCell {
                row: 12,
                food_code: "10330".into(),
                nutrient_key: "NA".into(),
                raw: "*".into(),
                remark: "ヨウ素： 第3章参照".into(),
            }]
        );
        assert!(!c
            .quarantine
            .iter()
            .any(|q| q.nutrient_key.as_deref() == Some("NA")));
        assert!(!c
            .values
            .iter()
            .any(|v| v.business_key == "mext-10330/NA"));
        // A `*` without the chapter 3 remark is still an anomaly.
        assert_eq!(chapter3_remark("*", Some("別名： あじ")), None);
        assert_eq!(chapter3_remark("*", None), None);
        assert_eq!(
            chapter3_remark(
                " * ",
                Some("*ヨウ素： 第3章参照\n硝酸イオン： 0 g")
            )
            .as_deref(),
            Some("*ヨウ素： 第3章参照")
        );
        assert_eq!(chapter3_remark("Tr", Some("ヨウ素： 第3章参照")), None);
    }

    #[test]
    fn counts_and_quarantine() {
        let c = fixture_catalog();
        assert_eq!(c.nutrients.len(), 10);
        let codes: Vec<_> = c
            .ingredients
            .iter()
            .map(|i| i.field(prop::SOURCE_FOOD_CODE).unwrap())
            .collect();
        assert_eq!(codes, vec!["01001", "06153", "06154", "10330"]);
        // 4 foods x 10 nutrients, minus: 10330 NA `*` (chapter 3),
        // 10330 VITB12 empty, 10330 FIB- empty.
        assert_eq!(c.values.len(), 37);
        let reasons: Vec<_> = c
            .quarantine
            .iter()
            .map(|q| {
                (
                    q.scope,
                    q.food_code.clone().unwrap_or_default(),
                    q.nutrient_key.clone().unwrap_or_default(),
                )
            })
            .collect();
        for expected in [
            ("cell", "10330", "VITB12"),
            ("cell", "10330", "FIB-"),
            ("row", "1002", ""),
            ("row", "99999", ""),
            ("row", "18001", ""),
        ] {
            assert!(
                reasons.contains(&(
                    expected.0,
                    expected.1.into(),
                    expected.2.into()
                )),
                "{expected:?} in {reasons:?}"
            );
        }
        assert!(c.withheld_keys.contains("mext-18001"));
        assert_eq!(c.notation_counts.get("9†"), Some(&1));
        assert!(c.unit_mismatches.is_empty());
    }

    #[test]
    fn ingredients_carry_source_fields_and_first_guesses() {
        let c = fixture_catalog();
        let onion = c
            .ingredients
            .iter()
            .find(|i| i.business_key == "mext-06153")
            .unwrap();
        assert_eq!(
            onion.name,
            "（たまねぎ類）\u{3000}たまねぎ\u{3000}りん茎\u{3000}生"
        );
        assert_eq!(onion.field(prop::SOURCE_FOOD_CODE), Some("06153"));
        assert_eq!(onion.field(prop::CATEGORY_CODE), Some("06"));
        assert_eq!(onion.field(prop::CATEGORY_NAME), Some("野菜類"));
        assert_eq!(onion.field(prop::REFUSE_RATE), Some("6"));
        assert_eq!(onion.field(prop::COOKING_STATE), Some("raw"));
        assert_eq!(
            onion.field(prop::ALIASES),
            Some("玉ねぎ\nオニオン\n玉葱")
        );
        assert_eq!(
            onion.field(prop::ATTRIBUTE_REVIEW_STATUS),
            Some("unreviewed")
        );
        let aji = c
            .ingredients
            .iter()
            .find(|i| i.business_key == "mext-10330")
            .unwrap();
        assert_eq!(aji.field(prop::COOKING_STATE), Some("breaded_fried"));
        assert_eq!(aji.field(prop::SKIN_BONE), Some("皮つき"));
        assert_eq!(aji.field(prop::REFUSE_RATE), Some("0"));
        let amaranth = c
            .ingredients
            .iter()
            .find(|i| i.business_key == "mext-01001")
            .unwrap();
        assert_eq!(amaranth.field(prop::COOKING_STATE), None);
        assert_eq!(
            c.unmapped_states.get("玄穀"),
            Some(&UnmappedState {
                count: 1,
                food_codes: vec!["01001".into()]
            })
        );
    }

    #[test]
    fn data_ids_are_stable_and_distinct() {
        let a = data_id(
            "library/food",
            "mext-sfct8-2023",
            RecordKind::Ingredient,
            "01001",
        );
        assert_eq!(
            a,
            data_id(
                "library/food",
                "mext-sfct8-2023",
                RecordKind::Ingredient,
                "01001"
            )
        );
        assert_eq!(a.len(), 31);
        assert!(a.starts_with("data_"));
        assert!(a[5..]
            .bytes()
            .all(|b| b.is_ascii_digit() || b.is_ascii_lowercase()));
        assert_ne!(
            a,
            data_id(
                "library/food",
                "mext-sfct8-2023",
                RecordKind::Ingredient,
                "1001"
            )
        );
        assert_ne!(
            a,
            data_id(
                "other/food",
                "mext-sfct8-2023",
                RecordKind::Ingredient,
                "01001"
            )
        );
        assert_ne!(
            a,
            data_id(
                "library/food",
                "mext-sfct8-2023",
                RecordKind::Value,
                "01001"
            )
        );
        // Pinned so an accidental change of the derivation is caught.
        assert_eq!(a, PINNED_ID);
    }

    const PINNED_ID: &str = "data_2qgz2drnpbdqqvj6fhpk84srqg";

    #[test]
    fn nutrient_definitions() {
        let c = fixture_catalog();
        let kcal = c
            .nutrients
            .iter()
            .find(|n| n.business_key == "ENERC_KCAL")
            .unwrap();
        assert_eq!(kcal.field(prop::UNIT), Some("kcal"));
        assert_eq!(kcal.field(prop::BASIS), Some("可食部100g当たり"));
        assert_eq!(kcal.field(prop::DEFAULT_DISPLAY), Some("true"));
        assert_eq!(kcal.field(prop::DISPLAY_ORDER), Some("20"));
        let kj = c
            .nutrients
            .iter()
            .find(|n| n.business_key == "ENERC")
            .unwrap();
        assert_eq!(kj.field(prop::DEFAULT_DISPLAY), Some("false"));
        assert_ne!(kj.data_id, kcal.data_id);
    }

    #[test]
    fn every_cooking_state_code_is_valid() {
        for (_, code) in COOKING_STATES {
            assert!(validate_cooking_state(Some(code)).is_ok(), "{code}");
        }
    }

    #[test]
    fn aliases_come_from_the_betsumei_line() {
        assert_eq!(
            aliases_from_remarks(Some(
                "試料： 和牛\n別名： レバー、きも\n"
            ))
            .as_deref(),
            Some("レバー\nきも")
        );
        assert_eq!(
            aliases_from_remarks(Some("別名：黒豚")).as_deref(),
            Some("黒豚")
        );
        assert_eq!(aliases_from_remarks(Some("廃棄部位： 株元")), None);
    }
}
