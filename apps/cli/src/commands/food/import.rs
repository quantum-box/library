//! Orchestration of `library food import`: read, correct, validate,
//! preview, and — only with `--apply` — write.
//!
//! Every run leaves a directory under `--state-dir`:
//!
//! - `manifest.json`   provenance (URLs, SHA-256, retrieval time, sheet,
//!   header rows, importer version) and the run status
//! - `report.json`     the preview: counts, special values, ignored
//!   columns, errata outcomes, add/change/delete per repo
//! - `quarantine.jsonl` rows, cells and errata left out, with reasons
//! - `writes.jsonl`    one line per upsert attempt (ok / failed)
//!
//! A run is resumed by running the same command again: the plan is
//! recomputed against the repos, so records already written show up as
//! unchanged and only the rest is sent. The status is `completed` only
//! when every planned write succeeded; nothing here publishes a release.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use futures::{stream, StreamExt};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::catalog::{
    build_catalog, prop, BuildOptions, DeferredCell, RecordKind,
    TargetCatalog, TargetRecord,
};
use super::errata::{
    apply_errata, parse_errata, ErrataBook, ErrataOutcome, ErrataStatus,
};
use super::plan::{plan_repo, Action, PlannedWrite, RepoPlan};
use super::sheet::Workbook;
use super::store::{
    encode_value, PropertyDef, Store, UpsertStatus, WriteError,
};
use super::table::{
    categories_from_sheet_names, parse_table, IgnoredColumn, SourceTable,
    EXPECTED_UNITS,
};
use super::{ImportArgs, MEXT_PAGE_URL};
use crate::output::{print_json, Format};

pub const IMPORTER_FORMAT: &str = "library-food-import/v1";
const SOURCE_NAME: &str = "日本食品標準成分表（八訂）増補2023年";
const SOURCE_PUBLISHER: &str =
    "文部科学省 科学技術・学術審議会 資源調査分科会";
const BASIS: &str = "可食部100g当たり";
const EXPECTED_FOOD_COUNT: usize = 2_538;

// ==================== inputs ====================

#[derive(Debug, Clone, Serialize)]
pub struct SourceFile {
    pub role: &'static str,
    pub url: String,
    pub file_name: String,
    pub sha256: String,
    pub bytes: usize,
    pub retrieved_at: String,
}

