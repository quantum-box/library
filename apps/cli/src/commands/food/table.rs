//! The main table (本表) of the Standard Tables of Food Composition.
//!
//! The layout is found from the header text rather than fixed column
//! numbers: the row holding `成分識別子` names every nutrient column by its
//! component identifier, `単位` gives the units, and `食品番号` / `食品名` /
//! `備考` locate the food columns. The same parser reads the errata
//! workbook's copy of the table, which is shifted by one column.

use std::collections::{BTreeMap, BTreeSet};

use ingredient_notation::validate_key;
use serde::Serialize;

use super::sheet::{match_key, narrow_alnum, squash, Sheet, Workbook};

/// Units the importer knows how to carry. Anything else in the unit row
/// makes the column unsupported rather than guessed.
const KNOWN_UNITS: [&str; 6] = ["%", "g", "mg", "μg", "kJ", "kcal"];
/// Nutrients that may have an adjacent `*` column identifying the energy input.
const ENERGY_MARKER_NUTRIENTS: [&str; 2] = ["CHOAVLM", "CHOAVLDF-"];

/// Expected unit per component identifier of the 2023 table. A header
/// that disagrees is reported as a unit mismatch and blocks writing.
pub const EXPECTED_UNITS: [(&str, &str); 53] = [
    ("ENERC", "kJ"),
    ("ENERC_KCAL", "kcal"),
    ("WATER", "g"),
    ("PROTCAA", "g"),
    ("PROT-", "g"),
    ("FATNLEA", "g"),
    ("CHOLE", "mg"),
    ("FAT-", "g"),
    ("CHOAVLM", "g"),
    ("CHOAVL", "g"),
    ("CHOAVLDF-", "g"),
    ("FIB-", "g"),
    ("POLYL", "g"),
    ("CHOCDF-", "g"),
    ("OA", "g"),
    ("ASH", "g"),
    ("NA", "mg"),
    ("K", "mg"),
    ("CA", "mg"),
    ("MG", "mg"),
    ("P", "mg"),
    ("FE", "mg"),
    ("ZN", "mg"),
    ("CU", "mg"),
    ("MN", "mg"),
    ("ID", "μg"),
    ("SE", "μg"),
    ("CR", "μg"),
    ("MO", "μg"),
    ("RETOL", "μg"),
    ("CARTA", "μg"),
    ("CARTB", "μg"),
    ("CRYPXB", "μg"),
    ("CARTBEQ", "μg"),
    ("VITA_RAE", "μg"),
    ("VITD", "μg"),
    ("TOCPHA", "mg"),
    ("TOCPHB", "mg"),
    ("TOCPHG", "mg"),
    ("TOCPHD", "mg"),
    ("VITK", "μg"),
    ("THIA", "mg"),
    ("RIBF", "mg"),
    ("NIA", "mg"),
    ("NE", "mg"),
    ("VITB6A", "mg"),
    ("VITB12", "μg"),
    ("FOL", "μg"),
    ("PANTAC", "mg"),
    ("BIOT", "μg"),
    ("VITC", "mg"),
    ("ALC", "g"),
    ("NACL_EQ", "g"),
];

