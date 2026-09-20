//! `rate_limit` — the identifier-keyed two-fixed-window limiter
//! (`security-design.md` SD-2, `infrastructure-specification.md` ID-19):
//! 10 requests/minute and 100 requests/hour per `anonymousDesignId`. Unlike
//! `osm-extract-proxy`'s own address-hashing limiter, the key here is the
//! identifier itself — BR4.1 protects the identifier from guessing, not the
//! requester from being profiled, so no address is read or hashed for this
//! check at all (NFR6.3.1). No dependency on `axum`, so this is
//! unit-testable without an HTTP server (`logical-components.md` LC-1).

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// One identifier's two counting windows — the only per-identifier state
/// this limiter holds, discarded on process restart (`security-design.md`'s
/// stated, accepted trade-off for Stage 1's threat model).
#[derive(Clone, Copy, Debug)]
struct Windows {
    minute_started: Instant,
    minute_count: u32,
    hour_started: Instant,
    hour_count: u32,
}

/// The outcome of a limiter check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LimitOutcome {
    Allowed,
    /// Limited; the number of seconds a client should wait, `1..=3600`.
    Limited {
        retry_after_secs: u32,
    },
}

const MINUTE: Duration = Duration::from_secs(60);
const HOUR: Duration = Duration::from_secs(3_600);

/// Per-`anonymousDesignId` request limiting (`security-design.md` SD-2).
/// `scalability-design.md` SC-1 — this map is per-instance bookkeeping,
/// not shared application state; it degrades safely (never over-counts a
/// legitimate client) under horizontal scaling.
pub struct RateLimiter {
    per_minute: u32,
    per_hour: u32,
    minute_window: Duration,
    hour_window: Duration,
    windows: Mutex<HashMap<String, Windows>>,
}

impl RateLimiter {
    /// The production shape: real minute/hour windows.
    pub fn new(per_minute: u32, per_hour: u32) -> RateLimiter {
        RateLimiter::with_windows(per_minute, per_hour, MINUTE, HOUR)
    }

    /// As [`RateLimiter::new`], but with explicit window durations instead
    /// of the real minute/hour. Production code always goes through `new`
    /// (`DesignStorageConfig::production_defaults` sets the real
    /// durations); this constructor exists so this crate's own tests can
    /// shrink the windows and observe [`RateLimiter::sweep`] actually
    /// reclaiming memory through the wired-up `lib.rs::build()` path,
    /// without waiting out a real hour (R-01, `security-design.md` SD-2).
    pub fn with_windows(
        per_minute: u32,
        per_hour: u32,
        minute_window: Duration,
        hour_window: Duration,
    ) -> RateLimiter {
        RateLimiter {
            per_minute,
            per_hour,
            minute_window,
            hour_window,
            windows: Mutex::new(HashMap::new()),
        }
    }

    /// Check and, if allowed, count one request against `identifier` at
    /// `now`. Windows use a fixed origin, not a rolling one: a request in
    /// a new minute resets the minute count without touching the hour
    /// count.
    pub fn check(&self, identifier: &str, now: Instant) -> LimitOutcome {
        let mut windows = self.windows.lock().expect("rate limiter mutex poisoned");
        let entry = windows.entry(identifier.to_string()).or_insert(Windows {
            minute_started: now,
            minute_count: 0,
            hour_started: now,
            hour_count: 0,
        });
        if now.duration_since(entry.minute_started) >= self.minute_window {
            entry.minute_started = now;
            entry.minute_count = 0;
        }
        if now.duration_since(entry.hour_started) >= self.hour_window {
            entry.hour_started = now;
            entry.hour_count = 0;
        }
        if entry.hour_count >= self.per_hour {
            let elapsed = now.duration_since(entry.hour_started);
            let remaining = self
                .hour_window
                .saturating_sub(elapsed)
                .as_secs()
                .clamp(1, self.hour_window.as_secs().max(1)) as u32;
            return LimitOutcome::Limited {
                retry_after_secs: remaining,
            };
        }
        if entry.minute_count >= self.per_minute {
            let elapsed = now.duration_since(entry.minute_started);
            let remaining = self
                .minute_window
                .saturating_sub(elapsed)
                .as_secs()
                .clamp(1, self.minute_window.as_secs().max(1)) as u32;
            return LimitOutcome::Limited {
                retry_after_secs: remaining,
            };
        }
        entry.minute_count += 1;
        entry.hour_count += 1;
        LimitOutcome::Allowed
    }

    /// Remove every window whose hour has fully elapsed as of `now`, so an
    /// identifier nobody retries again never grows the table forever
    /// (R-01, `security-design.md` SD-2 — this is what
    /// `lib.rs::build()`'s spawned periodic task calls).
    pub fn sweep(&self, now: Instant) {
        let mut windows = self.windows.lock().expect("rate limiter mutex poisoned");
        windows.retain(|_, w| now.duration_since(w.hour_started) < self.hour_window);
    }

