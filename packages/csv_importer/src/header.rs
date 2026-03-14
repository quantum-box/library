//! # header
//!
//! ## Todo
//!
//! TODO: add English documentation
//! TODO: add English documentation
//!
//!
//! TODO: add English documentation
//! TODO: add English documentation
//! TODO: add English documentation
//! TODO: add English documentation
//!
//! ```json,ignore
//! {
//!     "id": "",
//!     "name": "",
//!     "description": "",
//! }
//! ```

use csv::StringRecord;
use database_manager::{
    domain::{Database, Property},
    AddPropertyInputData, CreateDatabaseInputData,
};
use tachyon_sdk::auth::{ExecutorAction, MultiTenancyAction};

use crate::{CSVImporterClient, DatabaseConfig};

impl CSVImporterClient {
    pub async fn process_headers(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        headers: &StringRecord,
        config: &DatabaseConfig<'_>,
    ) -> anyhow::Result<(Database, Vec<Property>)> {
        let headers = headers
            .iter()
            .map(|h| h.to_string())
            .collect::<Vec<String>>();
        let (db, properties) = self
            .create_property_from_headers(
                executor,
                multi_tenancy,
                headers,
                config,
            )
            .await?;
        Ok((db, properties))
    }

    async fn create_property_from_headers(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        headers: Vec<String>,
        config: &DatabaseConfig<'_>,
    ) -> anyhow::Result<(Database, Vec<Property>)> {
        let database = self
            .db_manager
            .create_database()
            .execute(CreateDatabaseInputData {
                executor,
                multi_tenancy,
                database_id: config.database_id.as_ref(),
                tenant_id: config.tenant_id,
                name: config.name,
            })
            .await?;
        let mut properties = Vec::new();
        for header in headers.iter() {
            let property = self
                .db_manager
                .add_property()
                .execute(AddPropertyInputData {
                    executor,
                    multi_tenancy,
                    tenant_id: config.tenant_id,
                    database_id: database.id(),
                    name: header,
                    property_type:
                        database_manager::domain::PropertyType::String,
                })
                .await?;
            properties.push(property);
        }
        Ok((database, properties))
    }
}

// const SYSTEM_PROMPT: &str = r#"
// Extract the relevant information and output it in JSON format following specific rules.

// - Identify a column that corresponds to "id" and use it. The "id" should be a unique identifier or key and can be found flexibly.
// - For a column corresponding to "name," do not create a new property; use the existing one as is.
// - If there is a column corresponding to "description," create a "description" entry.

// # Output Format

// The output should be formatted in JSON as shown in the template below:

// ```json
// {
//     "id": "",
//     "name": "",
//     "description": ""
// }
// ```

// # Notes

// - If a corresponding column for any of the fields does not exist, that field should remain empty in the JSON output.
// - These instructions can be interpreted flexibly to best match the available data.
// "#;
