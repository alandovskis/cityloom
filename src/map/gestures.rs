//! What the pointer does to the map: a drag pans it, two fingers pinch it, and
//! a drag that ends over a place must not open it. The view forwards raw
//! pointers; this decides what they mean and moves the camera.

use std::collections::HashMap;

use crate::map::camera::Camera;

/// How far a pressed pointer moves before it is a drag, in pixels.
const DRAG_START_PX: f64 = 5.0;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Pan {
    id: i32,
    x: f64,
    y: f64,
    cx: f64,
    cy: f64,
    active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Pinch {
    distance: f64,
    k: f64,
}

/// What a moved pointer did, when the view has something to do about it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moved {
    Nothing,
    /// A drag has just begun: the view takes hold of the pointer.
    StartedPanning,
    Panned,
    Pinched,
}

#[derive(Default)]
pub struct MapGestures {
    pointers: HashMap<i32, (f64, f64)>,
    pan: Option<Pan>,
    pinch: Option<Pinch>,
    swallow_click: bool,
}

impl MapGestures {
    /// A pointer went down. A second one starts a pinch instead of a drag.
    pub fn down(&mut self, camera: &Camera, id: i32, x: f64, y: f64) {
        self.pointers.insert(id, (x, y));
        match self.pointers.len() {
            2 => {
                let mut points = self.pointers.values();
                let (a, b) = (points.next().copied().unwrap(), points.next().copied().unwrap());
                self.pinch = Some(Pinch { distance: (a.0 - b.0).hypot(a.1 - b.1), k: camera.k });
                self.pan = None;
            }
            1 => self.pan = Some(Pan { id, x, y, cx: camera.cx, cy: camera.cy, active: false }),
            _ => {}
        }
    }

    /// A pointer moved, and the camera moves with it. `origin` is the middle of
    /// the window in page pixels, for zooming about a point.
    pub fn moved(&mut self, camera: &mut Camera, id: i32, x: f64, y: f64, origin: (f64, f64)) -> Moved {
        if !self.pointers.contains_key(&id) {
            return Moved::Nothing;
        }
        self.pointers.insert(id, (x, y));
        if let (Some(pinch), 2) = (self.pinch, self.pointers.len()) {
            let mut points = self.pointers.values();
            let (a, b) = (points.next().copied().unwrap(), points.next().copied().unwrap());
            let distance = (a.0 - b.0).hypot(a.1 - b.1);
            camera.zoom_at(pinch.k * distance / pinch.distance, (a.0 + b.0) / 2.0 - origin.0, (a.1 + b.1) / 2.0 - origin.1);
            self.swallow_click = true;
            return Moved::Pinched;
        }
        let Some(pan) = &mut self.pan else { return Moved::Nothing };
        if pan.id != id {
            return Moved::Nothing;
        }
        let (dx, dy) = (x - pan.x, y - pan.y);
        if !pan.active && dx.hypot(dy) < DRAG_START_PX {
            return Moved::Nothing;
        }
        let started = !pan.active;
        pan.active = true;
        camera.cx = pan.cx - dx / camera.k;
        camera.cy = pan.cy - dy / camera.k;
        camera.moved = true;
        camera.clamp();
        if started { Moved::StartedPanning } else { Moved::Panned }
    }

    /// A pointer came up or was cancelled. Says whether a drag has ended.
    pub fn up(&mut self, id: i32) -> bool {
        self.pointers.remove(&id);
        let mut ended = false;
        if self.pan.is_some_and(|p| p.id == id) {
            let active = self.pan.is_some_and(|p| p.active);
            self.swallow_click = self.swallow_click || active;
            self.pan = None;
            ended = active;
        }
        if self.pointers.len() < 2 {
            self.pinch = None;
        }
        ended
    }

    /// Whether a click is the end of a drag or pinch, and so not a click. Asking
    /// uses it up.
    pub fn swallow_click(&mut self) -> bool {
        std::mem::take(&mut self.swallow_click)
    }

