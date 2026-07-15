//! Property
//!
//! TODO: add English documentation

use super::*;
use std::fmt::Debug;
use util::macros::*;

mod property_type;
pub use property_type::*;

pub const ID_PROPERTY_ALREADY_EXISTS: &str = "Id property already exists";
pub const RELATION_TARGET_DATABASE_IMMUTABLE: &str =
    "Relation target database is immutable after property creation";
pub const MAX_PROPERTY_NUM: u32 = 50;

pub fn validate_property_type_addition(
    existing_properties: &[Property],
    new_property_type: &PropertyType,
) -> errors::Result<()> {
    if matches!(new_property_type, PropertyType::Id(_))
        && existing_properties.iter().any(|property| {
            matches!(property.property_type(), PropertyType::Id(_))
        })
    {
        return Err(errors::Error::conflict(ID_PROPERTY_ALREADY_EXISTS));
    }

    Ok(())
}

pub fn next_property_num(
    existing_properties: &[Property],
) -> errors::Result<u32> {
    (0..=MAX_PROPERTY_NUM)
        .find(|candidate| {
            existing_properties
                .iter()
                .all(|property| property.property_num() != candidate)
        })
        .ok_or_else(|| {
            errors::Error::business_logic(format!(
                "Property limit reached: {} slots are available",
                MAX_PROPERTY_NUM + 1
            ))
        })
}

def_id!(PropertyId, "prop_");

#[derive(Getters, Debug, Clone)]
pub struct Property {
    id: PropertyId,
    tenant_id: TenantId,
    database_id: DatabaseId,
    name: String,
    property_type: PropertyType,
    is_indexed: bool,
    property_num: u32,
    /// JSON metadata for property configuration (e.g., ext_github repos)
    meta_json: Option<String>,
}

