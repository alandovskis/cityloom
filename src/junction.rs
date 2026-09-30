//! The junction editing model: three to five streets meeting in plan, with
//! corners, crossings, the lanes each street brings, turn bans and a control.
//! Pure Rust like `model.rs`, so it is tested natively. Lengths are integer
//! millimetres and angles integer degrees. Everything here is synthetic.

use crate::catalogue::{KINDS, Material, Mode, REGIONS, SAMPLES, Side};
use crate::model::Editor;
use crate::plan::gap;

pub const LEFT: u8 = 1;
pub const THROUGH: u8 = 2;
pub const RIGHT: u8 = 4;
const CLASSES: [u8; 3] = [LEFT, THROUGH, RIGHT];

pub const UNCONTROLLED: usize = 0;
pub const PRIORITY: usize = 1;
pub const ALL_WAY_STOP: usize = 2;
pub const SIGNAL: usize = 3;
pub const ROUNDABOUT: usize = 4;

pub const CONTROLS: [Material; 5] = [
    Material { id: "uncontrolled", name: "No control" },
    Material { id: "priority", name: "Side streets stop" },
    Material { id: "stop", name: "All-way stop" },
    Material { id: "signal", name: "Traffic signal" },
    Material { id: "roundabout", name: "Roundabout" },
];

pub const MAX_ARMS: usize = 5;
pub const MIN_ARMS: usize = 3;
pub const MIN_SEPARATION: i32 = 30;
pub const BEARING_STEP: i32 = 5;
pub const ARM_LENGTH_MM: i32 = 36_000;

