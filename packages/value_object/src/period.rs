use chrono::{DateTime, Days, FixedOffset, Months, NaiveDate};
use errors::Error;
use std::fmt;

/// TODO: add English documentation
///
/// TODO: add English documentation
///
/// ```
/// use value_object::Period;
/// use std::str::FromStr;
/// use chrono::NaiveDate;
///
/// let period = Period::from_str("2023-01-01 - 2023-12-31").unwrap();
/// assert_eq!(period.start_date(), NaiveDate::from_ymd(2023, 1, 1));
/// assert_eq!(period.end_date(), NaiveDate::from_ymd(2023, 12, 31));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Period {
    start_date: NaiveDate,
    end_date: NaiveDate,
}

impl Period {
    pub fn new(
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Self, Error> {
        if start_date > end_date {
            return Err(Error::type_error(
                "Start date must be before or equal to end date",
            ));
        }
        Ok(Self {
            start_date,
            end_date,
        })
    }

    pub fn start_date(&self) -> NaiveDate {
        self.start_date
    }

    pub fn end_date(&self) -> NaiveDate {
        self.end_date
    }

    /// TODO: add English documentation
    ///
    /// TODO: add English documentation
    ///
    /// ```
    /// use value_object::Period;
    /// use std::str::FromStr;
    /// use chrono::{NaiveDate, DateTime, FixedOffset};
    ///
    /// let period = Period::from_str("2023-01-01 - 2023-12-31").unwrap();
    /// assert_eq!(period.start_date_as_datetime().to_rfc3339(), "2023-01-01T00:00:00+09:00");
    /// ```
    pub fn start_date_as_datetime(&self) -> DateTime<FixedOffset> {
        self.start_date
            .and_hms_opt(0, 0, 0)
            .expect("Invalid date")
            .and_local_timezone(FixedOffset::east_opt(9 * 3600).unwrap())
            .unwrap()
    }

    /// TODO: add English documentation
    ///
    /// TODO: add English documentation
    ///
    /// ```
    /// use value_object::Period;
    /// use std::str::FromStr;
    /// use chrono::{NaiveDate, DateTime, FixedOffset};
    ///
    /// let period = Period::from_str("2023-01-01 - 2023-12-31").unwrap();
    /// assert_eq!(period.end_date_as_datetime().to_rfc3339(), "2023-12-31T00:00:00+09:00");
    /// ```
    pub fn end_date_as_datetime(&self) -> DateTime<FixedOffset> {
        self.end_date
            .and_hms_opt(0, 0, 0)
            .expect("Invalid date")
            .and_local_timezone(FixedOffset::east_opt(9 * 3600).unwrap())
            .unwrap()
    }

    /// TODO: add English documentation
    ///
    /// TODO: add English documentation
    ///
    /// ```
    /// use value_object::Period;
    /// use std::str::FromStr;
    /// use chrono::NaiveDate;
    ///
    /// let period = Period::from_str("2023-01-01 - 2023-12-31").unwrap();
    /// assert_eq!(period.shift_by_days(1).start_date(), NaiveDate::from_ymd_opt(2023, 1, 2).unwrap());
    /// assert_eq!(period.shift_by_days(1).end_date(), NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
    /// ```
    ///
    /// ```
    /// use value_object::Period;
    /// use std::str::FromStr;
    /// use chrono::NaiveDate;
    ///
    /// let period = Period::from_str("2023-01-01 - 2023-12-31").unwrap();
    /// assert_eq!(period.shift_by_days(-1).start_date(), NaiveDate::from_ymd_opt(2022, 12, 31).unwrap());
    /// assert_eq!(period.shift_by_days(-1).end_date(), NaiveDate::from_ymd_opt(2023, 12, 30).unwrap());
    /// ```
    pub fn shift_by_days(&self, days: i64) -> Self {
        if days > 0 {
            let days = Days::new(days as u64);
            Self {
                start_date: self.start_date.checked_add_days(days).unwrap(),
                end_date: self.end_date.checked_add_days(days).unwrap(),
            }
        } else {
            let days = Days::new(-days as u64);
            Self {
                start_date: self.start_date.checked_sub_days(days).unwrap(),
                end_date: self.end_date.checked_sub_days(days).unwrap(),
            }
        }
    }

    /// TODO: add English documentation
    ///
    /// TODO: add English documentation
    ///
    /// ```
    /// use value_object::Period;
    /// use std::str::FromStr;
    /// use chrono::NaiveDate;
    ///
    /// let period = Period::from_str("2023-01-01 - 2023-12-31").unwrap();
    /// assert_eq!(period.shift_by_weeks(1).start_date(), NaiveDate::from_ymd_opt(2023, 1, 8).unwrap());
    /// assert_eq!(period.shift_by_weeks(1).end_date(), NaiveDate::from_ymd_opt(2024, 1, 7).unwrap());
    /// ```
    pub fn shift_by_weeks(&self, weeks: i32) -> Self {
        self.shift_by_days((weeks as i64) * 7)
    }

    /// TODO: add English documentation
    ///
    /// TODO: add English documentation
    ///
    /// ```
    /// use value_object::Period;
    /// use std::str::FromStr;
    /// use chrono::NaiveDate;
    ///
    /// let period = Period::from_str("2023-01-01 - 2023-12-31").unwrap();
    /// assert_eq!(period.shift_by_months(1).start_date(), NaiveDate::from_ymd_opt(2023, 2, 1).unwrap());
    /// assert_eq!(period.shift_by_months(1).end_date(), NaiveDate::from_ymd_opt(2024, 1, 31).unwrap());
    /// ```
    ///
    /// ```
    /// use value_object::Period;
    /// use std::str::FromStr;
    /// use chrono::NaiveDate;
    ///
    /// let period = Period::from_str("2023-01-01 - 2023-12-31").unwrap();
    /// assert_eq!(period.shift_by_months(-1).start_date(), NaiveDate::from_ymd_opt(2022, 12, 1).unwrap());
    /// assert_eq!(period.shift_by_months(-1).end_date(), NaiveDate::from_ymd_opt(2023, 11, 30).unwrap());
    /// ```
    pub fn shift_by_months(&self, months: i32) -> Self {
        if months > 0 {
            let months = Months::new(months as u32);
            Self {
                start_date: self
                    .start_date
                    .checked_add_months(months)
                    .unwrap(),
                end_date: self.end_date.checked_add_months(months).unwrap(),
            }
        } else {
            let months = Months::new(-months as u32);
            Self {
                start_date: self
                    .start_date
                    .checked_sub_months(months)
                    .unwrap(),
                end_date: self.end_date.checked_sub_months(months).unwrap(),
            }
        }
    }
}

impl fmt::Display for Period {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} - {}", self.start_date, self.end_date)
    }
}

