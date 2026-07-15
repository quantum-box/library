/// Whether a type conversion is permitted and what rollout guarantees it
/// requires.
///
/// The kernel never performs an implicit conversion. Even lossless
/// conversions require an explicit conversion command and migration plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionPolicy {
    Identity,
    ExplicitLossless,
    ExplicitValidated,
    ExplicitLossy,
    Forbidden,
}

impl ConversionPolicy {
    pub const fn is_implicit(self) -> bool {
        matches!(self, Self::Identity)
    }
}
