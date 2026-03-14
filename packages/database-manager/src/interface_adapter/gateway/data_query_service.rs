use super::*;
use crate::DataQuery;

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
            let data_field_value = data_row.get_field(field.field_num)?;
            let property = Property::new(
                &field.id.parse()?,
                &field.tenant_id.parse()?,
                &field.object_id.parse()?,
                &field.field_name,
                &field.datatype.parse()?,
                field.is_indexed,
                field.field_num,
            );
            let property_data =
                PropertyData::new(&property, data_field_value)?;
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
        page: u32,
        page_size: u32,
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

        // TODO: add English comment
        let page_i64 = page as i64;
        let page_size_i64 = page_size as i64;

        let data_rows = sqlx::query_as!(
            DataRow,
            r#"
            SELECT
                *
            FROM
                tachyon_apps_database_manager.data
            WHERE
                object_id = ? and tenant_id = ? and name = ?
            ORDER BY
                created_at ASC
            LIMIT ? OFFSET ?
            "#,
            database_id.to_string(),
            tenant_id.to_string(),
            name,
            page_size_i64,
            page_i64
        )
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

        // TODO: add English comment
        let total_u32 = total as u32;
        let mut data_vec = vec![];
        for data_row in data_rows {
            let data = self.data_from_row(data_row, fields.clone())?;
            data_vec.push(data);
        }
        let paginator = OffsetPaginator::new(page, page_size, total_u32);
        Ok((data_vec, paginator))
    }
}
