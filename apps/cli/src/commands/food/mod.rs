//! `library food` — the common ingredient catalog (COM-860/COM-861).
//!
//! `library food import` reads the official Excel of the Standard Tables
//! of Food Composition in Japan (日本食品標準成分表（八訂）増補2023年) and its
//! errata, and writes the three COM-860 draft repos through the ordinary
//! upsert API. It lives in the CLI because it only needs what the CLI
//! already has — an API key and the REST client — and must not get a
//! database path the web API does not have. Publishing stays the separate
//! `POST .../ingredient-catalogs/{catalog}/releases` call.

mod catalog;
mod errata;
#[cfg(test)]
mod fixtures;
mod import;
mod plan;
mod sheet;
mod store;
mod table;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::config::ConfigOverrides;
use crate::output::Format;

pub const MEXT_TABLE_URL: &str =
    "https://www.mext.go.jp/content/20260327-mxt_kagsei-mext-000029402_02.xlsx";
pub const MEXT_ERRATA_URL: &str =
    "https://www.mext.go.jp/content/20260327-mxt_kagsei-mext-000029402_16.xlsx";
pub const MEXT_PAGE_URL: &str =
    "https://www.mext.go.jp/a_menu/syokuhinseibun/mext_00001.html";

#[derive(Subcommand)]
pub enum FoodCommand {
    /// Import the MEXT food composition Excel (+ errata) into the draft
    /// repos. Dry run unless --apply is given.
    Import(ImportArgs),
}

#[derive(Args, Clone, Debug)]
pub struct ImportArgs {
    /// Main table workbook (第2章（データ）), downloaded from --table-url
    #[arg(long, value_name = "XLSX")]
    pub table: PathBuf,
    /// Errata workbook (正誤表（データ）)
    #[arg(long, value_name = "XLSX")]
    pub errata: Option<PathBuf>,
    /// Where --table was downloaded from (recorded as provenance)
    #[arg(long, default_value = MEXT_TABLE_URL)]
    pub table_url: String,
    /// Where --errata was downloaded from (recorded as provenance)
    #[arg(long, default_value = MEXT_ERRATA_URL)]
    pub errata_url: String,
    /// When the files were downloaded (RFC 3339). Defaults to the
    /// modification time of --table.
    #[arg(long, value_name = "TIME")]
    pub retrieved_at: Option<String>,
    /// Sheet holding the whole table
    #[arg(long, default_value = "表全体")]
    pub sheet: String,
    /// Dataset identifier; part of every data ID and of the release
    #[arg(long, default_value = "mext-sfct8-2023")]
    pub source_id: String,
    /// Prefix of `ingredient_key` (`mext-` → `mext-01001`)
    #[arg(long, default_value = "mext-")]
    pub key_prefix: String,
    /// Ingredient draft repo as `org/repo`
    #[arg(long, default_value = "library/food")]
    pub ingredient_repo: String,
    /// Nutrient definition draft repo as `org/repo`
    #[arg(long, default_value = "library/food-nutrients")]
    pub nutrient_repo: String,
    /// Value draft repo as `org/repo`
    #[arg(long, default_value = "library/food-nutrient-values")]
    pub value_repo: String,
    /// Parse and validate the files only; do not read the repos
    #[arg(long, conflicts_with = "apply")]
    pub offline: bool,
    /// Write the planned upserts. Without it nothing is written.
    #[arg(long)]
    pub apply: bool,
    /// Allow --apply although some rows/cells/errata were quarantined
    /// (they are left out and listed in quarantine.jsonl)
    #[arg(long)]
    pub accept_quarantine: bool,
    /// Parallel requests while listing and writing (1-16)
    #[arg(long, default_value_t = 4, value_parser = clap::value_parser!(u16).range(1..=16))]
    pub concurrency: u16,
    /// Stop after this many failed writes
    #[arg(long, default_value_t = 20)]
    pub max_failures: usize,
    /// Where run manifests, reports and write logs go
    #[arg(long, default_value = ".food-import")]
    pub state_dir: PathBuf,
}

pub async fn run(
    command: FoodCommand,
    overrides: &ConfigOverrides,
    format: Format,
) -> Result<()> {
    match command {
        FoodCommand::Import(args) => {
            import::validate_source_id(&args.source_id)?;
            let client = if args.offline {
                None
            } else {
                Some(crate::client::LibraryClient::new(
                    crate::config::resolve(overrides)?,
                )?)
            };
            import::run(&args, client.as_ref(), format).await
        }
    }
}