pub const DEFAULT_CORNER_MM: i32 = 6_000;
pub const MIN_CORNER_MM: i32 = 1_000;
pub const MAX_CORNER_MM: i32 = 15_000;
pub const DEFAULT_SETBACK_MM: i32 = 3_000;
pub const MIN_SETBACK_MM: i32 = 2_000;
pub const MAX_SETBACK_MM: i32 = 8_000;
pub const DEFAULT_CROSSING_MM: i32 = 3_000;
pub const MIN_CROSSING_MM: i32 = 2_000;
pub const MAX_CROSSING_MM: i32 = 6_000;
/// A refuge island is this wide, and needs a carriageway at least this much
/// wider than two travel lanes of it.
pub const ISLAND_MM: i32 = 2_000;
pub const ISLAND_MIN_ROAD_MM: i32 = 9_000;
pub const OFFSET_STEP_MM: i32 = 100;
pub const MAX_RING_MM: i32 = 40_000;
pub const RING_STEP_MM: i32 = 500;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Crossing {
    pub setback_mm: i32,
    pub width_mm: i32,
    pub island: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arm {
    pub uid: u32,
    /// Index into `SAMPLES`: the street whose section this arm reads.
    pub street: usize,
    /// Clockwise from north, a multiple of `BEARING_STEP`.
    pub bearing: i32,
    /// Sideways shift of the arm's axis, to the arm's right.
    pub offset_mm: i32,
    /// Curb radius at the corner clockwise of this arm.
    pub corner_mm: i32,
    /// What each entering lane serves, driver's left to right.
    pub lanes: Vec<u8>,
    pub crossing: Option<Crossing>,
    /// Curb extension at the arm's left and right curb.
    pub bulb: [bool; 2],
    /// Arms this arm may not turn into.
    pub banned: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub label: String,
    pub arms: Vec<Arm>,
    pub control: usize,
    /// Roundabout size above the least that keeps the arms apart.
    pub ring_extra_mm: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    None,
    Arm(u32),
    Corner(u32),
    Crossing(u32),
}

// ---- the street an arm reads ------------------------------------------------

/// One piece of an arm's section, as read from its sample street.
#[derive(Clone, Debug)]
pub struct Piece {
    pub kind: usize,
    pub material: &'static str,
    pub x_mm: i32,
    pub width_mm: i32,
    pub direction: Option<&'static str>,
}

/// A street's section as the junction needs it. `x_mm` runs from the arm's
/// left edge to its right edge as seen looking outward from the junction, so
/// "away" traffic leaves the junction and "toward" traffic enters it.
#[derive(Clone, Debug)]
pub struct Profile {
    pub name: &'static str,
    pub row_mm: i32,
    pub pieces: Vec<Piece>,
    /// Extent of the carriageway across the section.
    pub road_l: i32,
    pub road_r: i32,
    /// Centres of the entering and leaving driving lanes. Entering lanes are
    /// ordered as a driver sees them, left to right.
    pub enter_x: Vec<i32>,
    pub leave_x: Vec<i32>,
    /// Extent across the section of the entering and leaving driving lanes.
    pub enter_span: (i32, i32),
    pub leave_span: (i32, i32),
    /// Width of parking or loading beside the left and right curb, or 0.
    pub park: [i32; 2],
}

pub fn is_roadway(kind: usize) -> bool {
    !matches!(KINDS[kind].mode, Mode::Foot | Mode::Green)
}

pub fn profile(street: usize, region: usize) -> Profile {
    let street = street.min(SAMPLES.len() - 1);
    let mut e = Editor::new(street);
    e.set_region(region);
    let view = e.view();
    let pieces: Vec<Piece> = view
        .segments
        .iter()
        .map(|s| Piece { kind: s.kind, material: s.material, x_mm: s.x_mm, width_mm: s.width_mm, direction: s.direction })
        .collect();
    let road: Vec<&Piece> = pieces.iter().filter(|p| is_roadway(p.kind)).collect();
    let road_l = road.first().map_or(0, |p| p.x_mm);
    let road_r = road.last().map_or(view.row_mm, |p| p.x_mm + p.width_mm);
    let lanes = |dir: &str| {
        let mut v: Vec<i32> = pieces
            .iter()
            .filter(|p| KINDS[p.kind].id == "travel" && p.direction == Some(dir))
            .map(|p| p.x_mm + p.width_mm / 2)
            .collect();
        v.sort_unstable_by(|a, b| b.cmp(a));
        v
    };
    let span = |dir: &str| {
        let mut it = pieces.iter().filter(|p| KINDS[p.kind].id == "travel" && p.direction == Some(dir));
        it.next().map_or((0, 0), |first| {
            let (lo, hi) = it.fold((first.x_mm, first.x_mm + first.width_mm), |(lo, hi), p| (lo.min(p.x_mm), hi.max(p.x_mm + p.width_mm)));
            (lo, hi)
        })
    };
    let park = |p: Option<&&Piece>| p.map_or(0, |p| if matches!(KINDS[p.kind].id, "parking" | "loading") { p.width_mm } else { 0 });
    let (enter_x, leave_x, enter_span, leave_span) = (lanes("toward"), lanes("away"), span("toward"), span("away"));
    Profile {
        name: SAMPLES[street].name,
        row_mm: view.row_mm,
        park: [park(road.first()), park(road.last())],
        pieces,
        road_l,
        road_r,
        enter_x,
        leave_x,
        enter_span,
        leave_span,
    }
}

impl Profile {
    pub fn road_mm(&self) -> i32 {
        self.road_r - self.road_l
    }
}

// ---- movements --------------------------------------------------------------

/// The class of the turn from entering `from` to leaving `to`: within 35
/// degrees of straight on is through, further clockwise is right, further
/// anticlockwise is left.
pub fn turn_class(from: i32, to: i32) -> u8 {
    let t = (to - from).rem_euclid(360) - 180; // the turn: heading out minus heading in, folded
    if t.abs() <= 35 {
        THROUGH
    } else if t > 0 {
        RIGHT
    } else {
        LEFT
    }
}

/// The classes of turn an arm has, given the other arms' bearings.
pub fn classes_of(arms: &[Arm], i: usize) -> u8 {
    arms.iter()
        .enumerate()
        .filter(|(j, _)| *j != i)
        .fold(0, |m, (_, b)| m | turn_class(arms[i].bearing, b.bearing))
}

/// The classes a lane in place `i` of `n` serves by default, given those the
/// arm has: one lane serves everything, two are left and through and through
/// and right, more are left, through and right.
pub fn default_uses(i: usize, n: usize, avail: u8) -> u8 {
    let raw = match (n, i) {
        (1, _) => LEFT | THROUGH | RIGHT,
        (2, 0) => LEFT | THROUGH,
        (2, _) => THROUGH | RIGHT,
        (_, 0) => LEFT,
        (_, i) if i + 1 == n => RIGHT,
        _ => THROUGH,
    };
    match raw & avail {
        0 => avail,
        m => m,
    }
}

fn compass(bearing: i32) -> &'static str {
    const NAMES: [&str; 8] = ["north", "north-east", "east", "south-east", "south", "south-west", "west", "north-west"];
    NAMES[(((bearing + 22).rem_euclid(360)) / 45) as usize]
}

pub fn arm_name(a: &Arm) -> String {
    format!("{} ({})", SAMPLES[a.street].name, compass(a.bearing))
}

// ---- samples ----------------------------------------------------------------

pub struct SampleArm {
    pub street: usize,
    pub bearing: i32,
    pub offset_mm: i32,
    pub corner_mm: i32,
    /// Starts with a refuge island in its crossing.
    pub island: bool,
}

const fn sa(street: usize, bearing: i32, offset_mm: i32, corner_mm: i32, island: bool) -> SampleArm {
    SampleArm { street, bearing, offset_mm, corner_mm, island }
}

pub struct JunctionSample {
    pub name: &'static str,
    pub control: usize,
    pub arms: &'static [SampleArm],
}

