use std::collections::HashMap;
use std::io::Cursor;
use std::sync::{Arc, Mutex};

use arrow::array::{ArrayRef, Int64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use bytes::Bytes;
use hex::encode as hex_encode;
use once_cell::sync::Lazy;
use parquet::arrow::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::properties::WriterProperties;
use sha2::{Digest, Sha256};

use database_manager::domain::{
    Data, Property, PropertyDataValue, PropertyType,
};

use axum::{
    extract::{Extension, Path as AxumPath, Query},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use persistence::Storage;
use value_object::{InMemoryFile, Queries};

use crate::app::LibraryApp;
use crate::handler::library_executor_extractor::LibraryExecutor;
use crate::handler::types::{
    convert_property_value, AddDataRequest, DataResponse, SearchDataQuery,
    UpdateDataRequest,
};
use crate::usecase::LibraryOrg;
use crate::usecase::{
    AddDataInputData, DeleteDataInputData, PropertyDataInputData,
    SearchDataInputData, UpdateDataInputData, ViewDataInputData,
    ViewDataListInputData,
};
use tachyon_sdk::auth::ExecutorAction;

use super::types::{
    DataListResponse, ParquetResponse, PropertyDataResponse,
};

#[derive(Clone)]
pub struct ParquetStorage {
    storage: Arc<dyn Storage>,
    presign_storage: Arc<dyn Storage>,
    bucket: String,
}

impl ParquetStorage {
    pub fn new(
        storage: Arc<dyn Storage>,
        presign_storage: Arc<dyn Storage>,
        bucket: String,
    ) -> Self {
        Self {
            storage,
            presign_storage,
            bucket,
        }
    }
}

static PARQUET_CACHE: Lazy<Mutex<HashMap<String, String>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Compose a Markdown document with YAML frontmatter from data and properties.
///
/// This function is used both for the markdown export endpoint and for
/// syncing data to external providers like GitHub.
///
/// This is a re-export from the usecase layer for backwards compatibility.
pub fn compose_markdown(
    data: &database_manager::domain::Data,
    properties: &[database_manager::domain::Property],
) -> String {
    crate::usecase::markdown_composer::compose_markdown(data, properties)
}

/// GitHub sync configuration extracted from ext_github property
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ExtGithub {
    pub repo: String,
    pub path: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Whether to delete the old file when path changes
    #[serde(default, rename = "deleteOldPath")]
    pub delete_old_path: bool,
    /// The old path (for deletion when path changes)
    #[serde(default, rename = "oldPath")]
    pub old_path: Option<String>,
}

fn default_enabled() -> bool {
    true
}

/// Extract ext_github configuration from data properties
pub fn extract_ext_github(
    data: &database_manager::domain::Data,
    properties: &[database_manager::domain::Property],
) -> Option<ExtGithub> {
    let property_map: std::collections::HashMap<_, _> =
        properties.iter().map(|p| (p.id(), p)).collect();

    for property_data in data.property_data() {
        let Some(property) = property_map.get(property_data.property_id())
        else {
            continue;
        };

        if property.name().as_str() == "ext_github" {
            if let Some(value) = property_data.value() {
                let value_str = value.string_value();
                if let Ok(ext_github) =
                    serde_json::from_str::<ExtGithub>(&value_str)
                {
                    return Some(ext_github);
                }
            }
        }
    }

    None
}

#[utoipa::path(
    get,
    path = "/v1beta/repos/{org}/{repo}/data/{data_id}",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username"),
        ("data_id" = String, Path, description = "Data ID")
    )
)]
#[axum::debug_handler]
pub async fn view_data(
    AxumPath((org, repo, data_id)): AxumPath<(String, String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
    library_org: LibraryOrg,
) -> errors::Result<Json<DataResponse>> {
    let input = ViewDataInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: org,
        repo_username: repo,
        data_id,
    };

    let (data, properties) = library_app.view_data.execute(&input).await?;
    let response = DataResponse {
        id: data.id().to_string(),
        name: data.name().to_string(),
        items: data
            .property_data()
            .iter()
            .map(|p| PropertyDataResponse {
                property_id: p.property_id().to_string(),
                key: properties
                    .iter()
                    .find(|property| property.id() == p.property_id())
                    .unwrap()
                    .name()
                    .to_string(),
                value: p.value().clone().map(|v| v.into()),
            })
            .collect(),
    };
    Ok(Json(response))
}

