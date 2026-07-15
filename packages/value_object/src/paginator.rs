pub const FIRST_PAGE: u32 = 1;
pub const DEFAULT_PAGE_SIZE: u32 = 20;
pub const MAX_PAGE_SIZE: u32 = 100;

/// A validated, 1-origin offset pagination request.
///
/// Keeping the offset calculation here prevents repositories and query
/// services from interpreting page numbers differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OffsetPage {
    current_page: u32,
    items_per_page: u32,
    offset: u32,
}

impl OffsetPage {
    pub fn from_options(
        current_page: Option<u32>,
        items_per_page: Option<u32>,
    ) -> errors::Result<Self> {
        Self::new(
            current_page.unwrap_or(FIRST_PAGE),
            items_per_page.unwrap_or(DEFAULT_PAGE_SIZE),
        )
    }

    pub fn new(
        current_page: u32,
        items_per_page: u32,
    ) -> errors::Result<Self> {
        let zero_based_page =
            current_page.checked_sub(1).ok_or_else(|| {
                errors::Error::bad_request("page must start at 1")
            })?;
        if !(1..=MAX_PAGE_SIZE).contains(&items_per_page) {
            return Err(errors::Error::bad_request(format!(
                "page_size must be between 1 and {MAX_PAGE_SIZE}"
            )));
        }
        let offset = zero_based_page
            .checked_mul(items_per_page)
            .ok_or_else(|| {
                errors::Error::bad_request(
                    "page and page_size produce an offset that is too large",
                )
            })?;

        Ok(Self {
            current_page,
            items_per_page,
            offset,
        })
    }

    pub fn current_page(self) -> u32 {
        self.current_page
    }

    pub fn items_per_page(self) -> u32 {
        self.items_per_page
    }

    pub fn offset(self) -> u32 {
        self.offset
    }
}

/// Metadata for a validated [`OffsetPage`].
///
/// `current_page` is never clamped to the last page. An out-of-range request
/// therefore returns an empty item list with the requested page preserved.
/// Empty result sets have zero total pages.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "async-graphql", derive(async_graphql::SimpleObject))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "axum", derive(utoipa::ToSchema))]
pub struct OffsetPaginator {
    #[cfg_attr(feature = "axum", schema(minimum = 1))]
    pub current_page: u32,
    #[cfg_attr(feature = "axum", schema(minimum = 1, maximum = 100))]
    pub items_per_page: u32,
    pub total_items: u32,
    pub total_pages: u32,
}

impl OffsetPaginator {
    pub fn new(page: OffsetPage, total_items: u32) -> Self {
        let items_per_page = page.items_per_page();
        let total_pages = total_items.div_ceil(items_per_page);

        Self {
            current_page: page.current_page(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_one_origin_and_bounded() {
        let page = OffsetPage::from_options(None, None).unwrap();

        assert_eq!(page.current_page(), FIRST_PAGE);
        assert_eq!(page.items_per_page(), DEFAULT_PAGE_SIZE);
        assert_eq!(page.offset(), 0);
    }

    #[test]
    fn offset_is_calculated_once_from_the_one_origin_page() {
        let page = OffsetPage::new(3, 20).unwrap();

        assert_eq!(page.offset(), 40);
    }

    #[test]
    fn page_zero_is_rejected() {
        let error = OffsetPage::new(0, 20).unwrap_err();

        assert!(error.is_bad_request());
        assert_eq!(error.to_string(), "BadRequest: page must start at 1");
    }

    #[test]
    fn page_size_must_be_nonzero_and_within_the_maximum() {
        assert!(OffsetPage::new(1, MAX_PAGE_SIZE).is_ok());

        for page_size in [0, MAX_PAGE_SIZE + 1] {
            let error = OffsetPage::new(1, page_size).unwrap_err();

            assert!(error.is_bad_request());
            assert_eq!(
                error.to_string(),
                format!(
                    "BadRequest: page_size must be between 1 and {MAX_PAGE_SIZE}"
                )
            );
        }
    }

    #[test]
    fn offset_overflow_is_rejected() {
        let error = OffsetPage::new(u32::MAX, MAX_PAGE_SIZE).unwrap_err();

        assert!(error.is_bad_request());
        assert_eq!(
            error.to_string(),
            "BadRequest: page and page_size produce an offset that is too large"
        );
    }

    #[test]
    fn paginator_preserves_the_request_and_rounds_up_the_last_page() {
        let paginator =
            OffsetPaginator::new(OffsetPage::new(3, 20).unwrap(), 50);

        assert_eq!(paginator.current_page, 3);
        assert_eq!(paginator.items_per_page, 20);
        assert_eq!(paginator.total_items, 50);
        assert_eq!(paginator.total_pages, 3);
    }
}