pub const JUNCTION_SAMPLES: [JunctionSample; 4] = [
    JunctionSample {
        name: "Avenue and street",
        control: SIGNAL,
        arms: &[sa(1, 0, 0, 6_000, true), sa(0, 90, 0, 6_000, false), sa(1, 180, 0, 6_000, true), sa(0, 270, 0, 6_000, false)],
    },
    JunctionSample {
        name: "Street and lane",
        control: PRIORITY,
        arms: &[sa(0, 90, 0, 3_000, false), sa(2, 180, 0, 3_000, false), sa(0, 270, 0, 3_000, false)],
    },
    JunctionSample {
        name: "Offset crossing",
        control: PRIORITY,
        arms: &[sa(2, 0, 2_500, 3_000, false), sa(0, 90, 0, 3_000, false), sa(2, 180, 2_500, 3_000, false), sa(0, 270, 0, 3_000, false)],
    },
    JunctionSample {
        name: "Five ways",
        control: ALL_WAY_STOP,
        arms: &[
            sa(0, 0, 0, 4_000, false),
            sa(2, 70, 0, 3_000, false),
            sa(1, 145, 0, 4_000, true),
            sa(2, 215, 0, 3_000, false),
            sa(0, 290, 0, 4_000, false),
        ],
    },
];

// ---- the editor ---------------------------------------------------------------

pub struct Junction {
    sample: usize,
    /// Index into `REGIONS`. A setting of the sheet, not part of the history.
    pub region: usize,
    states: Vec<State>,
    cursor: usize,
    next_uid: u32,
    pub selected: Target,
    gesture: Option<State>,
    pending_label: String,
}

fn snap(v: i32, step: i32) -> i32 {
    ((v as f64 / step as f64).round() as i32) * step
}

impl Junction {
    pub fn new(sample: usize) -> Junction {
        let mut j = Junction {
            sample: 0,
            region: 0,
            states: Vec::new(),
            cursor: 0,
            next_uid: 1,
            selected: Target::None,
            gesture: None,
            pending_label: String::new(),
        };
        j.load_sample(sample);
        j
    }

    pub fn sample(&self) -> usize {
        self.sample
    }

    pub fn load_sample(&mut self, sample: usize) {
        let sample = sample.min(JUNCTION_SAMPLES.len() - 1);
        let s = &JUNCTION_SAMPLES[sample];
        self.sample = sample;
        self.next_uid = 1;
        let mut arms: Vec<Arm> = s
            .arms
            .iter()
            .map(|a| {
                let uid = self.next_uid;
                self.next_uid += 1;
                let mut arm = self.fresh_arm(uid, a.street, a.bearing, a.offset_mm);
                arm.corner_mm = a.corner_mm;
                if let Some(c) = arm.crossing.as_mut() {
                    c.island = a.island;
                }
                arm
            })
            .collect();
        arms.sort_by_key(|a| a.bearing);
        normalize(&mut arms, self.region);
        self.states = vec![State { label: "Junction today".into(), arms, control: s.control, ring_extra_mm: 0 }];
        self.cursor = 0;
        self.selected = Target::None;
        self.gesture = None;
    }

    fn fresh_arm(&self, uid: u32, street: usize, bearing: i32, offset_mm: i32) -> Arm {
        Arm {
            uid,
            street,
            bearing,
            offset_mm,
            corner_mm: DEFAULT_CORNER_MM,
            lanes: Vec::new(),
            crossing: Some(Crossing { setback_mm: DEFAULT_SETBACK_MM, width_mm: DEFAULT_CROSSING_MM, island: false }),
            bulb: [false, false],
            banned: Vec::new(),
        }
    }

    pub fn current(&self) -> &State {
        &self.states[self.cursor]
    }

    fn current_mut(&mut self) -> &mut State {
        &mut self.states[self.cursor]
    }

