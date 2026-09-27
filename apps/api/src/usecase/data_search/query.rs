//! What a caller asks the search index for, and how it is spelled on the
//! wire. Parsing lives here so the REST handler, and any later MCP or
//! GraphQL surface, reject the same malformed input with the same message.

use std::str::FromStr;

/// Upper bound on `ids`, matching the largest page a caller can request.
pub const MAX_IDS: usize = 100;

/// A latitude/longitude rectangle.
///
/// Spelled `minLng,minLat,maxLng,maxLat`, the GeoJSON `bbox` order that map
/// SDKs hand out for the visible viewport. `min_lng > max_lng` describes a
/// box that crosses the antimeridian.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    pub min_lng: f64,
    pub min_lat: f64,
    pub max_lng: f64,
    pub max_lat: f64,
}

impl BoundingBox {
    pub fn contains(&self, lat: f64, lng: f64) -> bool {
        if lat < self.min_lat || lat > self.max_lat {
            return false;
        }
        if self.min_lng <= self.max_lng {
            (self.min_lng..=self.max_lng).contains(&lng)
        } else {
            lng >= self.min_lng || lng <= self.max_lng
        }
    }
}

impl FromStr for BoundingBox {
    type Err = errors::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let parts = value
            .split(',')
            .map(|part| part.trim().parse::<f64>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| invalid_bbox())?;
        let [min_lng, min_lat, max_lng, max_lat] = parts[..] else {
            return Err(invalid_bbox());
        };
        let lat_ok = |lat: f64| (-90.0..=90.0).contains(&lat);
        let lng_ok = |lng: f64| (-180.0..=180.0).contains(&lng);
        if !(lat_ok(min_lat)
            && lat_ok(max_lat)
            && lng_ok(min_lng)
            && lng_ok(max_lng))
        {
            return Err(invalid_bbox());
        }
        if min_lat > max_lat {
            return Err(errors::Error::invalid(
                "bbox minLat must not be greater than maxLat",
            ));
        }
        Ok(Self {
            min_lng,
            min_lat,
            max_lng,
            max_lat,
        })
    }
}

fn invalid_bbox() -> errors::Error {
    errors::Error::invalid(
        "bbox must be minLng,minLat,maxLng,maxLat in degrees \
         (longitude -180..180, latitude -90..90)",
    )
}

/// The point distances are measured from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeoPoint {
    pub lat: f64,
    pub lng: f64,
}

impl GeoPoint {
    pub fn new(lat: f64, lng: f64) -> errors::Result<Self> {
        if !(-90.0..=90.0).contains(&lat)
            || !(-180.0..=180.0).contains(&lng)
        {
            return Err(errors::Error::invalid(
                "lat must be within -90..90 and lng within -180..180",
            ));
        }
        Ok(Self { lat, lng })
    }

    /// Great-circle distance in meters.
    pub fn distance_meters(&self, lat: f64, lng: f64) -> f64 {
        const EARTH_RADIUS_METERS: f64 = 6_371_000.0;
        let lat1 = self.lat.to_radians();
        let lat2 = lat.to_radians();
        let delta_lat = (lat - self.lat).to_radians();
        let delta_lng = (lng - self.lng).to_radians();
        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1.cos() * lat2.cos() * (delta_lng / 2.0).sin().powi(2);
        2.0 * EARTH_RADIUS_METERS * a.sqrt().atan2((1.0 - a).sqrt())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterOp {
    /// `key:a|b` — the value equals, or for multi-valued properties
    /// contains, any of the listed values.
    AnyOf(Vec<String>),
    /// `key>=v`
    AtLeast(String),
    /// `key<=v`
    AtMost(String),
}

/// One `filter` parameter: a Property key, an operator and a value.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyFilter {
    pub key: String,
    pub op: FilterOp,
}

impl FromStr for PropertyFilter {
    type Err = errors::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // Find whichever operator appears first, so a value that itself
        // contains ":" or ">=" does not confuse the split.
        let operator = [">=", "<=", ":"]
            .into_iter()
            .filter_map(|op| value.find(op).map(|at| (at, op)))
            .min_by_key(|(at, op)| (*at, std::cmp::Reverse(op.len())));
        let Some((at, op)) = operator else {
            return Err(invalid_filter(value));
        };
        let key = value[..at].trim();
        let operand = value[at + op.len()..].trim();
        if key.is_empty() || operand.is_empty() {
            return Err(invalid_filter(value));
        }
        let op = match op {
            ">=" => FilterOp::AtLeast(operand.to_string()),
            "<=" => FilterOp::AtMost(operand.to_string()),
            _ => FilterOp::AnyOf(
                operand
                    .split('|')
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(str::to_string)
                    .collect(),
            ),
        };
        if matches!(&op, FilterOp::AnyOf(values) if values.is_empty()) {
            return Err(invalid_filter(value));
        }
        Ok(Self {
            key: key.to_string(),
            op,
        })
    }
}

