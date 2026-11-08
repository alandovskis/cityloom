//! The time, behind a trait so a test can move it.
use std::sync::Mutex;
use time::OffsetDateTime;

pub trait Clock: Send + Sync {
    fn now(&self) -> OffsetDateTime;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

pub struct FixedClock(Mutex<OffsetDateTime>);

impl FixedClock {
    pub fn new(at: OffsetDateTime) -> FixedClock {
        FixedClock(Mutex::new(at))
    }
    pub fn set(&self, at: OffsetDateTime) {
        *self.0.lock().unwrap() = at;
    }
    pub fn advance(&self, by: time::Duration) {
        *self.0.lock().unwrap() += by;
    }
}

impl Clock for FixedClock {
    fn now(&self) -> OffsetDateTime {
        *self.0.lock().unwrap()
    }
}
