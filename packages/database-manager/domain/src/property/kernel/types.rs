use crate::{
    DataId, DatabaseId, SelectItemId, TypeId, TypeLocation,
    TypeMultiSelect, TypeRelation, TypeSelect,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{BuiltinPropertyTypeHandler, KnownPropertyValue};

pub const PROPERTY_TYPE_VERSION_V1: PropertyTypeVersion =
    PropertyTypeVersion::new_unchecked(1);
pub const VALUE_ENCODING_VERSION_V1: ValueEncodingVersion =
    ValueEncodingVersion::new_unchecked(1);
pub const MAX_PROPERTY_TYPE_KEY_BYTES: usize = 64;

/// A stable, lower-snake-case Property Type key.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct PropertyTypeKey(String);

impl PropertyTypeKey {
    pub fn new(value: impl Into<String>) -> errors::Result<Self> {
        let value = value.into();
        let mut chars = value.chars();
        let valid_first = chars
            .next()
            .is_some_and(|character| character.is_ascii_lowercase());
        let valid_rest = chars.all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || character == '_'
        });
        let valid_segments =
            value.split('_').all(|segment| !segment.is_empty());
        if !valid_first || !valid_rest || !valid_segments {
            return Err(errors::Error::invalid(
                "property type key must be lower_snake_case",
            ));
        }
        if value.len() > MAX_PROPERTY_TYPE_KEY_BYTES {
            return Err(errors::Error::invalid(format!(
                "property type key must not exceed {MAX_PROPERTY_TYPE_KEY_BYTES} bytes"
            )));
        }
        Ok(Self(value))
    }

    pub(crate) fn builtin(value: &'static str) -> Self {
        Self(value.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for PropertyTypeKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for PropertyTypeKey {
    fn fmt(
        &self,
        formatter: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize,
)]
#[serde(transparent)]
pub struct PropertyTypeVersion(u16);

impl PropertyTypeVersion {
    pub fn new(value: u16) -> errors::Result<Self> {
        if value == 0 {
            return Err(errors::Error::invalid(
                "property type version must be greater than zero",
            ));
        }
        Ok(Self(value))
    }

    pub(crate) const fn new_unchecked(value: u16) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u16 {
        self.0
    }
}

impl<'de> Deserialize<'de> for PropertyTypeVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u16::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize,
)]
#[serde(transparent)]
pub struct ValueEncodingVersion(u16);

impl ValueEncodingVersion {
    pub fn new(value: u16) -> errors::Result<Self> {
        if value == 0 {
            return Err(errors::Error::invalid(
                "value encoding version must be greater than zero",
            ));
        }
        Ok(Self(value))
    }

    pub(crate) const fn new_unchecked(value: u16) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u16 {
        self.0
    }
}

impl<'de> Deserialize<'de> for ValueEncodingVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u16::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

/// Stable identity of a Property Type implementation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PropertyTypeRef {
    pub key: PropertyTypeKey,
    pub version: PropertyTypeVersion,
}

impl PropertyTypeRef {
    pub fn new(key: PropertyTypeKey, version: PropertyTypeVersion) -> Self {
        Self { key, version }
    }

    pub fn builtin(kind: PropertyKind) -> Self {
        Self {
            key: PropertyTypeKey::builtin(kind.key()),
            version: PROPERTY_TYPE_VERSION_V1,
        }
    }
}

impl std::fmt::Display for PropertyTypeRef {
    fn fmt(
        &self,
        formatter: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(formatter, "{}@{}", self.key, self.version.get())
    }
}

/// The known, compile-time Property kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropertyKind {
    String,
    Integer,
    Html,
    Markdown,
    Relation,
    Select,
    MultiSelect,
    Id,
    Location,
    Date,
    Image,
    /// A block document stored as JSON.
    ///
    /// The block shape happens to be BlockNote's today. That is
    /// deliberately not part of the type contract: the value encoding is
    /// versioned separately from the type, so a different representation
    /// becomes encoding v2 rather than a new property type.
    RichText,
    /// A two-state flag. Stored canonically as a JSON boolean.
    Boolean,
}

impl PropertyKind {
    pub const ALL: [Self; 13] = [
        Self::String,
        Self::Integer,
        Self::Html,
        Self::Markdown,
        Self::Relation,
        Self::Select,
        Self::MultiSelect,
        Self::Id,
        Self::Location,
        Self::Date,
        Self::Image,
        Self::RichText,
        Self::Boolean,
    ];

    pub const fn key(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Integer => "integer",
            Self::Html => "html",
            Self::Markdown => "markdown",
            Self::Relation => "relation",
            Self::Select => "select",
            Self::MultiSelect => "multi_select",
            Self::Id => "id",
            Self::Location => "location",
            Self::Date => "date",
            Self::Image => "image",
            Self::RichText => "rich_text",
            Self::Boolean => "boolean",
        }
    }

    pub fn from_key(key: &PropertyTypeKey) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.key() == key.as_str())
    }

    pub fn type_ref(self) -> PropertyTypeRef {
        PropertyTypeRef::builtin(self)
    }
}

