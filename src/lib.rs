//! CityLoom cross-section editor core, compiled to WebAssembly.
//!
//! The model owns every editing rule. The page only draws the view it
//! returns and relays pointer and keyboard input.

#![recursion_limit = "1024"]

pub mod city;
pub mod junction;
pub mod junction_view;
pub mod model;
pub mod plan;
pub mod street_measures;
pub mod shared;
pub mod vm;
pub mod ui;

use shared::{atlas, symbols};
use wasm_bindgen::prelude::*;

use shared::catalogue::{CURBS, DIRECTIONS, KINDS, REGIONS, MATERIALS, SAMPLES};

fn json<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).expect("view serialises")
}

/// One street being edited. The page drives it through the components the
/// module mounts; the script only starts it up and hands it what the shell
/// chooses (units, region).
#[wasm_bindgen]
pub struct Sheet(std::rc::Rc<vm::street::StreetVm>);

#[wasm_bindgen]
impl Sheet {
    #[wasm_bindgen(constructor)]
    pub fn new(sample: usize) -> Sheet {
        Sheet(vm::street::StreetVm::new(shared::platform::browser_ports(), model::Editor::new(sample), None))
    }

    /// Binds the shell every page shares (settings menu, sidebars, notes tabs)
    /// to this page.
    pub fn mount_shell(&self) {
        ui::shell::mount(self.0.clone(), "piece details");
    }

    /// The page is being left: what is waiting to be kept in the city is kept now.
    pub fn flush(&self) {
        self.0.flush();
    }

    /// The whole drawable state as JSON.
    pub fn view(&self) -> String {
        json(&*self.0.view_now())
    }
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

/// One junction being edited. The page drives it through the components the
/// module mounts; the script only starts it up and hands it what the shell
/// chooses (units, region).
#[wasm_bindgen]
pub struct Plan(std::rc::Rc<vm::junction::JunctionVm>);

#[wasm_bindgen]
impl Plan {
    #[wasm_bindgen(constructor)]
    pub fn new(sample: usize) -> Plan {
        Plan(vm::junction::JunctionVm::new(shared::platform::browser_ports(), junction::Junction::new(sample), None))
    }

    /// Binds the shell every page shares (settings menu, sidebars, notes tabs)
    /// to this page.
    pub fn mount_shell(&self) {
        ui::shell::mount(self.0.clone(), "details");
    }

    /// The page is being left: what is waiting to be kept in the city is kept now.
    pub fn flush(&self) {
        self.0.flush();
    }

    /// The whole drawable state as JSON.
    pub fn view(&self) -> String {
        json(&*self.0.view_now())
    }
}

/// Controls, sample junctions, the streets an arm may take and the limits, as JSON.
#[wasm_bindgen]
pub fn junction_catalogue() -> String {
    use junction::*;
    let samples: Vec<_> = JUNCTION_SAMPLES.iter().map(|s| serde_json::json!({ "name": s.name, "arms": s.arms.len() })).collect();
    let streets: Vec<_> = SAMPLES.iter().map(|s| serde_json::json!({ "name": s.name, "row_mm": s.row_mm, "pieces": s.segments, "freeway": s.freeway })).collect();
    json(&serde_json::json!({
        "controls": CONTROLS,
        "approaches": APPROACHES,
        "stops": STOPS,
        "rules": RULES,
        "samples": samples,
        "streets": streets,
        "limits": {
            "min_arms": MIN_ARMS, "max_arms": MAX_ARMS,
            "bearing_step": BEARING_STEP, "min_separation": MIN_SEPARATION,
            "corner": [MIN_CORNER_MM, MAX_CORNER_MM, RING_STEP_MM],
            "setback": [MIN_SETBACK_MM, MAX_SETBACK_MM, RING_STEP_MM],
            "crossing": [MIN_CROSSING_MM, MAX_CROSSING_MM, RING_STEP_MM],
            "offset_step": OFFSET_STEP_MM,
            "ring_step": RING_STEP_MM, "max_ring": MAX_RING_MM,
            "island_mm": ISLAND_MM,
            "approach": [APPROACH_MIN_MM, APPROACH_MAX_MM, APPROACH_STEP_MM, TRANSIT_LANE_MM],
            "cycle": [CYCLE_MIN_MM, CYCLE_MAX_MM, RING_STEP_MM, CYCLE_DEFAULT_MM],
        },
    }))
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

/// The name of a junction of the city kept in this browser; empty when there is none.
#[wasm_bindgen]
pub fn junction_name(node: u32) -> String {
    vm::city_store::CityStore::new(shared::platform::browser_ports().storage).open().junction_name(node).unwrap_or_default()
}

/// The junction editor on one junction of the city kept in this browser, which
/// writes what is made back to the city. Nothing when there is no such junction
/// or it cannot be drawn with the streets as they now are.
#[wasm_bindgen]
pub fn open_junction(node: u32) -> Option<Plan> {
    let ports = shared::platform::browser_ports();
    let store = vm::city_store::CityStore::new(ports.storage.clone());
    let junction = store.open().junction_editor(node, store.region())?;
    let place = vm::binding::Place::Junction(node);
    Some(Plan(vm::junction::JunctionVm::new(ports, junction, Some(vm::binding::CityBinding { store, place }))))
}

/// The street editor on one street of the city kept in this browser, which
/// writes what is made back to the city. Nothing when there is no such street.
#[wasm_bindgen]
pub fn open_street(edge: u32) -> Option<Sheet> {
    let ports = shared::platform::browser_ports();
    let store = vm::city_store::CityStore::new(ports.storage.clone());
    let street = store.open().street_editor(edge, store.region())?;
    let place = vm::binding::Place::Street(edge);
    Some(Sheet(vm::street::StreetVm::new(ports, street, Some(vm::binding::CityBinding { store, place }))))
}

/// The two ends of a street of the city kept in this browser as JSON: each a
/// junction or where it leaves the map.
#[wasm_bindgen]
pub fn street_ends(edge: u32) -> String {
    json(&vm::city_store::CityStore::new(shared::platform::browser_ports().storage).open().street_ends(edge))
}
