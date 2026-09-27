//! An in-memory view of one Database, shaped for map and listing queries.
//!
//! Built from a full snapshot of the Database and thrown away whenever the
//! Database's revision moves (see `cache.rs`). Every query is a linear scan:
//! at the sizes Library Databases hold today (thousands of records) that
//! takes well under a millisecond, and it lets filters combine freely
//! without a secondary index per Property. ADR-0011 records when that stops
//! being true and what replaces it.

use std::collections::{HashMap, HashSet};

use database_manager::domain::{
    Data, Property, PropertyDataValue, PropertyId, PropertyType, SelectItem,
};
use unicode_normalization::UnicodeNormalization;

use super::query::{DataSearchQuery, FilterOp, PropertyFilter, SortOrder};

/// The Property that marks whether a record is published.
///
/// A Database opts into publication control by defining it. As a Select,
/// only records whose option key is [`PUBLISHED_OPTION_KEY`] are published;
/// as a Boolean, only records set to `true`. A Database without it has no
/// drafts, so every record is published.
pub const PUBLICATION_PROPERTY_KEY: &str = "publication_status";
pub const PUBLISHED_OPTION_KEY: &str = "published";

/// Which records a caller may see.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Visibility {
    /// Readers: records that are published.
    PublishedOnly,
    /// Editors who asked for drafts as well.
    All,
}

#[derive(Debug)]
pub struct SearchIndex {
    properties: Vec<Property>,
    /// Select and MultiSelect options by Property, so filters and text
    /// search can match an option by key or display name.
    options: HashMap<PropertyId, Vec<SelectItem>>,
    records: Vec<IndexedRecord>,
}

#[derive(Debug)]
struct IndexedRecord {
    data: Data,
    /// Normalized name and text values, for `q`.
    text: String,
    /// Normalized name, for `sort=name`.
    sort_name: String,
}

#[derive(Debug, Clone)]
pub struct SearchHit<'a> {
    pub data: &'a Data,
    pub distance_meters: Option<f64>,
}

impl SearchIndex {
    pub fn build(data: Vec<Data>, properties: Vec<Property>) -> Self {
        let options = properties
            .iter()
            .filter_map(|property| match property.property_type() {
                PropertyType::Select(select) => {
                    Some((property.id().clone(), select.items.clone()))
                }
                PropertyType::MultiSelect(select) => {
                    Some((property.id().clone(), select.items.clone()))
                }
                _ => None,
            })
            .collect::<HashMap<_, _>>();
        let records = data
            .into_iter()
            .map(|data| {
                let sort_name = normalize(&data.name().to_string());
                let text = searchable_text(&data, &options);
                IndexedRecord {
                    data,
                    text,
                    sort_name,
                }
            })
            .collect();
        Self {
            properties,
            options,
            records,
        }
    }

