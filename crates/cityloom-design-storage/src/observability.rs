//! `observability` — the counter and log-row emission points
//! (`observability-design.md` OD-1 through OD-4): a failed operation emits
//! exactly one `tracing` JSON row built from a fixed field set (`operation`,
//! `reason`, `status`) — never the payload, the raw `anonymous_design_id`
//! value, or a requester-identifying field. A successful operation emits no
//! row at all.

use std::sync::atomic::{AtomicU64, Ordering};

use tracing_subscriber::fmt::MakeWriter;

use cityloom_api_types::contract2::FailureReason;

/// Build this crate's `tracing` subscriber: one flat JSON object per event,
/// no span-open/close events — the same shape `osm-extract-proxy`'s own
/// `emit::subscriber` already established for this workspace.
pub fn subscriber<W>(writer: W) -> impl tracing::Subscriber + Send + Sync
where
    W: for<'w> MakeWriter<'w> + Send + Sync + 'static,
{
    tracing_subscriber::fmt()
        .json()
        .flatten_event(true)
        .with_target(false)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
        .with_writer(writer)
        .finish()
}

/// The three operations this Unit's endpoints perform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Upload,
    Fetch,
    Remove,
}

impl Operation {
    pub fn as_str(self) -> &'static str {
        match self {
            Operation::Upload => "upload",
            Operation::Fetch => "fetch",
            Operation::Remove => "remove",
        }
    }
}

/// OD-1 — one failure row, built from this fixed field set only: never the
/// payload, the raw `anonymous_design_id`, or a requester-identifying
/// field. `security-design.md` SD-7's "nothing to leak" extends to
/// logging — the identifier genuinely never reaches this function, not
/// merely a value it chooses not to log.
pub fn record_failure(operation: Operation, reason: FailureReason, status: u16) {
    if status >= 500 {
        tracing::error!(
            event = "storage_failure",
            operation = operation.as_str(),
            reason = reason.as_str(),
            status = status,
        );
    } else {
        tracing::warn!(
            event = "storage_failure",
            operation = operation.as_str(),
            reason = reason.as_str(),
            status = status,
        );
    }
}

/// OD-3 — one aggregate row per sweep *cycle* (not per batch): exactly
/// `rows_examined`, `rows_deleted`, `duration_ms` — never the identifiers
/// of the rows deleted.
pub fn record_sweep_cycle(rows_examined: i64, rows_deleted: i64, duration_ms: u128) {
    tracing::info!(
        event = "sweep_cycle",
        rows_examined = rows_examined,
        rows_deleted = rows_deleted,
        duration_ms = duration_ms as u64,
    );
}

/// The number of `FailureReason` variants (`entities.md` `StorageFailure` —
/// pinned to exactly six by `failure.rs`'s own test).
const FAILURE_REASON_COUNT: usize = 6;

/// `NFR7.4.2` / OD-2 — aggregate counters, exported the same
/// stdout-JSON-rows way `osm-extract-proxy` already established
/// (`counters.rs` there is this module's precedent): `uploads_total` split
/// success vs. one series per `StorageFailure` reason, `fetches_total`
/// split success/403/404/429, `deletes_total` split success/429, and the
/// `stored_rows_count` gauge split anonymous/account, sampled on the
/// expiry-sweep's own cycle (`sweep.rs`) rather than per-request.
pub struct Counters {
    uploads_success: AtomicU64,
    uploads_failure_by_reason: [AtomicU64; FAILURE_REASON_COUNT],
    /// A conflicting `anonymousDesignId` (BR1.1) is not a `StorageFailure`
    /// — `entities.md` has no "already exists" reason, and `failure.rs`'s
    /// own test pins that set to exactly six — so it has no reason series
    /// of its own under OD-2's table. Counted here in addition to (never
    /// instead of) the confirmed NFR7.4.2 series, so a conflict is never
    /// silently invisible to the aggregate counters.
    uploads_conflict: AtomicU64,
    fetches_success: AtomicU64,
    fetches_403: AtomicU64,
    fetches_404: AtomicU64,
    fetches_429: AtomicU64,
    deletes_success: AtomicU64,
    deletes_429: AtomicU64,
    /// R-03 — a genuine repository/database failure on delete (as opposed
    /// to a successful delete or a 429), tracked separately so it is never
    /// folded into `deletes_success`. Additive to OD-2's confirmed
    /// success/429 series, not a replacement for either.
    deletes_failure: AtomicU64,
    stored_rows_anonymous: AtomicU64,
    stored_rows_account: AtomicU64,
}

