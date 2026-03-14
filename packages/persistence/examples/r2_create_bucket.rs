use persistence::{CloudflareR2Driver, StorageAdminAccess};
use std::env::var;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv()?;
    let account_id =
        var("R2_ACCOUNT_ID").expect("R2_ACCOUNT_ID is not set");
    let access_key =
        var("R2_ACCESS_KEY").expect("R2_ACCESS_KEY is not set");
    let secret_key =
        var("R2_SECRET_ACCESS_KEY").expect("R2_SECRET_KEY is not set");
    let r2 =
        CloudflareR2Driver::new(&account_id, &access_key, &secret_key)?;
    r2.create_bucket("test-bucket").await?;
    Ok(())
}
