use std::cmp;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "async-graphql", derive(async_graphql::SimpleObject))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "axum", derive(utoipa::ToSchema))]
pub struct OffsetPaginator {
    pub current_page: u32,
    pub items_per_page: u32,
    pub total_items: u32,
    pub total_pages: u32,
}

impl OffsetPaginator {
    pub fn new(
        current_page: u32,
        total_items: u32,
        items_per_page: u32,
    ) -> Self {
        let current_page = cmp::max(1, current_page);
        let total_pages =
            ((total_items as f64) / (items_per_page as f64)).ceil() as u32;

        Self {
            current_page,
            items_per_page,
            total_items,
            total_pages,
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "async-graphql", derive(async_graphql::SimpleObject))]
pub struct CursorPaginator {
    pub current_cursor: String,
    pub has_next_page: bool,
}

impl CursorPaginator {
    pub fn new(current_cursor: String, has_next_page: bool) -> Self {
        Self {
            current_cursor,
            has_next_page,
        }
    }
}