fn invalid_filter(value: &str) -> errors::Error {
    errors::Error::invalid(format!(
        "filter `{value}` must be `key:value`, `key:a|b`, `key>=value` \
         or `key<=value`"
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    /// Nearest first. Requires `lat`/`lng`.
    Distance,
    /// By record name.
    Name,
    /// Most recently updated first.
    Updated,
}

impl FromStr for SortOrder {
    type Err = errors::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "distance" => Ok(Self::Distance),
            "name" => Ok(Self::Name),
            "updated" => Ok(Self::Updated),
            other => Err(errors::Error::invalid(format!(
                "sort `{other}` must be one of distance, name, updated"
            ))),
        }
    }
}

/// A parsed, validated search request.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DataSearchQuery {
    /// Whitespace-separated terms; every term must appear.
    pub text: Option<String>,
    pub bbox: Option<BoundingBox>,
    pub near: Option<GeoPoint>,
    pub radius_meters: Option<f64>,
    /// Which Location Property positions a record. Defaults to the
    /// Database's first Location Property.
    pub location_property: Option<String>,
    /// All filters must hold.
    pub filters: Vec<PropertyFilter>,
    /// Restrict to these record ids, for fetching details through the same
    /// publication rules as the listing.
    pub ids: Vec<String>,
    pub sort: Option<SortOrder>,
}

