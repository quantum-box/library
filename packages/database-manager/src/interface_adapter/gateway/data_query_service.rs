use super::*;
use crate::DataQuery;

const SEARCH_DATA_BY_NAME_SQL: &str = r#"
    SELECT
        *
    FROM
        tachyon_apps_database_manager.data
    WHERE
        tenant_id = ? and object_id = ? and name = ?
    ORDER BY
        created_at ASC, id ASC
    LIMIT ? OFFSET ?
"#;

#[derive(Clone, Debug)]
pub struct DataQueryService {
    pub db: Arc<Db>,
}

impl DataQueryService {
    pub fn new(db: Arc<Db>) -> Arc<Self> {
        Arc::new(Self { db })
    }

    fn data_from_row(
        &self,
        data_row: DataRow,
        fields: Vec<FieldRow>,
    ) -> anyhow::Result<Data> {
        let tenant_id = TenantId::from_str(&data_row.tenant_id)?;
        let database_id = DatabaseId::from_str(&data_row.object_id)?;
        let mut data = Data::new(
            &data_row.id.parse()?,
            &tenant_id,
            &database_id,
            &data_row.name,
            vec![],
            data_row.created_at,
            data_row.updated_at,
        )?;
        for field in fields {
            let property_type = PropertyType::from_meta(
                &field.datatype,
                field.datatype_meta.clone(),
            )?;
            let data_field_value = project_property_value(
                &data_row.id,
                &property_type,
                data_row.get_field(field.field_num)?,
            )?;
            let property = Property::with_meta_json(
                &field.id.parse()?,
                &field.tenant_id.parse()?,
                &field.object_id.parse()?,
                &field.field_name,
                &property_type,
                field.is_indexed,
                field.field_num,
                field.meta_json.clone(),
            );
            let property_data =
                PropertyData::from_storage(&property, data_field_value)?;
            data.add_property_data(property_data)?;
        }
        Ok(data)
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
        let fields = sqlx::query_as!(
            FieldRow,
            r#"
            SELECT
                id,
                tenant_id,
                object_id,
                field_name,
                datatype,
                datatype_meta,
                is_indexed as `is_indexed: bool`,
                field_num,
                meta_json
            FROM
                tachyon_apps_database_manager.fields
            WHERE
                object_id = ? and tenant_id = ?
            ORDER BY
                field_num ASC
            "#,
            database_id.to_string(),
            tenant_id.to_string()
        )
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
                tachyon_apps_database_manager.data
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
        let mut data_vec = vec![];
        for data_row in data_rows {
            let data = self.data_from_row(data_row, fields.clone())?;
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
