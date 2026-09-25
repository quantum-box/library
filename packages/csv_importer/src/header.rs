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
use std::collections::HashSet;

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
        let mut used_property_keys = HashSet::new();
        for (index, header) in headers.iter().enumerate() {
            let name =
                property_key_from_header(header, &mut used_property_keys);
            let display_name = header
                .chars()
                .take(database_manager::domain::MAX_PROPERTY_DISPLAY_NAME_LENGTH)
                .collect::<String>();
            let display_name = if display_name.trim().is_empty() {
                format!("Column {}", index + 1)
            } else {
                display_name
            };
            let property = self
                .db_manager
                .add_property()
                .execute(AddPropertyInputData {
                    executor,
                    multi_tenancy,
                    tenant_id: config.tenant_id,
                    database_id: database.id(),
                    display_name: Some(&display_name),
                    name: &name,
                    property_type:
                        database_manager::domain::PropertyType::String,
                })
                .await?;
            properties.push(property);
        }
        Ok((database, properties))
    }
}

fn property_key_from_header(
    header: &str,
    used: &mut HashSet<String>,
) -> String {
    let mut key = String::new();
    let mut needs_separator = false;
    for character in header.chars() {
        if character.is_ascii_alphanumeric()
            || character == '_'
            || character == '-'
        {
            if needs_separator
                && !key.is_empty()
                && !key.ends_with('_')
                && !key.ends_with('-')
            {
                key.push('_');
            }
            key.push(character);
            needs_separator = false;
        } else {
            needs_separator = true;
        }
    }
    if key.is_empty() {
        key.push_str("column");
    } else if !key.as_bytes()[0].is_ascii_alphabetic() {
        key.insert_str(0, "column_");
    }

    let max_length = database_manager::domain::MAX_PROPERTY_KEY_LENGTH;
    key.truncate(max_length);
    let key = key.trim_end_matches(['_', '-']);
    let base = if key.is_empty() { "column" } else { key };
    let mut candidate = base.to_owned();
    let mut suffix = 2;
    while used.contains(&candidate.to_ascii_lowercase()) {
        let ending = format!("_{suffix}");
        let prefix_length = max_length.saturating_sub(ending.len());
        candidate =
            format!("{}{ending}", &base[..base.len().min(prefix_length)]);
        suffix += 1;
    }
    used.insert(candidate.to_ascii_lowercase());
    candidate
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
