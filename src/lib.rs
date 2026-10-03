//! CityLoom cross-section editor core, compiled to WebAssembly.
//!
//! The model owns every editing rule. The page only draws the view it
//! returns and relays pointer and keyboard input.

#![recursion_limit = "1024"]

pub mod atlas;
pub mod catalogue;
pub mod city;
pub mod junction;
pub mod junction_view;
pub mod model;
pub mod plan;
pub mod street_measures;
pub mod symbols;
pub mod units;
pub mod vm;
pub mod ui;

use wasm_bindgen::prelude::*;

use catalogue::{CURBS, DIRECTIONS, KINDS, REGIONS, MATERIALS, SAMPLES};

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
        Sheet(vm::street::StreetVm::new(ui::platform::browser_ports(), model::Editor::new(sample), None))
    }

    /// Sets the units lengths are shown in, `"m"` or `"ft"`, in the parts of the
    /// page drawn by the components.
    pub fn set_units(&self, units: &str) {
        self.0.set_units(units::Units::parse(units));
    }

    /// Sets the region whose materials and rules apply.
    pub fn set_region(&mut self, region: usize) -> bool {
        self.0.edit(|e| e.set_region(region))
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
        Plan(vm::junction::JunctionVm::new(ui::platform::browser_ports(), junction::Junction::new(sample), None))
    }

    /// Sets the units lengths are shown in, `"m"` or `"ft"`, in the parts of the
    /// page drawn by the components.
    pub fn set_units(&self, units: &str) {
        self.0.set_units(units::Units::parse(units));
    }

    /// Sets the region whose materials and rules apply.
    pub fn set_region(&mut self, region: usize) -> bool {
        self.0.edit(|j| j.set_region(region))
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

/// The city: its streets and junctions as the editors have left them. The
/// page keeps `save()` in storage and gives it back to `new` next time.
#[wasm_bindgen]
pub struct City(city::City);

#[wasm_bindgen]
impl City {
    /// The city as saved, or as first laid out when `saved` is empty or cannot be used.
    #[wasm_bindgen(constructor)]
    pub fn new(saved: &str) -> City {
        City(city::City::load(saved))
    }

    pub fn save(&self) -> String {
        self.0.save()
    }

    /// Every street and junction, and whether each works, as JSON.
    pub fn view(&self, region: usize) -> String {
        json(&self.0.view(region))
    }

    /// Puts every street and junction back as first laid out.
    pub fn reset(&mut self) {
        self.0.reset();
    }

    /// The name of a street, by its uid; empty when there is none.
    pub fn street_name(&self, edge: u32) -> String {
        self.0.street_name(edge).unwrap_or_default()
    }

    /// The name of a junction, by its uid; empty when there is none.
    pub fn junction_name(&self, node: u32) -> String {
        self.0.junction_name(node).unwrap_or_default()
    }

    /// The two ends of a street as JSON: each a junction or where it leaves the map.
    pub fn street_ends(&self, edge: u32) -> String {
        json(&self.0.street_ends(edge))
    }
}

/// The name of a junction of the city kept in this browser; empty when there is none.
#[wasm_bindgen]
pub fn junction_name(node: u32) -> String {
    vm::city_store::CityStore::new(ui::platform::browser_ports().storage).open().junction_name(node).unwrap_or_default()
}

/// The junction editor on one junction of the city kept in this browser, which
/// writes what is made back to the city. Nothing when there is no such junction
/// or it cannot be drawn with the streets as they now are.
#[wasm_bindgen]
pub fn open_junction(node: u32) -> Option<Plan> {
    let ports = ui::platform::browser_ports();
    let store = vm::city_store::CityStore::new(ports.storage.clone());
    let junction = store.open().junction_editor(node, store.region())?;
    let place = vm::binding::Place::Junction(node);
    Some(Plan(vm::junction::JunctionVm::new(ports, junction, Some(vm::binding::CityBinding { store, place }))))
}

/// The street editor on one street of the city kept in this browser, which
/// writes what is made back to the city. Nothing when there is no such street.
#[wasm_bindgen]
pub fn open_street(edge: u32) -> Option<Sheet> {
    let ports = ui::platform::browser_ports();
    let store = vm::city_store::CityStore::new(ports.storage.clone());
    let street = store.open().street_editor(edge, store.region())?;
    let place = vm::binding::Place::Street(edge);
    Some(Sheet(vm::street::StreetVm::new(ports, street, Some(vm::binding::CityBinding { store, place }))))
}
