//! CityLoom cross-section editor core, compiled to WebAssembly.
//!
//! The model owns every editing rule. The page only draws the view it
//! returns and relays pointer and keyboard input.

pub mod atlas;
pub mod catalogue;
pub mod junction;
pub mod junction_view;
pub mod model;
pub mod plan;
pub mod street_measures;

use wasm_bindgen::prelude::*;

use catalogue::{CURBS, DIRECTIONS, KINDS, REGIONS, MATERIALS, SAMPLES};
use junction::Target;

fn json<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).expect("view serialises")
}

/// One street being edited.
#[wasm_bindgen]
pub struct Sheet(model::Editor);

#[wasm_bindgen]
impl Sheet {
    #[wasm_bindgen(constructor)]
    pub fn new(sample: usize) -> Sheet {
        Sheet(model::Editor::new(sample))
    }

    pub fn load_sample(&mut self, sample: usize) {
        self.0.load_sample(sample);
    }

    /// The whole drawable state as JSON.
    pub fn view(&self) -> String {
        json(&self.0.view())
    }

    pub fn add(&mut self, kind: usize, index: usize) -> u32 {
        self.0.add(kind, index)
    }

    pub fn remove(&mut self, uid: u32) -> bool {
        self.0.remove(uid)
    }

    pub fn move_to(&mut self, uid: u32, index: usize) -> bool {
        self.0.move_to(uid, index)
    }

    pub fn set_width(&mut self, uid: u32, width_mm: i32) -> bool {
        self.0.set_width(uid, width_mm)
    }

    pub fn nudge_width(&mut self, uid: u32, delta_mm: i32) -> bool {
        self.0.nudge_width(uid, delta_mm)
    }

    /// Sets a piece's surface, by index into `materials().surfaces`.
    pub fn set_material(&mut self, uid: u32, material: usize) -> bool {
        self.0.set_material(uid, material)
    }

    /// Sets a piece's curb, by index into `materials().curbs`; -1 is none.
    pub fn set_curb(&mut self, uid: u32, curb: i32) -> bool {
        self.0.set_curb(uid, usize::try_from(curb).ok())
    }

    /// Sets the region, by index into `materials().regions`.
    pub fn set_region(&mut self, region: usize) -> bool {
        self.0.set_region(region)
    }

    /// Sets the time of day the sheet shows, in minutes after midnight.
    pub fn set_time(&mut self, minutes: i32) -> bool {
        self.0.set_time(minutes)
    }

    /// Gives a piece a different type at certain times.
    pub fn add_variant(&mut self, uid: u32) -> bool {
        self.0.add_variant(uid)
    }

    pub fn set_variant_kind(&mut self, uid: u32, index: usize, kind: usize) -> bool {
        self.0.set_variant_kind(uid, index, kind)
    }

    /// Sets which way an other-times type runs, by index into
    /// `materials().directions`; -1 is two-way.
    pub fn set_variant_direction(&mut self, uid: u32, index: usize, direction: i32) -> bool {
        self.0.set_variant_direction(uid, index, usize::try_from(direction).ok())
    }

    /// Arranges the roadway as an Atlas lane measure, by its code (A1 to F2).
    pub fn apply_measure(&mut self, code: &str) -> bool {
        self.0.apply_measure(code)
    }

    pub fn set_variant_time(&mut self, uid: u32, index: usize, from_min: i32, to_min: i32) -> bool {
        self.0.set_variant_time(uid, index, from_min, to_min)
    }

    pub fn remove_variant(&mut self, uid: u32, index: usize) -> bool {
        self.0.remove_variant(uid, index)
    }

    /// Sets a lane's direction, by index into `materials().directions`; -1 is
    /// two-way, allowed for a bike lane only.
    pub fn set_direction(&mut self, uid: u32, direction: i32) -> bool {
        self.0.set_direction(uid, usize::try_from(direction).ok())
    }

    pub fn begin_gesture(&mut self) {
        self.0.begin_gesture();
    }

    pub fn end_gesture(&mut self) -> bool {
        self.0.end_gesture()
    }

    pub fn cancel_gesture(&mut self) {
        self.0.cancel_gesture();
    }

    pub fn resize_boundary(&mut self, left_index: usize, delta_mm: i32) -> bool {
        self.0.resize_boundary(left_index, delta_mm)
    }

    pub fn resize_edge(&mut self, uid: u32, delta_mm: i32) -> bool {
        self.0.resize_edge(uid, delta_mm)
    }

    pub fn undo(&mut self) -> bool {
        self.0.undo()
    }

    pub fn redo(&mut self) -> bool {
        self.0.redo()
    }

    pub fn reset(&mut self) -> bool {
        self.0.reset()
    }

    /// Selects a segment; 0 clears the selection.
    pub fn select(&mut self, uid: u32) {
        self.0.select((uid != 0).then_some(uid));
    }

    pub fn select_relative(&mut self, delta: i32) {
        self.0.select_relative(delta);
    }

