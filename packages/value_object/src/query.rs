use derive_getters::Getters;

#[derive(Clone, Debug, Getters)]
#[cfg_attr(feature = "async-graphql", derive(async_graphql::InputObject))]
#[cfg_attr(
    feature = "axum",
    derive(utoipa::IntoParams, utoipa::ToSchema, serde::Deserialize)
)]
#[cfg_attr(feature = "axum", into_params(parameter_in = Query))]
pub struct Queries {
    /// Page size
    ///
    /// default 20
    pub limit: Option<u32>,

    /// Page number
    ///
    /// default 0
    pub offset: Option<u32>,

    /// Search word
    /// e.g.) name, title
    ///
    /// default empty string
    pub search: Option<String>,
}

impl Default for Queries {
    fn default() -> Self {
        Self {
            limit: Some(20),
            offset: Some(0),
            search: Some(String::new()),
        }
    }
}

// #[derive(OneofObject, Debug, Clone)]
// pub enum Query {
//     Offset(u32),
//     Limit(u32),
// }

// impl Queries {
//     pub fn offset(&self) -> Option<u32> {
//         self.value.iter().find_map(|v| match v {
//             Query::Offset(v) => Some(*v),
//             _ => None,
//         })
//     }

//     pub fn limit(&self) -> Option<u32> {
//         self.value.iter().find_map(|v| match v {
//             Query::Limit(v) => Some(*v),
//             _ => None,
//         })
//     }
// }
