mod header;

use database_manager::domain::{Database, DatabaseId};
use database_manager::*;
use domain::Property;
use futures::stream::{self, StreamExt};
use std::{io::Cursor, sync::Arc};
use tachyon_sdk::auth::{ExecutorAction, MultiTenancyAction};
use value_object::*;

#[derive(Debug, Clone)]
pub struct DatabaseConfig<'a> {
    pub name: &'a str,
    pub tenant_id: &'a TenantId,
    pub database_id: Option<DatabaseId>,
}

#[derive(Debug, Clone)]
pub struct CSVImporterClient {
    db_manager: Arc<database_manager::App>,
}

impl CSVImporterClient {
    pub fn new(db_manager: Arc<database_manager::App>) -> Arc<Self> {
        Arc::new(Self { db_manager })
    }
}

#[async_trait::async_trait]
pub trait CSVImporter {
    /// #
    async fn import_from_url<'a>(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        url: &str,
        config: DatabaseConfig<'a>,
    ) -> anyhow::Result<Database>;

    async fn import<'a>(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        reader: Cursor<String>,
        config: DatabaseConfig<'a>,
    ) -> anyhow::Result<Database>;
    /// # preview csv
    ///         
    /// ```rust
    /// let dsn =
    ///     "mysql://root:@localhost:15000/tachyon_apps_database_manager";
    /// let db_manager = database_manager::factory_client(dsn).await?;
    /// let csv_importer = CSVImporterClient::new(Arc::new(db_manager));
    /// TODO: add English documentation
    /// TODO: add English documentation
    /// TODO: add English documentation
    /// TODO: add English documentation
    /// TODO: add English documentation
    /// TODO: add English documentation
    /// TODO: add English documentation
    ///     .to_string();
    /// let buffer = std::io::Cursor::new(csv_str);
    /// let preview = csv_importer.preview(buffer, Some(3)).await?;
    /// for record in preview.iter() {
    ///     println!("{:?}", record);
    /// }
    /// Ok(())
    /// ```
    async fn preview(
        &self,
        reader: Cursor<String>,
        line: Option<u8>,
    ) -> anyhow::Result<Vec<Vec<String>>>;
}

#[async_trait::async_trait]
impl CSVImporter for CSVImporterClient {
    async fn import_from_url<'a>(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        url: &str,
        config: DatabaseConfig<'a>,
    ) -> anyhow::Result<Database> {
        // TODO: add English comment
        // TODO: add English comment

        let response: reqwest::Response = reqwest::get(url).await?;
        let bytes = response.bytes().await?;
        let (decoded_bytes, _, had_errors) =
            encoding_rs::SHIFT_JIS.decode(&bytes);
        let utf8_bytes = if had_errors {
            // TODO: add English comment
            std::str::from_utf8(&bytes)?.to_string()
        } else {
            decoded_bytes.into_owned()
        };
        let reader = std::io::Cursor::new(utf8_bytes);
        let mut rdr = csv::Reader::from_reader(reader);

        let headers = rdr.headers()?;
        let (db, properties) = self
            .process_headers(executor, multi_tenancy, headers, &config)
            .await?;
        self.process_records(
            executor,
            multi_tenancy,
            rdr,
            &db,
            &properties,
        )
        .await?;

        Ok(db)
    }

    /// # import csv
    ///
    /// TODO: add English documentation
    async fn import<'a>(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        reader: Cursor<String>,
        config: DatabaseConfig<'a>,
    ) -> anyhow::Result<Database> {
        let mut rdr = csv::Reader::from_reader(reader);
        let headers = rdr.headers()?;
        let (db, properties) = self
            .process_headers(executor, multi_tenancy, headers, &config)
            .await?;
        self.process_records(
            executor,
            multi_tenancy,
            rdr,
            &db,
            &properties,
        )
        .await?;

        Ok(db)
    }

    async fn preview(
        &self,
        reader: Cursor<String>,
        line: Option<u8>,
    ) -> anyhow::Result<Vec<Vec<String>>> {
        let mut rdr = csv::Reader::from_reader(reader);
        let headers = rdr.headers()?;
        let headers = headers
            .iter()
            .map(|h| h.to_string())
            .collect::<Vec<String>>();

        let mut records = vec![headers];
        let line_count = line.unwrap_or(4);

        // TODO: add English comment
        for result in rdr.records().take(line_count as usize) {
            let record = result?;
            let record = record
                .iter()
                .map(|r| r.to_string())
                .collect::<Vec<String>>();
            records.push(record);
        }

        Ok(records)
    }
}