pub fn expected_unit(key: &str) -> Option<&'static str> {
    EXPECTED_UNITS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, u)| *u)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NutrientColumn {
    /// 0-based column.
    pub col: usize,
    /// Component identifier (成分識別子), e.g. `ENERC_KCAL`, `PROT-`.
    pub key: String,
    /// Display name from the header, e.g. `エネルギー（kcal）`.
    pub name: String,
    /// Header labels from the top level down.
    pub label_path: Vec<String>,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IgnoredColumn {
    pub col: usize,
    pub label: String,
    pub reason: String,
    pub non_empty_cells: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct TableLayout {
    pub sheet: String,
    /// 1-based spreadsheet rows, as a person would look them up.
    pub header_rows: (usize, usize),
    pub identifier_row: usize,
    pub unit_row: usize,
    pub first_data_row: usize,
    pub group_col: Option<usize>,
    pub code_col: usize,
    pub index_col: Option<usize>,
    pub name_col: usize,
    pub refuse_col: Option<usize>,
    pub remarks_col: Option<usize>,
    pub nutrients: Vec<NutrientColumn>,
    /// Marker columns (the `*` beside available carbohydrates that marks
    /// the value used for energy), keyed by the nutrient they belong to.
    pub markers: BTreeMap<usize, String>,
    pub ignored: Vec<IgnoredColumn>,
    /// Problems that make the layout unusable for writing.
    pub errors: Vec<String>,
    /// Updated-on label printed above the table, e.g. `更新日：2026年3月27日`.
    pub updated_label: Option<String>,
}

impl TableLayout {
    pub fn nutrient(&self, key: &str) -> Option<&NutrientColumn> {
        self.nutrients.iter().find(|n| n.key == key)
    }
}

/// One food row exactly as the table prints it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceFood {
    /// 1-based spreadsheet row.
    pub row: usize,
    pub group_code: Option<String>,
    pub food_code: String,
    pub index_code: Option<String>,
    pub name: String,
    pub refuse: Option<String>,
    /// Component identifier → cell text. Empty cells are absent.
    pub values: BTreeMap<String, String>,
    /// Nutrient key → marker text (`*`) from the marker columns.
    pub markers: BTreeMap<String, String>,
    pub remarks: Option<String>,
}

/// A row that could not be read as a food.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RowIssue {
    pub row: usize,
    pub food_code: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceTable {
    pub layout: TableLayout,
    pub foods: Vec<SourceFood>,
    pub row_issues: Vec<RowIssue>,
    /// Non-blank rows below the header, including rejected ones.
    pub rows_read: usize,
}

/// Food group code (`01`) → name (`穀類`), from the per-group sheet names
/// (`1穀類`, `18調理済み流通食品類`).
pub fn categories_from_sheet_names(
    book: &Workbook,
) -> Result<BTreeMap<String, String>, String> {
    let mut categories = BTreeMap::new();
    for name in book.sheet_names() {
        let digits: String =
            name.chars().take_while(|c| c.is_ascii_digit()).collect();
        let rest = name[digits.len()..].trim();
        if digits.is_empty() || rest.is_empty() || digits.len() > 2 {
            continue;
        }
        let code = format!("{digits:0>2}");
        if let Some(existing) = categories.get(&code) {
            return Err(format!(
                "food-group sheets {existing:?} and {rest:?} normalize to the same code {code}"
            ));
        }
        categories.insert(code, rest.to_string());
    }
    Ok(categories)
}

fn find_cell(
    sheet: &Sheet,
    rows: std::ops::Range<usize>,
    want: &str,
) -> Option<(usize, usize)> {
    for r in rows {
        for c in 0..sheet.width() {
            if sheet.cell(r, c).is_some_and(|v| squash(v) == want) {
                return Some((r, c));
            }
        }
    }
    None
}

/// Locate the header and describe every column.
pub fn parse_layout(sheet: &Sheet) -> Result<TableLayout, String> {
    let scan = 0..sheet.height().min(40);
    let (id_row, _) = find_cell(sheet, scan.clone(), "成分識別子")
        .ok_or("no 成分識別子 (component identifier) row in the header")?;
    let header = 0..id_row;
    let (code_row, code_col) = find_cell(sheet, header.clone(), "食品番号")
        .ok_or("no 食品番号 (food number) column")?;
    let (name_row, name_col) = find_cell(sheet, header.clone(), "食品名")
        .ok_or("no 食品名 (food name) column")?;
    let (unit_row, _) = find_cell(sheet, header.clone(), "単位")
        .ok_or("no 単位 (unit) row")?;
    let group_col = find_cell(sheet, header.clone(), "食品群")
        .map(|p| p.1)
        .ok_or("no 食品群 (food group) column")?;
    let index_col =
        find_cell(sheet, header.clone(), "索引番号").map(|p| p.1);
    let remarks_col = find_cell(sheet, header.clone(), "備考")
        .map(|p| p.1)
        .ok_or("no 備考 (remarks) column")?;
    let updated_label = (0..id_row).find_map(|r| {
        (0..sheet.width()).find_map(|c| {
            sheet
                .cell(r, c)
                .filter(|v| squash(v).starts_with("更新日"))
                .map(|v| v.trim().to_string())
        })
    });

    let first_data_row = id_row + 1;
    let data_rows = first_data_row..sheet.height();
    let last_col = remarks_col;

    let mut layout = TableLayout {
        sheet: sheet.name.clone(),
        header_rows: (code_row.min(name_row) + 1, id_row + 1),
        identifier_row: id_row + 1,
        unit_row: unit_row + 1,
        first_data_row: first_data_row + 1,
        group_col: Some(group_col),
        code_col,
        index_col,
        name_col,
        refuse_col: None,
        remarks_col: Some(remarks_col),
        nutrients: Vec::new(),
        markers: BTreeMap::new(),
        ignored: Vec::new(),
        errors: Vec::new(),
        updated_label,
    };

    let non_empty = |col: usize| {
        data_rows
            .clone()
            .filter(|&r| {
                sheet.cell(r, col).is_some_and(|v| !v.trim().is_empty())
            })
            .count()
    };

    let mut last_nutrient: Option<String> = None;
    for col in (name_col + 1)..last_col {
        let identifier =
            sheet.cell(id_row, col).map(str::trim).unwrap_or_default();
        let labels = label_path(sheet, name_row..unit_row, name_col, col);
        let label = labels.last().cloned().unwrap_or_default();

        if identifier == "REFUSE" {
            if layout.refuse_col.is_some() {
                layout.errors.push(
                    "REFUSE identifier appears more than once".into(),
                );
            } else {
                layout.refuse_col = Some(col);
            }
            continue;
        }
        if identifier.is_empty() {
            let cells = non_empty(col);
            if cells == 0 {
                layout.ignored.push(IgnoredColumn {
                    col,
                    label: String::new(),
                    reason: "empty spacer column".into(),
                    non_empty_cells: 0,
                });
            } else {
                let marker_owner =
                    last_nutrient.as_ref().and_then(|owner| {
                        let nutrient_col = layout.nutrient(owner)?.col;
                        (ENERGY_MARKER_NUTRIENTS.contains(&owner.as_str())
                            && nutrient_col + 1 == col
                            && !layout
                                .markers
                                .values()
                                .any(|marker| marker == owner))
                        .then_some(owner)
                    });
                let only_marker_values = data_rows.clone().all(|row| {
                    match sheet.cell(row, col).map(str::trim) {
                        Some(value) if !value.is_empty() => value == "*",
                        _ => true,
                    }
                });

                if let Some(owner) =
                    marker_owner.filter(|_| only_marker_values)
                {
                    layout.markers.insert(col, owner.clone());
                    layout.ignored.push(IgnoredColumn {
                        col,
                        label: format!("{owner} marker"),
                        reason: format!(
                            "marker column without a component identifier (`*` = the {owner} value used for the energy calculation); not stored"
                        ),
                        non_empty_cells: cells,
                    });
                } else {
                    layout.errors.push(format!(
                        "column {} has data without a component identifier or a recognized carbohydrate marker",
                        col + 1
                    ));
                    layout.ignored.push(IgnoredColumn {
                        col,
                        label,
                        reason: "unexpected data in a column without a component identifier"
                            .into(),
                        non_empty_cells: cells,
                    });
                }
            }
            continue;
        }
        if let Err(e) = validate_key("nutrient_key", identifier) {
            let cells = non_empty(col);
            if cells > 0 {
                layout.errors.push(format!(
                    "column {} has data with an invalid component identifier {identifier:?}: {e}",
                    col + 1
                ));
            }
            layout.ignored.push(IgnoredColumn {
                col,
                label,
                reason: e.to_string(),
                non_empty_cells: cells,
            });
            continue;
        }
        let unit_text = nearest_left(sheet, unit_row, name_col, col)
            .map(|(_, v)| unit_token(v))
            .unwrap_or_default();
        if !KNOWN_UNITS.contains(&unit_text.as_str()) {
            layout.errors.push(format!(
                "component identifier {identifier} has unsupported unit {unit_text:?}"
            ));
            layout.ignored.push(IgnoredColumn {
                col,
                label,
                reason: format!(
                    "unsupported unit {unit_text:?} for {identifier}"
                ),
                non_empty_cells: non_empty(col),
            });
            continue;
        }
        if layout.nutrient(identifier).is_some() {
            layout.errors.push(format!(
                "component identifier {identifier} appears twice"
            ));
            continue;
        }
        let name = display_name(&label);
        if name.is_empty() {
            layout.errors.push(format!(
                "component identifier {identifier} has a blank display label"
            ));
        }
        last_nutrient = Some(identifier.to_string());
        layout.nutrients.push(NutrientColumn {
            col,
            key: identifier.to_string(),
            name,
            label_path: labels.iter().map(|l| display_name(l)).collect(),
            unit: unit_text,
        });
    }

    for col in (remarks_col + 1)..sheet.width() {
        let cells = (0..sheet.height())
            .filter(|&row| {
                sheet
                    .cell(row, col)
                    .is_some_and(|value| !value.trim().is_empty())
            })
            .count();
        if cells > 0 {
            layout.errors.push(format!(
                "column {} contains data outside the table boundary after 備考",
                col + 1
            ));
            layout.ignored.push(IgnoredColumn {
                col,
                label: label_path(sheet, name_row..unit_row, name_col, col)
                    .last()
                    .cloned()
                    .unwrap_or_default(),
                reason: "populated column after the 備考 table boundary"
                    .into(),
                non_empty_cells: cells,
            });
        }
    }

    if layout.refuse_col.is_none() {
        layout
            .errors
            .push("missing REFUSE identifier column".into());
    }

    // Two columns may share a label (`エネルギー` in kJ and in kcal); the
    // unit tells them apart.
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for n in &layout.nutrients {
        *seen.entry(n.name.clone()).or_default() += 1;
    }
    for n in &mut layout.nutrients {
        if n.name.is_empty() {
            n.name = n.key.clone();
        } else if seen.get(&n.name).copied().unwrap_or(0) > 1 {
            n.name = format!("{}（{}）", n.name, n.unit);
        }
    }

    if layout.nutrients.is_empty() {
        layout.errors.push("no nutrient columns found".into());
    }
    Ok(layout)
}

/// The cell at or left of `col` on `row`, not crossing `min_col`.
fn nearest_left(
    sheet: &Sheet,
    row: usize,
    min_col: usize,
    col: usize,
) -> Option<(usize, &str)> {
    (min_col + 1..=col).rev().find_map(|c| {
        sheet
            .cell(row, c)
            .filter(|v| !v.trim().is_empty())
            .map(|v| (c, v))
    })
}

/// Header labels for a column, top level first.
///
/// Group headers are merged cells whose text sits in their first column,
/// so a column inherits the nearest label to its left, but only from
/// within the span of the level above it. That keeps `食物繊維総量` from
/// inheriting the `差引き法による…` label of an earlier sibling.
fn label_path(
    sheet: &Sheet,
    rows: std::ops::Range<usize>,
    name_col: usize,
    col: usize,
) -> Vec<String> {
    let mut path = Vec::new();
    let mut span_start = name_col + 1;
    for row in rows {
        if let Some((c, text)) = nearest_left(sheet, row, name_col, col) {
            if c >= span_start {
                let text = squash(text);
                // Skip the banner that spans the whole table.
                if !text.contains("100g当たり") {
                    path.push(text);
                    span_start = c;
                }
            }
        }
    }
    path
}

fn display_name(label: &str) -> String {
    narrow_alnum(&squash(label)).replace('|', "-")
}

/// `(…… mg ……)` → `mg`.
fn unit_token(raw: &str) -> String {
    raw.chars()
        .filter(|c| {
            !c.is_whitespace()
                && !matches!(c, '(' | ')' | '（' | '）' | '…' | '.')
        })
        .collect::<String>()
        .replace('µ', "μ")
}

/// Read every food row under the header.
pub fn parse_table(sheet: &Sheet) -> Result<SourceTable, String> {
    let layout = parse_layout(sheet)?;
    let mut foods = Vec::new();
    let mut row_issues = Vec::new();
    let mut rows_read = 0;
    let text = |r: usize, c: Option<usize>| {
        c.and_then(|c| sheet.cell(r, c))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };

    for r in (layout.first_data_row - 1)..sheet.height() {
        if sheet.row_is_blank(r) {
            continue;
        }
        rows_read += 1;
        let row = r + 1;
        let code = text(r, Some(layout.code_col));
        let Some(code) = code else {
            row_issues.push(RowIssue {
                row,
                food_code: None,
                reason: "row has no food number".into(),
            });
            continue;
        };
        if !(code.len() == 5 && code.bytes().all(|b| b.is_ascii_digit())) {
            row_issues.push(RowIssue {
                row,
                food_code: Some(code.clone()),
                reason: format!(
                    "food number must be 5 digits kept as text (leading zeros lost?): {code:?}"
                ),
            });
            continue;
        }
        let Some(name) = text(r, Some(layout.name_col)) else {
            row_issues.push(RowIssue {
                row,
                food_code: Some(code),
                reason: "row has no food name".into(),
            });
            continue;
        };
        let mut values = BTreeMap::new();
        for n in &layout.nutrients {
            if let Some(v) = text(r, Some(n.col)) {
                values.insert(n.key.clone(), v);
            }
        }
        let mut markers = BTreeMap::new();
        for (col, owner) in &layout.markers {
            if let Some(v) = text(r, Some(*col)) {
                markers.insert(owner.clone(), v);
            }
        }
        foods.push(SourceFood {
            row,
            group_code: text(r, layout.group_col),
            food_code: code,
            index_code: text(r, layout.index_col),
            name,
            refuse: text(r, layout.refuse_col),
            values,
            markers,
            remarks: layout
                .remarks_col
                .and_then(|c| sheet.cell(r, c))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
        });
    }

    // A food number must identify one row. Every copy is rejected: which
    // one is right is a question for a person, not the importer.
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for f in &foods {
        *counts.entry(f.food_code.clone()).or_default() += 1;
    }
    let duplicated: BTreeSet<String> = counts
        .into_iter()
        .filter(|(_, n)| *n > 1)
        .map(|(c, _)| c)
        .collect();
    if !duplicated.is_empty() {
        foods.retain(|f| {
            if duplicated.contains(&f.food_code) {
                row_issues.push(RowIssue {
                    row: f.row,
                    food_code: Some(f.food_code.clone()),
                    reason: "duplicate food number".into(),
                });
                false
            } else {
                true
            }
        });
    }

    Ok(SourceTable {
        layout,
        foods,
        row_issues,
        rows_read,
    })
}

/// Label → nutrient key, for reading the errata's `項目等` column.
///
/// A label shared by two columns (`エネルギー` in kJ and kcal) is left
/// out, so an ambiguous errata item is reported instead of guessed.
pub fn label_index(layout: &TableLayout) -> BTreeMap<String, String> {
    let mut candidates: BTreeMap<String, BTreeSet<String>> =
        BTreeMap::new();
    for n in &layout.nutrients {
        let base = n.label_path.last().cloned().unwrap_or_default();
        let mut labels =
            vec![base.clone(), n.name.clone(), format!("{base}{}", n.unit)];
        // `利用可能炭水化物（差引き法による利用可能炭水化物）`: parent（child）.
        if n.label_path.len() >= 2 {
            let parent = &n.label_path[n.label_path.len() - 2];
            labels.push(format!("{parent}（{base}）"));
        }
        for label in labels {
            candidates
                .entry(match_key(&label))
                .or_default()
                .insert(n.key.clone());
        }
    }
    candidates
        .into_iter()
        .filter(|(label, keys)| !label.is_empty() && keys.len() == 1)
        .filter_map(|(label, keys)| {
            keys.into_iter().next().map(|k| (label, k))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::fixtures;
    use super::*;

    #[test]
    fn finds_the_layout_from_header_text() {
        let table = parse_table(&fixtures::main_sheet()).unwrap();
        let layout = &table.layout;
        assert!(layout.errors.is_empty(), "{:?}", layout.errors);
        assert_eq!(layout.code_col, 1);
        assert_eq!(layout.name_col, 3);
        assert_eq!(layout.refuse_col, Some(4));
        let keys: Vec<_> =
            layout.nutrients.iter().map(|n| n.key.as_str()).collect();
        assert_eq!(
            keys,
            vec![
                "ENERC",
                "ENERC_KCAL",
                "PROTCAA",
                "PROT-",
                "CHOAVLM",
                "CHOAVLDF-",
                "FIB-",
                "ID",
                "NA",
                "VITK",
                "VITB12"
            ]
        );
        let unit = |k: &str| layout.nutrient(k).unwrap().unit.clone();
        assert_eq!(unit("ENERC"), "kJ");
        assert_eq!(unit("ENERC_KCAL"), "kcal");
        assert_eq!(unit("NA"), "mg");
        assert_eq!(unit("VITB12"), "μg");
        let name = |k: &str| layout.nutrient(k).unwrap().name.clone();
        assert_eq!(name("ENERC"), "エネルギー（kJ）");
        assert_eq!(name("ENERC_KCAL"), "エネルギー（kcal）");
        assert_eq!(name("PROTCAA"), "アミノ酸組成によるたんぱく質");
        assert_eq!(name("FIB-"), "食物繊維総量");
        assert_eq!(name("ID"), "ヨウ素");
        assert_eq!(name("VITB12"), "ビタミンB12");
        // Malformed identifier columns with values must block the layout.
        let malformed_sheet = |second_identifier: &str| {
            Sheet::from_rows(
                "malformed",
                &[
                    &["食品群", "食品番号", "", "食品名", "x", "x", "備考"],
                    &["", "", "単位", "", "%", "%", ""],
                    &[
                        "成分識別子",
                        "",
                        "",
                        "",
                        "REFUSE",
                        second_identifier,
                        "",
                    ],
                    &["", "00001", "", "食品", "1", "1", ""],
                ],
            )
        };
        let duplicate_refuse =
            parse_layout(&malformed_sheet("REFUSE")).unwrap();
        assert!(duplicate_refuse.errors.iter().any(|error| {
            error.contains("REFUSE identifier appears more than once")
        }));
        let invalid_identifier =
            parse_layout(&malformed_sheet("BAD/ID")).unwrap();
        assert!(invalid_identifier.errors.iter().any(|error| {
            error.contains("invalid component identifier")
        }));

        let trailing_column = Sheet::from_rows(
            "trailing",
            &[
                &[
                    "食品群",
                    "食品番号",
                    "",
                    "食品名",
                    "x",
                    "x",
                    "備考",
                    "追加成分",
                ],
                &["", "", "単位", "", "%", "%", "", "mg"],
                &["成分識別子", "", "", "", "REFUSE", "PROT-", "", "IRON"],
                &["", "00001", "", "食品", "1", "1", "", "2"],
            ],
        );
        let trailing_layout = parse_layout(&trailing_column).unwrap();
        assert!(trailing_layout.errors.iter().any(|error| {
            error.contains("outside the table boundary after 備考")
        }));
        // The `*` column beside CHOAVLM is a marker, not a nutrient.
        assert_eq!(
            layout.markers.values().collect::<Vec<_>>(),
            vec!["CHOAVLM"]
        );
        assert!(layout
            .ignored
            .iter()
            .any(|c| c.reason.contains("marker column")));
    }

    #[test]
    fn keeps_food_numbers_as_text() {
        let table = parse_table(&fixtures::main_sheet()).unwrap();
        let codes: Vec<_> =
            table.foods.iter().map(|f| f.food_code.as_str()).collect();
        assert_eq!(
            codes,
            vec!["01001", "06153", "06154", "10330", "18001"]
        );
        let amaranth = &table.foods[0];
        assert_eq!(amaranth.group_code.as_deref(), Some("01"));
        assert_eq!(amaranth.index_code.as_deref(), Some("0001"));
        assert_eq!(amaranth.values["PROTCAA"], "(11.3)");
        assert_eq!(amaranth.values["ENERC_KCAL"], "343");
        assert_eq!(amaranth.markers["CHOAVLM"], "*");
    }

    #[test]
    fn rejects_rows_that_are_not_foods() {
        let table = parse_table(&fixtures::main_sheet()).unwrap();
        let reasons: Vec<_> = table
            .row_issues
            .iter()
            .map(|i| (i.food_code.clone(), i.reason.clone()))
            .collect();
        assert!(reasons
            .iter()
            .any(|(c, r)| c.as_deref() == Some("1002")
                && r.contains("5 digits")));
        assert_eq!(
            reasons
                .iter()
                .filter(|(c, r)| c.as_deref() == Some("99999")
                    && r == "duplicate food number")
                .count(),
            2
        );
        assert_eq!(table.rows_read, 8);
    }

    #[test]
    fn group_names_come_from_sheet_names() {
        let mut book = Workbook::default();
        for name in ["表全体", "1穀類", "18調理済み流通食品類"]
        {
            book.sheets.push(Sheet::new(name, vec![]));
        }
        let cats = categories_from_sheet_names(&book).unwrap();
        assert_eq!(cats["01"], "穀類");
        assert_eq!(cats["18"], "調理済み流通食品類");
        assert_eq!(cats.len(), 2);

        let duplicates = Workbook {
            sheets: vec![
                Sheet::new("1穀類", vec![]),
                Sheet::new("01別名", vec![]),
            ],
        };
        assert!(categories_from_sheet_names(&duplicates)
            .unwrap_err()
            .contains("normalize to the same code"));
    }

    #[test]
    fn errata_labels_resolve_to_component_identifiers() {
        let table = parse_table(&fixtures::main_sheet()).unwrap();
        let index = label_index(&table.layout);
        let find = |l: &str| index.get(&match_key(l)).cloned();
        assert_eq!(find("エネルギー\u{3000}kJ").as_deref(), Some("ENERC"));
        assert_eq!(
            find("エネルギー\u{3000}kcal").as_deref(),
            Some("ENERC_KCAL")
        );
        assert_eq!(find("食物繊維総量").as_deref(), Some("FIB-"));
        assert_eq!(
            find("利用可能炭水化物（差引き法による利用可能炭水化物）")
                .as_deref(),
            Some("CHOAVLDF-")
        );
        assert_eq!(
            find("利用可能炭水化物\n（単糖当量）").as_deref(),
            Some("CHOAVLM")
        );
        // Two columns are called エネルギー; the bare label is ambiguous.
        assert_eq!(find("エネルギー"), None);
    }
}
