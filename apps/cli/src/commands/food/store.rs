//! Reading and writing draft repos through the public REST API.
//!
//! The importer never touches the database: it lists records with
//! `GET /v1beta/repos/{org}/{repo}/data-list` and writes with the same
//! `PUT .../data/{data_id}/upsert` that `library data upsert` uses.

use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Context, Result};
use futures::{stream, StreamExt, TryStreamExt};
use reqwest::StatusCode;
use serde_json::{json, Value};

use super::plan::ExistingRecord;
use crate::client::LibraryClient;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyDef {
    pub id: String,
    pub name: String,
    /// `STRING`, `INTEGER`, `BOOLEAN`, ... as the API spells it.
    pub property_type: String,
}

/// Outcome of one upsert as the API reported it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpsertStatus {
    Created,
    Updated,
}

/// A failed write, and whether trying again could help.
#[derive(Debug, Clone)]
pub struct WriteError {
    pub message: String,
    pub retryable: bool,
}

#[allow(async_fn_in_trait)]
pub trait Store {
    async fn properties(&self, repo: &str) -> Result<Vec<PropertyDef>>;
    async fn list(
        &self,
        repo: &str,
        concurrency: usize,
    ) -> Result<Vec<ExistingRecord>>;
    async fn upsert(
        &self,
        repo: &str,
        data_id: &str,
        body: &Value,
    ) -> std::result::Result<UpsertStatus, WriteError>;
}

fn split(repo: &str) -> Result<(&str, &str)> {
    repo.split_once('/')
        .filter(|(o, r)| !o.is_empty() && !r.is_empty())
        .ok_or_else(|| {
            anyhow!("expected a repository as `org/repo`, got `{repo}`")
        })
}

/// A property value from a list response as text: `{"string": "x"}` → `x`.
pub fn value_text(value: &Value) -> Option<String> {
    let Value::Object(map) = value else {
        return None;
    };
    let (_, inner) = map.iter().next()?;
    Some(match inner {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => return None,
        other => other.to_string(),
    })
}

pub fn existing_from_json(record: &Value) -> Option<ExistingRecord> {
    let id = record.get("id")?.as_str()?.to_string();
    let name = record
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let fields = record
        .get("items")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            let key = item.get("key")?.as_str()?.trim().to_string();
            let value = value_text(item.get("value")?)?;
            Some((key, value))
        })
        .collect();
    Some(ExistingRecord { id, name, fields })
}

/// Encode a text value for a property's type, the way the API infers
/// property kinds from JSON.
pub fn encode_value(def: &PropertyDef, value: &str) -> Result<Value> {
    match def.property_type.to_ascii_uppercase().as_str() {
        "STRING" => Ok(Value::String(value.to_string())),
        "INTEGER" => {
            if value.is_empty() {
                return Ok(Value::Null);
            }
            let n: i64 = value
                .parse()
                .with_context(|| format!("{} expects an integer, got {value:?}", def.name))?;
            Ok(json!(n))
        }
        "BOOLEAN" => match value {
            "true" => Ok(json!(true)),
            "false" | "" => Ok(json!(false)),
            other => bail!("{} expects true/false, got {other:?}", def.name),
        },
        other => bail!(
            "property {} has type {other}; the importer writes String (or Integer/Boolean) properties only",
            def.name
        ),
    }
}

impl Store for LibraryClient {
    async fn properties(&self, repo: &str) -> Result<Vec<PropertyDef>> {
        let (org, repo) = split(repo)?;
        let response = self
            .get(&format!("/v1beta/repos/{org}/{repo}/properties"), &[])
            .await?;
        Ok(response
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|p| {
                Some(PropertyDef {
                    id: p.get("id")?.as_str()?.to_string(),
                    name: p.get("name")?.as_str()?.trim().to_string(),
                    property_type: p
                        .get("property_type")
                        .and_then(Value::as_str)
                        .unwrap_or("STRING")
                        .to_string(),
                })
            })
            .collect())
    }

    async fn list(
        &self,
        repo: &str,
        concurrency: usize,
    ) -> Result<Vec<ExistingRecord>> {
        let (org, name) = split(repo)?;
        let path = format!("/v1beta/repos/{org}/{name}/data-list");
        let page = |page: u32| {
            let path = path.clone();
            async move {
                self.get(
                    &path,
                    &[
                        ("page", page.to_string()),
                        ("page_size", "100".to_string()),
                        ("include_body", "true".to_string()),
                    ],
                )
                .await
            }
        };
        let first = page(1).await?;
        let total_pages = first
            .pointer("/paginator/total_pages")
            .and_then(Value::as_u64)
            .unwrap_or(1) as u32;
        let mut pages = vec![first];
        let rest: Vec<Value> = stream::iter(2..=total_pages)
            .map(page)
            .buffered(concurrency.max(1))
            .try_collect()
            .await?;
        pages.extend(rest);

        let mut records = BTreeMap::new();
        for p in &pages {
            for r in p
                .get("data")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if let Some(e) = existing_from_json(r) {
                    records.insert(e.id.clone(), e);
                }
            }
        }
        Ok(records.into_values().collect())
    }

    async fn upsert(
        &self,
        repo: &str,
        data_id: &str,
        body: &Value,
    ) -> std::result::Result<UpsertStatus, WriteError> {
        let (org, name) = split(repo).map_err(|e| WriteError {
            message: e.to_string(),
            retryable: false,
        })?;
        match self
            .try_request(
                reqwest::Method::PUT,
                &format!(
                    "/v1beta/repos/{org}/{name}/data/{data_id}/upsert"
                ),
                body,
            )
            .await
        {
            Ok((StatusCode::CREATED, _)) => Ok(UpsertStatus::Created),
            Ok((status, _)) if status.is_success() => {
                Ok(UpsertStatus::Updated)
            }
            Ok((status, text)) => Err(WriteError {
                message: format!(
                    "{status}: {}",
                    text.chars().take(300).collect::<String>()
                ),
                retryable: status.is_server_error()
                    || status == StatusCode::TOO_MANY_REQUESTS
                    || status == StatusCode::CONFLICT,
            }),
            Err(e) => Err(WriteError {
                message: format!("{e:#}"),
                retryable: true,
            }),
        }
    }
}

