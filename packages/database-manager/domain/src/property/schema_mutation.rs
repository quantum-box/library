use super::*;

/// Intent to append a property to a database schema.
///
/// The command is persistence-agnostic. Adapters are responsible for loading
/// the current schema under the serialization boundary before asking the
/// domain to plan the mutation.
#[derive(Clone, Debug)]
pub struct AddPropertyCommand {
    tenant_id: TenantId,
    database_id: DatabaseId,
    name: String,
    property_type: PropertyType,
}

/// Intent to update a Property definition. The adapter loads and locks the
/// current definition before the domain applies this command.
#[derive(Clone, Debug)]
pub struct UpdatePropertyCommand {
    tenant_id: TenantId,
    database_id: DatabaseId,
    property_id: PropertyId,
    name: Option<String>,
    property_type: Option<PropertyType>,
    meta_json: Option<Option<String>>,
}

impl UpdatePropertyCommand {
    pub fn new(
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        property_id: &PropertyId,
        name: Option<&str>,
        property_type: Option<&PropertyType>,
        meta_json: Option<Option<String>>,
    ) -> Self {
        Self {
            tenant_id: tenant_id.clone(),
            database_id: database_id.clone(),
            property_id: property_id.clone(),
            name: name.map(str::to_string),
            property_type: property_type.cloned(),
            meta_json,
        }
    }

    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }
    pub fn database_id(&self) -> &DatabaseId {
        &self.database_id
    }
    pub fn property_id(&self) -> &PropertyId {
        &self.property_id
    }
    pub fn property_type(&self) -> Option<&PropertyType> {
        self.property_type.as_ref()
    }

    pub fn apply(
        &self,
        current: &PropertyDefinition,
    ) -> errors::Result<PropertyDefinition> {
        if current.tenant_id() != &self.tenant_id
            || current.database_id() != &self.database_id
            || current.id() != &self.property_id
        {
            return Err(errors::Error::not_found("resource not found"));
        }
        current.update_known(
            self.name.as_deref(),
            self.property_type.as_ref(),
            self.meta_json.clone(),
        )
    }
}

