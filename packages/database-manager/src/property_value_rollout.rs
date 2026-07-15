/// Safe rollout states for normalized PropertyValue storage.
///
/// Canonical-only writes are intentionally absent until backfill, parity and
/// the rollback window are complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PropertyValueStorageMode {
    #[default]
    LegacyOnly,
    DualWriteLegacyRead,
    DualWriteCanonicalRead,
}

impl PropertyValueStorageMode {
    pub const fn writes_canonical(self) -> bool {
        !matches!(self, Self::LegacyOnly)
    }

    pub const fn reads_canonical_first(self) -> bool {
        matches!(self, Self::DualWriteCanonicalRead)
    }

    pub const fn reads_or_shadows_canonical(self) -> bool {
        !matches!(self, Self::LegacyOnly)
    }
}

impl std::str::FromStr for PropertyValueStorageMode {
    type Err = errors::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "legacy_only" => Ok(Self::LegacyOnly),
            "dual_write_legacy_read" => Ok(Self::DualWriteLegacyRead),
            "dual_write_canonical_read" => Ok(Self::DualWriteCanonicalRead),
            _ => Err(errors::Error::invalid(format!(
                "unknown PropertyValue storage mode {value}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_only_is_not_a_rollout_state() {
        assert!("canonical_only"
            .parse::<PropertyValueStorageMode>()
            .is_err());
    }

    #[test]
    fn storage_defaults_to_legacy_only() {
        assert_eq!(
            PropertyValueStorageMode::default(),
            PropertyValueStorageMode::LegacyOnly
        );
    }

    #[test]
    fn legacy_only_does_not_read_or_shadow_canonical_values() {
        assert!(!PropertyValueStorageMode::LegacyOnly
            .reads_or_shadows_canonical());
        assert!(PropertyValueStorageMode::DualWriteLegacyRead
            .reads_or_shadows_canonical());
    }
}
