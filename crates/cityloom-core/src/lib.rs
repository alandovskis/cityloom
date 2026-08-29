//! Canonical CityLoom domain model.
//!
//! This crate is the single source of truth for the street network, its
//! cross-sections, and the geometry operations over them. Both the server and
//! the wasm client depend on it, so it must stay free of platform-specific
//! dependencies (no tokio, no web-sys, no sqlx types in public signatures).

/// Schema version of the `City` document format.
///
/// Bump this whenever the serialized shape changes incompatibly. Loading a
/// document whose version exceeds this must fail with a typed error rather
/// than silently misinterpreting the data (see CORE-033).
pub const SCHEMA_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    use super::SCHEMA_VERSION;

    /// Skeleton smoke test: proves the crate builds and the harness runs.
    /// Replace with real model tests in task 004.
    #[test]
    fn schema_version_is_positive() {
        assert!(std::hint::black_box(SCHEMA_VERSION) >= 1);
    }
}
