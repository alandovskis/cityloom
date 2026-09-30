//! CityLoom cross-section editor core, compiled to WebAssembly.
//!
//! The model owns every editing rule. The page only draws the view it
//! returns and relays pointer and keyboard input.

pub mod catalogue;
pub mod model;

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

    /// Sets the region, by index into `materials().regions`.
    pub fn set_region(&mut self, region: usize) -> bool {
        self.0.set_region(region)
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
        .map(|s| serde_json::json!({ "name": s.name, "row_mm": s.row_mm }))
        .collect();
    json(&list)
}