impl Property {
    pub fn new(
        id: &PropertyId,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        name: &str,
        property_type: &PropertyType,
        is_indexed: bool,
        property_num: u32,
    ) -> Self {
        Self::with_meta_json(
            id,
            tenant_id,
            database_id,
            name,
            property_type,
            is_indexed,
            property_num,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_meta_json(
        id: &PropertyId,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        name: &str,
        property_type: &PropertyType,
        is_indexed: bool,
        property_num: u32,
        meta_json: Option<String>,
    ) -> Self {
        let name: String = if name.is_empty() {
            format!("property{property_num}")
        } else {
            name.into()
        };
        Self {
            id: id.clone(),
            tenant_id: tenant_id.clone(),
            database_id: database_id.clone(),
            name: name.to_string(),
            property_type: property_type.clone(),
            is_indexed,
            property_num,
            meta_json,
        }
    }

    pub fn update(
        &self,
        name: Option<&str>,
        property_type: Option<&PropertyType>,
    ) -> errors::Result<Self> {
        self.update_with_meta_json(name, property_type, None)
    }

    pub fn update_with_meta_json(
        &self,
        name: Option<&str>,
        property_type: Option<&PropertyType>,
        meta_json: Option<Option<String>>,
    ) -> errors::Result<Self> {
        let property = self.update_property_type(property_type)?;

        Ok(Self {
            name: name
                .map(|s| s.to_string())
                .unwrap_or(property.name.clone()),
            meta_json: meta_json
                .unwrap_or_else(|| property.meta_json.clone()),
            ..property
        })
    }

    fn update_property_type(
        &self,
        property_type: Option<&PropertyType>,
    ) -> errors::Result<Self> {
        if let Some(property_type) = property_type {
            if let (
                PropertyType::Relation(current),
                PropertyType::Relation(requested),
            ) = (&self.property_type, property_type)
                && current.database_id != requested.database_id
            {
                return Err(errors::invalid!(
                    "{}",
                    RELATION_TARGET_DATABASE_IMMUTABLE
                ));
            }
            if let (PropertyType::Id(current), PropertyType::Id(requested)) =
                (&self.property_type, property_type)
                && current.auto_generate != requested.auto_generate
            {
                return Err(errors::invalid!(
                    "Id auto_generate is immutable after property creation."
                ));
            }
            if self.property_type.to_string() != property_type.to_string() {
                // TODO: add English comment
                // TODO: add English comment
                return Err(errors::invalid!(
                    "Property type is not match. Cannot change property type currently."
                ));
            }
            return Ok(Self {
                property_type: property_type.clone(),
                ..self.clone()
            });
        }
        Ok(self.clone())
    }
}

#[async_trait::async_trait]
pub trait PropertyRepository: Debug + Send + Sync + 'static {
    async fn create(&self, property: &Property) -> errors::Result<()>;
    async fn update(&self, property: &Property) -> errors::Result<()>;
    async fn find_by_id(
        &self,
        id: &PropertyId,
        database_id: &DatabaseId,
        tenant_id: &TenantId,
    ) -> errors::Result<Option<Property>>;
    async fn find_all(
        &self,
        database_id: &DatabaseId,
        tenant_id: &TenantId,
    ) -> errors::Result<Vec<Property>>;
    async fn delete(
        &self,
        tenant_id: &TenantId,
        id: &PropertyId,
    ) -> errors::Result<()>;
    async fn delete_all(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
    ) -> errors::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn property(
        property_type: PropertyType,
        property_num: u32,
    ) -> Property {
        Property::new(
            &PropertyId::default(),
            &TenantId::default(),
            &DatabaseId::default(),
            "property",
            &property_type,
            false,
            property_num,
        )
    }

    #[test]
    fn string_property_can_be_added_after_id_property() {
        let existing =
            vec![property(PropertyType::Id(TypeId::new(true)), 0)];

        validate_property_type_addition(&existing, &PropertyType::String)
            .expect("a non-Id property must remain addable");
    }

    #[test]
    fn id_property_can_be_added_after_string_property() {
        let existing = vec![property(PropertyType::String, 0)];

        validate_property_type_addition(
            &existing,
            &PropertyType::Id(TypeId::new(true)),
        )
        .expect("the first Id property must be addable");
    }

    #[test]
    fn second_id_property_is_rejected_by_the_domain() {
        let existing =
            vec![property(PropertyType::Id(TypeId::new(true)), 0)];

        let error = validate_property_type_addition(
            &existing,
            &PropertyType::Id(TypeId::new(false)),
        )
        .expect_err("a second Id property must be rejected");

        assert!(matches!(error, errors::Error::Conflict { .. }));
        assert!(error.to_string().contains(ID_PROPERTY_ALREADY_EXISTS));
    }

    #[test]
    fn deleted_property_slot_is_reused_without_colliding() {
        let existing = vec![
            property(PropertyType::String, 0),
            property(PropertyType::String, 2),
        ];

        assert_eq!(
            next_property_num(&existing).expect("a slot must be available"),
            1
        );
    }

    #[test]
    fn property_limit_is_an_explicit_domain_error() {
        let existing = (0..=MAX_PROPERTY_NUM)
            .map(|property_num| {
                property(PropertyType::String, property_num)
            })
            .collect::<Vec<_>>();

        let error = next_property_num(&existing)
            .expect_err("all legacy storage slots are occupied");

        assert!(error.is_bad_request());
        assert!(error.to_string().contains("Property limit reached"));
    }

    #[test]
    fn id_auto_generate_policy_is_creation_only() {
        let property = property(PropertyType::Id(TypeId::new(true)), 0);

        let error = property
            .update(None, Some(&PropertyType::Id(TypeId::new(false))))
            .expect_err("auto_generate must not change after creation");

        assert!(error.is_bad_request());
        assert!(error.to_string().contains("auto_generate is immutable"));
    }

    #[test]
    fn relation_target_database_is_immutable() {
        let original_target = DatabaseId::default();
        let requested_target = DatabaseId::default();
        let property = property(
            PropertyType::Relation(TypeRelation::new(original_target)),
            0,
        );

        let error = property
            .update(
                None,
                Some(&PropertyType::Relation(TypeRelation::new(
                    requested_target,
                ))),
            )
            .expect_err("a Relation target must not change after creation");

        assert!(error.is_bad_request());
        assert!(
            error
                .to_string()
                .contains(RELATION_TARGET_DATABASE_IMMUTABLE)
        );
    }

    #[test]
    fn the_existing_relation_target_database_is_accepted() {
        let target = DatabaseId::default();
        let property = property(
            PropertyType::Relation(TypeRelation::new(target.clone())),
            0,
        );

        property
            .update(
                None,
                Some(&PropertyType::Relation(TypeRelation::new(target))),
            )
            .expect("repeating the configured Relation target is a no-op");
    }
}
