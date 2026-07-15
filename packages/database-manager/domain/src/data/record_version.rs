use serde::{Deserialize, Serialize};

/// Monotonic Library record revision used for optimistic concurrency.
///
/// This is a Database BC value. It is intentionally separate from Photon
/// Engine clocks, which order collaboration operations rather than persisted
/// Library record revisions.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize,
)]
#[serde(transparent)]
pub struct RecordVersion(u64);

impl RecordVersion {
    pub const INITIAL: Self = Self(1);

    pub fn new(value: u64) -> errors::Result<Self> {
        if value == 0 {
            return Err(errors::Error::invalid(
                "record version must be greater than zero",
            ));
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub fn checked_increment(self) -> errors::Result<Self> {
        self.0.checked_add(1).map(Self).ok_or_else(|| {
            errors::Error::conflict(
                "record version cannot be incremented beyond u64::MAX",
            )
        })
    }
}

impl Default for RecordVersion {
    fn default() -> Self {
        Self::INITIAL
    }
}

impl std::fmt::Display for RecordVersion {
    fn fmt(
        &self,
        formatter: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

impl<'de> Deserialize<'de> for RecordVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u64::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_nonzero_and_increments_without_wrapping() {
        assert!(RecordVersion::new(0).is_err());
        assert_eq!(RecordVersion::INITIAL.get(), 1);
        assert_eq!(
            RecordVersion::new(41)
                .expect("valid version")
                .checked_increment()
                .expect("increment")
                .get(),
            42
        );

        let error = RecordVersion::new(u64::MAX)
            .expect("maximum version is representable")
            .checked_increment()
            .expect_err("version increment must never wrap");
        assert!(matches!(error, errors::Error::Conflict { .. }));
    }

    #[test]
    fn deserialization_rejects_zero() {
        assert_eq!(
            serde_json::from_str::<RecordVersion>("7")
                .expect("valid serialized version")
                .get(),
            7
        );
        assert!(serde_json::from_str::<RecordVersion>("0").is_err());
    }
}