impl SourceFile {
    fn new(
        role: &'static str,
        url: &str,
        path: &Path,
        bytes: &[u8],
        retrieved_at: &str,
    ) -> Self {
        Self {
            role,
            url: url.to_string(),
            file_name: path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            sha256: hex(&Sha256::digest(bytes)),
            bytes: bytes.len(),
            retrieved_at: retrieved_at.to_string(),
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Everything derived from the files, before any repo is read.
pub struct Prepared {
    pub files: Vec<SourceFile>,
    pub table: SourceTable,
    pub errata: Option<ErrataBook>,
    pub errata_outcomes: Vec<ErrataOutcome>,
    pub catalog: TargetCatalog,
    pub source_release: String,
    pub categories: BTreeMap<String, String>,
}

pub struct PrepareInput<'a> {
    pub table_bytes: &'a [u8],
    pub table_file: SourceFile,
    pub errata: Option<(&'a [u8], SourceFile)>,
    pub sheet: &'a str,
    /// Enforce the complete fixed MEXT 2023 source layout and row count.
    pub validate_complete_source: bool,
    pub options: BuildOptions,
}

pub fn prepare(input: PrepareInput<'_>) -> Result<Prepared> {
    let book = Workbook::read_xlsx(input.table_bytes)
        .context("reading the table workbook")?;
    let sheet = book.sheet(input.sheet).with_context(|| {
        format!(
            "sheet {:?} not found; sheets: {}",
            input.sheet,
            book.sheet_names().join(", ")
        )
    })?;
    let mut table =
        parse_table(sheet).map_err(|e| anyhow::anyhow!("{e}"))?;
    if input.validate_complete_source {
        let actual_keys = table
            .layout
            .nutrients
            .iter()
            .map(|n| n.key.clone())
            .collect::<BTreeSet<_>>();
        let expected_keys = EXPECTED_UNITS
            .iter()
            .map(|(key, _)| (*key).to_string())
            .collect::<BTreeSet<_>>();
        for key in expected_keys.difference(&actual_keys) {
            table
                .layout
                .errors
                .push(format!("missing nutrient identifier {key:?}"));
        }
        for key in actual_keys.difference(&expected_keys) {
            table
                .layout
                .errors
                .push(format!("unexpected nutrient identifier {key:?}"));
        }
        if table.foods.len() != EXPECTED_FOOD_COUNT {
            table.layout.errors.push(format!(
                "expected {EXPECTED_FOOD_COUNT} food rows in the MEXT 2023 table, found {}",
                table.foods.len()
            ));
        }
    }
    let categories = categories_from_sheet_names(&book);

    let mut files = vec![input.table_file];
    let mut foods = table.foods.clone();
    let (errata, outcomes) = match input.errata {
        Some((bytes, file)) => {
            let errata_book = Workbook::read_xlsx(bytes)
                .context("reading the errata workbook")?;
            let parsed = parse_errata(&errata_book, &table.layout)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            if parsed.date.is_none() {
                return Err(anyhow::anyhow!(
                    "errata workbook is missing a recognized correction date"
                ));
            }
            let outcomes = apply_errata(&mut foods, &parsed);
            files.push(file);
            (Some(parsed), outcomes)
        }
        None => (None, Vec::new()),
    };
    let mut options = input.options;
    options.categories = categories.clone();
    let catalog = build_catalog(&table, &foods, &outcomes, &options);
    let source_release = match errata.as_ref().and_then(|e| e.date.clone())
    {
        Some(date) => format!("2023+errata-{date}"),
        None => "2023".to_string(),
    };
    Ok(Prepared {
        files,
        table,
        errata,
        errata_outcomes: outcomes,
        catalog,
        source_release,
        categories,
    })
}

// ==================== planning against the repos ====================

pub struct RepoTarget<'a> {
    pub kind: RecordKind,
    pub repo: &'a str,
    pub records: &'a [TargetRecord],
}

#[derive(Debug, Clone, Serialize)]
pub struct RepoReport {
    pub repo: String,
    pub kind: RecordKind,
    pub target_records: usize,
    pub existing_records: usize,
    pub create: usize,
    pub update: usize,
    pub unchanged: usize,
    pub key_conflicts: usize,
    pub delete_candidates: usize,
    /// property → number of records whose change touches it.
    pub changed_properties: BTreeMap<String, usize>,
    pub preserved_human_fields: BTreeMap<String, usize>,
    pub preserved_reviewed_fields: BTreeMap<String, usize>,
    pub skipped_optional_properties: Vec<String>,
    pub errors: Vec<String>,
}

pub struct RepoState {
    pub kind: RecordKind,
    pub repo: String,
    pub properties: BTreeMap<String, PropertyDef>,
    pub plan: RepoPlan,
    pub report: RepoReport,
}

/// Properties a repo must have: every one the import has a value for,
/// except the optional ones.
fn required_properties(records: &[TargetRecord]) -> BTreeSet<&'static str> {
    records
        .iter()
        .flat_map(|r| r.fields.iter())
        .filter(|f| {
            f.value.is_some() && !prop::OPTIONAL.contains(&f.property)
        })
        .map(|f| f.property)
        .collect()
}

fn type_ok(property: &str, def: &PropertyDef) -> bool {
    match def.property_type.to_ascii_uppercase().as_str() {
        "STRING" => true,
        "INTEGER" => property == prop::DISPLAY_ORDER,
        "BOOLEAN" => property == prop::DEFAULT_DISPLAY,
        _ => false,
    }
}

pub async fn plan_all<S: Store>(
    store: &S,
    targets: &[RepoTarget<'_>],
    withheld: &BTreeSet<String>,
    concurrency: usize,
) -> Result<Vec<RepoState>> {
    let mut out = Vec::new();
    for t in targets {
        let defs = store.properties(t.repo).await.with_context(|| {
            format!("reading the properties of {}", t.repo)
        })?;
        let relevant_properties: BTreeSet<String> = t
            .records
            .iter()
            .flat_map(|record| record.fields.iter())
            .map(|field| field.property.to_string())
            .collect();
        let mut properties = BTreeMap::new();
        let mut errors = Vec::new();
        for def in defs {
            let name = def.name.trim().to_string();
            if properties.contains_key(&name) {
                if relevant_properties.contains(&name) {
                    errors.push(format!(
                        "{} has duplicate property name {name:?}",
                        t.repo
                    ));
                }
                continue;
            }
            properties.insert(name, def);
        }
        for p in required_properties(t.records) {
            match properties.get(p) {
                None => {
                    errors.push(format!("{} has no `{p}` property", t.repo))
                }
                Some(d) if !type_ok(p, d) => errors.push(format!(
                    "{}: property `{p}` is {}, expected STRING",
                    t.repo, d.property_type
                )),
                _ => {}
            }
        }
        let optional_properties: BTreeSet<String> = t
            .records
            .iter()
            .flat_map(|record| record.fields.iter())
            .filter(|field| {
                field.value.is_some()
                    && prop::OPTIONAL.contains(&field.property)
            })
            .map(|field| field.property.to_string())
            .collect();
        for property in optional_properties {
            if let Some(def) = properties.get(&property) {
                if !type_ok(&property, def) {
                    errors.push(format!(
                        "{}: optional property `{property}` is {}, expected STRING",
                        t.repo, def.property_type
                    ));
                }
            }
        }
        let skip: BTreeSet<String> = prop::OPTIONAL
            .iter()
            .filter(|p| !properties.contains_key(**p))
            .map(|p| p.to_string())
            .collect();
        let existing = store
            .list(t.repo, concurrency)
            .await
            .with_context(|| format!("listing {}", t.repo))?;
        let plan = plan_repo(t.kind, t.records, &existing, withheld, &skip);

        let mut changed_properties: BTreeMap<String, usize> =
            BTreeMap::new();
        for w in &plan.writes {
            if let Action::Update { changed } = &w.action {
                for c in changed {
                    *changed_properties.entry(c.clone()).or_default() += 1;
                }
            }
        }
        if t.kind == RecordKind::Nutrient {
            for w in &plan.writes {
                if let Action::Update { changed } = &w.action {
                    if changed.iter().any(|c| c == prop::UNIT) {
                        errors.push(format!(
                            "unit mismatch: {} is stored with a different unit than the source ({}); fix it by hand before importing values",
                            w.business_key,
                            w.properties.get(prop::UNIT).cloned().unwrap_or_default()
                        ));
                    }
                }
            }
        }
        let report = RepoReport {
            repo: t.repo.to_string(),
            kind: t.kind,
            target_records: t.records.len(),
            existing_records: existing.len(),
            create: plan.creates(),
            update: plan.updates(),
            unchanged: plan.unchanged,
            key_conflicts: plan.conflicts.len(),
            delete_candidates: plan.delete_candidates.len(),
            changed_properties,
            preserved_human_fields: plan.preserved_human_fields.clone(),
            preserved_reviewed_fields: plan
                .preserved_reviewed_fields
                .clone(),
            skipped_optional_properties: skip.into_iter().collect(),
            errors,
        };
        out.push(RepoState {
            kind: t.kind,
            repo: t.repo.to_string(),
            properties,
            plan,
            report,
        });
    }
    Ok(out)
}

// ==================== writing ====================

#[derive(Debug, Clone, Serialize)]
pub struct WriteLog {
    pub at: String,
    pub repo: String,
    pub kind: RecordKind,
    pub data_id: String,
    pub business_key: String,
    pub action: &'static str,
    pub attempts: u32,
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ExecSummary {
    pub attempted: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub not_attempted: usize,
    pub stopped_early: Option<String>,
    pub elapsed_seconds: f64,
}

pub fn upsert_body(
    w: &PlannedWrite,
    properties: &BTreeMap<String, PropertyDef>,
) -> Result<Value> {
    let mut property_data = Vec::new();
    for (name, value) in &w.properties {
        let def = properties.get(name).with_context(|| {
            format!("no property `{name}` for {}", w.data_id)
        })?;
        let encoded = encode_value(def, value)?;
        if encoded.is_null() {
            continue;
        }
        property_data
            .push(json!({ "property_id": def.id, "value": encoded }));
    }
    Ok(json!({ "name": w.name, "property_data": property_data }))
}

async fn write_one<S: Store>(
    store: &S,
    repo: &str,
    w: &PlannedWrite,
    body: &Value,
    retry_delay: Duration,
) -> (u32, std::result::Result<UpsertStatus, WriteError>) {
    let mut attempt = 0;
    loop {
        attempt += 1;
        match store.upsert(repo, &w.data_id, body).await {
            Ok(s) => return (attempt, Ok(s)),
            Err(e) if e.retryable && attempt < 3 => {
                tokio::time::sleep(retry_delay * 2u32.pow(attempt - 1))
                    .await;
            }
            Err(e) => return (attempt, Err(e)),
        }
    }
}

/// Send the planned writes, nutrients → ingredients → values. A phase
/// with failures stops the next phases, so values are never written
/// against definitions that did not land.
pub async fn execute<S: Store>(
    store: &S,
    repos: &[RepoState],
    concurrency: usize,
    max_failures: usize,
    retry_delay: Duration,
    mut log: impl FnMut(&WriteLog),
) -> ExecSummary {
    let started = Instant::now();
    let mut summary = ExecSummary::default();
    let total: usize = repos.iter().map(|r| r.plan.writes.len()).sum();

    'phases: for kind in [
        RecordKind::Nutrient,
        RecordKind::Ingredient,
        RecordKind::Value,
    ] {
        let Some(state) = repos.iter().find(|r| r.kind == kind) else {
            continue;
        };
        let mut phase_failed = false;
        let mut jobs = stream::iter(state.plan.writes.iter())
            .map(|w| async move {
                let at = now();
                let result = match upsert_body(w, &state.properties) {
                    Ok(body) => {
                        write_one(store, &state.repo, w, &body, retry_delay)
                            .await
                    }
                    Err(e) => (
                        0,
                        Err(WriteError {
                            message: format!("{e:#}"),
                            retryable: false,
                        }),
                    ),
                };
                (w, at, result)
            })
            .buffer_unordered(concurrency.max(1));

        while let Some((w, at, (attempts, result))) = jobs.next().await {
            summary.attempted += 1;
            let entry = WriteLog {
                at,
                repo: state.repo.clone(),
                kind,
                data_id: w.data_id.clone(),
                business_key: w.business_key.clone(),
                action: match w.action {
                    Action::Create => "create",
                    Action::Update { .. } => "update",
                },
                attempts,
                status: if result.is_ok() { "ok" } else { "failed" },
                outcome: match &result {
                    Ok(UpsertStatus::Created) => Some("created"),
                    Ok(UpsertStatus::Updated) => Some("updated"),
                    Err(_) => None,
                },
                error: result.as_ref().err().map(|e| e.message.clone()),
            };
            log(&entry);
            match result {
                Ok(_) => summary.succeeded += 1,
                Err(_) => {
                    summary.failed += 1;
                    phase_failed = true;
                    if summary.failed >= max_failures {
                        summary.stopped_early = Some(format!(
                            "stopped after {} failed writes",
                            summary.failed
                        ));
                        break 'phases;
                    }
                }
            }
        }
        if phase_failed {
            summary.stopped_early = Some(format!(
                "{} writes failed; later phases were not started",
                kind.as_str()
            ));
            break;
        }
    }
    summary.not_attempted = total - summary.attempted;
    summary.elapsed_seconds = started.elapsed().as_secs_f64();
    summary
}

// ==================== report ====================

#[derive(Debug, Clone, Serialize)]
pub struct ErrataSummary {
    pub date: Option<String>,
    pub entries: usize,
    pub by_status: BTreeMap<String, usize>,
    pub skipped_sheets: Vec<super::errata::SkippedSheet>,
    pub outcomes: Vec<ErrataOutcome>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub mode: &'static str,
    pub generated_at: String,
    pub source_id: String,
    pub source_release: String,
    pub files: Vec<SourceFile>,
    pub sheet: String,
    pub header_rows: (usize, usize),
    pub identifier_row: usize,
    pub first_data_row: usize,
    pub updated_label: Option<String>,
    pub rows_read: usize,
    pub foods: usize,
    pub nutrients: usize,
    pub values: usize,
    pub categories: BTreeMap<String, String>,
    pub adopted_columns: Vec<String>,
    pub ignored_columns: Vec<IgnoredColumn>,
    pub layout_errors: Vec<String>,
    pub duplicate_food_codes: Vec<String>,
    pub unit_mismatches: Vec<String>,
    pub category_errors: Vec<String>,
    pub value_status_counts: BTreeMap<String, usize>,
    pub notation_counts: BTreeMap<String, usize>,
    /// Foods whose `cooking_state` was left blank for a person, grouped by
    /// the last word of the name that the importer does not map.
    pub cooking_state_review: CookingStateReview,
    /// `*` cells whose remark says `第3章参照` (iodine). No value record is
    /// written for them; a release reports them as `not_listed`.
    pub deferred_to_chapter3: Vec<DeferredCell>,
    pub quarantined: BTreeMap<String, usize>,
    pub errata: Option<ErrataSummary>,
    pub repos: Vec<RepoReport>,
    /// Why `--apply` would be refused, if it would.
    pub blockers: Vec<String>,
    pub needs_accept_quarantine: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct CookingStateReview {
    pub blank_foods: usize,
    /// Most frequent first.
    pub unmapped_last_words: Vec<UnmappedWord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UnmappedWord {
    pub word: String,
    pub count: usize,
    pub food_codes: Vec<String>,
}

fn cooking_state_review(
    unmapped: &BTreeMap<String, super::catalog::UnmappedState>,
) -> CookingStateReview {
    let mut words: Vec<UnmappedWord> = unmapped
        .iter()
        .map(|(word, u)| UnmappedWord {
            word: word.clone(),
            count: u.count,
            food_codes: u.food_codes.clone(),
        })
        .collect();
    words.sort_by(|a, b| b.count.cmp(&a.count).then(a.word.cmp(&b.word)));
    CookingStateReview {
        blank_foods: words.iter().map(|w| w.count).sum(),
        unmapped_last_words: words,
    }
}

pub fn build_report(
    p: &Prepared,
    sheet: &str,
    source_id: &str,
    repos: &[RepoState],
    mode: &'static str,
) -> Report {
    let layout = &p.table.layout;
    let mut quarantined: BTreeMap<String, usize> = BTreeMap::new();
    for q in &p.catalog.quarantine {
        *quarantined.entry(q.scope.to_string()).or_default() += 1;
    }
    let errata = p.errata.as_ref().map(|book| {
        let mut by_status: BTreeMap<String, usize> = BTreeMap::new();
        for o in &p.errata_outcomes {
            *by_status
                .entry(
                    serde_json::to_value(o.status)
                        .ok()
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_default(),
                )
                .or_default() += 1;
        }
        ErrataSummary {
            date: book.date.clone(),
            entries: book.entries.len(),
            by_status,
            skipped_sheets: book.skipped.clone(),
            outcomes: p.errata_outcomes.clone(),
        }
    });

    let mut blockers: Vec<String> = layout.errors.clone();
    blockers.extend(p.catalog.unit_mismatches.iter().cloned());
    blockers.extend(p.catalog.category_errors.iter().cloned());
    for r in repos {
        blockers.extend(r.report.errors.iter().cloned());
    }
    if p.catalog.values.is_empty() {
        blockers.push("the import produced no values".into());
    }

    let mut needs_accept = Vec::new();
    if !p.catalog.quarantine.is_empty() {
        needs_accept.push(format!(
            "{} rows/cells quarantined",
            p.catalog.quarantine.len()
        ));
    }
    let errata_problems = p
        .errata_outcomes
        .iter()
        .filter(|o| {
            matches!(
                o.status,
                ErrataStatus::Conflict | ErrataStatus::Unresolved
            )
        })
        .count();
    if errata_problems > 0 {
        needs_accept.push(format!(
            "{errata_problems} errata entries in conflict or unresolved"
        ));
    }
    let conflicts: usize =
        repos.iter().map(|r| r.report.key_conflicts).sum();
    if conflicts > 0 {
        needs_accept.push(format!(
            "{conflicts} records skipped: key held by another record"
        ));
    }

    Report {
        mode,
        generated_at: now(),
        source_id: source_id.to_string(),
        source_release: p.source_release.clone(),
        files: p.files.clone(),
        sheet: sheet.to_string(),
        header_rows: layout.header_rows,
        identifier_row: layout.identifier_row,
        first_data_row: layout.first_data_row,
        updated_label: layout.updated_label.clone(),
        rows_read: p.table.rows_read,
        foods: p.catalog.ingredients.len(),
        nutrients: p.catalog.nutrients.len(),
        values: p.catalog.values.len(),
        categories: p.categories.clone(),
        adopted_columns: layout
            .nutrients
            .iter()
            .map(|n| format!("{} {} [{}]", n.key, n.name, n.unit))
            .collect(),
        ignored_columns: layout.ignored.clone(),
        layout_errors: layout.errors.clone(),
        duplicate_food_codes: p
            .table
            .row_issues
            .iter()
            .filter(|i| i.reason == "duplicate food number")
            .filter_map(|i| i.food_code.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        unit_mismatches: p.catalog.unit_mismatches.clone(),
        category_errors: p.catalog.category_errors.clone(),
        value_status_counts: p.catalog.status_counts.clone(),
        notation_counts: p.catalog.notation_counts.clone(),
        cooking_state_review: cooking_state_review(
            &p.catalog.unmapped_states,
        ),
        deferred_to_chapter3: p.catalog.deferred_to_chapter3.clone(),
        quarantined,
        errata,
        repos: repos.iter().map(|r| r.report.clone()).collect(),
        blockers,
        needs_accept_quarantine: needs_accept,
    }
}

fn print_text(r: &Report, run_dir: &Path) {
    println!("{} — {} ({})", SOURCE_NAME, r.source_release, r.mode);
    for f in &r.files {
        println!(
            "  {}: {} sha256:{} ({} bytes, retrieved {})",
            f.role, f.file_name, f.sha256, f.bytes, f.retrieved_at
        );
    }
    println!(
        "  sheet {} header rows {}-{} (identifiers row {}), data from row {}{}",
        r.sheet,
        r.header_rows.0,
        r.header_rows.1,
        r.identifier_row,
        r.first_data_row,
        r.updated_label.as_ref().map(|l| format!(", {l}")).unwrap_or_default()
    );
    println!(
        "rows {} → foods {}, nutrients {}, values {}",
        r.rows_read, r.foods, r.nutrients, r.values
    );
    println!("value_status: {}", join_counts(&r.value_status_counts));
    println!("notations: {}", join_counts(&r.notation_counts));
    if !r.ignored_columns.is_empty() {
        println!("ignored columns:");
        for c in &r.ignored_columns {
            println!(
                "  col {} {} — {} ({} cells)",
                c.col + 1,
                c.label,
                c.reason,
                c.non_empty_cells
            );
        }
    }
    if !r.duplicate_food_codes.is_empty() {
        println!(
            "duplicate food numbers: {}",
            r.duplicate_food_codes.join(", ")
        );
    }
    println!("quarantined: {}", join_counts(&r.quarantined));
    if let Some(e) = &r.errata {
        println!(
            "errata {}: {} entries — {}",
            e.date.as_deref().unwrap_or("undated"),
            e.entries,
            join_counts(&e.by_status)
        );
        for o in e.outcomes.iter().filter(|o| {
            matches!(
                o.status,
                ErrataStatus::Applied
                    | ErrataStatus::AppliedOverConflict
                    | ErrataStatus::Conflict
                    | ErrataStatus::Unresolved
            )
        }) {
            println!(
                "  {:?} {} {} ({}): 誤 {:?} → 正 {:?}; table {:?} → {:?}",
                o.status,
                o.food_code,
                o.field,
                o.basis,
                o.wrong.as_deref().unwrap_or(""),
                o.right.as_deref().unwrap_or(""),
                o.before.as_deref().unwrap_or(""),
                o.after.as_deref().unwrap_or(""),
            );
        }
        for s in &e.skipped_sheets {
            println!(
                "  skipped sheet {} ({} entries): {}",
                s.sheet, s.entries, s.reason
            );
        }
    }
    if !r.deferred_to_chapter3.is_empty() {
        let cells: Vec<_> = r
            .deferred_to_chapter3
            .iter()
            .map(|d| format!("{} {}", d.food_code, d.nutrient_key))
            .collect();
        println!(
            "deferred to chapter 3 (no value written, reads as not_listed): {}",
            cells.join(", ")
        );
    }
    let review = &r.cooking_state_review;
    if review.blank_foods > 0 {
        let shown: Vec<_> = review
            .unmapped_last_words
            .iter()
            .take(8)
            .map(|w| format!("{}={}", w.word, w.count))
            .collect();
        println!(
            "cooking_state left blank for {} foods ({} distinct last words; full list in report.json cooking_state_review): {} …",
            review.blank_foods,
            review.unmapped_last_words.len(),
            shown.join(", ")
        );
    }
    for repo in &r.repos {
        println!(
            "{} ({}): create {}, update {}, unchanged {}, key conflicts {}, delete candidates {} (existing {})",
            repo.repo,
            repo.kind.as_str(),
            repo.create,
            repo.update,
            repo.unchanged,
            repo.key_conflicts,
            repo.delete_candidates,
            repo.existing_records
        );
        if !repo.changed_properties.is_empty() {
            println!(
                "  changes: {}",
                join_counts(&repo.changed_properties)
            );
        }
        if !repo.preserved_human_fields.is_empty() {
            println!(
                "  kept human edits: {}",
                join_counts(&repo.preserved_human_fields)
            );
        }
        if !repo.skipped_optional_properties.is_empty() {
            println!(
                "  not written (repo lacks the property): {}",
                repo.skipped_optional_properties.join(", ")
            );
        }
    }
    for b in &r.blockers {
        println!("BLOCKER: {b}");
    }
    for n in &r.needs_accept_quarantine {
        println!("needs --accept-quarantine: {n}");
    }
    println!("details: {}", run_dir.display());
}

fn join_counts(m: &BTreeMap<String, usize>) -> String {
    if m.is_empty() {
        return "none".into();
    }
    m.iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(", ")
}

// ==================== state directory ====================

fn run_dir(args: &ImportArgs, p: &Prepared) -> PathBuf {
    let mut h = Sha256::new();
    for part in [
        IMPORTER_FORMAT,
        &args.source_id,
        &args.sheet,
        &args.key_prefix,
        &args.ingredient_repo,
        &args.nutrient_repo,
        &args.value_repo,
    ] {
        h.update(part.as_bytes());
        h.update([0]);
    }
    for f in &p.files {
        h.update(f.sha256.as_bytes());
    }
    let short = &hex(&h.finalize())[..12];
    args.state_dir
        .join(format!("{}-{}-{short}", args.source_id, p.source_release))
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let text = serde_json::to_string_pretty(value)?;
    fs::write(path, text + "\n")
        .with_context(|| format!("writing {}", path.display()))
}

fn manifest(
    args: &ImportArgs,
    p: &Prepared,
    status: &str,
    extra: Value,
) -> Value {
    let table = &p.table.layout;
    let mut m = json!({
        "importer": {
            "name": "library food import",
            "format": IMPORTER_FORMAT,
            "version": env!("CARGO_PKG_VERSION"),
        },
        "source": {
            "id": args.source_id,
            "name": SOURCE_NAME,
            "publisher": SOURCE_PUBLISHER,
            "release": p.source_release,
            "page_url": MEXT_PAGE_URL,
            "usage": "https://www.mext.go.jp/a_menu/syokuhinseibun/ (出典を明記して利用)",
        },
        "files": p.files,
        "table": {
            "sheet": args.sheet,
            "header_rows": table.header_rows,
            "identifier_row": table.identifier_row,
            "unit_row": table.unit_row,
            "first_data_row": table.first_data_row,
            "updated_label": table.updated_label,
            "adopted_columns": table.nutrients.iter().map(|n| &n.key).collect::<Vec<_>>(),
        },
        "errata_date": p.errata.as_ref().and_then(|e| e.date.clone()),
        "repos": {
            "ingredient": args.ingredient_repo,
            "nutrient": args.nutrient_repo,
            "value": args.value_repo,
        },
        "status": status,
        "updated_at": now(),
    });
    if let (Value::Object(m), Value::Object(extra)) = (&mut m, extra) {
        m.extend(extra);
    }
    m
}

/// What to send to `POST .../ingredient-catalogs/{catalog}/releases` once
/// the drafts are complete. Printed, never called.
fn publish_hint(args: &ImportArgs, p: &Prepared) -> Value {
    let table = p.files.iter().find(|f| f.role == "table");
    let errata = p.files.iter().find(|f| f.role == "errata");
    let mut notes = format!(
        "{SOURCE_NAME}（{SOURCE_PUBLISHER}）. table sha256:{}",
        table.map(|f| f.sha256.as_str()).unwrap_or("-")
    );
    if let Some(e) = errata {
        notes.push_str(&format!("; errata sha256:{}", e.sha256));
    }
    notes.push_str(&format!(
        "; imported by {IMPORTER_FORMAT} library-cli {}",
        env!("CARGO_PKG_VERSION")
    ));
    json!({
        "source_id": args.source_id,
        "source_release": p.source_release,
        "source_url": args.table_url,
        "source_retrieved_at": table.map(|f| f.retrieved_at.clone()),
        "notes": notes,
    })
}

// ==================== entry point ====================

fn retrieved_at(args: &ImportArgs) -> Result<String> {
    if let Some(t) = &args.retrieved_at {
        let parsed =
            DateTime::parse_from_rfc3339(t).with_context(|| {
                format!("--retrieved-at {t:?} is not RFC 3339")
            })?;
        return Ok(parsed
            .with_timezone(&Utc)
            .to_rfc3339_opts(SecondsFormat::Secs, true));
    }
    let modified = fs::metadata(&args.table)?.modified()?;
    Ok(DateTime::<Utc>::from(modified)
        .to_rfc3339_opts(SecondsFormat::Secs, true))
}

pub(super) fn validate_source_id(source_id: &str) -> Result<()> {
    let valid = !source_id.is_empty()
        && source_id.len() <= 128
        && source_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._+-".contains(&b));
    if !valid {
        bail!(
            "--source-id must be 1-128 ASCII letters, digits, '.', '_', '+', or '-'"
        );
    }
    Ok(())
}

pub async fn run<S: Store>(
    args: &ImportArgs,
    store: Option<&S>,
    format: Format,
) -> Result<()> {
    validate_source_id(&args.source_id)?;
    let retrieved = retrieved_at(args)?;
    let table_bytes = fs::read(&args.table)
        .with_context(|| format!("reading {}", args.table.display()))?;
    let errata_bytes = match &args.errata {
        Some(path) => Some(
            fs::read(path)
                .with_context(|| format!("reading {}", path.display()))?,
        ),
        None => None,
    };
    let options = BuildOptions {
        source_id: args.source_id.clone(),
        ingredient_key_prefix: args.key_prefix.clone(),
        basis: BASIS.into(),
        ingredient_repo: args.ingredient_repo.clone(),
        nutrient_repo: args.nutrient_repo.clone(),
        value_repo: args.value_repo.clone(),
        categories: BTreeMap::new(),
    };
    let prepared = prepare(PrepareInput {
        table_bytes: &table_bytes,
        table_file: SourceFile::new(
            "table",
            &args.table_url,
            &args.table,
            &table_bytes,
            &retrieved,
        ),
        errata: match (&errata_bytes, &args.errata) {
            (Some(b), Some(path)) => Some((
                b.as_slice(),
                SourceFile::new(
                    "errata",
                    &args.errata_url,
                    path,
                    b,
                    &retrieved,
                ),
            )),
            _ => None,
        },
        sheet: &args.sheet,
        validate_complete_source: true,
        options,
    })?;

    let repos = match store {
        Some(store) => {
            let c = &prepared.catalog;
            let targets = [
                RepoTarget {
                    kind: RecordKind::Nutrient,
                    repo: &args.nutrient_repo,
                    records: &c.nutrients,
                },
                RepoTarget {
                    kind: RecordKind::Ingredient,
                    repo: &args.ingredient_repo,
                    records: &c.ingredients,
                },
                RepoTarget {
                    kind: RecordKind::Value,
                    repo: &args.value_repo,
                    records: &c.values,
                },
            ];
            plan_all(
                store,
                &targets,
                &c.withheld_keys,
                args.concurrency as usize,
            )
            .await?
        }
        None => Vec::new(),
    };

    let mode = if args.apply {
        "apply"
    } else if args.offline {
        "offline-preview"
    } else {
        "dry-run"
    };
    let report =
        build_report(&prepared, &args.sheet, &args.source_id, &repos, mode);
    let dir = run_dir(args, &prepared);
    fs::create_dir_all(&dir)
        .with_context(|| format!("creating {}", dir.display()))?;
    write_json(&dir.join("report.json"), &report)?;
    let mut q = String::new();
    for item in &prepared.catalog.quarantine {
        q.push_str(&serde_json::to_string(item)?);
        q.push('\n');
    }
    for o in prepared.errata_outcomes.iter().filter(|o| {
        matches!(
            o.status,
            ErrataStatus::Conflict | ErrataStatus::Unresolved
        )
    }) {
        q.push_str(&serde_json::to_string(
            &json!({"scope": "errata", "entry": o}),
        )?);
        q.push('\n');
    }
    fs::write(dir.join("quarantine.jsonl"), q)?;
    let plans: Vec<Value> = repos
        .iter()
        .map(|r| json!({"repo": r.repo, "writes": r.plan.writes, "conflicts": r.plan.conflicts, "delete_candidates": r.plan.delete_candidates}))
        .collect();
    write_json(&dir.join("plan.json"), &plans)?;

    if !args.apply {
        write_json(
            &dir.join("manifest.json"),
            &manifest(args, &prepared, "planned", json!({})),
        )?;
        match format {
            Format::Json => print_json(&serde_json::to_value(&report)?),
            Format::Text => print_text(&report, &dir),
        }
        return Ok(());
    }

    if format == Format::Text {
        print_text(&report, &dir);
    }
    if !report.blockers.is_empty() {
        write_json(
            &dir.join("manifest.json"),
            &manifest(
                args,
                &prepared,
                "blocked",
                json!({"blockers": report.blockers}),
            ),
        )?;
        bail!("refusing to write: {}", report.blockers.join("; "));
    }
    if !report.needs_accept_quarantine.is_empty() && !args.accept_quarantine
    {
        write_json(
            &dir.join("manifest.json"),
            &manifest(
                args,
                &prepared,
                "blocked",
                json!({"needs_accept_quarantine": report.needs_accept_quarantine}),
            ),
        )?;
        bail!(
            "refusing to write: {} (review {} and pass --accept-quarantine)",
            report.needs_accept_quarantine.join("; "),
            dir.join("quarantine.jsonl").display()
        );
    }
    let Some(store) = store else {
        bail!("--apply needs the repos; it cannot run with --offline");
    };

    let started_at = now();
    write_json(
        &dir.join("manifest.json"),
        &manifest(
            args,
            &prepared,
            "in_progress",
            json!({"started_at": started_at}),
        ),
    )?;
    let mut log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("writes.jsonl"))?;
    let mut log_error = None;
    let summary = execute(
        store,
        &repos,
        args.concurrency as usize,
        args.max_failures,
        Duration::from_millis(500),
        |entry| {
            if let Err(e) = serde_json::to_string(entry)
                .map_err(anyhow::Error::from)
                .and_then(|l| {
                    writeln!(log_file, "{l}").map_err(anyhow::Error::from)
                })
            {
                log_error.get_or_insert(e);
            }
        },
    )
    .await;

    let ok = summary.failed == 0
        && summary.not_attempted == 0
        && log_error.is_none();
    let status = if ok { "completed" } else { "failed" };
    let mut extra = json!({"started_at": started_at, "finished_at": now(), "writes": summary});
    if ok {
        extra["publish_hint"] = publish_hint(args, &prepared);
    }
    write_json(
        &dir.join("manifest.json"),
        &manifest(args, &prepared, status, extra),
    )?;

    match format {
        Format::Json => print_json(&json!({"report": report, "writes": summary, "status": status, "dir": dir})),
        Format::Text => println!(
            "writes: {} ok, {} failed, {} not attempted in {:.1}s — {status}",
            summary.succeeded, summary.failed, summary.not_attempted, summary.elapsed_seconds
        ),
    }
    if let Some(e) = log_error {
        bail!("the write log could not be written: {e:#}");
    }
    if !ok {
        bail!(
            "import {status}: {} writes failed, {} not attempted{}. Run the same command again to resume; see {}",
            summary.failed,
            summary.not_attempted,
            summary.stopped_early.map(|s| format!(" ({s})")).unwrap_or_default(),
            dir.join("writes.jsonl").display()
        );
    }
    if format == Format::Text {
        println!(
            "drafts are complete; publishing is a separate step (POST /v1beta/orgs/{{org}}/ingredient-catalogs/{{catalog}}/releases) — see publish_hint in manifest.json"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::fixtures;
    use super::super::store::fake::FakeStore;
    use super::*;

    const ING: &str = "t/food";
    const NUT: &str = "t/food-nutrients";
    const VAL: &str = "t/food-nutrient-values";

    fn prepared() -> Prepared {
        let file = |role| {
            SourceFile::new(
                role,
                "https://example.test",
                Path::new("x.xlsx"),
                b"x",
                "2026-09-25T08:36:35Z",
            )
        };
        prepare(PrepareInput {
            table_bytes: fixtures::TABLE_XLSX,
            table_file: file("table"),
            errata: Some((fixtures::ERRATA_XLSX, file("errata"))),
            sheet: "表全体",
            validate_complete_source: false,
            options: BuildOptions {
                source_id: "mext-sfct8-2023".into(),
                ingredient_key_prefix: "mext-".into(),
                basis: BASIS.into(),
                ingredient_repo: ING.into(),
                nutrient_repo: NUT.into(),
                value_repo: VAL.into(),
                categories: BTreeMap::new(),
            },
        })
        .unwrap()
    }

    fn store() -> FakeStore {
        use prop::*;
        FakeStore::default()
            .with_repo(
                ING,
                &[
                    INGREDIENT_KEY,
                    SOURCE_FOOD_CODE,
                    STANDARD_NAME,
                    READING,
                    ALIASES,
                    CATEGORY_CODE,
                    CATEGORY_NAME,
                    PART,
                    COOKING_STATE,
                    SKIN_BONE,
                    REFUSE_RATE,
                    ATTRIBUTE_REVIEW_STATUS,
                ],
            )
            .with_repo(
                NUT,
                &[
                    NUTRIENT_KEY,
                    UNIT,
                    BASIS,
                    METHOD,
                    DISPLAY_ORDER,
                    DEFAULT_DISPLAY,
                ],
            )
            .with_repo(
                VAL,
                &[
                    INGREDIENT_KEY,
                    NUTRIENT_KEY,
                    VALUE_STATUS,
                    AMOUNT,
                    RAW_NOTATION,
                ],
            )
    }

    async fn plan(p: &Prepared, s: &FakeStore) -> Vec<RepoState> {
        let c = &p.catalog;
        let targets = [
            RepoTarget {
                kind: RecordKind::Nutrient,
                repo: NUT,
                records: &c.nutrients,
            },
            RepoTarget {
                kind: RecordKind::Ingredient,
                repo: ING,
                records: &c.ingredients,
            },
            RepoTarget {
                kind: RecordKind::Value,
                repo: VAL,
                records: &c.values,
            },
        ];
        plan_all(s, &targets, &c.withheld_keys, 2).await.unwrap()
    }

    async fn apply(p: &Prepared, s: &FakeStore) -> ExecSummary {
        let repos = plan(p, s).await;
        execute(s, &repos, 3, 5, Duration::ZERO, |_| {}).await
    }

    #[tokio::test]
    async fn preview_counts_before_anything_is_written() {
        let p = prepared();
        let s = store();
        let repos = plan(&p, &s).await;
        let report = build_report(
            &p,
            "表全体",
            "mext-sfct8-2023",
            &repos,
            "dry-run",
        );
        assert_eq!(report.source_release, "2023+errata-2026-03-27");
        assert_eq!(report.rows_read, 8);
        assert_eq!(
            (report.foods, report.nutrients, report.values),
            (4, 11, 41)
        );
        assert_eq!(report.duplicate_food_codes, vec!["99999"]);
        assert_eq!(report.repos[0].create, 11);
        assert_eq!(report.repos[1].create, 4);
        assert_eq!(report.repos[2].create, 41);
        assert_eq!(report.deferred_to_chapter3.len(), 1);
        assert_eq!(report.cooking_state_review.blank_foods, 1);
        assert_eq!(
            report.cooking_state_review.unmapped_last_words[0].word,
            "玄穀"
        );
        // `remarks` is optional and this repo lacks it.
        assert_eq!(
            report.repos[1].skipped_optional_properties,
            vec!["remarks"]
        );
        assert!(report.blockers.is_empty(), "{:?}", report.blockers);
        assert!(!report.needs_accept_quarantine.is_empty());
        assert_eq!(*s.upserts.borrow(), 0, "a preview writes nothing");
    }

    #[tokio::test]
    async fn a_clean_run_needs_no_accept_quarantine() {
        // What the official files look like: errata applied (some over a
        // conflict) and iodine `*` deferred to chapter 3, no anomalies.
        let mut p = prepared();
        p.catalog.quarantine.clear();
        p.errata_outcomes.retain(|o| {
            !matches!(
                o.status,
                ErrataStatus::Conflict | ErrataStatus::Unresolved
            )
        });
        assert!(p
            .errata_outcomes
            .iter()
            .any(|o| o.status == ErrataStatus::AppliedOverConflict));
        assert!(!p.catalog.deferred_to_chapter3.is_empty());
        let s = store();
        let repos = plan(&p, &s).await;
        let report =
            build_report(&p, "表全体", "mext-sfct8-2023", &repos, "apply");
        assert!(report.blockers.is_empty(), "{:?}", report.blockers);
        assert!(
            report.needs_accept_quarantine.is_empty(),
            "{:?}",
            report.needs_accept_quarantine
        );

        // A real anomaly still needs the flag.
        let anomalous = prepared();
        let report = build_report(
            &anomalous,
            "表全体",
            "mext-sfct8-2023",
            &repos,
            "apply",
        );
        assert!(!report.needs_accept_quarantine.is_empty());
    }

    #[tokio::test]
    async fn import_is_idempotent_and_keeps_human_edits() {
        let p = prepared();
        let s = store();
        let first = apply(&p, &s).await;
        assert_eq!((first.succeeded, first.failed), (56, 0));
        assert_eq!(s.count(VAL), 41);

        let onion = p
            .catalog
            .ingredients
            .iter()
            .find(|i| i.business_key == "mext-06153")
            .unwrap();
        s.edit(ING, &onion.data_id, prop::STANDARD_NAME, "たまねぎ");
        s.edit(ING, &onion.data_id, prop::ALIASES, "玉ねぎ\n新たまねぎ");
        s.edit(
            ING,
            &onion.data_id,
            prop::ATTRIBUTE_REVIEW_STATUS,
            "reviewed",
        );

        let upserts_before = *s.upserts.borrow();
        let second = apply(&p, &s).await;
        assert_eq!(second.attempted, 0, "nothing changed, nothing sent");
        assert_eq!(*s.upserts.borrow(), upserts_before);
        assert_eq!(s.count(VAL), 41, "no duplicates");

        let stored = s.record(ING, &onion.data_id).unwrap();
        assert_eq!(stored.fields[prop::STANDARD_NAME], "たまねぎ");
        assert_eq!(stored.fields[prop::ALIASES], "玉ねぎ\n新たまねぎ");
        assert_eq!(
            stored.fields[prop::ATTRIBUTE_REVIEW_STATUS],
            "reviewed"
        );
    }

    #[tokio::test]
    async fn a_failed_run_is_reported_and_resumes() {
        let p = prepared();
        let s = store();
        let bad = p.catalog.ingredients[1].data_id.clone();
        s.failing.borrow_mut().insert(bad.clone());

        let mut logged = Vec::new();
        let repos = plan(&p, &s).await;
        let failed = execute(&s, &repos, 2, 5, Duration::ZERO, |l| {
            logged.push(l.clone())
        })
        .await;
        assert_eq!(failed.failed, 1);
        assert!(
            failed.not_attempted >= 36,
            "values are not written after an ingredient failure"
        );
        assert!(failed.stopped_early.is_some());
        assert!(logged
            .iter()
            .any(|l| l.data_id == bad && l.status == "failed"));
        assert_eq!(s.count(VAL), 0);

        s.failing.borrow_mut().clear();
        let resumed = apply(&p, &s).await;
        assert_eq!(resumed.failed, 0);
        assert_eq!(resumed.not_attempted, 0);
        // Nutrients and the three ingredients that landed are not re-sent.
        assert_eq!(resumed.succeeded, 1 + 41);
        assert_eq!(s.count(ING), 4);
        assert_eq!(s.count(VAL), 41);
    }

    #[tokio::test]
    async fn source_corrections_update_only_source_fields() {
        let p = prepared();
        let s = store();
        apply(&p, &s).await;
        let kcal = p
            .catalog
            .values
            .iter()
            .find(|v| v.business_key == "mext-06153/ENERC_KCAL")
            .unwrap();
        // Someone typed the pre-errata value back in.
        s.edit(VAL, &kcal.data_id, prop::AMOUNT, "33");
        let repos = plan(&p, &s).await;
        let values =
            repos.iter().find(|r| r.kind == RecordKind::Value).unwrap();
        assert_eq!(values.plan.writes.len(), 1);
        assert_eq!(
            values.plan.writes[0].action,
            Action::Update {
                changed: vec!["amount".into()]
            }
        );
    }
}
