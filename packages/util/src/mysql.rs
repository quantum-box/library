use std::env;
use std::sync::Arc;

use sqlx::{mysql::MySqlPoolOptions, MySql, Pool};

#[derive(Clone, Debug)]
pub struct Db(pub(crate) Arc<Pool<MySql>>);

impl Db {
    pub async fn new() -> Arc<Db> {
        let dsn = env::var("BAKUURE_DATABASE_URL").unwrap_or_else(|_| {
            "mysql://root:password@localhost:3306/bakuure".into()
        });
        let pool = MySqlPoolOptions::new()
            .max_connections(8)
            .connect(
                &dsn,
            )
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "Cannot connect to the database. Please check your configuration. Reason: {}, dsn: {}",
                    e, dsn
                )
            });
        Arc::new(Db(Arc::new(pool)))
    }

    pub async fn migrate(&self) {
        tracing::info!("Migrating database");
        // todo!();
        // sqlx::migrate("../../migrations")
        //     .run(self.pool().as_ref())
        //     .await
        //     .expect("migration failed");
        tracing::info!("Migration completed");
    }

    pub fn pool(&self) -> Arc<Pool<MySql>> {
        self.0.clone()
    }
}