#[utoipa::path(
	get,
	path = "/v1beta/repos/{org}/{repo}/data/{data_id}/md",
	params(
		("org" = String, Path, description = "Organization username"),
		("repo" = String, Path, description = "Repository username"),
		("data_id" = String, Path, description = "Data ID"),
	),
)]
#[axum::debug_handler]
pub async fn view_data_markdown(
    AxumPath((org, repo, data_id)): AxumPath<(String, String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
    library_org: LibraryOrg,
) -> errors::Result<impl IntoResponse> {
    let input = ViewDataInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: org,
        repo_username: repo,
        data_id,
    };

    let (data, properties) = library_app.view_data.execute(&input).await?;
    let markdown = compose_markdown(&data, &properties);

    Ok((
        StatusCode::OK,
        [("Content-Type", "text/markdown; charset=utf-8")],
        markdown,
    ))
}

#[utoipa::path(
    get,
    path = "/v1beta/repos/{org}/{repo}/data-list",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username")
    ),
    responses(
        (status = 200, description = "Data list found", body = Vec<DataResponse>),
        (status = 404, description = "Repository not found")
    )
)]
#[axum::debug_handler]
pub async fn view_data_list(
    AxumPath((org, repo)): AxumPath<(String, String)>,
    Query(query): Query<Queries>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
    library_org: LibraryOrg,
) -> errors::Result<Json<DataListResponse>> {
    let input = ViewDataListInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: org,
        repo_username: repo,
        page: query.offset,
        page_size: query.limit,
    };

    let result = library_app.view_data_list.execute(&input).await?;
    let (data_list, properties, paginator) = result;
    let response: Vec<DataResponse> = data_list
        .into_iter()
        .map(|data| DataResponse {
            id: data.id().to_string(),
            name: data.name().to_string(),
            items: data
                .property_data()
                .iter()
                .map(|p| PropertyDataResponse {
                    property_id: p.property_id().to_string(),
                    key: properties
                        .iter()
                        .find(|property| property.id() == p.property_id())
                        .unwrap()
                        .name()
                        .to_string(),
                    value: p.value().clone().map(|v| v.into()),
                })
                .collect(),
        })
        .collect();
    Ok(Json(DataListResponse {
        data: response,
        paginator,
    }))
}

#[utoipa::path(
    post,
    path = "/v1beta/repos/{org}/{repo}/data",
    request_body = AddDataRequest,
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username")
    ),
    responses(
        (status = 201, description = "Data created", body = DataResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Repository not found")
    )
)]
#[axum::debug_handler]
pub async fn add_data(
    executor: LibraryExecutor,
    library_org: LibraryOrg,
    AxumPath((org, repo)): AxumPath<(String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    Json(payload): Json<AddDataRequest>,
) -> errors::Result<Json<DataResponse>> {
    let property_data = payload
        .property_data
        .into_iter()
        .map(|p| PropertyDataInputData {
            property_id: p.property_id,
            value: convert_property_value(p.value),
        })
        .collect();

    let input = AddDataInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        actor: executor.get_id(),
        org_username: &org,
        repo_username: &repo,
        data_name: &payload.name,
        property_data,
    };

    let (data, properties) = library_app.save_data.execute(input).await?;
    let response = DataResponse {
        id: data.id().to_string(),
        name: data.name().to_string(),
        items: data
            .property_data()
            .iter()
            .map(|p| PropertyDataResponse {
                property_id: p.property_id().to_string(),
                key: properties
                    .iter()
                    .find(|property| property.id() == p.property_id())
                    .unwrap()
                    .name()
                    .to_string(),
                value: p.value().clone().map(|v| v.into()),
            })
            .collect(),
    };
    Ok(Json(response))
}

#[utoipa::path(
    put,
    path = "/v1beta/repos/{org}/{repo}/data/{data_id}",
    request_body = UpdateDataRequest,
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username"),
        ("data_id" = String, Path, description = "Data ID")
    ),
    responses(
        (status = 200, description = "Data updated", body = DataResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Data not found")
    )
)]
#[axum::debug_handler]
pub async fn update_data(
    executor: LibraryExecutor,
    library_org: LibraryOrg,
    AxumPath((org, repo, data_id)): AxumPath<(String, String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    Json(payload): Json<UpdateDataRequest>,
) -> errors::Result<Json<DataResponse>> {
    let property_data = payload
        .property_data
        .into_iter()
        .map(|p| PropertyDataInputData {
            property_id: p.property_id,
            value: convert_property_value(p.value),
        })
        .collect();

    let input = UpdateDataInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        actor: executor.get_id(),
        org_username: &org,
        repo_username: &repo,
        data_id: &data_id,
        data_name: &payload.name,
        property_data,
    };

    let (data, properties) = library_app.update_data.execute(input).await?;
    let response = DataResponse {
        id: data.id().to_string(),
        name: data.name().to_string(),
        items: data
            .property_data()
            .iter()
            .map(|p| PropertyDataResponse {
                property_id: p.property_id().to_string(),
                key: properties
                    .iter()
                    .find(|property| property.id() == p.property_id())
                    .unwrap()
                    .name()
                    .to_string(),
                value: p.value().clone().map(|v| v.into()),
            })
            .collect(),
    };
    Ok(Json(response))
}

