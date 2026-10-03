//! CityLoom cross-section editor core, compiled to WebAssembly.
//!
//! The model owns every editing rule. The page only draws the view it
//! returns and relays pointer and keyboard input.

#![recursion_limit = "1024"]

pub mod city;
pub mod junction;
pub mod shared;
pub mod street;
pub mod vm;
pub mod ui;

use shared::{atlas, symbols};
use wasm_bindgen::prelude::*;

use shared::catalogue::{CURBS, DIRECTIONS, KINDS, REGIONS, MATERIALS, SAMPLES};

pub(crate) fn json<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).expect("view serialises")
}

/// The segment catalogue as JSON.
#[wasm_bindgen]
pub fn catalogue() -> String {
    json(&KINDS)
}

/// The surface and curb material tables as JSON.
#[wasm_bindgen]
pub fn materials() -> String {
    json(&serde_json::json!({ "surfaces": MATERIALS, "curbs": CURBS, "directions": DIRECTIONS, "regions": REGIONS }))
}

/// Names of the sample streets as JSON.
#[wasm_bindgen]
pub fn samples() -> String {
    let list: Vec<_> = SAMPLES
        .iter()
        .map(|s| serde_json::json!({ "name": s.name, "row_mm": s.row_mm, "freeway": s.freeway }))
        .collect();
    json(&list)
}

/// The hatch patterns of the pieces, surfaces and curbs, as JSON: `{HATCH, MATERIAL_HATCH, CURB_HATCH}`,
/// each from a name to its SVG `<pattern>`.
#[wasm_bindgen]
pub fn hatches() -> String {
    symbols::hatches_json()
}

/// The Transit Priority Atlas toolbox measures and where each is modelled, as JSON.
#[wasm_bindgen]
pub fn atlas() -> String {
    json(&atlas::MEASURES)
}