impl Default for Counters {
    fn default() -> Counters {
        Counters::new()
    }
}

impl Counters {
    pub fn new() -> Counters {
        Counters {
            uploads_success: AtomicU64::new(0),
            uploads_failure_by_reason: std::array::from_fn(|_| AtomicU64::new(0)),
            uploads_conflict: AtomicU64::new(0),
            fetches_success: AtomicU64::new(0),
            fetches_403: AtomicU64::new(0),
            fetches_404: AtomicU64::new(0),
            fetches_429: AtomicU64::new(0),
            deletes_success: AtomicU64::new(0),
            deletes_429: AtomicU64::new(0),
            deletes_failure: AtomicU64::new(0),
            stored_rows_anonymous: AtomicU64::new(0),
            stored_rows_account: AtomicU64::new(0),
        }
    }

    /// OD-2 — `uploads_total{result="success"}`: a committed `INSERT`.
    pub fn record_upload_success(&self) {
        self.uploads_success.fetch_add(1, Ordering::Relaxed);
    }

    /// OD-2 — `uploads_total{result="failure", reason="<reason>"}`: any
    /// `StorageFailure` on upload, one series per reason.
    pub fn record_upload_failure(&self, reason: FailureReason) {
        self.uploads_failure_by_reason[reason_index(reason)].fetch_add(1, Ordering::Relaxed);
    }

    /// A conflicting `anonymousDesignId` — see the field doc above.
    pub fn record_upload_conflict(&self) {
        self.uploads_conflict.fetch_add(1, Ordering::Relaxed);
    }

    /// OD-2 — `fetches_total{result="success"}`.
    pub fn record_fetch_success(&self) {
        self.fetches_success.fetch_add(1, Ordering::Relaxed);
    }

    /// OD-2 — `fetches_total{result="403"|"404"|"429"}`. A `status` that
    /// is none of the three named failure results (for example a 5xx
    /// `Internal` failure, which OD-2's table does not enumerate a fetch
    /// bucket for) increments no counter here — it is still logged via
    /// [`record_failure`], just outside this fixed aggregate set.
    pub fn record_fetch_failure(&self, status: u16) {
        match status {
            403 => {
                self.fetches_403.fetch_add(1, Ordering::Relaxed);
            }
            404 => {
                self.fetches_404.fetch_add(1, Ordering::Relaxed);
            }
            429 => {
                self.fetches_429.fetch_add(1, Ordering::Relaxed);
            }
            _ => {}
        }
    }

    /// OD-2 — `deletes_total{result="success"}`.
    pub fn record_delete_success(&self) {
        self.deletes_success.fetch_add(1, Ordering::Relaxed);
    }

    /// OD-2 — `deletes_total{result="429"}`.
    pub fn record_delete_rate_limited(&self) {
        self.deletes_429.fetch_add(1, Ordering::Relaxed);
    }

    /// R-03 — a genuine delete failure (never folded into
    /// `deletes_success`).
    pub fn record_delete_failure(&self) {
        self.deletes_failure.fetch_add(1, Ordering::Relaxed);
    }

