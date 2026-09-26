use super::*;
use crate::property_definition_rollout::PropertyDefinitionStorageMode;
use crate::property_value_rollout::PropertyValueStorageMode;
use crate::DataQuery;

const SEARCH_DATA_BY_NAME_SQL: &str = r#"
    SELECT
        *
    FROM
        data
    WHERE
        tenant_id = ? and object_id = ? and name = ?
    ORDER BY
        created_at ASC, id ASC
    LIMIT ? OFFSET ?
"#;

#[derive(Clone, Debug)]
pub struct DataQueryService {
    pub db: Arc<Db>,
    pub property_value_mode: PropertyValueStorageMode,
    pub property_definition_mode: PropertyDefinitionStorageMode,
}

impl DataQueryService {
    pub fn new(db: Arc<Db>) -> Arc<Self> {
        Self::new_with_storage_modes(
            db,
            Default::default(),
            Default::default(),
        )
    }

    pub fn new_with_property_value_mode(
        db: Arc<Db>,
        property_value_mode: PropertyValueStorageMode,
    ) -> Arc<Self> {
        Self::new_with_storage_modes(
            db,
            property_value_mode,
            Default::default(),
        )
    }

    pub fn new_with_storage_modes(
        db: Arc<Db>,
        property_value_mode: PropertyValueStorageMode,
        property_definition_mode: PropertyDefinitionStorageMode,
    ) -> Arc<Self> {
        Arc::new(Self {
            db,
            property_value_mode,
            property_definition_mode,
        })
    }
}

#[async_trait::async_trait]
impl DataQuery for DataQueryService {
    async fn search_by_name(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        name: &str,
        page: OffsetPage,
    ) -> anyhow::Result<(Vec<Data>, OffsetPaginator)> {
        // TODO: add English comment
        let fields = sqlx::query_as::<_, FieldRow>(
            r#"
            SELECT
                id,
                tenant_id,
                object_id,
                field_name,
                datatype,
                datatype_meta,
                is_indexed,
                field_num,
                meta_json,
                type_key,
                type_version,
                type_config
            FROM
                fields
            WHERE
                object_id = ? and tenant_id = ?
            ORDER BY
                field_num ASC
            "#,
        )
        .bind(database_id.to_string())
        .bind(tenant_id.to_string())
        .fetch_all(self.db.pool().as_ref())
        .await?;

        let data_rows =
            sqlx::query_as::<_, DataRow>(SEARCH_DATA_BY_NAME_SQL)
                .bind(tenant_id.to_string())
                .bind(database_id.to_string())
                .bind(name)
                .bind(page.items_per_page())
                .bind(page.offset())
                .fetch_all(self.db.pool().as_ref())
                .await?;

        let total = sqlx::query_scalar!(
            r#"
            SELECT
                COUNT(*)
            FROM
                data
            WHERE
                tenant_id = ? and object_id = ? and name = ?    
            "#,
            tenant_id.to_string(),
            database_id.to_string(),
            name
        )
        .fetch_one(self.db.pool().as_ref())
        .await?;

        let total = u32::try_from(total).map_err(|_| {
            anyhow::anyhow!(
                "data count exceeds the supported pagination range"
            )
        })?;
        let data_ids = data_rows
            .iter()
            .map(|row| row.id.clone())
            .collect::<Vec<_>>();
        let canonical = load_canonical_values_for_mode(
            self.db.as_ref(),
            tenant_id,
            database_id,
            &data_ids,
            self.property_value_mode,
        )
        .await
        .map_err(anyhow::Error::from)?;
        let mut data_vec = vec![];
        for data_row in data_rows {
            let data = hydrate_data_row(
                data_row,
                &fields,
                &canonical,
                self.property_value_mode,
                self.property_definition_mode,
            )
            .map_err(anyhow::Error::from)?;
            data_vec.push(data);
        }
        let paginator = OffsetPaginator::new(page, total);
        Ok((data_vec, paginator))
    }
}

#[cfg(test)]
mod query_contract_tests {
    use super::*;

    fn normalize(sql: &str) -> String {
        sql.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase()
    }

    #[test]
    fn filtered_search_is_tenant_scoped_and_deterministic() {
        let sql = normalize(SEARCH_DATA_BY_NAME_SQL);

        assert!(sql.contains(
            "where tenant_id = ? and object_id = ? and name = ?"
        ));
        assert!(sql.contains("order by created_at asc, id asc"));
        assert!(sql.contains("limit ? offset ?"));
    }
}