    pub fn arm(&self, uid: u32) -> Option<&Arm> {
        self.current().arms.iter().find(|a| a.uid == uid)
    }

    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }

    pub fn can_redo(&self) -> bool {
        self.cursor + 1 < self.states.len()
    }

    pub fn revisions(&self) -> impl Iterator<Item = &str> {
        self.states.iter().skip(1).take(self.cursor).map(|s| s.label.as_str())
    }

    pub fn drive_side(&self) -> Side {
        REGIONS[self.region].drive_side
    }

    pub fn set_region(&mut self, region: usize) -> bool {
        if region >= REGIONS.len() || region == self.region {
            return false;
        }
        self.region = region;
        // The lanes each street brings follow the drive side; keep what the
        // resident set where the count still fits.
        let r = self.region;
        for s in &mut self.states {
            normalize(&mut s.arms, r);
        }
        true
    }

    // ---- history ----------------------------------------------------------

    /// Runs an edit on a copy and records it as a revision, or, inside a
    /// gesture, applies it without recording. Rejects an edit that leaves the
    /// junction invalid.
    fn edit<F>(&mut self, label: String, f: F) -> bool
    where
        F: FnOnce(&mut State) -> bool,
    {
        let region = self.region;
        let mut next = self.current().clone();
        if !f(&mut next) {
            return false;
        }
        next.arms.sort_by_key(|a| a.bearing);
        normalize(&mut next.arms, region);
        if !valid(&next, region) || next == *self.current() {
            return false;
        }
        if self.gesture.is_some() {
            *self.current_mut() = next;
            self.pending_label = label;
            return true;
        }
        next.label = label;
        self.states.truncate(self.cursor + 1);
        self.states.push(next);
        self.cursor += 1;
        true
    }

    pub fn begin_gesture(&mut self) {
        if self.gesture.is_none() {
            self.gesture = Some(self.current().clone());
            self.pending_label.clear();
        }
    }

    pub fn end_gesture(&mut self) -> bool {
        let Some(baseline) = self.gesture.take() else { return false };
        let mut now = self.current().clone();
        if now == baseline {
            return false;
        }
        *self.current_mut() = baseline;
        now.label = std::mem::take(&mut self.pending_label);
        self.states.truncate(self.cursor + 1);
        self.states.push(now);
        self.cursor += 1;
        true
    }

    pub fn cancel_gesture(&mut self) {
        if let Some(baseline) = self.gesture.take() {
            *self.current_mut() = baseline;
        }
    }

    pub fn undo(&mut self) -> bool {
        self.cancel_gesture();
        if !self.can_undo() {
            return false;
        }
        self.cursor -= 1;
        self.fix_selection();
        true
    }

    pub fn redo(&mut self) -> bool {
        if !self.can_redo() {
            return false;
        }
        self.cursor += 1;
        self.fix_selection();
        true
    }

    pub fn reset(&mut self) -> bool {
        if self.cursor == 0 && self.states.len() == 1 {
            return false;
        }
        self.load_sample(self.sample);
        true
    }

    pub fn changed(&self) -> bool {
        self.cursor > 0
    }

    fn fix_selection(&mut self) {
        let keep = match self.selected {
            Target::None => true,
            Target::Arm(u) | Target::Corner(u) => self.arm(u).is_some(),
            Target::Crossing(u) => self.arm(u).is_some_and(|a| a.crossing.is_some()),
        };
        if !keep {
            self.selected = Target::None;
        }
    }

    // ---- selection --------------------------------------------------------

    pub fn select(&mut self, t: Target) {
        self.selected = t;
        self.fix_selection();
    }

    /// Everything that can be selected, in clockwise order.
    pub fn targets(&self) -> Vec<Target> {
        let mut v = Vec::new();
        let arms = &self.current().arms;
        for (i, a) in arms.iter().enumerate() {
            v.push(Target::Arm(a.uid));
            if a.crossing.is_some() {
                v.push(Target::Crossing(a.uid));
            }
            // A roundabout has no curb radius to set, nor does a straight curb.
            if self.current().control != ROUNDABOUT && gap(a.bearing, arms[(i + 1) % arms.len()].bearing) != 180 {
                v.push(Target::Corner(a.uid));
            }
        }
        v
    }

    pub fn select_relative(&mut self, delta: i32) {
        let ts = self.targets();
        if ts.is_empty() {
            return;
        }
        let i = ts.iter().position(|t| *t == self.selected);
        let n = ts.len() as i32;
        let next = match i {
            Some(i) => (i as i32 + delta).rem_euclid(n),
            None if delta < 0 => n - 1,
            None => 0,
        };
        self.selected = ts[next as usize];
    }

    // ---- edits ------------------------------------------------------------

    fn arm_edit<F>(&mut self, uid: u32, label: impl FnOnce(&Arm) -> String, f: F) -> bool
    where
        F: FnOnce(&mut Arm) -> bool,
    {
        let Some(a) = self.arm(uid) else { return false };
        let label = label(a);
        self.edit(label, |s| s.arms.iter_mut().find(|a| a.uid == uid).is_some_and(f))
    }

    /// Adds a street at `bearing`, or at the middle of the widest gap when
    /// `bearing` is negative. Returns its uid, or 0 when it does not fit.
    pub fn add_arm(&mut self, street: usize, bearing: i32) -> u32 {
        if street >= SAMPLES.len() || self.current().arms.len() >= MAX_ARMS {
            return 0;
        }
        let bearing = if bearing < 0 { widest_gap(&self.current().arms) } else { snap(bearing, BEARING_STEP).rem_euclid(360) };
        let uid = self.next_uid;
        let arm = self.fresh_arm(uid, street, bearing, 0);
        let label = format!("Add {}", arm_name(&arm));
        // A tight angle between streets needs a tighter corner to fit, so try
        // smaller radii on the new corners before giving up.
        for radius in [DEFAULT_CORNER_MM, 4_500, 3_000, 2_000, MIN_CORNER_MM] {
            let mut arm = arm.clone();
            arm.corner_mm = radius;
            if self.edit(label.clone(), |s| {
                s.arms.push(arm);
                s.arms.sort_by_key(|a| a.bearing);
                let i = s.arms.iter().position(|a| a.uid == uid).unwrap_or(0);
                let before = (i + s.arms.len() - 1) % s.arms.len();
                s.arms[before].corner_mm = s.arms[before].corner_mm.min(radius);
                true
            }) {
                self.next_uid += 1;
                self.selected = Target::Arm(uid);
                return uid;
            }
        }
        0
    }

    pub fn remove_arm(&mut self, uid: u32) -> bool {
        let Some(a) = self.arm(uid) else { return false };
        let label = format!("Remove {}", arm_name(a));
        let ok = self.edit(label, |s| {
            if s.arms.len() <= MIN_ARMS {
                return false;
            }
            s.arms.retain(|a| a.uid != uid);
            true
        });
        if ok {
            self.fix_selection();
        }
        ok
    }

    pub fn set_bearing(&mut self, uid: u32, bearing: i32) -> bool {
        let b = snap(bearing, BEARING_STEP).rem_euclid(360);
        self.arm_edit(uid, |a| format!("{} bearing: {b}°", SAMPLES[a.street].name), |a| {
            a.bearing = b;
            true
        })
    }

    pub fn set_offset(&mut self, uid: u32, mm: i32) -> bool {
        let mm = snap(mm, OFFSET_STEP_MM);
        let region = self.region;
        self.arm_edit(uid, |a| format!("{} offset: {mm} mm", arm_name(a)), |a| {
            let half = profile(a.street, region).road_mm() / 2;
            if mm.abs() > half {
                return false;
            }
            a.offset_mm = mm;
            true
        })
    }

    /// Sets the curb radius at the corner clockwise of an arm.
    pub fn set_corner(&mut self, uid: u32, mm: i32) -> bool {
        let mm = snap(mm, RING_STEP_MM);
        if !(MIN_CORNER_MM..=MAX_CORNER_MM).contains(&mm) {
            return false;
        }
        self.arm_edit(uid, |a| format!("Corner after {}: {mm} mm radius", arm_name(a)), |a| {
            a.corner_mm = mm;
            true
        })
    }

    pub fn set_street(&mut self, uid: u32, street: usize) -> bool {
        if street >= SAMPLES.len() {
            return false;
        }
        self.arm_edit(uid, |a| format!("{} becomes {}", arm_name(a), SAMPLES[street].name), |a| {
            a.street = street;
            a.offset_mm = 0;
            a.lanes.clear();
            a.bulb = [false, false];
            true
        })
    }

    pub fn set_crossing(&mut self, uid: u32, on: bool) -> bool {
        self.arm_edit(uid, |a| format!("{} crossing: {}", arm_name(a), if on { "add" } else { "remove" }), |a| {
            a.crossing = on.then_some(Crossing { setback_mm: DEFAULT_SETBACK_MM, width_mm: DEFAULT_CROSSING_MM, island: false });
            true
        })
    }

    pub fn set_setback(&mut self, uid: u32, mm: i32) -> bool {
        let mm = snap(mm, RING_STEP_MM);
        if !(MIN_SETBACK_MM..=MAX_SETBACK_MM).contains(&mm) {
            return false;
        }
        self.arm_edit(uid, |a| format!("{} crossing set back {mm} mm", arm_name(a)), |a| {
            a.crossing.as_mut().map(|c| c.setback_mm = mm).is_some()
        })
    }

    pub fn set_crossing_width(&mut self, uid: u32, mm: i32) -> bool {
        let mm = snap(mm, RING_STEP_MM);
        if !(MIN_CROSSING_MM..=MAX_CROSSING_MM).contains(&mm) {
            return false;
        }
        self.arm_edit(uid, |a| format!("{} crossing {mm} mm wide", arm_name(a)), |a| {
            a.crossing.as_mut().map(|c| c.width_mm = mm).is_some()
        })
    }

    pub fn set_island(&mut self, uid: u32, on: bool) -> bool {
        let region = self.region;
        self.arm_edit(uid, |a| format!("{} refuge island: {}", arm_name(a), if on { "add" } else { "remove" }), |a| {
            if on && profile(a.street, region).road_mm() < ISLAND_MIN_ROAD_MM {
                return false;
            }
            a.crossing.as_mut().map(|c| c.island = on).is_some()
        })
    }

    /// Side 0 is the arm's left curb, 1 its right.
    pub fn set_bulb(&mut self, uid: u32, side: usize, on: bool) -> bool {
        let region = self.region;
        if side > 1 {
            return false;
        }
        let word = if side == 0 { "left" } else { "right" };
        self.arm_edit(uid, |a| format!("{} {word} bulb-out: {}", arm_name(a), if on { "add" } else { "remove" }), |a| {
            if on && profile(a.street, region).park[side] == 0 {
                return false;
            }
            a.bulb[side] = on;
            true
        })
    }

    pub fn set_lane_use(&mut self, uid: u32, lane: usize, class: u8, on: bool) -> bool {
        if !CLASSES.contains(&class) {
            return false;
        }
        let word = match class {
            LEFT => "left",
            THROUGH => "through",
            _ => "right",
        };
        let avail = self.current().arms.iter().position(|a| a.uid == uid).map_or(0, |i| classes_of(&self.current().arms, i));
        self.arm_edit(
            uid,
            |a| format!("{} lane {}: {word} {}", arm_name(a), lane + 1, if on { "on" } else { "off" }),
            |a| {
                let Some(u) = a.lanes.get_mut(lane) else { return false };
                let next = if on { *u | class } else { *u & !class };
                if next == 0 || next & !avail != 0 {
                    return false;
                }
                *u = next;
                true
            },
        )
    }

    pub fn set_turn(&mut self, from: u32, to: u32, allowed: bool) -> bool {
        let (Some(a), Some(b)) = (self.arm(from), self.arm(to)) else { return false };
        if from == to {
            return false;
        }
        let label = format!("{} to {}: {}", arm_name(a), arm_name(b), if allowed { "allow" } else { "no turn" });
        let others = self.current().arms.len() - 1;
        self.arm_edit(from, |_| label, |a| {
            if allowed {
                let before = a.banned.len();
                a.banned.retain(|&u| u != to);
                return a.banned.len() != before;
            }
            // An arm keeps at least one way out.
            if a.banned.contains(&to) || a.banned.len() + 2 > others {
                return false;
            }
            a.banned.push(to);
            true
        })
    }

    pub fn set_control(&mut self, control: usize) -> bool {
        if control >= CONTROLS.len() {
            return false;
        }
        self.edit(format!("Control: {}", CONTROLS[control].name.to_lowercase()), |s| {
            s.control = control;
            true
        })
    }

    /// Sets how much bigger than its least size a roundabout is.
    pub fn set_ring(&mut self, extra_mm: i32) -> bool {
        let extra = snap(extra_mm, RING_STEP_MM);
        if !(0..=MAX_RING_MM).contains(&extra) {
            return false;
        }
        self.edit(format!("Roundabout: {extra} mm larger"), |s| {
            s.ring_extra_mm = extra;
            true
        })
    }
}

