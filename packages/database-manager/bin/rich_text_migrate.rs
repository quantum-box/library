//! Convert a MARKDOWN body property to RICH_TEXT in place.
//!
//! Dry-run is the default; pass `--apply` to write. One database at a time,
//! one transaction per property, so a failure leaves every property either
//! fully converted or untouched.
//!
//! What a conversion touches:
//!   - the `fields` row: `datatype` and, when the definition envelope is
//!     populated, `type_key`
//!   - every non-empty legacy `value{N}` cell: Markdown text becomes the
//!     block document as JSON (values already holding a document are left
//!     alone, so re-running is safe)
//!   - matching canonical `property_values` rows, for deployments already
//!     dual-writing
//!
//! Reads never depended on this finishing: the legacy text boundary
//! converts stray Markdown under a RICH_TEXT property on the fly. Running
//! it moves the data to its canonical form eagerly instead of on next
//! write.

use std::env;

use database_manager::domain::rich_text::from_markdown;
use value_object::{DatabaseUrl, TenantId};

const DATABASE_NAME: &str = "tachyon_apps_database_manager";

#[derive(Debug)]
struct Options {
    environment: String,
    tenant_id: TenantId,
    database_id: String,
    property_name: String,
    dry_run: bool,
}

fn usage() -> &'static str {
    "usage: database_manager_rich_text_migrate \
     <dev|prod|tidb-playground> <tenant_id> <database_id> \
     [--property <name>] [--dry-run|--apply]"
}

fn parse_options() -> anyhow::Result<Options> {
    let mut args = env::args().skip(1);
    let environment =
        args.next().ok_or_else(|| anyhow::anyhow!(usage()))?;
    let tenant_id: TenantId = args
        .next()
        .ok_or_else(|| anyhow::anyhow!(usage()))?
        .parse()?;
    let database_id =
        args.next().ok_or_else(|| anyhow::anyhow!(usage()))?;

    let mut options = Options {
        environment,
        tenant_id,
        database_id,
        property_name: "content".to_string(),
        dry_run: true,
    };
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--property" => {
                options.property_name =
                    args.next().ok_or_else(|| anyhow::anyhow!(usage()))?;
            }
            "--dry-run" => options.dry_run = true,
            "--apply" => options.dry_run = false,
            _ => anyhow::bail!(usage()),
        }
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

/// Whether a stored value is already a block document.
fn is_document(raw: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(raw).is_ok_and(|value| {
        value.is_array() || value.get("blocks").is_some()
    })
}

#[derive(sqlx::FromRow)]
struct FieldRow {
    id: String,
    field_num: u32,
    type_key: Option<String>,
}

#[derive(sqlx::FromRow)]
struct ValueRow {
    id: String,
    value: Option<String>,
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
    let pool = db.pool();

    let fields: Vec<FieldRow> = sqlx::query_as(
        "SELECT id, field_num, type_key FROM fields \
         WHERE tenant_id = ? AND object_id = ? \
           AND field_name = ? AND datatype = 'MARKDOWN'",
    )
    .bind(options.tenant_id.to_string())
    .bind(&options.database_id)
    .bind(&options.property_name)
    .fetch_all(pool.as_ref())
    .await?;

    if fields.is_empty() {
        tracing::info!(
            property = %options.property_name,
            "no MARKDOWN property with this name; nothing to do"
        );
        return Ok(());
    }

    for field in fields {
        if field.field_num > 50 {
            anyhow::bail!(
                "field_num {} is outside the legacy column range",
                field.field_num
            );
        }
        let value_column = format!("value{}", field.field_num);

        let rows: Vec<ValueRow> = sqlx::query_as(&format!(
            "SELECT id, {value_column} AS value FROM data \
             WHERE tenant_id = ? AND object_id = ? \
               AND {value_column} IS NOT NULL AND {value_column} != '' \
             ORDER BY id",
        ))
        .bind(options.tenant_id.to_string())
        .bind(&options.database_id)
        .fetch_all(pool.as_ref())
        .await?;

        let pending: Vec<(String, String)> = rows
            .into_iter()
            .filter_map(|row| {
                let raw = row.value?;
                if is_document(&raw) {
                    return None;
                }
                Some((row.id, from_markdown(&raw).to_string()))
            })
            .collect();

        tracing::info!(
            property_id = %field.id,
            column = %value_column,
            values_to_convert = pending.len(),
            dry_run = options.dry_run,
            "conversion plan"
        );
        if options.dry_run {
            continue;
        }

        let mut transaction = pool.begin().await?;

        for (data_id, document) in &pending {
            sqlx::query(&format!(
                "UPDATE data SET {value_column} = ? \
                 WHERE tenant_id = ? AND object_id = ? AND id = ?",
            ))
            .bind(document)
            .bind(options.tenant_id.to_string())
            .bind(&options.database_id)
            .bind(data_id)
            .execute(&mut *transaction)
            .await?;
        }

        // Canonical rows exist once dual-write is on. A Markdown envelope
        // holds a JSON *string*; the document rides as real JSON.
        let canonical: Vec<ValueRow> = sqlx::query_as(
            "SELECT data_id AS id, value FROM property_values \
             WHERE tenant_id = ? AND database_id = ? AND property_id = ? \
               AND type_key = 'markdown'",
        )
        .bind(options.tenant_id.to_string())
        .bind(&options.database_id)
        .bind(&field.id)
        .fetch_all(&mut *transaction)
        .await?;
        for row in canonical {
            let Some(raw) = row.value else { continue };
            let markdown: String =
                serde_json::from_str(&raw).unwrap_or_else(|_| raw.clone());
            let document = if is_document(&markdown) {
                markdown
            } else {
                from_markdown(&markdown).to_string()
            };
            sqlx::query(
                "UPDATE property_values \
                 SET type_key = 'rich_text', value = ? \
                 WHERE tenant_id = ? AND database_id = ? \
                   AND property_id = ? AND data_id = ?",
            )
            .bind(&document)
            .bind(options.tenant_id.to_string())
            .bind(&options.database_id)
            .bind(&field.id)
            .bind(&row.id)
            .execute(&mut *transaction)
            .await?;
        }

        let new_type_key =
            field.type_key.as_deref().map(|_| "rich_text".to_string());
        sqlx::query(
            "UPDATE fields \
             SET datatype = 'RICH_TEXT', \
                 type_key = COALESCE(?, type_key) \
             WHERE tenant_id = ? AND object_id = ? AND id = ?",
        )
        .bind(new_type_key)
        .bind(options.tenant_id.to_string())
        .bind(&options.database_id)
        .bind(&field.id)
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;
        tracing::info!(
            property_id = %field.id,
            converted = pending.len(),
            "property converted"
        );
    }

    Ok(())
}
