/// The canonical storage family used by a Property Type handler.
///
/// This describes domain semantics. A persistence adapter may use a generic
/// envelope and project these classes into separate index storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageClass {
    Text,
    Integer,
    Date,
    Location,
    Reference,
    MultiReference,
}

/// Query and index operations that are meaningful for a Property Type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexCapabilities {
    pub exact: bool,
    pub range: bool,
    pub full_text: bool,
    pub sortable: bool,
    pub unique: bool,
    pub multi_value: bool,
}

impl IndexCapabilities {
    pub const fn scalar(
        range: bool,
        full_text: bool,
        sortable: bool,
        unique: bool,
    ) -> Self {
        Self {
            exact: true,
            range,
            full_text,
            sortable,
            unique,
            multi_value: false,
        }
    }

    pub const fn content() -> Self {
        Self {
            exact: false,
            range: false,
            full_text: true,
            sortable: false,
            unique: false,
            multi_value: false,
        }
    }

    pub const fn multi_reference() -> Self {
        Self {
            exact: true,
            range: false,
            full_text: false,
            sortable: false,
            unique: false,
            multi_value: true,
        }
    }
}
