//! `failure` — the `StorageFailure` type (`entities.md`), its HTTP status
//! mapping, and the one function mapping a `sqlx::Error` to it
//! (`security-design.md` SD-6): no `sqlx::Error`'s `Display` output, and no
//! raw Postgres error message, ever reaches a response body.

use thiserror::Error;

use cityloom_api_types::contract2::{ApiError, FailureReason};

/// This Unit's own closed failure surface (`entities.md` `StorageFailure`),
/// mirroring Contract 2's `ApiError`/`FailureReason` exactly. `detail` is
/// always project-authored text, never a dependency's native error text
/// (BR6.1, `team.md` Code Style).
#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[error("{reason:?}: {detail}")]
pub struct StorageFailure {
    pub reason: FailureReason,
    pub detail: &'static str,
}

impl StorageFailure {
    pub fn new(reason: FailureReason, detail: &'static str) -> StorageFailure {
        StorageFailure { reason, detail }
    }

    /// The HTTP status this reason answers with (Contract 2).
    pub fn status(&self) -> u16 {
        status_for(self.reason)
    }

    /// The wire body this failure answers with — `detail` carried through
    /// as project-authored text (never a raw driver error).
    pub fn to_api_error(&self) -> ApiError {
        ApiError::new(self.reason, self.detail)
    }
}

/// The HTTP status for each of Contract 2's six reasons.
pub fn status_for(reason: FailureReason) -> u16 {
    match reason {
        FailureReason::DesignNotFound => 404,
        FailureReason::NotGranted => 403,
        FailureReason::PayloadTooLarge => 413,
        FailureReason::UnsupportedPayloadVersion => 400,
        FailureReason::RateLimited => 429,
        FailureReason::Internal => 503,
    }
}

/// SD-6 — every `sqlx::Error` the pool or a query can return is mapped,
/// once, in this one function, to `StorageFailure { reason: internal }`
/// with a project-authored `detail`; the underlying error is logged by the
/// caller (with the operation name only, never re-derived identifiers),
/// never surfaced in the response.
///
/// The upload-conflict (`0` rows affected on `INSERT ... ON CONFLICT
/// (anonymous_design_id) DO NOTHING`) is *not* an error under this
/// mapping — `performance-design.md` PD-1 / `security-design.md` SD-6 both
/// state it is an expected, named outcome read from the statement's own
/// rows-affected count, handled by the caller before this function is ever
/// reached. No unique-violation `sqlx::Error` is caught or expected here,
/// because `ON CONFLICT DO NOTHING` never raises one.
pub fn from_sqlx_error(_error: &sqlx::Error) -> StorageFailure {
    #[cfg(test)]
    eprintln!("DEBUG sqlx error: {_error:?}");
    StorageFailure::new(FailureReason::Internal, "an unexpected error occurred")
}

#[cfg(test)]
mod tests {
    use super::*;

    // `entities.md` — StorageFailure's `reason` enum has exactly the six
    // values Contract 2's `ApiError` reason enum names.
    #[test]
    fn storage_failure_reason_has_exactly_contract_2s_six_values() {
        assert_eq!(FailureReason::ALL.len(), 6);
        let expected = [
            "design_not_found",
            "not_granted",
            "payload_too_large",
            "unsupported_payload_version",
            "rate_limited",
            "internal",
        ];
        let mut actual: Vec<&str> = FailureReason::ALL.iter().map(|r| r.as_str()).collect();
        let mut expected = expected.to_vec();
        actual.sort_unstable();
        expected.sort_unstable();
        assert_eq!(actual, expected);
    }

    #[test]
    fn every_reason_maps_to_the_contract_2_status() {
        let expected: &[(FailureReason, u16)] = &[
            (FailureReason::DesignNotFound, 404),
            (FailureReason::NotGranted, 403),
            (FailureReason::PayloadTooLarge, 413),
            (FailureReason::UnsupportedPayloadVersion, 400),
            (FailureReason::RateLimited, 429),
            (FailureReason::Internal, 503),
        ];
        for (reason, status) in expected.iter().copied() {
            assert_eq!(status_for(reason), status, "{reason:?}");
        }
    }

    // SD-6 — every sqlx::Error variant maps to `internal` with no driver
    // text leaked into `detail`.
    #[test]
    fn every_sqlx_error_variant_maps_to_internal_with_no_driver_text_leaked() {
        let cases: Vec<sqlx::Error> = vec![
            sqlx::Error::RowNotFound,
            sqlx::Error::ColumnNotFound("nonexistent_column_xyz".to_string()),
            sqlx::Error::Protocol("some raw protocol detail".to_string()),
            sqlx::Error::PoolTimedOut,
            sqlx::Error::PoolClosed,
        ];
        for error in &cases {
            let failure = from_sqlx_error(error);
            assert_eq!(failure.reason, FailureReason::Internal);
            assert_eq!(failure.detail, "an unexpected error occurred");
            // The mapped detail never contains the driver's own text.
            assert!(!failure.detail.contains("nonexistent_column_xyz"));
            assert!(!failure.detail.contains("some raw protocol detail"));
        }
    }

    #[test]
    fn to_api_error_carries_the_reason_and_detail_through() {
        let failure = StorageFailure::new(FailureReason::DesignNotFound, "no such design");
        let api_error = failure.to_api_error();
        assert_eq!(api_error.reason, FailureReason::DesignNotFound);
        assert_eq!(api_error.detail.as_deref(), Some("no such design"));
    }
}
