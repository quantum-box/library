use std::fmt::Debug;

use value_object::TenantId;

use crate::{
    DatabaseId, Property, PropertyDefinition, PropertyId, PropertyType,
    RelationCardinality, RelationDefinition, RelationGeneration,
    RelationOnDelete, ResolvedPropertyConfig, TypeRelation,
    next_property_definition_num,
};

/// Requested change to the generated inverse Property.
///
/// `SetAlias` creates a fresh owned inverse when one does not exist and
/// renames the existing owned inverse otherwise. `Remove` deletes only an
/// inverse owned by this RelationDefinition; a compatibility inverse is
/// detached but never deleted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum RelationInverseChange {
    #[default]
    Keep,
    SetAlias(String),
    Remove,
}

/// Versioned command for changing one RelationDefinition aggregate.
#[derive(Clone, Debug)]
pub struct ReconfigureRelationDefinitionCommand {
    tenant_id: TenantId,
    source_database_id: DatabaseId,
    source_property_id: PropertyId,
    expected_generation: RelationGeneration,
    forward_cardinality: Option<RelationCardinality>,
    reverse_cardinality: Option<RelationCardinality>,
    inverse: RelationInverseChange,
    on_target_delete: Option<RelationOnDelete>,
}

impl ReconfigureRelationDefinitionCommand {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: &TenantId,
        source_database_id: &DatabaseId,
        source_property_id: &PropertyId,
        expected_generation: RelationGeneration,
        forward_cardinality: Option<RelationCardinality>,
        reverse_cardinality: Option<RelationCardinality>,
        inverse: RelationInverseChange,
        on_target_delete: Option<RelationOnDelete>,
    ) -> Self {
        Self {
            tenant_id: tenant_id.clone(),
            source_database_id: source_database_id.clone(),
            source_property_id: source_property_id.clone(),
            expected_generation,
            forward_cardinality,
            reverse_cardinality,
            inverse,
            on_target_delete,
        }
    }

    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    pub fn source_database_id(&self) -> &DatabaseId {
        &self.source_database_id
    }

    pub fn source_property_id(&self) -> &PropertyId {
        &self.source_property_id
    }

    pub const fn expected_generation(&self) -> RelationGeneration {
        self.expected_generation
    }
}

/// Versioned command for deleting the source Relation Property, its
/// RelationDefinition, and any generated inverse Property as one unit.
#[derive(Clone, Debug)]
pub struct DeleteRelationDefinitionCommand {
    tenant_id: TenantId,
    source_database_id: DatabaseId,
    source_property_id: PropertyId,
    expected_generation: RelationGeneration,
}

impl DeleteRelationDefinitionCommand {
    pub fn new(
        tenant_id: &TenantId,
        source_database_id: &DatabaseId,
        source_property_id: &PropertyId,
        expected_generation: RelationGeneration,
    ) -> Self {
        Self {
            tenant_id: tenant_id.clone(),
            source_database_id: source_database_id.clone(),
            source_property_id: source_property_id.clone(),
            expected_generation,
        }
    }

    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    pub fn source_database_id(&self) -> &DatabaseId {
        &self.source_database_id
    }

    pub fn source_property_id(&self) -> &PropertyId {
        &self.source_property_id
    }

    pub const fn expected_generation(&self) -> RelationGeneration {
        self.expected_generation
    }
}

/// Exact inverse-Property write planned by the domain.
#[derive(Debug)]
pub enum InversePropertyMutation {
    None,
    Insert(PropertyDefinition),
    Replace(PropertyDefinition),
    Delete(PropertyDefinition),
}

/// Domain-approved RelationDefinition mutation.
#[derive(Debug)]
pub struct RelationSchemaMutation {
    definition: RelationDefinition,
    inverse_property: InversePropertyMutation,
}

impl RelationSchemaMutation {
    pub fn into_parts(
        self,
    ) -> (RelationDefinition, InversePropertyMutation) {
        (self.definition, self.inverse_property)
    }
}

/// Domain-approved deletion of one Relation schema aggregate.
#[derive(Debug)]
pub struct RelationSchemaDeletion {
    definition: RelationDefinition,
    source_property: PropertyDefinition,
    owned_inverse_property: Option<PropertyDefinition>,
}

impl RelationSchemaDeletion {
    pub fn into_parts(
        self,
    ) -> (
        RelationDefinition,
        PropertyDefinition,
        Option<PropertyDefinition>,
    ) {
        (
            self.definition,
            self.source_property,
            self.owned_inverse_property,
        )
    }
}

