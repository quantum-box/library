use csv_importer::{CSVImporter, CSVImporterClient};
use std::env;
use std::io::Read;
use std::sync::Arc;

#[tokio::test]
#[ignore]
async fn test_preview() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let dsn = std::env::var("DEV_DATABASE_URL")
        .map(|url| format!("{}/tachyon_apps_database_manager", url))
        .unwrap_or_else(|_| {
            "mysql://root:@localhost:15000/tachyon_apps_database_manager"
                .to_string()
        });
    let db_manager = database_manager::factory_client(dsn).await?;
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("mock.csv");
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    let mut buffer = String::new();
    reader.read_to_string(&mut buffer)?;
    let csv_importer = CSVImporterClient::new(Arc::new(db_manager));
    let buffer = std::io::Cursor::new(buffer);
    let preview = csv_importer.preview(buffer, Some(100)).await?;
    for record in preview.iter() {
        println!("{:?}", record);
    }
    Ok(())
}
