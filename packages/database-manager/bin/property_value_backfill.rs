//! Resumable, value-safe PropertyValue parity/backfill operator.
//!
//! Dry-run is the default. Pass `--apply` explicitly to insert missing known
//! canonical envelopes. Existing canonical rows are never updated.

use std::env;

use database_manager::domain::{DataId, DatabaseId};
use database_manager::interface_adapter::gateway::PropertyValueBackfillGateway;
use database_manager::usecase::PropertyValueBackfillInteractor;
use database_manager::{
    PropertyValueBackfillInputData, PropertyValueBackfillInputPort,
};
use value_object::{DatabaseUrl, TenantId};

const DATABASE_NAME: &str = "tachyon_apps_database_manager";

#[derive(Debug)]
struct Options {
    environment: String,
    tenant_id: TenantId,
    database_id: DatabaseId,
    after: Option<DataId>,
    batch_size: u16,
    max_chunks: u32,
    dry_run: bool,
    checksum_seed: [u8; 32],
}

fn usage() -> &'static str {
    "usage: database_manager_property_value_backfill \
     <dev|prod|tidb-playground> <tenant_id> <database_id> \
     [--after <data_id>] [--batch-size <1..1000>] \
     [--max-chunks <n>] [--checksum-seed <64 hex chars>] \
     [--dry-run|--apply]"
}

fn decode_checksum(value: &str) -> anyhow::Result<[u8; 32]> {
    if value.len() != 64
        || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        anyhow::bail!(
            "checksum seed must contain exactly 64 hex characters"
        );
    }
    let mut checksum = [0_u8; 32];
    for (index, byte) in checksum.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)?;
    }
    Ok(checksum)
}

fn parse_options() -> anyhow::Result<Options> {
    let mut args = env::args().skip(1);
    let environment =
        args.next().ok_or_else(|| anyhow::anyhow!(usage()))?;
    let tenant_id = args
        .next()
        .ok_or_else(|| anyhow::anyhow!(usage()))?
        .parse()?;
    let database_id = args
        .next()
        .ok_or_else(|| anyhow::anyhow!(usage()))?
        .parse()?;

    let mut options = Options {
        environment,
        tenant_id,
        database_id,
        after: None,
        batch_size: 100,
        max_chunks: 1,
        dry_run: true,
        checksum_seed: [0; 32],
    };
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--after" => {
                options.after = Some(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!(usage()))?
                        .parse()?,
                );
            }
            "--batch-size" => {
                options.batch_size = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!(usage()))?
                    .parse()?;
            }
            "--max-chunks" => {
                options.max_chunks = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!(usage()))?
                    .parse()?;
            }
            "--checksum-seed" => {
                options.checksum_seed = decode_checksum(
                    &args.next().ok_or_else(|| anyhow::anyhow!(usage()))?,
                )?;
            }
            "--dry-run" => options.dry_run = true,
            "--apply" => options.dry_run = false,
            _ => anyhow::bail!(usage()),
        }
    }
    if options.max_chunks == 0 {
        anyhow::bail!("max-chunks must be greater than zero");
    }
    Ok(options)
}

fn database_url(environment: &str) -> anyhow::Result<DatabaseUrl> {
    let raw = match environment {
        "dev" => env::var("DEV_DATABASE_URL")?,
        "prod" => env::var("PROD_DATABASE_URL")?,
        "tidb-playground" => "mysql://root@127.0.0.1:4000".to_string(),
        _ => anyhow::bail!(usage()),
    };
    Ok(raw.parse::<DatabaseUrl>()?.use_database(DATABASE_NAME))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    telemetry::init_debug_tracing();
    let options = parse_options()?;
    let dsn = database_url(&options.environment)?;
    let db = persistence::Db::new(dsn.to_string()).await;
    database_manager::migration_preflight::ensure_check_constraints_enforced(
        db.pool().as_ref(),
    )
    .await?;
    let gateway = PropertyValueBackfillGateway::new(db);
    let backfill = PropertyValueBackfillInteractor::new(gateway);

    let mut cursor = options.after;
    let mut checksum = options.checksum_seed;
    let mut complete = false;
    for chunk in 1..=options.max_chunks {
        let report = backfill
            .execute(&PropertyValueBackfillInputData {
                tenant_id: &options.tenant_id,
                database_id: &options.database_id,
                after_data_id: cursor.as_ref(),
                batch_size: options.batch_size,
                dry_run: options.dry_run,
                checksum_seed: checksum,
            })
            .await?;
        tracing::info!(
            chunk,
            dry_run = options.dry_run,
            scanned_records = report.scanned_records,
            compared_values = report.compared_values,
            expected_values = report.expected_values,
            missing_values = report.missing_values,
            written_values = report.written_values,
            matched_values = report.matched_values,
            absent_values = report.absent_values,
            opaque_values = report.opaque_values,
            next_cursor = ?report.next_cursor,
            complete = report.complete,
            parity_checksum = %report.parity_checksum,
            "PropertyValue backfill chunk complete"
        );
        checksum = decode_checksum(&report.parity_checksum)?;
        cursor = report.next_cursor;
        complete = report.complete;
        if complete {
            break;
        }
    }

    if !complete {
        tracing::warn!(
            next_cursor = ?cursor,
            parity_checksum = %database_manager_checksum_hex(&checksum),
            "PropertyValue backfill stopped at max-chunks; resume with --after and --checksum-seed"
        );
    }
    Ok(())
}

fn database_manager_checksum_hex(checksum: &[u8; 32]) -> String {
    let mut encoded = String::with_capacity(64);
    for byte in checksum {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}")
            .expect("writing to String cannot fail");
    }
    encoded
}
