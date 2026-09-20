//! `ImportFailure` — the typed, closed failure surface an import can
//! produce (`entities.md` `ImportFailure`; BR2.1, BR3.1).

pub use cityloom_api_types::FailureReason;

/// The typed, closed failure surface an import can produce. Never a
/// dependency's native error text (BR3.1) — the only way to construct one
/// is [`ImportFailure::new`] (or one of the convenience constructors below),
/// which takes a closed [`FailureReason`] and the street's selection-time
/// name; there is no field or constructor path that accepts a raw error
/// string as the failure itself. `retryable` is derived, never supplied by
/// the caller, so it can never drift from BR2.1's rule.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("could not import \"{street_name}\": {reason:?}")]
pub struct ImportFailure {
    reason: FailureReason,
    street_name: String,
    retryable: bool,
}

impl ImportFailure {
    /// Constructs an `ImportFailure` for `reason`, naming the street the
    /// same way it was shown at selection (BR3.1) — never a bare
    /// identifier. `retryable` is computed from `reason` per this Unit's
    /// own BR2.1, not reused from `cityloom_api_types::FailureReason::
    /// is_retryable` — that method implements Contract 1's own BR10.2,
    /// which disagrees with BR2.1 on `internal` (BR10.2 treats a proxy-side
    /// `internal` failure as retryable; BR2.1 does not treat an import-side
    /// `internal` failure as retryable). See `code-summary.md` for this
    /// Unit's record of that discrepancy.
    pub fn new(reason: FailureReason, street_name: impl Into<String>) -> ImportFailure {
        let retryable = matches!(
            reason,
            FailureReason::UpstreamUnavailable
                | FailureReason::RateLimited
                | FailureReason::Timeout
        );
        ImportFailure {
            reason,
            street_name: street_name.into(),
            retryable,
        }
    }

    /// An import abandoned after exceeding its timeout budget (BR1.1).
    pub fn timeout(street_name: impl Into<String>) -> ImportFailure {
        ImportFailure::new(FailureReason::Timeout, street_name)
    }

    /// An extract that could not be parsed or converted (SD-1's
    /// `catch_unwind` boundary; BR3.1).
    pub fn malformed_extract(street_name: impl Into<String>) -> ImportFailure {
        ImportFailure::new(FailureReason::MalformedExtract, street_name)
    }

    /// The closed failure reason.
    pub fn reason(&self) -> FailureReason {
        self.reason
    }

    /// The street's selection-time name (BR3.1) — never a bare identifier.
    pub fn street_name(&self) -> &str {
        &self.street_name
    }

    /// Whether a caller may retry the same request unchanged (BR2.1):
    /// `true` only for `upstream_unavailable`, `rate_limited`, `timeout`.
    pub fn retryable(&self) -> bool {
        self.retryable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test 2 (unit-test-instructions.md): a timeout classifies as
    // `{ reason: timeout, retryable: true }` (BR1.1).
    #[test]
    fn timeout_is_retryable() {
        let failure = ImportFailure::timeout("Northeast Pacific Street");

        assert_eq!(failure.reason(), FailureReason::Timeout);
        assert!(failure.retryable());
    }

    // Test 3: a 429/503-shaped response classifies as
    // `upstream_rate_limited`/`upstream_unavailable`, both retryable.
    #[test]
    fn rate_limited_and_upstream_unavailable_are_retryable() {
        let rate_limited = ImportFailure::new(FailureReason::RateLimited, "Main Street");
        let unavailable = ImportFailure::new(FailureReason::UpstreamUnavailable, "Main Street");

        assert!(rate_limited.retryable());
        assert!(unavailable.retryable());
    }

    // Test 4: a 400/404-shaped response classifies as
    // `area_too_large`/`area_not_found`, both non-retryable (BR2.1).
    #[test]
    fn area_too_large_and_area_not_found_are_not_retryable() {
        let too_large = ImportFailure::new(FailureReason::AreaTooLarge, "Main Street");
        let not_found = ImportFailure::new(FailureReason::AreaNotFound, "Main Street");

        assert!(!too_large.retryable());
        assert!(!not_found.retryable());
    }

    // BR2.1: `internal` and `malformed_extract` are also non-retryable.
    #[test]
    fn internal_and_malformed_extract_are_not_retryable() {
        assert!(!ImportFailure::new(FailureReason::Internal, "Main Street").retryable());
        assert!(!ImportFailure::malformed_extract("Main Street").retryable());
    }

    // Test 6: `street_name` matches the name passed in at construction,
    // never a bare identifier (AC7.1.1).
    #[test]
    fn street_name_matches_the_name_passed_at_construction() {
        let failure = ImportFailure::timeout("Northeast Pacific Street");

        assert_eq!(failure.street_name(), "Northeast Pacific Street");
    }

    // Test 7: no variant of `ImportFailure` can be constructed carrying raw
    // dependency error text — only the closed `FailureReason` plus the
    // street name (AC7.1.2, BR3.1). This is a structural property: `new`'s
    // only non-street-name parameter is the closed `FailureReason` enum,
    // so there is no code path from an arbitrary `String` (a caught panic
    // message or a dependency's `Display` output) into a constructed
    // `ImportFailure` — only into the local log (`log_locally`, SD-1/SD-2),
    // which this type never re-exposes.
    #[test]
    fn every_import_failure_reason_is_one_of_the_closed_set() {
        for reason in FailureReason::ALL {
            let failure = ImportFailure::new(reason, "Main Street");
            assert_eq!(failure.reason(), reason);
        }
    }
}
