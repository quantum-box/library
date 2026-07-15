use serde_json::Value;

use crate::PropertyDataValue;

use super::{
    BuiltinPropertyTypeHandler, OpaquePropertyConfig, OpaquePropertyValue,
    PROPERTY_TYPE_VERSION_V1, PropertyConfig, PropertyKind,
    PropertyTypeRef, PropertyValue, ResolvedPropertyConfig,
    ValueEncodingVersion,
};

/// A value whose type, encoding, and payload were validated by the built-in
/// registry.
///
/// Fields are intentionally private: callers can inspect a known value but
/// cannot forge an unsupported envelope and then mark it writable.
#[derive(Debug, Clone, PartialEq)]
pub struct KnownPropertyValue {
    type_ref: PropertyTypeRef,
    encoding_version: ValueEncodingVersion,
    value: PropertyDataValue,
}

impl KnownPropertyValue {
    fn from_validated_parts(
        type_ref: PropertyTypeRef,
        encoding_version: ValueEncodingVersion,
        value: PropertyDataValue,
    ) -> Self {
        Self {
            type_ref,
            encoding_version,
            value,
        }
    }

    pub fn type_ref(&self) -> &PropertyTypeRef {
        &self.type_ref
    }

    pub const fn encoding_version(&self) -> ValueEncodingVersion {
        self.encoding_version
    }

    pub fn value(&self) -> &PropertyDataValue {
        &self.value
    }

    pub fn into_value(self) -> PropertyDataValue {
        self.value
    }

    pub fn ensure_writable(&self) -> errors::Result<()> {
        let Some(handler) =
            BUILTIN_PROPERTY_TYPE_REGISTRY.resolve(&self.type_ref)
        else {
            return Err(super::unsupported_property_type(&self.type_ref));
        };
        if self.encoding_version != handler.value_encoding_version() {
            return Err(super::unsupported_property_value(
                &self.type_ref,
                self.encoding_version,
            ));
        }

        let value_type_ref =
            self.value.property_type().canonical_type_ref();
        if value_type_ref != self.type_ref {
            return Err(errors::Error::invalid(format!(
                "property value {value_type_ref} does not match envelope {}",
                self.type_ref
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
pub(super) fn forge_known_property_value_for_test(
    type_ref: PropertyTypeRef,
    encoding_version: ValueEncodingVersion,
    value: PropertyDataValue,
) -> KnownPropertyValue {
    KnownPropertyValue {
        type_ref,
        encoding_version,
        value,
    }
}

static BUILTIN_HANDLERS: [BuiltinPropertyTypeHandler; 11] = [
    BuiltinPropertyTypeHandler::new(PropertyKind::String),
    BuiltinPropertyTypeHandler::new(PropertyKind::Integer),
    BuiltinPropertyTypeHandler::new(PropertyKind::Html),
    BuiltinPropertyTypeHandler::new(PropertyKind::Markdown),
    BuiltinPropertyTypeHandler::new(PropertyKind::Relation),
    BuiltinPropertyTypeHandler::new(PropertyKind::Select),
    BuiltinPropertyTypeHandler::new(PropertyKind::MultiSelect),
    BuiltinPropertyTypeHandler::new(PropertyKind::Id),
    BuiltinPropertyTypeHandler::new(PropertyKind::Location),
    BuiltinPropertyTypeHandler::new(PropertyKind::Date),
    BuiltinPropertyTypeHandler::new(PropertyKind::Image),
];

/// The single compile-time registry for built-in Property Type handlers.
///
/// Runtime code loading is deliberately unsupported: introducing a type is a
/// source-controlled domain change with an explicit version and rollout plan.
#[derive(Debug, Clone, Copy)]
pub struct BuiltinPropertyTypeRegistry;

pub const BUILTIN_PROPERTY_TYPE_REGISTRY: BuiltinPropertyTypeRegistry =
    BuiltinPropertyTypeRegistry;

impl BuiltinPropertyTypeRegistry {
    pub fn handlers(&self) -> &'static [BuiltinPropertyTypeHandler] {
        &BUILTIN_HANDLERS
    }

    pub fn resolve(
        &self,
        type_ref: &PropertyTypeRef,
    ) -> Option<&'static BuiltinPropertyTypeHandler> {
        BUILTIN_HANDLERS.iter().find(|handler| {
            handler.kind().key() == type_ref.key.as_str()
                && type_ref.version == PROPERTY_TYPE_VERSION_V1
        })
    }

    pub fn decode_config(
        &self,
        type_ref: PropertyTypeRef,
        raw_config: Value,
    ) -> errors::Result<ResolvedPropertyConfig> {
        let Some(handler) = self.resolve(&type_ref) else {
            return Ok(ResolvedPropertyConfig::Opaque(
                OpaquePropertyConfig {
                    type_ref,
                    raw_config,
                },
            ));
        };

        Ok(ResolvedPropertyConfig::Known(
            handler.decode_config(raw_config)?,
        ))
    }

    pub fn encode_config(
        &self,
        config: &PropertyConfig,
    ) -> errors::Result<Value> {
        config.handler().encode_config(config)
    }

    pub fn decode_value(
        &self,
        type_ref: PropertyTypeRef,
        encoding_version: ValueEncodingVersion,
        config: &ResolvedPropertyConfig,
        raw_value: Value,
    ) -> errors::Result<PropertyValue> {
        let Some(handler) = self.resolve(&type_ref) else {
            return Ok(PropertyValue::Opaque(OpaquePropertyValue {
                type_ref,
                encoding_version,
                raw_value,
            }));
        };

        if encoding_version != handler.value_encoding_version() {
            return Ok(PropertyValue::Opaque(OpaquePropertyValue {
                type_ref,
                encoding_version,
                raw_value,
            }));
        }

        let ResolvedPropertyConfig::Known(config) = config else {
            return Ok(PropertyValue::Opaque(OpaquePropertyValue {
                type_ref,
                encoding_version,
                raw_value,
            }));
        };
        if config.type_ref() != type_ref {
            return Err(errors::Error::invalid(format!(
                "property config {} does not match value envelope {type_ref}",
                config.type_ref()
            )));
        }

        let value = handler.decode_value(config, raw_value)?;
        Ok(PropertyValue::Known(
            KnownPropertyValue::from_validated_parts(
                type_ref,
                encoding_version,
                value,
            ),
        ))
    }

    pub fn normalize_value(
        &self,
        config: &PropertyConfig,
        value: &PropertyDataValue,
    ) -> errors::Result<KnownPropertyValue> {
        let handler = config.handler();
        handler.validate_value(config, value)?;
        Ok(KnownPropertyValue::from_validated_parts(
            config.type_ref(),
            handler.value_encoding_version(),
            value.clone(),
        ))
    }

    pub fn encode_value(
        &self,
        config: &PropertyConfig,
        value: &PropertyDataValue,
    ) -> errors::Result<Value> {
        config.handler().encode_value(config, value)
    }
}