#[cfg(test)]
pub mod fake {
    //! An in-memory repo with the upsert API's patch semantics.

    use std::cell::RefCell;
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;

    #[derive(Default)]
    pub struct FakeStore {
        pub props: BTreeMap<String, Vec<PropertyDef>>,
        pub repos:
            RefCell<BTreeMap<String, BTreeMap<String, ExistingRecord>>>,
        /// Data IDs that fail (non-retryable) while listed here.
        pub failing: RefCell<BTreeSet<String>>,
        pub upserts: RefCell<usize>,
    }

    impl FakeStore {
        pub fn with_repo(
            mut self,
            repo: &str,
            properties: &[&str],
        ) -> Self {
            self.props.insert(
                repo.to_string(),
                properties
                    .iter()
                    .map(|p| PropertyDef {
                        id: format!("prop_{p}"),
                        name: p.to_string(),
                        property_type: "STRING".into(),
                    })
                    .collect(),
            );
            self.repos
                .borrow_mut()
                .insert(repo.to_string(), BTreeMap::new());
            self
        }

        pub fn record(
            &self,
            repo: &str,
            id: &str,
        ) -> Option<ExistingRecord> {
            self.repos.borrow().get(repo)?.get(id).cloned()
        }

        pub fn edit(
            &self,
            repo: &str,
            id: &str,
            property: &str,
            value: &str,
        ) {
            let mut repos = self.repos.borrow_mut();
            let r = repos.get_mut(repo).unwrap().get_mut(id).unwrap();
            r.fields.insert(property.to_string(), value.to_string());
        }

        pub fn count(&self, repo: &str) -> usize {
            self.repos.borrow().get(repo).map_or(0, |r| r.len())
        }
    }

    impl Store for FakeStore {
        async fn properties(&self, repo: &str) -> Result<Vec<PropertyDef>> {
            self.props
                .get(repo)
                .cloned()
                .ok_or_else(|| anyhow!("repo {repo} not found"))
        }

        async fn list(
            &self,
            repo: &str,
            _: usize,
        ) -> Result<Vec<ExistingRecord>> {
            Ok(self
                .repos
                .borrow()
                .get(repo)
                .map(|r| r.values().cloned().collect())
                .unwrap_or_default())
        }

        async fn upsert(
            &self,
            repo: &str,
            data_id: &str,
            body: &Value,
        ) -> std::result::Result<UpsertStatus, WriteError> {
            *self.upserts.borrow_mut() += 1;
            if self.failing.borrow().contains(data_id) {
                return Err(WriteError {
                    message: "500: injected failure".into(),
                    retryable: false,
                });
            }
            let props = &self.props[repo];
            let mut repos = self.repos.borrow_mut();
            let records = repos.get_mut(repo).unwrap();
            let created = !records.contains_key(data_id);
            let record =
                records.entry(data_id.to_string()).or_insert_with(|| {
                    ExistingRecord {
                        id: data_id.to_string(),
                        ..Default::default()
                    }
                });
            record.name =
                body["name"].as_str().unwrap_or_default().to_string();
            for p in body["property_data"].as_array().unwrap() {
                let id = p["property_id"].as_str().unwrap();
                let name = &props.iter().find(|d| d.id == id).unwrap().name;
                let value = match &p["value"] {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                record.fields.insert(name.clone(), value);
            }
            Ok(if created {
                UpsertStatus::Created
            } else {
                UpsertStatus::Updated
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_items_become_fields_by_property_name() {
        let record = json!({
            "id": "data_01",
            "name": "たまねぎ",
            "items": [
                {"property_id": "p1", "key": "source_food_code", "value": {"string": "06153"}},
                {"property_id": "p2", "key": "display_order", "value": {"integer": 20}},
                {"property_id": "p3", "key": "default_display", "value": {"boolean": true}},
                {"property_id": "p4", "key": "empty", "value": null},
            ],
        });
        let e = existing_from_json(&record).unwrap();
        assert_eq!(e.fields["source_food_code"], "06153");
        assert_eq!(e.fields["display_order"], "20");
        assert_eq!(e.fields["default_display"], "true");
        assert!(!e.fields.contains_key("empty"));
    }

    #[test]
    fn values_are_encoded_for_the_property_type() {
        let def = |t: &str| PropertyDef {
            id: "p".into(),
            name: "n".into(),
            property_type: t.into(),
        };
        assert_eq!(
            encode_value(&def("STRING"), "01001").unwrap(),
            json!("01001")
        );
        assert_eq!(encode_value(&def("INTEGER"), "20").unwrap(), json!(20));
        assert_eq!(
            encode_value(&def("BOOLEAN"), "true").unwrap(),
            json!(true)
        );
        assert!(encode_value(&def("RELATION"), "x").is_err());
        assert!(encode_value(&def("INTEGER"), "1.5").is_err());
    }
}
