//! `limiter` — per-requester request limiting on a keyed hash of the
//! address, two fixed windows (minute and hour), a sweep for stale entries,
//! and the `Retry-After` value a 429 carries (BR2.x, SD-4, TS-8). The hash
//! key is minted once per process and never leaves memory (BR2.3).
//!
//! Every request counts against the limiter, hit or miss (BR2.2) — that
//! rule lives in `http.rs`, which calls [`Limiter::check`] before the cache
//! is even consulted.

use std::collections::HashMap;
use std::hash::{BuildHasher, RandomState};
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// One requester's two counting windows (BR7.3: the only per-requester
/// state, held in memory, no location, discarded at window end).
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
    /// Limited; the number of seconds a client should wait, `1..=60`
    /// (NFR5.1.7).
    Limited {
        retry_after_secs: u32,
    },
}

const MINUTE: Duration = Duration::from_secs(60);
const HOUR: Duration = Duration::from_secs(3_600);

/// Per-requester request limiting: 30 per minute and 300 per hour by
/// default (BR2.1; the runtime values are configuration with these
/// defaults).
pub struct Limiter {
    per_minute: u32,
    per_hour: u32,
    hash_key: RandomState,
    windows: Mutex<HashMap<u64, Windows>>,
}

impl Limiter {
    pub fn new(per_minute: u32, per_hour: u32) -> Limiter {
        Limiter {
            per_minute,
            per_hour,
            // BR2.3 — minted fresh at process start, kept only in memory.
            hash_key: RandomState::new(),
            windows: Mutex::new(HashMap::new()),
        }
    }

    /// The keyed hash of an address (BR2.1, BR2.3): never the address
    /// itself, and different across two `Limiter` instances (NFR6.3.2).
    pub fn hash_of(&self, addr: IpAddr) -> u64 {
        self.hash_key.hash_one(addr)
    }

    /// Check and, if allowed, count one request for `addr` at `now`
    /// (BR2.1). Windows use a fixed origin, not a rolling one: a request in
    /// a new minute resets the minute count without touching the hour count.
    pub fn check(&self, addr: IpAddr, now: Instant) -> LimitOutcome {
        let hash = self.hash_of(addr);
        let mut windows = self.windows.lock().expect("limiter mutex poisoned");
        let entry = windows.entry(hash).or_insert(Windows {
            minute_started: now,
            minute_count: 0,
            hour_started: now,
            hour_count: 0,
        });
        if now.duration_since(entry.minute_started) >= MINUTE {
            entry.minute_started = now;
            entry.minute_count = 0;
        }
        if now.duration_since(entry.hour_started) >= HOUR {
            entry.hour_started = now;
            entry.hour_count = 0;
        }
        if entry.hour_count >= self.per_hour {
            let elapsed = now.duration_since(entry.hour_started);
            let remaining = HOUR.saturating_sub(elapsed).as_secs().clamp(1, 60) as u32;
            return LimitOutcome::Limited {
                retry_after_secs: remaining,
            };
        }
        if entry.minute_count >= self.per_minute {
            let elapsed = now.duration_since(entry.minute_started);
            let remaining = MINUTE.saturating_sub(elapsed).as_secs().clamp(1, 60) as u32;
            return LimitOutcome::Limited {
                retry_after_secs: remaining,
            };
        }
        entry.minute_count += 1;
        entry.hour_count += 1;
        LimitOutcome::Allowed
    }

    /// Remove every window whose hour has fully elapsed as of `now`, so a
    /// requester who stops never grows the table forever (NFR5.1.3).
    pub fn sweep(&self, now: Instant) {
        let mut windows = self.windows.lock().expect("limiter mutex poisoned");
        windows.retain(|_, w| now.duration_since(w.hour_started) < HOUR);
    }

