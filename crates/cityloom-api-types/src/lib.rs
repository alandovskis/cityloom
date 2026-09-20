//! Wire types shared by both ends of every CityLoom HTTP boundary
//! (`contract-summary.md`, "How these contracts are expressed").
//!
//! This crate is the source of truth every contract's OpenAPI block is
//! generated from. Contract 1 (extract fetch, U9 → U4) is owned by
//! `osm-extract-proxy`; Contract 2 (design upload/removal, U10 → U7) is
//! owned by `cityloom-design-storage`. Each Unit owns its own module here
//! and never edits another's; `contract1` and `contract2` each define their
//! own `ApiError`/`FailureReason` (the two boundaries have different closed
//! reason sets), so only `contract1`'s pair is re-exported at the crate
//! root for backward compatibility — `contract2`'s are reached through its
//! own module path to avoid a name collision.

pub mod contract1;
pub mod contract2;

pub use contract1::{ApiError, FailureReason, ReloadRequired};
