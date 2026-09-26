//! The 正誤表 (errata) workbook and how it is applied.
//!
//! MEXT republishes the main Excel with corrections folded in, so most
//! entries are normally found already applied. Every entry is compared
//! with the table before anything changes:
//!
//! - the table shows the correct (`正`) value → `already_applied`
//! - the table shows the wrong (`誤`) value → `applied`, before/after kept
//! - anything else → `conflict`; the table value is not trusted
//!
//! so running the same errata twice, or on a table that already has them,
//! never changes a value twice.

use std::collections::BTreeMap;

use ingredient_notation::NutrientValueStatus;
use serde::Serialize;

use super::sheet::{match_key, squash, Sheet, Workbook};
use super::table::{label_index, parse_layout, SourceFood, TableLayout};

/// Sheet with one row per corrected item of the main table.
pub const ITEMIZED_SHEET: &str = "本表第2章";
/// Sheet with 誤/正 row pairs for foods corrected in many columns.
pub const ROW_PAIR_SHEET: &str = "本表";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "key", rename_all = "snake_case")]
pub enum ErrataField {
    Nutrient(String),
    /// The `*` energy-calculation marker next to a nutrient, possibly with
    /// a changed value (`3.9` → `6.1*`).
    Marker(String),
    Name,
    Remarks,
    Refuse,
    /// `各成分`: see the row pair sheet.
    WholeRow,
}

