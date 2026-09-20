//! `emit` — the one module allowed to write a `tracing` event (BR7.1, BR7.2,
//! NFR6.3.1): the subscriber configuration (flattened JSON rows, no span
//! events) and the fixed set of events the rest of the service may raise
//! (OD-1..OD-5). Nothing here ever takes an address, a box or a key as a
//! parameter — the type signatures make that mistake impossible to make by
//! accident.

use std::io;
use std::sync::{Arc, Mutex};

use tracing_subscriber::fmt::MakeWriter;

use crate::failure::FailureRecord;

/// Build the service's `tracing` subscriber: one flat JSON object per event
/// (`flatten_event`, so `reason`, `status`, `phase` and `detail` sit
/// alongside `timestamp` and `level` rather than nested under `fields`),
/// and no span-open/close events (`with_span_events` defaults to none,
/// stated explicitly here so a later change to this function cannot
/// reintroduce them by accident).
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

/// The production subscriber: JSON rows to stdout.
pub fn init_stdout() {
    let subscriber = subscriber(io::stdout);
    tracing::subscriber::set_global_default(subscriber)
        .expect("the global subscriber is set exactly once, at start-up");
}

/// BR7.2 — record one failure: reason, status, phase and the
/// project-authored detail, and nothing else (BR7.1 forbids an address, a
/// box or a key, and this function's signature has no parameter that could
/// carry one). The level follows the status: a 5xx is `error`, anything
/// else is `warn` (OD-3).
pub fn record_failure(record: &FailureRecord) {
    if record.status >= 500 {
        tracing::error!(
            reason = record.reason.as_str(),
            status = record.status,
            phase = record.phase.as_str(),
            detail = record.detail,
        );
    } else {
        tracing::warn!(
            reason = record.reason.as_str(),
            status = record.status,
            phase = record.phase.as_str(),
            detail = record.detail,
        );
    }
}

/// OD-4 — the service became Ready: how many cells the manifest lists.
pub fn emit_ready(cell_count: u64) {
    tracing::info!(event = "ready", cell_count = cell_count);
}

/// OD-4 — the service is Unready: which check failed and how many cells, if
/// any, failed it.
pub fn emit_unready(check: &str, failing_cells: usize) {
    tracing::warn!(
        event = "unready",
        check = check,
        failing_cells = failing_cells
    );
}

/// OD-4 — the service is shutting down (RD-7's drain sequence).
pub fn emit_shutdown() {
    tracing::info!(event = "shutdown");
}

/// OD-2 — the periodic counters row: every aggregate, no per-request rows
/// (BR7.4). `failuresByReason` and the latency buckets are logged as their
/// `Debug` text rather than individual fields, since their arity varies
/// with `FailureReason::ALL`'s length rather than being fixed at compile
/// time.
pub fn emit_counters(snapshot: &crate::counters::Snapshot) {
    tracing::info!(
        event = "counters",
        uptime_seconds = snapshot.uptime_seconds,
        extracts_served = snapshot.extracts_served,
        bytes_served = snapshot.bytes_served,
        cache_hits = snapshot.cache_hits,
        cache_misses = snapshot.cache_misses,
        requests_limited = snapshot.requests_limited,
        last_extract_bytes = snapshot.last_extract_bytes,
        failures_by_reason = ?snapshot.failures_by_reason,
    );
}

/// A `tracing` writer that collects every emitted line into memory instead
/// of stdout, for the log-discipline tests (`tests/common/mod.rs` and this
/// module's own tests) and for nothing else — it is never installed in
/// production.
#[derive(Clone, Default)]
pub struct CaptureWriter(Arc<Mutex<Vec<u8>>>);

impl CaptureWriter {
    pub fn new() -> CaptureWriter {
        CaptureWriter(Arc::new(Mutex::new(Vec::new())))
    }

    /// Every captured line (one JSON object per `tracing` event), in order.
    pub fn lines(&self) -> Vec<String> {
        let buf = self.0.lock().expect("capture writer mutex poisoned");
        String::from_utf8_lossy(&buf)
            .lines()
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect()
    }
}

impl<'a> MakeWriter<'a> for CaptureWriter {
    type Writer = CaptureHandle;

    fn make_writer(&'a self) -> Self::Writer {
        CaptureHandle(Arc::clone(&self.0))
    }
}

#[doc(hidden)]
pub struct CaptureHandle(Arc<Mutex<Vec<u8>>>);

impl io::Write for CaptureHandle {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .expect("capture writer mutex poisoned")
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::failure::Phase;
    use cityloom_api_types::FailureReason;

    fn capture_one(f: impl FnOnce()) -> Vec<String> {
        let writer = CaptureWriter::new();
        let subscriber = subscriber(writer.clone());
        tracing::subscriber::with_default(subscriber, f);
        writer.lines()
    }

    // OD-3 — record_failure emits one JSON row with the five fields (plus
    // the subscriber's own timestamp/level) and the level per reason.
    #[test]
    fn record_failure_emits_one_row_with_the_expected_fields_and_level() {
        let record = FailureRecord::new(FailureReason::Timeout, Phase::Clip);
        let lines = capture_one(|| record_failure(&record));
        assert_eq!(lines.len(), 1);
        let row = &lines[0];
        assert!(row.contains("\"reason\":\"timeout\""));
        assert!(row.contains("\"status\":503"));
        assert!(row.contains("\"phase\":\"clip\""));
        assert!(row.contains("\"detail\":\"the request did not complete in time\""));
        assert!(row.contains("\"level\":\"ERROR\""), "{row}");
    }

    #[test]
    fn a_non_5xx_failure_is_logged_at_warn() {
        let record = FailureRecord::new(FailureReason::InvalidArea, Phase::Validation);
        let lines = capture_one(|| record_failure(&record));
        assert!(lines[0].contains("\"level\":\"WARN\""));
    }

    // OD-4 — ready/unready/shutdown carry their stated fields.
    #[test]
    fn ready_unready_and_shutdown_carry_their_fields() {
        let ready = capture_one(|| emit_ready(42));
        assert!(ready[0].contains("\"event\":\"ready\""));
        assert!(ready[0].contains("\"cell_count\":42"));

        let unready = capture_one(|| emit_unready("digest", 3));
        assert!(unready[0].contains("\"event\":\"unready\""));
        assert!(unready[0].contains("\"check\":\"digest\""));
        assert!(unready[0].contains("\"failing_cells\":3"));

        let shutdown = capture_one(emit_shutdown);
        assert!(shutdown[0].contains("\"event\":\"shutdown\""));
    }

    // OD-2 — the counters row carries the aggregate fields.
    #[test]
    fn the_counters_row_carries_the_aggregate_fields() {
        let counters = crate::counters::Counters::new();
        counters.record_served(500, false);
        let snapshot = counters.snapshot();
        let lines = capture_one(|| emit_counters(&snapshot));
        assert!(lines[0].contains("\"event\":\"counters\""));
        assert!(lines[0].contains("\"extracts_served\":1"));
        assert!(lines[0].contains("\"bytes_served\":500"));
    }

    // The subscriber emits no span events.
    #[test]
    fn the_subscriber_emits_no_span_events() {
        let writer = CaptureWriter::new();
        let subscriber = subscriber(writer.clone());
        tracing::subscriber::with_default(subscriber, || {
            let span = tracing::info_span!("a_span");
            let _entered = span.enter();
            tracing::info!(event = "inside_span");
        });
        let lines = writer.lines();
        assert_eq!(lines.len(), 1, "only the explicit event, no span rows");
        assert!(!lines[0].contains("\"new\""));
        assert!(!lines[0].contains("\"close\""));
    }
}
