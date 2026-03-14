mod data_query_service;
pub mod data_repository;
pub mod database_repository;
pub mod property_repository;
mod relation_repository;

pub use data_query_service::*;
pub use data_repository::*;
pub use database_repository::*;
pub use property_repository::*;
pub use relation_repository::*;

pub use crate::domain::*;
pub use persistence::Db;
pub use std::sync::Arc;

use value_object::*;

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct ObjectRow {
    pub id: String,
    pub tenant_id: String,
    pub object_name: String,
}

impl From<ObjectRow> for Database {
    fn from(val: ObjectRow) -> Self {
        Database::new(
            &DatabaseId::from_str(&val.id).unwrap(),
            &TenantId::from_str(&val.tenant_id).unwrap(),
            &val.object_name,
        )
    }
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct FieldRow {
    pub id: String,
    pub tenant_id: String,
    pub object_id: String,
    pub field_name: String,
    pub datatype: String,
    pub datatype_meta: serde_json::Value,
    pub is_indexed: bool,
    pub field_num: u32,
    pub meta_json: Option<String>,
}

impl From<FieldRow> for Property {
    fn from(val: FieldRow) -> Self {
        Property::with_meta_json(
            &PropertyId::new(&val.id).unwrap(),
            &TenantId::from_str(&val.tenant_id).unwrap(),
            &DatabaseId::from_str(&val.object_id).unwrap(),
            &val.field_name,
            &PropertyType::from_meta(&val.datatype, val.datatype_meta)
                .unwrap(),
            val.is_indexed,
            val.field_num,
            val.meta_json,
        )
    }
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct ClobRow {
    pub id: u32,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct IndexRow {
    pub id: u32,
    pub tenant_id: String,
    pub object_id: String,
    pub field_num: u32,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct RelationshipRow {
    pub id: String,
    pub tenant_id: String,
    pub object_id: String,
    pub field_id: String,
    pub relation_id: u32,
    pub target_object_id: String,
}

#[derive(sqlx::FromRow, Default)]
pub struct DataRow {
    pub id: String,
    pub tenant_id: String,
    pub object_id: String,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub value0: Option<String>,
    pub value1: Option<String>,
    pub value2: Option<String>,
    pub value3: Option<String>,
    pub value4: Option<String>,
    pub value5: Option<String>,
    pub value6: Option<String>,
    pub value7: Option<String>,
    pub value8: Option<String>,
    pub value9: Option<String>,
    pub value10: Option<String>,
    pub value11: Option<String>,
    pub value12: Option<String>,
    pub value13: Option<String>,
    pub value14: Option<String>,
    pub value15: Option<String>,
    pub value16: Option<String>,
    pub value17: Option<String>,
    pub value18: Option<String>,
    pub value19: Option<String>,
    pub value20: Option<String>,
    pub value21: Option<String>,
    pub value22: Option<String>,
    pub value23: Option<String>,
    pub value24: Option<String>,
    pub value25: Option<String>,
    pub value26: Option<String>,
    pub value27: Option<String>,
    pub value28: Option<String>,
    pub value29: Option<String>,
    pub value30: Option<String>,
    pub value31: Option<String>,
    pub value32: Option<String>,
    pub value33: Option<String>,
    pub value34: Option<String>,
    pub value35: Option<String>,
    pub value36: Option<String>,
    pub value37: Option<String>,
    pub value38: Option<String>,
    pub value39: Option<String>,
    pub value40: Option<String>,
    pub value41: Option<String>,
    pub value42: Option<String>,
    pub value43: Option<String>,
    pub value44: Option<String>,
    pub value45: Option<String>,
    pub value46: Option<String>,
    pub value47: Option<String>,
    pub value48: Option<String>,
    pub value49: Option<String>,
    pub value50: Option<String>,
}

impl DataRow {
    pub fn get_field(&self, field_num: u32) -> anyhow::Result<String> {
        Ok(match field_num {
            0 => self.value0.clone().unwrap_or_default(),
            1 => self.value1.clone().unwrap_or_default(),
            2 => self.value2.clone().unwrap_or_default(),
            3 => self.value3.clone().unwrap_or_default(),
            4 => self.value4.clone().unwrap_or_default(),
            5 => self.value5.clone().unwrap_or_default(),
            6 => self.value6.clone().unwrap_or_default(),
            7 => self.value7.clone().unwrap_or_default(),
            8 => self.value8.clone().unwrap_or_default(),
            9 => self.value9.clone().unwrap_or_default(),
            10 => self.value10.clone().unwrap_or_default(),
            11 => self.value11.clone().unwrap_or_default(),
            12 => self.value12.clone().unwrap_or_default(),
            13 => self.value13.clone().unwrap_or_default(),
            14 => self.value14.clone().unwrap_or_default(),
            15 => self.value15.clone().unwrap_or_default(),
            16 => self.value16.clone().unwrap_or_default(),
            17 => self.value17.clone().unwrap_or_default(),
            18 => self.value18.clone().unwrap_or_default(),
            19 => self.value19.clone().unwrap_or_default(),
            20 => self.value20.clone().unwrap_or_default(),
            21 => self.value21.clone().unwrap_or_default(),
            22 => self.value22.clone().unwrap_or_default(),
            23 => self.value23.clone().unwrap_or_default(),
            24 => self.value24.clone().unwrap_or_default(),
            25 => self.value25.clone().unwrap_or_default(),
            26 => self.value26.clone().unwrap_or_default(),
            27 => self.value27.clone().unwrap_or_default(),
            28 => self.value28.clone().unwrap_or_default(),
            29 => self.value29.clone().unwrap_or_default(),
            30 => self.value30.clone().unwrap_or_default(),
            31 => self.value31.clone().unwrap_or_default(),
            32 => self.value32.clone().unwrap_or_default(),
            33 => self.value33.clone().unwrap_or_default(),
            34 => self.value34.clone().unwrap_or_default(),
            35 => self.value35.clone().unwrap_or_default(),
            36 => self.value36.clone().unwrap_or_default(),
            37 => self.value37.clone().unwrap_or_default(),
            38 => self.value38.clone().unwrap_or_default(),
            39 => self.value39.clone().unwrap_or_default(),
            40 => self.value40.clone().unwrap_or_default(),
            41 => self.value41.clone().unwrap_or_default(),
            42 => self.value42.clone().unwrap_or_default(),
            43 => self.value43.clone().unwrap_or_default(),
            44 => self.value44.clone().unwrap_or_default(),
            45 => self.value45.clone().unwrap_or_default(),
            46 => self.value46.clone().unwrap_or_default(),
            47 => self.value47.clone().unwrap_or_default(),
            48 => self.value48.clone().unwrap_or_default(),
            49 => self.value49.clone().unwrap_or_default(),
            50 => self.value50.clone().unwrap_or_default(),
            _ => anyhow::bail!("Unknown field_num {}", field_num),
        })
    }

    pub fn update_field(
        &mut self,
        field_num: u32,
        value: String,
    ) -> anyhow::Result<()> {
        match field_num {
            0 => self.value0 = Some(value),
            1 => self.value1 = Some(value),
            2 => self.value2 = Some(value),
            3 => self.value3 = Some(value),
            4 => self.value4 = Some(value),
            5 => self.value5 = Some(value),
            6 => self.value6 = Some(value),
            7 => self.value7 = Some(value),
            8 => self.value8 = Some(value),
            9 => self.value9 = Some(value),
            10 => self.value10 = Some(value),
            11 => self.value11 = Some(value),
            12 => self.value12 = Some(value),
            13 => self.value13 = Some(value),
            14 => self.value14 = Some(value),
            15 => self.value15 = Some(value),
            16 => self.value16 = Some(value),
            17 => self.value17 = Some(value),
            18 => self.value18 = Some(value),
            19 => self.value19 = Some(value),
            20 => self.value20 = Some(value),
            21 => self.value21 = Some(value),
            22 => self.value22 = Some(value),
            23 => self.value23 = Some(value),
            24 => self.value24 = Some(value),
            25 => self.value25 = Some(value),
            26 => self.value26 = Some(value),
            27 => self.value27 = Some(value),
            28 => self.value28 = Some(value),
            29 => self.value29 = Some(value),
            30 => self.value30 = Some(value),
            31 => self.value31 = Some(value),
            32 => self.value32 = Some(value),
            33 => self.value33 = Some(value),
            34 => self.value34 = Some(value),
            35 => self.value35 = Some(value),
            36 => self.value36 = Some(value),
            37 => self.value37 = Some(value),
            38 => self.value38 = Some(value),
            39 => self.value39 = Some(value),
            40 => self.value40 = Some(value),
            41 => self.value41 = Some(value),
            42 => self.value42 = Some(value),
            43 => self.value43 = Some(value),
            44 => self.value44 = Some(value),
            45 => self.value45 = Some(value),
            46 => self.value46 = Some(value),
            47 => self.value47 = Some(value),
            48 => self.value48 = Some(value),
            49 => self.value49 = Some(value),
            50 => self.value50 = Some(value),
            _ => anyhow::bail!("Unknown field_num {}", field_num),
        }
        Ok(())
    }
}