/// Relation-schema policies evaluated after persistence has acquired all
/// endpoint locks and loaded the current Property schemas.
#[derive(Debug)]
pub struct RelationSchema;

impl RelationSchema {
    pub fn plan_reconfiguration(
        current: &RelationDefinition,
        source_property: &PropertyDefinition,
        current_inverse: Option<&PropertyDefinition>,
        target_properties: &[PropertyDefinition],
        command: &ReconfigureRelationDefinitionCommand,
    ) -> errors::Result<RelationSchemaMutation> {
        Self::validate_scope(
            current,
            source_property,
            command.tenant_id(),
            command.source_database_id(),
            command.source_property_id(),
            command.expected_generation(),
        )?;
        Self::validate_current_inverse(current, current_inverse)?;

        let (inverse_property_id, inverse_property_owned, inverse_property) =
            match &command.inverse {
                RelationInverseChange::Keep => (
                    current.inverse_property_id().clone(),
                    *current.inverse_property_owned(),
                    InversePropertyMutation::None,
                ),
                RelationInverseChange::SetAlias(alias) => {
                    let alias = Self::validate_alias(
                        alias,
                        current.inverse_property_id(),
                        target_properties,
                    )?;
                    match current_inverse {
                        Some(inverse) => {
                            if !current.inverse_property_owned() {
                                if inverse.name() == alias {
                                    (
                                        Some(inverse.id().clone()),
                                        false,
                                        InversePropertyMutation::None,
                                    )
                                } else {
                                    return Err(errors::Error::conflict(
                                        "a compatibility inverse Property cannot be renamed by this RelationDefinition",
                                    ));
                                }
                            } else if inverse.name() == alias {
                                (
                                    Some(inverse.id().clone()),
                                    true,
                                    InversePropertyMutation::None,
                                )
                            } else {
                                let updated = inverse.update_known(
                                    Some(alias),
                                    None,
                                    None,
                                )?;
                                (
                                    Some(updated.id().clone()),
                                    true,
                                    InversePropertyMutation::Replace(
                                        updated,
                                    ),
                                )
                            }
                        }
                        None => {
                            let inverse = Self::new_inverse_property(
                                current,
                                alias,
                                target_properties,
                            )?;
                            (
                                Some(inverse.id().clone()),
                                true,
                                InversePropertyMutation::Insert(inverse),
                            )
                        }
                    }
                }
                RelationInverseChange::Remove => match current_inverse {
                    Some(inverse) if *current.inverse_property_owned() => (
                        None,
                        false,
                        InversePropertyMutation::Delete(inverse.clone()),
                    ),
                    Some(_) | None => {
                        (None, false, InversePropertyMutation::None)
                    }
                },
            };

        let definition = current.reconfigure(
            command
                .forward_cardinality
                .unwrap_or(*current.forward_cardinality()),
            command
                .reverse_cardinality
                .unwrap_or(*current.reverse_cardinality()),
            inverse_property_id.as_ref(),
            inverse_property_owned,
            command
                .on_target_delete
                .unwrap_or(*current.on_target_delete()),
        )?;

        Ok(RelationSchemaMutation {
            definition,
            inverse_property,
        })
    }

    pub fn plan_deletion(
        current: &RelationDefinition,
        source_property: &PropertyDefinition,
        current_inverse: Option<&PropertyDefinition>,
        command: &DeleteRelationDefinitionCommand,
    ) -> errors::Result<RelationSchemaDeletion> {
        Self::validate_scope(
            current,
            source_property,
            command.tenant_id(),
            command.source_database_id(),
            command.source_property_id(),
            command.expected_generation(),
        )?;
        Self::validate_current_inverse(current, current_inverse)?;
        Ok(RelationSchemaDeletion {
            definition: current.clone(),
            source_property: source_property.clone(),
            owned_inverse_property: if *current.inverse_property_owned() {
                current_inverse.cloned()
            } else {
                None
            },
        })
    }

