//! The junction editing model: three to five streets meeting in plan, with
//! corners, crossings, the lanes each street brings, turn bans and a control.
//! Pure Rust like `model.rs`, so it is tested natively. Lengths are integer
//! millimetres and angles integer degrees. Everything here is synthetic.

use crate::catalogue::{KINDS, Material, Mode, REGIONS, SAMPLES, Side};
use serde::{Deserialize, Serialize};

use crate::model::{Editor, Street, View};
use crate::plan::gap;

pub const LEFT: u8 = 1;
pub const THROUGH: u8 = 2;
pub const RIGHT: u8 = 4;

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

/// One entry of a short list a junction measure is chosen from. `code` is the
/// Transit Priority Atlas toolbox code.
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct Item {
    pub id: &'static str,
    pub code: &'static str,
    pub name: &'static str,
}

pub const Q_OFFSET: usize = 1;
pub const Q_CURB: usize = 2;
pub const Q_VIRTUAL: usize = 3;
pub const GATE_SIGNAL: usize = 4;
pub const GATE_YIELD: usize = 5;

/// What a street does on its way in to help buses get to the front: a short
/// transit lane, a virtual lane made by signals, or a gate that stops the
/// other traffic upstream.
pub const APPROACHES: [Item; 6] = [
    Item { id: "none", code: "", name: "None" },
    Item { id: "queue-offset", code: "G1", name: "Offset queue-jump lane" },
    Item { id: "queue-curb", code: "G2", name: "Curbside queue-jump lane" },
    Item { id: "queue-virtual", code: "G3", name: "Virtual queue-jump lane" },
    Item { id: "gate-signal", code: "H1", name: "Signal-controlled bus gate" },
    Item { id: "gate-yield", code: "H2", name: "Yield-controlled bus gate" },
];

pub const STOP_BULB: usize = 1;
pub const STOP_PLATFORM: usize = 2;

pub const STOPS: [Item; 3] = [
    Item { id: "none", code: "", name: "No bus stop" },
    Item { id: "bulb", code: "M1", name: "Bus bulb" },
    Item { id: "platform", code: "M2", name: "Signal-protected on-street platform" },
];

/// The Atlas code of the transit modal filter, which an arm has or has not.
pub const FILTER_CODE: &str = "N1";

pub const RULE_AROUND: usize = 1;
pub const RULE_WITHIN: usize = 2;
pub const RULE_RIRO: usize = 3;
pub const RULE_DEAD_END: usize = 4;

pub const RULES: [Item; 5] = [
    Item { id: "none", code: "", name: "No turn management" },
    Item { id: "around", code: "L1", name: "Indirect left turn via alternative itinerary" },
    Item { id: "within", code: "L2", name: "Indirect left turn within the intersection" },
    Item { id: "riro", code: "L3", name: "Right-in/right-out" },
    Item { id: "dead-end", code: "L4", name: "Dead-ending of a lateral street" },
];

/// How far a queue jump runs back from the stop line, or a gate stands
/// upstream of it.
pub const APPROACH_DEFAULT_MM: i32 = 10_000;
pub const APPROACH_MIN_MM: i32 = 5_000;
pub const APPROACH_MAX_MM: i32 = 15_000;
pub const APPROACH_STEP_MM: i32 = 5_000;
/// Width of a transit lane on an approach.
pub const TRANSIT_LANE_MM: i32 = 3_300;

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
/// Width of the bus lane across a roundabout's middle.
pub const BUS_LANE_MM: i32 = 3_500;
/// A cycle track around a roundabout, in steps of `RING_STEP_MM`.
pub const CYCLE_DEFAULT_MM: i32 = 2_000;
pub const CYCLE_MIN_MM: i32 = 1_500;
pub const CYCLE_MAX_MM: i32 = 3_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Crossing {
    pub setback_mm: i32,
    pub width_mm: i32,
    pub island: bool,
}

/// An entering lane and the streets it can go to, by their uids.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lane {
    pub to: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Arm {
    pub uid: u32,
    /// Index into `SAMPLES`: the street whose section this arm reads, when it
    /// does not come from a city.
    pub street: usize,
    /// The city street this arm is, or 0 when it is not part of a city.
    #[serde(default)]
    pub edge: u32,
    /// That street, seen looking outward from the junction. The city puts the
    /// street's current state here when it opens the junction, so it is not
    /// kept with the junction.
    #[serde(skip)]
    pub section: Option<Street>,
    /// Clockwise from north, a multiple of `BEARING_STEP`.
    pub bearing: i32,
    /// Sideways shift of the arm's axis, to the arm's right.
    pub offset_mm: i32,
    /// Curb radius at the corner clockwise of this arm.
    pub corner_mm: i32,
    /// The entering lanes, driver's left to right, and the streets each goes to.
    pub lanes: Vec<Lane>,
    pub crossing: Option<Crossing>,
    /// Curb extension at the arm's left and right curb.
    pub bulb: [bool; 2],
    /// Arms this arm may not turn into.
    pub banned: Vec<u32>,
    /// A transit lane runs along the entering curb all the way in.
    pub bus_lane: bool,
    /// Index into `APPROACHES`, and its length or distance.
    pub approach: usize,
    pub approach_mm: i32,
    /// Index into `STOPS`.
    pub stop: usize,
    /// Index into `RULES`.
    pub rule: usize,
    /// A transit modal filter: only buses pass.
    pub filter: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub label: String,
    pub arms: Vec<Arm>,
    pub control: usize,
    /// Roundabout size above the least that keeps the arms apart.
    pub ring_extra_mm: i32,
    /// A bus-only lane straight across the middle of a roundabout, between two
    /// streets, by their uids (the lower first).
    pub bus: Option<(u32, u32)>,
    /// Width of a cycle track around the outside of a roundabout, if it has one.
    pub cycle: Option<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    None,
    Arm(u32),
    Corner(u32),
    Crossing(u32),
    /// An entering lane of an arm, by its place among that arm's lanes.
    Lane(u32, usize),
    /// The bus lane across a roundabout's middle.
    Bus,
    /// The cycle track around a roundabout.
    Cycle,
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
    read_profile(&e.view(), SAMPLES[street].name)
}

/// The profile of a street a city holds, with its lanes written for the side
/// of the road of `region`.
pub fn profile_of(street: &Street, region: usize) -> Profile {
    let e = Editor::from_street(street, street, region);
    read_profile(&e.view(), SAMPLES[street.sample.min(SAMPLES.len() - 1)].name)
}