    /// Where a dragged segment would land for a pointer `x_mm` from the
    /// street's left edge. `exclude` is the dragged uid, or 0.
    pub fn drop_index(&self, x_mm: f64, exclude: u32) -> usize {
        self.0.drop_index(x_mm, exclude)
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

/// One junction being edited.
#[wasm_bindgen]
pub struct Plan(junction::Junction);

#[wasm_bindgen]
impl Plan {
    #[wasm_bindgen(constructor)]
    pub fn new(sample: usize) -> Plan {
        Plan(junction::Junction::new(sample))
    }

    pub fn load_sample(&mut self, sample: usize) {
        self.0.load_sample(sample);
    }

    /// The whole drawable state as JSON.
    pub fn view(&self) -> String {
        json(&self.0.view())
    }

    /// Adds a street at `bearing`, or in the widest gap when it is negative.
    pub fn add_arm(&mut self, street: usize, bearing: i32) -> u32 {
        self.0.add_arm(street, bearing)
    }

    pub fn remove_arm(&mut self, uid: u32) -> bool {
        self.0.remove_arm(uid)
    }

    pub fn set_bearing(&mut self, uid: u32, degrees: i32) -> bool {
        self.0.set_bearing(uid, degrees)
    }

    pub fn set_offset(&mut self, uid: u32, mm: i32) -> bool {
        self.0.set_offset(uid, mm)
    }

    /// Sets the curb radius at the corner clockwise of an arm.
    pub fn set_corner(&mut self, uid: u32, mm: i32) -> bool {
        self.0.set_corner(uid, mm)
    }

    pub fn set_street(&mut self, uid: u32, street: usize) -> bool {
        self.0.set_street(uid, street)
    }

    pub fn set_crossing(&mut self, uid: u32, on: bool) -> bool {
        self.0.set_crossing(uid, on)
    }

    pub fn set_setback(&mut self, uid: u32, mm: i32) -> bool {
        self.0.set_setback(uid, mm)
    }

    pub fn set_crossing_width(&mut self, uid: u32, mm: i32) -> bool {
        self.0.set_crossing_width(uid, mm)
    }

    pub fn set_island(&mut self, uid: u32, on: bool) -> bool {
        self.0.set_island(uid, on)
    }

    /// Side 0 is the arm's left curb, 1 its right.
    pub fn set_bulb(&mut self, uid: u32, side: usize, on: bool) -> bool {
        self.0.set_bulb(uid, side, on)
    }

    /// Lets an entering lane go to a street, by the street's uid, or not.
    pub fn set_lane_dest(&mut self, uid: u32, lane: usize, to: u32, on: bool) -> bool {
        self.0.set_lane_dest(uid, lane, to, on)
    }

    pub fn set_turn(&mut self, from: u32, to: u32, allowed: bool) -> bool {
        self.0.set_turn(from, to, allowed)
    }

    pub fn set_bus_lane(&mut self, uid: u32, on: bool) -> bool {
        self.0.set_bus_lane(uid, on)
    }

    /// Sets an arm's approach measure (G1, G2, G3, H1, H2), by index into
    /// `junction_catalogue().approaches`.
    pub fn set_approach(&mut self, uid: u32, approach: usize) -> bool {
        self.0.set_approach(uid, approach)
    }

    pub fn set_approach_len(&mut self, uid: u32, mm: i32) -> bool {
        self.0.set_approach_len(uid, mm)
    }

    /// Sets an arm's bus stop (M1, M2), by index into `junction_catalogue().stops`.
    pub fn set_stop(&mut self, uid: u32, stop: usize) -> bool {
        self.0.set_stop(uid, stop)
    }

    /// Sets an arm's turn management (L1 to L4), by index into `junction_catalogue().rules`.
    pub fn set_rule(&mut self, uid: u32, rule: usize) -> bool {
        self.0.set_rule(uid, rule)
    }

    /// A transit modal filter (N1).
    pub fn set_filter(&mut self, uid: u32, on: bool) -> bool {
        self.0.set_filter(uid, on)
    }

    /// Sets the control, by index into `junction_catalogue().controls`.
    pub fn set_control(&mut self, control: usize) -> bool {
        self.0.set_control(control)
    }

    /// Runs a bus lane across the middle of a roundabout between two streets
    /// (uids); 0 for both takes it away.
    pub fn set_bus(&mut self, a: u32, b: u32) -> bool {
        self.0.set_bus((a != 0 && b != 0).then_some((a, b)))
    }

    /// Puts a cycle track of this width around a roundabout; 0 takes it away.
    pub fn set_cycle(&mut self, width_mm: i32) -> bool {
        self.0.set_cycle((width_mm > 0).then_some(width_mm))
    }

    pub fn set_ring(&mut self, extra_mm: i32) -> bool {
        self.0.set_ring(extra_mm)
    }

    /// Sets the region, by index into `materials().regions`.
    pub fn set_region(&mut self, region: usize) -> bool {
        self.0.set_region(region)
    }

    pub fn begin_gesture(&mut self) {
        self.0.begin_gesture();
    }

    pub fn end_gesture(&mut self) -> bool {
        self.0.end_gesture()
    }

    pub fn cancel_gesture(&mut self) {
        self.0.cancel_gesture();
    }

    pub fn undo(&mut self) -> bool {
        self.0.undo()
    }

    pub fn redo(&mut self) -> bool {
        self.0.redo()
    }

    pub fn reset(&mut self) -> bool {
        self.0.reset()
    }

    /// Selects an arm (1), a corner (2), a crossing (3) by the arm's uid, or the
    /// bus lane (5) or cycle track (6);
    /// kind 0 clears the selection.
    pub fn select(&mut self, kind: u8, uid: u32) {
        self.0.select(match kind {
            1 => Target::Arm(uid),
            2 => Target::Corner(uid),
            3 => Target::Crossing(uid),
            5 => Target::Bus,
            6 => Target::Cycle,
            _ => Target::None,
        });
    }

    /// Selects one entering lane of an arm.
    pub fn select_lane(&mut self, uid: u32, lane: usize) {
        self.0.select(Target::Lane(uid, lane));
    }

    pub fn select_relative(&mut self, delta: i32) {
        self.0.select_relative(delta);
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

/// The Transit Priority Atlas toolbox measures and where each is modelled, as JSON.
#[wasm_bindgen]
pub fn atlas() -> String {
    json(&atlas::MEASURES)
}