    /// Whether the map is being dragged.
    pub fn panning(&self) -> bool {
        self.pan.is_some_and(|p| p.active)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::camera::World;

    fn camera() -> Camera {
        Camera::new(World::round([0, 0, 1_000_000, 500_000]), 800.0, 520.0)
    }

    const ORIGIN: (f64, f64) = (400.0, 260.0);

    #[test]
    fn a_pointer_that_barely_moves_is_a_press_and_not_a_drag() {
        let (mut g, mut c) = (MapGestures::default(), camera());
        g.down(&c, 1, 100.0, 100.0);
        assert_eq!(g.moved(&mut c, 1, 103.0, 102.0, ORIGIN), Moved::Nothing);
        assert!(!g.panning());
        assert!(!g.up(1));
        assert!(!g.swallow_click(), "a click is a click");
    }

    #[test]
    fn dragging_pans_the_map_the_opposite_way_and_the_click_that_ends_it_is_swallowed() {
        let (mut g, mut c) = (MapGestures::default(), camera());
        let (cx, cy, k) = (c.cx, c.cy, c.k);
        g.down(&c, 1, 100.0, 100.0);
        assert_eq!(g.moved(&mut c, 1, 140.0, 80.0, ORIGIN), Moved::StartedPanning);
        assert!(g.panning());
        assert!((c.cx - (cx - 40.0 / k)).abs() < 1e-9 && (c.cy - (cy + 20.0 / k)).abs() < 1e-9);
        assert_eq!(g.moved(&mut c, 1, 150.0, 80.0, ORIGIN), Moved::Panned);
        assert!(c.moved);
        assert!(g.up(1));
        assert!(g.swallow_click());
        assert!(!g.swallow_click(), "only once");
    }

    #[test]
    fn a_pan_is_measured_from_where_the_drag_began_not_from_the_last_move() {
        let (mut g, mut c) = (MapGestures::default(), camera());
        let (cx, k) = (c.cx, c.k);
        g.down(&c, 1, 100.0, 100.0);
        g.moved(&mut c, 1, 120.0, 100.0, ORIGIN);
        g.moved(&mut c, 1, 160.0, 100.0, ORIGIN);
        assert!((c.cx - (cx - 60.0 / k)).abs() < 1e-9);
    }

    #[test]
    fn two_pointers_pinch_about_their_middle_and_the_click_after_is_swallowed() {
        let (mut g, mut c) = (MapGestures::default(), camera());
        let k0 = c.k;
        g.down(&c, 1, 300.0, 260.0);
        g.down(&c, 2, 500.0, 260.0);
        assert_eq!(g.moved(&mut c, 2, 700.0, 260.0, ORIGIN), Moved::Pinched);
        // The fingers are twice as far apart: twice as close.
        assert!((c.k - k0 * 2.0).abs() < 1e-9);
        g.up(2);
        g.up(1);
        assert!(g.swallow_click());
    }

    #[test]
    fn a_second_pointer_stops_a_drag_and_lifting_one_of_two_ends_the_pinch() {
        let (mut g, mut c) = (MapGestures::default(), camera());
        g.down(&c, 1, 100.0, 100.0);
        g.moved(&mut c, 1, 150.0, 100.0, ORIGIN);
        g.down(&c, 2, 300.0, 300.0);
        assert!(!g.panning());
        g.up(2);
        // The first pointer's drag is not resumed, as it was cancelled by the second.
        assert_eq!(g.moved(&mut c, 1, 200.0, 100.0, ORIGIN), Moved::Nothing);
    }

    #[test]
    fn a_pointer_that_never_went_down_moves_nothing() {
        let (mut g, mut c) = (MapGestures::default(), camera());
        let before = c;
        assert_eq!(g.moved(&mut c, 9, 5.0, 5.0, ORIGIN), Moved::Nothing);
        assert_eq!(c, before);
    }

    #[test]
    fn a_pan_cannot_take_the_map_off_the_city() {
        let (mut g, mut c) = (MapGestures::default(), camera());
        g.down(&c, 1, 0.0, 0.0);
        g.moved(&mut c, 1, 1e9, 1e9, ORIGIN);
        assert_eq!((c.cx, c.cy), (c.world.x0, c.world.y0));
    }
}
