//! Database-engine checks that must pass before schema migrations run.

use sqlx::MySqlPool;

/// Fails closed when TiDB would accept unenforced `CHECK` constraints.
///
/// MySQL 8 enforces the constraints used by Database BC migrations. TiDB
/// supports the same syntax, but its enforcement feature is disabled by
/// default and must be enabled by the operator.
pub async fn ensure_check_constraints_enforced(
    pool: &MySqlPool,
) -> errors::Result<()> {
    let version =
        sqlx::query_scalar::<_, String>("SELECT CAST(VERSION() AS CHAR)")
            .fetch_one(pool)
            .await
            .map_err(|error| {
                errors::Error::internal_server_error(format!(
            "Failed to detect database engine before migrations: {error}"
        ))
            })?;

    if !version.to_ascii_lowercase().contains("tidb") {
        return Ok(());
    }

    let setting = sqlx::query_scalar::<_, String>(
        "SELECT CAST(@@tidb_enable_check_constraint AS CHAR)",
    )
    .fetch_one(pool)
    .await
    .map_err(|error| {
        errors::Error::internal_server_error(format!(
            "Failed to verify TiDB CHECK constraint enforcement: {error}"
        ))
    })?;

    validate_check_constraint_setting(&version, Some(&setting))
}

fn validate_check_constraint_setting(
    version: &str,
    setting: Option<&str>,
) -> errors::Result<()> {
    if !version.to_ascii_lowercase().contains("tidb") {
        return Ok(());
    }

    let enabled = setting.map(str::trim).is_some_and(|value| {
        value.eq_ignore_ascii_case("ON") || value == "1"
    });
    if enabled {
        return Ok(());
    }

    Err(errors::Error::internal_server_error(
        "TiDB CHECK constraints are disabled; set \
         tidb_enable_check_constraint=ON before running Database BC migrations",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mysql_does_not_require_a_tidb_setting() {
        assert!(validate_check_constraint_setting("8.0.46", None).is_ok());
    }

    #[test]
    fn tidb_requires_check_constraint_enforcement() {
        assert!(validate_check_constraint_setting(
            "5.7.25-TiDB-v8.5.3",
            Some("ON")
        )
        .is_ok());
        assert!(validate_check_constraint_setting(
            "5.7.25-TiDB-v8.5.3",
            Some("1")
        )
        .is_ok());

        for setting in [None, Some("OFF"), Some("0"), Some("")] {
            let error = validate_check_constraint_setting(
                "5.7.25-TiDB-v8.5.3",
                setting,
            )
            .expect_err(
                "unenforced TiDB CHECK constraints must fail closed",
            );
            assert!(error
                .to_string()
                .contains("tidb_enable_check_constraint=ON"));
        }
    }
}
