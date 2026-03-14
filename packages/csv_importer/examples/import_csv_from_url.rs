use tachyon_sdk::auth::{Executor, MultiTenancy};
use csv_importer::{CSVImporter, CSVImporterClient, DatabaseConfig};
use persistence::{CloudflareR2Driver, Storage};
use std::env::var;
use std::sync::Arc;
use value_object::TenantId;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv()?;
    let tenant_id: TenantId = "tn_01hvtdk06gf8k57gfe2bwghbc6".parse()?;
    let account_id =
        var("R2_ACCOUNT_ID").expect("R2_ACCOUNT_ID is not set");
    let access_key =
        var("R2_ACCESS_KEY").expect("R2_ACCESS_KEY is not set");
    let secret_key =
        var("R2_SECRET_ACCESS_KEY").expect("R2_SECRET_KEY is not set");
    let db_manager_dsn = var("DB_MANAGER_DATABASE_URL")
        .expect("DB_MANAGER_DATABASE_URL is not set");
    let db_manager =
        database_manager::factory_client(&db_manager_dsn).await?;
    let r2 =
        CloudflareR2Driver::new(&account_id, &access_key, &secret_key)?;
    let csv_importer_client =
        CSVImporterClient::new(Arc::new(db_manager.clone()));

    let executor = &Executor::SystemUser;
    let multi_tenancy = &MultiTenancy::new_operator(&tenant_id);

    let url = r2
        .presigned_get("test-bucket", "meisai_20200915204146828.csv", 300)
        .await?;
    let database = csv_importer_client
        .import_from_url(
            executor,
            multi_tenancy,
            url.as_ref(),
            DatabaseConfig {
                name: "test2",
                tenant_id: &tenant_id,
                database_id: None,
            },
        )
        .await?;
    let database_definition = db_manager
        .get_database_definition_usecase()
        .execute(
            database_manager::usecase::GetDatabaseDefinitionInputData {
                executor,
                multi_tenancy,
                database_id: database.id(),
                tenant_id: &tenant_id,
            },
        )
        .await?;
    println!("{:#?}", database_definition);

    Ok(())
}
