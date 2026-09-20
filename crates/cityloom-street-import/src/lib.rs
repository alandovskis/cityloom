//! `cityloom-street-import` — the request-to-corrected-street pipeline
//! (Unit U4, "Streetmix at city scale"). Owns three components
//! (`components.md`): [`ExtractFetcher`] (fetch + classify),
//! [`StreetImportAdapter`] (the only component permitted to depend on
//! `osm2streets` directly), and [`CorrectionOverlay`] (correction storage
//! and re-import reconciliation).
//!
//! Compiles to `wasm32-unknown-unknown` as part of the browser client. This
//! crate forbids `unsafe` — every conversion at the `osm2streets` boundary
//! is done through safe Rust and `catch_unwind` (`security-design.md`
//! SD-1), never a raw pointer or FFI call of its own.

#![forbid(unsafe_code)]

mod adapter;
mod correction;
mod failure;
mod fetcher;
mod fingerprint;
mod log;
mod transport;

pub use adapter::{ImportedStreet, StreetImportAdapter};
pub use correction::{
    Correction, CorrectionOverlay, CorrectionReconciliationOutcome, CorrectionState,
    ReconciliationOutcome,
};
pub use failure::{FailureReason, ImportFailure};
pub use fetcher::ExtractFetcher;
pub use fingerprint::{DrivingSide, FingerprintMapConfig, ImportFingerprint};
pub use transport::{
    ExtractTransport, GlooExtractTransport, GlooTimeoutSource, TimeoutSource, TransportOutcome,
};

/// The pinned `osm2streets` commit SHA this Unit was built and tested
/// against (`docs/osm2streets-pin.md`) — the value `ImportFingerprint.
/// osm2streets_revision` should be stamped with at import time.
pub const OSM2STREETS_REVISION: &str = "fc119c47dac567d030c6ce7c24a48896f58ed906";