/// The middle of the widest gap between arms, snapped to the bearing step.
pub fn widest_gap(arms: &[Arm]) -> i32 {
    let mut b: Vec<i32> = arms.iter().map(|a| a.bearing).collect();
    b.sort_unstable();
    let n = b.len();
    if n == 0 {
        return 0;
    }
    // The first of the widest gaps, so the choice does not depend on order.
    let (start, width) = (0..n).map(|i| (b[i], gap(b[i], b[(i + 1) % n]))).fold((b[0], 0), |best, g| if g.1 > best.1 { g } else { best });
    snap(start + width / 2, BEARING_STEP).rem_euclid(360)
}

/// Keeps the parts that follow from the arms consistent: lane counts follow
/// each street, a lane serves only turns its arm has, bans name arms that
/// exist, and features that need room or parking stay only where they have it.
pub fn normalize(arms: &mut [Arm], region: usize) {
    let uids: Vec<u32> = arms.iter().map(|a| a.uid).collect();
    let avail: Vec<u8> = (0..arms.len()).map(|i| classes_of(arms, i)).collect();
    for (a, avail) in arms.iter_mut().zip(avail) {
        let p = profile(a.street, region);
        let n = p.enter_x.len();
        if a.lanes.len() != n {
            a.lanes = (0..n).map(|i| default_uses(i, n, avail)).collect();
        }
        for (i, u) in a.lanes.iter_mut().enumerate() {
            *u &= avail;
            if *u == 0 {
                *u = default_uses(i, n, avail);
            }
        }
        a.banned.retain(|b| uids.contains(b) && *b != a.uid);
        a.bulb = [a.bulb[0] && p.park[0] > 0, a.bulb[1] && p.park[1] > 0];
        if let Some(c) = a.crossing.as_mut() {
            c.island = c.island && p.road_mm() >= ISLAND_MIN_ROAD_MM;
        }
        // Offsets stay within the carriageway of whatever street the arm reads.
        let half = p.road_mm() / 2;
        a.offset_mm = a.offset_mm.clamp(-half, half);
    }
}