/// Typed configuration understood by a built-in handler.
#[derive(Debug, Clone)]
pub enum PropertyConfig {
    String,
    Integer,
    Html,
    Markdown,
    Relation(TypeRelation),
    Select(TypeSelect),
    MultiSelect(TypeMultiSelect),
    Id(TypeId),
    Location(TypeLocation),
    Date,
    Image,
    RichText,
    Boolean,
}

impl PropertyConfig {
    pub const fn kind(&self) -> PropertyKind {
        match self {
            Self::String => PropertyKind::String,
            Self::Integer => PropertyKind::Integer,
            Self::Html => PropertyKind::Html,
            Self::Markdown => PropertyKind::Markdown,
            Self::Relation(_) => PropertyKind::Relation,
            Self::Select(_) => PropertyKind::Select,
            Self::MultiSelect(_) => PropertyKind::MultiSelect,
            Self::Id(_) => PropertyKind::Id,
            Self::Location(_) => PropertyKind::Location,
            Self::Date => PropertyKind::Date,
            Self::Image => PropertyKind::Image,
            Self::RichText => PropertyKind::RichText,
            Self::Boolean => PropertyKind::Boolean,
        }
    }

    pub fn type_ref(&self) -> PropertyTypeRef {
        self.kind().type_ref()
    }

    pub fn handler(&self) -> &'static BuiltinPropertyTypeHandler {
        super::BUILTIN_PROPERTY_TYPE_REGISTRY
            .resolve(&self.type_ref())
            .expect("every PropertyConfig variant has a built-in handler")
    }
}

/// A config whose key or version is not present in this binary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpaquePropertyConfig {
    pub type_ref: PropertyTypeRef,
    pub raw_config: Value,
}

impl OpaquePropertyConfig {
    pub fn ensure_writable(&self) -> errors::Result<()> {
        Err(unsupported_property_type(&self.type_ref))
    }
}

#[derive(Debug, Clone)]
pub enum ResolvedPropertyConfig {
    Known(PropertyConfig),
    Opaque(OpaquePropertyConfig),
}

impl ResolvedPropertyConfig {
    pub fn type_ref(&self) -> PropertyTypeRef {
        match self {
            Self::Known(config) => config.type_ref(),
            Self::Opaque(config) => config.type_ref.clone(),
        }
    }

    pub fn ensure_writable(&self) -> errors::Result<()> {
        match self {
            Self::Known(config) => config.handler().validate_config(config),
            Self::Opaque(config) => config.ensure_writable(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpaquePropertyValue {
    pub type_ref: PropertyTypeRef,
    pub encoding_version: ValueEncodingVersion,
    pub raw_value: Value,
}

/// Storage-neutral representation of a normalized PropertyValue row.
///
/// Persistence adapters own record and Property identifiers; this envelope
/// owns only the versioned value contract shared by every adapter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EncodedPropertyValue {
    pub type_ref: PropertyTypeRef,
    pub encoding_version: ValueEncodingVersion,
    pub raw_value: Value,
}

impl OpaquePropertyValue {
    pub fn ensure_writable(&self) -> errors::Result<()> {
        Err(unsupported_property_value(
            &self.type_ref,
            self.encoding_version,
        ))
    }
}

/// A decoded normalized value, including forward-compatible opaque values.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    Known(KnownPropertyValue),
    Opaque(OpaquePropertyValue),
}

impl PropertyValue {
    pub fn type_ref(&self) -> &PropertyTypeRef {
        match self {
            Self::Known(value) => value.type_ref(),
            Self::Opaque(value) => &value.type_ref,
        }
    }

    pub fn encoding_version(&self) -> ValueEncodingVersion {
        match self {
            Self::Known(value) => value.encoding_version(),
            Self::Opaque(value) => value.encoding_version,
        }
    }

    pub fn ensure_writable(&self) -> errors::Result<()> {
        match self {
            Self::Known(value) => value.ensure_writable(),
            Self::Opaque(value) => value.ensure_writable(),
        }
    }
}

/// References extracted without giving persistence concerns to the domain
/// value itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyReference {
    Data {
        database_id: DatabaseId,
        data_id: DataId,
    },
    ExternalUrl(String),
    SelectOption(SelectItemId),
}

pub(crate) fn unsupported_property_type(
    type_ref: &PropertyTypeRef,
) -> errors::Error {
    errors::Error::not_supported(format!(
        "unsupported property type {type_ref} is read-only"
    ))
}

pub(crate) fn unsupported_property_value(
    type_ref: &PropertyTypeRef,
    encoding_version: ValueEncodingVersion,
) -> errors::Error {
    errors::Error::not_supported(format!(
        "unsupported property value {type_ref} encoding v{} is read-only",
        encoding_version.get()
    ))
}
