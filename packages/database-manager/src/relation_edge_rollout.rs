/// Internal rollout state for normalized RelationEdge writes.
///
/// This slice deliberately exposes no environment parser or `apps/api`
/// wiring. Production construction therefore remains disabled until the
/// create/delete writers, backfill checkpoint, parity gate, and mixed-fleet
/// drain are complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum RelationEdgeWriteMode {
    #[default]
    Disabled,
    #[cfg(test)]
    DualWriteLegacyRead,
}

impl RelationEdgeWriteMode {
    pub(crate) const fn writes_edges(self) -> bool {
        match self {
            Self::Disabled => false,
            #[cfg(test)]
            Self::DualWriteLegacyRead => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_default_has_no_edge_writer() {
        assert_eq!(
            RelationEdgeWriteMode::default(),
            RelationEdgeWriteMode::Disabled
        );
        assert!(!RelationEdgeWriteMode::default().writes_edges());
    }
}