#[utoipa::path(
    delete,
    path = "/v1beta/repos/{org}/{repo}/data/{data_id}",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username"),
        ("data_id" = String, Path, description = "Data ID")
    ),
    responses(
        (status = 204, description = "Data deleted"),
        (status = 404, description = "Data not found")
    )
)]
#[axum::debug_handler]
pub async fn delete_data(
    AxumPath((org, repo, data_id)): AxumPath<(String, String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
    library_org: LibraryOrg,
) -> errors::Result<StatusCode> {
    let input = DeleteDataInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        actor: executor.get_id().to_string(),
        org_username: org,
        repo_username: repo,
        data_id,
    };

    library_app.delete_data.execute(input).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/v1beta/repos/{org}/{repo}/data",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username"),
        ("name" = String, Query, description = "Data name to search for"),
        ("page" = Option<u32>, Query, description = "Page number"),
        ("page_size" = Option<u32>, Query, description = "Page size")
    ),
    responses(
        (status = 200, description = "Data found", body = Vec<DataResponse>),
        (status = 404, description = "Repository not found")
    )
)]
#[axum::debug_handler]
pub async fn search_data(
    AxumPath((org, repo)): AxumPath<(String, String)>,
    Query(query): Query<SearchDataQuery>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
    library_org: LibraryOrg,
) -> errors::Result<Json<DataListResponse>> {
    let input = SearchDataInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: &org,
        repo_username: &repo,
        name: &query.name,
        page: query.page,
        page_size: query.page_size,
    };

    let result = library_app.search_data.execute(&input).await?;
    let (data_list, properties, paginator) = result;
    let response: Vec<DataResponse> = data_list
        .into_iter()
        .map(|data| DataResponse {
            id: data.id().to_string(),
            name: data.name().to_string(),
            items: data
                .property_data()
                .iter()
                .map(|p| PropertyDataResponse {
                    property_id: p.property_id().to_string(),
                    key: properties
                        .iter()
                        .find(|property| property.id() == p.property_id())
                        .unwrap()
                        .name()
                        .to_string(),
                    value: p.value().clone().map(|v| v.into()),
                })
                .collect(),
        })
        .collect();
    Ok(Json(DataListResponse {
        data: response,
        paginator,
    }))
}

