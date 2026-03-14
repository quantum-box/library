/// TODO: add English documentation
/// TODO: add English documentation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
pub struct Percent(f32);

impl Percent {
    /// TODO: add English documentation
    ///
    /// TODO: add English documentation
    pub fn new(percent: u32) -> errors::Result<Self> {
        if percent > 100 {
            errors::type_error("invalid percent");
        }
        Ok(Self(percent as f32))
    }

    /// TODO: add English documentation
    ///
    /// TODO: add English documentation
    pub fn from_rate(rate: f32) -> errors::Result<Self> {
        if !(0.0..=1.0).contains(&rate) {
            errors::type_error("invalid rate");
        }
        Ok(Self(rate * 100.0))
    }

    /// TODO: add English documentation
    ///
    /// TODO: add English documentation
    pub fn to_rate(&self) -> f32 {
        self.0 / 100.0
    }

    /// TODO: add English documentation
    ///
    /// TODO: add English documentation
    pub fn to_percent(&self) -> u32 {
        self.0 as u32
    }
}

impl From<u32> for Percent {
    fn from(val: u32) -> Self {
        Percent::new(val).unwrap()
    }
}

impl PartialEq for Percent {
    fn eq(&self, other: &Self) -> bool {
        (self.0 * 100.0).round() == (other.0 * 100.0).round()
    }
}
