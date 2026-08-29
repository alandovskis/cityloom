//! CityLoom HTTP server: axum routes, Postgres persistence, OSM import jobs.
//!
//! Task 015 replaces this skeleton with the real axum application.

/// Default bind address used when `CITYLOOM_ADDR` is unset.
pub const DEFAULT_ADDR: &str = "127.0.0.1:8080";

#[cfg(test)]
mod tests {
    use super::DEFAULT_ADDR;

    /// Skeleton smoke test. Replace with route tests in task 015.
    #[test]
    fn default_addr_has_a_port() {
        assert!(std::hint::black_box(DEFAULT_ADDR).contains(':'));
    }
}
