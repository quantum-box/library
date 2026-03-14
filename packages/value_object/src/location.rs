use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// # Location
/// TODO: add English documentation
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "axum", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "async-graphql", derive(async_graphql::InputObject))]
pub struct Location {
    /// TODO: add English documentation
    latitude: f64,
    /// TODO: add English documentation
    longitude: f64,
}

impl Location {
    /// TODO: add English documentation
    ///
    /// # Arguments
    /// TODO: add English documentation
    /// TODO: add English documentation
    ///
    /// # Errors
    /// TODO: add English documentation
    /// TODO: add English documentation
    pub fn new(latitude: f64, longitude: f64) -> errors::Result<Self> {
        if !(-90.0..=90.0).contains(&latitude) {
            return Err(errors::Error::invalid(format!(
                "緯度は-90.0から90.0の範囲で指定してください: {latitude}",
            )));
        }
        if !(-180.0..=180.0).contains(&longitude) {
            return Err(errors::Error::invalid(format!(
                "経度は-180.0から180.0の範囲で指定してください: {longitude}",
            )));
        }

        Ok(Self {
            latitude,
            longitude,
        })
    }

    /// TODO: add English documentation
    pub fn latitude(&self) -> f64 {
        self.latitude
    }

    /// TODO: add English documentation
    pub fn longitude(&self) -> f64 {
        self.longitude
    }

    /// TODO: add English documentation
    ///
    /// # Arguments
    /// TODO: add English documentation
    ///
    /// # Returns
    /// TODO: add English documentation
    pub fn distance_to(&self, other: &Self) -> f64 {
        // TODO: add English comment
        const EARTH_RADIUS_KM: f64 = 6371.0;

        let lat1_rad = self.latitude.to_radians();
        let lat2_rad = other.latitude.to_radians();
        let delta_lat = (other.latitude - self.latitude).to_radians();
        let delta_lng = (other.longitude - self.longitude).to_radians();

        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1_rad.cos()
                * lat2_rad.cos()
                * (delta_lng / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

        EARTH_RADIUS_KM * c
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{}", self.latitude, self.longitude)
    }
}

impl FromStr for Location {
    type Err = errors::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() != 2 {
            return Err(errors::Error::invalid(
                "位置情報は「緯度,経度」の形式で指定してください",
            ));
        }

        let latitude = parts[0].trim().parse::<f64>().map_err(|_| {
            errors::Error::invalid("緯度が数値ではありません")
        })?;
        let longitude = parts[1].trim().parse::<f64>().map_err(|_| {
            errors::Error::invalid("経度が数値ではありません")
        })?;

        Location::new(latitude, longitude)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[derive(Serialize, Deserialize)]
    struct SampleWithLocation {
        location: Location,
    }

    #[rstest]
    #[case(35.6812362, 139.7649361)] // TODO: add English comment
    #[case(0.0, 0.0)] // TODO: add English comment
    #[case(-33.8688197, 151.2092955)] // TODO: add English comment
    #[case(90.0, 180.0)] // TODO: add English comment
    #[case(-90.0, -180.0)] // TODO: add English comment
    fn test_location_creation_valid(
        #[case] latitude: f64,
        #[case] longitude: f64,
    ) {
        let location = Location::new(latitude, longitude);
        assert!(location.is_ok());
        assert_eq!(location.unwrap().latitude(), latitude);
    }

    #[rstest]
    #[case(91.0, 0.0)] // TODO: add English comment
    #[case(-91.0, 0.0)] // TODO: add English comment
    #[case(0.0, 181.0)] // TODO: add English comment
    #[case(0.0, -181.0)] // TODO: add English comment
    fn test_location_creation_invalid(
        #[case] latitude: f64,
        #[case] longitude: f64,
    ) {
        let location = Location::new(latitude, longitude);
        assert!(location.is_err());
    }

    #[rstest]
    #[case("35.6812362,139.7649361", 35.6812362, 139.7649361)]
    #[case("0,0", 0.0, 0.0)]
    #[case("-33.8688197,151.2092955", -33.8688197, 151.2092955)]
    fn test_location_from_str_valid(
        #[case] input: &str,
        #[case] lat: f64,
        #[case] lng: f64,
    ) {
        let location = Location::from_str(input);
        assert!(location.is_ok());
        let location = location.unwrap();
        assert_eq!(location.latitude(), lat);
        assert_eq!(location.longitude(), lng);
    }

    #[rstest]
    #[case("invalid")]
    #[case("invalid,139.7649361")]
    #[case("35.6812362,invalid")]
    #[case("91.0,180.0")]
    #[case("90.0,181.0")]
    fn test_location_from_str_invalid(#[case] input: &str) {
        let location = Location::from_str(input);
        assert!(location.is_err());
    }

    #[test]
    fn test_location_distance() {
        // TODO: add English comment
        let tokyo = Location::new(35.6812362, 139.7649361).unwrap();
        let shinjuku = Location::new(35.6896067, 139.7005713).unwrap();
        let distance = tokyo.distance_to(&shinjuku);

        // TODO: add English comment
        println!("計算された距離: {}km", distance);
        assert!((distance - 5.5).abs() < 1.0);
    }

    #[test]
    fn test_location_serde() {
        let location = Location::new(35.6812362, 139.7649361).unwrap();
        let sample = SampleWithLocation { location };

        let json = serde_json::to_string(&sample).unwrap();
        let deserialized: SampleWithLocation =
            serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.location, sample.location);
    }

    #[test]
    fn test_location_display() {
        let location = Location::new(35.6812362, 139.7649361).unwrap();
        assert_eq!(location.to_string(), "35.6812362,139.7649361");
    }
}
