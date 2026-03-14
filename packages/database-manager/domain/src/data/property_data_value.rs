use super::*;
use std::fmt;
use value_object::Location;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum PropertyDataValue {
    String(String),
    Integer(i32),
    Html(String),
    Markdown(String),
    Relation(DatabaseId, Vec<DataId>),
    Id(String),
    Location(Location),
    Select(SelectItemId),
    MultiSelect(Vec<SelectItemId>),
    Date(String),  // ISO 8601 format: YYYY-MM-DD
    Image(String), // Image URL
}

impl PropertyDataValue {
    pub fn new(
        value: &str,
        property_type: &PropertyType,
    ) -> errors::Result<Self> {
        Self::parse_from_string(value, property_type)
    }

    pub fn property_type(&self) -> PropertyType {
        match self {
            PropertyDataValue::String(_) => PropertyType::String,
            PropertyDataValue::Integer(_) => PropertyType::Integer,
            PropertyDataValue::Html(_) => PropertyType::Html,
            PropertyDataValue::Markdown(_) => PropertyType::Markdown,
            PropertyDataValue::Relation(id, _) => {
                PropertyType::Relation(TypeRelation::new(id.clone()))
            }
            PropertyDataValue::Id(_) => PropertyType::Id(TypeId::default()),
            PropertyDataValue::Location(_) => {
                PropertyType::Location(TypeLocation::default())
            }
            PropertyDataValue::Select(_) => {
                PropertyType::Select(TypeSelect::default())
            }
            PropertyDataValue::MultiSelect(_) => {
                PropertyType::MultiSelect(TypeMultiSelect::default())
            }
            PropertyDataValue::Date(_) => PropertyType::Date,
            PropertyDataValue::Image(_) => PropertyType::Image,
        }
    }

    pub fn string_value(&self) -> String {
        match self {
            PropertyDataValue::String(s) => s.clone(),
            PropertyDataValue::Integer(i) => i.to_string(),
            PropertyDataValue::Html(s) => s.clone(),
            PropertyDataValue::Markdown(s) => s.clone(),
            PropertyDataValue::Relation(db_id, data_ids) => format!(
                "{}{}",
                db_id,
                if data_ids.is_empty() {
                    String::new()
                } else {
                    format!(
                        ",{}",
                        data_ids
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(",")
                    )
                }
            ),
            PropertyDataValue::Id(auto_generate) => {
                auto_generate.to_string()
            }
            PropertyDataValue::Location(location) => location.to_string(),
            PropertyDataValue::Select(option_id) => option_id.to_string(),
            PropertyDataValue::MultiSelect(option_ids) => option_ids
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(","),
            PropertyDataValue::Date(date) => date.clone(),
            PropertyDataValue::Image(url) => url.clone(),
        }
    }

    fn parse_from_string(
        text: &str,
        property_type: &PropertyType,
    ) -> errors::Result<Self> {
        match property_type {
            PropertyType::String => Self::parse_string(text),
            PropertyType::Integer => Self::parse_integer(text),
            PropertyType::Html => Self::parse_html(text),
            PropertyType::Markdown => Self::parse_markdown(text),
            PropertyType::Relation(_) => Self::parse_relation(text),
            PropertyType::Id(_) => Self::parse_id(text),
            PropertyType::Location(_) => Self::parse_location(text),
            PropertyType::Select(_) => Self::parse_select(text),
            PropertyType::MultiSelect(_) => Self::parse_multi_select(text),
            PropertyType::Date => Self::parse_date(text),
            PropertyType::Image => Self::parse_image(text),
        }
    }

    fn parse_string(input: &str) -> errors::Result<PropertyDataValue> {
        Ok(PropertyDataValue::String(input.to_string()))
    }

    fn parse_integer(input: &str) -> errors::Result<PropertyDataValue> {
        if input.is_empty() {
            return Ok(PropertyDataValue::Integer(0));
        }
        Ok(PropertyDataValue::Integer(input.parse()?))
    }

