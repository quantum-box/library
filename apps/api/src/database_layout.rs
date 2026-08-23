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

    /// Open one connection pool per *physical* database.
    ///
    /// The roles are two databases in production and one in an
    /// ADR-0049 preview, so a preview that built a pool per role held
    /// two pools to the same server for no reason. Callers pass these
    /// handles down instead of opening their own: every component that
    /// resolved a role to a DSN and called `Db::new` used to add
    /// another pool, and the API server ended up with four for two
    /// databases.
    pub fn open_pools(&self) -> DatabasePools {
        let library = persistence::Db::new_lazy(&self.library);
        let database_manager = if self.library.to_string()
            == self.database_manager.to_string()
        {
            library.clone()
        } else {
            persistence::Db::new_lazy(&self.database_manager)
        };

        DatabasePools {
            library,
            database_manager,
        }
    }
}

/// The pools backing library-api's logical database roles.
///
/// Two roles, but not necessarily two pools: a preview resolves both
/// to the same database and both fields then hold the same handle.
#[derive(Clone)]
pub struct DatabasePools {
    pub library: std::sync::Arc<persistence::Db>,
    pub database_manager: std::sync::Arc<persistence::Db>,
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

        assert_eq!(database_name(&layout.library), "library");
        assert_eq!(
            database_name(&layout.database_manager),
            "tachyon_apps_database_manager"
        );
        assert_ne!(
            database_name(&layout.library),
            database_name(&layout.database_manager)
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

        assert_eq!(database_name(&layout.library), "pr_3328_library");
        assert_eq!(
            database_name(&layout.database_manager),
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

        assert_eq!(database_name(&layout.library), "library");
        assert_eq!(
            database_name(&layout.database_manager),
            "tachyon_apps_database_manager"
        );
    }

    /// A preview resolves both roles to one database, so opening a
    /// pool per role would hold two to the same server.
    #[tokio::test]
    async fn a_preview_shares_one_pool_between_both_roles() {
        let database_url: DatabaseUrl =
            "mysql://app:password@tidb.example:4000/pr_3328_library"
                .parse()
                .unwrap();
        let layout =
            DatabaseLayout::resolve(&database_url, Some("preview"))
                .unwrap();

        let pools = layout.open_pools();

        assert!(
            std::sync::Arc::ptr_eq(&pools.library, &pools.database_manager),
            "both roles resolve to the same database, so one pool serves"
        );
    }

    /// Production keeps two databases, and each needs its own pool.
    #[tokio::test]
    async fn production_opens_a_pool_for_each_database() {
        let database_url: DatabaseUrl =
            "mysql://app:password@tidb.example:4000/library"
                .parse()
                .unwrap();
        let layout =
            DatabaseLayout::resolve(&database_url, Some("production"))
                .unwrap();

        let pools = layout.open_pools();

        assert!(!std::sync::Arc::ptr_eq(
            &pools.library,
            &pools.database_manager
        ));
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
