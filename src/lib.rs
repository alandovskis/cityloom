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
pub mod units;
pub mod ui;

use wasm_bindgen::prelude::*;

use catalogue::{CURBS, DIRECTIONS, KINDS, REGIONS, MATERIALS, SAMPLES};

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

    /// Sets whether a transit lane carries trams (true) or buses.
    pub fn set_tram(&mut self, uid: u32, tram: bool) -> bool {
        self.0.set_tram(uid, tram)
    }

    /// Puts a shelter on a sidewalk beside a transit lane, or takes it away.
    pub fn set_shelter(&mut self, uid: u32, shelter: bool) -> bool {
        self.0.set_shelter(uid, shelter)
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
pub struct Plan(std::rc::Rc<ui::shared::Shared>);

#[wasm_bindgen]
impl Plan {
    #[wasm_bindgen(constructor)]
    pub fn new(sample: usize) -> Plan {
        Plan(ui::shared::Shared::new(junction::Junction::new(sample)))
    }

    pub fn load_sample(&mut self, sample: usize) {
        self.0.edit(|j| j.load_sample(sample))
    }

    /// Sets the units lengths are shown in, `"m"` or `"ft"`, in the parts of the
    /// page drawn by the components.
    pub fn set_units(&self, units: &str) {
        self.0.set_units(units::Units::parse(units));
    }

    /// Says what to call after the page's own components have changed this
    /// junction: with whether the model took it, and `"edit"` or `"select"`.
    pub fn on_edit(&self, callback: js_sys::Function) {
        self.0.set_on_edit(callback);
    }

    /// The whole drawable state as JSON.
    pub fn view(&self) -> String {
        json(&*self.0.view())
    }

    /// Adds a street at `bearing`, or in the widest gap when it is negative.
    pub fn add_arm(&mut self, street: usize, bearing: i32) -> u32 {
        self.0.edit(|j| j.add_arm(street, bearing))
    }

    /// Why the last edit was refused, in words. Read it after a call returned false.
    pub fn refusal(&self) -> String {
        self.0.read(|j| j.refusal().unwrap_or(junction::Refusal::DoesNotFit).message().to_string())
    }

    /// Adds a street facing the point (x, y) on the plan, in millimetres from
    /// the junction's centre. Returns its uid, or 0 when there is no room.
    pub fn add_arm_toward(&mut self, street: usize, x: f64, y: f64) -> u32 {
        self.0.edit(|j| j.add_arm_toward(street, x, y))
    }

    pub fn remove_arm(&mut self, uid: u32) -> bool {
        self.0.edit(|j| j.remove_arm(uid))
    }

    /// Turns an arm to face the point (x, y) on the plan.
    pub fn drag_arm_to(&mut self, uid: u32, x: f64, y: f64) -> bool {
        self.0.edit(|j| j.drag_arm_to(uid, x, y))
    }

    /// Sets the corner radius clockwise of an arm from the corner handle's
    /// dragged position (x, y).
    pub fn drag_corner_to(&mut self, uid: u32, x: f64, y: f64) -> bool {
        self.0.edit(|j| j.drag_corner_to(uid, x, y))
    }

    /// Sets a crossing's setback from the crossing handle's dragged position (x, y).
    pub fn drag_crossing_to(&mut self, uid: u32, x: f64, y: f64) -> bool {
        self.0.edit(|j| j.drag_crossing_to(uid, x, y))
    }

    pub fn set_bearing(&mut self, uid: u32, degrees: i32) -> bool {
        self.0.edit(|j| j.set_bearing(uid, degrees))
    }

    pub fn set_offset(&mut self, uid: u32, mm: i32) -> bool {
        self.0.edit(|j| j.set_offset(uid, mm))
    }

    /// Sets the curb radius at the corner clockwise of an arm.
    pub fn set_corner(&mut self, uid: u32, mm: i32) -> bool {
        self.0.edit(|j| j.set_corner(uid, mm))
    }

    pub fn set_street(&mut self, uid: u32, street: usize) -> bool {
        self.0.edit(|j| j.set_street(uid, street))
    }

    pub fn set_crossing(&mut self, uid: u32, on: bool) -> bool {
        self.0.edit(|j| j.set_crossing(uid, on))
    }

    pub fn set_setback(&mut self, uid: u32, mm: i32) -> bool {
        self.0.edit(|j| j.set_setback(uid, mm))
    }

    pub fn set_crossing_width(&mut self, uid: u32, mm: i32) -> bool {
        self.0.edit(|j| j.set_crossing_width(uid, mm))
    }

    pub fn set_island(&mut self, uid: u32, on: bool) -> bool {
        self.0.edit(|j| j.set_island(uid, on))
    }

    /// Side 0 is the arm's left curb, 1 its right.
    pub fn set_bulb(&mut self, uid: u32, side: usize, on: bool) -> bool {
        self.0.edit(|j| j.set_bulb(uid, side, on))
    }

    /// Lets an entering lane go to a street, by the street's uid, or not.
    pub fn set_lane_dest(&mut self, uid: u32, lane: usize, to: u32, on: bool) -> bool {
        self.0.edit(|j| j.set_lane_dest(uid, lane, to, on))
    }

    pub fn set_turn(&mut self, from: u32, to: u32, allowed: bool) -> bool {
        self.0.edit(|j| j.set_turn(from, to, allowed))
    }

    pub fn set_bus_lane(&mut self, uid: u32, on: bool) -> bool {
        self.0.edit(|j| j.set_bus_lane(uid, on))
    }

    /// Sets an arm's approach measure (G1, G2, G3, H1, H2), by index into
    /// `junction_catalogue().approaches`.
    pub fn set_approach(&mut self, uid: u32, approach: usize) -> bool {
        self.0.edit(|j| j.set_approach(uid, approach))
    }

    pub fn set_approach_len(&mut self, uid: u32, mm: i32) -> bool {
        self.0.edit(|j| j.set_approach_len(uid, mm))
    }

    /// Sets an arm's bus stop (M1, M2), by index into `junction_catalogue().stops`.
    pub fn set_stop(&mut self, uid: u32, stop: usize) -> bool {
        self.0.edit(|j| j.set_stop(uid, stop))
    }

    /// Sets an arm's turn management (L1 to L4), by index into `junction_catalogue().rules`.
    pub fn set_rule(&mut self, uid: u32, rule: usize) -> bool {
        self.0.edit(|j| j.set_rule(uid, rule))
    }

    /// A transit modal filter (N1).
    pub fn set_filter(&mut self, uid: u32, on: bool) -> bool {
        self.0.edit(|j| j.set_filter(uid, on))
    }

    /// Sets the control, by index into `junction_catalogue().controls`.
    pub fn set_control(&mut self, control: usize) -> bool {
        self.0.edit(|j| j.set_control(control))
    }

    /// Runs a bus lane across the middle of a roundabout between two streets
    /// (uids); 0 for both takes it away.
    pub fn set_bus(&mut self, a: u32, b: u32) -> bool {
        self.0.edit(|j| j.set_bus((a != 0 && b != 0).then_some((a, b))))
    }

    /// Puts a cycle track of this width around a roundabout; 0 takes it away.
    pub fn set_cycle(&mut self, width_mm: i32) -> bool {
        self.0.edit(|j| j.set_cycle((width_mm > 0).then_some(width_mm)))
    }

    pub fn set_ring(&mut self, extra_mm: i32) -> bool {
        self.0.edit(|j| j.set_ring(extra_mm))
    }

    /// Sets the roundabout's outside radius; smaller than its least size is the least size.
    pub fn set_ring_radius(&mut self, radius_mm: i32) -> bool {
        self.0.edit(|j| j.set_ring_radius(radius_mm))
    }

    /// Puts a cycle track at its usual width around a roundabout, or takes it away.
    pub fn set_cycle_track(&mut self, on: bool) -> bool {
        self.0.edit(|j| j.set_cycle_track(on))
    }

    /// The `step_*` calls move one number by its step, `dir` 1 up and -1 down,
    /// within its limits.
    pub fn step_bearing(&mut self, uid: u32, dir: i32) -> bool {
        self.0.edit(|j| j.step_bearing(uid, dir))
    }

    pub fn step_offset(&mut self, uid: u32, dir: i32) -> bool {
        self.0.edit(|j| j.step_offset(uid, dir))
    }

    pub fn step_corner(&mut self, uid: u32, dir: i32) -> bool {
        self.0.edit(|j| j.step_corner(uid, dir))
    }

    pub fn step_setback(&mut self, uid: u32, dir: i32) -> bool {
        self.0.edit(|j| j.step_setback(uid, dir))
    }

    pub fn step_crossing_width(&mut self, uid: u32, dir: i32) -> bool {
        self.0.edit(|j| j.step_crossing_width(uid, dir))
    }

    pub fn step_approach_len(&mut self, uid: u32, dir: i32) -> bool {
        self.0.edit(|j| j.step_approach_len(uid, dir))
    }

    pub fn step_ring(&mut self, dir: i32) -> bool {
        self.0.edit(|j| j.step_ring(dir))
    }

    pub fn step_cycle(&mut self, dir: i32) -> bool {
        self.0.edit(|j| j.step_cycle(dir))
    }

    /// Sets the region, by index into `materials().regions`.
    pub fn set_region(&mut self, region: usize) -> bool {
        self.0.edit(|j| j.set_region(region))
    }

    pub fn begin_gesture(&mut self) {
        self.0.edit(|j| j.begin_gesture())
    }

    pub fn end_gesture(&mut self) -> bool {
        self.0.edit(|j| j.end_gesture())
    }

    pub fn cancel_gesture(&mut self) {
        self.0.edit(|j| j.cancel_gesture())
    }

    pub fn undo(&mut self) -> bool {
        self.0.edit(|j| j.undo())
    }

    pub fn redo(&mut self) -> bool {
        self.0.edit(|j| j.redo())
    }

    pub fn reset(&mut self) -> bool {
        self.0.edit(|j| j.reset())
    }

    /// Selects what the view calls `kind` (its `selected.kind`: arm, corner,
    /// crossing, lane, bus or cycle) at `uid`, and at `lane` for a lane; any
    /// other name clears the selection.
    pub fn select(&mut self, kind: &str, uid: u32, lane: usize) {
        self.0.edit(|j| j.select(junction::Target::named(kind, uid, lane)))
    }

    pub fn select_relative(&mut self, delta: i32) {
        self.0.edit(|j| j.select_relative(delta))
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
        self.0.street_editor(edge, region).map(Sheet)
    }

    /// Keeps what a street editor has made of the street.
    pub fn keep_street(&mut self, edge: u32, sheet: &Sheet) -> bool {
        self.0.keep_street(edge, sheet.0.snapshot())
    }

    /// The junction editor on one junction, or nothing when there is no such
    /// junction or it cannot be drawn with the streets as they now are.
    pub fn junction(&self, node: u32, region: usize) -> Option<Plan> {
        self.0.junction_editor(node, region).map(|j| Plan(ui::shared::Shared::new(j)))
    }

    /// Keeps what a junction editor has made of the junction.
    pub fn keep_junction(&mut self, node: u32, plan: &Plan) -> bool {
        self.0.keep_junction(node, plan.0.read(|j| j.snapshot()))
    }
}