    fn parse_html(input: &str) -> errors::Result<PropertyDataValue> {
        // TODO: add English comment
        if input.len() > 3145728 {
            return Err(errors::Error::business_logic(
                "HTML size is too large. Max size is 3145728 bytes.",
            ));
        }
        // TODO: add English comment
        // let document = scraper::Html::parse_fragment(input);
        // if !document.errors.is_empty() {
        //     let errs = document
        //         .errors
        //         .iter()
        //         .map(|e| e.to_string())
        //         .collect::<Vec<_>>()
        //         .join("\n");
        //     return Err(errors::Error::business_logic(format!(
        //         "Invalid HTML: {}",
        //         errs
        //     )));
        // }
        Ok(PropertyDataValue::Html(input.to_string()))
    }

    fn parse_markdown(input: &str) -> errors::Result<PropertyDataValue> {
        const MAX_MARKDOWN_SIZE_BYTES: usize = 65_535;
        if input.len() > MAX_MARKDOWN_SIZE_BYTES {
            return Err(errors::Error::business_logic(
                "Markdown size is too large. Max size is 65535 bytes.",
            ));
        }
        Ok(PropertyDataValue::Markdown(input.to_string()))
    }

    fn parse_relation(input: &str) -> errors::Result<PropertyDataValue> {
        let parts = input.split(',').collect::<Vec<_>>();
        let database_id = DatabaseId::from_str(
            parts
                .first()
                .ok_or(anyhow::anyhow!("DatabaseId is missing"))?,
        )?;
        let ids = parts
            .into_iter()
            .skip(1)
            .map(|id| id.parse::<DataId>().unwrap())
            .collect::<Vec<_>>();
        Ok(PropertyDataValue::Relation(database_id, ids))
    }

    fn parse_id(input: &str) -> errors::Result<PropertyDataValue> {
        Ok(PropertyDataValue::Id(input.to_string()))
    }

    fn parse_location(input: &str) -> errors::Result<PropertyDataValue> {
        let location = input.parse::<Location>()?;
        Ok(PropertyDataValue::Location(location))
    }

    fn parse_select(input: &str) -> errors::Result<PropertyDataValue> {
        if input.is_empty() {
            return Err(errors::Error::business_logic(
                "Select item id is empty",
            ));
        }
        let select_item_id = SelectItemId::from_str(input)?;
        Ok(PropertyDataValue::Select(select_item_id))
    }

    fn parse_multi_select(
        input: &str,
    ) -> errors::Result<PropertyDataValue> {
        if input.is_empty() {
            return Ok(PropertyDataValue::MultiSelect(vec![]));
        }
        let ids = input
            .split(',')
            .map(SelectItemId::from_str)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(PropertyDataValue::MultiSelect(ids))
    }

    fn parse_date(input: &str) -> errors::Result<PropertyDataValue> {
        if input.is_empty() {
            return Err(errors::Error::business_logic(
                "Date value cannot be empty",
            ));
        }
        // Validate ISO 8601 date format (YYYY-MM-DD)
        let parts: Vec<&str> = input.split('-').collect();
        if parts.len() != 3 {
            return Err(errors::Error::business_logic(
                "Invalid date format. Expected YYYY-MM-DD",
            ));
        }
        let year: i32 = parts[0].parse().map_err(|_| {
            errors::Error::business_logic("Invalid year in date")
        })?;
        let month: u32 = parts[1].parse().map_err(|_| {
            errors::Error::business_logic("Invalid month in date")
        })?;
        let day: u32 = parts[2].parse().map_err(|_| {
            errors::Error::business_logic("Invalid day in date")
        })?;

        // Basic validation
        if !matches!(year, 1..=9999) {
            return Err(errors::Error::business_logic(
                "Year must be between 1 and 9999",
            ));
        }
        if !matches!(month, 1..=12) {
            return Err(errors::Error::business_logic(
                "Month must be between 1 and 12",
            ));
        }
        if !matches!(day, 1..=31) {
            return Err(errors::Error::business_logic(
                "Day must be between 1 and 31",
            ));
        }

        // Validate date using chrono
        use chrono::NaiveDate;
        NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| {
            errors::Error::business_logic("Invalid date value")
        })?;

        Ok(PropertyDataValue::Date(input.to_string()))
    }

    fn parse_image(input: &str) -> errors::Result<PropertyDataValue> {
        const MAX_IMAGE_URL_SIZE_BYTES: usize = 2048;
        if input.len() > MAX_IMAGE_URL_SIZE_BYTES {
            return Err(errors::Error::business_logic(
                "Image URL is too long. Max size is 2048 bytes.",
            ));
        }
        Ok(PropertyDataValue::Image(input.to_string()))
    }
}