    fn validate_scope(
        current: &RelationDefinition,
        source_property: &PropertyDefinition,
        tenant_id: &TenantId,
        source_database_id: &DatabaseId,
        source_property_id: &PropertyId,
        expected_generation: RelationGeneration,
    ) -> errors::Result<()> {
        if current.tenant_id() != tenant_id
            || current.source_database_id() != source_database_id
            || current.source_property_id() != source_property_id
            || source_property.tenant_id() != tenant_id
            || source_property.database_id() != source_database_id
            || source_property.id() != source_property_id
        {
            return Err(errors::Error::not_found("resource not found"));
        }
        current.ensure_writable()?;
        if current.generation() != &expected_generation {
            return Err(errors::Error::conflict(
                "RelationDefinition generation does not match",
            ));
        }
        match source_property.config() {
            ResolvedPropertyConfig::Known(config) => {
                let property_type = PropertyType::from(config);
                let PropertyType::Relation(relation) = property_type else {
                    return Err(errors::Error::invalid(
                        "RelationDefinition source must be a Relation Property",
                    ));
                };
                if relation.database_id != *current.target_database_id() {
                    return Err(errors::Error::conflict(
                        "RelationDefinition target does not match its source Property",
                    ));
                }
            }
            ResolvedPropertyConfig::Opaque(_) => {
                source_property.config().ensure_writable()?;
            }
        }
        Ok(())
    }

    fn validate_current_inverse(
        current: &RelationDefinition,
        current_inverse: Option<&PropertyDefinition>,
    ) -> errors::Result<()> {
        match (current.inverse_property_id(), current_inverse) {
            (None, None) => Ok(()),
            (Some(expected), Some(inverse))
                if inverse.id() == expected
                    && inverse.tenant_id() == current.tenant_id()
                    && inverse.database_id()
                        == current.target_database_id() =>
            {
                if *current.inverse_property_owned() {
                    Self::validate_owned_inverse(current, inverse)?;
                }
                Ok(())
            }
            _ => Err(errors::Error::conflict(
                "RelationDefinition inverse Property does not match storage",
            )),
        }
    }

    fn validate_owned_inverse(
        current: &RelationDefinition,
        inverse: &PropertyDefinition,
    ) -> errors::Result<()> {
        inverse.config().ensure_writable()?;
        let property = inverse.to_property()?;
        let PropertyType::Relation(relation) = property.property_type()
        else {
            return Err(errors::Error::conflict(
                "an owned inverse must remain a Relation Property",
            ));
        };
        if relation.database_id != *current.source_database_id() {
            return Err(errors::Error::conflict(
                "an owned inverse must target the source Database",
            ));
        }
        if current.source_database_id() == current.target_database_id()
            && inverse.id() == current.source_property_id()
        {
            return Err(errors::Error::conflict(
                "a self Relation inverse must be a distinct Property",
            ));
        }
        Ok(())
    }

    fn validate_alias<'a>(
        alias: &'a str,
        current_inverse_id: &Option<PropertyId>,
        target_properties: &[PropertyDefinition],
    ) -> errors::Result<&'a str> {
        let alias = alias.trim();
        if alias.is_empty() {
            return Err(errors::Error::invalid(
                "inverse Property alias must not be empty",
            ));
        }
        if target_properties.iter().any(|property| {
            property.name() == alias
                && Some(property.id()) != current_inverse_id.as_ref()
        }) {
            return Err(errors::Error::conflict(
                "inverse Property alias already exists in the target Database",
            ));
        }
        Ok(alias)
    }

    fn new_inverse_property(
        current: &RelationDefinition,
        alias: &str,
        target_properties: &[PropertyDefinition],
    ) -> errors::Result<PropertyDefinition> {
        let property_num = next_property_definition_num(target_properties)?;
        let property = Property::new(
            &PropertyId::default(),
            current.tenant_id(),
            current.target_database_id(),
            alias,
            &PropertyType::Relation(TypeRelation::new(
                current.source_database_id().clone(),
            )),
            false,
            property_num,
        );
        if current.source_database_id() == current.target_database_id()
            && property.id() == current.source_property_id()
        {
            return Err(errors::Error::conflict(
                "a self Relation inverse must be a distinct Property",
            ));
        }
        Ok(PropertyDefinition::from_property(&property))
    }
}

