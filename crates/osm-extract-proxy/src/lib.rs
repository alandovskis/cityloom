//! `osm-extract-proxy` — Unit U9 of CityLoom.
//!
//! Serves OpenStreetMap extract clips for a bounding box, cut from regional
//! data built offline (`functional-spec.md` W1–W5). Two binaries share this
//! library: the service (`src/bin/osm-extract-proxy.rs`) and the region-build
//! tool (`src/bin/region-build.rs`).
//!
//! Module rings (`code-generation-plan.md`, "Layout"):
//! - pure, no HTTP, no I/O, no logging: [`geo`], [`grid`], [`manifest`],
//!   [`osm`], [`encode`], [`cut`], [`extract_key`], [`failure`];
//! - the I/O ring: [`store`], [`cache`], [`limiter`], [`counters`], [`emit`],
//!   [`readiness`], [`config`], [`build`];
//! - the outer ring: [`http`] and the binaries.

pub mod build;
pub mod cache;
pub mod config;
pub mod counters;
pub mod cut;
pub mod decode;
pub mod emit;
pub mod encode;
pub mod extract_key;
pub mod failure;
pub mod geo;
pub mod grid;
pub mod http;
pub mod limiter;
pub mod manifest;
pub mod osm;
pub mod readiness;
pub mod store;

/// Generated protobuf types for the OSM PBF format (see `build.rs`).
#[allow(
    clippy::all,
    clippy::pedantic,
    dead_code,
    unused_imports,
    unreachable_pub
)]
pub(crate) mod proto_gen {
    include!(concat!(env!("OUT_DIR"), "/proto_gen/mod.rs"));
}
