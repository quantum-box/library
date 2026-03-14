use std::sync::Arc;

use value_object::DatabaseUrl;

pub async fn setup_test_db(use_database_name: &str) -> Arc<crate::Db> {
    dotenvy::dotenv().ok();
    let database_url: DatabaseUrl = std::env::var("DEV_DATABASE_URL")
        .unwrap_or_else(|_| {
            "mysql://root:@localhost:15000/tachyon_apps_llms".to_string()
        })
        .parse()
        .unwrap();
    let database_url = database_url.use_database(use_database_name);
    crate::Db::new(database_url).await
}
