use super::*;
use serde_json::Value;
use std::fmt::Debug;

/// Canonical, storage-neutral definition of a Property.
///
/// `Property` remains the compatibility projection used by callers that only
/// understand built-in types. This model also retains unknown type/config
/// envelopes so an older binary can read them without inventing a String type.
#[derive(Debug, Clone)]
pub struct PropertyDefinition {
    id: PropertyId,
    tenant_id: TenantId,
    database_id: DatabaseId,
    name: String,
    config: ResolvedPropertyConfig,
    is_indexed: bool,
    property_num: u32,
    meta_json: Option<String>,
}

impl PropertyDefinition {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: &PropertyId,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        name: &str,
        config: ResolvedPropertyConfig,
        is_indexed: bool,
        property_num: u32,
        meta_json: Option<String>,
    ) -> Self {
        Self {
            id: id.clone(),
            tenant_id: tenant_id.clone(),
            database_id: database_id.clone(),
            name: if name.is_empty() {
                format!("property{property_num}")
            } else {
                name.to_string()
            },
            config,
            is_indexed,
            property_num,
            meta_json,
        }
    }

    pub fn from_property(property: &Property) -> Self {
        Self::new(
            property.id(),
            property.tenant_id(),
            property.database_id(),
            property.name(),
            ResolvedPropertyConfig::Known(
                property.property_type().canonical_config(),
            ),
            *property.is_indexed(),
            *property.property_num(),
            property.meta_json().clone(),
        )
    }

    pub fn id(&self) -> &PropertyId {
        &self.id
    }

    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    pub fn database_id(&self) -> &DatabaseId {
        &self.database_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn config(&self) -> &ResolvedPropertyConfig {
        &self.config
    }

    pub const fn is_indexed(&self) -> bool {
        self.is_indexed
    }

    pub const fn property_num(&self) -> u32 {
        self.property_num
    }

    pub fn meta_json(&self) -> &Option<String> {
        &self.meta_json
    }

    pub fn type_ref(&self) -> PropertyTypeRef {
        self.config.type_ref()
    }

    pub fn raw_config(&self) -> errors::Result<Value> {
        match &self.config {
            ResolvedPropertyConfig::Known(config) => {
                BUILTIN_PROPERTY_TYPE_REGISTRY.encode_config(config)
            }
            ResolvedPropertyConfig::Opaque(config) => {
                Ok(config.raw_config.clone())
            }
        }
    }

    /// Project a definition into the legacy built-in model.
    ///
    /// Unknown definitions deliberately fail rather than being downgraded to
    /// `PropertyType::String`.
    pub fn to_property(&self) -> errors::Result<Property> {
        let config = match &self.config {
            ResolvedPropertyConfig::Known(config) => config,
            ResolvedPropertyConfig::Opaque(_) => {
                self.config.ensure_writable()?;
                return Err(errors::Error::invalid(
                    "opaque Property definitions are read-only",
                ));
            }
        };
        Ok(Property::with_meta_json(
            &self.id,
            &self.tenant_id,
            &self.database_id,
            &self.name,
            &PropertyType::from(config),
            self.is_indexed,
            self.property_num,
            self.meta_json.clone(),
        ))
    }

    pub fn update_known(
        &self,
        name: Option<&str>,
        property_type: Option<&PropertyType>,
        meta_json: Option<Option<String>>,
    ) -> errors::Result<Self> {
        self.config.ensure_writable()?;
        let property = self.to_property()?.update_with_meta_json(
            name,
            property_type,
            meta_json,
        )?;
        Ok(Self::from_property(&property))
    }
}

/// Scoped read port for canonical Property definitions.
#[async_trait::async_trait]
pub trait PropertyDefinitionRepository:
    Debug + Send + Sync + 'static
{
    async fn find_definition_by_id(
        &self,
        id: &PropertyId,
        database_id: &DatabaseId,
        tenant_id: &TenantId,
    ) -> errors::Result<Option<PropertyDefinition>>;

    async fn find_all_definitions(
        &self,
        database_id: &DatabaseId,
        tenant_id: &TenantId,
    ) -> errors::Result<Vec<PropertyDefinition>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opaque_definition_is_preserved_and_never_downgraded() {
        let raw_config = serde_json::json!({"future": [1, 2, 3]});
        let definition = PropertyDefinition::new(
            &PropertyId::default(),
            &TenantId::default(),
            &DatabaseId::default(),
            "future",
            ResolvedPropertyConfig::Opaque(OpaquePropertyConfig {
                type_ref: PropertyTypeRef::new(
                    PropertyTypeKey::new("future_type").expect("type key"),
                    PropertyTypeVersion::new(7).expect("type version"),
                ),
                raw_config: raw_config.clone(),
            }),
            false,
            0,
            None,
        );

        assert_eq!(
            definition.raw_config().expect("raw config"),
            raw_config
        );
        let error = definition
            .to_property()
            .expect_err("an unknown definition has no legacy projection");
        assert!(error.to_string().contains("read-only"));
    }
}