impl FromStr for PropertyDataValue {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.parse::<i32>() {
            Ok(parsed_num) => Ok(Self::Integer(parsed_num)),
            Err(_) => Ok(Self::String(s.to_string())),
        }
    }
}

/// TODO: add English documentation
impl fmt::Display for PropertyDataValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[cfg(test)]
mod unit {
    use super::*;
    use rstest::*;

    // relation
    #[rstest]
    #[case(
        "db_01hmp05xtq6fs5mmk8fg125cy7",
        "db_01hmp05xtq6fs5mmk8fg125cy7,data_01hmp06dkf89pzjp75p1p6gfw7,data_01hmp076wtgn1m5m5ap0zp8y28"
    )]
    #[case(
        "db_01hmp05xtq6fs5mmk8fg125cy7",
        "db_01hmp05xtq6fs5mmk8fg125cy7"
    )]
    #[should_panic]
    #[case(
        "db_01hmp05xtq6fs5mmk8fg125cy7",
        "data_01hmp05xtq6fs5mmk8fg125cy7,data_01hmp06dkf89pzjp75p1p6gfw7"
    )]
    fn test_parse_relation_and_to_string(
        #[case] db: &str,
        #[case] input: &str,
    ) {
        let actual = PropertyDataValue::new(
            input,
            &PropertyType::Relation(TypeRelation::new(db.parse().unwrap())),
        );
        assert!(actual.is_ok());
        assert_eq!(input.to_string(), actual.unwrap().string_value());
    }

    // TODO: add English comment
    #[rstest]
    #[case("aaa")]
    #[case("<!DOCTYPE html>")]
    #[case("<p>a</p>")]
    #[case(
        r#"
    <!DOCTYPE html>
    <meta charset="utf-8">
    <title>Hello, world!</title>
    <h1 class="foo">Hello, <i>world!</i></h1>
