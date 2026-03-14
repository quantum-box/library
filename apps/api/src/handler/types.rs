use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use value_object::{Location, OffsetPaginator};

// TODO: add English comment
#[derive(Serialize, Deserialize, ToSchema)]
pub struct OrganizationResponse {
    pub id: String,
    pub name: String,
    pub username: String,
    pub description: Option<String>,
    pub website: Option<String>,
    pub repos: Vec<RepoResponse>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RepoResponse {
    pub id: String,
    pub name: String,
    pub username: String,
    pub description: Option<String>,
    pub is_public: bool,
    pub organization_id: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct DataListResponse {
    pub data: Vec<DataResponse>,
    pub paginator: OffsetPaginator,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ParquetResponse {
    pub presigned_url: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct DataResponse {
    pub id: String,
    pub name: String,
    pub items: Vec<PropertyDataResponse>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PropertyDataResponse {
    pub property_id: String,
    pub key: String,
    pub value: Option<PropertyDataValue>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum PropertyDataValue {
    String(String),
    Integer(i32),
    Html(String),
    Markdown(String),
    Relation {
        database_id: String,
        data_id: Vec<String>,
    },
    Id(String),
    Select(String),
    MultiSelect(Vec<String>),
    Location(Location),
    Date(String),
    Image(String),
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PropertyResponse {
    pub id: String,
    pub name: String,
    pub property_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecation: Option<String>,
}

// TODO: add English comment
#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateOrganizationRequest {
    pub name: String,
    pub username: String,
    pub description: Option<String>,
    pub website: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct UpdateOrganizationRequest {
    pub name: String,
    pub description: Option<String>,
    pub website: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateRepoRequest {
    pub name: String,
    pub username: String,
    pub description: Option<String>,
    pub is_public: bool,
    pub database_id: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct UpdateRepoRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_public: Option<bool>,
    pub tags: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AddDataRequest {
    pub name: String,
    pub property_data: Vec<PropertyDataRequest>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PropertyDataRequest {
    pub property_id: String,
    pub value: serde_json::Value,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct UpdateDataRequest {
    pub name: String,
    pub property_data: Vec<PropertyDataRequest>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AddPropertyRequest {
    pub name: String,
    pub property_type: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct UpdatePropertyRequest {
    pub name: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ChangeRepoUsernameRequest {
    pub new_username: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct SourceResponse {
    pub id: String,
    pub repo_id: String,
    pub name: String,
    pub url: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateSourceRequest {
    pub name: String,
    pub url: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct UpdateSourceRequest {
    pub name: Option<String>,
    pub url: Option<Option<String>>,
}

// TODO: add English comment
#[derive(Serialize, Deserialize, ToSchema)]
pub struct SearchRepoQuery {
    pub name: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct SearchDataQuery {
    pub name: String,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

// TODO: add English comment
use crate::usecase::PropertyDataValueInputData;

pub fn convert_property_value(
    value: serde_json::Value,
) -> PropertyDataValueInputData {
    match value {
        serde_json::Value::String(s) => {
            PropertyDataValueInputData::String(s)
        }
        serde_json::Value::Number(n) => {
            PropertyDataValueInputData::Integer(n.to_string())
        }
        serde_json::Value::Array(arr) => {
            if let Some(first) = arr.first() {
                if first.is_string() {
                    // TODO: add English comment
                    PropertyDataValueInputData::MultiSelect(
                        arr.into_iter()
                            .filter_map(|v| {
                                v.as_str().map(|s| s.to_string())
                            })
                            .collect(),
                    )
                } else {
                    // TODO: add English comment
                    PropertyDataValueInputData::Relation(
                        arr.into_iter()
                            .filter_map(|v| {
                                v.as_str().map(|s| s.to_string())
                            })
                            .collect(),
                    )
                }
            } else {
                PropertyDataValueInputData::MultiSelect(vec![])
            }
        }
        serde_json::Value::Object(obj) => {
            if let Some(markdown) =
                obj.get("markdown").and_then(|value| value.as_str())
            {
                PropertyDataValueInputData::Markdown(markdown.to_string())
            } else if let Some(html) =
                obj.get("html").and_then(|value| value.as_str())
            {
                PropertyDataValueInputData::Html(html.to_string())
            } else {
                // TODO: add English comment
                PropertyDataValueInputData::Html(
                    serde_json::to_string(&obj).unwrap_or_default(),
                )
            }
        }
        _ => PropertyDataValueInputData::String("".to_string()),
    }
}

impl From<database_manager::domain::PropertyDataValue>
    for PropertyDataValue
{
    fn from(value: database_manager::domain::PropertyDataValue) -> Self {
        match value {
            database_manager::domain::PropertyDataValue::String(s) => {
                PropertyDataValue::String(s)
            }
            database_manager::domain::PropertyDataValue::Integer(i) => {
                PropertyDataValue::Integer(i)
            }
            database_manager::domain::PropertyDataValue::Html(s) => {
                PropertyDataValue::Html(s)
            }
            database_manager::domain::PropertyDataValue::Markdown(s) => {
                PropertyDataValue::Markdown(s)
            }
            database_manager::domain::PropertyDataValue::Relation(s, v) => {
                PropertyDataValue::Relation {
                    database_id: s.to_string(),
                    data_id: v.into_iter().map(|v| v.to_string()).collect(),
                }
            }
            database_manager::domain::PropertyDataValue::Id(s) => {
                PropertyDataValue::Id(s)
            }
            database_manager::domain::PropertyDataValue::Select(s) => {
                PropertyDataValue::Select(s.to_string())
            }
            database_manager::domain::PropertyDataValue::MultiSelect(s) => {
                PropertyDataValue::MultiSelect(
                    s.into_iter().map(|v| v.to_string()).collect(),
                )
            }
            database_manager::domain::PropertyDataValue::Location(s) => {
                PropertyDataValue::Location(s)
            }
            database_manager::domain::PropertyDataValue::Date(date) => {
                PropertyDataValue::Date(date.to_string())
            }
            database_manager::domain::PropertyDataValue::Image(url) => {
                PropertyDataValue::Image(url)
            }
        }
    }
}