impl CSVImporterClient {
    async fn create_data_from_record(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        record: Vec<String>,
        properties: &[Property],
        database_id: &DatabaseId,
        tenant_id: &TenantId,
    ) -> anyhow::Result<()> {
        if record.len() != properties.len() {
            return Err(anyhow::anyhow!("record length is not match"));
        }
        let mut property_data_list = Vec::new();
        let name = record[0].clone(); // TODO: add English comment
        for (ix, val) in record.into_iter().enumerate() {
            property_data_list.push(PropertyDataInputData {
                property_id: properties[ix].id().to_string(),
                value: val,
            });
        }
        self.db_manager
            .add_data_usecase()
            .execute(AddDataInputData {
                executor,
                multi_tenancy,
                tenant_id,
                database_id,
                name: &name, // TODO: add English comment
                property_data: property_data_list,
            })
            .await?;
        Ok(())
    }

    async fn process_records<R: std::io::Read>(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        mut rdr: csv::Reader<R>,
        db: &Database,
        properties: &[Property],
    ) -> anyhow::Result<()> {
        let concurrency_limit = 36; // TODO: add English comment

        let results = stream::iter(rdr.records())
            .map(|result| {
                let db_id = db.id().clone();
                let tenant_id = db.tenant_id().clone();
                let db_manager = Arc::clone(&self.db_manager);

                async move {
                    let record = result?;
                    let record = record
                        .iter()
                        .map(|r| r.to_string())
                        .collect::<Vec<String>>();
                    let client = CSVImporterClient { db_manager };
                    client
                        .create_data_from_record(
                            executor,
                            multi_tenancy,
                            record,
                            properties,
                            &db_id,
                            &tenant_id,
                        )
                        .await
                }
            })
            .buffer_unordered(concurrency_limit)
            .collect::<Vec<anyhow::Result<()>>>()
            .await;

        // TODO: add English comment
        for res in results {
            res?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tachyon_sdk::auth::{Executor, MultiTenancy};

    const URL: &str = "https://firebasestorage.googleapis.com/v0/b/tachyon-4d47.appspot.com/o/tenant%2Fairsales%2F9e937f820b248a92.xlsx%20-%20%E5%BE%97%E6%84%8F%E5%85%88%E6%83%85%E5%A0%B1220621.csv?alt=media&token=414dcbc8-667d-4757-a8c0-184ff5bef74f";

    #[ignore]
    #[tokio::test]
    async fn test_import_from_url() -> anyhow::Result<()> {
        let tenant_id =
            TenantId::from_str("tn_01H2MWCYDMZ91EJHV5HES4YXS4")?;
        let dsn = "mysql://root:root@localhost:14000/db_test";
        let db_manager = database_manager::factory_client(dsn).await?;
        let csv_importer = CSVImporterClient::new(Arc::new(db_manager));
        csv_importer
            .import_from_url(
                &Executor::SystemUser,
                &MultiTenancy::new_operator(tenant_id.clone()),
                URL,
                DatabaseConfig {
                    name: "test",
                    tenant_id: &tenant_id,
                    database_id: None,
                },
            )
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_preview_from_string() -> anyhow::Result<()> {
        let dsn =
            "mysql://root:@localhost:15000/tachyon_apps_database_manager";
        let db_manager = database_manager::factory_client(dsn).await?;
        let csv_importer = CSVImporterClient::new(Arc::new(db_manager));
        let csv_str = "名前,年齢,職業\n\
        タナカ,30,エンジニア\n\
        ヤマダ,25,デザイナー\n\
        サトウ,35,マネージャー\n\
        スズキ,28,営業\n\
        ワタナベ,40,経理\n\
        イトウ,33,人事"
            .to_string();
        let buffer = std::io::Cursor::new(csv_str);
        let preview = csv_importer.preview(buffer, Some(3)).await?;
        for record in preview.iter() {
            println!("{:?}", record);
        }
        assert_eq!(preview.len(), 4);
        assert_eq!(preview[0], vec!["名前", "年齢", "職業"]);
        assert_eq!(preview[1], vec!["タナカ", "30", "エンジニア"]);
        assert_eq!(preview[2], vec!["ヤマダ", "25", "デザイナー"]);
        assert_eq!(preview[3], vec!["サトウ", "35", "マネージャー"]);
        Ok(())
    }
}