impl Arm {
    /// An arm with the defaults a street starts with at a junction.
    pub fn new(uid: u32, street: usize, bearing: i32, offset_mm: i32) -> Arm {
        Arm {
            uid,
            street,
            edge: 0,
            section: None,
            bearing,
            offset_mm,
            corner_mm: DEFAULT_CORNER_MM,
            lanes: Vec::new(),
            crossing: Some(Crossing { setback_mm: DEFAULT_SETBACK_MM, width_mm: DEFAULT_CROSSING_MM, island: false }),
            bulb: [false, false],
            banned: Vec::new(),
            bus_lane: false,
            approach: 0,
            approach_mm: APPROACH_DEFAULT_MM,
            stop: 0,
            rule: 0,
            filter: false,
        }
    }

    pub fn profile(&self, region: usize) -> Profile {
        match &self.section {
            Some(s) => profile_of(s, region),
            None => profile(self.street, region),
        }
    }

    /// Which sample street this arm is, or began as.
    pub fn street_index(&self) -> usize {
        self.section.as_ref().map_or(self.street, |s| s.sample).min(SAMPLES.len() - 1)
    }

    pub fn street_name(&self) -> &'static str {
        SAMPLES[self.street_index()].name
    }

    pub fn row_mm(&self) -> i32 {
        self.section.as_ref().map_or(SAMPLES[self.street_index()].row_mm, |s| s.row_mm)
    }
}

fn read_profile(view: &View, name: &'static str) -> Profile {
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
        name,
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
    format!("{} ({})", a.street_name(), compass(a.bearing))
}

/// Why traffic may not turn from arm `i` into arm `j`, when a measure stops
/// it: a dead end, a transit modal filter, right-in/right-out, or a left turn
/// sent round another way.
pub fn blocked(arms: &[Arm], i: usize, j: usize) -> Option<&'static str> {
    let (a, b) = (&arms[i], &arms[j]);
    let class = turn_class(a.bearing, b.bearing);
    if a.rule == RULE_DEAD_END || b.rule == RULE_DEAD_END {
        Some("A dead end")
    } else if a.filter || b.filter {
        Some("A transit modal filter lets only buses through")
    } else if a.rule == RULE_RIRO && class != RIGHT {
        Some("Right-in/right-out: only right turns leave")
    } else if b.rule == RULE_RIRO && class != RIGHT {
        Some("Right-in/right-out: only right turns enter")
    } else if a.rule == RULE_AROUND && class == LEFT {
        Some("Left turns go round by another street")
    } else {
        None
    }
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

/// Why an edit was refused, in words for the resident.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    NoRoomForStreet,
    NeedsThreeStreets,
    LinkedNoAdd,
    LinkedNoRemove,
    LinkedNoSwap,
    RoundaboutTooBig,
    BearingBlocked,
    LastWayOut,
    LaneNeedsStreet,
    IslandRoadTooNarrow,
    BulbNoParking,
    DoesNotFit,
}

impl Refusal {
    pub const ALL: [Refusal; 12] = [
        Refusal::NoRoomForStreet,
        Refusal::NeedsThreeStreets,
        Refusal::LinkedNoAdd,
        Refusal::LinkedNoRemove,
        Refusal::LinkedNoSwap,
        Refusal::RoundaboutTooBig,
        Refusal::BearingBlocked,
        Refusal::LastWayOut,
        Refusal::LaneNeedsStreet,
        Refusal::IslandRoadTooNarrow,
        Refusal::BulbNoParking,
        Refusal::DoesNotFit,
    ];

    pub fn message(self) -> &'static str {
        match self {
            Refusal::NoRoomForStreet => "There is no room for another street.",
            Refusal::NeedsThreeStreets => "A junction needs at least three streets.",
            Refusal::LinkedNoAdd => "The streets here belong to the city, so none can be added.",
            Refusal::LinkedNoRemove => "The streets here belong to the city, so they cannot be removed.",
            Refusal::LinkedNoSwap => "The streets here belong to the city, so they cannot be swapped.",
            Refusal::RoundaboutTooBig => "The streets are too wide to fit a roundabout.",
            Refusal::BearingBlocked => "That is too close to a neighbouring street, or leaves a gap wider than a straight road.",
            Refusal::LastWayOut => "A street has to keep at least one way out.",
            Refusal::LaneNeedsStreet => "A lane has to go to at least one street.",
            Refusal::IslandRoadTooNarrow => "This road is too narrow for an island.",
            Refusal::BulbNoParking => "There is no parking on that side to give up.",
            Refusal::DoesNotFit => "That change does not fit.",
        }
    }
}

pub struct Junction {
    sample: usize,
    /// Set when the junction is a place in a city: its streets are the city's
    /// and its name is the city's, so streets cannot be added, removed or swapped.
    linked: bool,
    name: String,
    /// Index into `REGIONS`. A setting of the sheet, not part of the history.
    pub region: usize,
    states: Vec<State>,
    cursor: usize,
    next_uid: u32,
    pub selected: Target,
    gesture: Option<State>,
    pending_label: String,
    /// Why the last edit was refused; None once one is taken.
    refusal: Option<Refusal>,
}

fn snap(v: i32, step: i32) -> i32 {
    ((v as f64 / step as f64).round() as i32) * step
}