/// Whether a state is a junction: three to five arms, kept apart, with no gap
/// wider than a straight line, and geometry the plan can draw.
pub fn valid(s: &State, region: usize) -> bool {
    let n = s.arms.len();
    if !(MIN_ARMS..=MAX_ARMS).contains(&n) {
        return false;
    }
    for i in 0..n {
        let g = gap(s.arms[i].bearing, s.arms[(i + 1) % n].bearing);
        if !(MIN_SEPARATION..=180).contains(&g) {
            return false;
        }
    }
    crate::junction_view::layout(s, region).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turn_classes_follow_the_angle() {
        assert_eq!(turn_class(0, 180), THROUGH);
        assert_eq!(turn_class(0, 90), LEFT); // entering from the north, heading south, east is a left
        assert_eq!(turn_class(0, 270), RIGHT);
        assert_eq!(turn_class(0, 145), THROUGH); // within 35 degrees of straight
        assert_eq!(turn_class(0, 140), LEFT);
    }

    #[test]
    fn default_lane_uses_split_by_count() {
        let all = LEFT | THROUGH | RIGHT;
        assert_eq!(default_uses(0, 1, all), all);
        assert_eq!(default_uses(0, 2, all), LEFT | THROUGH);
        assert_eq!(default_uses(1, 2, all), THROUGH | RIGHT);
        assert_eq!(default_uses(0, 3, all), LEFT);
        assert_eq!(default_uses(1, 3, all), THROUGH);
        assert_eq!(default_uses(2, 3, all), RIGHT);
        // The stem of a T has no through: everything falls back to what exists.
        assert_eq!(default_uses(0, 1, LEFT | RIGHT), LEFT | RIGHT);
        assert_eq!(default_uses(1, 3, LEFT | RIGHT), LEFT | RIGHT);
    }

    #[test]
    fn profile_reads_direction_and_lanes() {
        let p = profile(0, 0); // Sample Street 1, traffic keeps right
        assert_eq!(p.enter_x.len(), 1);
        assert_eq!(p.leave_x.len(), 1);
        // Entering traffic is on the arm's left half when traffic keeps right.
        assert!(p.enter_x[0] < p.row_mm / 2 && p.leave_x[0] > p.row_mm / 2);
        let l = profile(0, 3); // United Kingdom keeps left
        assert!(l.enter_x[0] > l.row_mm / 2 && l.leave_x[0] < l.row_mm / 2);
        assert_eq!(p.road_mm(), 18000 - 3300 * 2);
        assert_eq!(p.park, [2400, 2400]);
        assert_eq!(profile(1, 0).enter_x.len(), 2);
    }

    #[test]
    fn samples_are_valid_junctions() {
        for i in 0..JUNCTION_SAMPLES.len() {
            let j = Junction::new(i);
            assert!(valid(j.current(), 0), "{}", JUNCTION_SAMPLES[i].name);
            assert_eq!(j.current().arms.len(), JUNCTION_SAMPLES[i].arms.len());
        }
    }

    #[test]
    fn a_new_arm_finds_the_widest_gap() {
        let mut j = Junction::new(1); // 90, 180, 270
        let uid = j.add_arm(2, -1);
        assert_ne!(uid, 0);
        assert_eq!(j.arm(uid).unwrap().bearing, 0);
        assert_eq!(j.current().arms.len(), 4);
        assert_eq!(j.selected, Target::Arm(uid));
    }

    #[test]
    fn a_street_squeezed_between_wide_ones_takes_tighter_corners() {
        let mut j = Junction::new(0); // 0, 90, 180, 270: all gaps 90, first is 0 to 90
        let uid = j.add_arm(2, -1);
        assert_ne!(uid, 0, "there is room at 45 degrees with a smaller corner");
        assert_eq!(j.arm(uid).unwrap().bearing, 45);
        assert!(j.arm(uid).unwrap().corner_mm < DEFAULT_CORNER_MM);
    }

    #[test]
    fn arms_stay_between_three_and_five() {
        let mut j = Junction::new(1);
        let a = j.current().arms[0].uid;
        assert!(!j.remove_arm(a));
        let mut j = Junction::new(3);
        assert_eq!(j.add_arm(0, 0), 0);
        assert_eq!(j.current().arms.len(), 5);
    }

    #[test]
    fn bearings_keep_apart_and_no_gap_exceeds_a_straight_line() {
        let mut j = Junction::new(0); // 0, 90, 180, 270
        let east = j.current().arms[1].uid;
        assert!(!j.set_bearing(east, 10), "too close to north");
        assert!(j.set_bearing(east, 60));
        assert_eq!(j.arm(east).unwrap().bearing, 60);
        let mut t = Junction::new(1); // 90, 180, 270
        let south = t.current().arms[1].uid;
        assert!(!t.set_bearing(south, 60), "would leave a gap wider than 180");
        let north = t.add_arm(2, 0);
        assert_ne!(north, 0);
        let east = t.current().arms.iter().find(|a| a.bearing == 90).unwrap().uid;
        assert!(!t.remove_arm(east) || t.current().arms.len() == 3);
    }

    #[test]
    fn bearing_snaps_to_five_degrees() {
        let mut j = Junction::new(0);
        let west = j.current().arms[3].uid;
        assert!(!j.set_bearing(west, 272), "272 snaps to 270, which is where it is");
        assert!(j.set_bearing(west, 273));
        assert_eq!(j.arm(west).unwrap().bearing, 275);
        assert!(j.set_bearing(west, 360 + 270));
        assert_eq!(j.arm(west).unwrap().bearing, 270);
    }

    #[test]
    fn undo_and_redo_restore_edits() {
        let mut j = Junction::new(0);
        let a = j.current().arms[0].uid;
        assert!(j.set_corner(a, 9000));
        assert_eq!(j.arm(a).unwrap().corner_mm, 9000);
        assert!(j.changed());
        assert!(j.undo());
        assert_eq!(j.arm(a).unwrap().corner_mm, DEFAULT_CORNER_MM);
        assert!(j.redo());
        assert_eq!(j.arm(a).unwrap().corner_mm, 9000);
        assert!(j.reset());
        assert_eq!(j.arm(a).unwrap().corner_mm, DEFAULT_CORNER_MM);
    }

    #[test]
    fn a_gesture_is_one_revision() {
        let mut j = Junction::new(0);
        let a = j.current().arms[0].uid;
        j.begin_gesture();
        for r in [7000, 8000, 9000] {
            assert!(j.set_corner(a, r));
        }
        assert!(j.end_gesture());
        assert_eq!(j.revisions().count(), 1);
        assert!(j.undo());
        assert_eq!(j.arm(a).unwrap().corner_mm, DEFAULT_CORNER_MM);
        j.begin_gesture();
        j.set_corner(a, 12000);
        j.cancel_gesture();
        assert_eq!(j.arm(a).unwrap().corner_mm, DEFAULT_CORNER_MM);
    }

    #[test]
    fn corner_crossing_and_bulb_limits() {
        let mut j = Junction::new(0);
        let street = j.current().arms[1].uid; // Sample Street 1: parking, road 11.4 m
        assert!(!j.set_corner(street, 500));
        assert!(!j.set_corner(street, 16000));
        assert!(!j.set_setback(street, 1000));
        assert!(j.set_setback(street, 4000));
        assert!(!j.set_crossing_width(street, 7000));
        assert!(j.set_island(street, true), "11.4 m carriageway takes an island");
        assert!(j.set_bulb(street, 0, true));
        assert!(j.set_crossing(street, false));
        assert!(!j.set_setback(street, 4500), "no crossing to set back");
    }

    #[test]
    fn a_narrow_street_takes_no_island() {
        let mut j = Junction::new(1); // Street, lane, street
        let lane = j.current().arms.iter().find(|a| a.street == 2).unwrap().uid;
        assert!(!j.set_island(lane, true), "8.4 m carriageway is too narrow");
    }

    #[test]
    fn lane_uses_need_a_turn_the_arm_has_and_at_least_one_class() {
        let mut j = Junction::new(1);
        let lane = j.current().arms.iter().find(|a| a.street == 2).unwrap().uid;
        // The lane is the stem of a T: no through movement exists.
        assert!(!j.set_lane_use(lane, 0, THROUGH, true));
        assert_eq!(j.arm(lane).unwrap().lanes[0], LEFT | RIGHT);
        assert!(j.set_lane_use(lane, 0, LEFT, false));
        assert!(!j.set_lane_use(lane, 0, RIGHT, false), "a lane must serve something");
    }

    #[test]
    fn turn_bans_are_kept_and_cannot_close_every_exit() {
        let mut j = Junction::new(0); // 4 arms
        let (n, e, s, w) = {
            let a = &j.current().arms;
            (a[0].uid, a[1].uid, a[2].uid, a[3].uid)
        };
        let _ = s;
        assert!(j.set_turn(n, e, false));
        assert!(!j.set_turn(n, e, false), "already banned");
        assert!(j.set_turn(n, s, false));
        assert!(!j.set_turn(n, w, false), "the last way out stays open");
        assert_eq!(j.arm(n).unwrap().banned.len(), 2);
        assert!(j.set_turn(n, e, true));
    }

    #[test]
    fn removing_an_arm_drops_bans_that_named_it() {
        let mut j = Junction::new(0);
        let (n, e) = (j.current().arms[0].uid, j.current().arms[1].uid);
        j.set_turn(n, e, false);
        assert!(j.remove_arm(e));
        assert!(j.arm(n).unwrap().banned.is_empty());
    }

    #[test]
    fn control_and_ring_are_settings_of_the_junction() {
        let mut j = Junction::new(0);
        assert!(j.set_control(ROUNDABOUT));
        assert!(!j.set_control(ROUNDABOUT));
        assert!(!j.set_ring(50_000));
        assert!(j.set_ring(3000));
        assert!(j.undo() && j.undo());
        assert_eq!(j.current().control, SIGNAL);
    }

    #[test]
    fn selection_walks_clockwise_and_survives_removal() {
        let mut j = Junction::new(0);
        j.select_relative(1);
        assert_eq!(j.selected, Target::Arm(j.current().arms[0].uid));
        j.select_relative(1);
        assert_eq!(j.selected, Target::Crossing(j.current().arms[0].uid));
        j.select_relative(1);
        assert_eq!(j.selected, Target::Corner(j.current().arms[0].uid));
        let e = j.current().arms[1].uid;
        j.select(Target::Arm(e));
        j.remove_arm(e);
        assert_eq!(j.selected, Target::None);
        j.set_control(ROUNDABOUT);
        assert!(!j.targets().iter().any(|t| matches!(t, Target::Corner(_))));
    }

    #[test]
    fn region_change_flips_which_side_lanes_enter() {
        let mut j = Junction::new(0);
        assert!(j.set_region(3));
        assert!(!j.set_region(3));
        for a in &j.current().arms {
            let p = profile(a.street, 3);
            assert_eq!(a.lanes.len(), p.enter_x.len());
        }
    }
}
