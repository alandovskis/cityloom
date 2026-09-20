//! Contract 1 — extract fetch (U9 `osm-extract-proxy` → U4 `street-import`).
//!
//! `contract-summary.md` § Contract 1 with the amendments `functional-spec.md`
//! records: `invalid_area` added, `upstream_rate_limited` renamed
//! `rate_limited`, `malformed_extract` reserved, `internal` retryable, and
//! (from `security-requirements.md`) the `429` response carrying `Retry-After`.

use serde::{Deserialize, Serialize};

/// The closed failure set on the wire (`entities.md` `FailureReason`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureReason {
    InvalidArea,
    AreaTooLarge,
    AreaNotFound,
    UpstreamUnavailable,
    RateLimited,
    MalformedExtract,
    Timeout,
    Internal,
}

impl FailureReason {
    /// Every failure reason, in the order `entities.md` lists them.
    pub const ALL: [FailureReason; 8] = [
        FailureReason::InvalidArea,
        FailureReason::AreaTooLarge,
        FailureReason::AreaNotFound,
        FailureReason::UpstreamUnavailable,
        FailureReason::RateLimited,
        FailureReason::MalformedExtract,
        FailureReason::Timeout,
        FailureReason::Internal,
    ];

    /// The wire spelling (snake_case), the same text `serde` produces.
    pub fn as_str(self) -> &'static str {
        match self {
            FailureReason::InvalidArea => "invalid_area",
            FailureReason::AreaTooLarge => "area_too_large",
            FailureReason::AreaNotFound => "area_not_found",
            FailureReason::UpstreamUnavailable => "upstream_unavailable",
            FailureReason::RateLimited => "rate_limited",
            FailureReason::MalformedExtract => "malformed_extract",
            FailureReason::Timeout => "timeout",
            FailureReason::Internal => "internal",
        }
    }

    /// Whether a client may retry the same request unchanged (BR10.2):
    /// `upstream_unavailable`, `rate_limited`, `timeout`, `internal`.
    pub fn is_retryable(self) -> bool {
        matches!(
            self,
            FailureReason::UpstreamUnavailable
                | FailureReason::RateLimited
                | FailureReason::Timeout
                | FailureReason::Internal
        )
    }
}

/// The typed error body every 4xx/5xx answer carries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiError {
    pub reason: FailureReason,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl ApiError {
    pub fn new(reason: FailureReason, detail: impl Into<String>) -> ApiError {
        ApiError {
            reason,
            detail: Some(detail.into()),
        }
    }

    pub fn without_detail(reason: FailureReason) -> ApiError {
        ApiError {
            reason,
            detail: None,
        }
    }
}

/// The `426` body: the client build is no longer served; reload.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReloadRequired {
    reason: ReloadReasonTag,
    pub current_build: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReloadReasonTag {
    ReloadRequired,
}

impl ReloadRequired {
    pub fn new(current_build: impl Into<String>) -> ReloadRequired {
        ReloadRequired {
            reason: ReloadReasonTag::ReloadRequired,
            current_build: current_build.into(),
        }
    }
}

impl<'de> Deserialize<'de> for ReloadRequired {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<ReloadRequired, D::Error> {
        #[derive(Deserialize)]
        struct Wire {
            reason: ReloadReasonTag,
            current_build: String,
        }
        let wire = Wire::deserialize(deserializer)?;
        Ok(ReloadRequired {
            reason: wire.reason,
            current_build: wire.current_build,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Contract 1 wire spelling with the functional-spec amendments.
    #[test]
    fn failure_reasons_serialise_to_the_contract_spelling() {
        let expected = [
            (FailureReason::InvalidArea, "invalid_area"),
            (FailureReason::AreaTooLarge, "area_too_large"),
            (FailureReason::AreaNotFound, "area_not_found"),
            (FailureReason::UpstreamUnavailable, "upstream_unavailable"),
            (FailureReason::RateLimited, "rate_limited"),
            (FailureReason::MalformedExtract, "malformed_extract"),
            (FailureReason::Timeout, "timeout"),
            (FailureReason::Internal, "internal"),
        ];
        assert_eq!(FailureReason::ALL.len(), expected.len());
        for (reason, wire) in expected {
            assert_eq!(
                serde_json::to_string(&reason).unwrap(),
                format!("\"{wire}\"")
            );
            assert_eq!(
                serde_json::from_str::<FailureReason>(&format!("\"{wire}\"")).unwrap(),
                reason
            );
            assert_eq!(reason.as_str(), wire);
            assert!(FailureReason::ALL.contains(&reason));
        }
        assert!(
            serde_json::from_str::<FailureReason>("\"upstream_rate_limited\"").is_err(),
            "renamed"
        );
        assert!(serde_json::from_str::<FailureReason>("\"InvalidArea\"").is_err());
    }

    // BR10.2 — retryable: upstream_unavailable, rate_limited, timeout, internal.
    #[test]
    fn is_retryable_matches_br10_2_exactly() {
        assert!(FailureReason::UpstreamUnavailable.is_retryable());
        assert!(FailureReason::RateLimited.is_retryable());
        assert!(FailureReason::Timeout.is_retryable());
        assert!(FailureReason::Internal.is_retryable());
        assert!(!FailureReason::InvalidArea.is_retryable());
        assert!(!FailureReason::AreaTooLarge.is_retryable());
        assert!(!FailureReason::AreaNotFound.is_retryable());
        assert!(!FailureReason::MalformedExtract.is_retryable());
    }

    #[test]
    fn api_error_serialises_reason_and_optional_detail() {
        let with = ApiError::new(FailureReason::AreaNotFound, "nothing mapped here");
        assert_eq!(
            serde_json::to_string(&with).unwrap(),
            "{\"reason\":\"area_not_found\",\"detail\":\"nothing mapped here\"}"
        );
        let without = ApiError {
            reason: FailureReason::Timeout,
            detail: None,
        };
        assert_eq!(
            serde_json::to_string(&without).unwrap(),
            "{\"reason\":\"timeout\"}"
        );
        let parsed: ApiError = serde_json::from_str("{\"reason\":\"internal\"}").unwrap();
        assert_eq!(parsed, without_detail(FailureReason::Internal));
    }

    fn without_detail(reason: FailureReason) -> ApiError {
        ApiError {
            reason,
            detail: None,
        }
    }

    #[test]
    fn reload_required_carries_the_constant_reason_and_current_build() {
        let body = ReloadRequired::new("abc123");
        assert_eq!(
            serde_json::to_string(&body).unwrap(),
            "{\"reason\":\"reload_required\",\"current_build\":\"abc123\"}"
        );
        let parsed: ReloadRequired =
            serde_json::from_str("{\"reason\":\"reload_required\",\"current_build\":\"x\"}")
                .unwrap();
        assert_eq!(parsed.current_build, "x");
        assert!(
            serde_json::from_str::<ReloadRequired>(
                "{\"reason\":\"other\",\"current_build\":\"x\"}"
            )
            .is_err()
        );
    }
}
