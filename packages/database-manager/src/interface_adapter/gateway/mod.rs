mod data_query_service;
pub mod data_repository;
pub mod database_repository;
mod index_definition_repository;
pub mod property_repository;
mod property_value_backfill;
mod property_value_storage;
mod record_mutation_uow;
mod relation_edge_repository;
mod relation_repository;
mod relation_schema_repository;

pub use data_query_service::*;
pub use data_repository::*;
pub use database_repository::*;
pub use index_definition_repository::*;
pub use property_repository::*;
pub use property_value_backfill::*;
pub use property_value_storage::*;
pub use relation_edge_repository::*;
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
    pub type_key: Option<String>,
    pub type_version: Option<u16>,
    pub type_config: Option<String>,
}

impl FieldRow {
    fn definition_with_config(
        &self,
        config: ResolvedPropertyConfig,
    ) -> errors::Result<PropertyDefinition> {
        Ok(PropertyDefinition::new(
            &PropertyId::new(&self.id)?,
            &TenantId::from_str(&self.tenant_id)?,
            &DatabaseId::from_str(&self.object_id)?,
            &self.field_name,
            config,
            self.is_indexed,
            self.field_num,
            self.meta_json.clone(),
        ))
    }

    pub fn legacy_definition(&self) -> errors::Result<PropertyDefinition> {
        self.definition_with_config(ResolvedPropertyConfig::Known(
            PropertyType::from_meta(
                &self.datatype,
                self.datatype_meta.clone(),
            )?
            .canonical_config(),
        ))
    }

    /// Decode the canonical envelope without consulting the legacy columns.
    /// `None` is the only state where canonical-read may fall back to legacy.
    pub fn canonical_definition(
        &self,
    ) -> errors::Result<Option<PropertyDefinition>> {
        let (type_key, type_version, type_config) = match (
            self.type_key.as_ref(),
            self.type_version,
            self.type_config.as_ref(),
        ) {
            (None, None, None) => return Ok(None),
            (Some(key), Some(version), Some(config)) => {
                (key, version, config)
            }
            _ => {
                return Err(errors::Error::invalid(
                    "partial PropertyDefinition envelope",
                ));
            }
        };
        let type_ref = PropertyTypeRef::new(
            PropertyTypeKey::new(type_key.clone())?,
            PropertyTypeVersion::new(type_version)?,
        );
        let raw_config = serde_json::from_str(type_config)
            .map_err(errors::Error::invalid)?;
        let config = BUILTIN_PROPERTY_TYPE_REGISTRY
            .decode_config(type_ref, raw_config)?;
        Ok(Some(self.definition_with_config(config)?))
    }

    pub fn definition(
        &self,
        mode: crate::property_definition_rollout::PropertyDefinitionStorageMode,
    ) -> errors::Result<PropertyDefinition> {
        if mode.reads_canonical_first() {
            return match self.canonical_definition()? {
                Some(definition) => Ok(definition),
                None => self.legacy_definition(),
            };
        }

        let legacy = self.legacy_definition()?;
        let parity = match self.canonical_definition() {
            Ok(None) => "missing_canonical",
            Err(error) => {
                tracing::warn!(
                    tenant_id = %self.tenant_id,
                    database_id = %self.object_id,
                    property_id = %self.id,
                    %error,
                    "ignored canonical PropertyDefinition during legacy-first read"
                );
                "decode_failure"
            }
            Ok(Some(canonical)) => match canonical.config() {
                ResolvedPropertyConfig::Opaque(_) => "opaque",
                ResolvedPropertyConfig::Known(_) => {
                    if canonical.type_ref() == legacy.type_ref()
                        && canonical.raw_config()? == legacy.raw_config()?
                    {
                        "match"
                    } else {
                        "mismatch"
                    }
                }
            },
        };
        tracing::debug!(
            tenant_id = %self.tenant_id,
            database_id = %self.object_id,
            property_id = %self.id,
            parity,
            "PropertyDefinition dual-read parity"
        );
        Ok(legacy)
    }

