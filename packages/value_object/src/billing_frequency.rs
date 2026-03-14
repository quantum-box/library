use strum_macros::{Display, EnumString};

/// Define billing cycle
#[derive(Clone, Debug, PartialEq, Eq, Copy, EnumString, Display)]
#[cfg_attr(feature = "async-graphql", derive(async_graphql::Enum))]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "sqlx", sqlx(rename_all = "SCREAMING_SNAKE_CASE"))]
pub enum RecurringBillingFrequency {
    Monthly,
    Yearly,
    Weekly,
    Daily,
    Once,
    UsageBased,
}

// TODO: add English comment
impl From<String> for RecurringBillingFrequency {
    fn from(val: String) -> Self {
        val.parse().unwrap()
    }
}