impl Junction {
    pub fn new(sample: usize) -> Junction {
        let mut j = Junction {
            sample: 0,
            linked: false,
            name: String::new(),
            region: 0,
            states: Vec::new(),
            cursor: 0,
            next_uid: 1,
            selected: Target::None,
            gesture: None,
            pending_label: String::new(),
            refusal: None,
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
        self.states = vec![State { label: "Junction today".into(), arms, control: s.control, ring_extra_mm: 0, bus: None, cycle: None }];
        self.cursor = 0;
        self.selected = Target::None;
        self.gesture = None;
    }

    /// A junction that is a place in a city. `today` is how it was first laid
    /// out and `now` how it stands, each with its arms' streets attached. When
    /// the streets have changed so that `now` can no longer be drawn it is
    /// eased (small corners, no offsets, no roundabout) before it is given up
    /// for `today`. None when not even `today` can be drawn.
    pub fn from_city(name: &str, today: &State, now: &State, region: usize) -> Option<Junction> {
        let settle = |s: &State| {
            let mut s = s.clone();
            normalize(&mut s.arms, region);
            s
        };
        let today = ease(&settle(today), region)?;
        let now = ease(&settle(now), region);
        let mut j = Junction {
            sample: 0,
            linked: true,
            name: name.to_string(),
            region,
            next_uid: today.arms.iter().map(|a| a.uid).max().unwrap_or(0) + 1,
            states: vec![State { label: "Junction today".into(), ..today.clone() }],
            cursor: 0,
            selected: Target::None,
            gesture: None,
            pending_label: String::new(),
            refusal: None,
        };
        let same = |a: &State, b: &State| State { label: String::new(), ..a.clone() } == State { label: String::new(), ..b.clone() };
        if let Some(now) = now.filter(|n| !same(n, &today)) {
            j.states.push(State { label: "Earlier changes".into(), ..now });
            j.cursor = 1;
        }
        Some(j)
    }

    /// The junction as it is now, without its streets, for the city to keep.
    pub fn snapshot(&self) -> State {
        let mut s = self.current().clone();
        for a in &mut s.arms {
            a.section = None;
        }
        s
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_linked(&self) -> bool {
        self.linked
    }

    fn fresh_arm(&self, uid: u32, street: usize, bearing: i32, offset_mm: i32) -> Arm {
        Arm::new(uid, street, bearing, offset_mm)
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
        self.edit_why(Refusal::DoesNotFit, label, f)
    }

    /// Like `edit`, with the reason to give when the result cannot be drawn.
    fn edit_why<F>(&mut self, why: Refusal, label: String, f: F) -> bool
    where
        F: FnOnce(&mut State) -> bool,
    {
        let region = self.region;
        let mut next = self.current().clone();
        if !f(&mut next) {
            return self.refuse(Refusal::DoesNotFit);
        }
        next.arms.sort_by_key(|a| a.bearing);
        normalize(&mut next.arms, region);
        if next.control != ROUNDABOUT {
            next.cycle = None;
        }
        // A bus lane needs a roundabout and both its streets.
        if let Some((a, b)) = next.bus {
            if next.control != ROUNDABOUT || !next.arms.iter().any(|x| x.uid == a) || !next.arms.iter().any(|x| x.uid == b) {
                next.bus = None;
            }
        }
        if !valid(&next, region) {
            return self.refuse(why);
        }
        if next == *self.current() {
            return self.refuse(Refusal::DoesNotFit);
        }
        self.refusal = None;
        if self.gesture.is_some() {
            *self.current_mut() = next;
            self.pending_label = label;
            self.fix_selection();
            return true;
        }
        next.label = label;
        self.states.truncate(self.cursor + 1);
        self.states.push(next);
        self.cursor += 1;
        self.fix_selection();
        true
    }

    fn refuse(&mut self, why: Refusal) -> bool {
        self.refusal = Some(why);
        false
    }

    /// Why the last edit was refused, if the last one was.
    pub fn refusal(&self) -> Option<Refusal> {
        self.refusal
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
        if self.linked {
            self.states.truncate(1);
            self.cursor = 0;
            self.selected = Target::None;
            self.gesture = None;
            return true;
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
            Target::Lane(u, i) => self.arm(u).is_some_and(|a| i < a.lanes.len()),
            Target::Bus => self.current().bus.is_some(),
            Target::Cycle => self.current().cycle.is_some(),
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
            v.extend((0..a.lanes.len()).map(|i| Target::Lane(a.uid, i)));
            if a.crossing.is_some() {
                v.push(Target::Crossing(a.uid));
            }
            // A roundabout has no curb radius to set, nor does a straight curb.
            if self.current().control != ROUNDABOUT && gap(a.bearing, arms[(i + 1) % arms.len()].bearing) != 180 {
                v.push(Target::Corner(a.uid));
            }
        }
        if self.current().cycle.is_some() {
            v.push(Target::Cycle);
        }
        if self.current().bus.is_some() {
            v.push(Target::Bus);
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
        self.arm_edit_why(Refusal::DoesNotFit, uid, label, f)
    }

    fn arm_edit_why<F>(&mut self, why: Refusal, uid: u32, label: impl FnOnce(&Arm) -> String, f: F) -> bool
    where
        F: FnOnce(&mut Arm) -> bool,
    {
        let Some(a) = self.arm(uid) else { return self.refuse(Refusal::DoesNotFit) };
        let label = label(a);
        self.edit_why(why, label, |s| s.arms.iter_mut().find(|a| a.uid == uid).is_some_and(f))
    }

    /// Adds a street at `bearing`, or at the middle of the widest gap when
    /// `bearing` is negative. Returns its uid, or 0 when it does not fit.
    pub fn add_arm(&mut self, street: usize, bearing: i32) -> u32 {
        if self.linked {
            self.refuse(Refusal::LinkedNoAdd);
            return 0;
        }
        if street >= SAMPLES.len() || SAMPLES[street].freeway {
            self.refuse(Refusal::DoesNotFit);
            return 0;
        }
        if self.current().arms.len() >= MAX_ARMS {
            self.refuse(Refusal::NoRoomForStreet);
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
                // Lanes that already serve a turn of the new street's kind take it too.
                let new_bearing = s.arms[i].bearing;
                let bearings: Vec<(u32, i32)> = s.arms.iter().map(|a| (a.uid, a.bearing)).collect();
                for a in s.arms.iter_mut().filter(|a| a.uid != uid) {
                    let class = turn_class(a.bearing, new_bearing);
                    for l in &mut a.lanes {
                        if l.to.iter().any(|t| bearings.iter().any(|(u, b)| u == t && turn_class(a.bearing, *b) == class)) {
                            l.to.push(uid);
                        }
                    }
                }
                true
            }) {
                self.next_uid += 1;
                self.selected = Target::Arm(uid);
                return uid;
            }
        }
        self.refuse(Refusal::NoRoomForStreet);
        0
    }

    pub fn remove_arm(&mut self, uid: u32) -> bool {
        let Some(a) = self.arm(uid) else { return self.refuse(Refusal::DoesNotFit) };
        if self.linked {
            return self.refuse(Refusal::LinkedNoRemove);
        }
        if self.current().arms.len() <= MIN_ARMS {
            return self.refuse(Refusal::NeedsThreeStreets);
        }
        let label = format!("Remove {}", arm_name(a));
        let ok = self.edit(label, |s| {
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
        self.arm_edit_why(Refusal::BearingBlocked, uid, |a| format!("{} bearing: {b}°", a.street_name()), |a| {
            a.bearing = b;
            true
        })
    }

    pub fn set_offset(&mut self, uid: u32, mm: i32) -> bool {
        let mm = snap(mm, OFFSET_STEP_MM);
        let region = self.region;
        self.arm_edit(uid, |a| format!("{} offset: {mm} mm", arm_name(a)), |a| {
            let half = a.profile(region).road_mm() / 2;
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
            return self.refuse(Refusal::DoesNotFit);
        }
        self.arm_edit(uid, |a| format!("Corner after {}: {mm} mm radius", arm_name(a)), |a| {
            a.corner_mm = mm;
            true
        })
    }

    pub fn set_street(&mut self, uid: u32, street: usize) -> bool {
        if self.linked {
            return self.refuse(Refusal::LinkedNoSwap);
        }
        if street >= SAMPLES.len() || SAMPLES[street].freeway {
            return self.refuse(Refusal::DoesNotFit);
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
            return self.refuse(Refusal::DoesNotFit);
        }
        self.arm_edit(uid, |a| format!("{} crossing set back {mm} mm", arm_name(a)), |a| {
            a.crossing.as_mut().map(|c| c.setback_mm = mm).is_some()
        })
    }

    pub fn set_crossing_width(&mut self, uid: u32, mm: i32) -> bool {
        let mm = snap(mm, RING_STEP_MM);
        if !(MIN_CROSSING_MM..=MAX_CROSSING_MM).contains(&mm) {
            return self.refuse(Refusal::DoesNotFit);
        }
        self.arm_edit(uid, |a| format!("{} crossing {mm} mm wide", arm_name(a)), |a| {
            a.crossing.as_mut().map(|c| c.width_mm = mm).is_some()
        })
    }

    pub fn set_island(&mut self, uid: u32, on: bool) -> bool {
        let region = self.region;
        if on && self.arm(uid).is_some_and(|a| a.profile(region).road_mm() < ISLAND_MIN_ROAD_MM) {
            return self.refuse(Refusal::IslandRoadTooNarrow);
        }
        self.arm_edit(uid, |a| format!("{} refuge island: {}", arm_name(a), if on { "add" } else { "remove" }), |a| {
            a.crossing.as_mut().map(|c| c.island = on).is_some()
        })
    }

    /// Side 0 is the arm's left curb, 1 its right.
    pub fn set_bulb(&mut self, uid: u32, side: usize, on: bool) -> bool {
        let region = self.region;
        if side > 1 {
            return self.refuse(Refusal::DoesNotFit);
        }
        let word = if side == 0 { "left" } else { "right" };
        if on && self.arm(uid).is_some_and(|a| a.profile(region).park[side] == 0) {
            return self.refuse(Refusal::BulbNoParking);
        }
        self.arm_edit(uid, |a| format!("{} {word} bulb-out: {}", arm_name(a), if on { "add" } else { "remove" }), |a| {
            a.bulb[side] = on;
            true
        })
    }

    /// Lets a lane go to a street, or not. A lane keeps at least one street.
    pub fn set_lane_dest(&mut self, uid: u32, lane: usize, to: u32, on: bool) -> bool {
        let Some(dest) = self.arm(to).map(arm_name) else { return self.refuse(Refusal::DoesNotFit) };
        if uid == to {
            return self.refuse(Refusal::DoesNotFit);
        }
        if !on && self.arm(uid).and_then(|a| a.lanes.get(lane)).is_some_and(|l| l.to == [to]) {
            return self.refuse(Refusal::LaneNeedsStreet);
        }
        self.arm_edit(
            uid,
            |a| format!("{} lane {}: {} {dest}", arm_name(a), lane + 1, if on { "to" } else { "not to" }),
            |a| {
                let Some(l) = a.lanes.get_mut(lane) else { return false };
                if on {
                    if l.to.contains(&to) {
                        return false;
                    }
                    l.to.push(to);
                } else {
                    if !l.to.contains(&to) {
                        return false;
                    }
                    l.to.retain(|u| *u != to);
                }
                true
            },
        )
    }

    pub fn set_bus_lane(&mut self, uid: u32, on: bool) -> bool {
        self.arm_edit(uid, |a| format!("{} bus lane: {}", arm_name(a), if on { "add" } else { "remove" }), |a| {
            a.bus_lane = on;
            true
        })
    }

    /// Sets the approach measure, by index into `APPROACHES`.
    pub fn set_approach(&mut self, uid: u32, approach: usize) -> bool {
        if approach >= APPROACHES.len() {
            return self.refuse(Refusal::DoesNotFit);
        }
        self.arm_edit(
            uid,
            |a| format!("{}: {}", arm_name(a), if approach == 0 { "no approach measure".to_string() } else { format!("{} {}", APPROACHES[approach].code, APPROACHES[approach].name.to_lowercase()) }),
            |a| {
                a.approach = approach;
                true
            },
        )
    }

    pub fn set_approach_len(&mut self, uid: u32, mm: i32) -> bool {
        let mm = snap(mm, APPROACH_STEP_MM);
        if !(APPROACH_MIN_MM..=APPROACH_MAX_MM).contains(&mm) {
            return self.refuse(Refusal::DoesNotFit);
        }
        self.arm_edit(uid, |a| format!("{} approach measure: {mm} mm", arm_name(a)), |a| {
            a.approach_mm = mm;
            true
        })
    }

    /// Sets the bus stop, by index into `STOPS`.
    pub fn set_stop(&mut self, uid: u32, stop: usize) -> bool {
        if stop >= STOPS.len() {
            return self.refuse(Refusal::DoesNotFit);
        }
        self.arm_edit(
            uid,
            |a| format!("{}: {}", arm_name(a), if stop == 0 { "no bus stop".to_string() } else { format!("{} {}", STOPS[stop].code, STOPS[stop].name.to_lowercase()) }),
            |a| {
                a.stop = stop;
                true
            },
        )
    }

    /// Sets the turn management, by index into `RULES`.
    pub fn set_rule(&mut self, uid: u32, rule: usize) -> bool {
        if rule >= RULES.len() {
            return self.refuse(Refusal::DoesNotFit);
        }
        self.arm_edit(
            uid,
            |a| format!("{}: {}", arm_name(a), if rule == 0 { "no turn management".to_string() } else { format!("{} {}", RULES[rule].code, RULES[rule].name.to_lowercase()) }),
            |a| {
                a.rule = rule;
                true
            },
        )
    }

    pub fn set_filter(&mut self, uid: u32, on: bool) -> bool {
        self.arm_edit(uid, |a| format!("{} transit modal filter (N1): {}", arm_name(a), if on { "add" } else { "remove" }), |a| {
            a.filter = on;
            true
        })
    }

    pub fn set_turn(&mut self, from: u32, to: u32, allowed: bool) -> bool {
        let (Some(a), Some(b)) = (self.arm(from), self.arm(to)) else { return self.refuse(Refusal::DoesNotFit) };
        if from == to {
            return self.refuse(Refusal::DoesNotFit);
        }
        let label = format!("{} to {}: {}", arm_name(a), arm_name(b), if allowed { "allow" } else { "no turn" });
        let others = self.current().arms.len() - 1;
        // An arm keeps at least one way out.
        if !allowed && a.banned.len() + 2 > others && !a.banned.contains(&to) {
            return self.refuse(Refusal::LastWayOut);
        }
        self.arm_edit(from, |_| label, |a| {
            if allowed {
                let before = a.banned.len();
                a.banned.retain(|&u| u != to);
                return a.banned.len() != before;
            }
            if a.banned.contains(&to) {
                return false;
            }
            a.banned.push(to);
            true
        })
    }

    pub fn set_control(&mut self, control: usize) -> bool {
        if control >= CONTROLS.len() {
            return self.refuse(Refusal::DoesNotFit);
        }
        // Only a roundabout can leave a junction that cannot be drawn.
        let why = if control == ROUNDABOUT { Refusal::RoundaboutTooBig } else { Refusal::DoesNotFit };
        self.edit_why(why, format!("Control: {}", CONTROLS[control].name.to_lowercase()), |s| {
            s.control = control;
            true
        })
    }

    /// Runs a bus-only lane across the middle of a roundabout between two
    /// streets, or takes it away with `None`.
    pub fn set_bus(&mut self, pair: Option<(u32, u32)>) -> bool {
        let pair = pair.map(|(a, b)| (a.min(b), a.max(b)));
        let label = match pair {
            Some((a, b)) => match (self.arm(a), self.arm(b)) {
                (Some(x), Some(y)) => format!("Bus lane across the middle: {} to {}", arm_name(x), arm_name(y)),
                _ => return self.refuse(Refusal::DoesNotFit),
            },
            None => "Bus lane across the middle: remove".into(),
        };
        self.edit(label, |s| {
            if pair.is_some() && (s.control != ROUNDABOUT || pair.is_some_and(|(a, b)| a == b)) {
                return false;
            }
            s.bus = pair;
            true
        })
    }

    /// Puts a cycle track of `width_mm` around the outside of a roundabout, or
    /// takes it away with `None`.
    pub fn set_cycle(&mut self, width_mm: Option<i32>) -> bool {
        let width = width_mm.map(|w| snap(w, RING_STEP_MM));
        if width.is_some_and(|w| !(CYCLE_MIN_MM..=CYCLE_MAX_MM).contains(&w)) {
            return self.refuse(Refusal::DoesNotFit);
        }
        let label = match width {
            Some(w) => format!("Cycle track around the roundabout: {w} mm"),
            None => "Cycle track around the roundabout: remove".into(),
        };
        self.edit(label, |s| {
            if width.is_some() && s.control != ROUNDABOUT {
                return false;
            }
            s.cycle = width;
            true
        })
    }

    /// Moves an arm's bearing one step round, clockwise for `dir` 1.
    pub fn step_bearing(&mut self, uid: u32, dir: i32) -> bool {
        let Some(a) = self.arm(uid) else { return self.refuse(Refusal::DoesNotFit) };
        self.set_bearing(uid, a.bearing + dir * BEARING_STEP)
    }

    pub fn step_offset(&mut self, uid: u32, dir: i32) -> bool {
        let Some(a) = self.arm(uid) else { return self.refuse(Refusal::DoesNotFit) };
        self.set_offset(uid, a.offset_mm + dir * OFFSET_STEP_MM)
    }

    /// Steps the curb radius at the corner clockwise of an arm.
    pub fn step_corner(&mut self, uid: u32, dir: i32) -> bool {
        let Some(a) = self.arm(uid) else { return self.refuse(Refusal::DoesNotFit) };
        self.set_corner(uid, a.corner_mm + dir * RING_STEP_MM)
    }

    pub fn step_setback(&mut self, uid: u32, dir: i32) -> bool {
        let Some(c) = self.arm(uid).and_then(|a| a.crossing) else { return self.refuse(Refusal::DoesNotFit) };
        self.set_setback(uid, c.setback_mm + dir * RING_STEP_MM)
    }

    pub fn step_crossing_width(&mut self, uid: u32, dir: i32) -> bool {
        let Some(c) = self.arm(uid).and_then(|a| a.crossing) else { return self.refuse(Refusal::DoesNotFit) };
        self.set_crossing_width(uid, c.width_mm + dir * RING_STEP_MM)
    }

    pub fn step_approach_len(&mut self, uid: u32, dir: i32) -> bool {
        let Some(a) = self.arm(uid) else { return self.refuse(Refusal::DoesNotFit) };
        self.set_approach_len(uid, a.approach_mm + dir * APPROACH_STEP_MM)
    }

    pub fn step_ring(&mut self, dir: i32) -> bool {
        self.set_ring(self.current().ring_extra_mm + dir * RING_STEP_MM)
    }

    /// Steps the width of the cycle track; there has to be one.
    pub fn step_cycle(&mut self, dir: i32) -> bool {
        let Some(w) = self.current().cycle else { return self.refuse(Refusal::DoesNotFit) };
        self.set_cycle(Some(w + dir * RING_STEP_MM))
    }

    /// Puts the cycle track around a roundabout at its usual width, or takes it away.
    pub fn set_cycle_track(&mut self, on: bool) -> bool {
        self.set_cycle(on.then_some(CYCLE_DEFAULT_MM))
    }

    /// Sets how much bigger than its least size a roundabout is.
    pub fn set_ring(&mut self, extra_mm: i32) -> bool {
        let extra = snap(extra_mm, RING_STEP_MM);
        if !(0..=MAX_RING_MM).contains(&extra) {
            return self.refuse(Refusal::DoesNotFit);
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

/// A state that can be drawn, or the nearest to it that can: first with the
/// corners as small as they go, then with offsets, a roundabout and its extras
/// undone. None when neither can be drawn.
fn ease(s: &State, region: usize) -> Option<State> {
    if valid(s, region) {
        return Some(s.clone());
    }
    let mut t = s.clone();
    for a in &mut t.arms {
        a.corner_mm = MIN_CORNER_MM;
    }
    if valid(&t, region) {
        return Some(t);
    }
    for a in &mut t.arms {
        a.offset_mm = 0;
    }
    if t.control == ROUNDABOUT {
        t.control = PRIORITY;
    }
    (t.bus, t.cycle, t.ring_extra_mm) = (None, None, 0);
    valid(&t, region).then_some(t)
}

/// Keeps the parts that follow from the arms consistent: lane counts follow
/// each street, a lane serves only turns its arm has, bans name arms that
/// exist, and features that need room or parking stay only where they have it.
pub fn normalize(arms: &mut [Arm], region: usize) {
    let uids: Vec<u32> = arms.iter().map(|a| a.uid).collect();
    let bearings: Vec<i32> = arms.iter().map(|a| a.bearing).collect();
    let avail: Vec<u8> = (0..arms.len()).map(|i| classes_of(arms, i)).collect();
    for (a, avail) in arms.iter_mut().zip(avail) {
        let p = a.profile(region);
        let n = p.enter_x.len();
        let (uid, bearing) = (a.uid, a.bearing);
        // Where lane `k` goes by default: every street whose turn it serves.
        let default = |k: usize| -> Vec<u32> {
            let want = default_uses(k, n, avail);
            uids.iter().zip(&bearings).filter(|(u, b)| **u != uid && want & turn_class(bearing, **b) != 0).map(|(u, _)| *u).collect()
        };
        if a.lanes.len() != n {
            a.lanes = (0..n).map(|k| Lane { to: default(k) }).collect();
        }
        for (k, l) in a.lanes.iter_mut().enumerate() {
            l.to.retain(|u| uids.contains(u) && *u != uid);
            l.to.sort_unstable();
            l.to.dedup();
            if l.to.is_empty() {
                l.to = default(k);
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
    fn a_lane_goes_to_streets_one_by_one_and_always_to_at_least_one() {
        let mut j = Junction::new(1); // street, lane (south), street
        let lane = j.current().arms.iter().find(|a| a.street == 2).unwrap().uid;
        let (e, w) = (arm_at(&j, 90), arm_at(&j, 270));
        assert_eq!(j.arm(lane).unwrap().lanes[0].to, vec![e, w], "the stem's one lane goes both ways");
        assert!(!j.set_lane_dest(lane, 0, e, true), "already goes there");
        assert!(j.set_lane_dest(lane, 0, e, false));
        assert!(!j.set_lane_dest(lane, 0, w, false), "a lane must go somewhere");
        assert!(!j.set_lane_dest(lane, 0, lane, true), "not back where it came from");
        assert!(j.set_lane_dest(lane, 0, e, true));
    }

    #[test]
    fn two_straight_ons_at_a_five_way_can_be_split_between_lanes() {
        let mut j = Junction::new(3);
        let av = arm_at(&j, 145); // two lanes in; north and north-west are both straight on from here
        let (n, nw) = (arm_at(&j, 0), arm_at(&j, 290));
        let to = |j: &Junction, lane: usize| j.arm(av).unwrap().lanes[lane].to.clone();
        assert!(to(&j, 0).contains(&n) && to(&j, 0).contains(&nw));
        assert!(to(&j, 1).contains(&n) && to(&j, 1).contains(&nw));
        assert!(j.set_lane_dest(av, 1, nw, false));
        assert!(to(&j, 0).contains(&nw) && !to(&j, 1).contains(&nw), "one lane to each");
    }

    fn arm_at(j: &Junction, bearing: i32) -> u32 {
        j.current().arms.iter().find(|a| a.bearing == bearing).unwrap().uid
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
    fn a_bus_lane_across_the_middle_needs_a_roundabout_and_goes_when_either_street_does() {
        let mut j = Junction::new(0);
        let (n, s, e) = (j.current().arms[0].uid, j.current().arms[2].uid, j.current().arms[1].uid);
        assert!(!j.set_bus(Some((n, s))), "only a roundabout has a middle");
        assert!(j.set_control(ROUNDABOUT));
        assert!(!j.set_bus(Some((n, n))));
        assert!(j.set_bus(Some((s, n))));
        assert_eq!(j.current().bus, Some((n, s)));
        assert!(j.remove_arm(e));
        assert_eq!(j.current().bus, Some((n, s)), "an unrelated street leaving keeps it");
        let mut k = Junction::new(0);
        k.set_control(ROUNDABOUT);
        let (n, s) = (k.current().arms[0].uid, k.current().arms[2].uid);
        k.set_bus(Some((n, s)));
        assert!(k.set_control(SIGNAL));
        assert_eq!(k.current().bus, None);
        assert!(k.undo());
        assert_eq!(k.current().bus, Some((n, s)));
    }

    #[test]
    fn the_bus_lane_can_be_selected_and_is_dropped_with_it() {
        let mut j = Junction::new(0);
        j.set_control(ROUNDABOUT);
        let (n, s) = (j.current().arms[0].uid, j.current().arms[2].uid);
        j.set_bus(Some((n, s)));
        j.select(Target::Bus);
        assert_eq!(j.selected, Target::Bus);
        assert_eq!(j.targets().last(), Some(&Target::Bus));
        j.set_bus(None);
        assert_eq!(j.selected, Target::None);
    }

    #[test]
    fn a_cycle_track_needs_a_roundabout_and_goes_with_it() {
        let mut j = Junction::new(0);
        assert!(!j.set_cycle(Some(2000)));
        assert!(j.set_control(ROUNDABOUT));
        assert!(!j.set_cycle(Some(1000)) && !j.set_cycle(Some(3500)));
        assert!(j.set_cycle(Some(2200)));
        assert_eq!(j.current().cycle, Some(2000), "snaps to half metres");
        j.select(Target::Cycle);
        assert_eq!(j.selected, Target::Cycle);
        assert!(j.set_control(SIGNAL));
        assert_eq!(j.current().cycle, None);
        assert_eq!(j.selected, Target::None);
        assert!(j.undo());
        assert_eq!(j.current().cycle, Some(2000));
    }

    #[test]
    fn selection_walks_clockwise_and_survives_removal() {
        let mut j = Junction::new(0);
        j.select_relative(1);
        assert_eq!(j.selected, Target::Arm(j.current().arms[0].uid));
        j.select_relative(1);
        assert_eq!(j.selected, Target::Lane(j.current().arms[0].uid, 0));
        j.select_relative(1);
        assert_eq!(j.selected, Target::Lane(j.current().arms[0].uid, 1));
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
    fn a_lane_can_be_selected_and_is_dropped_when_it_goes() {
        let mut j = Junction::new(0);
        let n = j.current().arms[0].uid;
        j.select(Target::Lane(n, 1));
        assert_eq!(j.selected, Target::Lane(n, 1));
        j.select(Target::Lane(n, 2));
        assert_eq!(j.selected, Target::None, "the avenue has two lanes in");
        j.select(Target::Lane(n, 0));
        assert!(j.set_street(n, 2)); // a lane has one lane in
        assert_eq!(j.selected, Target::Lane(n, 0));
        j.select(Target::Lane(n, 1));
        assert_eq!(j.selected, Target::None);
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

    fn linked(sample: usize) -> Junction {
        let mut state = Junction::new(sample).current().clone();
        for a in &mut state.arms {
            a.edge = a.uid;
            a.section = Some(Street::sample(a.street, Side::Right));
        }
        Junction::from_city("Test junction", &state, &state, 0).expect("the sample draws")
    }

    #[test]
    fn a_junction_in_a_city_keeps_its_streets_and_name() {
        let mut j = linked(0);
        assert!(j.is_linked());
        let uid = j.current().arms[0].uid;
        assert!(!j.remove_arm(uid));
        assert!(!j.set_street(uid, 2));
        assert_eq!(j.add_arm(2, 45), 0);
        assert_eq!(j.view().name, "Test junction");
        assert!(j.view().linked);
        // What it keeps for the city has no street attached.
        assert!(j.snapshot().arms.iter().all(|a| a.section.is_none() && a.edge != 0));
        // Other edits still work, and Start over goes back to today.
        assert!(j.set_corner(uid, 4_000));
        assert!(j.changed());
        assert!(j.reset());
        assert!(!j.changed());
    }

    #[test]
    fn a_junction_reads_the_street_the_city_holds_not_the_sample() {
        let mut state = Junction::new(0).current().clone();
        let mut narrow = Street::sample(0, Side::Right);
        narrow.segments.retain(|g| KINDS[g.kind].id != "parking");
        for a in &mut state.arms {
            a.edge = a.uid;
            a.section = Some(if a.street == 0 { narrow.clone() } else { Street::sample(a.street, Side::Right) });
        }
        let j = Junction::from_city("Test", &state, &state, 0).unwrap();
        let arm = j.current().arms.iter().find(|a| a.street == 0).unwrap();
        assert_eq!(arm.profile(0).park, [0, 0]);
        assert!(profile(0, 0).park != [0, 0]);
    }

    #[test]
    fn earlier_changes_show_as_one_revision_and_start_over_clears_them() {
        let today = linked(0).snapshot();
        let mut state = today.clone();
        state.control = PRIORITY;
        for a in &mut state.arms {
            a.section = Some(Street::sample(a.street, Side::Right));
        }
        let mut today = today;
        for a in &mut today.arms {
            a.section = Some(Street::sample(a.street, Side::Right));
        }
        let mut j = Junction::from_city("Test", &today, &state, 0).unwrap();
        assert!(j.changed());
        assert_eq!(j.revisions().collect::<Vec<_>>(), ["Earlier changes"]);
        assert_eq!(j.current().control, PRIORITY);
        assert!(j.undo());
        assert_eq!(j.current().control, SIGNAL);
    }

    #[test]
    fn stepping_a_bearing_moves_it_one_step_and_wraps_past_north() {
        let mut j = Junction::new(0);
        let n = arm_at(&j, 0);
        assert!(j.step_bearing(n, 1));
        assert_eq!(j.arm(n).unwrap().bearing, BEARING_STEP);
        assert!(j.step_bearing(n, -1) && j.step_bearing(n, -1));
        assert_eq!(j.arm(n).unwrap().bearing, 360 - BEARING_STEP);
    }

    #[test]
    fn stepping_a_bearing_towards_its_neighbour_stops_where_the_model_refuses() {
        let mut j = Junction::new(0);
        let e = arm_at(&j, 90);
        while j.step_bearing(e, -1) {}
        let stopped = j.arm(e).unwrap().bearing;
        assert!(stopped > 0 && stopped < 90, "stopped at {stopped}");
        assert!(!j.step_bearing(e, -1));
        assert_eq!(j.arm(e).unwrap().bearing, stopped);
    }

    #[test]
    fn stepping_an_offset_moves_it_a_tenth_of_a_metre_either_way() {
        let mut j = Junction::new(0);
        let n = arm_at(&j, 0);
        assert!(j.step_offset(n, 1));
        assert_eq!(j.arm(n).unwrap().offset_mm, OFFSET_STEP_MM);
        assert!(j.step_offset(n, -1) && j.step_offset(n, -1));
        assert_eq!(j.arm(n).unwrap().offset_mm, -OFFSET_STEP_MM);
    }

    #[test]
    fn stepping_a_corner_changes_its_radius_by_a_step_within_the_limits() {
        let mut j = Junction::new(0);
        let n = arm_at(&j, 0);
        assert!(j.step_corner(n, 1));
        assert_eq!(j.arm(n).unwrap().corner_mm, DEFAULT_CORNER_MM + RING_STEP_MM);
        assert!(j.set_corner(n, MIN_CORNER_MM));
        assert!(!j.step_corner(n, -1));
        assert_eq!(j.arm(n).unwrap().corner_mm, MIN_CORNER_MM);
    }

    #[test]
    fn stepping_a_setback_stops_at_its_limits_and_needs_a_crossing() {
        let mut j = Junction::new(0);
        let e = arm_at(&j, 90);
        assert!(j.step_setback(e, 1));
        assert_eq!(j.arm(e).unwrap().crossing.unwrap().setback_mm, DEFAULT_SETBACK_MM + RING_STEP_MM);
        assert!(j.set_setback(e, MAX_SETBACK_MM));
        assert!(!j.step_setback(e, 1));
        assert!(j.set_crossing(e, false));
        assert!(!j.step_setback(e, 1));
    }

    #[test]
    fn stepping_a_crossing_width_stops_at_its_limits_and_needs_a_crossing() {
        let mut j = Junction::new(0);
        let e = arm_at(&j, 90);
        assert!(j.step_crossing_width(e, 1));
        assert_eq!(j.arm(e).unwrap().crossing.unwrap().width_mm, DEFAULT_CROSSING_MM + RING_STEP_MM);
        assert!(j.set_crossing_width(e, MAX_CROSSING_MM));
        assert!(!j.step_crossing_width(e, 1));
        assert!(j.set_crossing(e, false));
        assert!(!j.step_crossing_width(e, 1));
    }

    #[test]
    fn stepping_an_approach_length_moves_it_by_its_step_within_the_limits() {
        let mut j = Junction::new(0);
        let n = arm_at(&j, 0);
        assert!(j.step_approach_len(n, 1));
        assert_eq!(j.arm(n).unwrap().approach_mm, APPROACH_DEFAULT_MM + APPROACH_STEP_MM);
        assert!(!j.step_approach_len(n, 1));
        assert!(j.step_approach_len(n, -1) && j.step_approach_len(n, -1));
        assert_eq!(j.arm(n).unwrap().approach_mm, APPROACH_DEFAULT_MM - APPROACH_STEP_MM);
        assert!(!j.step_approach_len(n, -1));
    }

    #[test]
    fn stepping_the_roundabout_changes_its_size_but_never_below_the_least() {
        let mut j = Junction::new(0);
        assert!(j.set_control(ROUNDABOUT));
        assert!(!j.step_ring(-1));
        assert!(j.step_ring(1));
        assert_eq!(j.current().ring_extra_mm, RING_STEP_MM);
        assert!(j.step_ring(-1));
        assert_eq!(j.current().ring_extra_mm, 0);
    }

    #[test]
    fn stepping_the_cycle_track_stays_within_its_widths_and_needs_one() {
        let mut j = Junction::new(0);
        assert!(j.set_control(ROUNDABOUT));
        assert!(!j.step_cycle(1), "no track to widen");
        assert!(j.set_cycle(Some(CYCLE_MIN_MM)));
        assert!(!j.step_cycle(-1));
        assert!(j.step_cycle(1) && j.step_cycle(1) && j.step_cycle(1));
        assert_eq!(j.current().cycle, Some(CYCLE_MAX_MM));
        assert!(!j.step_cycle(1));
    }

    #[test]
    fn switching_the_cycle_track_on_gives_it_the_default_width() {
        let mut j = Junction::new(0);
        assert!(!j.set_cycle_track(true), "only a roundabout has one");
        assert!(j.set_control(ROUNDABOUT));
        assert!(j.set_cycle_track(true));
        assert_eq!(j.current().cycle, Some(CYCLE_DEFAULT_MM));
        assert!(j.set_cycle_track(false));
        assert_eq!(j.current().cycle, None);
    }

    #[test]
    fn a_refused_edit_says_why_and_a_taken_one_clears_it() {
        let mut j = Junction::new(0);
        let n = arm_at(&j, 0);
        assert_eq!(j.refusal(), None);
        assert!(!j.set_corner(n, 99_000));
        assert_eq!(j.refusal(), Some(Refusal::DoesNotFit));
        assert!(j.set_corner(n, 7_000));
        assert_eq!(j.refusal(), None);
    }

    #[test]
    fn every_refusal_has_a_message() {
        for r in Refusal::ALL {
            assert!(r.message().ends_with('.'), "{r:?}");
        }
    }

    #[test]
    fn a_sixth_street_has_no_room() {
        let mut j = Junction::new(3); // five ways
        assert_eq!(j.add_arm(0, -1), 0);
        assert_eq!(j.refusal(), Some(Refusal::NoRoomForStreet));
    }

    #[test]
    fn a_junction_keeps_three_streets() {
        let mut j = Junction::new(1);
        let uid = j.current().arms[0].uid;
        assert!(!j.remove_arm(uid));
        assert_eq!(j.refusal(), Some(Refusal::NeedsThreeStreets));
    }

    #[test]
    fn the_streets_of_a_city_junction_cannot_be_added_removed_or_swapped() {
        let mut j = linked(1); // three streets: removal is also below the least
        let uid = j.current().arms[0].uid;
        assert!(!j.remove_arm(uid));
        assert_eq!(j.refusal(), Some(Refusal::LinkedNoRemove));
        assert_eq!(j.add_arm(0, -1), 0);
        assert_eq!(j.refusal(), Some(Refusal::LinkedNoAdd));
        assert!(!j.set_street(uid, 2));
        assert_eq!(j.refusal(), Some(Refusal::LinkedNoSwap));
    }

    #[test]
    fn a_bearing_too_close_to_a_neighbour_is_refused_for_that() {
        let mut j = Junction::new(0);
        let e = arm_at(&j, 90);
        assert!(!j.set_bearing(e, 10));
        assert_eq!(j.refusal(), Some(Refusal::BearingBlocked));
    }

    #[test]
    fn a_street_keeps_a_way_out() {
        let mut j = Junction::new(0);
        let n = arm_at(&j, 0);
        let others: Vec<u32> = j.current().arms.iter().map(|a| a.uid).filter(|u| *u != n).collect();
        assert!(j.set_turn(n, others[0], false));
        assert!(j.set_turn(n, others[1], false));
        assert!(!j.set_turn(n, others[2], false));
        assert_eq!(j.refusal(), Some(Refusal::LastWayOut));
    }

    #[test]
    fn a_lane_keeps_a_street_to_go_to() {
        let mut j = Junction::new(0);
        let n = arm_at(&j, 0);
        let to = j.arm(n).unwrap().lanes[0].to.clone();
        for u in &to[1..] {
            assert!(j.set_lane_dest(n, 0, *u, false));
        }
        assert!(!j.set_lane_dest(n, 0, to[0], false));
        assert_eq!(j.refusal(), Some(Refusal::LaneNeedsStreet));
    }

    #[test]
    fn a_narrow_road_has_no_island_and_a_street_without_parking_no_bulge() {
        let mut j = Junction::new(1); // the stem is Sample Lane 3: 8.4 m road, parking on one side
        let lane = j.current().arms.iter().find(|a| a.street == 2).unwrap().uid;
        assert!(!j.set_island(lane, true));
        assert_eq!(j.refusal(), Some(Refusal::IslandRoadTooNarrow));
        assert!(!j.set_bulb(lane, 0, true));
        assert_eq!(j.refusal(), Some(Refusal::BulbNoParking));
    }

    #[test]
    fn an_edit_that_cannot_be_drawn_is_refused_with_the_reason_it_was_given() {
        let mut j = Junction::new(0);
        // Two streets on one bearing cannot be drawn.
        assert!(!j.edit_why(Refusal::RoundaboutTooBig, "x".into(), |s| {
            s.arms[1].bearing = s.arms[0].bearing;
            true
        }));
        assert_eq!(j.refusal(), Some(Refusal::RoundaboutTooBig));
    }

    #[test]
    fn a_control_that_does_not_exist_is_refused_as_not_fitting() {
        let mut j = Junction::new(0);
        assert!(!j.set_control(99));
        assert_eq!(j.refusal(), Some(Refusal::DoesNotFit));
    }

    #[test]
    fn every_refused_edit_leaves_a_reason() {
        let j = Junction::new(0);
        let n = arm_at(&j, 0);
        let e = arm_at(&j, 90);
        let ops: Vec<(&str, Box<dyn Fn(&mut Junction) -> bool>)> = vec![
            ("bearing", Box::new(move |j| j.set_bearing(e, 10))),
            ("offset", Box::new(move |j| j.set_offset(n, 99_000))),
            ("corner", Box::new(move |j| j.set_corner(n, 99_000))),
            ("street", Box::new(move |j| j.set_street(n, 99))),
            ("setback", Box::new(move |j| j.set_setback(n, 99_000))),
            ("crossing width", Box::new(move |j| j.set_crossing_width(n, 99_000))),
            ("bulb side", Box::new(move |j| j.set_bulb(n, 2, true))),
            ("lane", Box::new(move |j| j.set_lane_dest(n, 9, e, true))),
            ("lane to self", Box::new(move |j| j.set_lane_dest(n, 0, n, true))),
            ("approach", Box::new(move |j| j.set_approach(n, 99))),
            ("approach length", Box::new(move |j| j.set_approach_len(n, 99_000))),
            ("stop", Box::new(move |j| j.set_stop(n, 99))),
            ("rule", Box::new(move |j| j.set_rule(n, 99))),
            ("turn to self", Box::new(move |j| j.set_turn(n, n, false))),
            ("control", Box::new(|j| j.set_control(99))),
            ("bus", Box::new(move |j| j.set_bus(Some((n, e))))),
            ("cycle", Box::new(|j| j.set_cycle(Some(2_000)))),
            ("cycle width", Box::new(|j| j.set_cycle(Some(99_000)))),
            ("ring", Box::new(|j| j.set_ring(-500))),
            ("unknown arm", Box::new(|j| j.set_offset(9_999, 100))),
            ("step crossing none", Box::new(move |j| j.set_crossing(n, false) && j.step_setback(n, 1))),
            ("step cycle none", Box::new(|j| j.step_cycle(1))),
            ("step unknown arm", Box::new(|j| j.step_bearing(9_999, 1))),
        ];
        for (name, op) in &ops {
            let mut k = Junction::new(0);
            k.refusal = None;
            assert!(!op(&mut k), "{name} was meant to be refused");
            assert!(k.refusal().is_some(), "{name} gave no reason");
        }
    }
}
