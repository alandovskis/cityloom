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

/// One street being edited.
#[wasm_bindgen]
pub struct Sheet(std::rc::Rc<ui::sheet::SharedSheet>);

#[wasm_bindgen]
impl Sheet {
    #[wasm_bindgen(constructor)]
    pub fn new(sample: usize) -> Sheet {
        Sheet(ui::sheet::SharedSheet::new(model::Editor::new(sample)))
    }

    /// Says what to call, with no arguments, after any change to this street:
    /// the page keeps what the city holds.
    pub fn on_change(&self, callback: js_sys::Function) {
        self.0.set_on_change(callback);
    }

    /// Sets the units lengths are shown in, `"m"` or `"ft"`, in the parts of the
    /// page drawn by the components.
    pub fn set_units(&self, units: &str) {
        self.0.set_units(units::Units::parse(units));
    }

    pub fn load_sample(&mut self, sample: usize) {
        self.0.edit(|e| e.load_sample(sample))
    }

    /// The whole drawable state as JSON.
    pub fn view(&self) -> String {
        json(&*self.0.view())
    }

    pub fn add(&mut self, kind: usize, index: usize) -> u32 {
        self.0.edit(|e| e.add(kind, index))
    }

    pub fn remove(&mut self, uid: u32) -> bool {
        self.0.edit(|e| e.remove(uid))
    }

    pub fn move_to(&mut self, uid: u32, index: usize) -> bool {
        self.0.edit(|e| e.move_to(uid, index))
    }

    pub fn set_width(&mut self, uid: u32, width_mm: i32) -> bool {
        self.0.edit(|e| e.set_width(uid, width_mm))
    }

    pub fn nudge_width(&mut self, uid: u32, delta_mm: i32) -> bool {
        self.0.edit(|e| e.nudge_width(uid, delta_mm))
    }

    /// Sets a piece's surface, by index into `materials().surfaces`.
    pub fn set_material(&mut self, uid: u32, material: usize) -> bool {
        self.0.edit(|e| e.set_material(uid, material))
    }

    /// Sets a piece's curb, by index into `materials().curbs`; -1 is none.
    pub fn set_curb(&mut self, uid: u32, curb: i32) -> bool {
        self.0.edit(|e| e.set_curb(uid, usize::try_from(curb).ok()))
    }

    /// Sets whether a transit lane carries trams (true) or buses.
    pub fn set_tram(&mut self, uid: u32, tram: bool) -> bool {
        self.0.edit(|e| e.set_tram(uid, tram))
    }

    /// Puts a shelter on a sidewalk beside a transit lane, or takes it away.
    pub fn set_shelter(&mut self, uid: u32, shelter: bool) -> bool {
        self.0.edit(|e| e.set_shelter(uid, shelter))
    }

    /// Sets the region, by index into `materials().regions`.
    pub fn set_region(&mut self, region: usize) -> bool {
        self.0.edit(|e| e.set_region(region))
    }

    /// Sets the time of day the sheet shows, in minutes after midnight.
    pub fn set_time(&mut self, minutes: i32) -> bool {
        self.0.edit(|e| e.set_time(minutes))
    }

    /// Gives a piece a different type at certain times.
    pub fn add_variant(&mut self, uid: u32) -> bool {
        self.0.edit(|e| e.add_variant(uid))
    }

    pub fn set_variant_kind(&mut self, uid: u32, index: usize, kind: usize) -> bool {
        self.0.edit(|e| e.set_variant_kind(uid, index, kind))
    }

    /// Sets which way an other-times type runs, by index into
    /// `materials().directions`; -1 is two-way.
    pub fn set_variant_direction(&mut self, uid: u32, index: usize, direction: i32) -> bool {
        self.0.edit(|e| e.set_variant_direction(uid, index, usize::try_from(direction).ok()))
    }

    /// Arranges the roadway as an Atlas lane measure, by its code (A1 to F2).
    pub fn apply_measure(&mut self, code: &str) -> bool {
        self.0.edit(|e| e.apply_measure(code))
    }

    pub fn set_variant_time(&mut self, uid: u32, index: usize, from_min: i32, to_min: i32) -> bool {
        self.0.edit(|e| e.set_variant_time(uid, index, from_min, to_min))
    }

    pub fn remove_variant(&mut self, uid: u32, index: usize) -> bool {
        self.0.edit(|e| e.remove_variant(uid, index))
    }

    /// Sets a lane's direction, by index into `materials().directions`; -1 is
    /// two-way, allowed for a bike lane only.
    pub fn set_direction(&mut self, uid: u32, direction: i32) -> bool {
        self.0.edit(|e| e.set_direction(uid, usize::try_from(direction).ok()))
    }

    pub fn begin_gesture(&mut self) {
        self.0.edit(|e| e.begin_gesture())
    }

    pub fn end_gesture(&mut self) -> bool {
        self.0.edit(|e| e.end_gesture())
    }

    pub fn cancel_gesture(&mut self) {
        self.0.edit(|e| e.cancel_gesture())
    }

    pub fn resize_boundary(&mut self, left_index: usize, delta_mm: i32) -> bool {
        self.0.edit(|e| e.resize_boundary(left_index, delta_mm))
    }

    pub fn resize_edge(&mut self, uid: u32, delta_mm: i32) -> bool {
        self.0.edit(|e| e.resize_edge(uid, delta_mm))
    }

    pub fn undo(&mut self) -> bool {
        self.0.edit(|e| e.undo())
    }

    pub fn redo(&mut self) -> bool {
        self.0.edit(|e| e.redo())
    }

    pub fn reset(&mut self) -> bool {
        self.0.edit(|e| e.reset())
    }

    /// Selects a segment; 0 clears the selection.
    pub fn select(&mut self, uid: u32) {
        self.0.edit(|e| e.select((uid != 0).then_some(uid)))
    }

    pub fn select_relative(&mut self, delta: i32) {
        self.0.edit(|e| e.select_relative(delta))
    }

    /// Where a dragged segment would land for a pointer `x_mm` from the
    /// street's left edge. `exclude` is the dragged uid, or 0.
    pub fn drop_index(&self, x_mm: f64, exclude: u32) -> usize {
        self.0.read(|e| e.drop_index(x_mm, exclude))
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

    /// The street editor on one street, or nothing when there is no such street.
    pub fn street(&self, edge: u32, region: usize) -> Option<Sheet> {
        self.0.street_editor(edge, region).map(|e| Sheet(ui::sheet::SharedSheet::new(e)))
    }

    /// Keeps what a street editor has made of the street.
    pub fn keep_street(&mut self, edge: u32, sheet: &Sheet) -> bool {
        self.0.keep_street(edge, sheet.0.read(|e| e.snapshot()))
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