/// Output port for endpoint-serialized Relation schema mutations.
#[async_trait::async_trait]
pub trait RelationSchemaMutationPort:
    Debug + Send + Sync + 'static
{
    async fn reconfigure_relation_atomically(
        &self,
        command: &ReconfigureRelationDefinitionCommand,
    ) -> errors::Result<RelationDefinition>;

    async fn delete_relation_atomically(
        &self,
        command: &DeleteRelationDefinitionCommand,
    ) -> errors::Result<RelationDefinition>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        RELATION_DEFINITION_VERSION_V1, RELATION_GENERATION_V1,
        RelationDefinitionVersion, RelationId,
    };

    fn relation_fixture(
        self_relation: bool,
    ) -> (
        RelationDefinition,
        PropertyDefinition,
        Vec<PropertyDefinition>,
    ) {
        let tenant_id = TenantId::default();
        let source_database_id = DatabaseId::default();
        let target_database_id = if self_relation {
            source_database_id.clone()
        } else {
            DatabaseId::default()
        };
        let source_property =
            PropertyDefinition::from_property(&Property::new(
                &PropertyId::default(),
                &tenant_id,
                &source_database_id,
                "parent",
                &PropertyType::Relation(TypeRelation::new(
                    target_database_id.clone(),
                )),
                false,
                0,
            ));
        let relation = RelationDefinition::legacy_default(
            &RelationId::default(),
            &tenant_id,
            &source_database_id,
            source_property.id(),
            &target_database_id,
        );
        let target_properties = if self_relation {
            vec![source_property.clone()]
        } else {
            Vec::new()
        };
        (relation, source_property, target_properties)
    }

    #[test]
    fn creates_a_fresh_owned_inverse_for_a_self_relation() {
        let (relation, source, target_properties) = relation_fixture(true);
        let command = ReconfigureRelationDefinitionCommand::new(
            relation.tenant_id(),
            relation.source_database_id(),
            relation.source_property_id(),
            RELATION_GENERATION_V1,
            Some(RelationCardinality::One),
            Some(RelationCardinality::Many),
            RelationInverseChange::SetAlias("children".to_string()),
            Some(RelationOnDelete::Nullify),
        );

        let (updated, inverse) = RelationSchema::plan_reconfiguration(
            &relation,
            &source,
            None,
            &target_properties,
            &command,
        )
        .expect("valid schema mutation")
        .into_parts();
        let InversePropertyMutation::Insert(inverse) = inverse else {
            panic!("a fresh inverse must be inserted");
        };

        assert_ne!(inverse.id(), source.id());
        assert_eq!(inverse.name(), "children");
        assert_eq!(
            updated.inverse_property_id(),
            &Some(inverse.id().clone())
        );
        assert!(*updated.inverse_property_owned());
        assert_eq!(updated.generation().get(), 2);
    }

    #[test]
    fn stale_generation_and_duplicate_alias_fail_closed() {
        let (relation, source, mut target_properties) =
            relation_fixture(false);
        target_properties.push(PropertyDefinition::from_property(
            &Property::new(
                &PropertyId::default(),
                relation.tenant_id(),
                relation.target_database_id(),
                "children",
                &PropertyType::String,
                false,
                0,
            ),
        ));
        let stale = ReconfigureRelationDefinitionCommand::new(
            relation.tenant_id(),
            relation.source_database_id(),
            relation.source_property_id(),
            RelationGeneration::new(2).expect("generation"),
            None,
            None,
            RelationInverseChange::Keep,
            None,
        );
        assert!(matches!(
            RelationSchema::plan_reconfiguration(
                &relation,
                &source,
                None,
                &target_properties,
                &stale,
            ),
            Err(errors::Error::Conflict { .. })
        ));

        let duplicate = ReconfigureRelationDefinitionCommand::new(
            relation.tenant_id(),
            relation.source_database_id(),
            relation.source_property_id(),
            RELATION_GENERATION_V1,
            None,
            None,
            RelationInverseChange::SetAlias("children".to_string()),
            None,
        );
        assert!(matches!(
            RelationSchema::plan_reconfiguration(
                &relation,
                &source,
                None,
                &target_properties,
                &duplicate,
            ),
            Err(errors::Error::Conflict { .. })
        ));
    }

    #[test]
    fn future_definition_versions_are_read_only() {
        let (relation, source, target_properties) = relation_fixture(false);
        let future = RelationDefinition::restore(
            relation.id(),
            relation.tenant_id(),
            relation.source_database_id(),
            relation.source_property_id(),
            relation.target_database_id(),
            *relation.forward_cardinality(),
            *relation.reverse_cardinality(),
            None,
            false,
            *relation.on_target_delete(),
            RelationDefinitionVersion::new(
                RELATION_DEFINITION_VERSION_V1.get() + 1,
            )
            .expect("future version"),
            RELATION_GENERATION_V1,
        )
        .expect("future definitions remain readable");
        let command = ReconfigureRelationDefinitionCommand::new(
            future.tenant_id(),
            future.source_database_id(),
            future.source_property_id(),
            RELATION_GENERATION_V1,
            None,
            None,
            RelationInverseChange::Keep,
            None,
        );

        let error = RelationSchema::plan_reconfiguration(
            &future,
            &source,
            None,
            &target_properties,
            &command,
        )
        .expect_err("a future version must not be overwritten");
        assert!(error.to_string().contains("read-only"));
    }
}
