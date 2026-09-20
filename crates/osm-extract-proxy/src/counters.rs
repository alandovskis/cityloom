//! `counters` — the aggregate `ServiceCounters` (BR7.4: aggregate only, no
//! per-request row) and the latency histogram the periodic row reports
//! (OD-2, PD-6). Lock-free atomics; readable from any thread without a
//! `Mutex`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use cityloom_api_types::FailureReason;

/// The upper bound (inclusive, milliseconds) of each of the nine latency
/// buckets; the last bucket catches everything above the second-to-last
/// bound.
pub const LATENCY_BUCKET_BOUNDS_MS: [u64; 9] = [1, 5, 10, 30, 50, 100, 300, 1_000, u64::MAX];

const FAILURE_REASON_COUNT: usize = 8;

/// Aggregate counters, updated per request (BR7.4) and read for the
/// periodic counters row (OD-2) and `GET /internal/counters` if one exists.
pub struct Counters {
    started_at: Instant,
    extracts_served: AtomicU64,
    bytes_served: AtomicU64,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    requests_limited: AtomicU64,
    last_extract_bytes: AtomicU64,
    failures_by_reason: [AtomicU64; FAILURE_REASON_COUNT],
    latency_buckets: [AtomicU64; LATENCY_BUCKET_BOUNDS_MS.len()],
}

impl Default for Counters {
    fn default() -> Counters {
        Counters::new()
    }
}

impl Counters {
    pub fn new() -> Counters {
        Counters {
            started_at: Instant::now(),
            extracts_served: AtomicU64::new(0),
            bytes_served: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            requests_limited: AtomicU64::new(0),
            last_extract_bytes: AtomicU64::new(0),
            failures_by_reason: std::array::from_fn(|_| AtomicU64::new(0)),
            latency_buckets: std::array::from_fn(|_| AtomicU64::new(0)),
        }
    }

    /// Record one successfully served extract of `bytes`, hit or miss.
    pub fn record_served(&self, bytes: u64, was_hit: bool) {
        self.extracts_served.fetch_add(1, Ordering::Relaxed);
        self.bytes_served.fetch_add(bytes, Ordering::Relaxed);
        self.last_extract_bytes.store(bytes, Ordering::Relaxed);
        if was_hit {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn record_limited(&self) {
        self.requests_limited.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_failure(&self, reason: FailureReason) {
        self.failures_by_reason[failure_index(reason)].fetch_add(1, Ordering::Relaxed);
    }

    /// Record one request's latency into the bucket it falls in (PD-6).
    pub fn record_latency_ms(&self, latency_ms: u64) {
        let bucket = LATENCY_BUCKET_BOUNDS_MS
            .iter()
            .position(|bound| latency_ms <= *bound)
            .unwrap_or(LATENCY_BUCKET_BOUNDS_MS.len() - 1);
        self.latency_buckets[bucket].fetch_add(1, Ordering::Relaxed);
    }

    pub fn uptime_seconds(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }

    /// A consistent-enough (not atomic-across-fields) snapshot for the
    /// periodic row and any introspection endpoint (OD-2).
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            uptime_seconds: self.uptime_seconds(),
            extracts_served: self.extracts_served.load(Ordering::Relaxed),
            bytes_served: self.bytes_served.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
            requests_limited: self.requests_limited.load(Ordering::Relaxed),
            last_extract_bytes: self.last_extract_bytes.load(Ordering::Relaxed),
            failures_by_reason: FailureReason::ALL
                .iter()
                .map(|r| {
                    (
                        *r,
                        self.failures_by_reason[failure_index(*r)].load(Ordering::Relaxed),
                    )
                })
                .collect(),
            latency_buckets_ms: LATENCY_BUCKET_BOUNDS_MS
                .iter()
                .zip(self.latency_buckets.iter())
                .map(|(bound, count)| (*bound, count.load(Ordering::Relaxed)))
                .collect(),
        }
    }
}

fn failure_index(reason: FailureReason) -> usize {
    FailureReason::ALL
        .iter()
        .position(|r| *r == reason)
        .expect("FailureReason::ALL lists every variant")
}

/// A point-in-time read of every counter (OD-2's periodic row, W5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub uptime_seconds: u64,
    pub extracts_served: u64,
    pub bytes_served: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub requests_limited: u64,
    pub last_extract_bytes: u64,
    pub failures_by_reason: Vec<(FailureReason, u64)>,
    pub latency_buckets_ms: Vec<(u64, u64)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // OD-2 — increments are visible in the next snapshot.
    #[test]
    fn increments_are_visible_in_the_next_snapshot() {
        let counters = Counters::new();
        counters.record_served(1_234, false);
        counters.record_served(500, true);
        counters.record_limited();
        counters.record_failure(FailureReason::Timeout);

        let snapshot = counters.snapshot();
        assert_eq!(snapshot.extracts_served, 2);
        assert_eq!(snapshot.bytes_served, 1_734);
        assert_eq!(snapshot.cache_hits, 1);
        assert_eq!(snapshot.cache_misses, 1);
        assert_eq!(snapshot.requests_limited, 1);
        assert_eq!(snapshot.last_extract_bytes, 500);
        let timeout_count = snapshot
            .failures_by_reason
            .iter()
            .find(|(r, _)| *r == FailureReason::Timeout)
            .map(|(_, n)| *n);
        assert_eq!(timeout_count, Some(1));
    }

    // PD-6 — latency lands in the right one of the nine buckets.
    #[test]
    fn latency_lands_in_the_right_bucket() {
        let counters = Counters::new();
        counters.record_latency_ms(1); // bucket 0 (<=1)
        counters.record_latency_ms(7); // bucket 2 (<=10)
        counters.record_latency_ms(10_000); // bucket 8 (overflow)
        let snapshot = counters.snapshot();
        assert_eq!(snapshot.latency_buckets_ms.len(), 9);
        assert_eq!(snapshot.latency_buckets_ms[0].1, 1);
        assert_eq!(snapshot.latency_buckets_ms[2].1, 1);
        assert_eq!(snapshot.latency_buckets_ms[8].1, 1);
        let total: u64 = snapshot.latency_buckets_ms.iter().map(|(_, n)| n).sum();
        assert_eq!(total, 3);
    }

    // OD-2 — uptimeSeconds grows.
    #[test]
    fn uptime_seconds_grows() {
        let counters = Counters::new();
        let first = counters.uptime_seconds();
        std::thread::sleep(std::time::Duration::from_millis(1_100));
        assert!(counters.uptime_seconds() > first);
    }

    #[test]
    fn every_failure_reason_has_its_own_counter() {
        let counters = Counters::new();
        for reason in FailureReason::ALL {
            counters.record_failure(reason);
        }
        let snapshot = counters.snapshot();
        assert_eq!(snapshot.failures_by_reason.len(), FailureReason::ALL.len());
        assert!(snapshot.failures_by_reason.iter().all(|(_, n)| *n == 1));
    }

    #[test]
    fn a_fresh_counters_snapshot_is_all_zero() {
        let snapshot = Counters::new().snapshot();
        assert_eq!(snapshot.extracts_served, 0);
        assert_eq!(snapshot.bytes_served, 0);
        assert_eq!(snapshot.last_extract_bytes, 0);
        assert!(snapshot.failures_by_reason.iter().all(|(_, n)| *n == 0));
    }
}