impl std::str::FromStr for Period {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let dates: Vec<&str> = s.split(" - ").collect();
        if dates.len() != 2 {
            return Err(Error::type_error("Invalid period format"));
        }
        let start_date = NaiveDate::parse_from_str(dates[0], "%Y-%m-%d")
            .map_err(|e| Error::type_error(e.to_string()))?;
        let end_date = NaiveDate::parse_from_str(dates[1], "%Y-%m-%d")
            .map_err(|e| Error::type_error(e.to_string()))?;
        Period::new(start_date, end_date)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_period_creation() {
        let start_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2023, 12, 31).unwrap();
        let period = Period::new(start_date, end_date).unwrap();
        assert_eq!(period.start_date(), start_date);
        assert_eq!(period.end_date(), end_date);
    }

    #[test]
    fn test_period_invalid_creation() {
        let start_date = NaiveDate::from_ymd_opt(2023, 12, 31).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let period = Period::new(start_date, end_date);
        assert!(period.is_err());
    }

    #[test]
    fn test_period_from_str() {
        let period = Period::from_str("2023-01-01 - 2023-12-31").unwrap();
        assert_eq!(
            period.start_date(),
            NaiveDate::from_ymd_opt(2023, 1, 1).unwrap()
        );
        assert_eq!(
            period.end_date(),
            NaiveDate::from_ymd_opt(2023, 12, 31).unwrap()
        );
    }

    #[test]
    fn test_period_invalid_from_str() {
        let period = Period::from_str("2023-01-01 to 2023-12-31");
        assert!(period.is_err());
    }
}