    pub fn properties(&self) -> &[Property] {
        &self.properties
    }

    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    /// Every record matching `query` that `visibility` allows, in the
    /// query's order.
    pub fn search(
        &self,
        query: &DataSearchQuery,
        visibility: Visibility,
    ) -> errors::Result<Vec<SearchHit<'_>>> {
        let sort = query.effective_sort();
        let location = if query.is_spatial() {
            Some(
                self.location_property(query.location_property.as_deref())?,
            )
        } else {
            None
        };
        let mut predicates = query
            .filters
            .iter()
            .map(|filter| self.compile(filter))
            .collect::<errors::Result<Vec<_>>>()?;
        if visibility == Visibility::PublishedOnly {
            if let Some(publication) = self.publication_predicate() {
                predicates.push(publication);
            }
        }
        let terms = query
            .text
            .as_deref()
            .map(|text| {
                normalize(text)
                    .split_whitespace()
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let ids =
            query.ids.iter().map(String::as_str).collect::<HashSet<_>>();

        let mut hits = Vec::new();
        for record in &self.records {
            if !ids.is_empty() && !ids.contains(record.data.id().as_ref()) {
                continue;
            }
            if !predicates.iter().all(|p| p.matches(&record.data)) {
                continue;
            }
            if !terms.iter().all(|term| record.text.contains(term.as_str()))
            {
                continue;
            }
            let mut distance_meters = None;
            if let Some(location) = location {
                let Some((lat, lng)) = location_of(&record.data, location)
                else {
                    continue;
                };
                if let Some(bbox) = &query.bbox {
                    if !bbox.contains(lat, lng) {
                        continue;
                    }
                }
                if let Some(near) = &query.near {
                    let distance = near.distance_meters(lat, lng);
                    if query.radius_meters.is_some_and(|r| distance > r) {
                        continue;
                    }
                    distance_meters = Some(distance);
                }
            }
            hits.push((record, distance_meters));
        }

        match sort {
            SortOrder::Distance => hits.sort_by(|(a, da), (b, db)| {
                da.unwrap_or(f64::MAX)
                    .total_cmp(&db.unwrap_or(f64::MAX))
                    .then_with(|| {
                        a.data.id().as_ref().cmp(b.data.id().as_ref())
                    })
            }),
            SortOrder::Name => hits.sort_by(|(a, _), (b, _)| {
                a.sort_name.cmp(&b.sort_name).then_with(|| {
                    a.data.id().as_ref().cmp(b.data.id().as_ref())
                })
            }),
            SortOrder::Updated => hits.sort_by(|(a, _), (b, _)| {
                b.data.updated_at().cmp(a.data.updated_at()).then_with(
                    || a.data.id().as_ref().cmp(b.data.id().as_ref()),
                )
            }),
        }

        Ok(hits
            .into_iter()
            .map(|(record, distance_meters)| SearchHit {
                data: &record.data,
                distance_meters,
            })
            .collect())
    }

    fn property(&self, key: &str) -> Option<&Property> {
        self.properties.iter().find(|p| p.name() == key)
    }

    fn location_property(
        &self,
        key: Option<&str>,
    ) -> errors::Result<&PropertyId> {
        let property = match key {
            Some(key) => self.property(key).ok_or_else(|| {
                errors::Error::invalid(format!(
                    "location_property `{key}` is not a Property of this repo"
                ))
            })?,
            None => self
                .properties
                .iter()
                .filter(|p| {
                    matches!(p.property_type(), PropertyType::Location(_))
                })
                .min_by_key(|p| *p.property_num())
                .ok_or_else(|| {
                    errors::Error::invalid(
                        "bbox and lat/lng need a Location Property, and \
                         this repo has none",
                    )
                })?,
        };
        if !matches!(property.property_type(), PropertyType::Location(_)) {
            return Err(errors::Error::invalid(format!(
                "location_property `{}` is not a Location Property",
                property.name()
            )));
        }
        Ok(property.id())
    }

    fn publication_predicate(&self) -> Option<Predicate> {
        let property = self.property(PUBLICATION_PROPERTY_KEY)?;
        let property_id = property.id().clone();
        Some(match property.property_type() {
            PropertyType::Select(select) => Predicate::OptionIn {
                property_id,
                option_ids: select
                    .items
                    .iter()
                    .filter(|item| {
                        item.key().to_string() == PUBLISHED_OPTION_KEY
                    })
                    .map(|item| item.id().to_string())
                    .collect(),
            },
            PropertyType::Boolean => Predicate::BooleanIn {
                property_id,
                accepted: vec![true],
            },
            // A publication Property this rule cannot read must not leak
            // drafts: fail closed.
            _ => Predicate::Nothing,
        })
    }

    fn compile(
        &self,
        filter: &PropertyFilter,
    ) -> errors::Result<Predicate> {
        let property = self.property(&filter.key).ok_or_else(|| {
            errors::Error::invalid(format!(
                "filter `{}` does not name a Property of this repo",
                filter.key
            ))
        })?;
        let property_id = property.id().clone();
        let not_ordered = || {
            errors::Error::invalid(format!(
                "filter `{}` supports only `:`; >= and <= need an Integer \
                 or Date Property",
                filter.key
            ))
        };
        match (property.property_type(), &filter.op) {
            (
                PropertyType::Select(_) | PropertyType::MultiSelect(_),
                FilterOp::AnyOf(values),
            ) => {
                let options = self
                    .options
                    .get(&property_id)
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                // An option can be named by id, key or display name, so
                // callers can use whichever they have. A value that names
                // no option matches nothing rather than failing, so a
                // client that still sends a retired option keeps working.
                let option_ids = values
                    .iter()
                    .flat_map(|value| {
                        let wanted = normalize(value);
                        options.iter().filter(move |item| {
                            item.id().as_ref() == value.as_str()
                                || normalize(&item.key().to_string())
                                    == wanted
                                || normalize(&item.name().to_string())
                                    == wanted
                        })
                    })
                    .map(|item| item.id().to_string())
                    .collect();
                Ok(Predicate::OptionIn {
                    property_id,
                    option_ids,
                })
            }
            (PropertyType::Boolean, FilterOp::AnyOf(values)) => {
                let accepted = values
                    .iter()
                    .map(|value| parse_bool(value))
                    .collect::<errors::Result<Vec<_>>>()?;
                Ok(Predicate::BooleanIn {
                    property_id,
                    accepted,
                })
            }
            (PropertyType::Integer, op) => {
                let parse = |value: &str| {
                    value.parse::<i64>().map_err(|_| {
                        errors::Error::invalid(format!(
                            "filter `{}` needs an integer, got `{value}`",
                            filter.key
                        ))
                    })
                };
                Ok(match op {
                    FilterOp::AnyOf(values) => Predicate::IntegerIn {
                        property_id,
                        values: values
                            .iter()
                            .map(|v| parse(v))
                            .collect::<errors::Result<_>>()?,
                    },
                    FilterOp::AtLeast(v) => Predicate::IntegerRange {
                        property_id,
                        min: Some(parse(v)?),
                        max: None,
                    },
                    FilterOp::AtMost(v) => Predicate::IntegerRange {
                        property_id,
                        min: None,
                        max: Some(parse(v)?),
                    },
                })
            }
            (PropertyType::Date, op) => {
                let parse = |value: &str| {
                    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
                        .map(|_| value.to_string())
                        .map_err(|_| {
                            errors::Error::invalid(format!(
                                "filter `{}` needs a YYYY-MM-DD date, got \
                                 `{value}`",
                                filter.key
                            ))
                        })
                };
                Ok(match op {
                    FilterOp::AnyOf(values) => Predicate::DateRange {
                        property_id,
                        allowed: Some(
                            values
                                .iter()
                                .map(|v| parse(v))
                                .collect::<errors::Result<_>>()?,
                        ),
                        min: None,
                        max: None,
                    },
                    FilterOp::AtLeast(v) => Predicate::DateRange {
                        property_id,
                        allowed: None,
                        min: Some(parse(v)?),
                        max: None,
                    },
                    FilterOp::AtMost(v) => Predicate::DateRange {
                        property_id,
                        allowed: None,
                        min: None,
                        max: Some(parse(v)?),
                    },
                })
            }
            (PropertyType::Relation(_), FilterOp::AnyOf(values)) => {
                Ok(Predicate::RelationIn {
                    property_id,
                    data_ids: values.iter().cloned().collect(),
                })
            }
            (
                PropertyType::String
                | PropertyType::Id(_)
                | PropertyType::Html
                | PropertyType::Markdown
                | PropertyType::Image,
                FilterOp::AnyOf(values),
            ) => Ok(Predicate::TextIn {
                property_id,
                values: values.iter().map(|v| normalize(v)).collect(),
            }),
            (PropertyType::Location(_) | PropertyType::RichText, _) => {
                Err(errors::Error::invalid(format!(
                    "filter `{}` names a Property that cannot be filtered; \
                     use bbox or lat/lng for locations and q for text",
                    filter.key
                )))
            }
            _ => Err(not_ordered()),
        }
    }
}

/// One compiled filter, checked against every candidate record.
#[derive(Debug)]
enum Predicate {
    OptionIn {
        property_id: PropertyId,
        option_ids: HashSet<String>,
    },
    /// A record with no value counts as `false`: an unticked checkbox is
    /// usually never written.
    BooleanIn {
        property_id: PropertyId,
        accepted: Vec<bool>,
    },
    IntegerIn {
        property_id: PropertyId,
        values: Vec<i64>,
    },
    IntegerRange {
        property_id: PropertyId,
        min: Option<i64>,
        max: Option<i64>,
    },
    /// Dates are `YYYY-MM-DD`, so string order is date order.
    DateRange {
        property_id: PropertyId,
        allowed: Option<Vec<String>>,
        min: Option<String>,
        max: Option<String>,
    },
    RelationIn {
        property_id: PropertyId,
        data_ids: HashSet<String>,
    },
    TextIn {
        property_id: PropertyId,
        values: Vec<String>,
    },
    Nothing,
}

impl Predicate {
    fn matches(&self, data: &Data) -> bool {
        match self {
            Self::OptionIn {
                property_id,
                option_ids,
            } => match value_of(data, property_id) {
                Some(PropertyDataValue::Select(id)) => {
                    option_ids.contains(id.as_ref())
                }
                Some(PropertyDataValue::MultiSelect(ids)) => {
                    ids.iter().any(|id| option_ids.contains(id.as_ref()))
                }
                _ => false,
            },
            Self::BooleanIn {
                property_id,
                accepted,
            } => {
                let value = match value_of(data, property_id) {
                    Some(PropertyDataValue::Boolean(value)) => *value,
                    None => false,
                    Some(_) => return false,
                };
                accepted.contains(&value)
            }
            Self::IntegerIn {
                property_id,
                values,
            } => match value_of(data, property_id) {
                Some(PropertyDataValue::Integer(value)) => {
                    values.contains(&i64::from(*value))
                }
                _ => false,
            },
            Self::IntegerRange {
                property_id,
                min,
                max,
            } => match value_of(data, property_id) {
                Some(PropertyDataValue::Integer(value)) => {
                    let value = i64::from(*value);
                    min.is_none_or(|min| value >= min)
                        && max.is_none_or(|max| value <= max)
                }
                _ => false,
            },
            Self::DateRange {
                property_id,
                allowed,
                min,
                max,
            } => match value_of(data, property_id) {
                Some(PropertyDataValue::Date(value)) => {
                    allowed.as_ref().is_none_or(|a| a.contains(value))
                        && min.as_ref().is_none_or(|min| value >= min)
                        && max.as_ref().is_none_or(|max| value <= max)
                }
                _ => false,
            },
            Self::RelationIn {
                property_id,
                data_ids,
            } => match value_of(data, property_id) {
                Some(PropertyDataValue::Relation(_, ids)) => {
                    ids.iter().any(|id| data_ids.contains(id.as_ref()))
                }
                _ => false,
            },
            Self::TextIn {
                property_id,
                values,
            } => match value_of(data, property_id).and_then(text_value) {
                Some(text) => values.contains(&normalize(text)),
                None => false,
            },
            Self::Nothing => false,
        }
    }
}

fn value_of<'a>(
    data: &'a Data,
    property_id: &PropertyId,
) -> Option<&'a PropertyDataValue> {
    data.property_data()
        .iter()
        .find(|p| p.property_id() == property_id)
        .and_then(|p| p.value().as_ref())
}