impl ErrataField {
    pub fn label(&self) -> String {
        match self {
            Self::Nutrient(k) => k.clone(),
            Self::Marker(k) => format!("{k}+marker"),
            Self::Name => "name".into(),
            Self::Remarks => "remarks".into(),
            Self::Refuse => "refuse_rate".into(),
            Self::WholeRow => "whole_row".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrataEntry {
    pub sheet: String,
    /// 1-based spreadsheet row.
    pub row: usize,
    pub food_code: String,
    /// `項目等` as printed.
    pub item: String,
    /// `None` when the item could not be placed in the table.
    pub field: Option<ErrataField>,
    pub wrong: Option<String>,
    pub right: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkippedSheet {
    pub sheet: String,
    pub entries: usize,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ErrataBook {
    /// Date printed on the errata, ISO form (`2026-03-27`).
    pub date: Option<String>,
    pub entries: Vec<ErrataEntry>,
    pub skipped: Vec<SkippedSheet>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrataStatus {
    Applied,
    AlreadyApplied,
    /// The table showed neither the wrong nor the right value; the right
    /// value was applied anyway (decided 2026-09-25: the errata wins).
    AppliedOverConflict,
    /// A text correction whose fragment could not be placed in the
    /// table's text. Nothing was changed.
    Conflict,
    Unresolved,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrataOutcome {
    pub sheet: String,
    pub row: usize,
    pub food_code: String,
    pub item: String,
    pub field: String,
    pub wrong: Option<String>,
    pub right: Option<String>,
    /// Table value before this entry was considered.
    pub before: Option<String>,
    /// Table value after it.
    pub after: Option<String>,
    pub status: ErrataStatus,
    /// Where the correction comes from, e.g. `正誤表 2026-03-27 本表第2章!R6`.
    pub basis: String,
    pub detail: Option<String>,
}

// ==================== reading ====================

pub fn parse_errata(
    book: &Workbook,
    table: &TableLayout,
) -> Result<ErrataBook, String> {
    let labels = label_index(table);
    let mut out = ErrataBook::default();
    for sheet in &book.sheets {
        if out.date.is_none() {
            out.date = find_date(sheet);
        }
        match sheet.name.as_str() {
            ITEMIZED_SHEET => {
                out.entries.extend(parse_itemized(sheet, &labels)?)
            }
            ROW_PAIR_SHEET => {
                out.entries.extend(parse_row_pairs(sheet)?)
            },
            other => {
                let reason = if other.contains("第1章") {
                    "chapter 1 text (cooking conditions etc.), not table data"
                } else {
                    "errata for another table (amino acid / fatty acid / carbohydrate editions); not imported"
                };
                out.skipped.push(SkippedSheet {
                    sheet: other.to_string(),
                    entries: count_entries(sheet),
                    reason: reason.into(),
                });
            }
        }
    }
    Ok(out)
}

fn text(sheet: &Sheet, r: usize, c: Option<usize>) -> Option<String> {
    c.and_then(|c| sheet.cell(r, c))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

struct ItemizedHeader {
    row: usize,
    target: Option<usize>,
    code: usize,
    item: usize,
    wrong: usize,
    right: usize,
    note: Option<usize>,
}

fn itemized_header(sheet: &Sheet) -> Option<ItemizedHeader> {
    (0..sheet.height().min(20)).find_map(|r| {
        let find = |want: &str| {
            (0..sheet.width()).find(|&c| {
                sheet.cell(r, c).is_some_and(|v| match_key(v) == want)
            })
        };
        Some(ItemizedHeader {
            row: r,
            target: find("変更対象"),
            code: find("食品番号")?,
            item: find("項目等")?,
            wrong: find("誤")?,
            right: find("正")?,
            note: find("備考"),
        })
    })
}

fn count_entries(sheet: &Sheet) -> usize {
    match itemized_header(sheet) {
        Some(h) => ((h.row + 1)..sheet.height())
            .filter(|&r| text(sheet, r, Some(h.code)).is_some())
            .count(),
        None => 0,
    }
}

fn parse_itemized(
    sheet: &Sheet,
    labels: &BTreeMap<String, String>,
) -> Result<Vec<ErrataEntry>, String> {
    let Some(h) = itemized_header(sheet) else {
        return Err(format!(
            concat!(
                "sheet {:?} is missing one or more required headers: ",
                "食品番号, 項目等, 誤, 正"
            ),
            sheet.name
        ));
    };
    let mut entries = Vec::new();
    for r in (h.row + 1)..sheet.height() {
        let Some(food_code) = text(sheet, r, Some(h.code)) else {
            continue;
        };
        let item = text(sheet, r, Some(h.item)).unwrap_or_default();
        let target = text(sheet, r, h.target).unwrap_or_default();
        let field =
            if !target.is_empty() && !squash(&target).contains("本表") {
                None
            } else {
                resolve_item(&item, labels)
            };
        entries.push(ErrataEntry {
            sheet: sheet.name.clone(),
            row: r + 1,
            food_code,
            item,
            field,
            wrong: text(sheet, r, Some(h.wrong)),
            right: text(sheet, r, Some(h.right)),
            note: text(sheet, r, h.note),
        });
    }
    entries
}

/// Place an errata item (`項目等`) in the table. Unknown wording yields
/// `None`; the entry is then reported, never guessed.
pub fn resolve_item(
    item: &str,
    labels: &BTreeMap<String, String>,
) -> Option<ErrataField> {
    let key = match_key(item);
    match key.as_str() {
        "食品名" => return Some(ErrataField::Name),
        "備考" => return Some(ErrataField::Remarks),
        "廃棄率" => return Some(ErrataField::Refuse),
        "各成分" => return Some(ErrataField::WholeRow),
        _ => {}
    }
    if let Some(base) = key.strip_suffix("アスタリスク") {
        return lookup(base, labels).map(ErrataField::Marker);
    }
    lookup(&key, labels).map(ErrataField::Nutrient)
}

fn lookup(key: &str, labels: &BTreeMap<String, String>) -> Option<String> {
    if let Some(k) = labels.get(key) {
        return Some(k.clone());
    }
    // `親（子）`: try the inner label on its own.
    let inner = key.strip_suffix(')')?.split_once('(')?.1;
    labels.get(inner).cloned()
}

/// `本表`: the table layout again, one column to the right, with rows in
/// 誤/正 pairs. Each differing column becomes one entry.
fn parse_row_pairs(sheet: &Sheet) -> Result<Vec<ErrataEntry>, String> {
    let layout = parse_layout(sheet)?;
    let marker_col = |r: usize| {
        (0..layout.code_col).find_map(|c| {
            sheet
                .cell(r, c)
                .map(squash)
                .filter(|v| v == "誤" || v == "正")
        })
    };
    let mut entries = Vec::new();
    let mut pending: Option<usize> = None;
    for r in (layout.first_data_row - 1)..sheet.height() {
        match marker_col(r).as_deref() {
            Some("誤") => {
                if let Some(unmatched) = pending.replace(r) {
                    return Err(format!(
                        "row {} has 誤 without a matching 正 row",
                        unmatched + 1
                    ));
                }
            }
            Some("正") => {
                let w = pending.take().ok_or_else(|| {
                    format!("row {} has 正 without a preceding 誤 row", r + 1)
                })?;
                let wrong_code = text(sheet, w, Some(layout.code_col));
                let right_code = text(sheet, r, Some(layout.code_col));
                if right_code.is_none() || right_code != wrong_code {
                    return Err(format!(
                        "errata rows {}-{} have mismatched food numbers: 誤={wrong_code:?}, 正={right_code:?}",
                        w + 1,
                        r + 1
                    ));
                }
                let code = right_code.unwrap_or_default();
                let mut fields: Vec<(ErrataField, Option<usize>, String)> = vec![
                    (
                        ErrataField::Name,
                        Some(layout.name_col),
                        "食品名".into(),
                    ),
                    (
                        ErrataField::Refuse,
                        layout.refuse_col,
                        "廃棄率".into(),
                    ),
                ];
                for n in &layout.nutrients {
                    fields.push((
                        ErrataField::Nutrient(n.key.clone()),
                        Some(n.col),
                        n.name.clone(),
                    ));
                }
                for (col, owner) in &layout.markers {
                    fields.push((
                        ErrataField::Marker(owner.clone()),
                        Some(*col),
                        format!("{owner} アスタリスク"),
                    ));
                }
                fields.push((
                    ErrataField::Remarks,
                    layout.remarks_col,
                    "備考".into(),
                ));
                for (field, col, item) in fields {
                    let wrong = text(sheet, w, col);
                    let right = text(sheet, r, col);
                    if wrong == right {
                        continue;
                    }
                    // A marker cell holds only `*`; carry it the way the
                    // itemized sheet writes it so both apply the same way.
                    let (wrong, right) = if let ErrataField::Marker(owner) =
                        &field
                    {
                        let value = |row| {
                            layout
                                .nutrient(owner)
                                .and_then(|n| text(sheet, row, Some(n.col)))
                                .unwrap_or_default()
                        };
                        (
                            Some(format!(
                                "{}{}",
                                value(w),
                                wrong.unwrap_or_default()
                            )),
                            Some(format!(
                                "{}{}",
                                value(r),
                                right.unwrap_or_default()
                            )),
                        )
                    } else {
                        (wrong, right)
                    };
                    entries.push(ErrataEntry {
                        sheet: sheet.name.clone(),
                        row: r + 1,
                        food_code: code.clone(),
                        item,
                        field: Some(field),
                        wrong,
                        right,
                        note: Some(format!(
                            "誤 R{} / 正 R{}",
                            w + 1,
                            r + 1
                        )),
                    });
                }
            }
            _ => {}
        }
    }
    if let Some(unmatched) = pending {
        return Err(format!(
            "row {} has 誤 without a matching 正 row",
            unmatched + 1
        ));
    }
    Ok(entries)
}

/// `令和8年3月27日` → `2026-03-27`.
fn find_date(sheet: &Sheet) -> Option<String> {
    for r in 0..sheet.height().min(10) {
        for c in 0..sheet.width() {
            if let Some(d) = sheet.cell(r, c).and_then(parse_reiwa_date) {
                return Some(d);
            }
        }
    }
    None
}

pub fn parse_reiwa_date(s: &str) -> Option<String> {
    let s = squash(s);
    let rest = s.strip_prefix("令和")?;
    let (y, rest) = rest.split_once('年')?;
    let (m, rest) = rest.split_once('月')?;
    let (d, _) = rest.split_once('日')?;
    let y: u32 = if y == "元" { 1 } else { y.parse().ok()? };
    let (m, d): (u32, u32) = (m.parse().ok()?, d.parse().ok()?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some(format!("{:04}-{m:02}-{d:02}", 2018 + y))
}

// ==================== applying ====================

/// Footnote marks printed after a value (`20.3†`). The value is read
/// without them; the raw notation keeps them.
pub const FOOTNOTE_MARKS: [char; 2] = ['†', '‡'];

pub fn strip_footnotes(raw: &str) -> &str {
    raw.trim().trim_end_matches(FOOTNOTE_MARKS).trim_end()
}

/// Whether two cells say the same thing: `12` and `12.0` do, `0` and
/// `(0)` do not. Text that is not a notation is compared verbatim.
pub fn same_value(a: Option<&str>, b: Option<&str>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            let (a, b) = (strip_footnotes(a), strip_footnotes(b));
            match (
                NutrientValueStatus::from_notation(a),
                NutrientValueStatus::from_notation(b),
            ) {
                (Ok(x), Ok(y)) => x == y,
                _ => squash(a) == squash(b),
            }
        }
        _ => false,
    }
}

enum Check {
    Already,
    Apply(Option<String>),
    /// Neither wrong nor right: apply the right value regardless.
    Override(Option<String>),
    Conflict,
}

fn check_value(
    current: Option<&str>,
    wrong: Option<&str>,
    right: Option<&str>,
) -> Check {
    if same_value(current, right) {
        Check::Already
    } else if same_value(current, wrong) {
        Check::Apply(right.map(str::to_string))
    } else {
        Check::Override(right.map(str::to_string))
    }
}

/// Where a corrected fragment belongs in text that shows neither the
/// wrong nor the right fragment: the equally long stretch that differs
/// from it in the fewest characters, if it differs in at most a quarter
/// of them (`…含まれている油で調理` for `…含まれている脂で調理`).
fn place_fragment(cur: &str, right: &str) -> Option<String> {
    let c: Vec<char> = cur.chars().collect();
    let r: Vec<char> = right.chars().collect();
    if r.is_empty() || r.len() > c.len() {
        return None;
    }
    let limit = (r.len() / 4).max(1);
    let (start, distance) = (0..=c.len() - r.len())
        .map(|i| {
            let d = c[i..i + r.len()]
                .iter()
                .zip(&r)
                .filter(|(a, b)| a != b)
                .count();
            (i, d)
        })
        .min_by_key(|(_, d)| *d)?;
    if distance > limit {
        return None;
    }
    let mut out: String = c[..start].iter().collect();
    out.push_str(right);
    out.extend(&c[start + r.len()..]);
    Some(out)
}

/// Text fields: the errata may quote only the changed fragment of a
/// multi-line remark or a long name.
fn check_text(
    current: Option<&str>,
    wrong: Option<&str>,
    right: Option<&str>,
    multi_line: bool,
) -> Check {
    match check_text_strict(current, wrong, right) {
        Check::Conflict => {}
        other => return other,
    }
    let cur = current.map(str::trim).unwrap_or("");
    let r = right.map(str::trim).unwrap_or("");
    if cur.is_empty() || r.is_empty() {
        return Check::Override((!r.is_empty()).then(|| r.to_string()));
    }
    if let Some(placed) = place_fragment(cur, r) {
        return Check::Override(Some(placed));
    }
    if multi_line {
        // A remark line the table does not have at all: add it.
        return Check::Override(Some(format!("{cur}\n{r}")));
    }
    Check::Conflict
}

fn check_text_strict(
    current: Option<&str>,
    wrong: Option<&str>,
    right: Option<&str>,
) -> Check {
    let cur = current.map(str::trim).unwrap_or("");
    let w = wrong.map(str::trim).unwrap_or("");
    let r = right.map(str::trim).unwrap_or("");
    if cur == r {
        return Check::Already;
    }
    if cur == w {
        return Check::Apply((!r.is_empty()).then(|| r.to_string()));
    }
    if w.is_empty() {
        return if !r.is_empty() && cur.contains(r) {
            Check::Already
        } else {
            Check::Conflict
        };
    }
    let replaced = || Check::Apply(Some(cur.replacen(w, r, 1)));
    // When the right text extends the wrong one, finding the wrong text
    // inside the right one must not count as "still wrong".
    if !r.is_empty() && r.contains(w) {
        if cur.contains(r) {
            Check::Already
        } else if cur.contains(w) {
            replaced()
        } else {
            Check::Conflict
        }
    } else if cur.contains(w) {
        replaced()
    } else if !r.is_empty() && cur.contains(r) {
        Check::Already
    } else {
        Check::Conflict
    }
}

fn has_marker(s: Option<&str>) -> bool {
    s.is_some_and(|s| s.contains('*'))
}

fn without_marker(s: Option<&str>) -> Option<String> {
    s.map(|s| s.replace('*', "").trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Apply the errata to the parsed foods in place and say what happened
/// to every entry.
pub fn apply_errata(
    foods: &mut [SourceFood],
    book: &ErrataBook,
) -> Vec<ErrataOutcome> {
    let date = book.date.clone().unwrap_or_else(|| "undated".into());
    let pair_codes: Vec<&str> = book
        .entries
        .iter()
        .filter(|e| e.sheet == ROW_PAIR_SHEET)
        .map(|e| e.food_code.as_str())
        .collect();
    let mut outcomes = Vec::new();

    for e in &book.entries {
        let mut outcome = ErrataOutcome {
            sheet: e.sheet.clone(),
            row: e.row,
            food_code: e.food_code.clone(),
            item: e.item.clone(),
            field: e
                .field
                .as_ref()
                .map(ErrataField::label)
                .unwrap_or_default(),
            wrong: e.wrong.clone(),
            right: e.right.clone(),
            before: None,
            after: None,
            status: ErrataStatus::Unresolved,
            basis: format!("正誤表 {date} {}!R{}", e.sheet, e.row),
            detail: e.note.clone(),
        };
        let Some(field) = &e.field else {
            outcome.detail = Some(format!(
                "item {:?} does not match a table column",
                e.item
            ));
            outcomes.push(outcome);
            continue;
        };
        if *field == ErrataField::WholeRow {
            outcome.status = if pair_codes.contains(&e.food_code.as_str()) {
                ErrataStatus::AlreadyApplied
            } else {
                ErrataStatus::Unresolved
            };
            outcome.detail = Some(
                if outcome.status == ErrataStatus::Unresolved {
                    format!(
                        "sheet {ROW_PAIR_SHEET} has no row pair for {}",
                        e.food_code
                    )
                } else {
                    format!("expanded from sheet {ROW_PAIR_SHEET}; see its entries")
                },
            );
            outcomes.push(outcome);
            continue;
        }
        let Some(food) =
            foods.iter_mut().find(|f| f.food_code == e.food_code)
        else {
            outcome.detail = Some("food number is not in the table".into());
            outcomes.push(outcome);
            continue;
        };
        let (wrong, right) = (e.wrong.as_deref(), e.right.as_deref());

        let (before, check) = match field {
            ErrataField::Nutrient(k) => {
                let cur = food.values.get(k).cloned();
                let check = check_value(cur.as_deref(), wrong, right);
                (cur, check)
            }
            ErrataField::Refuse => {
                let cur = food.refuse.clone();
                let check = check_value(cur.as_deref(), wrong, right);
                (cur, check)
            }
            ErrataField::Name => {
                let cur = Some(food.name.clone());
                let check = check_text(cur.as_deref(), wrong, right, false);
                (cur, check)
            }
            ErrataField::Remarks => {
                let cur = food.remarks.clone();
                let check = check_text(cur.as_deref(), wrong, right, true);
                (cur, check)
            }
            ErrataField::Marker(k) => {
                let cur_value = food.values.get(k).cloned();
                let cur_marker = food.markers.get(k).cloned();
                let marker_now = has_marker(cur_marker.as_deref());
                let value_check = check_value(
                    cur_value.as_deref(),
                    without_marker(wrong).as_deref(),
                    without_marker(right).as_deref(),
                );
                let marker_check = if marker_now == has_marker(right) {
                    Check::Already
                } else {
                    Check::Apply(has_marker(right).then(|| "*".to_string()))
                };
                let before = Some(format!(
                    "{}{}",
                    cur_value.clone().unwrap_or_default(),
                    cur_marker.clone().unwrap_or_default()
                ));
                let over_conflict =
                    matches!(value_check, Check::Override(_));
                let check = match (value_check, marker_check) {
                    (Check::Conflict, _) => Check::Conflict,
                    (Check::Already, Check::Already) => Check::Already,
                    (v, m) => {
                        if let Check::Apply(v) | Check::Override(v) = v {
                            match v {
                                Some(v) => food.values.insert(k.clone(), v),
                                None => food.values.remove(k),
                            };
                        }
                        if let Check::Apply(m) = m {
                            match m {
                                Some(m) => {
                                    food.markers.insert(k.clone(), m)
                                }
                                None => food.markers.remove(k),
                            };
                        }
                        let after = Some(format!(
                            "{}{}",
                            food.values.get(k).cloned().unwrap_or_default(),
                            food.markers
                                .get(k)
                                .cloned()
                                .unwrap_or_default()
                        ));
                        if over_conflict {
                            Check::Override(after)
                        } else {
                            Check::Apply(after)
                        }
                    }
                };
                outcome.detail = Some(
                    "the `*` marker itself is not stored (marker columns are not imported)".into(),
                );
                (before, check)
            }
            ErrataField::WholeRow => unreachable!(),
        };

        outcome.before = before.clone();
        let over_conflict = matches!(check, Check::Override(_));
        match check {
            Check::Already => {
                outcome.status = ErrataStatus::AlreadyApplied;
                outcome.after = before;
            }
            Check::Conflict => {
                outcome.status = ErrataStatus::Conflict;
                outcome.after = before;
                outcome.detail = Some(format!(
                    "table shows {:?}, which is neither the wrong nor the right value, and the corrected text could not be placed in it",
                    outcome.before.clone().unwrap_or_default()
                ));
            }
            Check::Apply(after) | Check::Override(after) => {
                if over_conflict {
                    outcome.status = ErrataStatus::AppliedOverConflict;
                    outcome.detail = Some(format!(
                        "table showed {:?}, which is neither the wrong nor the right value; the errata's right value was applied",
                        outcome.before.clone().unwrap_or_default()
                    ));
                } else {
                    outcome.status = ErrataStatus::Applied;
                }
                match field {
                    ErrataField::Nutrient(k) => match &after {
                        Some(v) => {
                            food.values.insert(k.clone(), v.clone());
                        }
                        None => {
                            food.values.remove(k);
                        }
                    },
                    ErrataField::Refuse => food.refuse = after.clone(),
                    ErrataField::Name => {
                        if let Some(v) = &after {
                            food.name = v.clone();
                        }
                    }
                    ErrataField::Remarks => food.remarks = after.clone(),
                    ErrataField::Marker(_) | ErrataField::WholeRow => {}
                }
                outcome.after = after;
            }
        }
        outcomes.push(outcome);
    }
    outcomes
}

#[cfg(test)]
mod tests {
    use super::super::fixtures;
    use super::super::table::parse_table;
    use super::*;

    fn setup() -> (Vec<SourceFood>, ErrataBook) {
        let table = parse_table(&fixtures::main_sheet()).unwrap();
        let book =
            parse_errata(&fixtures::errata_book(), &table.layout).unwrap();
        (table.foods, book)
    }

    fn outcome<'a>(
        outcomes: &'a [ErrataOutcome],
        code: &str,
        field: &str,
    ) -> &'a ErrataOutcome {
        outcomes
            .iter()
            .find(|o| o.food_code == code && o.field == field)
            .unwrap_or_else(|| panic!("no outcome for {code} {field}"))
    }

    #[test]
    fn reads_the_date_and_skips_other_tables() {
        let (_, book) = setup();
        assert_eq!(book.date.as_deref(), Some("2026-03-27"));
        let skipped: Vec<_> = book
            .skipped
            .iter()
            .map(|s| (s.sheet.as_str(), s.entries))
            .collect();
        assert_eq!(skipped, vec![("本表第1章", 1), ("ア第2章", 1)]);
    }

    #[test]
    fn applies_only_what_the_table_still_gets_wrong() {
        let (mut foods, book) = setup();
        let outcomes = apply_errata(&mut foods, &book);
        let status = |code, field| outcome(&outcomes, code, field).status;

        assert_eq!(status("01001", "PROT-"), ErrataStatus::AlreadyApplied);
        let kcal = outcome(&outcomes, "06153", "ENERC_KCAL");
        assert_eq!(kcal.status, ErrataStatus::Applied);
        assert_eq!(kcal.before.as_deref(), Some("33"));
        assert_eq!(kcal.after.as_deref(), Some("31"));
        assert_eq!(kcal.basis, "正誤表 2026-03-27 本表第2章!R6");
        let onion = foods.iter().find(|f| f.food_code == "06153").unwrap();
        assert_eq!(onion.values["ENERC_KCAL"], "31");
        assert!(onion
            .remarks
            .as_deref()
            .unwrap()
            .contains("オニオン、玉葱"));

        let over = outcome(&outcomes, "10330", "VITK");
        assert_eq!(over.status, ErrataStatus::AppliedOverConflict);
        assert_eq!(over.before.as_deref(), Some("7"));
        assert_eq!(over.after.as_deref(), Some("-"));
        assert_eq!(over.basis, "正誤表 2026-03-27 本表第2章!R8");
        let aji = foods.iter().find(|f| f.food_code == "10330").unwrap();
        assert_eq!(aji.values["VITK"], "-");
        assert_eq!(status("10330", "name"), ErrataStatus::AlreadyApplied);
        assert_eq!(
            status("01001", "CHOAVLM+marker"),
            ErrataStatus::AlreadyApplied
        );
        assert!(outcomes.iter().any(|o| o.item == "ほげ成分"
            && o.status == ErrataStatus::Unresolved));
        assert_eq!(
            status("06154", "whole_row"),
            ErrataStatus::AlreadyApplied
        );

        // Row pair 本表: ENERC/ENERC_KCAL/NA already right, PROT- applied.
        assert_eq!(status("06154", "ENERC"), ErrataStatus::AlreadyApplied);
        assert_eq!(status("06154", "NA"), ErrataStatus::AlreadyApplied);
        assert_eq!(status("06154", "PROT-"), ErrataStatus::Applied);
        let boiled = foods.iter().find(|f| f.food_code == "06154").unwrap();
        assert_eq!(boiled.values["PROT-"], "0.9");
    }

    #[test]
    fn a_second_pass_changes_nothing() {
        let (mut foods, book) = setup();
        apply_errata(&mut foods, &book);
        let after_first = foods.clone();
        let second = apply_errata(&mut foods, &book);
        assert_eq!(foods, after_first);
        assert!(second.iter().all(|o| !matches!(
            o.status,
            ErrataStatus::Applied | ErrataStatus::AppliedOverConflict
        )));
    }

    #[test]
    fn value_comparison_keeps_notations_apart() {
        assert!(same_value(Some("12"), Some("12.0")));
        assert!(same_value(Some("20.3†"), Some("20.3")));
        assert!(!same_value(Some("0"), Some("(0)")));
        assert!(!same_value(Some("Tr"), Some("0")));
        assert!(!same_value(Some("-"), Some("0")));
        assert!(!same_value(Some("3.6"), Some("(3.6)")));
    }

    #[test]
    fn text_fragments_are_not_applied_twice() {
        // Right text extends the wrong one.
        let once = match check_text(
            Some("別名： オニオン"),
            Some("オニオン"),
            Some("オニオン、玉葱"),
            true,
        ) {
            Check::Apply(Some(v)) => v,
            _ => panic!("expected apply"),
        };
        assert_eq!(once, "別名： オニオン、玉葱");
        assert!(matches!(
            check_text(
                Some(&once),
                Some("オニオン"),
                Some("オニオン、玉葱"),
                true
            ),
            Check::Already
        ));
        // Wrong text extends the right one (a deleted fragment).
        assert!(matches!(
            check_text(
                Some("a\n植物油（調合油）"),
                Some("植物油（調合油）： 4.1 g"),
                Some("植物油（調合油）"),
                true
            ),
            Check::Already
        ));
    }

    #[test]
    fn a_remark_in_conflict_gets_the_right_text_once() {
        // 11316: the Excel says 油, the errata's 正 says 脂, and 誤 is gone.
        let cur = "別名：ベーコン\nヨウ素： 第3章参照\nばらベーコンに含まれている油で調理";
        let wrong = Some("調理による脂質の増減：第1章表14参照");
        let right = Some("ばらベーコンに含まれている脂で調理");
        let fixed = match check_text(Some(cur), wrong, right, true) {
            Check::Override(Some(v)) => v,
            _ => panic!("expected an override"),
        };
        assert_eq!(
            fixed,
            "別名：ベーコン\nヨウ素： 第3章参照\nばらベーコンに含まれている脂で調理"
        );
        assert!(matches!(
            check_text(Some(&fixed), wrong, right, true),
            Check::Already
        ));
        // A remark line the table lacks entirely is appended.
        assert!(matches!(
            check_text(Some("別名：ベーコン"), wrong, Some("全く別の注記"), true),
            Check::Override(Some(v)) if v == "別名：ベーコン\n全く別の注記"
        ));
        // A name fragment that fits nowhere is left alone and reported.
        assert!(matches!(
            check_text(
                Some("まあじ 生"),
                Some("半固形状"),
                Some("半固体状ドレッシング"),
                false
            ),
            Check::Conflict
        ));
    }

    #[test]
    fn a_value_in_conflict_takes_the_right_value() {
        assert!(matches!(
            check_value(Some("7"), Some("5"), Some("-")),
            Check::Override(Some(v)) if v == "-"
        ));
        assert!(matches!(
            check_value(Some("-"), Some("5"), Some("-")),
            Check::Already
        ));
    }

    #[test]
    fn reiwa_dates() {
        assert_eq!(
            parse_reiwa_date("令和8年3月27日").as_deref(),
            Some("2026-03-27")
        );
        assert_eq!(
            parse_reiwa_date("令和元年5月1日").as_deref(),
            Some("2019-05-01")
        );
        assert_eq!(parse_reiwa_date("2026年"), None);
    }
}