    /// OD-2 — `stored_rows_count{ownership="anonymous"|"account"}`: a
    /// gauge, overwritten (not accumulated) on every sample — one sample
    /// per expiry-sweep cycle (`sweep.rs`), per OD-3/RD-3. Takes `i64`
    /// (a `count(*)` result is never negative in practice, but `sqlx`
    /// reports it as `i64`); clamped to `0` rather than panicking on the
    /// type conversion.
    pub fn set_stored_rows_count(&self, anonymous: i64, account: i64) {
        self.stored_rows_anonymous
            .store(anonymous.max(0) as u64, Ordering::Relaxed);
        self.stored_rows_account
            .store(account.max(0) as u64, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> CountersSnapshot {
        CountersSnapshot {
            uploads_success: self.uploads_success.load(Ordering::Relaxed),
            uploads_failure_by_reason: FailureReason::ALL
                .iter()
                .map(|r| {
                    (
                        *r,
                        self.uploads_failure_by_reason[reason_index(*r)].load(Ordering::Relaxed),
                    )
                })
                .collect(),
            uploads_conflict: self.uploads_conflict.load(Ordering::Relaxed),
            fetches_success: self.fetches_success.load(Ordering::Relaxed),
            fetches_403: self.fetches_403.load(Ordering::Relaxed),
            fetches_404: self.fetches_404.load(Ordering::Relaxed),
            fetches_429: self.fetches_429.load(Ordering::Relaxed),
            deletes_success: self.deletes_success.load(Ordering::Relaxed),
            deletes_429: self.deletes_429.load(Ordering::Relaxed),
            deletes_failure: self.deletes_failure.load(Ordering::Relaxed),
            stored_rows_anonymous: self.stored_rows_anonymous.load(Ordering::Relaxed),
            stored_rows_account: self.stored_rows_account.load(Ordering::Relaxed),
        }
    }
}

fn reason_index(reason: FailureReason) -> usize {
    FailureReason::ALL
        .iter()
        .position(|r| *r == reason)
        .expect("FailureReason::ALL lists every variant")
}

/// A point-in-time read of every counter (`NFR7.4.2`/OD-2's periodic row).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CountersSnapshot {
    pub uploads_success: u64,
    pub uploads_failure_by_reason: Vec<(FailureReason, u64)>,
    pub uploads_conflict: u64,
    pub fetches_success: u64,
    pub fetches_403: u64,
    pub fetches_404: u64,
    pub fetches_429: u64,
    pub deletes_success: u64,
    pub deletes_429: u64,
    pub deletes_failure: u64,
    pub stored_rows_anonymous: u64,
    pub stored_rows_account: u64,
}

/// OD-2 — the periodic counters row, the same stdout-JSON-rows shape
/// `osm-extract-proxy`'s own `emit::emit_counters` established. Every
/// per-reason failure count is logged as its `Debug` text rather than a
/// fixed field per reason, since the arity varies with
/// `FailureReason::ALL`'s length rather than being fixed at compile time
/// (the same reasoning `osm-extract-proxy`'s own `emit_counters` states).
pub fn emit_counters(snapshot: &CountersSnapshot) {
    tracing::info!(
        event = "counters",
        uploads_success = snapshot.uploads_success,
        uploads_failure_by_reason = ?snapshot.uploads_failure_by_reason,
        uploads_conflict = snapshot.uploads_conflict,
        fetches_success = snapshot.fetches_success,
        fetches_403 = snapshot.fetches_403,
        fetches_404 = snapshot.fetches_404,
        fetches_429 = snapshot.fetches_429,
        deletes_success = snapshot.deletes_success,
        deletes_429 = snapshot.deletes_429,
        deletes_failure = snapshot.deletes_failure,
        stored_rows_anonymous = snapshot.stored_rows_anonymous,
        stored_rows_account = snapshot.stored_rows_account,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // NFR7.4.2/OD-2 — uploads_total: success and one series per
    // StorageFailure reason, plus the additive conflict counter.
    #[test]
    fn upload_counters_track_success_and_per_reason_failure_independently() {
        let counters = Counters::new();
        counters.record_upload_success();
        counters.record_upload_success();
        counters.record_upload_failure(FailureReason::PayloadTooLarge);
        counters.record_upload_failure(FailureReason::UnsupportedPayloadVersion);
        counters.record_upload_failure(FailureReason::PayloadTooLarge);
        counters.record_upload_conflict();

        let snapshot = counters.snapshot();
        assert_eq!(snapshot.uploads_success, 2);
        assert_eq!(snapshot.uploads_conflict, 1);
        let too_large = snapshot
            .uploads_failure_by_reason
            .iter()
            .find(|(r, _)| *r == FailureReason::PayloadTooLarge)
            .map(|(_, n)| *n);
        assert_eq!(too_large, Some(2));
        let unsupported_version = snapshot
            .uploads_failure_by_reason
            .iter()
            .find(|(r, _)| *r == FailureReason::UnsupportedPayloadVersion)
            .map(|(_, n)| *n);
        assert_eq!(unsupported_version, Some(1));
        let untouched = snapshot
            .uploads_failure_by_reason
            .iter()
            .find(|(r, _)| *r == FailureReason::Internal)
            .map(|(_, n)| *n);
        assert_eq!(untouched, Some(0));
    }

    // NFR7.4.2/OD-2 — every FailureReason variant has its own upload
    // failure series (mirrors `osm-extract-proxy/src/counters.rs`'s own
    // "every failure reason has its own counter" test).
    #[test]
    fn every_failure_reason_has_its_own_upload_failure_counter() {
        let counters = Counters::new();
        for reason in FailureReason::ALL {
            counters.record_upload_failure(reason);
        }
        let snapshot = counters.snapshot();
        assert_eq!(
            snapshot.uploads_failure_by_reason.len(),
            FailureReason::ALL.len()
        );
        assert!(
            snapshot
                .uploads_failure_by_reason
                .iter()
                .all(|(_, n)| *n == 1)
        );
    }

    // NFR7.4.2/OD-2 — fetches_total: success/403/404/429 tracked
    // independently; a status outside that set (e.g. a 503 Internal
    // failure) increments none of them.
    #[test]
    fn fetch_counters_track_success_403_404_429_independently() {
        let counters = Counters::new();
        counters.record_fetch_success();
        counters.record_fetch_failure(404);
        counters.record_fetch_failure(404);
        counters.record_fetch_failure(429);
        counters.record_fetch_failure(503); // outside OD-2's fetch result set

        let snapshot = counters.snapshot();
        assert_eq!(snapshot.fetches_success, 1);
        assert_eq!(snapshot.fetches_403, 0);
        assert_eq!(snapshot.fetches_404, 2);
        assert_eq!(snapshot.fetches_429, 1);
    }

    // NFR7.4.2/OD-2 — deletes_total: success/429, plus R-03's additive
    // failure bucket, never folded into success.
    #[test]
    fn delete_counters_track_success_429_and_failure_independently() {
        let counters = Counters::new();
        counters.record_delete_success();
        counters.record_delete_rate_limited();
        counters.record_delete_failure();

        let snapshot = counters.snapshot();
        assert_eq!(snapshot.deletes_success, 1);
        assert_eq!(snapshot.deletes_429, 1);
        assert_eq!(snapshot.deletes_failure, 1);
    }

    // NFR7.4.2/OD-2 — stored_rows_count is a gauge: the latest sample
    // overwrites, it never accumulates across samples.
    #[test]
    fn stored_rows_count_is_a_gauge_that_overwrites_not_accumulates() {
        let counters = Counters::new();
        counters.set_stored_rows_count(5, 2);
        counters.set_stored_rows_count(3, 9);
        let snapshot = counters.snapshot();
        assert_eq!(snapshot.stored_rows_anonymous, 3);
        assert_eq!(snapshot.stored_rows_account, 9);
    }

    #[test]
    fn a_fresh_counters_snapshot_is_all_zero() {
        let snapshot = Counters::new().snapshot();
        assert_eq!(snapshot.uploads_success, 0);
        assert_eq!(snapshot.uploads_conflict, 0);
        assert_eq!(snapshot.fetches_success, 0);
        assert_eq!(snapshot.deletes_success, 0);
        assert_eq!(snapshot.stored_rows_anonymous, 0);
        assert_eq!(snapshot.stored_rows_account, 0);
        assert!(
            snapshot
                .uploads_failure_by_reason
                .iter()
                .all(|(_, n)| *n == 0)
        );
    }

    #[test]
    fn operation_names_are_stable_wire_strings() {
        assert_eq!(Operation::Upload.as_str(), "upload");
        assert_eq!(Operation::Fetch.as_str(), "fetch");
        assert_eq!(Operation::Remove.as_str(), "remove");
    }
}
