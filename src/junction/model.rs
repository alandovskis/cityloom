//! The junction editing model: three to five streets meeting in plan, with
//! corners, crossings, the lanes each street brings, turn bans and a control.
//! Pure Rust like `model.rs`, so it is tested natively. Lengths are integer
//! millimetres and angles integer degrees. Everything here is synthetic.

use crate::shared::catalogue::{KINDS, REGIONS, Side, StreetClass, is_roadway};
use crate::shared::provenance::OsmRef;
use crate::shared::said::{Arg, Said};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use crate::junction::geometry::gap;
use crate::junction::signal::{self, Pattern};
use crate::junction::turbo::{self, BLOCKS, LAYOUTS, Turbo};
use crate::street::model::{Beyond, Editor, Street, View, named_said};

#[cfg(test)]
mod fixtures;
#[cfg(test)]
pub use fixtures::{JUNCTION_SAMPLES, junction_of, junction_two_way, junction_with_walkways, suspended_turbo};

pub const LEFT: u8 = 1;
pub const THROUGH: u8 = 2;
pub const RIGHT: u8 = 4;

pub const UNCONTROLLED: usize = 0;
pub const PRIORITY: usize = 1;
pub const ALL_WAY_STOP: usize = 2;
pub const SIGNAL: usize = 3;
pub const ROUNDABOUT: usize = 4;

/// How a junction is run: its id, and the message that names it.
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct Control {
    pub id: &'static str,
    #[serde(skip)]
    pub key: &'static str,
}

pub const CONTROLS: [Control; 5] = [
    Control { id: "uncontrolled", key: "jn-control-uncontrolled" },
    Control { id: "priority", key: "jn-control-priority" },
    Control { id: "stop", key: "jn-control-stop" },
    Control { id: "signal", key: "jn-control-signal" },
    Control { id: "roundabout", key: "jn-control-roundabout" },
];

/// The message that names control `i` (an index into `CONTROLS`).
pub fn control_key(i: usize) -> &'static str {
    CONTROLS[i].key
}

/// One entry of a short list a junction measure is chosen from. `code` is the
/// Transit Priority Atlas toolbox code; `key` the message that names it.
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct Item {
    pub id: &'static str,
    pub code: &'static str,
    #[serde(skip)]
    pub key: &'static str,
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
    Item { id: "none", code: "", key: "jn-approach-none" },
    Item { id: "queue-offset", code: "G1", key: "jn-measure-g1" },
    Item { id: "queue-curb", code: "G2", key: "jn-measure-g2" },
    Item { id: "queue-virtual", code: "G3", key: "jn-measure-g3" },
    Item { id: "gate-signal", code: "H1", key: "jn-measure-h1" },
    Item { id: "gate-yield", code: "H2", key: "jn-measure-h2" },
];

pub const STOP_BULB: usize = 1;
pub const STOP_PLATFORM: usize = 2;

pub const STOPS: [Item; 3] = [
    Item { id: "none", code: "", key: "jn-stop-none" },
    Item { id: "bulb", code: "M1", key: "jn-measure-m1" },
    Item { id: "platform", code: "M2", key: "jn-measure-m2" },
];

/// The Atlas code of the transit modal filter, which an arm has or has not.
pub const FILTER_CODE: &str = "N1";

pub const RULE_AROUND: usize = 1;
pub const RULE_WITHIN: usize = 2;
pub const RULE_RIRO: usize = 3;
pub const RULE_DEAD_END: usize = 4;

pub const RULES: [Item; 5] = [
    Item { id: "none", code: "", key: "jn-rule-none" },
    Item { id: "around", code: "L1", key: "jn-measure-l1" },
    Item { id: "within", code: "L2", key: "jn-measure-l2" },
    Item { id: "riro", code: "L3", key: "jn-measure-l3" },
    Item { id: "dead-end", code: "L4", key: "jn-measure-l4" },
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
    /// The sidewalk runs on across the mouth of the street at its own level, in place of a zebra.
    #[serde(default)]
    pub continuous: bool,
}

/// Limits and defaults of a signal's timing, in whole seconds. Synthetic, like the rest.
pub const GREEN_MIN_S: i32 = 5;
pub const GREEN_MAX_S: i32 = 120;
pub const GREEN_DEFAULT_S: u32 = 25;
pub const YELLOW_MIN_S: i32 = 3;
pub const YELLOW_MAX_S: i32 = 6;
pub const YELLOW_DEFAULT_S: u32 = 3;
pub const ALL_RED_MIN_S: i32 = 1;
pub const ALL_RED_MAX_S: i32 = 6;
pub const ALL_RED_DEFAULT_S: u32 = 2;
/// The most a transit lane's early green may lead its street's own green by.
pub const LEAD_MAX_S: i32 = 15;

/// How long a street's signal phase runs: the early green for transit (if any), the green, the yellow and the
/// all-red before the next street's turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timing {
    pub green_s: u32,
    pub yellow_s: u32,
    pub all_red_s: u32,
    pub transit_lead_s: u32,
}

impl Default for Timing {
    fn default() -> Timing {
        Timing { green_s: GREEN_DEFAULT_S, yellow_s: YELLOW_DEFAULT_S, all_red_s: ALL_RED_DEFAULT_S, transit_lead_s: 0 }
    }
}

impl Timing {
    /// The seconds the whole phase takes.
    pub fn length_s(&self) -> u32 {
        self.transit_lead_s + self.green_s + self.yellow_s + self.all_red_s
    }
}

/// One of the four numbers of a `Timing`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimingField {
    Green,
    Yellow,
    AllRed,
    TransitLead,
}

impl TimingField {
    pub const ALL: [TimingField; 4] = [TimingField::Green, TimingField::Yellow, TimingField::AllRed, TimingField::TransitLead];

    /// The least and the most seconds it takes.
    pub fn range(self) -> (i32, i32) {
        match self {
            TimingField::Green => (GREEN_MIN_S, GREEN_MAX_S),
            TimingField::Yellow => (YELLOW_MIN_S, YELLOW_MAX_S),
            TimingField::AllRed => (ALL_RED_MIN_S, ALL_RED_MAX_S),
            TimingField::TransitLead => (0, LEAD_MAX_S),
        }
    }

    /// The message that names it.
    pub fn key(self) -> &'static str {
        match self {
            TimingField::Green => "jn-timing-green",
            TimingField::Yellow => "jn-timing-yellow",
            TimingField::AllRed => "jn-timing-all-red",
            TimingField::TransitLead => "jn-timing-transit-lead",
        }
    }

    pub fn get(self, t: &Timing) -> u32 {
        match self {
            TimingField::Green => t.green_s,
            TimingField::Yellow => t.yellow_s,
            TimingField::AllRed => t.all_red_s,
            TimingField::TransitLead => t.transit_lead_s,
        }
    }

    fn put(self, t: &mut Timing, seconds: u32) {
        match self {
            TimingField::Green => t.green_s = seconds,
            TimingField::Yellow => t.yellow_s = seconds,
            TimingField::AllRed => t.all_red_s = seconds,
            TimingField::TransitLead => t.transit_lead_s = seconds,
        }
    }
}

/// An entering lane and the streets it can go to, by their uids.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lane {
    pub to: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Arm {
    pub uid: u32,
    /// The city street this arm is, or 0 when it is not part of a city.
    #[serde(default)]
    pub edge: u32,
    /// That street, seen looking outward from the junction. The city puts the
    /// street's current state here when it opens the junction, so it is not
    /// kept with the junction.
    #[serde(skip)]
    pub section: Option<Street>,
    /// The neighbour of the end zone across its first boundary, as the street has it now; set with `section`, never
    /// kept.
    #[serde(skip)]
    pub beyond: Option<Beyond>,
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
    /// A waiting box for cyclists across the entering lanes, ahead of the cars' stop line.
    #[serde(default)]
    pub bike_box: bool,
    /// The corner clockwise of this arm is a protected corner: the bike lanes of the two streets go round it on a
    /// track of their own, round a corner island.
    #[serde(default)]
    pub protected: bool,
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
    /// The arm's signal timing, once it has been set; it reads as `Timing::default()` until then. It exists only while
    /// the junction's control is a signal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timing: Option<Timing>,
    /// The street was a pedestrian one the last time the arm was settled: when it carries traffic again the arm gets
    /// the crossing and the lanes a new arm has, once.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub walked: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    /// What the step to this state was, for the history of one visit to the page. It is not kept:
    /// a saved state comes back as "Earlier changes" (and an older save's `label` is ignored).
    #[serde(skip, default = "earlier")]
    pub label: Said,
    pub arms: Vec<Arm>,
    pub control: usize,
    /// Roundabout size above the least that keeps the arms apart.
    pub ring_extra_mm: i32,
    /// A bus-only lane straight across the middle of a roundabout, between two
    /// streets, by their uids (the lower first).
    pub bus: Option<(u32, u32)>,
    /// Width of a cycle track around the outside of a roundabout, if it has one.
    pub cycle: Option<i32>,
    /// The whole carriageway of the junction, crossings included, is raised to sidewalk height, with a ramp on
    /// each arm. Never a roundabout's.
    #[serde(default)]
    pub raised: bool,
    /// The turbo layout of a roundabout, if it has one (only while `control` is `ROUNDABOUT`; a state that says
    /// otherwise is settled to none). A record made before turbos has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turbo: Option<Turbo>,
    /// The signal plan the person has edited; none means the suggestion (`signal::plan`). Only while `control` is a
    /// signal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signal_plan: Option<crate::junction::signal::SignalPlan>,
    /// The OSM nodes it was made from. The sample junctions are made from none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source: Vec<OsmRef>,
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

impl Target {
    /// The target the view calls `kind` (its `selected.kind`), at `uid`, and
    /// at `lane` for a lane. An unknown name is no target.
    pub fn named(kind: &str, uid: u32, lane: usize) -> Target {
        match kind {
            "arm" => Target::Arm(uid),
            "corner" => Target::Corner(uid),
            "crossing" => Target::Crossing(uid),
            "lane" => Target::Lane(uid, lane),
            "bus" => Target::Bus,
            "cycle" => Target::Cycle,
            _ => Target::None,
        }
    }
}

/// The label of a state whose own label is not known: one read back from storage. It is shown, in the
/// history, as "Earlier changes".
pub fn earlier() -> Said {
    Said::new("city-earlier")
}

/// A name or label that stands for nothing: where a `Said` is needed and nothing shows it (a junction
/// opened to be checked, two states compared for everything but their label).
pub fn unlabelled() -> Said {
    named_said("")
}

/// The label of the junction as the city first laid it out.
pub fn today() -> Said {
    Said::new("city-junction-today")
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
    pub name: Said,
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
    /// What lies just beyond the left and right curb: the kind (an index into `KINDS`) of the nearest piece
    /// outside the carriageway, or `None` where the carriageway reaches the property line.
    pub edge_kind: [Option<usize>; 2],
}

/// The profile of a street a city holds, with its lanes written for the side
/// of the road of `region`.
pub fn profile_of(street: &Street, region: usize) -> Profile {
    let e = Editor::from_street(street, street, region);
    read_profile(&e.view(), street.title())
}

/// How many of the junction's streets carry traffic: every arm but the pedestrian ones.
pub fn traffic_arms(s: &State) -> usize {
    s.arms.iter().filter(|a| !a.is_pedestrian()).count()
}

/// The state as the junction is run. With at most one street carrying traffic there is nothing to control, so
/// it runs uncontrolled whatever control was kept; the kept control returns when traffic does. What reads the
/// control (and a turbo) to draw or judge the junction asks for this; an edit changes the kept state.
pub fn in_force(s: &State) -> Cow<'_, State> {
    if s.control != UNCONTROLLED && traffic_arms(s) <= 1 {
        // A turbo is part of a roundabout: with the roundabout suspended it is too. The kept state still has it.
        Cow::Owned(State { control: UNCONTROLLED, turbo: None, ..s.clone() })
    } else {
        Cow::Borrowed(s)
    }
}

/// The profile of a street, kept. Reading a street's profile builds an editor on it, and a junction asks for it of
/// every arm many times over, so what has been read is kept.
fn read_cached(street: &Street, region: usize) -> Profile {
    thread_local! {
        static READ: std::cell::RefCell<std::collections::HashMap<(Street, usize), Profile>> = Default::default();
    }
    let key = (street.clone(), region);
    if let Some(p) = READ.with(|r| r.borrow().get(&key).cloned()) {
        return p;
    }
    let p = profile_of(street, region);
    READ.with(|r| {
        let mut r = r.borrow_mut();
        if r.len() > 4096 {
            r.clear();
        }
        r.insert(key, p.clone());
    });
    p
}

impl Arm {
    /// An arm with the defaults a street starts with at a junction.
    pub fn new(uid: u32, bearing: i32, offset_mm: i32) -> Arm {
        Arm {
            uid,
            edge: 0,
            section: None,
            beyond: None,
            bearing,
            offset_mm,
            corner_mm: DEFAULT_CORNER_MM,
            lanes: Vec::new(),
            crossing: Some(default_crossing()),
            bulb: [false, false],
            bike_box: false,
            protected: false,
            banned: Vec::new(),
            bus_lane: false,
            approach: 0,
            approach_mm: APPROACH_DEFAULT_MM,
            stop: 0,
            rule: 0,
            filter: false,
            timing: None,
            walked: false,
        }
    }

    pub fn profile(&self, region: usize) -> Profile {
        read_cached(self.section.as_ref().expect("an arm carries the street it reads"), region)
    }

    /// The profile of the zone across the end zone's boundary, read as the end zone is; `None` without a beyond.
    pub fn beyond_profile(&self, region: usize) -> Option<Profile> {
        let (street, beyond) = (self.section.as_ref()?, self.beyond.as_ref()?);
        Some(read_cached(&Street { segments: beyond.next.clone(), ..street.clone() }, region))
    }

    /// The name of the street the arm reads; a junction outside a city reads one always.
    pub fn street_name(&self) -> Said {
        self.section.as_ref().map_or_else(|| named_said(""), Street::title)
    }

    pub fn row_mm(&self) -> i32 {
        self.section.as_ref().map_or(0, |s| s.row_mm)
    }

    /// What sort of street the arm is.
    pub fn class(&self) -> Option<StreetClass> {
        self.section.as_ref().map(|s| s.class)
    }

    /// Whether the street the arm reads is a pedestrian street: no lanes, no crossing, nothing to cross.
    pub fn is_pedestrian(&self) -> bool {
        self.class() == Some(StreetClass::Pedestrian)
    }
}

fn read_profile(view: &View, name: Said) -> Profile {
    let pieces: Vec<Piece> =
        view.segments.iter().map(|s| Piece { kind: s.kind, material: s.material, x_mm: s.x_mm, width_mm: s.width_mm, direction: s.direction }).collect();
    let road: Vec<&Piece> = pieces.iter().filter(|p| is_roadway(p.kind)).collect();
    let road_l = road.first().map_or(0, |p| p.x_mm);
    let road_r = road.last().map_or(view.row_mm, |p| p.x_mm + p.width_mm);
    let lanes = |dir: &str| {
        let mut v: Vec<i32> = pieces.iter().filter(|p| KINDS[p.kind].id == "travel" && p.direction == Some(dir)).map(|p| p.x_mm + p.width_mm / 2).collect();
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
    let beyond_l = pieces.iter().filter(|p| p.x_mm + p.width_mm <= road_l).max_by_key(|p| p.x_mm).map(|p| p.kind);
    let beyond_r = pieces.iter().filter(|p| p.x_mm >= road_r).min_by_key(|p| p.x_mm).map(|p| p.kind);
    let (enter_x, leave_x, enter_span, leave_span) = (lanes("toward"), lanes("away"), span("toward"), span("away"));
    Profile {
        name,
        row_mm: view.row_mm,
        park: [park(road.first()), park(road.last())],
        edge_kind: [beyond_l, beyond_r],
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
    /// The bike lane beside the left and the right curb (indexes into `pieces`), if there is one: the outermost lane
    /// that is not parked on, so a bike lane outside parking counts and one between travel lanes does not.
    pub fn outer_bike_sides(&self) -> [Option<usize>; 2] {
        let moving = |p: &Piece| is_roadway(p.kind) && !matches!(KINDS[p.kind].id, "parking" | "loading");
        let bike = |i: Option<usize>| i.filter(|&i| KINDS[self.pieces[i].kind].id == "bike");
        [bike(self.pieces.iter().position(moving)), bike(self.pieces.iter().rposition(moving))]
    }

    /// The bike lane that touches the left and the right curb (indexes into `pieces`): the first or last piece of the
    /// carriageway itself, so a bike lane outside a parking lane is not one.
    pub fn curb_bike_sides(&self) -> [Option<usize>; 2] {
        let bike = |i: Option<usize>| i.filter(|&i| KINDS[self.pieces[i].kind].id == "bike");
        [bike(self.pieces.iter().position(|p| is_roadway(p.kind))), bike(self.pieces.iter().rposition(|p| is_roadway(p.kind)))]
    }

    /// The indexes into `pieces` of the bike lanes beside the carriageway's edges.
    pub fn outer_bike(&self) -> Vec<usize> {
        let [left, right] = self.outer_bike_sides();
        let mut v: Vec<usize> = left.into_iter().chain(right).collect();
        v.dedup();
        v
    }

    pub fn road_mm(&self) -> i32 {
        self.road_r - self.road_l
    }
}

/// Why a street cannot have a bike box. The one rule the edit, the settling of a state, the view and the details
/// panel all ask.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BikeBoxBlocker {
    Roundabout,
    NoLane,
    NoEntry,
    Free,
}

impl BikeBoxBlocker {
    /// The message that says it, as the name of the switch that cannot be pressed.
    pub fn key(self) -> &'static str {
        match self {
            BikeBoxBlocker::Roundabout => "jn-insp-bike-box-roundabout",
            BikeBoxBlocker::NoLane => "jn-insp-bike-box-no-lane",
            BikeBoxBlocker::NoEntry => "jn-insp-bike-box-no-entry",
            BikeBoxBlocker::Free => "jn-insp-bike-box-free",
        }
    }
}

/// Why a corner cannot be protected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtectedBlocker {
    Roundabout,
    Straight,
    NoLane,
    NoRoom,
}

/// How deep a corner island is, at most.
pub const PROTECTED_ISLAND_MM: i32 = 2_000;
/// The least a corner island is worth drawing.
pub const MIN_PROTECTED_ISLAND_MM: i32 = 500;
/// What a street keeps clear between the corner islands on its two sides: a lane.
pub const PROTECTED_CLEAR_MM: i32 = 3_000;

/// How deep the corner island of the corner between a street reading `pa` and the next reading `pb` can be: the track
/// and the island take their width from each curb of a street, and a lane must stay clear between the two sides.
pub fn protected_island_mm(pa: &Profile, pb: &Profile) -> i32 {
    let lane = |p: &Profile| p.curb_bike_sides().into_iter().flatten().map(|i| p.pieces[i].width_mm).max().unwrap_or(0);
    let room = |p: &Profile| (p.road_mm() - PROTECTED_CLEAR_MM) / 2 - lane(p);
    room(pa).min(room(pb)).min(PROTECTED_ISLAND_MM)
}

impl ProtectedBlocker {
    /// The message that says it, as the name of the switch that cannot be pressed.
    pub fn key(self) -> &'static str {
        match self {
            ProtectedBlocker::Roundabout => "jn-insp-protected-roundabout",
            ProtectedBlocker::Straight => "jn-insp-protected-straight",
            ProtectedBlocker::NoLane => "jn-insp-protected-no-lane",
            ProtectedBlocker::NoRoom => "jn-insp-protected-no-room",
        }
    }
}

/// What stops the corner between arm `i` (street `pa`) and the next arm (street `pb`) from being protected: it needs a
/// real corner (a 180 degree gap has none to follow), a bike lane at the right curb of arm `i` and one at the left curb
/// of the next arm (the two curbs the corner joins), room for the island, and a junction that is not a roundabout.
pub fn protected_blocker(s: &State, i: usize, pa: &Profile, pb: &Profile) -> Option<ProtectedBlocker> {
    let next = (i + 1) % s.arms.len();
    if s.control == ROUNDABOUT {
        Some(ProtectedBlocker::Roundabout)
    } else if gap(s.arms[i].bearing, s.arms[next].bearing) == 180 {
        Some(ProtectedBlocker::Straight)
    } else if pa.curb_bike_sides()[1].is_none() || pb.curb_bike_sides()[0].is_none() {
        Some(ProtectedBlocker::NoLane)
    } else if protected_island_mm(pa, pb) < MIN_PROTECTED_ISLAND_MM {
        Some(ProtectedBlocker::NoRoom)
    } else {
        None
    }
}

