//! Idempotent replay classification for replicated operations.
//!
//! A retried operation keeps its id and its content fingerprint, so replaying
//! it is safe. Reusing the same id with different content is a conflict: the
//! stored operation must never be silently overwritten.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayDecision {
    /// The operation id has never been seen.
    New,
    /// The same id was already stored with identical content.
    Idempotent,
    /// The same id was stored with different content.
    PayloadMismatch,
}

impl ReplayDecision {
    pub const fn is_conflict(self) -> bool {
        matches!(self, Self::PayloadMismatch)
    }
}

/// `stored_fingerprint` is recomputed from the persisted operation, never taken
/// from a client supplied hash.
pub fn classify_replay(
    stored_fingerprint: Option<&str>,
    incoming_fingerprint: &str,
) -> ReplayDecision {
    match stored_fingerprint {
        None => ReplayDecision::New,
        Some(stored) if stored == incoming_fingerprint => ReplayDecision::Idempotent,
        Some(_) => ReplayDecision::PayloadMismatch,
    }
}

#[cfg(test)]
mod tests {
    use super::{ReplayDecision, classify_replay};

    #[test]
    fn unseen_operations_are_new() {
        assert_eq!(classify_replay(None, "abc"), ReplayDecision::New);
        assert!(!ReplayDecision::New.is_conflict());
    }

    #[test]
    fn identical_content_is_idempotent() {
        assert_eq!(
            classify_replay(Some("abc"), "abc"),
            ReplayDecision::Idempotent
        );
        assert!(!ReplayDecision::Idempotent.is_conflict());
    }

    #[test]
    fn changed_content_is_a_conflict() {
        let decision = classify_replay(Some("abc"), "abd");
        assert_eq!(decision, ReplayDecision::PayloadMismatch);
        assert!(decision.is_conflict());
    }

    #[test]
    fn an_empty_stored_fingerprint_never_matches_a_real_one() {
        assert_eq!(
            classify_replay(Some(""), "abc"),
            ReplayDecision::PayloadMismatch
        );
    }
}
