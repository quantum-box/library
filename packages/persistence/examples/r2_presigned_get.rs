use persistence::{CloudflareR2Driver, Storage};
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
    let url = r2
        .presigned_get("test-bucket", "meisai_20200915204146828.csv", 300)
        .await?;
    println!("{:?}", url.to_string());
    Ok(())
}