impl DataSearchQuery {
    /// Build a query from raw wire values, validating how they combine.
    #[allow(clippy::too_many_arguments)]
    pub fn parse(
        text: Option<&str>,
        bbox: Option<&str>,
        lat: Option<f64>,
        lng: Option<f64>,
        radius_meters: Option<f64>,
        location_property: Option<&str>,
        filters: &[String],
        ids: Option<&str>,
        sort: Option<&str>,
    ) -> errors::Result<Self> {
        let text = text
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_string);
        let bbox = bbox
            .map(str::trim)
            .filter(|b| !b.is_empty())
            .map(BoundingBox::from_str)
            .transpose()?;
        let near = match (lat, lng) {
            (Some(lat), Some(lng)) => Some(GeoPoint::new(lat, lng)?),
            (None, None) => None,
            _ => {
                return Err(errors::Error::invalid(
                    "lat and lng must be given together",
                ))
            }
        };
        if let Some(radius) = radius_meters {
            if near.is_none() {
                return Err(errors::Error::invalid(
                    "radius_m requires lat and lng",
                ));
            }
            if !radius.is_finite() || radius <= 0.0 {
                return Err(errors::Error::invalid(
                    "radius_m must be a positive number of meters",
                ));
            }
        }
        let filters = filters
            .iter()
            .map(|f| f.parse())
            .collect::<errors::Result<Vec<PropertyFilter>>>()?;
        let ids: Vec<String> = ids
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(str::to_string)
            .collect();
        if ids.len() > MAX_IDS {
            return Err(errors::Error::invalid(format!(
                "ids accepts at most {MAX_IDS} ids"
            )));
        }
        let sort = sort
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(SortOrder::from_str)
            .transpose()?;
        if sort == Some(SortOrder::Distance) && near.is_none() {
            return Err(errors::Error::invalid(
                "sort=distance requires lat and lng",
            ));
        }
        Ok(Self {
            text,
            bbox,
            near,
            radius_meters,
            location_property: location_property
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(str::to_string),
            filters,
            ids,
            sort,
        })
    }

    pub fn is_spatial(&self) -> bool {
        self.bbox.is_some() || self.near.is_some()
    }

    /// The order results come back in when the caller names none.
    pub fn effective_sort(&self) -> SortOrder {
        self.sort.unwrap_or(if self.near.is_some() {
            SortOrder::Distance
        } else {
            SortOrder::Name
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bbox_parses_geojson_order() {
        let bbox: BoundingBox = "141.2,42.9,141.5,43.2".parse().unwrap();
        assert_eq!(bbox.min_lng, 141.2);
        assert_eq!(bbox.min_lat, 42.9);
        assert!(bbox.contains(43.06, 141.35));
        assert!(!bbox.contains(43.06, 141.6));
        assert!(!bbox.contains(43.3, 141.35));
    }

    #[test]
    fn bbox_across_the_antimeridian_wraps() {
        let bbox: BoundingBox = "170,-10,-170,10".parse().unwrap();
        assert!(bbox.contains(0.0, 175.0));
        assert!(bbox.contains(0.0, -175.0));
        assert!(!bbox.contains(0.0, 0.0));
    }

    #[test]
    fn bbox_rejects_malformed_input() {
        assert!("1,2,3".parse::<BoundingBox>().is_err());
        assert!("a,b,c,d".parse::<BoundingBox>().is_err());
        assert!("0,91,1,92".parse::<BoundingBox>().is_err());
        assert!("0,10,1,5".parse::<BoundingBox>().is_err());
    }

    #[test]
    fn distance_between_sapporo_and_otaru_is_about_34km() {
        let sapporo = GeoPoint::new(43.0687, 141.3508).unwrap();
        let meters = sapporo.distance_meters(43.1907, 140.9947);
        assert!((31_000.0..35_000.0).contains(&meters), "{meters}");
    }

    #[test]
    fn filter_parses_each_operator() {
        assert_eq!(
            "category:home_center|mall"
                .parse::<PropertyFilter>()
                .unwrap(),
            PropertyFilter {
                key: "category".into(),
                op: FilterOp::AnyOf(vec![
                    "home_center".into(),
                    "mall".into()
                ]),
            }
        );
        assert_eq!(
            "max_dog_weight>=30".parse::<PropertyFilter>().unwrap().op,
            FilterOp::AtLeast("30".into())
        );
        assert_eq!(
            "verified_at<=2026-01-01"
                .parse::<PropertyFilter>()
                .unwrap()
                .op,
            FilterOp::AtMost("2026-01-01".into())
        );
    }

    #[test]
    fn filter_value_may_contain_operator_characters() {
        let filter: PropertyFilter =
            "website:https://example.com".parse().unwrap();
        assert_eq!(filter.key, "website");
        assert_eq!(
            filter.op,
            FilterOp::AnyOf(vec!["https://example.com".into()])
        );
    }

    #[test]
    fn filter_rejects_missing_parts() {
        for bad in ["category", ":x", "category:", "weight>=", "a:|"] {
            assert!(bad.parse::<PropertyFilter>().is_err(), "{bad}");
        }
    }

    fn parse(
        lat: Option<f64>,
        lng: Option<f64>,
        radius: Option<f64>,
        sort: Option<&str>,
    ) -> errors::Result<DataSearchQuery> {
        DataSearchQuery::parse(
            None,
            None,
            lat,
            lng,
            radius,
            None,
            &[],
            None,
            sort,
        )
    }

    #[test]
    fn point_parameters_must_come_together() {
        assert!(parse(Some(43.0), None, None, None).is_err());
        assert!(parse(None, None, Some(100.0), None).is_err());
        assert!(parse(None, None, None, Some("distance")).is_err());
        assert!(parse(Some(43.0), Some(141.0), Some(0.0), None).is_err());
        let query =
            parse(Some(43.0), Some(141.0), Some(500.0), None).unwrap();
        assert_eq!(query.effective_sort(), SortOrder::Distance);
    }

    #[test]
    fn default_sort_without_a_point_is_name() {
        let query = parse(None, None, None, None).unwrap();
        assert_eq!(query.effective_sort(), SortOrder::Name);
        assert!(!query.is_spatial());
    }

    #[test]
    fn ids_are_split_and_bounded() {
        let query = DataSearchQuery::parse(
            None,
            None,
            None,
            None,
            None,
            None,
            &[],
            Some(" a, b ,,"),
            None,
        )
        .unwrap();
        assert_eq!(query.ids, vec!["a".to_string(), "b".to_string()]);

        let too_many =
            (0..=MAX_IDS).map(|i| i.to_string()).collect::<Vec<_>>();
        assert!(DataSearchQuery::parse(
            None,
            None,
            None,
            None,
            None,
            None,
            &[],
            Some(&too_many.join(",")),
            None,
        )
        .is_err());
    }
}