fn location_of(
    data: &Data,
    property_id: &PropertyId,
) -> Option<(f64, f64)> {
    match value_of(data, property_id)? {
        PropertyDataValue::Location(location) => {
            Some((location.latitude(), location.longitude()))
        }
        _ => None,
    }
}

fn text_value(value: &PropertyDataValue) -> Option<&str> {
    match value {
        PropertyDataValue::String(text)
        | PropertyDataValue::Html(text)
        | PropertyDataValue::Markdown(text)
        | PropertyDataValue::Id(text)
        | PropertyDataValue::Image(text) => Some(text),
        _ => None,
    }
}

fn parse_bool(value: &str) -> errors::Result<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Ok(true),
        "false" | "0" | "no" => Ok(false),
        other => Err(errors::Error::invalid(format!(
            "`{other}` is not a boolean; use true or false"
        ))),
    }
}

/// Fold width and case so `ＤＣＭ`, `dcm` and `DCM` all match.
fn normalize(text: &str) -> String {
    text.nfkc().flat_map(char::to_lowercase).collect()
}

/// Everything `q` looks at: the name, text values, and the key and display
/// name of each selected option. Fields are separated by a control
/// character so a term cannot match across two of them.
fn searchable_text(
    data: &Data,
    options: &HashMap<PropertyId, Vec<SelectItem>>,
) -> String {
    const SEPARATOR: char = '\u{1f}';
    let mut parts = vec![data.name().to_string()];
    for property_data in data.property_data() {
        let Some(value) = property_data.value() else {
            continue;
        };
        let option_text = |id: &str| {
            options
                .get(property_data.property_id())
                .and_then(|items| {
                    items.iter().find(|item| item.id().as_ref() == id)
                })
                .map(|item| format!("{} {}", item.key(), item.name()))
        };
        match value {
            PropertyDataValue::Select(id) => {
                parts.extend(option_text(id.as_ref()));
            }
            PropertyDataValue::MultiSelect(ids) => {
                parts.extend(
                    ids.iter().filter_map(|id| option_text(id.as_ref())),
                );
            }
            PropertyDataValue::Image(_) => {}
            other => parts.extend(text_value(other).map(str::to_string)),
        }
    }
    normalize(&parts.join(&SEPARATOR.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use database_manager::domain::{
        DataId, DatabaseId, PropertyData, SelectItemId, TypeLocation,
        TypeMultiSelect, TypeSelect,
    };
    use value_object::TenantId;

    /// A small Place database: 札幌駅 is the reference point, 小樽 about
    /// 34km away, and 函館 far outside any Sapporo viewport.
    struct Fixture {
        tenant: TenantId,
        database: DatabaseId,
        properties: Vec<Property>,
        home_center: SelectItemId,
        mall: SelectItemId,
        published: SelectItemId,
        draft: SelectItemId,
        brand_dcm: SelectItemId,
        brand_viva: SelectItemId,
    }

    impl Fixture {
        fn new(with_publication: bool) -> Self {
            let tenant = TenantId::default();
            let database = DatabaseId::default();
            let home_center = SelectItemId::default();
            let mall = SelectItemId::default();
            let published = SelectItemId::default();
            let draft = SelectItemId::default();
            let brand_dcm = SelectItemId::default();
            let brand_viva = SelectItemId::default();
            let item = |id: &SelectItemId, key: &str, name: &str| {
                SelectItem::new(
                    id.clone(),
                    key.parse().unwrap(),
                    name.parse().unwrap(),
                )
            };
            let mut types = vec![
                (
                    "location",
                    PropertyType::Location(TypeLocation::default()),
                ),
                (
                    "category",
                    PropertyType::Select(TypeSelect::new(vec![
                        item(&home_center, "home_center", "ホームセンター"),
                        item(&mall, "mall", "商業施設"),
                    ])),
                ),
                (
                    "brand",
                    PropertyType::MultiSelect(TypeMultiSelect::new(vec![
                        item(&brand_dcm, "dcm", "DCM"),
                        item(&brand_viva, "viva_home", "ビバホーム"),
                    ])),
                ),
                ("large_dog_allowed", PropertyType::Boolean),
                ("max_dog_weight", PropertyType::Integer),
                ("verified_at", PropertyType::Date),
                ("address", PropertyType::String),
            ];
            if with_publication {
                types.push((
                    PUBLICATION_PROPERTY_KEY,
                    PropertyType::Select(TypeSelect::new(vec![
                        item(&published, PUBLISHED_OPTION_KEY, "公開"),
                        item(&draft, "draft", "下書き"),
                    ])),
                ));
            }
            let properties = types
                .into_iter()
                .enumerate()
                .map(|(num, (name, property_type))| {
                    Property::new(
                        &Default::default(),
                        &tenant,
                        &database,
                        name,
                        &property_type,
                        false,
                        num as u32,
                    )
                })
                .collect();
            Self {
                tenant,
                database,
                properties,
                home_center,
                mall,
                published,
                draft,
                brand_dcm,
                brand_viva,
            }
        }

        fn record(
            &self,
            name: &str,
            updated_minutes_ago: i64,
            values: &[(&str, String)],
        ) -> Data {
            let property_data = values
                .iter()
                .map(|(key, value)| {
                    let property = self
                        .properties
                        .iter()
                        .find(|p| p.name() == key)
                        .unwrap();
                    PropertyData::new(property, value.clone()).unwrap()
                })
                .collect();
            let updated =
                Utc::now() - Duration::minutes(updated_minutes_ago);
            Data::new(
                &DataId::default(),
                &self.tenant,
                &self.database,
                name,
                property_data,
                updated,
                updated,
            )
            .unwrap()
        }

        fn index(&self) -> SearchIndex {
            let with_status = self
                .properties
                .iter()
                .any(|p| p.name() == PUBLICATION_PROPERTY_KEY);
            let values =
                |status: &SelectItemId,
                 rest: Vec<(&'static str, String)>| {
                    let mut values = rest;
                    if with_status {
                        values.push((
                            PUBLICATION_PROPERTY_KEY,
                            status.to_string(),
                        ));
                    }
                    values
                };
            let published = &self.published;
            let mut records = vec![
                self.record(
                    "DCM 札幌駅前店",
                    30,
                    &values(
                        published,
                        vec![
                            ("location", "43.0687,141.3508".into()),
                            ("category", self.home_center.to_string()),
                            ("brand", self.brand_dcm.to_string()),
                            ("large_dog_allowed", "true".into()),
                            ("max_dog_weight", "40".into()),
                            ("verified_at", "2026-09-01".into()),
                            ("address", "札幌市北区".into()),
                        ],
                    ),
                ),
                self.record(
                    "スーパービバホーム 小樽店",
                    10,
                    &values(
                        published,
                        vec![
                            ("location", "43.1907,140.9947".into()),
                            ("category", self.home_center.to_string()),
                            ("brand", self.brand_viva.to_string()),
                            ("max_dog_weight", "20".into()),
                            ("verified_at", "2026-03-01".into()),
                        ],
                    ),
                ),
                self.record(
                    "サッポロファクトリー",
                    20,
                    &values(
                        published,
                        vec![
                            ("location", "43.0655,141.3627".into()),
                            ("category", self.mall.to_string()),
                            ("large_dog_allowed", "false".into()),
                        ],
                    ),
                ),
                self.record(
                    "函館のカフェ",
                    40,
                    &values(
                        published,
                        vec![("location", "41.7687,140.7288".into())],
                    ),
                ),
                // No location, and no publication status either.
                self.record("場所未登録の店", 50, &[]),
            ];
            if with_status {
                records.push(self.record(
                    "DCM 下書き店",
                    5,
                    &values(
                        &self.draft,
                        vec![
                            ("location", "43.07,141.35".into()),
                            ("category", self.home_center.to_string()),
                        ],
                    ),
                ));
            }
            SearchIndex::build(records, self.properties.clone())
        }
    }

    fn names(hits: &[SearchHit<'_>]) -> Vec<String> {
        hits.iter().map(|h| h.data.name().to_string()).collect()
    }

    fn query(apply: impl FnOnce(&mut DataSearchQuery)) -> DataSearchQuery {
        let mut query = DataSearchQuery::default();
        apply(&mut query);
        query
    }

    fn filter(value: &str) -> PropertyFilter {
        value.parse().unwrap()
    }

    #[test]
    fn nearest_first_with_distances() {
        let fixture = Fixture::new(false);
        let index = fixture.index();
        let hits = index
            .search(
                &query(|q| {
                    q.near = Some(
                        super::super::query::GeoPoint::new(
                            43.0687, 141.3508,
                        )
                        .unwrap(),
                    )
                }),
                Visibility::PublishedOnly,
            )
            .unwrap();
        assert_eq!(
            names(&hits),
            vec![
                "DCM 札幌駅前店",
                "サッポロファクトリー",
                "スーパービバホーム 小樽店",
                "函館のカフェ",
            ],
            "records without a location are left out"
        );
        assert!(hits[0].distance_meters.unwrap() < 1.0);
        assert!(
            (1_000.0..1_500.0).contains(&hits[1].distance_meters.unwrap())
        );
    }

    #[test]
    fn radius_and_bbox_narrow_the_area() {
        let fixture = Fixture::new(false);
        let index = fixture.index();
        let near_station = query(|q| {
            q.near = Some(
                super::super::query::GeoPoint::new(43.0687, 141.3508)
                    .unwrap(),
            );
            q.radius_meters = Some(5_000.0);
        });
        assert_eq!(
            names(&index.search(&near_station, Visibility::All).unwrap()),
            vec!["DCM 札幌駅前店", "サッポロファクトリー"]
        );

        let sapporo_viewport = query(|q| {
            q.bbox = Some("141.2,42.95,141.5,43.15".parse().unwrap())
        });
        assert_eq!(
            names(
                &index.search(&sapporo_viewport, Visibility::All).unwrap()
            ),
            vec!["DCM 札幌駅前店", "サッポロファクトリー"],
            "no point means name order"
        );
    }

    #[test]
    fn select_filter_matches_by_key_label_or_id() {
        let fixture = Fixture::new(false);
        let index = fixture.index();
        for value in [
            "category:home_center".to_string(),
            "category:ホームセンター".to_string(),
            format!("category:{}", fixture.home_center),
        ] {
            let hits = index
                .search(
                    &query(|q| q.filters = vec![filter(&value)]),
                    Visibility::All,
                )
                .unwrap();
            assert_eq!(
                names(&hits),
                vec!["DCM 札幌駅前店", "スーパービバホーム 小樽店"],
                "{value}"
            );
        }
        let either = index
            .search(
                &query(|q| q.filters = vec![filter("brand:dcm|viva_home")]),
                Visibility::All,
            )
            .unwrap();
        assert_eq!(either.len(), 2);
        let retired = index
            .search(
                &query(|q| q.filters = vec![filter("category:closed")]),
                Visibility::All,
            )
            .unwrap();
        assert!(retired.is_empty(), "an unknown option matches nothing");
    }

    #[test]
    fn boolean_integer_and_date_filters() {
        let fixture = Fixture::new(false);
        let index = fixture.index();
        let run = |filters: &[&str]| {
            names(
                &index
                    .search(
                        &query(|q| {
                            q.filters =
                                filters.iter().map(|f| filter(f)).collect()
                        }),
                        Visibility::All,
                    )
                    .unwrap(),
            )
        };
        assert_eq!(
            run(&["large_dog_allowed:true"]),
            vec!["DCM 札幌駅前店"]
        );
        assert_eq!(
            run(&["large_dog_allowed:false", "category:home_center"]),
            vec!["スーパービバホーム 小樽店"],
            "no value counts as false, and filters combine with AND"
        );
        assert_eq!(run(&["max_dog_weight>=30"]), vec!["DCM 札幌駅前店"]);
        assert_eq!(
            run(&["max_dog_weight<=30"]),
            vec!["スーパービバホーム 小樽店"]
        );
        assert_eq!(
            run(&["verified_at>=2026-06-01"]),
            vec!["DCM 札幌駅前店"]
        );
    }

    #[test]
    fn invalid_filters_are_rejected() {
        let fixture = Fixture::new(false);
        let index = fixture.index();
        for bad in [
            "unknown:x",
            "location:43,141",
            "category>=1",
            "max_dog_weight:heavy",
            "verified_at>=yesterday",
            "large_dog_allowed:maybe",
        ] {
            assert!(
                index
                    .search(
                        &query(|q| q.filters = vec![filter(bad)]),
                        Visibility::All
                    )
                    .is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn text_search_folds_width_and_case_and_reads_option_labels() {
        let fixture = Fixture::new(false);
        let index = fixture.index();
        let run = |text: &str| {
            names(
                &index
                    .search(
                        &query(|q| q.text = Some(text.into())),
                        Visibility::All,
                    )
                    .unwrap(),
            )
        };
        assert_eq!(run("ｄｃｍ"), vec!["DCM 札幌駅前店"]);
        assert_eq!(run("北区"), vec!["DCM 札幌駅前店"], "text Property");
        assert_eq!(
            run("ビバホーム 小樽"),
            vec!["スーパービバホーム 小樽店"],
            "every term must match"
        );
        assert_eq!(
            run("商業施設"),
            vec!["サッポロファクトリー"],
            "option label"
        );
        assert!(run("dcm 小樽").is_empty());
    }

    #[test]
    fn readers_see_only_published_records() {
        let fixture = Fixture::new(true);
        let index = fixture.index();
        let home_centers =
            query(|q| q.filters = vec![filter("category:home_center")]);
        assert_eq!(
            names(
                &index
                    .search(&home_centers, Visibility::PublishedOnly)
                    .unwrap()
            ),
            vec!["DCM 札幌駅前店", "スーパービバホーム 小樽店"]
        );
        assert_eq!(
            names(&index.search(&home_centers, Visibility::All).unwrap()),
            vec![
                "DCM 下書き店",
                "DCM 札幌駅前店",
                "スーパービバホーム 小樽店"
            ]
        );
        let everything = DataSearchQuery::default();
        let published = index
            .search(&everything, Visibility::PublishedOnly)
            .unwrap();
        assert!(
            !names(&published).contains(&"場所未登録の店".to_string()),
            "a record with no status is not published"
        );
    }

    #[test]
    fn without_a_publication_property_every_record_is_published() {
        let fixture = Fixture::new(false);
        let index = fixture.index();
        let all = index
            .search(&DataSearchQuery::default(), Visibility::PublishedOnly)
            .unwrap();
        assert_eq!(all.len(), 5);
    }

    #[test]
    fn ids_restrict_and_updated_sort_is_newest_first() {
        let fixture = Fixture::new(false);
        let index = fixture.index();
        let all = index
            .search(
                &query(|q| q.sort = Some(SortOrder::Updated)),
                Visibility::All,
            )
            .unwrap();
        assert_eq!(
            all[0].data.name().to_string(),
            "スーパービバホーム 小樽店"
        );
        let wanted = all[2].data.id().to_string();
        let one = index
            .search(
                &query(|q| q.ids = vec![wanted.clone()]),
                Visibility::All,
            )
            .unwrap();
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].data.id().to_string(), wanted);
    }

    #[test]
    fn spatial_queries_need_a_location_property() {
        let fixture = Fixture::new(false);
        let index = SearchIndex::build(
            vec![],
            fixture
                .properties
                .iter()
                .filter(|p| p.name() != "location")
                .cloned()
                .collect(),
        );
        let bbox =
            query(|q| q.bbox = Some("141,43,142,44".parse().unwrap()));
        assert!(index.search(&bbox, Visibility::All).is_err());
        let wrong = query(|q| {
            q.bbox = Some("141,43,142,44".parse().unwrap());
            q.location_property = Some("address".into());
        });
        assert!(fixture.index().search(&wrong, Visibility::All).is_err());
    }
}