#[utoipa::path(
    get,
    path = "/v1beta/repos/{org}/{repo}/data/parquet",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username")
    ),
    responses(
        (status = 200, description = "Parquet file URL", body = ParquetResponse),
        (status = 404, description = "Repository not found")
    )
)]
#[axum::debug_handler]
pub async fn view_data_parquet(
    AxumPath((org, repo)): AxumPath<(String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    Extension(parquet_storage): Extension<ParquetStorage>,
    executor: LibraryExecutor,
    library_org: LibraryOrg,
) -> errors::Result<Json<ParquetResponse>> {
    let (data_list, properties) = fetch_all_data(
        &library_app,
        &executor,
        &library_org,
        org.clone(),
        repo.clone(),
    )
    .await?;

    let fingerprint = build_parquet_fingerprint(&data_list, &properties);
    let file_name = format!("data-{fingerprint}.parquet");
    let object_path = format!("library/{org}/{repo}/{file_name}");
    let cache_key = format!("{org}/{repo}");
    let should_upload = {
        let mut cache = PARQUET_CACHE.lock().map_err(|_| {
            errors::Error::internal_server_error("cache lock")
        })?;
        if cache.get(&cache_key) == Some(&fingerprint) {
            false
        } else {
            cache.insert(cache_key, fingerprint.clone());
            true
        }
    };

    if should_upload {
        let parquet_bytes = build_parquet_bytes(&data_list, &properties)?;
        let file = InMemoryFile::new(
            file_name,
            Some("application/parquet".to_string()),
            Bytes::from(parquet_bytes),
        )?;

        parquet_storage
            .storage
            .put_object(&parquet_storage.bucket, &object_path, &file)
            .await?;
    }

    let presigned_url = parquet_storage
        .presign_storage
        .presigned_get(&parquet_storage.bucket, &object_path, 900)
        .await?
        .to_string();

    Ok(Json(ParquetResponse { presigned_url }))
}

const PARQUET_PAGE_SIZE: u32 = 500;

enum ColumnValues {
    Strings(Vec<Option<String>>),
    Ints(Vec<Option<i64>>),
}

struct PropertyColumn {
    id: database_manager::domain::PropertyId,
    name: String,
    data_type: DataType,
    values: ColumnValues,
}

async fn fetch_all_data(
    library_app: &LibraryApp,
    executor: &LibraryExecutor,
    library_org: &LibraryOrg,
    org: String,
    repo: String,
) -> errors::Result<(Vec<Data>, Vec<Property>)> {
    let mut page = 1;
    let mut data_list = Vec::new();
    let mut properties = Vec::new();
    let mut total_pages = 1;

    while page <= total_pages {
        let input = ViewDataListInputData {
            executor,
            multi_tenancy: library_org,
            org_username: org.clone(),
            repo_username: repo.clone(),
            page: Some(page),
            page_size: Some(PARQUET_PAGE_SIZE),
        };
        let (page_data, page_properties, paginator) =
            library_app.view_data_list.execute(&input).await?;
        if page == 1 {
            properties = page_properties;
            total_pages = paginator.total_pages;
        }
        data_list.extend(page_data);
        page += 1;
    }

    Ok((data_list, properties))
}

fn build_parquet_fingerprint(
    data_list: &[Data],
    properties: &[Property],
) -> String {
    let mut hasher = Sha256::new();
    for data in data_list {
        hasher.update(data.id().to_string().as_bytes());
        hasher.update(data.updated_at().timestamp_millis().to_le_bytes());
    }
    for property in properties {
        hasher.update(property.id().to_string().as_bytes());
        hasher.update(property.property_type().to_string().as_bytes());
    }
    hex_encode(hasher.finalize())
}

fn build_parquet_bytes(
    data_list: &[Data],
    properties: &[Property],
) -> errors::Result<Vec<u8>> {
    let mut fields = vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("name", DataType::Utf8, false),
        Field::new("created_at", DataType::Utf8, true),
        Field::new("updated_at", DataType::Utf8, true),
    ];

    let mut property_columns = properties
        .iter()
        .map(|property| {
            let (data_type, values) = match property.property_type() {
                PropertyType::Integer => (
                    DataType::Int64,
                    ColumnValues::Ints(Vec::with_capacity(data_list.len())),
                ),
                _ => (
                    DataType::Utf8,
                    ColumnValues::Strings(Vec::with_capacity(
                        data_list.len(),
                    )),
                ),
            };
            PropertyColumn {
                id: property.id().clone(),
                name: format!("prop_{}", property.id()),
                data_type,
                values,
            }
        })
        .collect::<Vec<_>>();

    let mut ids = Vec::with_capacity(data_list.len());
    let mut names = Vec::with_capacity(data_list.len());
    let mut created_at = Vec::with_capacity(data_list.len());
    let mut updated_at = Vec::with_capacity(data_list.len());

    for data in data_list {
        ids.push(Some(data.id().to_string()));
        names.push(Some(data.name().to_string()));
        created_at.push(Some(data.created_at().to_rfc3339()));
        updated_at.push(Some(data.updated_at().to_rfc3339()));

        for column in &mut property_columns {
            let value = data
                .get_property_data(&column.id)
                .and_then(|property_data| property_data.value().as_ref());
            match &mut column.values {
                ColumnValues::Ints(values) => {
                    let number = match value {
                        Some(PropertyDataValue::Integer(number)) => {
                            Some(i64::from(*number))
                        }
                        Some(other) => {
                            other.string_value().parse::<i64>().ok()
                        }
                        None => None,
                    };
                    values.push(number);
                }
                ColumnValues::Strings(values) => {
                    let text = value
                        .map(|value| value.string_value())
                        .filter(|value| !value.is_empty());
                    values.push(text);
                }
            }
        }
    }

    for column in &property_columns {
        fields.push(Field::new(
            column.name.as_str(),
            column.data_type.clone(),
            true,
        ));
    }

    let mut arrays: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from(ids)) as ArrayRef,
        Arc::new(StringArray::from(names)) as ArrayRef,
        Arc::new(StringArray::from(created_at)) as ArrayRef,
        Arc::new(StringArray::from(updated_at)) as ArrayRef,
    ];

    for column in property_columns {
        match column.values {
            ColumnValues::Strings(values) => {
                arrays
                    .push(Arc::new(StringArray::from(values)) as ArrayRef);
            }
            ColumnValues::Ints(values) => {
                arrays.push(Arc::new(Int64Array::from(values)) as ArrayRef);
            }
        }
    }

    let schema = Arc::new(Schema::new(fields));
    let batch = RecordBatch::try_new(schema.clone(), arrays)
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

    let mut buffer = Vec::new();
    let cursor = Cursor::new(&mut buffer);
    let properties = WriterProperties::builder()
        .set_compression(Compression::SNAPPY)
        .build();
    let mut writer = ArrowWriter::try_new(cursor, schema, Some(properties))
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;
    writer
        .write(&batch)
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;
    writer
        .close()
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

    Ok(buffer)
}
