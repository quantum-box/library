use value_object::DatabaseUrl;

pub const LIBRARY_DATABASE_NAME: &str = "library";
pub const DATABASE_MANAGER_DATABASE_NAME: &str =
    "tachyon_apps_database_manager";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DatabaseTopology {
    Split,
    UnifiedPreview,
}

/// Resolves library-api's logical database roles to physical databases.
///
/// ADR-0049 deliberately provisions one database per app and PR. Applications
/// with multiple bounded contexts must therefore use the injected preview
/// database for every logical role. Production is intentionally different:
/// `library` and `tachyon_apps_database_manager` remain separate databases and
/// separate SQLx migration histories. Do not collapse these roles globally.
#[derive(Clone)]
pub struct DatabaseLayout {
    library: DatabaseUrl,
    database_manager: DatabaseUrl,
}

impl DatabaseLayout {
    pub fn from_runtime(database_url: DatabaseUrl) -> errors::Result<Self> {
        let tachyon_environment = std::env::var("TACHYON_ENV").ok();
        Self::resolve(&database_url, tachyon_environment.as_deref())
    }

    pub fn resolve(
        database_url: &DatabaseUrl,
        tachyon_environment: Option<&str>,
    ) -> errors::Result<Self> {
        let environment = tachyon_environment
            .map(str::trim)
            .unwrap_or_default()
            .to_ascii_lowercase();
        let selected_database = database_url.database().as_deref();
        let selects_pr_database =
            selected_database.is_some_and(|name| name.starts_with("pr_"));

        if environment == "production" && selects_pr_database {
            return Err(errors::Error::bad_request(
                "production DATABASE_URL must not select a PR-scoped database",
            ));
        }

        let topology = if environment == "preview" && selects_pr_database {
            DatabaseTopology::UnifiedPreview
        } else {
            DatabaseTopology::Split
        };

        let (library, database_manager) = match topology {
            DatabaseTopology::Split => (
                database_url.use_database(LIBRARY_DATABASE_NAME),
                database_url.use_database(DATABASE_MANAGER_DATABASE_NAME),
            ),
            DatabaseTopology::UnifiedPreview => {
                (database_url.clone(), database_url.clone())
            }
        };

        Ok(Self {
            library,
            database_manager,
        })
    }

    pub fn library(&self) -> &DatabaseUrl {
        &self.library
    }

    pub fn database_manager(&self) -> &DatabaseUrl {
        &self.database_manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn database_name(database_url: &DatabaseUrl) -> &str {
        database_url.database().as_deref().unwrap()
    }

    #[test]
    fn production_keeps_two_logical_databases() {
        let database_url: DatabaseUrl =
            "mysql://app:password@tidb.example:4000/library"
                .parse()
                .unwrap();

        let layout =
            DatabaseLayout::resolve(&database_url, Some("production"))
                .unwrap();

        assert_eq!(database_name(layout.library()), "library");
        assert_eq!(
            database_name(layout.database_manager()),
            "tachyon_apps_database_manager"
        );
        assert_ne!(
            database_name(layout.library()),
            database_name(layout.database_manager())
        );
    }

    #[test]
    fn pr_scoped_preview_uses_one_physical_database() {
        let database_url: DatabaseUrl =
            "mysql://app:password@tidb.example:4000/pr_3328_library"
                .parse()
                .unwrap();

        let layout =
            DatabaseLayout::resolve(&database_url, Some("preview"))
                .unwrap();

        assert_eq!(database_name(layout.library()), "pr_3328_library");
        assert_eq!(
            database_name(layout.database_manager()),
            "pr_3328_library"
        );
    }

    #[test]
    fn legacy_preview_database_stays_split_during_staged_rollout() {
        let database_url: DatabaseUrl =
            "mysql://app:password@tidb.example:4000/shared_preview"
                .parse()
                .unwrap();

        let layout =
            DatabaseLayout::resolve(&database_url, Some("preview"))
                .unwrap();

        assert_eq!(database_name(layout.library()), "library");
        assert_eq!(
            database_name(layout.database_manager()),
            "tachyon_apps_database_manager"
        );
    }

    #[test]
    fn production_rejects_pr_scoped_database() {
        let database_url: DatabaseUrl =
            "mysql://app:password@tidb.example:4000/pr_3328_library"
                .parse()
                .unwrap();

        let result =
            DatabaseLayout::resolve(&database_url, Some("production"));

        assert!(result.is_err());
    }
}