    pub fn ensure_canonical_definition_writable(
        &self,
    ) -> errors::Result<()> {
        if let Some(definition) = self.canonical_definition()? {
            definition.config().ensure_writable()?;
        }
        Ok(())
    }

    /// Decode a definition for a schema write without crossing the active
    /// rollout mode or allowing one representation to overwrite a mismatch in
    /// the other. Unknown and malformed canonical envelopes remain read-only.
    pub fn definition_for_schema_write(
        &self,
        mode: crate::property_definition_rollout::PropertyDefinitionStorageMode,
    ) -> errors::Result<PropertyDefinition> {
        self.ensure_canonical_definition_writable()?;
        let selected = self.definition(mode)?;
        if let Some(canonical) = self.canonical_definition()? {
            let legacy = self.legacy_definition()?;
            if canonical.type_ref() != legacy.type_ref()
                || canonical.raw_config()? != legacy.raw_config()?
            {
                return Err(errors::Error::conflict(
                    "legacy and canonical PropertyDefinition configs do not match",
                ));
            }
        }
        Ok(selected)
    }
}

#[cfg(test)]
mod property_definition_row_tests {
    use super::*;
    use crate::property_definition_rollout::PropertyDefinitionStorageMode;

    fn row() -> FieldRow {
        FieldRow {
            id: PropertyId::default().to_string(),
            tenant_id: TenantId::default().to_string(),
            object_id: DatabaseId::default().to_string(),
            field_name: "title".to_string(),
            datatype: "STRING".to_string(),
            datatype_meta: serde_json::Value::Null,
            is_indexed: false,
            field_num: 0,
            meta_json: None,
            type_key: None,
            type_version: None,
            type_config: None,
        }
    }

    #[test]
    fn canonical_read_falls_back_only_when_the_envelope_is_absent() {
        let mut row = row();
        row.datatype = "UNREADABLE_LEGACY_TYPE".to_string();
        row.type_key = Some("string".to_string());
        row.type_version = Some(1);
        row.type_config = Some("null".to_string());

        let canonical = row
            .definition(
                PropertyDefinitionStorageMode::DualWriteCanonicalRead,
            )
            .expect("canonical definition must not decode legacy columns");
        assert_eq!(canonical.type_ref().key.as_str(), "string");

        row.type_key = None;
        row.type_version = None;
        row.type_config = None;
        assert!(row
            .definition(
                PropertyDefinitionStorageMode::DualWriteCanonicalRead
            )
            .is_err());
    }

    #[test]
    fn malformed_known_canonical_config_fails_closed() {
        let mut row = row();
        row.type_key = Some("select".to_string());
        row.type_version = Some(1);
        row.type_config = Some("null".to_string());

        assert!(row
            .definition(
                PropertyDefinitionStorageMode::DualWriteCanonicalRead
            )
            .is_err());
        assert!(row
            .definition(PropertyDefinitionStorageMode::DualWriteLegacyRead)
            .is_ok());
    }

    #[test]
    fn partial_canonical_envelope_never_falls_back_to_legacy() {
        let mut row = row();
        row.type_key = Some("string".to_string());

        assert!(row
            .definition(
                PropertyDefinitionStorageMode::DualWriteCanonicalRead
            )
            .is_err());
    }

