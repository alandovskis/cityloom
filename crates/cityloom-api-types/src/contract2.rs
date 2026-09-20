//! Contract 2 — design upload and removal (U10 `cityloom-design-storage` →
//! U7 `local-persistence`), `contract-summary.md` § Contract 2.
//!
//! Owned by `cityloom-design-storage` (U10) — `tech-stack-decisions.md`:
//! "`cityloom-api-types`: extended with this Unit's own request/response
//! types (`UploadRequest`, `UploadReceipt`, this Unit's `ApiError`/reason
//! enum per Contract 2)".

use serde::{Deserialize, Serialize};

/// The closed failure set on the wire (`entities.md` `StorageFailure`,
/// Contract 2's `ApiError.reason` enum) — exactly the six values Contract 2
/// names, no extra, no missing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureReason {
    DesignNotFound,
    NotGranted,
    PayloadTooLarge,
    UnsupportedPayloadVersion,
    RateLimited,
    Internal,
}

impl FailureReason {
    /// Every failure reason, in the order Contract 2's `ApiError.reason`
    /// enum lists them.
    pub const ALL: [FailureReason; 6] = [
        FailureReason::DesignNotFound,
        FailureReason::NotGranted,
        FailureReason::PayloadTooLarge,
        FailureReason::UnsupportedPayloadVersion,
        FailureReason::RateLimited,
        FailureReason::Internal,
    ];

    /// The wire spelling (snake_case), the same text `serde` produces.
    pub fn as_str(self) -> &'static str {
        match self {
            FailureReason::DesignNotFound => "design_not_found",
            FailureReason::NotGranted => "not_granted",
            FailureReason::PayloadTooLarge => "payload_too_large",
            FailureReason::UnsupportedPayloadVersion => "unsupported_payload_version",
            FailureReason::RateLimited => "rate_limited",
            FailureReason::Internal => "internal",
        }
    }
}

/// The typed error body every 4xx/5xx answer on this boundary carries
/// (`entities.md` `StorageFailure`).
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

/// `POST /api/designs` request body. `anonymousDesignId` is client-minted
/// from a cryptographic source (`decisions.md` ADR-006); `payload` is
/// Contract 3's `DesignPayload`, carried here as the exact raw JSON text
/// the client sent so storage and re-serving never alter a single byte of
/// it (AC8.1.1).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UploadRequest {
    #[serde(rename = "anonymousDesignId")]
    pub anonymous_design_id: String,
    /// The exact payload bytes, preserved verbatim (see the struct doc).
    pub payload: Box<serde_json::value::RawValue>,
}

/// `POST /api/designs`'s `201` response body (`entities.md`
/// `UploadReceipt`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UploadReceipt {
    #[serde(rename = "anonymousDesignId")]
    pub anonymous_design_id: String,
    #[serde(rename = "uploadedAt")]
    pub uploaded_at: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Contract 2 wire spelling — exactly the six named reasons.
    #[test]
    fn failure_reasons_serialise_to_the_contract_spelling() {
        let expected = [
            (FailureReason::DesignNotFound, "design_not_found"),
            (FailureReason::NotGranted, "not_granted"),
            (FailureReason::PayloadTooLarge, "payload_too_large"),
            (
                FailureReason::UnsupportedPayloadVersion,
                "unsupported_payload_version",
            ),
            (FailureReason::RateLimited, "rate_limited"),
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
    }

    #[test]
    fn api_error_serialises_reason_and_optional_detail() {
        let with = ApiError::new(FailureReason::DesignNotFound, "no such design");
        assert_eq!(
            serde_json::to_string(&with).unwrap(),
            "{\"reason\":\"design_not_found\",\"detail\":\"no such design\"}"
        );
        let without = ApiError::without_detail(FailureReason::Internal);
        assert_eq!(
            serde_json::to_string(&without).unwrap(),
            "{\"reason\":\"internal\"}"
        );
    }

    #[test]
    fn upload_request_round_trips_and_preserves_payload_bytes_verbatim() {
        let json = r#"{"anonymousDesignId":"abc123","payload":{"z":1,"a":2}}"#;
        let parsed: UploadRequest = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.anonymous_design_id, "abc123");
        // The raw payload text is preserved exactly, key order and all —
        // not re-serialized through a `Value` that could reorder it.
        assert_eq!(parsed.payload.get(), r#"{"z":1,"a":2}"#);
    }

    #[test]
    fn upload_receipt_serialises_with_contract_2_field_names() {
        let receipt = UploadReceipt {
            anonymous_design_id: "abc123".to_string(),
            uploaded_at: "2026-09-19T00:00:00Z".to_string(),
            expires_at: "2026-10-19T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&receipt).unwrap();
        assert!(json.contains("\"anonymousDesignId\":\"abc123\""));
        assert!(json.contains("\"uploadedAt\":\"2026-09-19T00:00:00Z\""));
        assert!(json.contains("\"expiresAt\":\"2026-10-19T00:00:00Z\""));
    }
}
