//! `street-core` — the imported street/lane baseline domain model (Unit U2,
//! "Streetmix at city scale").
//!
//! This crate owns the **imported baseline**: the street and lane types
//! exactly as an import produced them, converted into this project's own
//! provenance-carrying types (BR1.1). It owns no editing state
//! (`u5-design-editing`), no serialization (`u3-design-payload-spec`), and
//! no rendering (`u6-client-surfaces`). It depends on nothing beyond the
//! Rust standard library (SD-2) and forbids `unsafe` (SD-3).

#![forbid(unsafe_code)]

pub mod graph;
pub mod lane;
pub mod provenance;
pub mod source;
pub mod street;

pub use graph::{IntersectionId, StreetNetworkGraph};
pub use lane::{Direction, Lane, LaneKey, LaneList};
pub use provenance::{Dimension, Provenance};
pub use source::StreetSource;
pub use street::{
    BoundingNodeIds, KERB_BUFFER_LANE_TYPE, Street, StreetIdentity, VERGE_BUFFER_LANE_TYPE,
    WALKABLE_LANE_TYPES,
};