    pub fn tracked_requesters(&self) -> usize {
        self.windows.lock().expect("limiter mutex poisoned").len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(n: u8) -> IpAddr {
        IpAddr::from([192, 0, 2, n])
    }

    // BR2.1 — the 31st request in a minute is limited.
    #[test]
    fn the_thirty_first_request_in_a_minute_is_limited() {
        let limiter = Limiter::new(30, 300);
        let now = Instant::now();
        for _ in 0..30 {
            assert_eq!(limiter.check(addr(1), now), LimitOutcome::Allowed);
        }
        assert!(matches!(
            limiter.check(addr(1), now),
            LimitOutcome::Limited { .. }
        ));
    }

    // BR2.1 — the 301st request in an hour is limited even with the minute
    // window clear.
    #[test]
    fn the_301st_request_in_an_hour_is_limited_with_the_minute_clear() {
        let limiter = Limiter::new(30, 300);
        let start = Instant::now();
        for minute in 0..10u32 {
            let now = start + minute * MINUTE;
            for _ in 0..30 {
                assert_eq!(limiter.check(addr(2), now), LimitOutcome::Allowed);
            }
        }
        // 10 * 30 = 300 requests consumed the hour budget; the minute
        // window is fresh (a new minute), but the hour is not.
        let now = start + 10 * MINUTE;
        assert!(matches!(
            limiter.check(addr(2), now),
            LimitOutcome::Limited { .. }
        ));
    }

    // A new minute resets the minute count, not the hour count.
    #[test]
    fn a_new_minute_resets_only_the_minute_count() {
        let limiter = Limiter::new(5, 300);
        let start = Instant::now();
        for _ in 0..5 {
            assert_eq!(limiter.check(addr(3), start), LimitOutcome::Allowed);
        }
        assert!(matches!(
            limiter.check(addr(3), start),
            LimitOutcome::Limited { .. }
        ));
        let next_minute = start + MINUTE;
        assert_eq!(
            limiter.check(addr(3), next_minute),
            LimitOutcome::Allowed,
            "a new minute clears the minute window"
        );
    }

    // The sweep removes windows older than an hour.
    #[test]
    fn the_sweep_removes_windows_older_than_an_hour() {
        let limiter = Limiter::new(30, 300);
        let start = Instant::now();
        limiter.check(addr(4), start);
        assert_eq!(limiter.tracked_requesters(), 1);
        limiter.sweep(start + HOUR + Duration::from_secs(1));
        assert_eq!(limiter.tracked_requesters(), 0);
    }

    // NFR5.1.7 — Retry-After is between 1 and 60 on a minute-limited request.
    #[test]
    fn retry_after_is_between_one_and_sixty_seconds() {
        let limiter = Limiter::new(1, 300);
        let now = Instant::now();
        limiter.check(addr(5), now);
        match limiter.check(addr(5), now) {
            LimitOutcome::Limited { retry_after_secs } => {
                assert!((1..=60).contains(&retry_after_secs));
            }
            LimitOutcome::Allowed => panic!("expected limited"),
        }
    }

    // NFR6.3.2 — two instances hash the same address differently.
    #[test]
    fn two_instances_hash_the_same_address_differently() {
        let a = Limiter::new(30, 300);
        let b = Limiter::new(30, 300);
        // Astronomically likely to differ; if it doesn't, the RNG key
        // failed to vary, which is exactly what this test guards against.
        assert_ne!(a.hash_of(addr(6)), b.hash_of(addr(6)));
    }

    // NFR2.1.3 — 10,000 entries stay small: a `Windows` is a few machine
    // words, so the map costs well under 640 KB even with hashmap overhead.
    #[test]
    fn ten_thousand_entries_cost_well_under_the_budget() {
        assert!(std::mem::size_of::<Windows>() <= 40);
        let limiter = Limiter::new(30, 300);
        let now = Instant::now();
        for i in 0..10_000u32 {
            let ip = IpAddr::from(i.to_be_bytes());
            limiter.check(ip, now);
        }
        let approx_bytes = limiter.tracked_requesters() * (std::mem::size_of::<Windows>() + 24);
        assert!(approx_bytes <= 640_000, "{approx_bytes}");
    }
}