    /// The number of identifiers currently tracked — test-only
    /// introspection, `pub(crate)` so the crate's `lib.rs::build()`
    /// wiring test can observe [`RateLimiter::sweep`] actually shrinking
    /// the map without reaching into this module's private state.
    #[cfg(test)]
    pub(crate) fn tracked_identifiers(&self) -> usize {
        self.windows
            .lock()
            .expect("rate limiter mutex poisoned")
            .len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Under both thresholds, a request against one identifier passes.
    #[test]
    fn under_both_thresholds_a_request_passes() {
        let limiter = RateLimiter::new(10, 100);
        let now = Instant::now();
        for _ in 0..9 {
            assert_eq!(limiter.check("design-1", now), LimitOutcome::Allowed);
        }
    }

    // At the 10/minute threshold, the 11th request within the window
    // returns rate_limited with a computed Retry-After.
    #[test]
    fn the_eleventh_request_in_a_minute_is_limited_with_retry_after() {
        let limiter = RateLimiter::new(10, 100);
        let now = Instant::now();
        for _ in 0..10 {
            assert_eq!(limiter.check("design-2", now), LimitOutcome::Allowed);
        }
        match limiter.check("design-2", now) {
            LimitOutcome::Limited { retry_after_secs } => {
                assert!((1..=60).contains(&retry_after_secs));
            }
            LimitOutcome::Allowed => panic!("expected the 11th request to be limited"),
        }
    }

    // The 101st request in an hour is limited even with the minute window
    // clear.
    #[test]
    fn the_101st_request_in_an_hour_is_limited_with_the_minute_window_clear() {
        let limiter = RateLimiter::new(10, 100);
        let start = Instant::now();
        for minute in 0..10u32 {
            let now = start + minute * MINUTE;
            for _ in 0..10 {
                assert_eq!(limiter.check("design-3", now), LimitOutcome::Allowed);
            }
        }
        let now = start + 10 * MINUTE;
        assert!(matches!(
            limiter.check("design-3", now),
            LimitOutcome::Limited { .. }
        ));
    }

    // A new minute resets only the minute count.
    #[test]
    fn a_new_minute_resets_only_the_minute_count() {
        let limiter = RateLimiter::new(2, 100);
        let start = Instant::now();
        for _ in 0..2 {
            assert_eq!(limiter.check("design-4", start), LimitOutcome::Allowed);
        }
        assert!(matches!(
            limiter.check("design-4", start),
            LimitOutcome::Limited { .. }
        ));
        let next_minute = start + MINUTE;
        assert_eq!(
            limiter.check("design-4", next_minute),
            LimitOutcome::Allowed
        );
    }

    // Two different identifiers are counted independently.
    #[test]
    fn two_different_identifiers_are_counted_independently() {
        let limiter = RateLimiter::new(1, 100);
        let now = Instant::now();
        assert_eq!(limiter.check("design-a", now), LimitOutcome::Allowed);
        assert_eq!(limiter.check("design-b", now), LimitOutcome::Allowed);
    }

    // The sweep removes windows older than an hour.
    #[test]
    fn the_sweep_removes_windows_older_than_an_hour() {
        let limiter = RateLimiter::new(10, 100);
        let start = Instant::now();
        limiter.check("design-5", start);
        assert_eq!(limiter.tracked_identifiers(), 1);
        limiter.sweep(start + HOUR + Duration::from_secs(1));
        assert_eq!(limiter.tracked_identifiers(), 0);
    }

    // R-01 — `with_windows` lets the sweep's own eligibility threshold be
    // shrunk below a real hour, which is what makes the end-to-end
    // `lib.rs::build()` wiring test (below, in `lib.rs`) able to observe a
    // real eviction inside a fast test.
    #[test]
    fn with_windows_uses_the_given_durations_instead_of_a_real_minute_and_hour() {
        let tiny_hour = Duration::from_millis(50);
        let limiter = RateLimiter::with_windows(10, 100, Duration::from_millis(10), tiny_hour);
        let start = Instant::now();
        limiter.check("design-6", start);
        assert_eq!(limiter.tracked_identifiers(), 1);
        // Not yet eligible — less than the shrunk hour window has passed.
        limiter.sweep(start + Duration::from_millis(10));
        assert_eq!(limiter.tracked_identifiers(), 1);
        // Eligible — the shrunk hour window has fully elapsed.
        limiter.sweep(start + tiny_hour + Duration::from_millis(1));
        assert_eq!(limiter.tracked_identifiers(), 0);
    }
}
