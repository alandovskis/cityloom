//! `failure` — the one table mapping every [`FailureReason`] to its HTTP
//! status, retryability and project-authored detail (BR10.1, BR10.2), and
//! the [`FailureRecord`] shape the log carries (BR7.2, BR10.1). Pure: no
//! HTTP types, no I/O, no logging — `http.rs` reads the status code from
//! here and `emit.rs` writes the record this module builds.

use cityloom_api_types::FailureReason;

/// The phase in which a request failed (BR7.2); `http.rs` names the exact
/// phase, `unexpected` is BR10.4's catch-all for a fault the design did not
/// anticipate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    BuildStamp,
    RateLimit,
    Validation,
    Coverage,
    Clip,
    Readiness,
    Unexpected,
}

impl Phase {
    pub fn as_str(self) -> &'static str {
        match self {
            Phase::BuildStamp => "build_stamp",
            Phase::RateLimit => "rate_limit",
            Phase::Validation => "validation",
            Phase::Coverage => "coverage",
            Phase::Clip => "clip",
            Phase::Readiness => "readiness",
            Phase::Unexpected => "unexpected",
        }
    }
}

/// One row of the failure mapping table (BR10.1): the HTTP status and the
/// project-authored detail text for a `FailureReason`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FailureMapping {
    pub status: u16,
    pub detail: &'static str,
}

/// The closed mapping BR10.1 requires: every `FailureReason` to exactly one
/// status and one project-authored detail. `http.rs`'s handlers never write
/// their own status codes or detail text; they look it up here.
pub fn mapping(reason: FailureReason) -> FailureMapping {
    match reason {
        FailureReason::InvalidArea => FailureMapping {
            status: 400,
            detail: "the area could not be read",
        },
        FailureReason::AreaTooLarge => FailureMapping {
            status: 400,
            detail: "the area covers too many cells",
        },
        FailureReason::AreaNotFound => FailureMapping {
            status: 404,
            detail: "not an area this deployment covers",
        },
        FailureReason::UpstreamUnavailable => FailureMapping {
            status: 503,
            detail: "the service is not ready to serve extracts",
        },
        FailureReason::RateLimited => FailureMapping {
            status: 429,
            detail: "too many requests",
        },
        FailureReason::MalformedExtract => FailureMapping {
            status: 503,
            detail: "the extract could not be assembled",
        },
        FailureReason::Timeout => FailureMapping {
            status: 503,
            detail: "the request did not complete in time",
        },
        FailureReason::Internal => FailureMapping {
            status: 503,
            detail: "an unexpected error occurred",
        },
    }
}

/// The "nothing mapped here" variant of `area_not_found` (BR4.2), distinct
/// from "not an area this deployment covers" (BR4.1) though both carry the
/// same reason and status.
pub const NOTHING_MAPPED_DETAIL: &str = "nothing mapped here";

/// Exactly the fields BR7.2 allows in a failure log entry: no address, no
/// box, no key (BR7.1) — `occurred_at` is filled by `emit.rs` at write time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FailureRecord {
    pub reason: FailureReason,
    pub status: u16,
    pub phase: Phase,
    pub detail: &'static str,
}

impl FailureRecord {
    pub fn new(reason: FailureReason, phase: Phase) -> FailureRecord {
        let m = mapping(reason);
        FailureRecord {
            reason,
            status: m.status,
            phase,
            detail: m.detail,
        }
    }

    /// The BR4.2 variant, whose detail differs from the reason's default.
    pub fn nothing_mapped(phase: Phase) -> FailureRecord {
        FailureRecord {
            reason: FailureReason::AreaNotFound,
            status: 404,
            phase,
            detail: NOTHING_MAPPED_DETAIL,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // BR10.1 / BR10.2 — every reason maps to exactly its status and
    // retryability from functional-spec.md's table.
    #[test]
    fn every_reason_maps_to_its_status_and_retryability() {
        let expected: &[(FailureReason, u16, bool)] = &[
            (FailureReason::InvalidArea, 400, false),
            (FailureReason::AreaTooLarge, 400, false),
            (FailureReason::AreaNotFound, 404, false),
            (FailureReason::UpstreamUnavailable, 503, true),
            (FailureReason::RateLimited, 429, true),
            (FailureReason::MalformedExtract, 503, false),
            (FailureReason::Timeout, 503, true),
            (FailureReason::Internal, 503, true),
        ];
        assert_eq!(expected.len(), FailureReason::ALL.len());
        for (reason, status, retryable) in expected.iter().copied() {
            assert_eq!(mapping(reason).status, status, "{reason:?}");
            assert_eq!(reason.is_retryable(), retryable, "{reason:?}");
        }
    }

    // Every detail is project-authored: plain ASCII prose, never empty, and
    // never a dependency's own error text (BR10.1).
    #[test]
    fn every_detail_is_non_empty_project_prose() {
        for reason in FailureReason::ALL {
            let detail = mapping(reason).detail;
            assert!(!detail.is_empty());
            assert!(detail.is_ascii());
        }
    }

    // BR7.2 / BR10.1 — a record carries exactly reason, status, phase, detail
    // (occurred_at is added by the caller at write time, not stored here).
    #[test]
    fn a_record_carries_exactly_the_allowed_fields() {
        let record = FailureRecord::new(FailureReason::Timeout, Phase::Clip);
        assert_eq!(record.reason, FailureReason::Timeout);
        assert_eq!(record.status, 503);
        assert_eq!(record.phase, Phase::Clip);
        assert_eq!(record.detail, "the request did not complete in time");
    }

    // BR4.2 — the "nothing mapped" variant shares reason/status with
    // area_not_found but carries its own detail.
    #[test]
    fn nothing_mapped_shares_reason_with_area_not_found_but_not_detail() {
        let empty = FailureRecord::nothing_mapped(Phase::Clip);
        let uncovered = FailureRecord::new(FailureReason::AreaNotFound, Phase::Coverage);
        assert_eq!(empty.reason, uncovered.reason);
        assert_eq!(empty.status, uncovered.status);
        assert_ne!(empty.detail, uncovered.detail);
    }

    #[test]
    fn every_phase_has_a_stable_wire_name() {
        let phases = [
            Phase::BuildStamp,
            Phase::RateLimit,
            Phase::Validation,
            Phase::Coverage,
            Phase::Clip,
            Phase::Readiness,
            Phase::Unexpected,
        ];
        let mut names: Vec<&str> = phases.iter().map(|p| p.as_str()).collect();
        let before = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), before, "phase names must be unique");
    }
}