impl AddPropertyCommand {
    pub fn new(
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        name: &str,
        property_type: &PropertyType,
    ) -> Self {
        Self {
            tenant_id: tenant_id.clone(),
            database_id: database_id.clone(),
            name: name.to_string(),
            property_type: property_type.clone(),
        }
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

    pub fn property_type(&self) -> &PropertyType {
        &self.property_type
    }
}

/// A domain-approved schema mutation that must be persisted as one unit.
#[derive(Debug)]
pub struct PropertySchemaMutation {
    definition: PropertyDefinition,
    relation_definition: Option<RelationDefinition>,
}

impl PropertySchemaMutation {
    pub fn into_parts(
        self,
    ) -> (PropertyDefinition, Option<RelationDefinition>) {
        (self.definition, self.relation_definition)
    }
}

/// Domain service for property-schema invariants and mutation planning.
///
/// It deliberately receives the current schema rather than loading it. This
/// keeps the invariant in the domain while allowing an adapter to re-evaluate
/// it after acquiring its database-specific serialization lock.
#[derive(Debug)]
pub struct PropertySchema;

impl PropertySchema {
    pub fn plan_addition(
        existing_definitions: &[PropertyDefinition],
        command: &AddPropertyCommand,
    ) -> errors::Result<PropertySchemaMutation> {
        validate_property_definition_addition(
            existing_definitions,
            command.property_type(),
        )?;
        let property_num =
            next_property_definition_num(existing_definitions)?;
        let property = Property::new(
            &PropertyId::default(),
            command.tenant_id(),
            command.database_id(),
            command.name(),
            command.property_type(),
            false,
            property_num,
        );
        let relation_definition = match property.property_type() {
            PropertyType::Relation(relation_type) => {
                Some(RelationDefinition::legacy_default(
                    &RelationId::default(),
                    command.tenant_id(),
                    command.database_id(),
                    property.id(),
                    &relation_type.database_id,
                ))
            }
            _ => None,
        };

        Ok(PropertySchemaMutation {
            definition: PropertyDefinition::from_property(&property),
            relation_definition,
        })
    }
}

/// Output port for the serialized property-schema unit of work.
#[async_trait::async_trait]
pub trait PropertySchemaMutationPort:
    Debug + Send + Sync + 'static
{
    async fn add_property_atomically(
        &self,
        command: &AddPropertyCommand,
    ) -> errors::Result<Property>;

    async fn update_property_atomically(
        &self,
        command: &UpdatePropertyCommand,
    ) -> errors::Result<Property>;

    async fn delete_property_atomically(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        property_id: &PropertyId,
    ) -> errors::Result<Property>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn property(
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        property_type: PropertyType,
        property_num: u32,
    ) -> PropertyDefinition {
        PropertyDefinition::from_property(&Property::new(
            &PropertyId::default(),
            tenant_id,
            database_id,
            "existing",
            &property_type,
            false,
            property_num,
        ))
    }

    #[test]
    fn plans_the_first_free_legacy_slot() {
        let tenant_id = TenantId::default();
        let database_id = DatabaseId::default();
        let existing = vec![
            property(&tenant_id, &database_id, PropertyType::String, 0),
            property(&tenant_id, &database_id, PropertyType::String, 2),
        ];
        let command = AddPropertyCommand::new(
            &tenant_id,
            &database_id,
            "new",
            &PropertyType::String,
        );

        let (planned, relation_definition) =
            PropertySchema::plan_addition(&existing, &command)
                .expect("schema mutation")
                .into_parts();

        assert_eq!(planned.property_num(), 1);
        assert!(relation_definition.is_none());
    }

    #[test]
    fn relation_metadata_is_part_of_the_same_domain_mutation() {
        let tenant_id = TenantId::default();
        let database_id = DatabaseId::default();
        let target_database_id = DatabaseId::default();
        let command = AddPropertyCommand::new(
            &tenant_id,
            &database_id,
            "relation",
            &PropertyType::Relation(TypeRelation::new(
                target_database_id.clone(),
            )),
        );

        let (property, relation_definition) =
            PropertySchema::plan_addition(&[], &command)
                .expect("schema mutation")
                .into_parts();
        let relation_definition =
            relation_definition.expect("Relation definition");

        assert_eq!(relation_definition.source_property_id(), property.id());
        assert_eq!(
            relation_definition.target_database_id(),
            &target_database_id
        );
        assert_eq!(
            *relation_definition.forward_cardinality(),
            RelationCardinality::Many
        );
        assert_eq!(
            *relation_definition.reverse_cardinality(),
            RelationCardinality::Many
        );
        assert_eq!(
            *relation_definition.on_target_delete(),
            RelationOnDelete::Restrict
        );
    }

    #[test]
    fn opaque_sibling_does_not_block_a_known_property_addition() {
        let tenant_id = TenantId::default();
        let database_id = DatabaseId::default();
        let opaque = PropertyDefinition::new(
            &PropertyId::default(),
            &tenant_id,
            &database_id,
            "future",
            ResolvedPropertyConfig::Opaque(OpaquePropertyConfig {
                type_ref: PropertyTypeRef::new(
                    PropertyTypeKey::new("future_type").expect("type key"),
                    PropertyTypeVersion::new(7).expect("type version"),
                ),
                raw_config: serde_json::json!({"future": true}),
            }),
            false,
            0,
            None,
        );
        let command = AddPropertyCommand::new(
            &tenant_id,
            &database_id,
            "known",
            &PropertyType::String,
        );

        let (definition, _) =
            PropertySchema::plan_addition(&[opaque], &command)
                .expect("opaque siblings are retained but not mutated")
                .into_parts();

        assert_eq!(definition.property_num(), 1);
    }
}