    #[test]
    fn unknown_canonical_config_stays_opaque_and_lossless() {
        let mut row = row();
        row.type_key = Some("future_type".to_string());
        row.type_version = Some(9);
        row.type_config =
            Some(r#"{"feature":{"enabled":true}}"#.to_string());

        let definition = row
            .definition(
                PropertyDefinitionStorageMode::DualWriteCanonicalRead,
            )
            .expect("unknown envelopes are readable");
        assert!(matches!(
            definition.config(),
            ResolvedPropertyConfig::Opaque(_)
        ));
        assert_eq!(
            definition.raw_config().expect("opaque raw config"),
            serde_json::json!({"feature": {"enabled": true}})
        );
        assert!(definition.to_property().is_err());
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
pub struct RelationDefinitionRow {
    pub id: String,
    pub tenant_id: String,
    pub object_id: String,
    pub field_id: String,
    pub target_object_id: String,
    pub forward_cardinality: String,
    pub reverse_cardinality: String,
    pub inverse_field_id: Option<String>,
    pub inverse_owned: bool,
    pub on_target_delete: String,
    pub definition_version: u16,
    pub generation: u64,
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
    pub record_version: u64,
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
    pub fn get_field(
        &self,
        field_num: u32,
    ) -> anyhow::Result<Option<String>> {
        Ok(match field_num {
            0 => self.value0.clone(),
            1 => self.value1.clone(),
            2 => self.value2.clone(),
            3 => self.value3.clone(),
            4 => self.value4.clone(),
            5 => self.value5.clone(),
            6 => self.value6.clone(),
            7 => self.value7.clone(),
            8 => self.value8.clone(),
            9 => self.value9.clone(),
            10 => self.value10.clone(),
            11 => self.value11.clone(),
            12 => self.value12.clone(),
            13 => self.value13.clone(),
            14 => self.value14.clone(),
            15 => self.value15.clone(),
            16 => self.value16.clone(),
            17 => self.value17.clone(),
            18 => self.value18.clone(),
            19 => self.value19.clone(),
            20 => self.value20.clone(),
            21 => self.value21.clone(),
            22 => self.value22.clone(),
            23 => self.value23.clone(),
            24 => self.value24.clone(),
            25 => self.value25.clone(),
            26 => self.value26.clone(),
            27 => self.value27.clone(),
            28 => self.value28.clone(),
            29 => self.value29.clone(),
            30 => self.value30.clone(),
            31 => self.value31.clone(),
            32 => self.value32.clone(),
            33 => self.value33.clone(),
            34 => self.value34.clone(),
            35 => self.value35.clone(),
            36 => self.value36.clone(),
            37 => self.value37.clone(),
            38 => self.value38.clone(),
            39 => self.value39.clone(),
            40 => self.value40.clone(),
            41 => self.value41.clone(),
            42 => self.value42.clone(),
            43 => self.value43.clone(),
            44 => self.value44.clone(),
            45 => self.value45.clone(),
            46 => self.value46.clone(),
            47 => self.value47.clone(),
            48 => self.value48.clone(),
            49 => self.value49.clone(),
            50 => self.value50.clone(),
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

/// Project an auto-generated Id from the canonical row ID without mutating
/// legacy rows. Pre-policy values remain visible (and immutable) so rollout
/// does not make existing records unreadable; they are also surfaced in logs
/// for a later repair migration.
pub(super) fn project_property_value(
    data_id: &str,
    property_type: &PropertyType,
    stored_value: String,
) -> errors::Result<String> {
    if let PropertyType::Id(TypeId {
        auto_generate: true,
    }) = property_type
    {
        if stored_value.is_empty() || stored_value == data_id {
            return Ok(data_id.to_string());
        }

        tracing::warn!(
            canonical_data_id = data_id,
            "legacy auto-generated Id value differs from canonical DataId"
        );
    }

    Ok(stored_value)
}

#[cfg(test)]
mod projection_tests {
    use super::*;

    #[test]
    fn missing_auto_generated_id_projects_the_canonical_data_id() {
        let projected = project_property_value(
            "data_01canonical",
            &PropertyType::Id(TypeId::new(true)),
            String::new(),
        )
        .expect("projection");

        assert_eq!(projected, "data_01canonical");
    }

    #[test]
    fn legacy_non_canonical_auto_generated_id_remains_visible() {
        let projected = project_property_value(
            "data_01canonical",
            &PropertyType::Id(TypeId::new(true)),
            "external-id".to_string(),
        )
        .expect("legacy projection");

        assert_eq!(projected, "external-id");
    }

    #[test]
    fn manual_id_preserves_its_stored_value() {
        let projected = project_property_value(
            "data_01canonical",
            &PropertyType::Id(TypeId::new(false)),
            "external-id".to_string(),
        )
        .expect("projection");

        assert_eq!(projected, "external-id");
    }
}
