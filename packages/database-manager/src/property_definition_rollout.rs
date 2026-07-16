/// Read precedence for the always-dual-written PropertyDefinition envelope.
///
/// New writes populate both the legacy `datatype`/`datatype_meta` columns and
/// the canonical `type_*` envelope in one statement. The default keeps legacy
/// reads authoritative until mixed-fleet writers are drained and parity has
/// been checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PropertyDefinitionStorageMode {
    #[default]
    DualWriteLegacyRead,
    DualWriteCanonicalRead,
}

impl PropertyDefinitionStorageMode {
    pub const fn reads_canonical_first(self) -> bool {
        matches!(self, Self::DualWriteCanonicalRead)
    }
}

impl std::str::FromStr for PropertyDefinitionStorageMode {
    type Err = errors::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "dual_write_legacy_read" => Ok(Self::DualWriteLegacyRead),
            "dual_write_canonical_read" => Ok(Self::DualWriteCanonicalRead),
            _ => Err(errors::Error::invalid(format!(
                "unknown PropertyDefinition storage mode {value}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollout_defaults_to_dual_write_with_legacy_reads() {
        assert_eq!(
            PropertyDefinitionStorageMode::default(),
            PropertyDefinitionStorageMode::DualWriteLegacyRead
        );
        assert!(!PropertyDefinitionStorageMode::default()
            .reads_canonical_first());
    }

    #[test]
    fn legacy_only_is_not_a_new_writer_mode() {
        assert!("legacy_only"
            .parse::<PropertyDefinitionStorageMode>()
            .is_err());
    }
}