/// What stops arm `i` of `s`, whose street reads as `p`, from having a bike box: it needs a bike lane to reach it, a
/// lane cars enter by, a stop line (a free approach has none), and a junction that is not a roundabout.
pub fn bike_box_blocker(s: &State, i: usize, p: &Profile) -> Option<BikeBoxBlocker> {
    // The junction runs uncontrolled with at most one street of traffic, whatever control is kept.
    let s = &*in_force(s);
    if s.control == ROUNDABOUT {
        Some(BikeBoxBlocker::Roundabout)
    } else if p.outer_bike().is_empty() {
        Some(BikeBoxBlocker::NoLane)
    } else if p.enter_x.is_empty() {
        Some(BikeBoxBlocker::NoEntry)
    } else if crate::junction::read_model::control_role(s, i) == "free" {
        Some(BikeBoxBlocker::Free)
    } else {
        None
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

/// The crossing a new arm has, and an arm that regains traffic.
fn default_crossing() -> Crossing {
    Crossing { setback_mm: DEFAULT_SETBACK_MM, width_mm: DEFAULT_CROSSING_MM, island: false, continuous: false }
}

/// The classes of turn an arm has, given the other arms' bearings (a pedestrian arm takes none).
pub fn classes_of(arms: &[Arm], i: usize) -> u8 {
    arms.iter().enumerate().filter(|(j, b)| *j != i && !b.is_pedestrian()).fold(0, |m, (_, b)| m | turn_class(arms[i].bearing, b.bearing))
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

/// The point of the compass a bearing is nearest, named in a sentence ("north-east").
pub fn compass(bearing: i32) -> Said {
    const KEYS: [&str; 8] =
        ["jn-compass-n", "jn-compass-ne", "jn-compass-e", "jn-compass-se", "jn-compass-s", "jn-compass-sw", "jn-compass-w", "jn-compass-nw"];
    Said::new(KEYS[(((bearing + 22).rem_euclid(360)) / 45) as usize])
}

/// An arm by its street and the way it leaves the junction: "Main Street (north)".
pub fn arm_name(a: &Arm) -> Said {
    Said::new("jn-arm-name").with("street", Arg::Said(Box::new(a.street_name()))).with("compass", Arg::Said(Box::new(compass(a.bearing))))
}

/// Something said about one arm.
fn about(key: &'static str, a: &Arm) -> Said {
    Said::new(key).with("arm", Arg::Said(Box::new(arm_name(a))))
}

/// The word for switching a feature on or off, said in the middle of a label.
fn change(on: bool) -> Arg {
    Arg::Msg(if on { "jn-word-add" } else { "jn-word-remove" })
}

/// Why traffic may not turn from arm `i` into arm `j`, when a measure stops
/// it: a dead end, a transit modal filter, right-in/right-out, or a left turn
/// sent round another way.
pub fn blocked(arms: &[Arm], i: usize, j: usize) -> Option<Said> {
    let (a, b) = (&arms[i], &arms[j]);
    let class = turn_class(a.bearing, b.bearing);
    let key = if a.rule == RULE_DEAD_END || b.rule == RULE_DEAD_END {
        "jn-blocked-dead-end"
    } else if a.filter || b.filter {
        "jn-blocked-filter"
    } else if a.rule == RULE_RIRO && class != RIGHT {
        "jn-blocked-riro-leave"
    } else if b.rule == RULE_RIRO && class != RIGHT {
        "jn-blocked-riro-enter"
    } else if a.rule == RULE_AROUND && class == LEFT {
        "jn-blocked-around"
    } else {
        return None;
    };
    Some(Said::new(key))
}

// ---- the editor ---------------------------------------------------------------

/// Why an edit was refused; the view-model words it for the resident.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    NeedsThreeStreets,
    LinkedNoRemove,
    RoundaboutTooBig,
    BearingBlocked,
    LastWayOut,
    LaneNeedsStreet,
    IslandRoadTooNarrow,
    BulbNoParking,
    DoesNotFit,
    RaisedOnRoundabout,
    BikeBoxNotHere,
    ProtectedNotHere,
    CrossingOnPedestrian,
    NotSignal,
    TimingOutOfRange,
    NoTransitLane,
    MeasureOnPedestrian,
    TurboArmSkewed,
    TurboEntryTooNarrow,
    TurboLanesFixed,
    BusOnTurbo,
    QueueJumpOnTurbo,
    PhaseServesNothing,
    LastPhase,
    TooManyPhases,
    PhaseAtEnd,
    PlanSuggested,
    PatternNotFit,
    PatternAlready,
}

impl Refusal {
    pub const ALL: [Refusal; 29] = [
        Refusal::NeedsThreeStreets,
        Refusal::LinkedNoRemove,
        Refusal::RoundaboutTooBig,
        Refusal::BearingBlocked,
        Refusal::LastWayOut,
        Refusal::LaneNeedsStreet,
        Refusal::IslandRoadTooNarrow,
        Refusal::BulbNoParking,
        Refusal::DoesNotFit,
        Refusal::RaisedOnRoundabout,
        Refusal::BikeBoxNotHere,
        Refusal::ProtectedNotHere,
        Refusal::CrossingOnPedestrian,
        Refusal::NotSignal,
        Refusal::TimingOutOfRange,
        Refusal::NoTransitLane,
        Refusal::MeasureOnPedestrian,
        Refusal::TurboArmSkewed,
        Refusal::TurboEntryTooNarrow,
        Refusal::TurboLanesFixed,
        Refusal::BusOnTurbo,
        Refusal::QueueJumpOnTurbo,
        Refusal::PhaseServesNothing,
        Refusal::LastPhase,
        Refusal::TooManyPhases,
        Refusal::PhaseAtEnd,
        Refusal::PlanSuggested,
        Refusal::PatternNotFit,
        Refusal::PatternAlready,
    ];

    /// The message that says why.
    pub fn key(self) -> &'static str {
        match self {
            Refusal::NeedsThreeStreets => "jn-refusal-needs-three-streets",
            Refusal::LinkedNoRemove => "jn-refusal-linked-no-remove",
            Refusal::RoundaboutTooBig => "jn-refusal-roundabout-too-big",
            Refusal::BearingBlocked => "jn-refusal-bearing-blocked",
            Refusal::LastWayOut => "jn-refusal-last-way-out",
            Refusal::LaneNeedsStreet => "jn-refusal-lane-needs-street",
            Refusal::IslandRoadTooNarrow => "jn-refusal-island-road-too-narrow",
            Refusal::BulbNoParking => "jn-refusal-bulb-no-parking",
            Refusal::DoesNotFit => "jn-refusal-does-not-fit",
            Refusal::RaisedOnRoundabout => "jn-refusal-raised-roundabout",
            Refusal::BikeBoxNotHere => "jn-refusal-bike-box-not-here",
            Refusal::NotSignal => "jn-refusal-not-signal",
            Refusal::TimingOutOfRange => "jn-refusal-timing-out-of-range",
            Refusal::NoTransitLane => "jn-refusal-no-transit-lane",
            Refusal::ProtectedNotHere => "jn-refusal-protected-not-here",
            Refusal::CrossingOnPedestrian => "jn-refusal-crossing-pedestrian",
            Refusal::MeasureOnPedestrian => "jn-refusal-measure-pedestrian",
            Refusal::TurboArmSkewed => "jn-refusal-turbo-skewed",
            Refusal::TurboEntryTooNarrow => "jn-refusal-turbo-entry-too-narrow",
            Refusal::TurboLanesFixed => "jn-refusal-turbo-lanes-fixed",
            Refusal::BusOnTurbo => "jn-refusal-bus-on-turbo",
            Refusal::QueueJumpOnTurbo => "jn-refusal-queue-jump-on-turbo",
            Refusal::PhaseServesNothing => "jn-refusal-phase-serves-nothing",
            Refusal::LastPhase => "jn-refusal-last-phase",
            Refusal::TooManyPhases => "jn-refusal-too-many-phases",
            Refusal::PhaseAtEnd => "jn-refusal-phase-at-end",
            Refusal::PlanSuggested => "jn-refusal-plan-suggested",
            Refusal::PatternNotFit => "jn-refusal-pattern-not-fit",
            Refusal::PatternAlready => "jn-refusal-pattern-already",
        }
    }
}

pub struct Junction {
    /// Set when the junction is a place in a city: its streets are the city's
    /// and its name is the city's, so its streets cannot be removed.
    linked: bool,
    name: Said,
    /// Index into `REGIONS`. A setting of the sheet, not part of the history.
    pub region: usize,
    states: Vec<State>,
    cursor: usize,
    pub selected: Target,
    gesture: Option<State>,
    pending_label: Said,
    /// Why the last edit was refused; None once one is taken.
    refusal: Option<Refusal>,
    /// The arm a `TurboArmSkewed` or `TurboEntryTooNarrow` refusal names; None for any other.
    refusal_arm: Option<Said>,
}

fn snap(v: i32, step: i32) -> i32 {
    ((v as f64 / step as f64).round() as i32) * step
}

impl Junction {
    /// A junction that is a place in a city. `today` is how it was first laid
    /// out and `now` how it stands, each with its arms' streets attached. When
    /// the streets have changed so that `now` can no longer be drawn it is
    /// eased (small corners, no offsets, no roundabout) before it is given up
    /// for `today`. None when not even `today` can be drawn.
    pub fn from_city(name: Said, today: &State, now: &State, region: usize) -> Option<Junction> {
        let settle = |s: &State| {
            let mut s = s.clone();
            normalize(&mut s.arms, region);
            settle_table_and_boxes(&mut s, region);
            s
        };
        let today = ease(&settle(today), region)?;
        let now = ease(&settle(now), region);
        let mut j = Junction {
            linked: true,
            name,
            region,
            states: vec![State { label: self::today(), ..today.clone() }],
            cursor: 0,
            selected: Target::None,
            gesture: None,
            pending_label: earlier(),
            refusal: None,
            refusal_arm: None,
        };
        let same = |a: &State, b: &State| State { label: unlabelled(), ..a.clone() } == State { label: unlabelled(), ..b.clone() };
        if let Some(now) = now.filter(|n| !same(n, &today)) {
            j.states.push(State { label: earlier(), ..now });
            j.cursor = 1;
        }
        Some(j)
    }

    /// Whether `other` is drawn as this is: the same state, the arms' street sections included, whatever it is
    /// labelled.
    pub fn drawn_as(&self, other: &Junction) -> bool {
        let bare = |j: &Junction| State { label: unlabelled(), ..j.current().clone() };
        bare(self) == bare(other)
    }

    /// The junction as it is now, without its streets, for the city to keep.
    pub fn snapshot(&self) -> State {
        let mut s = self.current().clone();
        for a in &mut s.arms {
            a.section = None;
            a.beyond = None;
        }
        s
    }

    pub fn name(&self) -> &Said {
        &self.name
    }

    pub fn is_linked(&self) -> bool {
        self.linked
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

    pub fn revisions(&self) -> impl Iterator<Item = &Said> {
        self.states.iter().skip(1).take(self.cursor).map(|s| &s.label)
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
            settle_table_and_boxes(s, r);
        }
        true
    }

    // ---- history ----------------------------------------------------------

    /// Runs an edit on a copy and records it as a revision, or, inside a
    /// gesture, applies it without recording. Rejects an edit that leaves the
    /// junction invalid.
    fn edit<F>(&mut self, label: Said, f: F) -> bool
    where
        F: FnOnce(&mut State) -> bool,
    {
        self.edit_why(Refusal::DoesNotFit, label, f)
    }

    /// Like `edit`, with the reason to give when the result cannot be drawn.
    fn edit_why<F>(&mut self, why: Refusal, label: Said, f: F) -> bool
    where
        F: FnOnce(&mut State) -> bool,
    {
        let region = self.region;
        let mut next = self.current().clone();
        if !f(&mut next) {
            return self.refuse(Refusal::DoesNotFit);
        }
        next.arms.sort_by_key(|a| a.bearing);
        // A turbo wants its arms near its template and room in each street for the lanes it widens the entry to: an edit that
        // takes an arm past the skew limit, or leaves an entry that would pass its street's property line, is refused, naming the arm.
        if in_force(&next).control == ROUNDABOUT
            && let Some(t) = next.turbo.as_ref()
        {
            let skewed = t.worst_skew(&next.arms).filter(|(_, deg)| *deg > turbo::MAX_SKEW_DEG).map(|(uid, _)| (Refusal::TurboArmSkewed, uid));
            let narrow = || t.entry_too_narrow(&next.arms, region).map(|uid| (Refusal::TurboEntryTooNarrow, uid));
            if let Some((why, uid)) = skewed.or_else(narrow) {
                self.refusal = Some(why);
                self.refusal_arm = next.arms.iter().find(|a| a.uid == uid).map(arm_name);
                return false;
            }
        }
        normalize(&mut next.arms, region);
        if next.control != ROUNDABOUT {
            next.cycle = None;
        }
        settle_table_and_boxes(&mut next, region);
        // An edit that takes the person's whole signal plan away says so (restoring the suggestion does it on purpose).
        let label = if self.current().signal_plan.is_some() && next.signal_plan.is_none() && label.key != "jn-rev-phase-restore" {
            Said::new("jn-rev-plan-lost").with("change", Arg::Said(Box::new(label)))
        } else {
            label
        };
        // A bus lane needs a roundabout and both its streets.
        if let Some((a, b)) = next.bus
            && (next.control != ROUNDABOUT || !next.arms.iter().any(|x| x.uid == a) || !next.arms.iter().any(|x| x.uid == b))
        {
            next.bus = None;
        }
        if !valid(&next, region) {
            return self.refuse(why);
        }
        if next == *self.current() {
            return self.refuse(Refusal::DoesNotFit);
        }
        self.refusal = None;
        self.refusal_arm = None;
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
        self.refusal_arm = None;
        false
    }

    /// Why the last edit was refused, if the last one was.
    pub fn refusal(&self) -> Option<Refusal> {
        self.refusal
    }

    /// The arm a refusal names (a turbo arm that would be too far off square, or whose entry would be widened past its
    /// property line), if it names one.
    pub fn refusal_arm(&self) -> Option<&Said> {
        self.refusal_arm.as_ref()
    }

    pub fn begin_gesture(&mut self) {
        if self.gesture.is_none() {
            self.gesture = Some(self.current().clone());
            self.pending_label = earlier();
        }
    }

    pub fn end_gesture(&mut self) -> bool {
        let Some(baseline) = self.gesture.take() else { return false };
        let mut now = self.current().clone();
        if now == baseline {
            return false;
        }
        *self.current_mut() = baseline;
        now.label = std::mem::replace(&mut self.pending_label, earlier());
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
        self.states.truncate(1);
        self.cursor = 0;
        self.selected = Target::None;
        self.gesture = None;
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

    fn arm_edit<F>(&mut self, uid: u32, label: impl FnOnce(&Arm) -> Said, f: F) -> bool
    where
        F: FnOnce(&mut Arm) -> bool,
    {
        self.arm_edit_why(Refusal::DoesNotFit, uid, label, f)
    }

    fn arm_edit_why<F>(&mut self, why: Refusal, uid: u32, label: impl FnOnce(&Arm) -> Said, f: F) -> bool
    where
        F: FnOnce(&mut Arm) -> bool,
    {
        let Some(a) = self.arm(uid) else { return self.refuse(Refusal::DoesNotFit) };
        let label = label(a);
        self.edit_why(why, label, |s| s.arms.iter_mut().find(|a| a.uid == uid).is_some_and(f))
    }

    pub fn remove_arm(&mut self, uid: u32) -> bool {
        let Some(a) = self.arm(uid) else { return self.refuse(Refusal::DoesNotFit) };
        if self.linked {
            return self.refuse(Refusal::LinkedNoRemove);
        }
        if self.current().arms.len() <= MIN_ARMS {
            return self.refuse(Refusal::NeedsThreeStreets);
        }
        let label = about("jn-rev-remove", a);
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
        self.arm_edit_why(
            Refusal::BearingBlocked,
            uid,
            |a| Said::new("jn-rev-bearing").with("street", Arg::Said(Box::new(a.street_name()))).with("degrees", Arg::Num(b.into())),
            |a| {
                a.bearing = b;
                true
            },
        )
    }

    pub fn set_offset(&mut self, uid: u32, mm: i32) -> bool {
        let mm = snap(mm, OFFSET_STEP_MM);
        let region = self.region;
        self.arm_edit(
            uid,
            |a| about("jn-rev-offset", a).with("length", Arg::Length(mm)),
            |a| {
                let half = a.profile(region).road_mm() / 2;
                if mm.abs() > half {
                    return false;
                }
                a.offset_mm = mm;
                true
            },
        )
    }

    /// Sets the curb radius at the corner clockwise of an arm.
    pub fn set_corner(&mut self, uid: u32, mm: i32) -> bool {
        let mm = snap(mm, RING_STEP_MM);
        if !(MIN_CORNER_MM..=MAX_CORNER_MM).contains(&mm) {
            return self.refuse(Refusal::DoesNotFit);
        }
        self.arm_edit(
            uid,
            |a| about("jn-rev-corner", a).with("length", Arg::Length(mm)),
            |a| {
                a.corner_mm = mm;
                true
            },
        )
    }

    pub fn set_crossing(&mut self, uid: u32, on: bool) -> bool {
        if on && self.arm(uid).is_some_and(Arm::is_pedestrian) {
            return self.refuse(Refusal::CrossingOnPedestrian);
        }
        self.arm_edit(
            uid,
            |a| about("jn-rev-crossing", a).with("change", change(on)),
            |a| {
                a.crossing = on.then(default_crossing);
                true
            },
        )
    }

    pub fn set_setback(&mut self, uid: u32, mm: i32) -> bool {
        let mm = snap(mm, RING_STEP_MM);
        if !(MIN_SETBACK_MM..=MAX_SETBACK_MM).contains(&mm) {
            return self.refuse(Refusal::DoesNotFit);
        }
        self.arm_edit(uid, |a| about("jn-rev-setback", a).with("length", Arg::Length(mm)), |a| a.crossing.as_mut().map(|c| c.setback_mm = mm).is_some())
    }

    pub fn set_crossing_width(&mut self, uid: u32, mm: i32) -> bool {
        let mm = snap(mm, RING_STEP_MM);
        if !(MIN_CROSSING_MM..=MAX_CROSSING_MM).contains(&mm) {
            return self.refuse(Refusal::DoesNotFit);
        }
        self.arm_edit(uid, |a| about("jn-rev-crossing-width", a).with("length", Arg::Length(mm)), |a| a.crossing.as_mut().map(|c| c.width_mm = mm).is_some())
    }

    pub fn set_island(&mut self, uid: u32, on: bool) -> bool {
        let region = self.region;
        if on && self.arm(uid).is_some_and(|a| a.profile(region).road_mm() < ISLAND_MIN_ROAD_MM) {
            return self.refuse(Refusal::IslandRoadTooNarrow);
        }
        self.arm_edit(
            uid,
            |a| about("jn-rev-island", a).with("change", change(on)),
            |a| {
                // A person on a continuous sidewalk never leaves it, so there is nothing to refuge from.
                a.crossing.as_mut().is_some_and(|c| {
                    if on && c.continuous {
                        return false;
                    }
                    c.island = on;
                    true
                })
            },
        )
    }

    /// Makes the street's crossing a continuous sidewalk, or a plain one again.
    pub fn set_continuous(&mut self, uid: u32, on: bool) -> bool {
        self.arm_edit(
            uid,
            |a| {
                // Taking the refuge island away is said in the same breath, or a screen reader would never hear of it.
                let island_goes = on && a.crossing.is_some_and(|c| c.island);
                about(if island_goes { "jn-rev-continuous-island" } else { "jn-rev-continuous" }, a).with("change", change(on))
            },
            |a| {
                a.crossing.as_mut().is_some_and(|c| {
                    c.continuous = on;
                    c.island = c.island && !on;
                    true
                })
            },
        )
    }

    /// Puts a bike box ahead of the cars' stop line, or takes it away. It needs a bike lane to reach it, a street cars
    /// enter and stop on (not a free approach), and a junction that is not a roundabout.
    pub fn set_bike_box(&mut self, uid: u32, on: bool) -> bool {
        let region = self.region;
        let Some(at) = self.current().arms.iter().position(|a| a.uid == uid) else { return self.refuse(Refusal::DoesNotFit) };
        if on {
            let s = self.current();
            if bike_box_blocker(s, at, &s.arms[at].profile(region)).is_some() {
                return self.refuse(Refusal::BikeBoxNotHere);
            }
        }
        self.arm_edit(
            uid,
            |a| about("jn-rev-bike-box", a).with("change", change(on)),
            |a| {
                a.bike_box = on;
                true
            },
        )
    }

    /// Sets one number of the signal timing of arm `uid`, in whole seconds. The control has to be a signal, an early
    /// green needs a transit lane on the street, and the value has to be within `field.range()`. A value the arm
    /// already has is not an edit.
    pub fn set_timing(&mut self, uid: u32, field: TimingField, seconds: i32) -> bool {
        let Some(arm) = self.arm(uid) else { return self.refuse(Refusal::DoesNotFit) };
        if arm.is_pedestrian() {
            return self.refuse(Refusal::MeasureOnPedestrian);
        }
        let (bus_lane, timing) = (arm.bus_lane, arm.timing);
        if in_force(self.current()).control != SIGNAL {
            return self.refuse(Refusal::NotSignal);
        }
        if field == TimingField::TransitLead && !bus_lane {
            return self.refuse(Refusal::NoTransitLane);
        }
        let (lo, hi) = field.range();
        if !(lo..=hi).contains(&seconds) {
            return self.refuse(Refusal::TimingOutOfRange);
        }
        let mut t = timing.unwrap_or_default();
        if field.get(&t) == seconds as u32 {
            return true;
        }
        field.put(&mut t, seconds as u32);
        self.arm_edit(
            uid,
            |a| about("jn-rev-timing", a).with("field", Arg::MsgLower(field.key())).with("seconds", Arg::Num(i64::from(seconds))),
            |a| {
                a.timing = Some(t);
                true
            },
        )
    }

    /// Steps one number of the signal timing of arm `uid` by a second, up or down.
    pub fn step_timing(&mut self, uid: u32, field: TimingField, dir: i32) -> bool {
        let Some(arm) = self.arm(uid) else { return self.refuse(Refusal::DoesNotFit) };
        let now = field.get(&arm.timing.unwrap_or_default()) as i32;
        self.set_timing(uid, field, now.saturating_add(dir.signum()))
    }

    /// The signal plan as the person sees it, if the control in force is a signal, and the control has to be one for
    /// an edit of it.
    fn signal_plan_to_edit(&mut self) -> Option<signal::Plan> {
        if in_force(self.current()).control != SIGNAL {
            self.refuse(Refusal::NotSignal);
            return None;
        }
        let plan = signal::plan(self.current(), self.region);
        if plan.is_none() {
            self.refuse(Refusal::NotSignal);
        }
        plan
    }

    /// Adds a phase for the traffic entering from arm `uid`, last in the cycle: all its movements, with the crossings
    /// and bike movements the suggestion runs with it. The first edit of a plan keeps the suggestion as the person's
    /// own plan, then changes it.
    pub fn add_phase(&mut self, uid: u32) -> bool {
        let Some(arm) = self.arm(uid) else { return self.refuse(Refusal::DoesNotFit) };
        if arm.is_pedestrian() {
            return self.refuse(Refusal::MeasureOnPedestrian);
        }
        let Some(plan) = self.signal_plan_to_edit() else { return false };
        let Some(phase) = signal::phase_for(self.current(), self.region, uid) else { return self.refuse(Refusal::PhaseServesNothing) };
        if plan.phases.len() >= MAX_ARMS * 2 {
            return self.refuse(Refusal::TooManyPhases);
        }
        let mut kept = plan.to_stored();
        kept.phases.push(phase);
        let label = self.phase_label("jn-rev-phase-add", uid, kept.phases.len() - 1);
        self.edit(label, |s| {
            s.signal_plan = Some(kept);
            true
        })
    }

    /// Takes out the phase at `index` (from 0, in the order they run). The last phase stays.
    pub fn remove_phase(&mut self, index: usize) -> bool {
        let Some(plan) = self.signal_plan_to_edit() else { return false };
        let Some(phase) = plan.phases.get(index) else { return self.refuse(Refusal::DoesNotFit) };
        if plan.phases.len() <= 1 {
            return self.refuse(Refusal::LastPhase);
        }
        let label = self.phase_label("jn-rev-phase-remove", phase.uid, index);
        let mut kept = plan.to_stored();
        kept.phases.remove(index);
        self.edit(label, |s| {
            s.signal_plan = Some(kept);
            true
        })
    }

    /// Moves the phase at `index` one place earlier (`dir` < 0) or later in the cycle.
    pub fn move_phase(&mut self, index: usize, dir: i32) -> bool {
        let Some(plan) = self.signal_plan_to_edit() else { return false };
        let Some(phase) = plan.phases.get(index) else { return self.refuse(Refusal::DoesNotFit) };
        let to = if dir < 0 { index.checked_sub(1) } else { Some(index + 1).filter(|&t| t < plan.phases.len()) };
        let Some(to) = to else { return self.refuse(Refusal::PhaseAtEnd) };
        let key = if dir < 0 { "jn-rev-phase-earlier" } else { "jn-rev-phase-later" };
        let label = self.phase_label(key, phase.uid, to).with("total", Arg::Num(plan.phases.len() as i64));
        let mut kept = plan.to_stored();
        kept.phases.swap(index, to);
        self.edit(label, |s| {
            s.signal_plan = Some(kept);
            true
        })
    }

    /// Makes the phase at `index` (from 0, in the order they run) the ready-made `pattern`, keeping its street and so
    /// its timing. Refused when the pattern does not fit the junction's lanes (or is Custom, which is shown and never
    /// chosen) and when the phase is that pattern already.
    pub fn set_phase_pattern(&mut self, index: usize, pattern: Pattern) -> bool {
        let Some(plan) = self.signal_plan_to_edit() else { return false };
        let Some(phase) = plan.phases.get(index) else { return self.refuse(Refusal::DoesNotFit) };
        let Some(made) = signal::pattern_phase(self.current(), self.region, phase.uid, pattern) else { return self.refuse(Refusal::PatternNotFit) };
        if signal::pattern_of(self.current(), self.region, phase) == pattern {
            return self.refuse(Refusal::PatternAlready);
        }
        let label = self.phase_label("jn-rev-phase-pattern", phase.uid, index).with("pattern", Arg::MsgLower(pattern.name_key()));
        let mut kept = plan.to_stored();
        kept.phases[index] = made;
        self.edit(label, |s| {
            s.signal_plan = Some(kept);
            true
        })
    }

    /// Gives the junction the suggested plan again, in place of the one the person edited.
    pub fn restore_suggestion(&mut self) -> bool {
        if self.signal_plan_to_edit().is_none() {
            return false;
        }
        if self.current().signal_plan.is_none() {
            return self.refuse(Refusal::PlanSuggested);
        }
        self.edit(Said::new("jn-rev-phase-restore"), |s| {
            s.signal_plan = None;
            true
        })
    }

    /// A label about the phase of arm `uid` that is now place `index` (from 0) in the cycle.
    fn phase_label(&self, key: &'static str, uid: u32, index: usize) -> Said {
        let arm = self.arm(uid).map_or_else(|| named_said(""), arm_name);
        Said::new(key).with("arm", Arg::Said(Box::new(arm))).with("n", Arg::Num(index as i64 + 1))
    }

    /// Protects the corner clockwise of arm `uid`, or makes it plain again. See `protected_blocker` for where it may stand.
    pub fn set_protected(&mut self, uid: u32, on: bool) -> bool {
        let region = self.region;
        let Some(at) = self.current().arms.iter().position(|a| a.uid == uid) else { return self.refuse(Refusal::DoesNotFit) };
        if on {
            let s = self.current();
            let next = (at + 1) % s.arms.len();
            if protected_blocker(s, at, &s.arms[at].profile(region), &s.arms[next].profile(region)).is_some() {
                return self.refuse(Refusal::ProtectedNotHere);
            }
        }
        self.arm_edit(
            uid,
            |a| about("jn-rev-protected", a).with("change", change(on)),
            |a| {
                a.protected = on;
                true
            },
        )
    }

    /// Side 0 is the arm's left curb, 1 its right.
    pub fn set_bulb(&mut self, uid: u32, side: usize, on: bool) -> bool {
        let region = self.region;
        if side > 1 {
            return self.refuse(Refusal::DoesNotFit);
        }
        let key = if side == 0 { "jn-rev-bulb-left" } else { "jn-rev-bulb-right" };
        if on && self.arm(uid).is_some_and(|a| a.profile(region).park[side] == 0) {
            return self.refuse(Refusal::BulbNoParking);
        }
        self.arm_edit(
            uid,
            |a| about(key, a).with("change", change(on)),
            |a| {
                a.bulb[side] = on;
                true
            },
        )
    }

    /// Lets a lane go to a street, or not. A lane keeps at least one street.
    pub fn set_lane_dest(&mut self, uid: u32, lane: usize, to: u32, on: bool) -> bool {
        if self.turbo_in_force() {
            return self.refuse(Refusal::TurboLanesFixed);
        }
        let Some(dest) = self.arm(to).map(arm_name) else { return self.refuse(Refusal::DoesNotFit) };
        if uid == to {
            return self.refuse(Refusal::DoesNotFit);
        }
        if self.arm(to).is_some_and(Arm::is_pedestrian) {
            return self.refuse(Refusal::DoesNotFit);
        }
        if !on && self.arm(uid).and_then(|a| a.lanes.get(lane)).is_some_and(|l| l.to == [to]) {
            return self.refuse(Refusal::LaneNeedsStreet);
        }
        self.arm_edit(
            uid,
            |a| {
                about(if on { "jn-rev-lane-to" } else { "jn-rev-lane-not-to" }, a)
                    .with("lane", Arg::Num(lane as i64 + 1))
                    .with("dest", Arg::Said(Box::new(dest)))
            },
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
        if self.arm(uid).is_some_and(Arm::is_pedestrian) {
            return self.refuse(Refusal::MeasureOnPedestrian);
        }
        self.arm_edit(
            uid,
            |a| about("jn-rev-bus-lane", a).with("change", change(on)),
            |a| {
                a.bus_lane = on;
                true
            },
        )
    }

    /// Sets the approach measure, by index into `APPROACHES`.
    pub fn set_approach(&mut self, uid: u32, approach: usize) -> bool {
        if self.arm(uid).is_some_and(Arm::is_pedestrian) {
            return self.refuse(Refusal::MeasureOnPedestrian);
        }
        if approach >= APPROACHES.len() {
            return self.refuse(Refusal::DoesNotFit);
        }
        if matches!(approach, Q_OFFSET | Q_CURB | Q_VIRTUAL) && self.turbo_in_force() {
            return self.refuse(Refusal::QueueJumpOnTurbo);
        }
        self.arm_edit(
            uid,
            |a| measure(a, &APPROACHES[approach], "jn-rev-no-approach"),
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
        self.arm_edit(
            uid,
            |a| about("jn-rev-approach-len", a).with("length", Arg::Length(mm)),
            |a| {
                a.approach_mm = mm;
                true
            },
        )
    }

    /// Sets the bus stop, by index into `STOPS`.
    pub fn set_stop(&mut self, uid: u32, stop: usize) -> bool {
        if self.arm(uid).is_some_and(Arm::is_pedestrian) {
            return self.refuse(Refusal::MeasureOnPedestrian);
        }
        if stop >= STOPS.len() {
            return self.refuse(Refusal::DoesNotFit);
        }
        self.arm_edit(
            uid,
            |a| measure(a, &STOPS[stop], "jn-rev-no-stop"),
            |a| {
                a.stop = stop;
                true
            },
        )
    }

    /// Sets the turn management, by index into `RULES`.
    pub fn set_rule(&mut self, uid: u32, rule: usize) -> bool {
        if self.arm(uid).is_some_and(Arm::is_pedestrian) {
            return self.refuse(Refusal::MeasureOnPedestrian);
        }
        if rule >= RULES.len() {
            return self.refuse(Refusal::DoesNotFit);
        }
        self.arm_edit(
            uid,
            |a| measure(a, &RULES[rule], "jn-rev-no-rule"),
            |a| {
                a.rule = rule;
                true
            },
        )
    }

    pub fn set_filter(&mut self, uid: u32, on: bool) -> bool {
        if self.arm(uid).is_some_and(Arm::is_pedestrian) {
            return self.refuse(Refusal::MeasureOnPedestrian);
        }
        self.arm_edit(
            uid,
            |a| about("jn-rev-filter", a).with("change", change(on)),
            |a| {
                a.filter = on;
                true
            },
        )
    }

    pub fn set_turn(&mut self, from: u32, to: u32, allowed: bool) -> bool {
        let (Some(a), Some(b)) = (self.arm(from), self.arm(to)) else { return self.refuse(Refusal::DoesNotFit) };
        if from == to {
            return self.refuse(Refusal::DoesNotFit);
        }
        let label = Said::new(if allowed { "jn-rev-turn-allow" } else { "jn-rev-turn-ban" })
            .with("from", Arg::Said(Box::new(arm_name(a))))
            .with("to", Arg::Said(Box::new(arm_name(b))));
        let others = self.current().arms.len() - 1;
        // An arm keeps at least one way out.
        if !allowed && a.banned.len() + 2 > others && !a.banned.contains(&to) {
            return self.refuse(Refusal::LastWayOut);
        }
        self.arm_edit(
            from,
            |_| label,
            |a| {
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
            },
        )
    }

    pub fn set_control(&mut self, control: usize) -> bool {
        if control >= CONTROLS.len() {
            return self.refuse(Refusal::DoesNotFit);
        }
        // Only a roundabout can leave a junction that cannot be drawn.
        let why = if control == ROUNDABOUT { Refusal::RoundaboutTooBig } else { Refusal::DoesNotFit };
        // A change that takes bike boxes away says so, or a screen reader would never hear of it.
        let mut after = self.current().clone();
        after.control = control;
        settle_table_and_boxes(&mut after, self.region);
        let lost = |f: fn(&Arm) -> bool| self.current().arms.iter().zip(&after.arms).any(|(a, b)| f(a) && !f(b));
        let timing_lost = self.current().arms.iter().zip(&after.arms).any(|(a, b)| a.timing.is_some() && b.timing.is_none());
        let key = match (lost(|a| a.bike_box), lost(|a| a.protected), timing_lost) {
            (false, false, false) => "jn-rev-control",
            (true, false, false) => "jn-rev-control-boxes",
            (false, true, false) => "jn-rev-control-protected",
            (true, true, false) => "jn-rev-control-both",
            (false, false, true) => "jn-rev-control-timing",
            (true, false, true) => "jn-rev-control-boxes-timing",
            (false, true, true) => "jn-rev-control-protected-timing",
            (true, true, true) => "jn-rev-control-all",
        };
        self.edit_why(why, Said::new(key).with("control", Arg::MsgLower(control_key(control))), |s| {
            s.control = control;
            true
        })
    }

    /// Raises the whole junction to sidewalk height, or makes it flat again. A roundabout has no table.
    pub fn set_raised(&mut self, on: bool) -> bool {
        if in_force(self.current()).control == ROUNDABOUT {
            return self.refuse(Refusal::RaisedOnRoundabout);
        }
        self.edit(Said::new("jn-rev-raised").with("change", change(on)), |s| {
            s.raised = on;
            true
        })
    }

    /// Runs a bus-only lane across the middle of a roundabout between two
    /// streets, or takes it away with `None`.
    pub fn set_bus(&mut self, pair: Option<(u32, u32)>) -> bool {
        let pair = pair.map(|(a, b)| (a.min(b), a.max(b)));
        if pair.is_some() && self.turbo_in_force() {
            return self.refuse(Refusal::BusOnTurbo);
        }
        let label = match pair {
            Some((a, b)) => match (self.arm(a), self.arm(b)) {
                (Some(x), Some(y)) => {
                    Said::new("jn-rev-bus-across").with("from", Arg::Said(Box::new(arm_name(x)))).with("to", Arg::Said(Box::new(arm_name(y))))
                }
                _ => return self.refuse(Refusal::DoesNotFit),
            },
            None => Said::new("jn-rev-bus-across-remove"),
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
            Some(w) => Said::new("jn-rev-cycle").with("length", Arg::Length(w)),
            None => Said::new("jn-rev-cycle-remove"),
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
        if self.turbo_in_force() {
            return self.step_turbo_size(dir);
        }
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

    /// Whether a turbo is in force: the roundabout is run as one (not suspended for want of traffic).
    fn turbo_in_force(&self) -> bool {
        in_force(self.current()).turbo.is_some()
    }

    /// Gives the roundabout a turbo layout (an index into `LAYOUTS`), or takes it away with `None`. The main road is
    /// the best opposite pair that stands (a turbo already on keeps its own if the new layout allows it), the block the smallest
    /// whose ring fits the streets (a turbo already on starts from its own).
    pub fn set_turbo(&mut self, layout: Option<usize>) -> bool {
        let region = self.region;
        let Some(layout) = layout else {
            return self.edit(Said::new("jn-rev-turbo-off"), |s| s.turbo.take().is_some());
        };
        if layout >= LAYOUTS.len() {
            return self.refuse(Refusal::DoesNotFit);
        }
        let label = Said::new("jn-rev-turbo-layout").with("layout", Arg::MsgLower(LAYOUTS[layout].key));
        self.edit_why(Refusal::RoundaboutTooBig, label, move |s| {
            if in_force(s).control != ROUNDABOUT {
                return false;
            }
            // The main road and block `choose_turbo` finds; if none fits, the largest stays and the edit is refused as too big
            // (or, for a main road that does not stand, naming the arm).
            let Some((turbo, _)) = crate::junction::read_model::choose_turbo(s, region, layout) else { return false };
            s.turbo = Some(turbo);
            true
        })
    }

    /// Sets the block (an index into `BLOCKS`) of the turbo.
    pub fn set_turbo_size(&mut self, size: usize) -> bool {
        if size >= BLOCKS.len() || !self.turbo_in_force() {
            return self.refuse(Refusal::DoesNotFit);
        }
        let label = Said::new("jn-rev-turbo-size").with("size", Arg::MsgLower(BLOCKS[size].key));
        self.edit_why(Refusal::RoundaboutTooBig, label, move |s| s.turbo.as_mut().map(|t| t.size = size).is_some())
    }

    /// Steps the turbo's block one up (1) or down (-1).
    pub fn step_turbo_size(&mut self, dir: i32) -> bool {
        let Some(t) = in_force(self.current()).turbo.clone() else { return self.refuse(Refusal::DoesNotFit) };
        match usize::try_from(t.size as i32 + dir) {
            Ok(size) => self.set_turbo_size(size),
            Err(_) => self.refuse(Refusal::DoesNotFit),
        }
    }

    /// Makes the next opposite pair of streets the main road (a knee also swaps which of the two has two lanes).
    pub fn switch_turbo_main(&mut self) -> bool {
        let region = self.region;
        if !self.turbo_in_force() {
            return self.refuse(Refusal::DoesNotFit);
        }
        // One that stands, else the next in turn, which the edit then refuses by saying why.
        let Some((next, _)) = crate::junction::read_model::next_main(self.current(), region) else { return self.refuse(Refusal::DoesNotFit) };
        let (Some(a), Some(b)) = (self.arm(next[0]), self.arm(next[1])) else { return self.refuse(Refusal::DoesNotFit) };
        let label = Said::new("jn-rev-turbo-main").with("from", Arg::Said(Box::new(arm_name(a)))).with("to", Arg::Said(Box::new(arm_name(b))));
        self.edit_why(Refusal::RoundaboutTooBig, label, move |s| s.turbo.as_mut().map(|t| t.main = next.clone()).is_some())
    }

    /// Sets how much bigger than its least size a roundabout is.
    pub fn set_ring(&mut self, extra_mm: i32) -> bool {
        if self.turbo_in_force() {
            return self.refuse(Refusal::DoesNotFit); // a turbo has blocks, not a length
        }
        let extra = snap(extra_mm, RING_STEP_MM);
        if !(0..=MAX_RING_MM).contains(&extra) {
            return self.refuse(Refusal::DoesNotFit);
        }
        self.edit(Said::new("jn-rev-ring").with("length", Arg::Length(extra)), |s| {
            s.ring_extra_mm = extra;
            true
        })
    }
}

/// The label of choosing a measure from a list for an arm: its code and name, or `none` for the first entry.
fn measure(a: &Arm, item: &Item, none: &'static str) -> Said {
    if item.code.is_empty() {
        about(none, a)
    } else {
        about("jn-rev-measure", a).with("code", Arg::Text(item.code.to_string())).with("measure", Arg::MsgLower(item.key))
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

/// A roundabout is no raised table, and a bike box stands only where `bike_box_blocker` allows one: whatever put it
/// elsewhere (an edit, a save), it goes. A turbo is settled first (`settle_turbo`).
fn settle_table_and_boxes(s: &mut State, region: usize) {
    settle_turbo(s, region);
    if in_force(s).control == ROUNDABOUT {
        s.raised = false;
    }
    let blocked: Vec<bool> = (0..s.arms.len()).map(|i| bike_box_blocker(s, i, &s.arms[i].profile(region)).is_some()).collect();
    for (a, blocked) in s.arms.iter_mut().zip(blocked) {
        a.bike_box = a.bike_box && !blocked;
    }
    let n = s.arms.len();
    let profiles: Vec<Profile> = s.arms.iter().map(|a| a.profile(region)).collect();
    let stands: Vec<bool> = (0..n).map(|i| protected_blocker(s, i, &profiles[i], &profiles[(i + 1) % n]).is_none()).collect();
    for (a, stands) in s.arms.iter_mut().zip(stands) {
        a.protected = a.protected && stands;
    }
    // Timing belongs to a signal, and an early green to a transit lane.
    let signal = s.control == SIGNAL;
    for a in &mut s.arms {
        if !signal {
            a.timing = None;
        } else if let (Some(t), false) = (&mut a.timing, a.bus_lane) {
            t.transit_lead_s = 0;
        }
    }
    signal::settle(s);
}

/// A turbo stands only on a roundabout whose arms suit it, none is more than 20° off its template and every entry it
/// widens stays inside its street's property line: otherwise it goes. While it stands there is no bus lane across the
/// island, no queue jump, no ring size of its own, crossings are at least 6 m from the ring, and the entry lanes are
/// the layout's. Without one the lanes follow the streets.
fn settle_turbo(s: &mut State, region: usize) {
    // The kept control decides, not the one in force: a roundabout whose streets are all walkways but one is only
    // suspended (`in_force`), and its turbo comes back with the traffic. Suspended, the turbo is left as it is (its
    // conditions apply again when it is in force) and the read side draws no ring.
    if s.control == ROUNDABOUT && traffic_arms(s) <= 1 {
        // Only a turbo that cannot be one at all goes: no such layout or block, or a main road that is not two arms.
        if s.turbo.as_ref().is_some_and(|t| !t.fits_arms(&s.arms)) {
            s.turbo = None;
        }
        return;
    }
    let stands = s.control == ROUNDABOUT && s.turbo.as_ref().is_some_and(|t| t.standable(&s.arms, region));
    if !stands {
        // Only a turbo that goes leaves lanes of its own behind: they follow the streets again.
        if s.turbo.take().is_some() {
            normalize(&mut s.arms, region);
        }
        return;
    }
    s.bus = None;
    for a in &mut s.arms {
        if matches!(a.approach, Q_OFFSET | Q_CURB | Q_VIRTUAL) {
            a.approach = 0;
        }
        if let Some(c) = a.crossing.as_mut() {
            c.setback_mm = c.setback_mm.max(turbo::CROSSING_SETBACK_MM);
        }
    }
    turbo_lanes(s, region);
}

/// The lanes of a turbo's arms: the layout's count where cars enter the street, none where they do not, each serving
/// the turns `turbo::uses` says among the turns the arm has (the outermost lane of a side street turns toward the outside only).
fn turbo_lanes(s: &mut State, region: usize) {
    let Some(t) = s.turbo.clone() else { return };
    let avail: Vec<u8> = (0..s.arms.len()).map(|i| classes_of(&s.arms, i)).collect();
    let others: Vec<(u32, i32)> = s.arms.iter().filter(|a| !a.is_pedestrian()).map(|a| (a.uid, a.bearing)).collect();
    let right_hand = REGIONS[region].drive_side == Side::Right;
    for (a, avail) in s.arms.iter_mut().zip(avail) {
        let (uid, bearing) = (a.uid, a.bearing);
        let n = if a.profile(region).enter_x.is_empty() { 0 } else { t.entry_lanes(uid) };
        a.lanes = (0..n)
            .map(|k| {
                // An arm that is not on the main road is a side street: see `turbo::uses`.
                let want = turbo::uses(k, n, avail, !t.main.contains(&uid), right_hand);
                let mut to: Vec<u32> = others.iter().filter(|(u, b)| *u != uid && want & turn_class(bearing, *b) != 0).map(|(u, _)| *u).collect();
                to.sort_unstable();
                Lane { to }
            })
            .collect();
    }
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
    // Keep the roundabout and drop only the turbo, if that is enough.
    if t.turbo.is_some() {
        let mut plain = t.clone();
        plain.turbo = None;
        normalize(&mut plain.arms, region); // the lanes the turbo set follow the streets again
        settle_table_and_boxes(&mut plain, region);
        if valid(&plain, region) {
            return Some(plain);
        }
    }
    for a in &mut t.arms {
        a.offset_mm = 0;
    }
    if t.control == ROUNDABOUT {
        t.control = PRIORITY;
    }
    (t.bus, t.cycle, t.ring_extra_mm) = (None, None, 0);
    t.turbo = None;
    normalize(&mut t.arms, region); // the lanes a turbo set follow the streets again
    valid(&t, region).then_some(t)
}

/// Keeps the parts that follow from the arms consistent: lane counts follow
/// each street, a lane serves only turns its arm has, bans name arms that
/// exist, and features that need room or parking stay only where they have it.
pub fn normalize(arms: &mut [Arm], region: usize) {
    // The streets a lane can go to: every street but the pedestrian ones, which nothing drives into.
    let dests: Vec<(u32, i32)> = arms.iter().filter(|a| !a.is_pedestrian()).map(|a| (a.uid, a.bearing)).collect();
    let avail: Vec<u8> = (0..arms.len()).map(|i| classes_of(arms, i)).collect();
    // The arms that carry traffic again after having none: the other arms' lanes lead to them by default. A street that
    // goes only one way out has no lanes of its own, so it is told by having been a pedestrian street.
    let regained: Vec<(u32, i32)> = arms
        .iter()
        .filter(|a| {
            let p = a.profile(region);
            !a.is_pedestrian() && a.lanes.is_empty() && (!p.enter_x.is_empty() || (a.walked && !p.leave_x.is_empty()))
        })
        .map(|a| (a.uid, a.bearing))
        .collect();
    for (a, avail) in arms.iter_mut().zip(avail) {
        let p = a.profile(region);
        let n = p.enter_x.len();
        let (uid, bearing) = (a.uid, a.bearing);
        // An arm that had no lanes (a pedestrian street) and has some again has the crossing a new arm has.
        let regains_traffic = regained.iter().any(|(u, _)| *u == a.uid);
        // Where lane `k` goes by default: every street whose turn it serves.
        let default = |k: usize| -> Vec<u32> {
            let want = default_uses(k, n, avail);
            dests.iter().filter(|(u, b)| *u != uid && want & turn_class(bearing, *b) != 0).map(|(u, _)| *u).collect()
        };
        if a.lanes.len() != n {
            a.lanes = (0..n).map(|k| Lane { to: default(k) }).collect();
        }
        for (k, l) in a.lanes.iter_mut().enumerate() {
            let want = default_uses(k, n, avail);
            l.to.extend(regained.iter().filter(|(u, b)| *u != uid && want & turn_class(bearing, *b) != 0).map(|(u, _)| *u));
            l.to.retain(|u| *u != uid && dests.iter().any(|(d, _)| d == u));
            l.to.sort_unstable();
            l.to.dedup();
            if l.to.is_empty() {
                l.to = default(k);
            }
        }
        a.banned.retain(|b| *b != a.uid && dests.iter().any(|(d, _)| d == b));
        a.bulb = [a.bulb[0] && p.park[0] > 0, a.bulb[1] && p.park[1] > 0];
        if a.is_pedestrian() {
            // There is no roadway to cross, and no traffic to give a lane, a stop or a rule to. The timing stays: with
            // no phase it does nothing, and it returns with traffic.
            a.crossing = None;
            a.bus_lane = false;
            a.filter = false;
            a.approach = 0;
            a.stop = 0;
            a.rule = 0;
            a.bike_box = false;
            a.walked = true;
        } else {
            if regains_traffic && a.crossing.is_none() {
                a.crossing = Some(default_crossing());
            }
            a.walked = false;
            if let Some(c) = a.crossing.as_mut() {
                c.island = c.island && p.road_mm() >= ISLAND_MIN_ROAD_MM && !c.continuous;
            }
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
    crate::junction::read_model::layout(s, region).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::i18n::Locale;
    use crate::shared::testing::{en, fr};
    use crate::street::model::SAMPLES;

    #[test]
    fn a_buffer_is_not_part_of_the_carriageway_the_profile_reads() {
        use crate::shared::catalogue::{Side, StreetClass, kind_index};
        use crate::street::model::Piece;
        let piece = |id: &str, width_mm| Piece { kind: kind_index(id).unwrap(), width_mm, direction: None, variants: Vec::new() };
        let street =
            Street::imported(StreetClass::Local, Side::Right, &[piece("sidewalk", 2000), piece("buffer", 600), piece("travel", 3300), piece("sidewalk", 2000)]);
        let p = profile_of(&street, 0);
        assert_eq!((p.road_l, p.road_r), (2600, 5900), "the carriageway begins at the lane, not at the buffer");
        assert_eq!(p.edge_kind[0], kind_index("buffer"));
    }

    #[test]
    fn an_arm_is_pedestrian_when_the_street_it_reads_is() {
        let j = junction_with_walkways(&[90], &[("sidewalk", 4000), ("sidewalk", 4000)]);
        let walking: Vec<i32> = j.current().arms.iter().filter(|a| a.is_pedestrian()).map(|a| a.bearing).collect();
        assert_eq!(walking, [90]);
        assert_eq!(traffic_arms(j.current()), 3);
        assert!(!Junction::new(0).current().arms.iter().any(Arm::is_pedestrian), "a sandbox arm reads a sample street");
        assert!(!Arm::new(1, 0, 0).is_pedestrian(), "an arm that reads no street is not pedestrian");
    }

    #[test]
    fn a_junction_with_at_most_one_street_carrying_traffic_is_run_as_uncontrolled_and_keeps_its_control() {
        use crate::shared::catalogue::{Side, StreetClass, kind_index};
        use crate::street::model::Piece;
        let mut s = Junction::new(0).current().clone();
        assert_eq!(s.control, SIGNAL);
        assert_eq!(in_force(&s).control, SIGNAL, "four streets carry traffic");
        let piece = Piece { kind: kind_index("sidewalk").unwrap(), width_mm: 4000, direction: None, variants: Vec::new() };
        let walkway = Street::imported(StreetClass::Pedestrian, Side::Right, &[piece]);
        for a in s.arms.iter_mut().skip(2) {
            a.section = Some(walkway.clone());
        }
        assert_eq!(traffic_arms(&s), 2);
        assert_eq!(in_force(&s).control, SIGNAL, "two streets still meet");
        s.arms[1].section = Some(walkway);
        assert_eq!(traffic_arms(&s), 1);
        assert_eq!(in_force(&s).control, UNCONTROLLED);
        assert_eq!(s.control, SIGNAL, "the control that was kept is not touched");
    }

    #[test]
    fn a_walkway_arm_has_no_crossing_and_no_lane_goes_to_it() {
        let j = junction_with_walkways(&[90], &[("sidewalk", 3000), ("terrace", 2000), ("sidewalk", 3000)]);
        let walk = j.current().arms.iter().find(|a| a.bearing == 90).unwrap();
        assert!(walk.crossing.is_none() && walk.lanes.is_empty());
        for a in j.current().arms.iter().filter(|a| a.uid != walk.uid) {
            assert!(a.crossing.is_some(), "a street of traffic keeps its crossing");
            assert!(!a.lanes.is_empty());
            for l in &a.lanes {
                assert!(!l.to.is_empty(), "a lane keeps a street to go to");
                assert!(!l.to.contains(&walk.uid), "nothing is driven into a walkway");
            }
        }
    }

    #[test]
    fn a_crossing_cannot_be_added_to_a_walkway_arm_and_a_lane_cannot_be_sent_into_it() {
        let mut j = junction_with_walkways(&[90], &[("sidewalk", 3000), ("sidewalk", 3000)]);
        let (walk, other) = (arm_at(&j, 90), arm_at(&j, 0));
        assert!(!j.set_crossing(walk, true));
        assert_eq!(j.refusal(), Some(Refusal::CrossingOnPedestrian));
        assert!(j.current().arms.iter().find(|a| a.uid == walk).unwrap().crossing.is_none());
        assert!(j.set_crossing(other, false), "a street of traffic can lose its crossing");
        assert_eq!(j.refusal(), None);
        assert!(!j.set_lane_dest(other, 0, walk, true));
        assert_eq!(j.refusal(), Some(Refusal::DoesNotFit));
    }

    #[test]
    fn a_kept_roundabout_with_one_street_of_traffic_runs_uncontrolled_and_allows_a_raised_table() {
        let mut j = junction_with_walkways(&[90, 180, 270], &[("sidewalk", 4000), ("sidewalk", 4000)]);
        assert_eq!(traffic_arms(j.current()), 1);
        assert!(j.set_control(ROUNDABOUT), "{:?}", j.refusal());
        assert!(j.set_raised(true), "{:?}", j.refusal());
        assert!(j.current().raised && j.current().control == ROUNDABOUT);
        assert!(j.view().raised);
    }

    #[test]
    fn a_street_made_pedestrian_loses_its_traffic_measures_and_a_walkway_takes_none() {
        let mut j = junction_with_walkways(&[90], &[("sidewalk", 3000), ("sidewalk", 3000)]);
        let mut arms = j.current().arms.clone();
        let walk = arms.iter_mut().find(|a| a.bearing == 90).unwrap();
        (walk.bus_lane, walk.filter, walk.approach, walk.stop, walk.rule, walk.bike_box) = (true, true, 1, 1, 1, true);
        normalize(&mut arms, 0);
        let walk = arms.iter().find(|a| a.bearing == 90).unwrap();
        assert!(!walk.bus_lane && !walk.filter && !walk.bike_box);
        assert_eq!((walk.approach, walk.stop, walk.rule), (0, 0, 0));
        let w = arm_at(&j, 90);
        assert!(!j.set_bus_lane(w, true));
        assert_eq!(j.refusal(), Some(Refusal::MeasureOnPedestrian));
        assert!(!j.set_filter(w, true) && j.refusal() == Some(Refusal::MeasureOnPedestrian));
        assert!(!j.set_approach(w, 1) && j.refusal() == Some(Refusal::MeasureOnPedestrian));
        assert!(!j.set_stop(w, 1) && j.refusal() == Some(Refusal::MeasureOnPedestrian));
        assert!(!j.set_rule(w, 1) && j.refusal() == Some(Refusal::MeasureOnPedestrian));
        assert_eq!(j.current().control, SIGNAL);
        assert!(!j.set_timing(w, TimingField::Green, 25) && j.refusal() == Some(Refusal::MeasureOnPedestrian));
        assert!(j.set_bus_lane(arm_at(&j, 0), true), "a street of traffic takes its measures");
    }

    #[test]
    fn the_refusal_of_a_crossing_on_a_walkway_is_a_refusal_with_a_message() {
        assert_eq!(Refusal::ALL.len(), 29);
        assert_eq!(Refusal::CrossingOnPedestrian.key(), "jn-refusal-crossing-pedestrian");
        assert!(Refusal::ALL.contains(&Refusal::CrossingOnPedestrian));
    }

    #[test]
    fn the_turns_an_arm_has_leave_out_a_walkway() {
        let j = junction_with_walkways(&[90], &[("sidewalk", 3000), ("sidewalk", 3000)]);
        let arms = &j.current().arms;
        let north = arms.iter().position(|a| a.bearing == 0).unwrap();
        let plain = Junction::new(0).current().arms.clone();
        assert!(classes_of(arms, north).count_ones() < classes_of(&plain, north).count_ones());
    }

    #[test]
    fn a_junction_whose_other_streets_are_all_walkways_keeps_a_lane_with_nowhere_to_go() {
        let j = junction_with_walkways(&[90, 180, 270], &[("sidewalk", 4000), ("sidewalk", 4000)]);
        assert_eq!(traffic_arms(j.current()), 1);
        let arm = j.current().arms.iter().find(|a| a.bearing == 0).unwrap();
        assert!(!arm.lanes.is_empty() && arm.lanes.iter().all(|l| l.to.is_empty()));
        let _ = j.view();
    }

    #[test]
    fn the_lanes_of_the_other_arms_lead_again_to_an_arm_that_regains_traffic() {
        let j = junction_with_walkways(&[90], &[("sidewalk", 3000), ("sidewalk", 3000)]);
        let mut arms = j.current().arms.clone();
        let walk = arms.iter().position(|a| a.bearing == 90).unwrap();
        let x = arms[walk].uid;
        assert!(arms.iter().all(|a| a.lanes.iter().all(|l| !l.to.contains(&x))));
        // The resident took one street out of a lane of the north arm that has two or more.
        let north = arms.iter().position(|a| a.bearing == 0).unwrap();
        let lane = arms[north].lanes.iter().position(|l| l.to.len() > 1).expect("a lane with a choice");
        let removed = arms[north].lanes[lane].to.remove(0);
        let street = arms[north].section.clone();
        arms[walk].section = street;
        normalize(&mut arms, 0);
        for (i, a) in arms.iter().enumerate().filter(|(i, _)| *i != walk) {
            let avail = classes_of(&arms, i);
            for (k, l) in a.lanes.iter().enumerate() {
                let serves = default_uses(k, a.lanes.len(), avail) & turn_class(a.bearing, 90) != 0;
                assert_eq!(l.to.contains(&x), serves, "arm {} lane {k}", a.bearing);
            }
        }
        assert!(!arms[north].lanes[lane].to.contains(&removed), "what the resident removed stays removed");
    }

    fn one_way_out() -> Street {
        use crate::shared::catalogue::{Side, StreetClass, kind_index};
        use crate::street::model::Piece;
        let pieces: Vec<Piece> = [("sidewalk", 2000), ("travel", 3200), ("sidewalk", 2000)]
            .iter()
            .map(|&(id, width_mm)| Piece { kind: kind_index(id).unwrap(), width_mm, direction: (id == "travel").then_some(0), variants: Vec::new() })
            .collect();
        Street::imported(StreetClass::Local, Side::Right, &pieces)
    }

    #[test]
    fn an_arm_returning_to_traffic_one_way_out_has_the_default_crossing_and_the_lanes_of_the_others_lead_to_it() {
        let j = junction_with_walkways(&[90], &[("sidewalk", 3000), ("sidewalk", 3000)]);
        let mut arms = j.current().arms.clone();
        let walk = arms.iter().position(|a| a.bearing == 90).unwrap();
        arms[walk].section = Some(one_way_out());
        assert!(arms[walk].profile(0).enter_x.is_empty() && !arms[walk].profile(0).leave_x.is_empty(), "the street goes only out");
        normalize(&mut arms, 0);
        assert!(arms[walk].crossing.is_some(), "the crossing is given back");
        assert!(arms[walk].lanes.is_empty());
        let x = arms[walk].uid;
        assert!(arms.iter().filter(|a| a.uid != x).any(|a| a.lanes.iter().any(|l| l.to.contains(&x))), "lanes lead into it again");
    }

    #[test]
    fn an_arm_that_goes_one_way_out_all_along_keeps_a_removed_crossing_and_a_removed_destination() {
        let mut j = junction_with_walkways(&[90], &[("sidewalk", 3000), ("sidewalk", 3000)]);
        let walk = arm_at(&j, 90);
        let (north, south) = (arm_at(&j, 0), arm_at(&j, 180));
        let street = Some(one_way_out());
        let mut arms = j.current().arms.clone();
        arms.iter_mut().find(|a| a.uid == walk).unwrap().section = street;
        normalize(&mut arms, 0);
        let mut s = j.current().clone();
        s.arms = arms;
        j = Junction::from_city(named_said("Test"), &s, &s, 0).expect("draws");
        assert!(j.set_crossing(walk, false));
        let lane = j.arm(north).unwrap().lanes.iter().position(|l| l.to.contains(&walk) && l.to.len() > 1).expect("a lane with a choice");
        assert!(j.set_lane_dest(north, lane, walk, false));
        let mut arms = j.current().arms.clone();
        normalize(&mut arms, 0);
        let (w, n) = (arms.iter().find(|a| a.uid == walk).unwrap(), arms.iter().find(|a| a.uid == north).unwrap());
        assert!(w.crossing.is_none(), "the resident's choice stands");
        assert!(!n.lanes[lane].to.contains(&walk));
        let _ = south;
    }

    #[test]
    fn the_only_street_of_traffic_going_one_way_out_regains_its_crossing_once_and_a_removed_one_stays_removed() {
        let j = junction_with_walkways(&[0, 90, 180, 270], &[("sidewalk", 3000), ("sidewalk", 3000)]);
        let mut s = j.current().clone();
        let north = s.arms.iter().position(|a| a.bearing == 0).unwrap();
        assert!(s.arms[north].walked && s.arms[north].crossing.is_none());
        s.arms[north].section = Some(one_way_out());
        let mut j = Junction::from_city(named_said("Test"), &s, &s, 0).expect("draws");
        let uid = arm_at(&j, 0);
        assert!(j.arm(uid).unwrap().crossing.is_some() && !j.arm(uid).unwrap().walked, "the crossing is given back");
        assert!(j.set_crossing(uid, false));
        assert!(j.set_corner(uid, 4_000));
        let mut arms = j.current().arms.clone();
        normalize(&mut arms, 0);
        assert!(j.arm(uid).unwrap().crossing.is_none(), "the resident's choice stands after an edit");
        assert!(arms.iter().find(|a| a.uid == uid).unwrap().crossing.is_none(), "and after a further normalize");
    }

    #[test]
    fn an_arm_whose_street_regains_traffic_has_the_default_crossing_and_lanes_with_destinations() {
        let j = junction_with_walkways(&[90], &[("sidewalk", 3000), ("sidewalk", 3000)]);
        let mut arms = j.current().arms.clone();
        let walk = arms.iter().position(|a| a.bearing == 90).unwrap();
        assert!(arms[walk].crossing.is_none() && arms[walk].lanes.is_empty());
        let street = arms.iter().find_map(|a| (a.bearing == 0).then(|| a.section.clone())).unwrap();
        arms[walk].section = street;
        normalize(&mut arms, 0);
        let c = arms[walk].crossing.expect("the arm has a crossing again");
        assert_eq!((c.setback_mm, c.width_mm, c.island, c.continuous), (DEFAULT_SETBACK_MM, DEFAULT_CROSSING_MM, false, false));
        assert!(!arms[walk].lanes.is_empty());
        assert!(arms[walk].lanes.iter().all(|l| !l.to.is_empty()), "its lanes go somewhere");
    }

    fn named(name: &str) -> Said {
        Said::new("city-name").with("name", Arg::Text(name.into()))
    }

    #[test]
    fn a_junction_from_a_city_has_the_name_the_city_gives_it() {
        let city = crate::city::model::City::new();
        let view = city.view(0);
        let node = view.nodes.iter().find(|n| n.junction).unwrap();
        let j = city.junction_editor(node.uid, 0).unwrap();
        assert_eq!(j.view().name, node.name);
    }

    #[test]
    fn an_arm_is_the_street_it_reads() {
        let city = crate::city::model::City::new();
        let node = city.view(0).nodes.iter().find(|n| n.junction).unwrap().uid;
        let j = city.junction_editor(node, 0).unwrap();
        for a in &j.current().arms {
            let section = a.section.as_ref().expect("a city arm carries its street");
            assert_eq!(a.street_name(), section.title());
            assert_eq!(a.row_mm(), section.row_mm);
            assert_eq!(a.class(), Some(section.class));
        }
    }

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

    /// The profile of sample street `sample`, laid out for the side of the road of `region`.
    fn profile(sample: usize, region: usize) -> Profile {
        profile_of(&Street::sample(sample, Side::Right), region)
    }

    /// Which sample street an arm of a sample junction reads.
    fn sample_of(a: &Arm) -> usize {
        let name = en(&a.street_name());
        SAMPLES.iter().position(|s| s.name == name).expect("an arm of a sample junction reads a sample street")
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
    fn arms_stay_between_three_and_five() {
        let mut j = Junction::new(1);
        let a = j.current().arms[0].uid;
        assert!(!j.remove_arm(a));
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
        let lane = j.current().arms.iter().find(|a| sample_of(a) == 2).unwrap().uid;
        assert!(!j.set_island(lane, true), "8.4 m carriageway is too narrow");
    }

    #[test]
    fn a_lane_goes_to_streets_one_by_one_and_always_to_at_least_one() {
        let mut j = Junction::new(1); // street, lane (south), street
        let lane = j.current().arms.iter().find(|a| sample_of(a) == 2).unwrap().uid;
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
    fn a_lane_can_be_selected_only_where_the_arm_has_it() {
        let mut j = Junction::new(0);
        let n = j.current().arms[0].uid;
        j.select(Target::Lane(n, 1));
        assert_eq!(j.selected, Target::Lane(n, 1));
        j.select(Target::Lane(n, 2));
        assert_eq!(j.selected, Target::None, "the avenue has two lanes in");
        let mut j = Junction::new(1); // street, lane (south), street
        let lane = j.current().arms.iter().find(|a| sample_of(a) == 2).unwrap().uid;
        j.select(Target::Lane(lane, 0));
        assert_eq!(j.selected, Target::Lane(lane, 0));
        j.select(Target::Lane(lane, 1));
        assert_eq!(j.selected, Target::None, "a lane has one lane in");
    }

    #[test]
    fn region_change_flips_which_side_lanes_enter() {
        let mut j = Junction::new(0);
        assert!(j.set_region(3));
        assert!(!j.set_region(3));
        for a in &j.current().arms {
            let p = profile(sample_of(a), 3);
            assert_eq!(a.lanes.len(), p.enter_x.len());
        }
    }

    fn linked(sample: usize) -> Junction {
        let mut state = Junction::new(sample).current().clone();
        for a in &mut state.arms {
            a.edge = a.uid;
        }
        Junction::from_city(named("Test junction"), &state, &state, 0).expect("the sample draws")
    }

    #[test]
    fn a_junction_in_a_city_keeps_its_streets_and_name() {
        let mut j = linked(0);
        assert!(j.is_linked());
        let uid = j.current().arms[0].uid;
        assert!(!j.remove_arm(uid));
        assert_eq!(en(&j.view().name), "Test junction");
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
            if sample_of(a) == 0 {
                a.section = Some(narrow.clone());
            }
        }
        let j = Junction::from_city(named("Test"), &state, &state, 0).unwrap();
        let arm = j.current().arms.iter().find(|a| sample_of(a) == 0).unwrap();
        assert_eq!(arm.profile(0).park, [0, 0]);
        assert!(profile(0, 0).park != [0, 0]);
    }

    #[test]
    fn earlier_changes_show_as_one_revision_and_start_over_clears_them() {
        let base = linked(0);
        // What the city holds has no street attached; it attaches them again as it opens the junction.
        let attach = |s: &mut State| {
            for a in &mut s.arms {
                a.section = base.arm(a.uid).and_then(|b| b.section.clone());
                a.beyond = base.arm(a.uid).and_then(|b| b.beyond.clone());
            }
        };
        let mut today = base.snapshot();
        attach(&mut today);
        let mut state = today.clone();
        state.control = PRIORITY;
        let mut j = Junction::from_city(named("Test"), &today, &state, 0).unwrap();
        assert!(j.changed());
        assert_eq!(j.revisions().map(en).collect::<Vec<_>>(), ["Earlier changes"]);
        assert_eq!(j.revisions().map(fr).collect::<Vec<_>>(), ["Modifications antérieures"]);
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
    fn a_junction_is_raised_and_flat_again_and_undo_goes_back() {
        let mut j = Junction::new(0);
        assert!(!j.current().raised);
        assert!(j.set_raised(true));
        assert!(j.current().raised);
        assert!(!j.set_raised(true), "nothing changes");
        assert!(j.undo());
        assert!(!j.current().raised);
        assert!(j.redo());
        assert!(j.set_raised(false));
        assert!(!j.current().raised);
    }

    #[test]
    fn a_roundabout_cannot_be_a_raised_table_and_becoming_one_flattens_it() {
        let mut j = Junction::new(0);
        assert!(j.set_control(ROUNDABOUT));
        assert!(!j.set_raised(true));
        assert_eq!(j.refusal(), Some(Refusal::RaisedOnRoundabout));
        let mut k = Junction::new(0);
        assert!(k.set_raised(true));
        assert!(k.set_control(ROUNDABOUT));
        assert!(!k.current().raised);
        assert!(k.undo());
        assert!(k.current().raised, "undo brings the table back");
    }

    #[test]
    fn a_roundabout_loaded_from_a_save_that_says_raised_is_not_raised() {
        let mut state = Junction::new(0).current().clone();
        state.control = ROUNDABOUT;
        state.raised = true;
        for a in &mut state.arms {
            a.edge = a.uid;
        }
        let mut j = Junction::from_city(named("Test"), &state, &state, 0).expect("draws");
        assert!(!j.current().raised);
        assert!(j.set_control(SIGNAL));
        assert!(!j.current().raised, "it does not turn into a table nobody asked for");
    }

    #[test]
    fn raising_a_junction_is_said_in_both_languages() {
        let mut j = Junction::new(0);
        assert!(j.set_raised(true));
        let said = j.revisions().last().cloned().unwrap();
        assert_eq!(en(&said), "Raised table: add");
        assert_eq!(fr(&said), "Carrefour surélevé\u{a0}: ajout");
        assert!(j.set_raised(false));
        assert_eq!(en(&j.revisions().last().cloned().unwrap()), "Raised table: remove");
    }

    #[test]
    fn a_crossing_is_made_a_continuous_sidewalk_and_flat_again_and_undo_goes_back() {
        let mut j = Junction::new(0);
        let n = arm_at(&j, 0);
        let on = |j: &Junction| j.arm(n).unwrap().crossing.unwrap().continuous;
        assert!(!on(&j));
        assert!(j.set_continuous(n, true));
        assert!(on(&j));
        assert!(!j.set_continuous(n, true), "nothing changes");
        assert!(j.undo());
        assert!(!on(&j));
        assert!(j.redo());
        assert!(j.set_continuous(n, false));
        assert!(!on(&j));
    }

    #[test]
    fn a_street_with_no_crossing_cannot_be_continuous_and_removing_the_crossing_takes_it_away() {
        let mut j = Junction::new(0);
        let n = arm_at(&j, 0);
        assert!(j.set_crossing(n, false));
        assert!(!j.set_continuous(n, true));
        assert!(j.refusal().is_some());
        assert!(j.undo());
        assert!(j.set_continuous(n, true));
        assert!(j.set_crossing(n, false));
        assert!(j.set_crossing(n, true));
        assert!(!j.arm(n).unwrap().crossing.unwrap().continuous, "a new crossing is a plain one");
        assert!(j.undo() && j.undo());
        assert!(j.arm(n).unwrap().crossing.unwrap().continuous, "undo brings the continuous crossing back");
    }

    #[test]
    fn a_continuous_sidewalk_has_no_refuge_island() {
        let mut j = Junction::new(0);
        let n = arm_at(&j, 0);
        let island = |j: &Junction| j.arm(n).unwrap().crossing.unwrap().island;
        assert!(island(&j), "the sample's avenue starts with one");
        assert!(j.set_continuous(n, true));
        assert!(!island(&j), "making it continuous takes the island away");
        assert!(!j.set_island(n, true));
        assert!(j.refusal().is_some());
        assert!(j.undo());
        assert!(island(&j), "undo brings the island back");
    }

    #[test]
    fn a_saved_crossing_that_is_both_continuous_and_has_an_island_keeps_only_the_sidewalk() {
        let mut state = Junction::new(0).current().clone();
        let n = state.arms.iter().find(|a| a.bearing == 0).unwrap().uid;
        for a in &mut state.arms {
            a.edge = a.uid;
            if a.uid == n {
                let c = a.crossing.as_mut().unwrap();
                c.island = true;
                c.continuous = true;
            }
        }
        let j = Junction::from_city(named("Test"), &state, &state, 0).expect("draws");
        let c = j.arm(n).unwrap().crossing.unwrap();
        assert!(c.continuous && !c.island);
    }

    #[test]
    fn making_a_crossing_continuous_says_when_it_takes_the_island_away() {
        let mut j = Junction::new(0);
        let n = arm_at(&j, 0);
        assert!(j.set_continuous(n, true));
        let said = j.revisions().last().cloned().unwrap();
        assert_eq!(en(&said), "Sample Avenue 2 (north) continuous sidewalk: add; refuge island removed");
        assert_eq!(fr(&said), "Sample Avenue 2 (nord), trottoir continu\u{a0}: ajout; îlot refuge retiré");
        assert!(j.set_continuous(n, false));
        assert_eq!(en(&j.revisions().last().cloned().unwrap()), "Sample Avenue 2 (north) continuous sidewalk: remove");
    }

    #[test]
    fn a_continuous_sidewalk_is_said_in_both_languages() {
        let mut j = Junction::new(0);
        let e = arm_at(&j, 90); // no island there
        assert!(j.set_continuous(e, true));
        let said = j.revisions().last().cloned().unwrap();
        assert_eq!(en(&said), "Sample Street 1 (east) continuous sidewalk: add");
        assert_eq!(fr(&said), "Sample Street 1 (est), trottoir continu\u{a0}: ajout");
    }

    const BIKED: [(&str, i32); 6] = [("sidewalk", 2000), ("bike", 1500), ("travel", 3200), ("travel", 3200), ("bike", 1500), ("sidewalk", 2000)];

    /// A street of these pieces left to right whose travel lanes run `travel_dir` (1 toward the junction, 0 away).
    fn street_of(pieces: &[(&str, i32)], travel_dir: usize) -> Street {
        use crate::shared::catalogue::{Side, StreetClass, kind_index};
        use crate::street::model::Piece;
        let pieces: Vec<Piece> = pieces
            .iter()
            .map(|&(id, width_mm)| Piece { kind: kind_index(id).unwrap(), width_mm, direction: (id == "travel").then_some(travel_dir), variants: Vec::new() })
            .collect();
        Street::imported(StreetClass::Local, Side::Right, &pieces)
    }

    /// The sample four-way with every arm reading `street`, as a junction of a city.
    fn junction_reading(street: &Street) -> Junction {
        let mut state = Junction::new(0).current().clone();
        for a in &mut state.arms {
            a.section = Some(street.clone());
            a.edge = a.uid;
        }
        Junction::from_city(named("Test"), &state, &state, 0).expect("the street draws")
    }

    #[test]
    fn the_outer_bike_lanes_are_beside_the_curb_or_beside_parking_and_never_between_travel_lanes() {
        let lanes = |pieces: &[(&str, i32)]| junction_of(pieces).current().arms[0].profile(0).outer_bike();
        assert_eq!(lanes(&BIKED), vec![1, 4]);
        assert_eq!(lanes(&[("sidewalk", 2000), ("parking", 2000), ("bike", 1500), ("travel", 3200), ("travel", 3200), ("sidewalk", 2000)]), vec![2]);
        assert!(lanes(&[("sidewalk", 2000), ("travel", 3200), ("bike", 1500), ("travel", 3200), ("sidewalk", 2000)]).is_empty());
        assert!(lanes(&[("sidewalk", 2000), ("travel", 3200), ("travel", 3200), ("sidewalk", 2000)]).is_empty());
    }

    #[test]
    fn a_bike_box_is_switched_on_and_off_and_undo_goes_back() {
        let mut j = junction_of(&BIKED);
        let a = j.current().arms[0].uid;
        let on = |j: &Junction| j.arm(a).unwrap().bike_box;
        assert!(!on(&j));
        assert!(j.set_bike_box(a, true));
        assert!(on(&j));
        assert!(!j.set_bike_box(a, true), "nothing changes");
        assert!(j.undo());
        assert!(!on(&j));
        assert!(j.redo());
        assert!(j.set_bike_box(a, false));
        assert!(!on(&j));
    }

    #[test]
    fn a_bike_box_needs_a_bike_lane_a_street_cars_enter_and_no_roundabout() {
        let mut none = junction_of(&[("sidewalk", 2000), ("travel", 3200), ("travel", 3200), ("sidewalk", 2000)]);
        let a = none.current().arms[0].uid;
        assert!(!none.set_bike_box(a, true));
        assert_eq!(none.refusal(), Some(Refusal::BikeBoxNotHere));

        let mut out = junction_reading(&street_of(&[("sidewalk", 2000), ("bike", 1500), ("travel", 3200), ("bike", 1500), ("sidewalk", 2000)], 0));
        let a = out.current().arms[0].uid;
        assert!(!out.set_bike_box(a, true), "no lane comes in");
        assert_eq!(out.refusal(), Some(Refusal::BikeBoxNotHere));

        let mut ring = junction_of(&BIKED);
        assert!(ring.set_control(ROUNDABOUT));
        let a = ring.current().arms[0].uid;
        assert!(!ring.set_bike_box(a, true));
        assert_eq!(ring.refusal(), Some(Refusal::BikeBoxNotHere));
    }

    #[test]
    fn a_roundabout_takes_every_bike_box_away_and_undo_brings_them_back() {
        let mut j = junction_of(&BIKED);
        for uid in j.current().arms.iter().map(|a| a.uid).collect::<Vec<_>>() {
            assert!(j.set_bike_box(uid, true));
        }
        assert!(j.set_control(ROUNDABOUT));
        assert!(j.current().arms.iter().all(|a| !a.bike_box));
        assert!(j.undo());
        assert!(j.current().arms.iter().all(|a| a.bike_box));
    }

    #[test]
    fn a_saved_bike_box_on_a_street_that_lost_its_bike_lane_is_dropped() {
        let mut state = junction_of(&BIKED).current().clone();
        let plain = street_of(&[("sidewalk", 2000), ("travel", 3200), ("travel", 3200), ("sidewalk", 2000)], 1);
        for a in &mut state.arms {
            a.bike_box = true;
            a.section = Some(plain.clone());
            a.edge = a.uid;
        }
        let j = Junction::from_city(named("Test"), &state, &state, 0).expect("draws");
        assert!(j.current().arms.iter().all(|a| !a.bike_box));
    }

    #[test]
    fn a_bike_box_is_only_for_an_approach_that_stops_and_a_free_one_loses_it() {
        let mut j = junction_of(&BIKED);
        let uids: Vec<u32> = j.current().arms.iter().map(|a| a.uid).collect();
        for &uid in &uids {
            assert!(j.set_bike_box(uid, true));
        }
        assert!(j.set_control(PRIORITY));
        let boxed: Vec<bool> = j.current().arms.iter().map(|a| a.bike_box).collect();
        assert_eq!(boxed.iter().filter(|b| **b).count(), 2, "the two side streets keep theirs: {boxed:?}");
        let free = j.current().arms.iter().find(|a| !a.bike_box).unwrap().uid;
        assert!(!j.set_bike_box(free, true), "the main street of a priority junction does not stop");
        assert_eq!(j.refusal(), Some(Refusal::BikeBoxNotHere));
        assert!(j.undo());
        assert!(j.current().arms.iter().all(|a| a.bike_box), "undo brings them back");
    }

    #[test]
    fn a_control_change_that_takes_bike_boxes_away_says_so() {
        let boxed = || {
            let mut j = junction_of(&BIKED);
            for uid in j.current().arms.iter().map(|a| a.uid).collect::<Vec<_>>() {
                assert!(j.set_bike_box(uid, true));
            }
            j
        };
        let last = |j: &Junction| j.revisions().last().cloned().unwrap();
        let mut keeps = boxed();
        assert!(keeps.set_control(ALL_WAY_STOP));
        assert_eq!(en(&last(&keeps)), "Control: all-way stop", "every approach still stops: nothing is lost");
        let mut loses = boxed();
        assert!(loses.set_control(PRIORITY));
        assert!(en(&last(&loses)).ends_with("; bike boxes removed"), "{}", en(&last(&loses)));
        assert!(fr(&last(&loses)).ends_with("; sas cyclables retirés"), "{}", fr(&last(&loses)));
        let mut plain = junction_of(&BIKED);
        assert!(plain.set_control(PRIORITY));
        assert!(!en(&last(&plain)).contains("removed"), "no box, nothing to say");
    }

    #[test]
    fn the_bike_lane_beside_each_curb_is_found_and_outer_bike_still_agrees() {
        let sides = |pieces: &[(&str, i32)]| junction_of(pieces).current().arms[0].profile(0).outer_bike_sides();
        assert_eq!(sides(&BIKED), [Some(1), Some(4)]);
        assert_eq!(sides(&[("sidewalk", 2000), ("bike", 1500), ("travel", 3200), ("travel", 3200), ("sidewalk", 2000)]), [Some(1), None]);
        assert_eq!(sides(&[("sidewalk", 2000), ("travel", 3200), ("travel", 3200), ("bike", 1500), ("sidewalk", 2000)]), [None, Some(3)]);
        assert_eq!(sides(&[("sidewalk", 2000), ("parking", 2000), ("bike", 1500), ("travel", 3200), ("travel", 3200), ("sidewalk", 2000)]), [Some(2), None]);
        assert_eq!(sides(&[("sidewalk", 2000), ("bike", 2500), ("sidewalk", 2000)]), [Some(1), Some(1)]);
        assert_eq!(junction_of(&BIKED).current().arms[0].profile(0).outer_bike(), vec![1, 4]);
    }

    #[test]
    fn the_bike_lane_a_protected_corner_follows_touches_the_curb() {
        let sides = |pieces: &[(&str, i32)]| junction_of(pieces).current().arms[0].profile(0).curb_bike_sides();
        assert_eq!(sides(&BIKED), [Some(1), Some(4)]);
        assert_eq!(
            sides(&[
                ("sidewalk", 2000),
                ("parking", 2000),
                ("bike", 1500),
                ("travel", 3200),
                ("travel", 3200),
                ("bike", 1500),
                ("parking", 2000),
                ("sidewalk", 2000)
            ]),
            [None, None]
        );
        assert_eq!(sides(&[("sidewalk", 2000), ("bike", 1500), ("travel", 3200), ("travel", 3200), ("sidewalk", 2000)]), [Some(1), None]);
    }

    #[test]
    fn a_corner_needs_a_lane_at_the_curb_and_room_for_the_island() {
        let mut parked = junction_of(&[
            ("sidewalk", 2000),
            ("parking", 2000),
            ("bike", 1500),
            ("travel", 3200),
            ("travel", 3200),
            ("bike", 1500),
            ("parking", 2000),
            ("sidewalk", 2000),
        ]);
        let a = parked.current().arms[0].uid;
        assert!(!parked.set_protected(a, true), "the lane is outside parking, not at the curb");
        // a one-way 6.2 m street: the track and an island on each side would leave less than a lane
        let mut narrow = junction_of(&[("sidewalk", 2000), ("bike", 1500), ("travel", 3200), ("bike", 1500), ("sidewalk", 2000)]);
        let a = narrow.current().arms[0].uid;
        assert!(!narrow.set_protected(a, true));
        assert_eq!(narrow.refusal(), Some(Refusal::ProtectedNotHere));
        let mut wide = junction_of(&BIKED);
        let a = wide.current().arms[0].uid;
        assert!(wide.set_protected(a, true));
    }

    #[test]
    fn a_corner_is_protected_and_plain_again_and_undo_goes_back() {
        let mut j = junction_of(&BIKED);
        let a = j.current().arms[0].uid;
        let on = |j: &Junction| j.arm(a).unwrap().protected;
        assert!(!on(&j));
        assert!(j.set_protected(a, true));
        assert!(on(&j));
        assert!(!j.set_protected(a, true), "nothing changes");
        assert!(j.undo());
        assert!(!on(&j));
        assert!(j.redo());
        assert!(j.set_protected(a, false));
        assert!(!on(&j));
    }

    #[test]
    fn a_corner_needs_a_bike_lane_beside_the_curb_of_each_street_a_real_corner_and_no_roundabout() {
        let refused = |j: &mut Junction| {
            let a = j.current().arms[0].uid;
            assert!(!j.set_protected(a, true));
            assert_eq!(j.refusal(), Some(Refusal::ProtectedNotHere));
        };
        refused(&mut junction_of(&[("sidewalk", 2000), ("travel", 3200), ("travel", 3200), ("sidewalk", 2000)]));
        // a lane beside the left curb only (or the right only): the corner joins arm i's right curb and the next arm's left one
        refused(&mut junction_of(&[("sidewalk", 2000), ("bike", 1500), ("travel", 3200), ("travel", 3200), ("sidewalk", 2000)]));
        refused(&mut junction_of(&[("sidewalk", 2000), ("travel", 3200), ("travel", 3200), ("bike", 1500), ("sidewalk", 2000)]));
        let mut ring = junction_of(&BIKED);
        assert!(ring.set_control(ROUNDABOUT));
        refused(&mut ring);
        // a three-way: the gap from the arm at 180 to the arm at 0 is a straight line, the one from 0 to 90 is a corner
        let mut state = junction_of(&BIKED).current().clone();
        state.arms.retain(|a| a.bearing != 270);
        let mut t = Junction::from_city(named("Test"), &state, &state, 0).expect("draws");
        let (arm_0, arm_180) = (arm_at(&t, 0), arm_at(&t, 180));
        assert!(!t.set_protected(arm_180, true), "no corner to follow");
        assert_eq!(t.refusal(), Some(Refusal::ProtectedNotHere));
        assert!(t.set_protected(arm_0, true));
    }

    #[test]
    fn a_roundabout_takes_every_protected_corner_away_and_says_so_and_undo_brings_them_back() {
        let mut j = junction_of(&BIKED);
        for uid in j.current().arms.iter().map(|a| a.uid).collect::<Vec<_>>() {
            assert!(j.set_protected(uid, true));
        }
        assert!(j.set_control(ROUNDABOUT));
        assert!(j.current().arms.iter().all(|a| !a.protected));
        let said = j.revisions().last().cloned().unwrap();
        assert!(en(&said).ends_with("; protected corners removed"), "{}", en(&said));
        assert!(fr(&said).ends_with("; coins protégés retirés"), "{}", fr(&said));
        assert!(j.undo());
        assert!(j.current().arms.iter().all(|a| a.protected));
        let mut k = junction_of(&BIKED);
        for uid in k.current().arms.iter().map(|a| a.uid).collect::<Vec<_>>() {
            assert!(k.set_protected(uid, true) && k.set_bike_box(uid, true));
        }
        assert!(k.set_control(ROUNDABOUT));
        assert!(en(&k.revisions().last().cloned().unwrap()).ends_with("; bike boxes and protected corners removed"));
    }

    #[test]
    fn a_saved_protected_corner_that_no_longer_stands_is_dropped_on_load() {
        let mut state = junction_of(&BIKED).current().clone();
        let plain = street_of(&[("sidewalk", 2000), ("travel", 3200), ("travel", 3200), ("sidewalk", 2000)], 1);
        for a in &mut state.arms {
            a.protected = true;
            a.section = Some(plain.clone());
            a.edge = a.uid;
        }
        let j = Junction::from_city(named("Test"), &state, &state, 0).expect("draws");
        assert!(j.current().arms.iter().all(|a| !a.protected));
    }

    #[test]
    fn a_saved_state_without_a_turbo_loads_and_one_with_a_turbo_keeps_it() {
        let state = Junction::new(0).current().clone();
        let mut saved = serde_json::to_value(&state).unwrap();
        assert!(saved.get("turbo").is_none(), "no turbo, nothing written: an older build reads the record as it did");
        assert_eq!(serde_json::from_value::<State>(saved.clone()).unwrap().turbo, None, "a record made before turbos has none");

        let mut with = state.clone();
        with.turbo = Some(Turbo { layout: 1, size: 2, main: vec![1, 3] });
        saved = serde_json::to_value(&with).unwrap();
        assert_eq!(saved["turbo"], serde_json::json!({"layout": 1, "size": 2, "main": [1, 3]}));
        assert_eq!(serde_json::from_value::<State>(saved).unwrap().turbo, with.turbo, "the label and the arms' streets are not saved, the turbo is");
    }

    #[test]
    fn a_saved_arm_without_the_protected_field_loads() {
        let mut v = serde_json::to_value(Junction::new(0).current()).unwrap();
        assert_eq!(v["arms"][0]["protected"], serde_json::json!(false));
        v["arms"][0].as_object_mut().unwrap().remove("protected");
        let back: State = serde_json::from_value(v).expect("an older save loads");
        assert!(!back.arms[0].protected);
    }

    #[test]
    fn a_protected_corner_is_said_in_both_languages() {
        let mut j = junction_of(&BIKED);
        let a = j.current().arms[0].uid;
        assert!(j.set_protected(a, true));
        let said = j.revisions().last().cloned().unwrap();
        assert!(en(&said).ends_with("protected: add"), "{}", en(&said));
        assert!(fr(&said).ends_with("protégé\u{a0}: ajout"), "{}", fr(&said));
    }

    #[test]
    fn a_bike_box_on_an_unknown_street_is_refused_as_any_edit_of_one_is() {
        let mut j = junction_of(&BIKED);
        assert!(!j.set_bike_box(9_999, true));
        assert_eq!(j.refusal(), Some(Refusal::DoesNotFit));
    }

    #[test]
    fn a_saved_bike_box_is_dropped_where_no_lane_comes_in_and_on_a_roundabout() {
        let mut state =
            junction_reading(&street_of(&[("sidewalk", 2000), ("bike", 1500), ("travel", 3200), ("bike", 1500), ("sidewalk", 2000)], 0)).current().clone();
        for a in &mut state.arms {
            a.bike_box = true;
        }
        let j = Junction::from_city(named("Test"), &state, &state, 0).expect("draws");
        assert!(j.current().arms.iter().all(|a| !a.bike_box), "no lane comes in");

        let mut ring = junction_of(&BIKED).current().clone();
        ring.control = ROUNDABOUT;
        for a in &mut ring.arms {
            a.bike_box = true;
            a.edge = a.uid;
        }
        let mut j = Junction::from_city(named("Test"), &ring, &ring, 0).expect("draws");
        assert!(j.current().arms.iter().all(|a| !a.bike_box), "a roundabout has none");
        assert!(j.set_control(SIGNAL));
        assert!(j.current().arms.iter().all(|a| !a.bike_box), "and none comes back when it is changed");
    }

    #[test]
    fn a_saved_arm_without_the_bike_box_field_loads() {
        let mut v = serde_json::to_value(Junction::new(0).current()).unwrap();
        assert_eq!(v["arms"][0]["bike_box"], serde_json::json!(false));
        v["arms"][0].as_object_mut().unwrap().remove("bike_box");
        let back: State = serde_json::from_value(v).expect("an older save loads");
        assert!(!back.arms[0].bike_box);
    }

    #[test]
    fn a_bike_box_is_said_in_both_languages() {
        let mut j = junction_of(&BIKED);
        let a = j.current().arms[0].uid;
        assert!(j.set_bike_box(a, true));
        let said = j.revisions().last().cloned().unwrap();
        assert!(en(&said).ends_with("bike box: add"), "{}", en(&said));
        assert!(fr(&said).ends_with("sas cyclable\u{a0}: ajout"), "{}", fr(&said));
    }

    #[test]
    fn every_refusal_has_a_message_in_both_languages() {
        for locale in Locale::ALL {
            let i18n = crate::i18n_for(locale);
            for r in Refusal::ALL {
                assert!(i18n.has_message(r.key(), locale), "{r:?} in {}", locale.tag());
                assert!(i18n.tr_now(r.key(), &crate::shared::i18n::Args::new()).ends_with('.'), "{r:?} in {}", locale.tag());
            }
        }
        assert_eq!(en(&Said::new(Refusal::DoesNotFit.key())), "That change does not fit.");
    }

    #[test]
    fn every_control_and_compass_point_has_a_message_in_both_languages() {
        for locale in Locale::ALL {
            let i18n = crate::i18n_for(locale);
            for i in 0..CONTROLS.len() {
                assert!(i18n.has_message(control_key(i), locale), "{} in {}", control_key(i), locale.tag());
            }
            for b in (0..360).step_by(45) {
                assert!(i18n.has_message(compass(b).key, locale), "{b} in {}", locale.tag());
            }
            for item in APPROACHES.iter().chain(&STOPS).chain(&RULES) {
                assert!(i18n.has_message(item.key, locale), "{} in {}", item.key, locale.tag());
            }
        }
        assert_eq!(en(&Said::new(control_key(SIGNAL))), "Traffic signal");
        assert_eq!(fr(&Said::new(control_key(ROUNDABOUT))), "Carrefour giratoire");
        assert_eq!((en(&compass(45)), fr(&compass(225))), ("north-east".into(), "sud-ouest".into()));
    }

    #[test]
    fn what_each_edit_was_is_said_as_it_was_in_english_and_also_in_french() {
        let last = |j: &Junction| j.revisions().last().cloned().expect("the edit was taken");
        let mut table: Vec<(Said, &str)> = Vec::new();
        let mut j = Junction::new(0);
        let (n, e, s) = (arm_at(&j, 0), arm_at(&j, 90), arm_at(&j, 180));
        assert!(j.set_corner(n, 7_000));
        table.push((last(&j), "Corner after Sample Avenue 2 (north): 7.0 m radius"));
        assert!(j.set_control(ALL_WAY_STOP));
        table.push((last(&j), "Control: all-way stop"));
        assert!(j.set_turn(n, e, false));
        table.push((last(&j), "Sample Avenue 2 (north) to Sample Street 1 (east): no turn"));
        assert!(j.set_island(e, true));
        table.push((last(&j), "Sample Street 1 (east) refuge island: add"));
        assert!(j.set_bulb(e, 0, true));
        table.push((last(&j), "Sample Street 1 (east) left bulb-out: add"));
        assert!(j.set_crossing(e, false));
        table.push((last(&j), "Sample Street 1 (east) crossing: remove"));
        let lane_to = j.arm(n).unwrap().lanes[0].to[0];
        assert!(j.set_lane_dest(n, 0, lane_to, false));
        table.push((last(&j), "Sample Avenue 2 (north) lane 1: not to Sample Street 1 (east)"));
        assert!(j.set_approach(s, Q_CURB));
        table.push((last(&j), "Sample Avenue 2 (south): G2 curbside queue-jump lane"));
        assert!(j.remove_arm(e));
        table.push((last(&j), "Remove Sample Street 1 (east)"));
        let mut k = Junction::new(0);
        assert!(k.set_control(ROUNDABOUT));
        assert!(k.set_ring(1_500));
        table.push((last(&k), "Roundabout: 1.5 m larger"));
        assert!(k.set_bearing(arm_at(&k, 90), 95));
        table.push((last(&k), "Sample Street 1 bearing: 95°"));
        for (said, english) in &table {
            assert_eq!(en(said), *english);
            let french = fr(said);
            assert_ne!(french, *english, "{english} is said in French too");
            for word in ["Corner", "Control", "Remove", "Roundabout", "bearing", " to ", "add", "remove", "lane", "no turn", "north", "east"] {
                assert!(!french.contains(word), "{word:?} in {french:?}");
            }
        }
        assert_eq!(fr(&table[0].0), "Coin après Sample Avenue 2 (nord)\u{a0}: rayon de 7,0\u{a0}m");
    }

    #[test]
    fn a_junction_keeps_three_streets() {
        let mut j = Junction::new(1);
        let uid = j.current().arms[0].uid;
        assert!(!j.remove_arm(uid));
        assert_eq!(j.refusal(), Some(Refusal::NeedsThreeStreets));
    }

    #[test]
    fn the_streets_of_a_city_junction_cannot_be_removed() {
        let mut j = linked(1); // three streets: removal is also below the least
        let uid = j.current().arms[0].uid;
        assert!(!j.remove_arm(uid));
        assert_eq!(j.refusal(), Some(Refusal::LinkedNoRemove));
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
        let lane = j.current().arms.iter().find(|a| sample_of(a) == 2).unwrap().uid;
        assert!(!j.set_island(lane, true));
        assert_eq!(j.refusal(), Some(Refusal::IslandRoadTooNarrow));
        assert!(!j.set_bulb(lane, 0, true));
        assert_eq!(j.refusal(), Some(Refusal::BulbNoParking));
    }

    #[test]
    fn an_edit_that_cannot_be_drawn_is_refused_with_the_reason_it_was_given() {
        let mut j = Junction::new(0);
        // Two streets on one bearing cannot be drawn.
        assert!(!j.edit_why(Refusal::RoundaboutTooBig, earlier(), |s| {
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
            ("continuous without a crossing", Box::new(move |j| j.set_crossing(n, false) && j.set_continuous(n, true))),
            ("protected on a roundabout", Box::new(move |j| j.set_control(ROUNDABOUT) && j.set_protected(n, true))),
            ("bike box on a roundabout", Box::new(move |j| j.set_control(ROUNDABOUT) && j.set_bike_box(n, true))),
            ("raised on a roundabout", Box::new(|j| j.set_control(ROUNDABOUT) && j.set_raised(true))),
            ("cycle width", Box::new(|j| j.set_cycle(Some(99_000)))),
            ("ring", Box::new(|j| j.set_ring(-500))),
            ("lane on a turbo", Box::new(move |j| j.set_control(ROUNDABOUT) && j.set_turbo(Some(0)) && j.set_lane_dest(n, 0, e, true))),
            ("bus lane on a turbo", Box::new(move |j| j.set_control(ROUNDABOUT) && j.set_turbo(Some(0)) && j.set_bus(Some((n, e))))),
            ("queue jump on a turbo", Box::new(move |j| j.set_control(ROUNDABOUT) && j.set_turbo(Some(0)) && j.set_approach(n, Q_OFFSET))),
            ("turbo entry too narrow", Box::new(|j| j.set_control(ROUNDABOUT) && j.set_turbo(Some(4)))),
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

    // ---- the signal plan as a list of phases ----

    fn phase_uids(j: &Junction) -> Vec<u32> {
        signal::plan(j.current(), 0).unwrap().phases.iter().map(|p| p.uid).collect()
    }

    #[test]
    fn a_junction_nobody_edited_shows_the_suggestion_and_stores_no_plan() {
        let j = Junction::new(0);
        let p = signal::plan(j.current(), 0).unwrap();
        assert!(p.suggested);
        assert_eq!(p.phases.len(), 4);
        assert!(p.phases.iter().all(|ph| !ph.moves.is_empty()), "each phase says the movements it serves");
        assert!(j.current().signal_plan.is_none());
        assert!(serde_json::to_value(j.current()).unwrap().get("signal_plan").is_none(), "nothing is written for it");
    }

    #[test]
    fn removing_a_phase_keeps_the_edited_plan_in_the_junction_and_undo_brings_the_suggestion_back() {
        let mut j = Junction::new(0);
        let before = phase_uids(&j);
        assert!(j.remove_phase(1));
        let p = signal::plan(j.current(), 0).unwrap();
        assert!(!p.suggested);
        assert_eq!(p.phases.iter().map(|p| p.uid).collect::<Vec<_>>(), [before[0], before[2], before[3]]);
        assert!(j.current().signal_plan.is_some());
        assert!(plain(&en(j.revisions().last().unwrap())).contains("Phase 2 for"), "{}", en(j.revisions().last().unwrap()));
        assert!(j.undo());
        assert!(signal::plan(j.current(), 0).unwrap().suggested);
        assert_eq!(phase_uids(&j), before);
        assert!(j.redo());
        assert_eq!(phase_uids(&j).len(), 3);
    }

    #[test]
    fn the_last_phase_stays() {
        let mut j = Junction::new(0);
        assert!(j.remove_phase(0) && j.remove_phase(0) && j.remove_phase(0));
        assert!(!j.remove_phase(0));
        assert_eq!(j.refusal(), Some(Refusal::LastPhase));
        assert_eq!(phase_uids(&j).len(), 1);
        assert!(!j.remove_phase(9));
        assert_eq!(j.refusal(), Some(Refusal::DoesNotFit));
    }

    #[test]
    fn a_phase_moves_earlier_or_later_and_not_past_the_ends() {
        let mut j = Junction::new(0);
        let [a, b, c, d] = phase_uids(&j)[..] else { panic!("four phases") };
        assert!(j.move_phase(2, -1));
        assert_eq!(phase_uids(&j), [a, c, b, d]);
        assert!(plain(&en(j.revisions().last().unwrap())).contains("moved earlier, now 2 of 4"));
        assert!(j.move_phase(0, 1));
        assert_eq!(phase_uids(&j), [c, a, b, d]);
        assert!(plain(&en(j.revisions().last().unwrap())).contains("moved later, now 2 of 4"));
        assert!(!j.move_phase(0, -1));
        assert_eq!(j.refusal(), Some(Refusal::PhaseAtEnd));
        assert!(!j.move_phase(3, 1));
        assert_eq!(j.refusal(), Some(Refusal::PhaseAtEnd));
        assert!(!j.move_phase(7, 1));
        assert_eq!(j.refusal(), Some(Refusal::DoesNotFit));
        assert!(j.undo() && j.undo());
        assert_eq!(phase_uids(&j), [a, b, c, d]);
    }

    #[test]
    fn a_removed_street_can_be_given_a_phase_again_last() {
        let mut j = Junction::new(0);
        let first = phase_uids(&j)[0];
        assert!(j.remove_phase(0));
        assert!(j.add_phase(first));
        assert_eq!(*phase_uids(&j).last().unwrap(), first);
        assert!(plain(&en(j.revisions().last().unwrap())).contains("Phase 4 added for"));
        assert!(j.add_phase(first), "a street may have a second phase");
        assert_eq!(phase_uids(&j).len(), 5);
        assert!(!j.add_phase(9_999));
        assert_eq!(j.refusal(), Some(Refusal::DoesNotFit));
    }

    #[test]
    fn a_street_nothing_enters_from_has_no_phase_to_add() {
        let j = Junction::new(0);
        let n = arm_at(&j, 0);
        let mut s = j.current().clone();
        assert!(signal::phase_for(&s, 0, n).is_some());
        s.arms.iter_mut().find(|a| a.uid == n).unwrap().lanes.clear();
        assert!(signal::phase_for(&s, 0, n).is_none());
    }

    #[test]
    fn the_plan_can_be_restored_to_the_suggestion_but_not_when_it_is_one() {
        let mut j = Junction::new(0);
        assert!(!j.restore_suggestion());
        assert_eq!(j.refusal(), Some(Refusal::PlanSuggested));
        let all = phase_uids(&j);
        assert!(j.move_phase(0, 1));
        assert!(j.restore_suggestion());
        assert!(signal::plan(j.current(), 0).unwrap().suggested);
        assert_eq!(phase_uids(&j), all);
        assert!(plain(&en(j.revisions().last().unwrap())).contains("restored to the suggestion"));
        assert!(j.undo());
        assert!(!signal::plan(j.current(), 0).unwrap().suggested, "undo brings the edited plan back");
    }

    #[test]
    fn the_plan_is_edited_only_at_a_signal() {
        let mut j = Junction::new(1);
        j.set_control(2);
        for ok in [j.remove_phase(0), j.move_phase(0, 1), j.restore_suggestion(), j.add_phase(arm_at(&j, 90))] {
            assert!(!ok);
            assert_eq!(j.refusal(), Some(Refusal::NotSignal));
        }
    }

    #[test]
    fn a_plan_holds_at_most_two_phases_for_each_possible_street() {
        let mut j = Junction::new(0);
        let uid = j.current().arms[0].uid;
        while j.add_phase(uid) {}
        assert_eq!(j.refusal(), Some(Refusal::TooManyPhases));
        assert_eq!(signal::plan(j.current(), 0).unwrap().phases.len(), MAX_ARMS * 2);
    }

    #[test]
    fn a_movement_that_still_exists_stays_in_the_plan_whatever_its_lane_is_numbered() {
        let mut j = Junction::new(0);
        assert!(j.remove_phase(3));
        let pairs = |j: &Junction| -> Vec<(u32, u32)> {
            let mut v: Vec<(u32, u32)> = signal::plan(j.current(), 0).unwrap().phases.iter().flat_map(|p| &p.moves).map(|m| (m.from, m.to)).collect();
            v.sort_unstable();
            v.dedup();
            v
        };
        let lanes_of = |j: &Junction| -> Vec<(u32, u32)> {
            j.current().arms.iter().flat_map(|a| a.lanes.iter().flat_map(move |l| l.to.iter().map(move |t| (a.uid, *t)))).collect()
        };
        let before = pairs(&j);
        // The lanes of the first arm are renumbered: its lane order is reversed, each still going where it went.
        let uid = arm_at(&j, 0);
        let mut s = j.current().clone();
        let arm = s.arms.iter_mut().find(|a| a.uid == uid).unwrap();
        assert!(arm.lanes.len() > 1, "the sample's first arm has more than one lane");
        arm.lanes.reverse();
        signal::settle(&mut s);
        let mut j2 = Junction::new(0);
        assert!(j2.remove_phase(3));
        *j2.current_mut() = s;
        let after = pairs(&j2);
        let still: Vec<(u32, u32)> = before.iter().copied().filter(|p| lanes_of(&j2).contains(p)).collect();
        assert_eq!(after, still, "every movement a lane still serves stays in the plan");
    }

    #[test]
    fn leaving_the_signal_drops_the_edited_plan_and_undo_brings_it_back() {
        let mut j = Junction::new(0);
        assert!(j.remove_phase(0));
        assert!(j.set_control(PRIORITY));
        assert!(j.current().signal_plan.is_none());
        assert!(plain(&en(j.revisions().last().unwrap())).contains("edited signal plan removed"), "the step says so");
        assert!(j.undo());
        assert!(j.current().signal_plan.is_some());
    }

    #[test]
    fn restoring_the_suggestion_does_not_say_the_plan_was_removed() {
        let mut j = Junction::new(0);
        assert!(j.remove_phase(0));
        assert!(j.restore_suggestion());
        assert!(!plain(&en(j.revisions().last().unwrap())).contains("removed"));
    }

    #[test]
    fn a_lane_that_no_longer_goes_where_a_phase_served_it_is_dropped_from_the_plan() {
        let mut j = Junction::new(0);
        assert!(j.remove_phase(3));
        let n = arm_at(&j, 0);
        let to = j.arm(n).unwrap().lanes[0].to.clone();
        assert!(to.len() > 1);
        assert!(j.set_lane_dest(n, 0, to[0], false));
        let p = signal::plan(j.current(), 0).unwrap();
        assert!(p.phases.iter().flat_map(|p| &p.moves).all(|m| !(m.from == n && m.to == to[0])), "the served movement is gone");
    }

    #[test]
    fn the_plan_round_trips_and_an_old_record_loads_as_suggested() {
        let mut j = Junction::new(0);
        let old: serde_json::Value = serde_json::to_value(j.current()).unwrap();
        let back: State = serde_json::from_value(old).unwrap();
        assert_eq!(back.signal_plan, None, "a record from before the plan reads as suggested");
        assert!(j.move_phase(0, 1));
        let v = serde_json::to_value(j.current()).unwrap();
        assert!(v["signal_plan"]["phases"].as_array().unwrap().len() == 4);
        let back: State = serde_json::from_value(v).unwrap();
        assert_eq!(back.signal_plan, j.current().signal_plan);
    }

    // ---- ready-made phase patterns ----

    fn phase_of(j: &Junction, index: usize) -> signal::Phase {
        signal::plan(j.current(), 0).unwrap().phases[index].clone()
    }

    #[test]
    fn choosing_a_pattern_is_one_undoable_edit_that_changes_what_the_phase_serves() {
        let mut j = Junction::new(0);
        let n = phase_of(&j, 0).uid;
        let all = phase_of(&j, 0).moves.len();
        assert!(all > 1);
        assert!(j.set_phase_pattern(0, Pattern::ThroughOnly));
        let p = phase_of(&j, 0);
        assert!(!p.moves.is_empty() && p.moves.len() < all);
        assert!(p.moves.iter().all(|m| m.from == n && turn_class(arm_bearing(&j, n), arm_bearing(&j, m.to)) == THROUGH));
        assert!(!signal::plan(j.current(), 0).unwrap().suggested, "the first edit keeps the plan as the person's own");
        assert_eq!(
            en(j.revisions().last().unwrap()).replace(['\u{a0}', '\u{202f}'], " ").contains("through only"),
            true,
            "{}",
            en(j.revisions().last().unwrap())
        );
        assert!(fr(j.revisions().last().unwrap()).contains("tout droit"), "{}", fr(j.revisions().last().unwrap()));
        assert!(j.undo());
        assert!(signal::plan(j.current(), 0).unwrap().suggested);
        assert_eq!(phase_of(&j, 0).moves.len(), all);
        assert!(j.redo());
        assert_eq!(signal::pattern_of(j.current(), 0, &phase_of(&j, 0)), Pattern::ThroughOnly);
    }

    fn arm_bearing(j: &Junction, uid: u32) -> i32 {
        j.arm(uid).unwrap().bearing
    }

    #[test]
    fn a_scramble_keeps_its_street_and_timing_and_serves_no_vehicle() {
        let mut j = Junction::new(0);
        let n = phase_of(&j, 0).uid;
        assert!(j.set_timing(n, TimingField::Green, 44));
        assert!(j.set_phase_pattern(0, Pattern::Scramble));
        let p = phase_of(&j, 0);
        assert!(p.moves.is_empty() && p.uid == n);
        assert_eq!(p.timing.green_s, 44);
        assert_eq!(p.walks.len(), j.current().arms.iter().filter(|a| a.crossing.is_some()).count(), "every crossing walks");
        let back: State = serde_json::from_value(serde_json::to_value(j.current()).unwrap()).unwrap();
        assert_eq!(back.signal_plan, j.current().signal_plan, "kept and read back");
        assert!(j.set_phase_pattern(0, Pattern::All), "a scramble can go back to the street's movements");
        assert!(!phase_of(&j, 0).moves.is_empty());
    }

    #[test]
    fn a_pattern_that_does_not_fit_or_changes_nothing_is_refused_with_a_reason() {
        let mut j = Junction::new(0);
        assert!(!j.set_phase_pattern(0, Pattern::All));
        assert_eq!(j.refusal(), Some(Refusal::PatternAlready), "the suggestion is already all movements");
        assert!(!j.set_phase_pattern(0, Pattern::Custom));
        assert_eq!(j.refusal(), Some(Refusal::PatternNotFit));
        assert!(!j.set_phase_pattern(9, Pattern::Scramble));
        assert_eq!(j.refusal(), Some(Refusal::DoesNotFit));
        assert!(signal::plan(j.current(), 0).unwrap().suggested, "nothing was changed");
        // North goes only straight on: there is no left to protect.
        let n = phase_of(&j, 0).uid;
        let turns: Vec<u32> =
            j.arm(n).unwrap().lanes.iter().flat_map(|l| l.to.clone()).filter(|&t| turn_class(arm_bearing(&j, n), arm_bearing(&j, t)) != THROUGH).collect();
        for t in turns {
            for lane in 0..j.arm(n).unwrap().lanes.len() {
                j.set_lane_dest(n, lane, t, false);
            }
        }
        assert!(!j.set_phase_pattern(0, Pattern::ProtectedLeftThenThrough));
        assert_eq!(j.refusal(), Some(Refusal::PatternNotFit));
        let mut k = Junction::new(1);
        assert_ne!(k.current().control, SIGNAL);
        assert!(!k.set_phase_pattern(0, Pattern::Scramble));
        assert_eq!(k.refusal(), Some(Refusal::NotSignal));
    }

    // ---- signal timing ----

    /// What was said, with the engine's non-breaking spaces (before a colon, before a unit) as ordinary ones.
    fn plain(said: &str) -> String {
        said.replace(['\u{a0}', '\u{202f}'], " ")
    }

    #[test]
    fn an_arm_never_timed_writes_no_timing_and_a_timed_one_round_trips() {
        let mut j = Junction::new(0);
        let uid = j.current().arms[0].uid;
        let plain = serde_json::to_value(&j.current().arms[0]).unwrap();
        assert!(plain.get("timing").is_none(), "nothing is written for an arm never timed");
        let back: Arm = serde_json::from_value(plain).unwrap();
        assert_eq!(back.timing, None);
        assert!(j.set_timing(uid, TimingField::Green, 30));
        let timed = serde_json::to_value(j.arm(uid).unwrap()).unwrap();
        assert_eq!(timed["timing"]["green_s"], 30);
        let back: Arm = serde_json::from_value(timed).unwrap();
        assert_eq!(back.timing, Some(Timing { green_s: 30, ..Timing::default() }));
    }

    #[test]
    fn the_four_numbers_are_set_and_stepped_within_their_limits() {
        let mut j = Junction::new(0);
        let uid = j.current().arms[0].uid;
        assert!(j.set_bus_lane(uid, true));
        for field in TimingField::ALL {
            let (lo, hi) = field.range();
            assert!(j.set_timing(uid, field, hi), "{field:?} takes its top");
            assert_eq!(field.get(&j.arm(uid).unwrap().timing.unwrap()) as i32, hi);
            assert!(!j.step_timing(uid, field, 1), "{field:?} does not go past its top");
            assert_eq!(j.refusal(), Some(Refusal::TimingOutOfRange));
            assert!(j.set_timing(uid, field, lo), "{field:?} takes its bottom");
            assert!(!j.step_timing(uid, field, -1), "{field:?} does not go under its bottom");
            assert!(j.step_timing(uid, field, 1));
            assert_eq!(field.get(&j.arm(uid).unwrap().timing.unwrap()) as i32, lo + 1);
        }
        assert!(!j.set_timing(uid, TimingField::Green, 4) && !j.set_timing(uid, TimingField::Green, 121));
        assert!(!j.set_timing(uid, TimingField::Green, -5) && !j.set_timing(uid, TimingField::Green, i32::MAX));
    }

    #[test]
    fn timing_needs_a_signal_and_an_early_green_needs_a_transit_lane() {
        let mut j = Junction::new(1); // street and lane: priority control
        let uid = j.current().arms[0].uid;
        assert!(!j.set_timing(uid, TimingField::Green, 30));
        assert_eq!(j.refusal(), Some(Refusal::NotSignal));
        assert!(j.set_control(SIGNAL));
        assert!(!j.set_timing(uid, TimingField::TransitLead, 5));
        assert_eq!(j.refusal(), Some(Refusal::NoTransitLane));
        assert!(j.set_bus_lane(uid, true));
        assert!(j.set_timing(uid, TimingField::TransitLead, 5));
        assert!(!j.set_timing(9_999, TimingField::Green, 30), "no such street");
    }

    #[test]
    fn an_edit_is_one_step_of_history_said_in_both_languages_and_undone() {
        let mut j = Junction::new(0);
        let uid = j.current().arms[0].uid;
        assert!(j.set_timing(uid, TimingField::Green, 30));
        let said = j.revisions().last().unwrap().clone();
        assert!(plain(&en(&said)).contains("green 30 s"), "{}", en(&said));
        assert!(plain(&fr(&said)).contains("vert 30 s"), "{}", fr(&said));
        assert!(j.undo());
        assert_eq!(j.arm(uid).unwrap().timing, None);
        assert!(j.redo());
        assert_eq!(j.arm(uid).unwrap().timing.unwrap().green_s, 30);
        assert!(j.set_timing(uid, TimingField::Green, 30), "the same value is not an error");
    }

    #[test]
    fn leaving_the_signal_clears_the_timing_says_so_and_undo_brings_it_back() {
        let mut j = Junction::new(0);
        let uid = j.current().arms[0].uid;
        assert!(j.set_timing(uid, TimingField::Green, 30));
        assert!(j.set_control(PRIORITY));
        assert_eq!(j.arm(uid).unwrap().timing, None);
        assert!(plain(&en(j.revisions().last().unwrap())).contains("signal timing removed"));
        assert!(plain(&fr(j.revisions().last().unwrap())).contains("réglage des feux retiré"));
        assert!(j.undo());
        assert_eq!(j.arm(uid).unwrap().timing.unwrap().green_s, 30);
    }

    #[test]
    fn losing_the_transit_lane_loses_the_early_green() {
        let mut j = Junction::new(0);
        let uid = j.current().arms[0].uid;
        assert!(j.set_bus_lane(uid, true));
        assert!(j.set_timing(uid, TimingField::TransitLead, 8));
        assert!(j.set_bus_lane(uid, false));
        assert_eq!(j.arm(uid).unwrap().timing.unwrap().transit_lead_s, 0);
    }

    // ---- choosing a turbo ----

    /// The Avenue and street (sample 0) as a roundabout: uids 1 and 3 are the avenue (north, south), 2 and 4 the street (east, west).
    fn roundabout() -> Junction {
        let mut j = Junction::new(0);
        assert!(j.set_control(ROUNDABOUT));
        j
    }

    #[test]
    fn on_a_turbo_the_layout_sets_where_each_lane_goes_and_no_bus_lane_or_queue_jump_is_taken() {
        let mut j = roundabout();
        assert!(j.set_turbo(Some(0)));
        let (n, e, so, w) = (arm_at(&j, 0), arm_at(&j, 90), arm_at(&j, 180), arm_at(&j, 270));
        let on_lane = j.arm(n).unwrap().lanes[0].to[0];
        assert!(!j.set_lane_dest(n, 0, w, true), "a lane that does not go there cannot be sent there");
        assert_eq!(j.refusal(), Some(Refusal::TurboLanesFixed));
        assert!(!j.set_lane_dest(n, 0, on_lane, false), "nor one that does be kept from it");
        assert_eq!(j.refusal(), Some(Refusal::TurboLanesFixed));
        assert!(!j.set_bus(Some((n, so))));
        assert_eq!(j.refusal(), Some(Refusal::BusOnTurbo));
        for q in [Q_OFFSET, Q_CURB, Q_VIRTUAL] {
            assert!(!j.set_approach(e, q));
            assert_eq!(j.refusal(), Some(Refusal::QueueJumpOnTurbo));
        }
        assert!(j.set_approach(e, GATE_YIELD), "a gate set back upstream is allowed");
        assert!(j.set_approach(e, 0));
        assert!(j.set_bus_lane(e, true), "a transit lane along the curb is not the bus lane across the island");
    }

    #[test]
    fn the_ring_size_of_a_turbo_is_its_block_not_a_length() {
        let mut j = roundabout();
        assert!(j.set_turbo(Some(0)));
        assert!(!j.set_ring(500));
        assert_eq!(j.refusal(), Some(Refusal::DoesNotFit));
        assert!(!j.set_ring_radius(30_000));
        assert!(j.step_ring(1), "the step controls of the ring step the block");
        assert_eq!(j.current().turbo.as_ref().unwrap().size, 1);
        assert!(j.step_ring(-1));
        assert_eq!(j.current().turbo.as_ref().unwrap().size, 0);
    }

    #[test]
    fn a_turbo_offers_no_bus_lane_across_its_island() {
        let mut j = roundabout();
        assert!(!j.view().bus_options.is_empty());
        assert!(j.set_turbo(Some(0)));
        assert!(j.view().bus_options.is_empty());
    }

    /// Four equal narrow streets of a city, as a roundabout: any block fits them.
    fn narrow_roundabout() -> Junction {
        let mut j = junction_of(&[("sidewalk", 3_000), ("travel", 3_500), ("travel", 3_500), ("sidewalk", 3_000)]);
        assert!(j.set_control(ROUNDABOUT));
        j
    }

    #[test]
    fn a_turbo_is_chosen_for_a_roundabout_with_its_main_road_and_the_smallest_block_that_fits() {
        let mut j = roundabout();
        assert!(j.set_turbo(Some(0)));
        assert_eq!(j.current().turbo, Some(Turbo { layout: 0, size: 0, main: vec![1, 3] }), "the avenue is the main road; the streets fit the mini block");
        assert_eq!(j.current().ring_extra_mm, 0);
        let label = j.revisions().last().unwrap().clone();
        assert_eq!(en(&label), "Roundabout layout: basic turbo");
        assert_eq!(fr(&label), "Disposition du giratoire\u{a0}: turbo de base");
        assert!(j.undo());
        assert_eq!(j.current().turbo, None);
        assert!(j.redo());
        assert!(j.current().turbo.is_some());
    }

    #[test]
    fn the_default_block_is_the_smallest_whose_ring_fits_the_streets() {
        let mut j = junction_of(&[("travel", 3_500); 8]);
        assert!(j.set_control(ROUNDABOUT));
        assert!(j.set_turbo(Some(0)));
        assert_eq!(j.current().turbo.as_ref().unwrap().size, 2, "mini and regular are too small for streets 28 m wide: medium");
    }

    #[test]
    fn a_layout_is_offered_by_arm_count_and_only_for_a_roundabout() {
        let mut three = Junction::new(1);
        assert!(three.set_control(ROUNDABOUT));
        assert!(!three.set_turbo(Some(0)), "basic needs four streets");
        assert_eq!(three.refusal(), Some(Refusal::DoesNotFit));
        assert!(three.set_turbo(Some(1)), "an egg suits three");
        assert_eq!(three.current().turbo.as_ref().unwrap().main, [1, 3], "the through road");
        assert!(three.set_turbo(Some(2)), "and so does a knee, which keeps the main road");
        assert_eq!(three.current().turbo.as_ref().unwrap().layout, 2);
        assert!(!three.set_turbo(Some(9)), "there are five layouts");
        let mut signal = Junction::new(0);
        assert!(!signal.set_turbo(Some(0)), "a signal is not a roundabout");
        assert_eq!(signal.refusal(), Some(Refusal::DoesNotFit));
        assert!(!signal.set_turbo(None), "and has no turbo to take away");
    }

    #[test]
    fn a_junction_with_no_two_opposite_streets_is_offered_no_layout() {
        let mut y = Junction::new(1);
        let (a, b) = (y.current().arms[1].uid, y.current().arms[2].uid);
        assert!(y.set_bearing(a, 210) && y.set_bearing(b, 330), "three streets 120 degrees apart");
        assert!(y.set_control(ROUNDABOUT));
        for layout in [1, 2] {
            assert!(!y.set_turbo(Some(layout)));
        }
        assert_eq!(y.refusal(), Some(Refusal::DoesNotFit));
    }

    #[test]
    fn each_layout_brings_its_entry_lanes_and_sets_where_they_go() {
        let counts = |j: &Junction| j.current().arms.iter().map(|a| a.lanes.len()).collect::<Vec<_>>();
        let to = |j: &Junction, uid: u32| j.arm(uid).unwrap().lanes.iter().map(|l| l.to.clone()).collect::<Vec<_>>();
        let mut j = roundabout();
        assert!(j.set_turbo(Some(0)));
        assert_eq!(counts(&j), [2, 2, 2, 2], "basic");
        assert_eq!(to(&j, 1), [vec![2, 3], vec![3, 4]], "north: the left lane goes left and through, the right lane through and right");
        assert_eq!(to(&j, 2), [vec![3, 4], vec![1]], "east, a side street: its right lane turns right only");
        assert!(j.set_turbo(Some(1)));
        assert_eq!(counts(&j), [2, 1, 2, 1], "egg: two on the main road, one on the others");
        assert_eq!(to(&j, 2), [vec![1, 3, 4]], "a single lane serves every turn");
        assert!(j.set_turbo(Some(2)));
        assert_eq!(counts(&j), [2, 1, 1, 1], "knee: two on the first main arm only");
        assert!(j.set_turbo(Some(3)));
        assert_eq!(counts(&j), [3, 2, 3, 2], "spiral: three on the Avenue, two on the Street");
        assert_eq!(to(&j, 1), [vec![2, 3], vec![3], vec![3, 4]], "the middle lane goes through only");
        assert_eq!(j.current().turbo.as_ref().unwrap().size, 0, "all of them fit the mini block here");
    }

    #[test]
    fn a_side_street_s_outermost_lane_turns_only_toward_the_outside_and_a_main_street_s_lanes_are_unchanged() {
        let to = |j: &Junction, uid: u32| j.arm(uid).unwrap().lanes.iter().map(|l| l.to.clone()).collect::<Vec<_>>();
        // Uids 1, 2, 3, 4 are north, east, south, west; the Avenue (north, south) is the main road.
        let mut j = roundabout();
        assert!(j.set_turbo(Some(0)));
        assert_eq!(to(&j, 1), [vec![2, 3], vec![3, 4]], "basic, north (main): as before");
        assert_eq!(to(&j, 3), [vec![1, 4], vec![1, 2]], "south (main): as before");
        assert_eq!(to(&j, 2), [vec![3, 4], vec![1]], "east (side): the right lane turns right only");
        assert_eq!(to(&j, 4), [vec![1, 2], vec![3]], "west (side)");
        assert!(j.set_turbo(Some(3)));
        assert_eq!(to(&j, 1), [vec![2, 3], vec![3], vec![3, 4]], "spiral, north (main): as before");
        assert_eq!(to(&j, 2), [vec![3, 4], vec![1]], "east (side, two lanes)");
        // Left-hand traffic: the outside is the driver's left, and the first lane is the outermost.
        let mut left = Junction::new(0);
        assert!(left.set_control(ROUNDABOUT) && left.set_region(3) && left.set_turbo(Some(0)));
        assert_eq!(to(&left, 1), [vec![2, 3], vec![3, 4]], "left-hand, north (main): as before");
        assert_eq!(to(&left, 2), [vec![3], vec![1, 4]], "left-hand, east (side): the left lane turns left only");
        // A one-lane entry keeps every turn (UNVERIFIED).
        let mut egg = roundabout();
        assert!(egg.set_turbo(Some(1)));
        assert_eq!(to(&egg, 2), [vec![1, 3, 4]]);
    }

    #[test]
    fn a_rotor_brings_three_lanes_on_every_street_that_has_the_room_for_them() {
        // Four freeways (sample 3): each is entered by three lanes of 3.6 m, 10.8 m, more than the 10.5 m a rotor wants, so nothing is widened.
        let mut state = Junction::new(0).current().clone();
        for a in &mut state.arms {
            a.section = Some(Street::sample(3, Side::Right));
            a.edge = a.uid;
        }
        state.control = ROUNDABOUT;
        let mut j = Junction::from_city(named("Test"), &state, &state, 0).expect("a roundabout of freeways draws");
        assert_eq!(j.current().control, ROUNDABOUT);
        assert!(j.set_turbo(Some(4)));
        assert_eq!(j.current().arms.iter().map(|a| a.lanes.len()).collect::<Vec<_>>(), [3, 3, 3, 3], "rotor");
        assert_eq!(
            j.arm(1).unwrap().lanes.iter().map(|l| l.to.clone()).collect::<Vec<_>>(),
            [vec![2, 3], vec![3], vec![3, 4]],
            "the middle lane goes through only"
        );
        assert_eq!(
            j.arm(2).unwrap().lanes.iter().map(|l| l.to.clone()).collect::<Vec<_>>(),
            [vec![3, 4], vec![4], vec![1]],
            "east is a side street: its outermost lane turns right only"
        );
    }

    #[test]
    fn the_single_lane_roundabout_comes_back_with_its_streets_own_lanes() {
        let mut j = roundabout();
        let plain = j.current().clone();
        assert!(j.set_turbo(Some(0)));
        assert_ne!(j.arm(2).unwrap().lanes.len(), plain.arms[1].lanes.len(), "the street gained a lane");
        assert!(j.set_turbo(None));
        assert_eq!(j.current().turbo, None);
        assert_eq!(j.arm(2).unwrap().lanes, plain.arms[1].lanes, "its own lanes again");
        assert_eq!(en(j.revisions().last().unwrap()), "Roundabout: single lane");
        assert_eq!(fr(j.revisions().last().unwrap()), "Giratoire\u{a0}: une seule voie");
    }

    #[test]
    fn choosing_a_turbo_drops_the_bus_lane_and_queue_jumps_and_keeps_gates_and_the_cycle_track() {
        let mut j = roundabout();
        let (n, e, s, so) = (arm_at(&j, 0), arm_at(&j, 90), arm_at(&j, 270), arm_at(&j, 180));
        assert!(j.set_bus(Some((n, so))));
        assert!(j.set_cycle_track(true));
        assert!(j.set_approach(e, Q_OFFSET));
        assert!(j.set_approach(s, GATE_YIELD));
        assert!(j.set_turbo(Some(0)));
        assert_eq!(j.current().bus, None);
        assert_eq!(j.arm(e).unwrap().approach, 0, "the queue jump goes");
        assert_eq!(j.arm(s).unwrap().approach, GATE_YIELD, "a gate set back upstream stays");
        assert_eq!(j.current().cycle, Some(CYCLE_DEFAULT_MM), "and so does the cycle track");
        assert!(j.undo());
        assert!(j.current().bus.is_some() && j.arm(e).unwrap().approach == Q_OFFSET, "undo brings them back");
    }

    #[test]
    fn crossings_stand_at_least_6_m_from_a_turbo_ring() {
        let mut j = roundabout();
        let n = arm_at(&j, 0);
        assert_eq!(j.arm(n).unwrap().crossing.unwrap().setback_mm, DEFAULT_SETBACK_MM);
        assert!(j.set_turbo(Some(0)));
        assert_eq!(j.arm(n).unwrap().crossing.unwrap().setback_mm, 6_000);
        assert!(!j.set_setback(n, 3_000), "closer than that is no change on a turbo");
        assert!(j.set_setback(n, 7_000));
    }

    /// `s` (a saved state, without streets) on the first sample street, where the arms facing `walk` read a walkway.
    fn on_streets(s: &State, walk: &[i32]) -> State {
        use crate::shared::catalogue::{Side, StreetClass};
        let walkway = Street::imported(StreetClass::Pedestrian, Side::Right, &[]);
        let mut s = s.clone();
        for a in &mut s.arms {
            a.section = Some(if walk.contains(&a.bearing) { walkway.clone() } else { Street::sample(0, Side::Right) });
            a.edge = a.uid;
        }
        s
    }

    #[test]
    fn a_turbo_whose_streets_are_made_pedestrian_is_only_suspended_and_comes_back_with_the_traffic() {
        let mut j = roundabout();
        assert!(j.set_turbo(Some(0)) && j.step_turbo_size(1));
        let (saved, turbo) = (j.snapshot(), j.current().turbo.clone());
        assert!(turbo.is_some());
        let walk = on_streets(&saved, &[90, 180, 270]);
        let mut quiet = Junction::from_city(named_said("Test"), &walk, &walk, 0).expect("it draws");
        assert_eq!(traffic_arms(quiet.current()), 1);
        assert_eq!(quiet.current().turbo, turbo, "the turbo is kept while the control is in force nowhere");
        assert!(quiet.set_corner(arm_at(&quiet, 0), 4_000), "an edit made meanwhile");
        assert_eq!(quiet.current().turbo, turbo, "does not write the record without it");
        let _ = quiet.view(); // and it can be read
        let back = on_streets(&quiet.snapshot(), &[]);
        let busy = Junction::from_city(named_said("Test"), &back, &back, 0).expect("it draws");
        assert_eq!(busy.current().control, ROUNDABOUT);
        assert_eq!(busy.current().turbo, turbo, "the same layout, block and main");
    }

    #[test]
    fn a_main_road_that_is_no_longer_an_opposite_pair_can_be_moved_to_one_that_is() {
        let mut j = narrow_roundabout();
        assert!(j.set_turbo(Some(0)));
        let (north, south) = (arm_at(&j, 0), arm_at(&j, 180));
        assert!(j.set_bearing(north, 20) && j.set_bearing(south, 160), "each main arm 20 degrees off, which is allowed: the pair is 140 apart");
        let t = j.current().turbo.clone().expect("the turbo stands");
        assert_eq!(t.main, [1, 3], "the north-south road is still the main road");
        assert!(j.view().turbo.as_ref().unwrap().can_switch_main, "the button is shown");
        assert!(j.switch_turbo_main(), "and the switch is accepted");
        assert_eq!(j.current().turbo.as_ref().unwrap().main, [2, 4]);
        assert!(!j.view().turbo.as_ref().unwrap().can_switch_main, "the only pair that stands is the main road now");
        assert!(!j.switch_turbo_main());
    }

    #[test]
    fn a_suspended_turbo_is_kept_but_neither_read_nor_enforced_and_comes_back_with_the_traffic() {
        let mut j = suspended_turbo();
        assert!(j.current().turbo.is_some(), "kept for the save");
        assert!(j.snapshot().turbo.is_some());
        let v = j.view();
        assert!(v.turbo.is_none() && v.ring.is_none() && v.turbo_options.is_empty(), "the junction has no roundabout, so no turbo");
        let n = arm_at(&j, 0);
        assert!(j.set_approach(n, Q_OFFSET), "a queue jump is not refused for the sake of a turbo that is not in force");
        assert!(j.view().arms.iter().all(|a| a.transit.approaches_out.is_empty()));
        assert!(!j.set_turbo_size(1) && !j.step_turbo_size(1) && !j.switch_turbo_main(), "and there is no turbo to size or move");
        assert!(j.current().turbo.is_some());
    }

    #[test]
    fn a_suspended_turbo_that_cannot_be_a_turbo_at_all_is_dropped_but_a_badly_shaped_one_is_not() {
        let mut j = roundabout();
        assert!(j.set_turbo(Some(0)));
        let state = j.snapshot();
        let quiet = |state: &State, turbo: Turbo| {
            let mut s = on_streets(state, &[90, 180, 270]);
            s.turbo = Some(turbo);
            Junction::from_city(named_said("Test"), &s, &s, 0).expect("it draws").current().turbo.clone()
        };
        let good = Turbo { layout: 0, size: 1, main: vec![1, 3] };
        assert_eq!(quiet(&state, good.clone()), Some(good));
        assert_eq!(quiet(&state, Turbo { layout: 9, size: 0, main: vec![1, 3] }), None, "no such layout");
        assert_eq!(quiet(&state, Turbo { layout: 0, size: 9, main: vec![1, 3] }), None, "no such block");
        assert_eq!(quiet(&state, Turbo { layout: 0, size: 0, main: vec![1, 77] }), None, "a main road that is not an arm");
        let mut three = state.clone();
        three.arms.truncate(3);
        assert_eq!(quiet(&three, Turbo { layout: 0, size: 0, main: vec![1, 3] }), None, "basic needs four streets");
    }

    #[test]
    fn the_ring_size_chosen_before_a_turbo_is_back_when_the_turbo_goes() {
        let mut j = roundabout();
        assert!(j.set_ring(3_000));
        assert!(j.set_turbo(Some(0)));
        assert!(!j.set_ring(1_000), "a turbo has blocks, not a length");
        assert_eq!(j.current().ring_extra_mm, 3_000, "kept, not used");
        let mut bare = j.current().clone();
        bare.ring_extra_mm = 0;
        let radius = |s: &State| crate::junction::read_model::layout(s, 0).and_then(|l| l.ring).map(|r| r.radius);
        assert_eq!(radius(j.current()), radius(&bare), "the ring of a turbo is its block's, whatever the size kept");
        assert!(j.set_turbo(None));
        assert_eq!(j.current().ring_extra_mm, 3_000, "the earlier size is back");
    }

    #[test]
    fn the_block_is_stepped_and_set_among_the_four_and_each_change_is_labelled() {
        let mut j = roundabout();
        assert!(j.set_turbo(Some(0)));
        assert!(j.step_turbo_size(1));
        assert_eq!(j.current().turbo.as_ref().unwrap().size, 1);
        assert_eq!(en(j.revisions().last().unwrap()), "Turbo roundabout size: regular");
        assert_eq!(fr(j.revisions().last().unwrap()), "Taille du giratoire turbo\u{a0}: normale");
        assert!(j.set_turbo_size(3));
        assert!(!j.set_turbo_size(4), "there are four blocks");
        assert!(!j.step_turbo_size(1), "and none larger than the large");
        assert!(j.set_turbo_size(0));
        assert!(!j.step_turbo_size(-1), "or smaller than the mini");
        assert!(!roundabout().step_turbo_size(1), "a roundabout with no turbo has no block");
    }

    #[test]
    fn undo_takes_back_a_size_and_a_main_road_switch_and_a_refused_size_adds_no_step() {
        let mut j = roundabout();
        assert!(j.set_turbo(Some(0)));
        let before = j.current().clone();
        assert!(j.set_turbo_size(2));
        assert!(j.undo());
        assert_eq!(j.current().turbo, before.turbo, "the earlier block is back");
        assert!(j.switch_turbo_main());
        assert!(j.undo());
        assert_eq!(j.current().turbo, before.turbo, "and the earlier main road");
        assert!(j.redo());
        assert_eq!(j.current().turbo.as_ref().unwrap().main, [2, 4], "and redo does it again");
        let steps = j.revisions().count();
        let state = j.current().clone();
        assert!(!j.set_turbo_size(4) && !j.set_turbo_size(0), "a block that does not exist, and the one it has");
        assert_eq!((j.current(), j.revisions().count()), (&state, steps), "a refused size adds no step");
    }

    #[test]
    fn the_main_road_switches_to_the_other_pair_and_back_when_there_is_one() {
        let mut j = roundabout();
        assert!(j.set_turbo(Some(0)));
        assert!(j.switch_turbo_main());
        assert_eq!(j.current().turbo.as_ref().unwrap().main, [2, 4]);
        assert!(en(j.revisions().last().unwrap()).starts_with("Main road of the turbo roundabout: "));
        assert!(fr(j.revisions().last().unwrap()).starts_with("Axe principal du giratoire turbo\u{a0}: "));
        assert!(j.switch_turbo_main());
        assert_eq!(j.current().turbo.as_ref().unwrap().main, [1, 3]);
        let mut egg = Junction::new(1);
        assert!(egg.set_control(ROUNDABOUT) && egg.set_turbo(Some(1)));
        assert!(!egg.switch_turbo_main(), "three streets have one through road");
        assert!(!roundabout().switch_turbo_main(), "no turbo, no main road");
    }

    #[test]
    fn an_arm_more_than_20_degrees_off_square_is_refused_by_name_and_one_10_off_is_allowed() {
        let mut j = narrow_roundabout();
        let east = arm_at(&j, 90);
        assert!(j.set_turbo(Some(0)));
        assert!(!j.set_bearing(east, 115), "25 degrees off the east template");
        assert_eq!(j.refusal(), Some(Refusal::TurboArmSkewed));
        assert!(j.refusal_arm().is_some(), "the arm is named");
        assert!(j.set_bearing(east, 100), "10 degrees off is allowed (and reported by a check)");
        assert_eq!(j.refusal(), None);
        assert!(j.refusal_arm().is_none(), "a taken edit clears the arm too");

        let mut k = narrow_roundabout();
        assert!(k.set_bearing(east, 125));
        assert!(!k.set_turbo(Some(0)), "choosing the layout is refused for the same reason: 125 degrees has no opposite street to be the main road instead");
        assert_eq!(k.refusal(), Some(Refusal::TurboArmSkewed));
        assert_eq!(k.current().turbo, None);
    }

    #[test]
    fn a_turbo_is_dropped_when_the_street_count_no_longer_fits_and_undo_brings_it_back() {
        let mut j = roundabout();
        assert!(j.set_turbo(Some(0)));
        assert!(j.remove_arm(2));
        assert_eq!(j.current().control, ROUNDABOUT, "it is a single-lane roundabout again");
        assert_eq!(j.current().turbo, None, "basic is for four streets");
        assert!(j.undo());
        assert!(j.current().turbo.is_some());
        assert_eq!(j.current().arms.len(), 4);
    }

    #[test]
    fn a_turbo_ends_with_the_roundabout() {
        let mut j = roundabout();
        assert!(j.set_turbo(Some(0)));
        assert!(j.set_control(PRIORITY));
        assert_eq!(j.current().turbo, None);
        assert!(j.undo());
        assert!(j.current().turbo.is_some());
    }

    #[test]
    fn a_turbo_roundabout_made_from_a_timed_signal_has_no_timing_and_a_signal_made_from_it_has_no_turbo() {
        let mut j = Junction::new(0);
        let n = arm_at(&j, 0);
        assert!(j.set_timing(n, TimingField::Green, 30));
        assert!(j.set_control(ROUNDABOUT));
        assert_eq!(j.arm(n).unwrap().timing, None, "a roundabout has no signal to time");
        assert!(plain(&en(j.revisions().last().unwrap())).contains("signal timing removed"), "the step says so");
        assert!(j.set_turbo(Some(0)));
        assert!(j.current().arms.iter().all(|a| a.timing.is_none()));
        assert!(!j.set_timing(n, TimingField::Green, 30), "a turbo roundabout is no signal either");
        assert_eq!(j.refusal(), Some(Refusal::NotSignal));
        assert!(j.set_control(SIGNAL));
        assert_eq!(j.current().turbo, None, "the turbo goes with the roundabout");
        assert_eq!(j.arm(n).unwrap().timing, None, "and the signal starts untimed");
        assert!(
            !plain(&en(j.revisions().last().unwrap())).contains("removed"),
            "nothing was lost that the step has to name: {}",
            en(j.revisions().last().unwrap())
        );
        assert!(j.undo());
        assert!(j.current().turbo.is_some(), "undo brings the turbo back");
    }

    #[test]
    fn a_turbo_follows_a_change_of_region() {
        let mut j = roundabout();
        assert!(j.set_turbo(Some(0)));
        assert!(j.set_region(3));
        assert!(j.current().turbo.is_some(), "left-hand traffic mirrors the ring, it does not end it");
        assert_eq!(j.view().ring.unwrap().circulation, "clockwise");
    }

    #[test]
    fn a_turbo_that_fits_no_block_is_refused_as_too_big_and_changes_nothing() {
        let mut j = junction_of(&[("travel", 3_500); 11]);
        assert!(j.set_control(ROUNDABOUT), "the plain roundabout fits");
        let (before, steps) = (j.current().clone(), j.revisions().count());
        assert!(!j.set_turbo(Some(0)));
        assert_eq!(j.refusal(), Some(Refusal::RoundaboutTooBig));
        assert_eq!(*j.current(), before);
        assert_eq!(j.revisions().count(), steps, "a refused edit adds no step to the history");
    }

    #[test]
    fn an_arm_that_cars_do_not_enter_gets_no_turbo_lane() {
        // The east street goes only one way out of the junction, and the west one is a walkway: neither is entered by cars.
        let mut state = Junction::new(0).current().clone();
        state.control = ROUNDABOUT;
        for a in &mut state.arms {
            a.edge = a.uid;
            match a.bearing {
                90 => a.section = Some(one_way_out()),
                270 => a.section = Some(Street::imported(StreetClass::Pedestrian, Side::Right, &[])),
                _ => {}
            }
        }
        let mut j = Junction::from_city(named("Test"), &state, &state, 0).expect("it draws");
        assert!(j.arm(arm_at(&j, 90)).unwrap().profile(0).enter_x.is_empty(), "nothing enters from the east");
        assert!(j.set_turbo(Some(0)), "nothing to widen there, nothing in the way: {:?}", j.refusal());
        let lanes = |bearing: i32| j.arm(arm_at(&j, bearing)).unwrap().lanes.len();
        assert_eq!((lanes(0), lanes(90), lanes(180), lanes(270)), (2, 0, 2, 0), "the layout's lanes where cars enter, none where they do not");
        let _ = j.view();
    }

    #[test]
    fn a_turbo_is_not_offered_where_the_streets_leave_nothing_to_control() {
        // Every street a walkway: the junction is run uncontrolled whatever control was kept, and a layout of the roundabout it is not has no meaning.
        let mut j = junction_of(&[("sidewalk", 3_000), ("bike", 2_000), ("sidewalk", 3_000)]);
        assert!(j.set_control(ROUNDABOUT));
        assert!(!j.set_turbo(Some(0)));
        assert_eq!(j.refusal(), Some(Refusal::DoesNotFit));
    }

    #[test]
    fn a_saved_turbo_whose_arm_is_now_far_off_square_loads_as_a_single_lane_roundabout() {
        let mut state = Junction::new(0).current().clone();
        state.control = ROUNDABOUT;
        state.turbo = Some(Turbo { layout: 0, size: 0, main: vec![1, 3] });
        state.arms[1].bearing = 125; // the east street, now 35 degrees off
        for a in &mut state.arms {
            a.edge = a.uid;
        }
        let j = Junction::from_city(named("Test"), &state, &state, 0).expect("it still draws, as a plain roundabout");
        assert_eq!(j.current().control, ROUNDABOUT);
        assert_eq!(j.current().turbo, None);
    }

    #[test]
    fn a_saved_turbo_that_fits_no_block_loads_as_a_single_lane_roundabout() {
        let mut plain = junction_of(&[("travel", 3_500); 11]);
        assert!(plain.set_control(ROUNDABOUT));
        let mut state = plain.current().clone();
        state.turbo = Some(Turbo { layout: 0, size: 3, main: vec![1, 3] });
        let j = Junction::from_city(named("Test"), &state, &state, 0).expect("it draws");
        assert_eq!((j.current().control, j.current().turbo.clone()), (ROUNDABOUT, None));
        assert_eq!(j.current().arms[0].lanes.len(), 11, "the streets' own lanes again, not the layout's two");
    }

    #[test]
    fn a_layout_whose_entry_lanes_would_pass_a_streets_property_line_is_refused_naming_the_street() {
        // Sample Street 1 is 18 m across and its entry lane lies at 5.7..9 m. A rotor wants three lanes of 3.5 m there: (-1.5 m, 9 m), 1.5 m past the line at 0.
        let mut j = roundabout();
        let (before, steps) = (j.current().clone(), j.revisions().count());
        assert!(!j.set_turbo(Some(4)));
        assert_eq!(j.refusal(), Some(Refusal::TurboEntryTooNarrow));
        assert_eq!(
            en(j.refusal_arm().unwrap()),
            en(&arm_name(j.arm(2).unwrap())),
            "the first street in the order of the arms that cannot take it: the east one"
        );
        assert_eq!(*j.current(), before, "nothing changed");
        assert_eq!(j.revisions().count(), steps, "a refused edit adds no step to the history");
        assert!(j.set_turbo(Some(3)), "a spiral wants three lanes on the Avenue, which has the room, and two on the Street");
        assert!(!j.switch_turbo_main(), "with the Street as the main road the spiral would give it three");
        assert_eq!(j.refusal(), Some(Refusal::TurboEntryTooNarrow));
        assert_eq!(en(j.refusal_arm().unwrap()), en(&arm_name(j.arm(2).unwrap())));
        assert_eq!(j.current().turbo.as_ref().unwrap().main, [1, 3], "the main road is as it was");
        assert!(j.set_turbo(Some(0)), "and a refusal leaves the junction as it was, so another layout can still be chosen");
        assert_eq!(j.refusal(), None);
        assert!(j.refusal_arm().is_none(), "a taken edit clears the arm too");
    }

    #[test]
    fn a_saved_turbo_whose_street_has_not_the_room_for_its_entry_loads_as_a_single_lane_roundabout() {
        let mut state = Junction::new(0).current().clone();
        state.control = ROUNDABOUT;
        state.turbo = Some(Turbo { layout: 4, size: 0, main: vec![1, 3] }); // a rotor: three lanes on every street, too many for Street 1
        for a in &mut state.arms {
            a.edge = a.uid;
        }
        let j = Junction::from_city(named("Test"), &state, &state, 0).expect("it still draws, as a plain roundabout");
        assert_eq!((j.current().control, j.current().turbo.clone()), (ROUNDABOUT, None));
    }
}
