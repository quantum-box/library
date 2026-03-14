pub mod resolver;

use crate::domain;
use async_graphql::{
    Context, Enum, InputObject, Object, OneofObject, Result,
};
use strum::Display;

pub struct Resolver;

#[Object]
impl Resolver {
    async fn hello(&self, _ctx: &Context<'_>) -> Result<String> {
        Ok("Hello, world!".to_string())
    }
}

pub struct Mutation;

#[Object]
impl Mutation {
    async fn add_property(
        &self,
        _ctx: &Context<'_>,
        _input: AddPropertyInput,
    ) -> Result<String> {
        Ok("Hello, world!".to_string())
    }
}

impl TryFrom<SelectItem> for domain::SelectItem {
    type Error = errors::Error;

    fn try_from(
        value: SelectItem,
    ) -> std::result::Result<Self, Self::Error> {
        Ok(domain::SelectItem::new(
            value.id.parse()?,
            value.key.parse()?,
            value.name.parse()?,
        ))
    }
}

impl TryFrom<AddPropertyInput> for domain::PropertyType {
    type Error = errors::Error;

    fn try_from(
        input: AddPropertyInput,
    ) -> std::result::Result<Self, Self::Error> {
        match input {
            AddPropertyInput {
                property_type: PropertyType::Integer,
                ..
            } => Ok(domain::PropertyType::Integer),
            AddPropertyInput {
                property_type: PropertyType::String,
                ..
            } => Ok(domain::PropertyType::String),
            AddPropertyInput {
                property_type: PropertyType::Html,
                ..
            } => Ok(domain::PropertyType::Html),
            AddPropertyInput {
                property_type: PropertyType::Markdown,
                ..
            } => Ok(domain::PropertyType::Markdown),
            AddPropertyInput {
                property_type: PropertyType::Relation,
                meta:
                    Some(PropertyTypeMeta::Relation(TypeRelation {
                        database_id,
                    })),
                ..
            } => Ok(domain::PropertyType::Relation(
                domain::TypeRelation::new(database_id.parse()?),
            )),
            AddPropertyInput {
                property_type: PropertyType::Select,
                meta: Some(PropertyTypeMeta::Select(TypeSelect { items })),
                ..
            } => Ok(domain::PropertyType::Select(domain::TypeSelect::new(
                items
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<errors::Result<Vec<_>>>()?,
            ))),
            AddPropertyInput {
                property_type: PropertyType::MultiSelect,
                meta:
                    Some(PropertyTypeMeta::MultiSelect(TypeMultiSelect {
                        items,
                    })),
                ..
            } => Ok(domain::PropertyType::MultiSelect(
                domain::TypeMultiSelect::new(
                    items
                        .into_iter()
                        .map(TryInto::try_into)
                        .collect::<errors::Result<Vec<_>>>()?,
                ),
            )),
            AddPropertyInput {
                property_type: PropertyType::Id,
                meta: Some(PropertyTypeMeta::Id(IdType { auto_generate })),
                ..
            } => Ok(domain::PropertyType::Id(domain::TypeId::new(
                auto_generate,
            ))),
            AddPropertyInput {
                property_type: PropertyType::Date,
                ..
            } => Ok(domain::PropertyType::Date),
            other => {
                tracing::error!("not supported property type: {:?}", other);
                Err(errors::Error::invalid("Not supported property type"))
            }
        }
    }
}

#[derive(InputObject, Debug)]
pub struct AddPropertyInput {
    pub database_id: String,
    pub name: String,
    pub is_indexed: String,
    pub property_type: PropertyType,
    pub meta: Option<PropertyTypeMeta>,
}

#[derive(Clone, Copy, PartialEq, Eq, Enum, Display, Debug)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum PropertyType {
    String,
    Integer,
    #[graphql(deprecation = "Use MARKDOWN instead of HTML.")]
    Html,
    Markdown,
    Relation,
    Select,
    MultiSelect,
    Id,
    Date,
}

#[derive(OneofObject, Debug)]
pub enum PropertyTypeMeta {
    Relation(TypeRelation),
    Select(TypeSelect),
    MultiSelect(TypeMultiSelect),
    Id(IdType),
}

#[derive(InputObject, Debug)]
pub struct SelectItem {
    pub id: String,
    pub key: String,
    pub name: String,
}

#[derive(InputObject, Debug)]
pub struct TypeRelation {
    pub database_id: String,
    // multi: true,
}

#[derive(InputObject, Debug)]
pub struct TypeSelect {
    pub items: Vec<SelectItem>,
}

#[derive(InputObject, Debug)]
pub struct TypeMultiSelect {
    pub items: Vec<SelectItem>,
}

#[derive(InputObject, Debug)]
pub struct IdType {
    pub auto_generate: bool,
}