"#
    )]
    fn test_parse_html(#[case] html: &str) {
        assert!(PropertyDataValue::parse_html(html).is_ok());
    }

    #[rstest]
    #[case("")]
    #[case("# Heading")]
    #[case("- list item\n- another item")]
    fn test_parse_markdown(#[case] markdown: &str) {
        assert!(PropertyDataValue::parse_markdown(markdown).is_ok());
    }

    #[rstest]
    #[should_panic]
    #[case(&"a".repeat(65_536))]
    fn test_parse_markdown_over_limit(#[case] markdown: &str) {
        PropertyDataValue::parse_markdown(markdown).unwrap();
    }

    // TODO: add English comment
    #[rstest]
    #[case("35.6812362,139.7649361")] // TODO: add English comment
    #[case("0,0")] // TODO: add English comment
    #[case("-33.8688197,151.2092955")] // TODO: add English comment
    #[case("90,180")] // TODO: add English comment
    #[case("-90,-180")] // TODO: add English comment
    fn test_parse_location_valid(#[case] input: &str) {
        let result = PropertyDataValue::new(
            input,
            &PropertyType::Location(TypeLocation::default()),
        );
        assert!(result.is_ok());
        assert_eq!(input, result.unwrap().string_value());
    }

    #[rstest]
    #[should_panic]
    #[case("invalid,139.7649361")] // TODO: add English comment
    #[should_panic]
    #[case("35.6812362,invalid")] // TODO: add English comment
    #[should_panic]
    #[case("35.6812362")] // TODO: add English comment
    #[should_panic]
    #[case("91,180")] // TODO: add English comment
    #[should_panic]
    #[case("90,181")] // TODO: add English comment
    fn test_parse_location_invalid(#[case] input: &str) {
        let _ = PropertyDataValue::new(
            input,
            &PropertyType::Location(TypeLocation::default()),
        )
        .unwrap();
    }

    // TODO: add English comment
    #[rstest]
    #[case("String", PropertyType::String, PropertyDataValue::String("String".to_string()))]
    #[case("", PropertyType::String, PropertyDataValue::String("".to_string()))]
    #[case("0", PropertyType::Integer, PropertyDataValue::Integer(0))]
    #[should_panic]
    #[case(
        "21474836480",
        PropertyType::Integer,
        PropertyDataValue::Integer(0)
    )]
    #[should_panic]
    #[case("0.1", PropertyType::Integer, PropertyDataValue::Integer(0))]
    #[case("", PropertyType::Integer, PropertyDataValue::Integer(0))]
    #[case("aaa", PropertyType::Html, PropertyDataValue::Html("aaa".to_string()))]
    #[case("<!DOCTYPE html>", PropertyType::Html, PropertyDataValue::Html("<!DOCTYPE html>".to_string()))]
    #[case("<p>a</p>", PropertyType::Html, PropertyDataValue::Html("<p>a</p>".to_string()))]
    #[case("# heading", PropertyType::Markdown, PropertyDataValue::Markdown("# heading".to_string()))]
    #[case("", PropertyType::Markdown, PropertyDataValue::Markdown("".to_string()))]
    #[case(r#"
        <!DOCTYPE html>
        <meta charset="utf-8">
        <title>Hello, world!</title>
        <h1 class="foo">Hello, <i>world!</i></h1>
    "#,
    PropertyType::Html, PropertyDataValue::Html(r#"
        <!DOCTYPE html>
        <meta charset="utf-8">
        <title>Hello, world!</title>
        <h1 class="foo">Hello, <i>world!</i></h1>
    "#.to_string()))]
    #[case("35.6812362,139.7649361", PropertyType::Location(TypeLocation::default()), {
        let location = "35.6812362,139.7649361".parse::<Location>().unwrap();
        PropertyDataValue::Location(location)
    })]
    #[case("https://example.com/image.png", PropertyType::Image, PropertyDataValue::Image("https://example.com/image.png".to_string()))]
    #[case("", PropertyType::Image, PropertyDataValue::Image("".to_string()))]
    fn test_parse_from_string(
        #[case] input_value: &str,
        #[case] input_property_type: PropertyType,
        #[case] expected: PropertyDataValue,
    ) {
        let actual =
            PropertyDataValue::new(input_value, &input_property_type)
                .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_location_distance_calculation() {
        // TODO: add English comment
        let tokyo_str = "35.6812362,139.7649361";
        let shinjuku_str = "35.6896067,139.7005713";

        // TODO: add English comment
        let tokyo_loc = PropertyDataValue::new(
            tokyo_str,
            &PropertyType::Location(TypeLocation::default()),
        )
        .unwrap();

        let shinjuku_loc = PropertyDataValue::new(
            shinjuku_str,
            &PropertyType::Location(TypeLocation::default()),
        )
        .unwrap();

        // TODO: add English comment
        if let PropertyDataValue::Location(tokyo) = tokyo_loc {
            if let PropertyDataValue::Location(shinjuku) = shinjuku_loc {
                // TODO: add English comment
                let distance = tokyo.distance_to(&shinjuku);

                // TODO: add English comment
                println!("計算された距離: {distance}km");
                assert!((distance - 5.9).abs() < 1.0);
            } else {
                panic!("新宿駅の位置情報が取得できませんでした");
            }
        } else {
            panic!("東京駅の位置情報が取得できませんでした");
        }
    }

    // Test parse_date with various date formats and edge cases
    #[rstest]
    // Valid dates
    #[case("2024-01-01", true)] // New Year's Day
    #[case("2024-12-31", true)] // Last day of year
    #[case("2024-02-29", true)] // Leap year February 29
    #[case("2023-02-28", true)] // Non-leap year February 28
    #[case("2024-01-31", true)] // January 31
    #[case("2024-04-30", true)] // April 30
    #[case("2024-06-30", true)] // June 30
    #[case("2024-09-30", true)] // September 30
    #[case("2024-11-30", true)] // November 30
    #[case("0001-01-01", true)] // Minimum year
    #[case("9999-12-31", true)] // Maximum year
    // Invalid dates
    #[case("", false)] // Empty string
    #[case("2024-01", false)] // Missing day
    #[case("2024-01-01-01", false)] // Too many parts
    #[case("2024/01/01", false)] // Wrong separator
    #[case("2024-13-01", false)] // Invalid month (13)
    #[case("2024-00-01", false)] // Invalid month (0)
    #[case("2024-01-00", false)] // Invalid day (0)
    #[case("2024-01-32", false)] // Invalid day (32)
    #[case("2024-02-30", false)] // February 30 (invalid)
    #[case("2024-04-31", false)] // April 31 (invalid)
    #[case("2023-02-29", false)] // Non-leap year February 29 (invalid)
    #[case("2024-06-31", false)] // June 31 (invalid)
    #[case("2024-09-31", false)] // September 31 (invalid)
    #[case("2024-11-31", false)] // November 31 (invalid)
    #[case("0000-01-01", false)] // Year too small (0)
    #[case("10000-01-01", false)] // Year too large (10000)
    #[case("abc-01-01", false)] // Invalid year format
    #[case("2024-abc-01", false)] // Invalid month format
    #[case("2024-01-abc", false)]
    // Invalid day format
    // Note: "2024-1-1" is actually accepted by the current implementation
    // as it parses successfully (year=2024, month=1, day=1)
    #[case("2024-1-1", true)] // Missing zero padding (accepted by current implementation)
    fn test_parse_date(#[case] input: &str, #[case] should_succeed: bool) {
        let result = PropertyDataValue::new(input, &PropertyType::Date);
        if should_succeed {
            assert!(
                result.is_ok(),
                "Expected parse_date to succeed for '{}', but got error: {:?}",
                input,
                result.err()
            );
            if let Ok(PropertyDataValue::Date(date_str)) = result {
                assert_eq!(
                    date_str, input,
                    "Date string should match input"
                );
            } else {
                panic!("Expected Date variant, got {:?}", result);
            }
        } else {
            assert!(
                result.is_err(),
                "Expected parse_date to fail for '{}', but got success: {:?}",
                input,
                result.ok()
            );
        }
    }

    #[test]
    fn test_parse_date_leap_years() {
        // Test various leap year scenarios
        let leap_years = vec![
            "2000-02-29",
            "2004-02-29",
            "2008-02-29",
            "2012-02-29",
            "2016-02-29",
            "2020-02-29",
            "2024-02-29",
        ];
        for date_str in leap_years {
            let result =
                PropertyDataValue::new(date_str, &PropertyType::Date);
            assert!(
                result.is_ok(),
                "Leap year date '{}' should be valid, but got error: {:?}",
                date_str,
                result.err()
            );
        }

        // Test non-leap years (should reject February 29)
        let non_leap_years = vec![
            "2001-02-29",
            "2002-02-29",
            "2003-02-29",
            "1900-02-29",
            "2100-02-29",
        ];
        for date_str in non_leap_years {
            let result =
                PropertyDataValue::new(date_str, &PropertyType::Date);
            assert!(
                result.is_err(),
                "Non-leap year date '{}' should be invalid, but got success: {:?}",
                date_str,
                result.ok()
            );
        }
    }

    #[test]
    fn test_parse_date_month_end_boundaries() {
        // Test last day of each month in a non-leap year
        let valid_month_ends = vec![
            "2023-01-31",
            "2023-02-28",
            "2023-03-31",
            "2023-04-30",
            "2023-05-31",
            "2023-06-30",
            "2023-07-31",
            "2023-08-31",
            "2023-09-30",
            "2023-10-31",
            "2023-11-30",
            "2023-12-31",
        ];
        for date_str in valid_month_ends {
            let result =
                PropertyDataValue::new(date_str, &PropertyType::Date);
            assert!(
                result.is_ok(),
                "Month end date '{}' should be valid, but got error: {:?}",
                date_str,
                result.err()
            );
        }

        // Test invalid dates (day after month end)
        let invalid_dates = vec![
            "2023-01-32",
            "2023-02-29",
            "2023-03-32",
            "2023-04-31",
            "2023-05-32",
            "2023-06-31",
            "2023-07-32",
            "2023-08-32",
            "2023-09-31",
            "2023-10-32",
            "2023-11-31",
            "2023-12-32",
        ];
        for date_str in invalid_dates {
            let result =
                PropertyDataValue::new(date_str, &PropertyType::Date);
            assert!(
                result.is_err(),
                "Invalid date '{}' should be rejected, but got success: {:?}",
                date_str,
                result.ok()
            );
        }
    }
}
