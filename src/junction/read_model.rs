//! Junction geometry, checks and the serialised view. `layout` decides where
//! everything sits in plan; `Junction::view` turns it into what the page draws.
//! The page does no geometry of its own.

use serde::Serialize;
use serde_json::Value;

use crate::junction::beyond;
use crate::junction::geometry::{
    P, add, arc, at, cross, dir, dist, fillet_setback, gap, l, m, meet, on_circle, poly, polylines_cross, quad, ring_arc, sample_quad, scale, strip, sub, z,
};
use crate::junction::model::{
    ALL_WAY_STOP, APPROACH_DEFAULT_MM, APPROACHES, ARM_LENGTH_MM, Arm, BUS_LANE_MM, BikeBoxBlocker, CONTROLS, FILTER_CODE, GATE_SIGNAL, GATE_YIELD,
    ISLAND_MIN_ROAD_MM, ISLAND_MM, Junction, LEFT, MAX_RING_MM, MIN_ARMS, PRIORITY, Profile, ProtectedBlocker, Q_CURB, Q_OFFSET, Q_VIRTUAL, RIGHT,
    RING_STEP_MM, ROUNDABOUT, RULE_AROUND, RULE_DEAD_END, RULE_RIRO, RULE_WITHIN, RULES, SIGNAL, STOP_BULB, STOP_PLATFORM, STOPS, State, THROUGH,
    TRANSIT_LANE_MM, Target, arm_name, bike_box_blocker, blocked, classes_of, in_force, protected_blocker, protected_island_mm, traffic_arms, turn_class,
};

use crate::junction::signal::{self, Pattern};
use crate::junction::stops::{self, StopView};
use crate::junction::turbo::{self, BLOCKS, Edge, LAYOUTS, Turbo, WARN_SKEW_DEG, layouts_for, main_candidates, ring_lane};
use crate::junction::turbo_ring::{follow, mouth_on};
use crate::junction::turbo_spiral::{Axes, Mouth, RingLane, RingShape, ring_outline, ring_shape, spiral_walk};
use crate::shared::catalogue::{KINDS, REGIONS, Side, is_roadway};
use crate::shared::provenance::OsmRef;
use crate::shared::said::{Arg, Said};
use crate::street::model::{Beyond, STOP_CLEAR_MM, stop_gap_mm};

/// One rule of the junction and whether it holds, said as data.
#[derive(Serialize)]
pub struct Check {
    pub id: &'static str,
    pub ok: bool,
    /// The length the detail speaks of, so the page can print it in its units.
    pub amount_mm: i32,
    pub label: Said,
    pub detail: Said,
}

/// One step of the junction's history.
#[derive(Serialize)]
pub struct Revision {
    pub step: usize,
    pub label: Said,
}

/// Things said one after another, as one: "A; B; C".
fn list(items: Vec<Said>) -> Option<Said> {
    items.into_iter().reduce(|head, tail| Said::new("jn-list").with("head", Arg::Said(Box::new(head))).with("tail", Arg::Said(Box::new(tail))))
}

/// Two arms, from one to the other.
fn arm_to_arm(from: Said, to: Said) -> Said {
    Said::new("jn-arm-to-arm").with("from", Arg::Said(Box::new(from))).with("to", Arg::Said(Box::new(to)))
}

/// Ring width of a roundabout, and the smallest island in its middle.
const RING_WIDTH_MM: f64 = 6_000.0;
const MIN_ISLAND_MM: f64 = 1_500.0;
/// A ring's least outer radius, and the arc between two arms it keeps clear.
const MIN_RING_MM: i32 = 12_000;
const MIN_RING_ARC_MM: f64 = 8_000.0;
/// Bits of the road stay clear of parking this far past a crossing.
const PARK_CLEAR_MM: f64 = 1_000.0;
const NO_CROSSING_CLEAR_MM: f64 = 4_000.0;
/// How far past its mouth the corner pavement is drawn as one piece.
const WEDGE_MM: f64 = 3_000.0;
/// How far an arm's outer bike lane runs into the junction core.
const CORE_BIKE_MM: f64 = 5_000.0;
/// How deep a bike box is.
const BIKE_BOX_MM: f64 = 3_000.0;
/// Synthetic thresholds for the checks.
const MAX_STAGE_MM: i32 = 15_000;
const MAX_TURN_KMH: f64 = 20.0;
/// The most a turn may do beside a crossing on a raised table, which slows traffic. The fastest corner the editor
/// allows (15 m) does 23.9 km/h, so this still flags the widest corners.
const RAISED_TURN_KMH: f64 = 23.0;
/// How long the ramp up to a raised table is: 160 mm in 15.
const RAMP_MM: f64 = 2_400.0;
const MIN_SIDEWALK_AT_CORNER_MM: f64 = 1_800.0;
const MAX_SIGNAL_ARMS: usize = 4;
/// How fast a person walks across a street, and the longest cycle a signal should keep. Synthetic, like the rest.
const WALK_SPEED_MPS: f64 = 1.0;
const MAX_CYCLE_S: u32 = 120;

pub fn speed_kmh(radius_mm: i32) -> f64 {
    (127.0 * (radius_mm as f64 / 1000.0) * 0.3).sqrt()
}

// ---- layout -----------------------------------------------------------------

pub struct ArmLayout {
    pub bearing: f64,
    pub prof: Profile,
    /// Lateral positions, including the arm's offset: curbs and property lines.
    pub cl: f64,
    pub cr: f64,
    pub pl: f64,
    pub pr: f64,
    /// Where the carriageway's mouth is, and where its strips start.
    pub mouth: f64,
    pub strip0: f64,
    /// On a turbo: how its entry was widened to the layout's lanes. None for a plain junction, and for an arm cars do not enter.
    pub entry: Option<EntryWiden>,
    /// On a turbo: the width of each entering lane.
    pub lane_mm: Option<i32>,
}

/// An entry's extent across its section, laterally and with the arm's offset, as the street has it (`old`) and as a
/// turbo layout wants it (`new`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EntryWiden {
    pub new: (f64, f64),
    pub old: (f64, f64),
}

impl ArmLayout {
    fn lat(&self, offset: f64, x_mm: i32) -> f64 {
        offset + x_mm as f64 - self.prof.row_mm as f64 / 2.0
    }
}

pub struct CornerLayout {
    /// Arms this corner lies between, clockwise: `a` then `b`.
    pub a: usize,
    pub b: usize,
    pub delta: i32,
    /// The pieces of a plain corner: where the curb lines meet, the tangent
    /// points, the arc's centre and radius. None for a straight curb.
    pub fillet: Option<Fillet>,
}

pub struct Fillet {
    pub c: P,
    pub a: P,
    pub b: P,
    pub o: P,
    pub r: f64,
    pub u: P,
}

pub struct RingLayout {
    pub radius: f64,
    pub floor: i32,
    pub turbo: Option<TurboRing>,
}

/// The ring of a turbo roundabout: the axis its edges are shifted along, its block, its lanes, and the outermost edge
/// (the cycle track included), which the arms meet: a closed outline that steps along the axis at both tips.
pub struct TurboRing {
    pub axis_deg: f64,
    pub block: usize,
    pub lanes: usize,
    pub outline: Vec<P>,
}

pub struct Layout {
    pub arms: Vec<ArmLayout>,
    pub corners: Vec<CornerLayout>,
    pub ring: Option<RingLayout>,
}

/// The turbo `Junction::set_turbo` gives `layout` on this roundabout, and whether it stands and some block holds it
/// (a layout is offered exactly when it does). The main road is the one a turbo already on has if it still stands with
/// the new layout, else the best that stands, else the best (it then does not stand); the block is the smallest that
/// fits, from the one a turbo already on has up, else the largest. `None` when there is no opposite pair.
pub fn choose_turbo(s: &State, region: usize, which: usize) -> Option<(Turbo, bool)> {
    let candidates = main_candidates(&s.arms, region, which);
    let stands = |m: &Vec<u32>| Turbo { layout: which, size: 0, main: m.clone() }.standable(&s.arms, region);
    let kept = s.turbo.as_ref().map(|t| t.main.clone()).filter(|m| candidates.contains(m) && stands(m));
    let main = kept.or_else(|| candidates.iter().find(|m| stands(m)).cloned()).or_else(|| candidates.first().cloned())?;
    let standing = stands(&main);
    let mut held = s.clone();
    let from = s.turbo.as_ref().map_or(0, |t| t.size);
    for size in from..BLOCKS.len() {
        let turbo = Turbo { layout: which, size, main: main.clone() };
        held.turbo = Some(turbo.clone());
        if layout(&held, region).is_some() {
            return Some((turbo, standing));
        }
    }
    Some((Turbo { layout: which, size: BLOCKS.len() - 1, main }, false))
}

/// The main road `switch_turbo_main` moves the turbo on `s` to, and whether the move is accepted (so whether the
/// inspector offers the button): the next candidate after the turbo's own that stands and that its block holds. A main
/// road that is not a candidate any more (its arms were turned off a straight line, which the skew limit allows) is left
/// for the best candidate that stands. When none stands, the next candidate in turn with `false`, for the refusal to
/// say why. None when there is no turbo or no other main road to go to.
pub fn next_main(s: &State, region: usize) -> Option<(Vec<u32>, bool)> {
    let t = s.turbo.as_ref()?;
    let candidates = main_candidates(&s.arms, region, t.layout);
    let after = candidates.iter().position(|m| *m == t.main).map_or(0, |i| i + 1);
    let others: Vec<&Vec<u32>> = candidates.iter().cycle().skip(after).take(candidates.len()).filter(|m| **m != t.main).collect();
    let mut held = s.clone();
    let stands = |m: &Vec<u32>, held: &mut State| {
        let turbo = Turbo { main: m.clone(), ..t.clone() };
        held.turbo = Some(turbo.clone());
        turbo.standable(&s.arms, region) && layout(held, region).is_some()
    };
    match others.iter().find(|m| stands(m, &mut held)) {
        Some(m) => Some(((*m).clone(), true)),
        None => others.first().map(|m| ((*m).clone(), false)),
    }
}

/// How far each arm's own works reach along it from the junction's centre, by the arm's uid: to the far side of its
/// crossing, or to its mouth where it has none. Empty when the junction cannot be laid out.
pub fn arm_reaches(s: &State, region: usize) -> Vec<(u32, i32)> {
    let Some(lay) = layout(s, region) else { return Vec::new() };
    s.arms.iter().zip(&lay.arms).map(|(a, l)| (a.uid, l.mouth.round() as i32 + a.crossing.map_or(0, |c| c.setback_mm + c.width_mm))).collect()
}

/// Puts the junction on the sheet, or says it cannot be drawn: a corner whose
/// curbs meet behind the junction, a fillet longer than its arm, a roundabout
/// that will not fit its arms in the largest ring.
pub fn layout(s: &State, region: usize) -> Option<Layout> {
    let in_force = in_force(s);
    let s: &State = &in_force;
    let n = s.arms.len();
    let mut arms: Vec<ArmLayout> = s
        .arms
        .iter()
        .map(|a| {
            let prof = a.profile(region);
            let half = prof.row_mm as f64 / 2.0;
            let off = a.offset_mm as f64;
            ArmLayout {
                bearing: a.bearing as f64,
                cl: off + prof.road_l as f64 - half,
                cr: off + prof.road_r as f64 - half,
                pl: off - half,
                pr: off + half,
                prof,
                mouth: 0.0,
                strip0: 0.0,
                entry: None,
                lane_mm: None,
            }
        })
        .collect();
    let limit = ARM_LENGTH_MM as f64 * 0.75;
    let mut corners = Vec::new();

    if s.control == ROUNDABOUT {
        if let Some(t) = s.turbo.as_ref() {
            return turbo_layout(s, arms, t, region);
        }
        let floor = ring_floor(&arms)?;
        let radius = (floor + s.ring_extra_mm).min(MAX_RING_MM) as f64;
        for a in arms.iter_mut() {
            let (tl, tr) = (on_circle(a.cl, radius)?, on_circle(a.cr, radius)?);
            a.mouth = tl.max(tr);
            a.strip0 = tl.min(tr);
        }
        return Some(Layout { arms, corners: ring_corners(s), ring: Some(RingLayout { radius, floor, turbo: None }) });
    }

    let mut mouths = vec![0.0f64; n];
    for i in 0..n {
        let j = (i + 1) % n;
        let delta = gap(s.arms[i].bearing, s.arms[j].bearing);
        let fillet = if delta == 180 {
            None
        } else {
            let (t, u) = meet(arms[i].bearing, arms[i].cr, arms[j].bearing, arms[j].cl)?;
            let r = s.arms[i].corner_mm as f64;
            let setback = fillet_setback(r, delta as f64);
            let (ta, sb) = (t + setback, u + setback);
            if ta < 0.0 || sb < 0.0 || ta > limit || sb > limit {
                return None;
            }
            mouths[i] = mouths[i].max(ta);
            mouths[j] = mouths[j].max(sb);
            let c = at(arms[i].bearing, arms[i].cr, t);
            let (da, db) = (dir(arms[i].bearing), dir(arms[j].bearing));
            let bis = add(da, db);
            let len = (bis.0 * bis.0 + bis.1 * bis.1).sqrt();
            let uvec = (bis.0 / len, bis.1 / len);
            let o = add(c, scale(uvec, r / (delta as f64 / 2.0).to_radians().sin()));
            Some(Fillet { c, a: at(arms[i].bearing, arms[i].cr, ta), b: at(arms[j].bearing, arms[j].cl, sb), o, r, u: uvec })
        };
        corners.push(CornerLayout { a: i, b: j, delta, fillet });
    }
    for (a, m) in arms.iter_mut().zip(mouths) {
        a.mouth = m;
        a.strip0 = m;
    }
    Some(Layout { arms, corners, ring: None })
}

/// The corners of a ring: one between each arm and the next, with no fillet.
fn ring_corners(s: &State) -> Vec<CornerLayout> {
    let n = s.arms.len();
    (0..n)
        .map(|i| {
            let j = (i + 1) % n;
            CornerLayout { a: i, b: j, delta: gap(s.arms[i].bearing, s.arms[j].bearing), fillet: None }
        })
        .collect()
}

/// A roundabout with a turbo: each entered arm widened to the layout's lanes, the ring built on the block's outer
/// edge, the arms meeting it where their curbs cross it. None when the turbo does not suit the arms, or the ring is
/// too small for them.
fn turbo_layout(s: &State, mut arms: Vec<ArmLayout>, t: &Turbo, region: usize) -> Option<Layout> {
    let (spec, block) = (t.spec()?, t.block()?);
    if !t.fits_arms(&s.arms) {
        return None;
    }
    let right_hand = REGIONS[region].drive_side == Side::Right;
    for (a, arm) in arms.iter_mut().zip(&s.arms) {
        if a.prof.enter_x.is_empty() {
            continue; // cars do not enter this street, so it has no entry to widen
        }
        let n = t.entry_lanes(arm.uid);
        let old = a.prof.enter_span;
        let (xs, span) = turbo::widen_entry(&a.prof, n, turbo::TURBO_ENTRY_LANE_MM);
        let off = arm.offset_mm as f64;
        let (new_l, new_r) = (a.lat(off, span.0), a.lat(off, span.1));
        a.entry = Some(EntryWiden { new: (new_l, new_r), old: (a.lat(off, old.0), a.lat(off, old.1)) });
        a.lane_mm = Some((span.1 - span.0) / n.max(1) as i32);
        a.prof.enter_x = xs;
        a.prof.enter_span = span;
        a.cl = a.cl.min(new_l);
        a.cr = a.cr.max(new_r);
    }
    let outer = block.road_outer_mm(spec.ring_lanes) + s.cycle.unwrap_or(0);
    if !ring_fits(&arms, outer) {
        return None;
    }
    let axis = t.axis_deg(&s.arms, right_hand)?;
    let outline = ring_outline(&Axes::new(axis, right_hand), Edge { radius_mm: outer, shift_mm: block.du_mm }, right_hand);
    for a in arms.iter_mut() {
        let (tl, tr) = (mouth_on(&outline, a.bearing, a.cl)?, mouth_on(&outline, a.bearing, a.cr)?);
        a.mouth = tl.max(tr);
        a.strip0 = tl.min(tr);
    }
    let ring = RingLayout { radius: outer as f64, floor: outer, turbo: Some(TurboRing { axis_deg: axis, block: t.size, lanes: spec.ring_lanes, outline }) };
    Some(Layout { corners: ring_corners(s), arms, ring: Some(ring) })
}

/// Whether a ring of outer radius `r` keeps each arm's curbs on it and leaves room between neighbouring arms.
fn ring_fits(arms: &[ArmLayout], r: i32) -> bool {
    let n = arms.len();
    let rf = r as f64;
    (0..n).all(|i| {
        let (a, b) = (&arms[i], &arms[(i + 1) % n]);
        if a.cr.abs() >= rf || b.cl.abs() >= rf {
            return false;
        }
        let pa = a.bearing + (a.cr / rf).asin().to_degrees();
        let qb = b.bearing + (b.cl / rf).asin().to_degrees();
        let arc = (qb - pa).rem_euclid(360.0);
        arc >= (MIN_RING_ARC_MM / rf).to_degrees() && arc < 360.0 - 1e-6 && arc <= 180.0 + 1e-6 + (b.bearing - a.bearing).rem_euclid(360.0)
    }) && arms.iter().all(|a| a.cl.abs() < rf && a.cr.abs() < rf)
}

/// The least outer radius that keeps each arm's curbs on the ring and leaves room between neighbouring arms.
fn ring_floor(arms: &[ArmLayout]) -> Option<i32> {
    (MIN_RING_MM..=MAX_RING_MM).step_by(RING_STEP_MM as usize).find(|&r| ring_fits(arms, r))
}

// ---- view -------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct PieceView {
    pub kind: usize,
    pub material: &'static str,
    pub direction: Option<&'static str>,
    pub poly: Vec<Value>,
    /// A taper's quadrilateral, drawn beyond the end of an arm's end zone; not sent when false.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub taper: bool,
}

/// The name of one class of turn, as the page and the checks say it.
fn class_word(c: u8) -> &'static str {
    match c {
        LEFT => "left",
        THROUGH => "through",
        _ => "right",
    }
}

fn word<S: serde::Serializer>(c: &u8, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(class_word(*c))
}

/// The classes in a set of them, left to right.
fn words<S: serde::Serializer>(mask: &u8, s: S) -> Result<S::Ok, S::Error> {
    s.collect_seq([LEFT, THROUGH, RIGHT].into_iter().filter(|c| mask & c != 0).map(class_word))
}

#[derive(Serialize)]
pub struct LaneView {
    /// The kinds of turn it serves, for the arrow drawn on it.
    #[serde(serialize_with = "words")]
    pub uses: u8,
    /// Every other street, in turn order from the driver's left, and whether this lane goes there.
    pub dests: Vec<DestView>,
    /// Serves only turns that are banned.
    pub bad: bool,
    pub at: P,
    /// Direction the lane's traffic travels, as a bearing.
    pub heading: i32,
    /// The lane along the whole arm, for pointing at it.
    pub poly: Vec<Value>,
    pub width_mm: i32,
    /// The layout of a turbo sets where this lane goes: it cannot be edited.
    pub fixed: bool,
}

#[derive(Serialize)]
pub struct DestView {
    pub uid: u32,
    pub label: Said,
    #[serde(serialize_with = "word")]
    pub class: u8,
    pub on: bool,
    /// The street can be left.
    pub open: bool,
}

#[derive(Serialize)]
pub struct CrossingView {
    pub setback_mm: i32,
    pub width_mm: i32,
    pub island: bool,
    pub poly: Vec<Value>,
    pub island_poly: Option<Vec<Value>>,
    /// The crossing is a continuous sidewalk.
    pub continuous: bool,
    /// The ramps up to a continuous sidewalk: the strip before the crossing (not before the mouth), then the one beyond it.
    pub ramps: Vec<Vec<Value>>,
    /// Ends of the crossing's far side and the position of its handle.
    pub handle: P,
    pub distance_mm: i32,
    pub stage_mm: i32,
    pub stages: i32,
    /// A stage is longer than a person should have to cross in one go.
    pub too_far: bool,
}

/// The transit priority measures on one arm, drawn and checked.
#[derive(Serialize)]
pub struct TransitView {
    pub bus_lane: bool,
    pub approach: usize,
    /// The approach measures (indexes into `APPROACHES`) this arm cannot be given: the queue jumps, on a turbo.
    pub approaches_out: Vec<usize>,
    pub approach_mm: i32,
    pub stop: usize,
    pub rule: usize,
    pub filter: bool,
    /// Which curb the traffic coming in runs beside: 0 left, 1 right.
    pub kerb_side: usize,
    pub bus: Option<Vec<Value>>,
    pub queue: Option<Vec<Value>>,
    pub virtual_loop: Option<Vec<Value>>,
    pub gate: Option<[P; 2]>,
    pub stop_poly: Option<Vec<Value>>,
    pub stop_at: Option<P>,
    pub bollards: Vec<P>,
    pub cap: Option<[P; 2]>,
    pub island: Option<Vec<Value>>,
    pub icon_at: P,
    /// What a measure here needs and does not have.
    pub problems: Vec<Said>,
    /// What kind of approach measure there is: none, queue, virtual or gate.
    pub approach_kind: &'static str,
    /// The gate gives way to the buses it lets through, rather than a signal.
    pub gate_yields: bool,
    /// The arm is dead-ended: its street stops short of the junction.
    pub dead: bool,
    /// Where each measure is labelled with its Atlas code, in the order they are drawn.
    pub tags: Vec<MeasureTag>,
    /// The Atlas codes of the measures on this arm.
    pub codes: Vec<&'static str>,
}

/// A measure's Atlas code and where to write it on the plan.
#[derive(Serialize)]
pub struct MeasureTag {
    pub code: &'static str,
    pub at: P,
}

#[derive(Serialize)]
pub struct RaisedView {
    /// The carriageway from the mouth to the far side of the crossing, level with the sidewalk.
    pub top: Vec<Value>,
    /// The ramp up to it.
    pub ramp: Vec<Value>,
}

#[derive(Serialize)]
pub struct ArmView {
    pub uid: u32,
    /// The city street this arm is, or 0 outside a city.
    pub edge: u32,
    pub label: Said,
    pub street: Said,
    pub bearing: i32,
    pub offset_mm: i32,
    pub corner_mm: i32,
    pub pieces: Vec<PieceView>,
    /// What lies beyond the end zone's boundary, for the plan and the 2.5D: the taper's quadrilaterals, then the zone
    /// across's strips. Empty for an arm with no beyond, a pedestrian arm and a turbo's.
    pub beyond: Vec<PieceView>,
    /// The lines across the arm at the taper's ends (at the boundary alone for a jog).
    pub beyond_marks: Vec<[P; 2]>,
    /// Why the end zone's boundary is not safe, said; empty for an arm that draws no beyond.
    pub beyond_problems: Vec<Said>,
    /// How far from the centre the end zone's steady part reaches (the taper's start, else the boundary), and how long
    /// the taper is (0 for a jog); both 0 without a beyond.
    pub beyond_reach_mm: i32,
    pub beyond_taper_mm: i32,
    /// What the zone across is called; `None` without a beyond.
    pub beyond_zone: Option<Said>,
    /// The arm's outer bike lanes, carried into the core from its mouth: the lane arrives, and where.
    pub core_bike: Vec<Vec<Value>>,
    /// The flat top and the ramp of a raised table on this arm.
    pub raised: Option<RaisedView>,
    /// No-parking stretches beside the curb, drawn as plain road.
    pub gaps: Vec<Vec<Value>>,
    pub bulbs: [Option<Vec<Value>>; 2],
    pub crossing: Option<CrossingView>,
    pub stop_line: Option<[P; 2]>,
    /// The bike box across the entering lanes, ahead of the stop line.
    pub bike_box: Option<Vec<Value>>,
    /// The street has a bike lane to reach a box from, cars enter, and the junction is not a roundabout.
    pub can_bike_box: bool,
    /// The message that says why not, when `can_bike_box` is false.
    pub bike_box_why: Option<&'static str>,
    pub lanes: Vec<LaneView>,
    pub leave_arrows: Vec<LaneView>,
    /// The widened carriageway of a turbo entry, where the layout wants more than the street has.
    pub turbo_entry: Option<Vec<Value>>,
    /// "free", "stop", "yield" or "signal" for the entering traffic.
    pub role: &'static str,
    /// The arm from its mouth to its end, property line to property line.
    pub outline: Vec<Value>,
    pub end: P,
    pub mouth_at: P,
    pub banned: Vec<u32>,
    #[serde(serialize_with = "words")]
    pub classes: u8,
    pub road_mm: i32,
    /// Which side's curb can bulge out into parking: only beside parking.
    pub can_bulb: [bool; 2],
    pub can_island: bool,
    pub max_offset_mm: i32,
    pub enters: bool,
    pub leaves: bool,
    /// The street this arm reads is a pedestrian street.
    pub pedestrian: bool,
    pub transit: TransitView,
    /// The street's transit stops on the arm, nearest first: those with a part on the drawn arm. Empty (and not sent)
    /// for an arm with none, a pedestrian arm and a turbo's.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub stops: Vec<StopView>,
    /// The stops that stand too near the junction, as the check says them.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub stop_clear: Vec<Said>,
    /// The stops whose bus is trapped in a turning lane.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub stop_lane: Vec<Said>,
}

#[derive(Serialize)]
pub struct ProtectedView {
    /// The bike track round the corner, between the curb and the island.
    pub track: Vec<Value>,
    /// The corner island inside it.
    pub island: Vec<Value>,
}

#[derive(Serialize)]
pub struct CornerView {
    /// The arm this corner is clockwise of.
    pub uid: u32,
    pub next_uid: u32,
    pub radius_mm: i32,
    pub speed_kmh: f64,
    pub ok: bool,
    /// Turns here are faster than is safe beside a crossing.
    pub fast: bool,
    pub straight: bool,
    /// A bordering arm has a sidewalk next to its curb here, so the corner is pavement for walking.
    pub walk: bool,
    /// The corner is protected: the bike track and the island.
    pub protected: Option<ProtectedView>,
    /// The message that says why the corner cannot be protected, when it cannot.
    pub protected_why: Option<&'static str>,
    pub wedge: Vec<Value>,
    pub curb: Vec<Value>,
    /// The direction the corner's arc bulges.
    pub bisector: Option<P>,
    pub handle: P,
}

#[derive(Serialize)]
pub struct MoveView {
    pub from: u32,
    pub to: u32,
    #[serde(serialize_with = "word")]
    pub class: u8,
    pub allowed: bool,
    /// Why a measure stops this turn, if one does.
    pub blocked: Option<Said>,
    /// The turn is made the indirect way (L2).
    pub indirect: bool,
    pub lane: bool,
    pub path: Vec<Value>,
}

#[derive(Serialize)]
pub struct Conflicts {
    pub crossing: usize,
    pub merging: usize,
    pub diverging: usize,
    pub by_phase: bool,
}

#[derive(Serialize)]
pub struct RingView {
    pub radius_mm: i32,
    pub floor_mm: i32,
    /// The carriageway's outer radius, inside any cycle track.
    pub road_mm: i32,
    pub island_mm: i32,
    pub cycle_mm: Option<i32>,
    pub circulation: &'static str,
    /// A turbo's ring lanes, innermost first, two for each (one on each half of the ring, as open paths in the direction of traffic); empty for a plain roundabout.
    pub lanes: Vec<RingLane>,
    /// The centre line of each painted divider between two ring lanes, in open pieces, each between two streets (or a street and a tip).
    pub dividers: Vec<Vec<P>>,
    /// The edges, as closed paths of points that step along the axis at the two tips: the island, the carriageway, and the cycle track's outer edge (the carriageway's again when there is none).
    pub island_edge: Vec<P>,
    pub road_edge: Vec<P>,
    pub outer_edge: Vec<P>,
}

/// A turbo layout the roundabout could take.
#[derive(Serialize)]
pub struct TurboOption {
    pub layout: usize,
    pub id: &'static str,
    pub key: &'static str,
    pub on: bool,
}

/// The turbo a roundabout has.
#[derive(Serialize)]
pub struct TurboView {
    pub layout: usize,
    pub size: usize,
    pub layout_id: &'static str,
    pub layout_key: &'static str,
    /// The message that names the layout and size for a screen reader: "Basic turbo roundabout, { $size } size".
    pub label_key: &'static str,
    pub size_key: &'static str,
    pub main: Vec<u32>,
    pub can_switch_main: bool,
    /// Pressing the switch only moves the two entry lanes to the other street of the same pair (a knee): the next candidate is this one reversed.
    pub main_switch_moves_lanes: bool,
    pub can_smaller: bool,
    pub can_larger: bool,
}

#[derive(Serialize)]
pub struct BusView {
    pub from: u32,
    pub to: u32,
    pub poly: Vec<Value>,
    pub width_mm: i32,
}

#[derive(Serialize)]
pub struct BusOption {
    pub a: u32,
    pub b: u32,
    pub label: Said,
}

#[derive(Serialize)]
pub struct Selection {
    pub kind: Option<&'static str>,
    pub uid: u32,
    /// For a lane, its place among the arm's entering lanes.
    pub lane: usize,
}

/// A traffic signal's cycle and the phases of it, as the sheet reads them.
#[derive(Clone, Debug, Serialize)]
pub struct SignalView {
    pub cycle_s: u32,
    pub phases: Vec<PhaseView>,
    /// The plan is the suggestion: the person has not edited it.
    pub suggested: bool,
}

/// One street's turn: its numbers, and what runs with it besides its own traffic.
#[derive(Clone, Debug, Serialize)]
pub struct PhaseView {
    pub uid: u32,
    pub label: Said,
    pub street: Said,
    pub timing: crate::junction::model::Timing,
    pub length_s: u32,
    /// Crossings walked during the phase: the streets' names.
    pub walks: Option<Said>,
    /// Bike movements that run during the phase, besides this street's own.
    pub rides: Option<Said>,
    /// "Phase 2: Main Street", the streets whose traffic the phase serves.
    pub name: Said,
    /// The vehicle movements the phase serves, by street uid.
    pub moves: Vec<crate::junction::signal::Move>,
    /// The streets whose crossing is walked, and whose bike movements run, during the phase.
    pub walk_arms: Vec<u32>,
    pub ride_arms: Vec<u32>,
    /// What the phase allows, in a sentence by compass point: "Cars go: north. People cross: east and west."
    pub summary: Said,
    /// The ready-made pattern the phase is, Custom when it matches none.
    pub pattern: Pattern,
    /// The patterns that fit this junction's lanes, in the order they are offered (never Custom).
    pub patterns: Vec<PatternView>,
}

/// One ready-made pattern a phase can be given: the message keys of its name and its one-sentence description.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PatternView {
    pub pattern: Pattern,
    pub name: &'static str,
    pub description: &'static str,
}

/// The compass point an arm leaves by, said in full ("north-east").
fn compass_said(bearing: i32) -> Said {
    Said::new(crate::junction::turns::compass_name_key(bearing))
}

/// "a", "a and b", "a, b and c", each item a `Said`.
fn and_list(mut items: Vec<Said>) -> Option<Said> {
    let last = items.pop()?;
    let head =
        items.into_iter().reduce(|head, tail| Said::new("jn-list-comma").with("head", Arg::Said(Box::new(head))).with("tail", Arg::Said(Box::new(tail))));
    Some(match head {
        None => last,
        Some(head) => Said::new("jn-ph-and").with("head", Arg::Said(Box::new(head))).with("tail", Arg::Said(Box::new(last))),
    })
}

fn signal_view(s: &State, region: usize, arms: &[ArmView]) -> Option<SignalView> {
    let plan = signal::plan(s, region)?;
    let find = |uid: u32| arms.iter().find(|a| a.uid == uid);
    let name = |uid: u32| find(uid).map_or_else(|| crate::street::model::named_said(""), |a| a.label.clone());
    let phases = plan
        .phases
        .iter()
        .enumerate()
        .map(|(n, p)| PhaseView {
            name: phase_name(n, p, &find, &name),
            summary: phase_summary(p, arms),
            pattern: signal::pattern_of(s, region, p),
            patterns: signal::patterns_for(s, region, p)
                .into_iter()
                .map(|pattern| PatternView { pattern, name: pattern.name_key(), description: pattern.description_key() })
                .collect(),
            moves: p.moves.clone(),
            walk_arms: p.walks.clone(),
            ride_arms: p.rides.clone(),
            uid: p.uid,
            label: name(p.uid),
            street: find(p.uid).map_or_else(|| crate::street::model::named_said(""), |a| a.street.clone()),
            timing: p.timing,
            length_s: p.length_s,
            walks: list(p.walks.iter().copied().map(name).collect()),
            rides: list(p.rides.iter().copied().filter(|&u| u != p.uid).map(name).collect()),
        })
        .collect();
    Some(SignalView { cycle_s: plan.cycle_s, phases, suggested: plan.suggested })
}

/// "Phase 2: Main Street": the number, and the streets whose traffic goes in it.
fn phase_name<'a>(n: usize, p: &signal::Phase, find: &impl Fn(u32) -> Option<&'a ArmView>, name: &impl Fn(u32) -> Said) -> Said {
    let from = p.sources();
    let streets = and_list(from.into_iter().map(|u| find(u).map_or_else(|| name(u), |a| a.street.clone())).collect()).unwrap_or_else(|| name(p.uid));
    Said::new("jn-ph-name").with("n", Arg::Num(n as i64 + 1)).with("streets", Arg::Said(Box::new(streets)))
}

/// What a phase allows, by compass point: the cars that go, the crossings that are walked, the bikes that ride.
fn phase_summary(p: &signal::Phase, arms: &[ArmView]) -> Said {
    let compass = |uids: &[u32]| {
        let mut points: Vec<Said> = Vec::new();
        for a in uids.iter().filter_map(|u| arms.iter().find(|a| a.uid == *u)) {
            let said = compass_said(a.bearing);
            if !points.contains(&said) {
                points.push(said);
            }
        }
        and_list(points)
    };
    let from = p.sources();
    let part = |key: &'static str, uids: &[u32]| compass(uids).map(|a| Said::new(key).with("arms", Arg::Said(Box::new(a))));
    let parts = [part("jn-ph-cars", &from), part("jn-ph-walks", &p.walks), part("jn-ph-rides", &p.rides)];
    parts
        .into_iter()
        .flatten()
        .reduce(|head, tail| Said::new("jn-ph-join").with("head", Arg::Said(Box::new(head))).with("tail", Arg::Said(Box::new(tail))))
        .unwrap_or_else(|| Said::new("jn-sig-alone"))
}

#[derive(Serialize)]
pub struct JView {
    pub name: Said,
    /// A place in a city: its streets are the city's.
    pub linked: bool,
    /// A street can be taken away: the junction is not the city's and keeps three.
    pub can_remove: bool,
    pub region: &'static str,
    pub drive_side: &'static str,
    pub control: &'static str,
    pub control_index: usize,
    /// How many of the junction's streets carry traffic.
    pub traffic_arms: usize,
    pub ring: Option<RingView>,
    pub ring_extra_mm: i32,
    /// The turbo the roundabout has, if any.
    pub turbo: Option<TurboView>,
    /// The turbo layouts that suit the roundabout's streets (a layout whose arm count fits, and some main road makes a turbo that stands); empty off a roundabout.
    pub turbo_options: Vec<TurboOption>,
    /// The junction is a raised table.
    pub raised: bool,
    /// The bus lane across the middle of a roundabout, if there is one.
    pub bus: Option<BusView>,
    /// Pairs of streets it could join, the straightest first.
    pub bus_options: Vec<BusOption>,
    pub core: Vec<Value>,
    pub arms: Vec<ArmView>,
    pub corners: Vec<CornerView>,
    pub movements: Vec<MoveView>,
    pub conflicts: Conflicts,
    pub checks: Vec<Check>,
    /// The signal's cycle and phases, when the junction is run by a signal.
    pub signal: Option<SignalView>,
    pub revisions: Vec<Revision>,
    pub selected: Selection,
    pub bounds: [f64; 4],
    pub can_undo: bool,
    pub can_redo: bool,
    pub changed: bool,
    /// The OSM nodes the junction was made from.
    pub source: Vec<OsmRef>,
}

/// Every pair of streets a bus lane could join across a roundabout, the one
/// nearest a straight line first.
fn bus_options(s: &State) -> Vec<BusOption> {
    if s.control != ROUNDABOUT || s.turbo.is_some() {
        return Vec::new();
    }
    let mut v: Vec<(i32, BusOption)> = Vec::new();
    for (i, a) in s.arms.iter().enumerate() {
        for b in &s.arms[i + 1..] {
            let off = (gap(a.bearing, b.bearing) - 180).abs();
            v.push((off, BusOption { a: a.uid, b: b.uid, label: arm_to_arm(arm_name(a), arm_name(b)) }));
        }
    }
    v.sort_by_key(|(off, o)| (*off, o.a, o.b));
    v.into_iter().map(|(_, o)| o).collect()
}

fn heading(bearing: f64) -> i32 {
    (bearing + 180.0).rem_euclid(360.0).round() as i32
}

pub fn control_role(s: &State, i: usize) -> &'static str {
    match s.control {
        SIGNAL => "signal",
        ALL_WAY_STOP => "stop",
        ROUNDABOUT => "yield",
        PRIORITY => {
            if priority_pair(&s.arms).contains(&i) {
                "free"
            } else {
                "stop"
            }
        }
        _ => "free",
    }
}

/// The two arms nearest to a straight line make the priority street.
fn priority_pair(arms: &[Arm]) -> [usize; 2] {
    let n = arms.len();
    let traffic = arms.iter().filter(|a| !a.is_pedestrian()).count();
    let mut best = ([0, 1], i32::MAX);
    for i in 0..n {
        for j in i + 1..n {
            // A walkway has no traffic to give way; with fewer than two streets of traffic the pair is as it was.
            if traffic >= 2 && (arms[i].is_pedestrian() || arms[j].is_pedestrian()) {
                continue;
            }
            let d = (gap(arms[i].bearing, arms[j].bearing) - 180).abs();
            let wide = arms[i].row_mm() + arms[j].row_mm();
            let score = d * 100_000 - wide;
            if score < best.1 {
                best = ([i, j], score);
            }
        }
    }
    best.0
}

/// How far from the junction's edge the middle of a lane's arrow sits: past the
/// crossing and the stop line, so even the bigger arrows (`plan_svg`) clear them.
pub const ARROW_FROM_EDGE_MM: f64 = 5_500.0;

/// A lane along its arm from the mouth to `end`, where its piece ends. `t` is where its arrow stands.
fn lane_view(a: &ArmLayout, off: f64, x_mm: i32, t: f64, end: f64, head: i32, uses: u8, width: Option<i32>) -> LaneView {
    let w =
        width.unwrap_or_else(|| a.prof.pieces.iter().find(|p| p.x_mm + p.width_mm / 2 == x_mm && KINDS[p.kind].id == "travel").map_or(3_000, |p| p.width_mm));
    let c = a.lat(off, x_mm);
    LaneView {
        uses,
        dests: Vec::new(),
        bad: false,
        at: at(a.bearing, c, t),
        heading: head,
        poly: poly(&strip(a.bearing, c - w as f64 / 2.0, c + w as f64 / 2.0, a.mouth, end)),
        width_mm: w,
        fixed: false,
    }
}

/// The bearing, clockwise from north, of a point on the plan (north up, y
/// growing down), in whole degrees.
fn bearing_toward(x: f64, y: f64) -> i32 {
    x.atan2(-y).to_degrees().rem_euclid(360.0).round() as i32
}

impl Junction {
    /// Turns an arm to face a point on the plan, in millimetres from the
    /// junction's centre.
    pub fn drag_arm_to(&mut self, uid: u32, x: f64, y: f64) -> bool {
        self.set_bearing(uid, bearing_toward(x, y))
    }

    /// Sets the roundabout's outside radius, which cannot be less than its
    /// least size with these streets: smaller than that is the least size.
    pub fn set_ring_radius(&mut self, radius_mm: i32) -> bool {
        let Some(ring) = layout(self.current(), self.region).and_then(|l| l.ring) else { return false };
        self.set_ring((radius_mm - ring.floor).max(0))
    }

    /// Sets the curb radius at the corner clockwise of an arm from a point on
    /// the plan: the corner's handle stands `apex_per_radius` times the radius
    /// away from where its curbs would meet. A straight curb has no radius.
    pub fn drag_corner_to(&mut self, uid: u32, x: f64, y: f64) -> bool {
        let Some(i) = self.current().arms.iter().position(|a| a.uid == uid) else { return false };
        let Some(lay) = layout(self.current(), self.region) else { return false };
        let Some(c) = lay.corners.iter().find(|c| c.a == i) else { return false };
        let Some(f) = &c.fillet else { return false };
        let apex_per_radius = 1.0 / (c.delta as f64 / 2.0).to_radians().sin() - 1.0;
        self.set_corner(uid, (dist(f.c, (x, y)) / apex_per_radius).round() as i32)
    }

    /// Sets how far a crossing stands back from the junction from a point on
    /// the plan: its handle is on the far edge, the setback plus the crossing's
    /// width out along the arm from the mouth.
    pub fn drag_crossing_to(&mut self, uid: u32, x: f64, y: f64) -> bool {
        let Some(i) = self.current().arms.iter().position(|a| a.uid == uid) else { return false };
        let Some(c) = self.current().arms[i].crossing.as_ref() else { return false };
        let width = c.width_mm;
        let Some(lay) = layout(self.current(), self.region) else { return false };
        let a = &lay.arms[i];
        let mouth = at(a.bearing, self.current().arms[i].offset_mm as f64, a.mouth);
        let out = dir(a.bearing);
        let t = (x - mouth.0) * out.0 + (y - mouth.1) * out.1;
        self.set_setback(uid, (t - width as f64).round() as i32)
    }

    pub fn view(&self) -> JView {
        let in_force = in_force(self.current());
        let s: &State = &in_force;
        let region = self.region;
        // The current state is always valid; the layout it was checked with is
        // rebuilt here. A gesture may hold a state the layout rejects only if a
        // rule above was skipped, so fall back to the last good one.
        let lay = layout(s, region).expect("a junction state is always drawable");
        // A roundabout is never a table, whatever its state says.
        let raised = s.raised && lay.ring.is_none();
        // Which corners are protected: the flag of the arm before, where the corner can stand.
        let n_arms = s.arms.len();
        let blockers: Vec<Option<ProtectedBlocker>> =
            (0..n_arms).map(|i| protected_blocker(s, i, &lay.arms[i].prof, &lay.arms[(i + 1) % n_arms].prof)).collect();
        let protected_at: Vec<bool> = (0..n_arms).map(|i| s.arms[i].protected && blockers[i].is_none()).collect();
        let n = s.arms.len();
        let side = self.drive_side();
        let classes: Vec<u8> = (0..n).map(|i| classes_of(&s.arms, i)).collect();

        // Arms.
        let mut arms = Vec::new();
        let mut mins = (f64::MAX, f64::MAX);
        let mut maxs = (f64::MIN, f64::MIN);
        let mut widen = |p: P| {
            mins = (mins.0.min(p.0), mins.1.min(p.1));
            maxs = (maxs.0.max(p.0), maxs.1.max(p.1));
        };
        for (i, a) in s.arms.iter().enumerate() {
            let l = &lay.arms[i];
            let off = a.offset_mm as f64;
            let len = ARM_LENGTH_MM as f64;
            let cx = a.crossing;
            let walk = a.is_pedestrian();
            let far = l.mouth + cx.map_or(0.0, |c| (c.setback_mm + c.width_mm) as f64);
            let park_t = far + if cx.is_some() { PARK_CLEAR_MM } else { NO_CROSSING_CLEAR_MM };
            let clear_far = clear_far(a, l);
            // Where the end zone ends and what follows it, for an arm that has a beyond and draws it (a walkway and a
            // turbo's arms draw as they always did). Nothing of it is drawn nearer than where the roadway strips begin.
            let mut beyond = (!walk && s.turbo.is_none())
                .then(|| a.beyond.as_ref().zip(a.beyond_profile(region)))
                .flatten()
                .map(|(b, next)| beyond::draw(l.bearing, &|x| l.lat(off, x), &l.prof, &next, b, l.strip0, len));
            // The end of the strip of piece `pi`, which starts at `t0`.
            let cut = |pi: usize, t0: f64| beyond.as_ref().map_or(len, |d| d.cuts[pi].max(t0));
            let mut pieces = Vec::new();
            let mut gaps = Vec::new();
            let mut bulbs: [Option<Vec<Value>>; 2] = [None, None];
            let first_road = l.prof.pieces.iter().position(|p| is_roadway(p.kind));
            let last_road = l.prof.pieces.iter().rposition(|p| is_roadway(p.kind));
            for (pi, p) in l.prof.pieces.iter().enumerate() {
                let (x0, x1) = (l.lat(off, p.x_mm), l.lat(off, p.x_mm + p.width_mm));
                let road = is_roadway(p.kind);
                let inside = x0 >= l.cl - 1.0 && x1 <= l.cr + 1.0;
                let parks = matches!(KINDS[p.kind].id, "parking" | "loading");
                let t0 = if parks {
                    park_t
                } else if road {
                    l.strip0
                } else if inside {
                    clear_far
                } else if KINDS[p.kind].id == "sidewalk" {
                    l.mouth
                } else {
                    clear_far
                };
                let t0 = if walk { l.mouth } else { t0 };
                if inside && !road && !walk {
                    // The median stops short of the junction; the road runs on beside it.
                    gaps.push(poly(&strip(l.bearing, x0, x1, l.strip0, t0)));
                }
                if parks {
                    let side_ix = usize::from(Some(pi) != first_road);
                    if Some(pi) == first_road || Some(pi) == last_road {
                        let poly = poly(&strip(l.bearing, x0, x1, l.strip0, park_t));
                        if a.bulb[side_ix] {
                            bulbs[side_ix] = Some(poly);
                        } else {
                            gaps.push(poly);
                        }
                    }
                }
                pieces.push(PieceView {
                    kind: p.kind,
                    material: p.material,
                    direction: p.direction,
                    poly: poly(&strip(l.bearing, x0, x1, t0, cut(pi, t0))),
                    taper: false,
                });
            }
            let mut core_bike = Vec::new();
            if lay.ring.is_none() {
                let sides = l.prof.outer_bike_sides();
                let (left_protected, right_protected) = (protected_at[(i + n_arms - 1) % n_arms], protected_at[i]);
                for pi in l.prof.outer_bike() {
                    let (on_left, on_right) = (sides[0] == Some(pi), sides[1] == Some(pi));
                    let p = &l.prof.pieces[pi];
                    let (x0, x1) = (l.lat(off, p.x_mm), l.lat(off, p.x_mm + p.width_mm));
                    // The corner's track replaces the stub on the sides it runs round, but the lane still has to reach
                    // the track: where the track starts nearer the middle than the mouth, a lead-in joins them.
                    if (!on_left || left_protected) && (!on_right || right_protected) {
                        let heading = dir(l.bearing);
                        let along = |p: P| p.0 * heading.0 + p.1 * heading.1;
                        let left = lay.corners[(i + n_arms - 1) % n_arms].fillet.as_ref().map(|f| along(f.b));
                        let right = lay.corners[i].fillet.as_ref().map(|f| along(f.a));
                        let start = [left.filter(|_| on_left), right.filter(|_| on_right)].into_iter().flatten().fold(f64::MAX, f64::min);
                        if start < l.strip0 - 1.0 {
                            core_bike.push(poly(&strip(l.bearing, x0, x1, start, l.strip0)));
                        }
                        continue;
                    }
                    core_bike.push(poly(&strip(l.bearing, x0, x1, (l.strip0 - CORE_BIKE_MM).max(0.0), l.strip0)));
                }
            }
            let table = (raised && !walk)
                .then(|| RaisedView { top: poly(&strip(l.bearing, l.cl, l.cr, l.mouth, far)), ramp: poly(&strip(l.bearing, l.cl, l.cr, far, far + RAMP_MM)) });
            for c in strip(l.bearing, l.pl, l.pr, len, len) {
                widen(c);
            }
            widen(at(l.bearing, l.pl, len));
            widen(at(l.bearing, l.pr, len));

            // Crossing.
            let bulb_l = if a.bulb[0] { l.prof.park[0] as f64 } else { 0.0 };
            let bulb_r = if a.bulb[1] { l.prof.park[1] as f64 } else { 0.0 };
            let (cl, cr) = (l.cl + bulb_l, l.cr - bulb_r);
            let crossing_present = cx.is_some();
            let crossing = cx.map(|c| {
                let t0 = l.mouth + c.setback_mm as f64;
                let t1 = t0 + c.width_mm as f64;
                let distance = (cr - cl).round() as i32;
                let island = c.island.then(|| {
                    let mid = (cl + cr) / 2.0;
                    poly(&strip(l.bearing, mid - ISLAND_MM as f64 / 2.0, mid + ISLAND_MM as f64 / 2.0, t0 - 1_000.0, t1 + 1_000.0))
                });
                CrossingView {
                    setback_mm: c.setback_mm,
                    width_mm: c.width_mm,
                    island: c.island,
                    poly: poly(&strip(l.bearing, cl, cr, t0, t1)),
                    island_poly: island,
                    continuous: c.continuous,
                    ramps: if c.continuous && !raised {
                        vec![poly(&strip(l.bearing, cl, cr, (t0 - RAMP_MM).max(l.mouth), t0)), poly(&strip(l.bearing, cl, cr, t1, t1 + RAMP_MM))]
                    } else {
                        Vec::new()
                    },
                    handle: at(l.bearing, (cl + cr) / 2.0 - (cr - cl) * 0.28, t1),
                    distance_mm: distance,
                    stage_mm: if c.island { (distance - ISLAND_MM) / 2 } else { distance },
                    stages: if c.island { 2 } else { 1 },
                    too_far: (if c.island { (distance - ISLAND_MM) / 2 } else { distance }) > MAX_STAGE_MM,
                }
            });
            // The beyond the approach check judges and the arm sentence describes: the one that is drawn, not one that
            // lies past the end of the drawn arm.
            let judged = beyond.as_ref().filter(|d| d.shown).and_then(|_| a.beyond.as_ref());
            let enters = !l.prof.enter_x.is_empty();
            let leaves = !l.prof.leave_x.is_empty();
            let blocker = bike_box_blocker(s, i, &l.prof);
            let boxed = a.bike_box && blocker.is_none();
            let box_mm = if boxed { BIKE_BOX_MM } else { 0.0 };
            let lat = |x: i32| l.lat(off, x);
            let stop_line = ((cx.is_some() || boxed) && enters).then(|| {
                let t = far + 600.0 + box_mm;
                [at(l.bearing, lat(l.prof.enter_span.0), t), at(l.bearing, lat(l.prof.enter_span.1), t)]
            });
            let bike_box = boxed.then(|| poly(&strip(l.bearing, lat(l.prof.enter_span.0), lat(l.prof.enter_span.1), far + 600.0, far + 600.0 + BIKE_BOX_MM)));
            let can_bike_box = blocker.is_none();
            // The street's stops that have a part on the drawn arm, each judged against what reaches out from the
            // centre (the crossing's far edge, the mouth with no crossing). A stop's pieces are the end zone's, which
            // `l.prof` is.
            let mut stops: Vec<StopView> = if walk || s.turbo.is_some() {
                Vec::new()
            } else {
                a.stops
                    .iter()
                    .filter(|e| f64::from(e.near_mm) < len)
                    .map(|e| {
                        let mut v = stops::draw(l, off, e, (cx.is_some() || boxed).then_some(far + 600.0 + box_mm), &|pi| cut(pi, l.mouth), len);
                        v.gap_mm = stop_gap_mm(e.near_mm, far.round() as i32);
                        v
                    })
                    .collect()
            };
            let arrow_t = if cx.is_some() { far + ARROW_FROM_EDGE_MM } else { l.mouth + ARROW_FROM_EDGE_MM + 1_000.0 };
            // Where the lane at `x` ends: where its piece does.
            let lane_end = |x: i32| {
                let piece = l.prof.pieces.iter().position(|p| p.x_mm <= x && x < p.x_mm + p.width_mm);
                piece.map_or(len, |pi| cut(pi, l.mouth))
            };
            let lanes: Vec<LaneView> = l
                .prof
                .enter_x
                .iter()
                .zip(&a.lanes)
                .map(|(&x, lane)| {
                    let uses = lane.to.iter().filter_map(|t| s.arms.iter().find(|b| b.uid == *t)).fold(0, |m, b| m | turn_class(a.bearing, b.bearing));
                    let mut v = lane_view(l, off, x, arrow_t + box_mm, lane_end(x), heading(l.bearing), uses, l.lane_mm);
                    v.fixed = s.turbo.is_some();
                    let mut dests: Vec<(i32, DestView)> = s
                        .arms
                        .iter()
                        .zip(&lay.arms)
                        .enumerate()
                        .filter(|(_, (b, _))| b.uid != a.uid && !b.is_pedestrian())
                        .map(|(j, (b, lb))| {
                            let turn = (b.bearing - a.bearing).rem_euclid(360) - 180;
                            (
                                turn,
                                DestView {
                                    uid: b.uid,
                                    label: arm_name(b),
                                    class: turn_class(a.bearing, b.bearing),
                                    on: lane.to.contains(&b.uid),
                                    open: !lb.prof.leave_x.is_empty() && blocked(&s.arms, i, j).is_none(),
                                },
                            )
                        })
                        .collect();
                    dests.sort_by_key(|d| d.0);
                    v.dests = dests.into_iter().map(|d| d.1).collect();
                    v
                })
                .collect();
            let (stop_clear, stop_lane) =
                judge_stops(&mut stops, &arm_name(a), if cx.is_some() { "jn-edge-crossing" } else { "jn-edge-mouth" }, &lanes, classes[i], side);
            let leave_arrows = l.prof.leave_x.iter().map(|&x| lane_view(l, off, x, arrow_t, lane_end(x), l.bearing.round() as i32, 0, None)).collect();
            let road_mm = l.prof.road_mm();
            let turbo_entry = l.entry.filter(|e| e.new.0 < e.old.0 - 1.0 || e.new.1 > e.old.1 + 1.0).map(|e| {
                // The widened lanes run straight out over the approach length, then flare back to the street's width, 1 m sideways in 10 along.
                let t1 = (l.mouth + APPROACH_DEFAULT_MM as f64).min(len);
                let flare = (e.old.0 - e.new.0).abs().max((e.new.1 - e.old.1).abs()) * turbo::FLARE as f64;
                let t2 = (t1 + flare).min(len);
                let f = (t2 - t1) / flare;
                let spread = |new: f64, old: f64| new + (old - new) * f;
                poly(&[
                    at(l.bearing, e.new.0, l.strip0),
                    at(l.bearing, e.new.1, l.strip0),
                    at(l.bearing, e.new.1, t1),
                    at(l.bearing, spread(e.new.1, e.old.1), t2),
                    at(l.bearing, spread(e.new.0, e.old.0), t2),
                    at(l.bearing, e.new.0, t1),
                ])
            });
            arms.push(ArmView {
                uid: a.uid,
                edge: a.edge,
                label: arm_name(a),
                street: l.prof.name.clone(),
                bearing: a.bearing,
                offset_mm: a.offset_mm,
                corner_mm: a.corner_mm,
                pieces,
                beyond: beyond.as_mut().map_or_else(Vec::new, |d| d.taper.drain(..).chain(d.extra.drain(..)).collect()),
                beyond_marks: beyond.as_mut().map_or_else(Vec::new, |d| std::mem::take(&mut d.marks)),
                beyond_problems: judged.map_or_else(Vec::new, |b| approach_problems(&arm_name(a), b, arrow_t + box_mm)),
                beyond_reach_mm: judged.map_or(0, steady_mm),
                beyond_taper_mm: judged.and_then(|b| b.taper.as_ref()).map_or(0, |t| t.to_mm - t.from_mm),
                beyond_zone: judged.map(|b| b.zone.clone()),
                core_bike,
                raised: table,
                gaps,
                bulbs,
                crossing,
                stop_line,
                bike_box,
                can_bike_box,
                bike_box_why: blocker.map(BikeBoxBlocker::key),
                lanes,
                leave_arrows,
                turbo_entry,
                role: control_role(s, i),
                outline: poly(&strip(l.bearing, l.pl, l.pr, l.mouth, len)),
                end: at(l.bearing, off, len),
                mouth_at: at(l.bearing, off, l.mouth),
                banned: a.banned.clone(),
                classes: classes[i],
                road_mm,
                can_bulb: l.prof.park.map(|p| p > 0),
                can_island: road_mm >= ISLAND_MIN_ROAD_MM,
                max_offset_mm: road_mm / 2,
                enters,
                leaves,
                pedestrian: a.is_pedestrian(),
                transit: transit_view(s, a, l, off, far, crossing_present, side),
                stops,
                stop_clear,
                stop_lane,
            });
        }

        // Corners, the pavement wedges and the carriageway core.
        let mut corners = Vec::new();
        let mut core = Vec::new();
        let ring = lay.ring.as_ref();
        // Where each street's carriageway meets the ring: the dividers are cut there.
        let mouths: Vec<Mouth> = lay
            .arms
            .iter()
            .zip(&s.arms)
            .filter(|(_, a)| !a.is_pedestrian())
            .map(|(lay_arm, _)| Mouth { bearing: lay_arm.bearing, cl: lay_arm.cl, cr: lay_arm.cr })
            .collect();
        let turbo_shape: Option<RingShape> = ring.and_then(|r| r.turbo.as_ref()).and_then(|t| {
            BLOCKS.get(t.block).map(|b| ring_shape(b, t.lanes, s.cycle.unwrap_or(0), &Axes::new(t.axis_deg, side == Side::Right), side == Side::Right, &mouths))
        });
        for c in &lay.corners {
            let (la, lb) = (&lay.arms[c.a], &lay.arms[c.b]);
            let (fa, fb) = (clear_far(&s.arms[c.a], la), clear_far(&s.arms[c.b], lb));
            // The curb's two ends, where it leaves each arm's straight edge.
            let (ca_in, cb_in, curb_mid): (P, P, Vec<Value>) = match (&c.fillet, ring) {
                (Some(f), _) => (f.a, f.b, vec![arc(f.r, false, cross(sub(f.a, f.o), sub(f.b, f.o)) > 0.0, f.b)]),
                (None, Some(r)) => match &r.turbo {
                    Some(t) => {
                        // The curb walks the outline of the ring from the first street's right curb to the next street's left.
                        let hit = |a: &ArmLayout, lat: f64| at(a.bearing, lat, mouth_on(&t.outline, a.bearing, lat).unwrap_or(0.0));
                        let (pa, qb) = (hit(la, la.cr), hit(lb, lb.cl));
                        (pa, qb, follow(&t.outline, pa, qb, true).into_iter().map(l_).collect())
                    }
                    None => {
                        let pa = at(la.bearing, la.cr, on_circle(la.cr, r.radius).unwrap_or(0.0));
                        let qb = at(lb.bearing, lb.cl, on_circle(lb.cl, r.radius).unwrap_or(0.0));
                        (pa, qb, vec![ring_arc(r.radius, pa, qb)])
                    }
                },
                (None, None) => (at(la.bearing, la.cr, la.mouth), at(lb.bearing, lb.cl, lb.mouth), Vec::new()),
            };
            let curb_a_far = at(la.bearing, la.cr, fa);
            let curb_b_far = at(lb.bearing, lb.cl, fb);
            let mut curb = vec![m(curb_a_far), l_(ca_in)];
            curb.extend(if c.fillet.is_none() && ring.is_none() { vec![l_(cb_in)] } else { curb_mid.clone() });
            curb.push(l_(curb_b_far));
            let prop_a = at(la.bearing, la.pr, fa);
            let prop_b = at(lb.bearing, lb.pl, fb);
            let mut wedge = curb.clone();
            wedge.push(l_(prop_b));
            let corner_prop = meet(la.bearing, la.pr, lb.bearing, lb.pl).map(|(t, _)| at(la.bearing, la.pr, t));
            match corner_prop {
                Some(p) => wedge.push(l_(p)),
                None => {
                    wedge.push(l_(at(lb.bearing, lb.pl, lb.mouth)));
                    wedge.push(l_(at(la.bearing, la.pr, la.mouth)));
                }
            }
            wedge.push(l_(prop_a));
            wedge.push(z());
            let ua = s.arms[c.a].uid;
            let ub = s.arms[c.b].uid;
            let radius = s.arms[c.a].corner_mm;
            let (handle, bis, ok) = match &c.fillet {
                Some(f) => {
                    let mid = sub(f.o, scale(f.u, f.r));
                    let margin = corner_prop.map_or(f64::MAX, |p| dist(mid, p));
                    (mid, Some(f.u), margin >= MIN_SIDEWALK_AT_CORNER_MM)
                }
                None => (scale(add(ca_in, cb_in), 0.5), None, true),
            };
            let protected = match (&c.fillet, protected_at[c.a]) {
                (Some(f), true) => {
                    let sides = (la.prof.curb_bike_sides()[1], lb.prof.curb_bike_sides()[0]);
                    let width = |prof: &Profile, i: Option<usize>| i.map_or(0, |i| prof.pieces[i].width_mm) as f64;
                    let w = width(&la.prof, sides.0).min(width(&lb.prof, sides.1));
                    let sweep = cross(sub(f.a, f.o), sub(f.b, f.o)) > 0.0;
                    // The point on the ray from the fillet's centre through `p`, at radius `r`.
                    let on = |p: P, r: f64| add(f.o, scale(sub(p, f.o), r / f.r));
                    let (r1, r2) = (f.r + w, f.r + w + protected_island_mm(&la.prof, &lb.prof) as f64);
                    let (a1, b1, a2, b2) = (on(f.a, r1), on(f.b, r1), on(f.a, r2), on(f.b, r2));
                    Some(ProtectedView {
                        track: vec![m(f.a), arc(f.r, false, sweep, f.b), l_(b1), arc(r1, false, !sweep, a1), z()],
                        island: vec![m(a1), arc(r1, false, sweep, b1), l_(b2), arc(r2, false, !sweep, a2), z()],
                    })
                }
                _ => None,
            };
            corners.push(CornerView {
                uid: ua,
                next_uid: ub,
                radius_mm: radius,
                speed_kmh: if c.fillet.is_some() { speed_kmh(radius) } else { 0.0 },
                ok,
                fast: c.fillet.is_some()
                    && speed_kmh(radius)
                        > if raised || [&s.arms[c.a], &s.arms[c.b]].iter().all(|a| a.crossing.is_none_or(|x| x.continuous)) {
                            RAISED_TURN_KMH
                        } else {
                            MAX_TURN_KMH
                        }
                    && (s.arms[c.a].crossing.is_some() || s.arms[c.b].crossing.is_some())
                    && !s.arms[c.a].is_pedestrian()
                    && !s.arms[c.b].is_pedestrian(),
                straight: c.fillet.is_none(),
                walk: is_walk(la.prof.edge_kind[1]) || is_walk(lb.prof.edge_kind[0]),
                protected,
                protected_why: blockers[c.a].map(ProtectedBlocker::key),
                wedge,
                curb,
                bisector: bis,
                handle,
            });
        }
        if ring.is_none() {
            let m0 = |a: &ArmLayout, side_l: bool| at(a.bearing, if side_l { a.cl } else { a.cr }, a.mouth);
            core.push(m(m0(&lay.arms[0], true)));
            for (i, c) in lay.corners.iter().enumerate() {
                core.push(l_(m0(&lay.arms[i], false)));
                if let Some(f) = &c.fillet {
                    core.push(l_(f.a));
                    core.push(arc(f.r, false, cross(sub(f.a, f.o), sub(f.b, f.o)) > 0.0, f.b));
                }
                if c.b != 0 {
                    core.push(l_(m0(&lay.arms[c.b], true)));
                }
            }
            core.push(z());
        }
        for c in &corners {
            widen(c.handle);
        }

        // Movements.
        let mut movements = Vec::new();
        let rad = ring.map(|r| r.radius - s.cycle.unwrap_or(0) as f64);
        let right_hand = side == Side::Right;
        for (i, a) in s.arms.iter().enumerate() {
            for (j, b) in s.arms.iter().enumerate() {
                if i == j {
                    continue;
                }
                let (la, lb) = (&lay.arms[i], &lay.arms[j]);
                let lane_k = turbo_shape.as_ref().and_then(|_| serving_lane(a, b.uid, right_hand));
                let ea = lane_k.and_then(|k| la.prof.enter_x.get(k).copied()).or_else(|| span_mid(la.prof.enter_span));
                let (Some(ea), Some(xb)) = (ea, span_mid(lb.prof.leave_span)) else { continue };
                let pe = at(la.bearing, la.lat(a.offset_mm as f64, ea), la.mouth);
                let px = at(lb.bearing, lb.lat(b.offset_mm as f64, xb), lb.mouth);
                let blocked = blocked(&s.arms, i, j);
                let allowed = !a.banned.contains(&b.uid) && blocked.is_none();
                let class = turn_class(a.bearing, b.bearing);
                let lane = a.lanes.iter().any(|l| l.to.contains(&b.uid));
                let indirect = a.rule == RULE_WITHIN && class == LEFT && rad.is_none();
                let path = if indirect {
                    // Straight on past the middle, then back to the street on the left.
                    let w = at(la.bearing, la.lat(a.offset_mm as f64, ea), -0.4 * la.mouth);
                    vec![m(pe), l(w), quad(scale(add(w, px), 0.5), px)]
                } else if let Some((shape, k)) = turbo_shape.as_ref().map(|shape| {
                    // The entry lane feeds a ring lane; a movement no lane serves takes the innermost.
                    (shape, lane_k.map_or(0, |k| ring_lane(k, a.lanes.len(), shape.ring_lanes(), right_hand)))
                }) {
                    turbo_path(la, lb, pe, px, la.lat(a.offset_mm as f64, ea), lb.lat(b.offset_mm as f64, xb), k, shape)
                } else {
                    movement_path(la.bearing, pe, lb.bearing, px, rad, side, a.bearing, b.bearing)
                };
                movements.push(MoveView { from: a.uid, to: b.uid, class, allowed, blocked, indirect, lane, path });
            }
        }
        for (a, arm) in arms.iter_mut().zip(&s.arms) {
            for (lane, l) in a.lanes.iter_mut().zip(&arm.lanes) {
                lane.bad = !l.to.is_empty() && !l.to.iter().any(|t| movements.iter().any(|m| m.from == arm.uid && m.to == *t && m.allowed));
            }
        }
        let conflicts = conflicts(s, &movements, &lay, side);

        let checks = checks(s, self.region, &arms, &corners, &movements, &lay);
        let signal = signal_view(s, self.region, &arms);
        let selected = match self.selected {
            Target::None => Selection { kind: None, uid: 0, lane: 0 },
            Target::Arm(u) => Selection { kind: Some("arm"), uid: u, lane: 0 },
            Target::Corner(u) => Selection { kind: Some("corner"), uid: u, lane: 0 },
            Target::Crossing(u) => Selection { kind: Some("crossing"), uid: u, lane: 0 },
            Target::Lane(u, i) => Selection { kind: Some("lane"), uid: u, lane: i },
            Target::Bus => Selection { kind: Some("bus"), uid: 0, lane: 0 },
            Target::Cycle => Selection { kind: Some("cycle"), uid: 0, lane: 0 },
        };
        JView {
            name: self.name().clone(),
            linked: self.is_linked(),
            can_remove: !self.is_linked() && n > MIN_ARMS,
            region: REGIONS[region].id,
            drive_side: if side == Side::Left { "left" } else { "right" },
            control: CONTROLS[s.control].id,
            control_index: s.control,
            traffic_arms: traffic_arms(s),
            ring: ring.map(|r| {
                let cycle = s.cycle.unwrap_or(0);
                let (lanes, dividers, island_edge, road_edge, outer_edge) = match &turbo_shape {
                    Some(x) => (x.lanes.clone(), x.dividers.clone(), x.island.clone(), x.road.clone(), x.outer.clone()),
                    None => Default::default(),
                };
                RingView {
                    radius_mm: r.radius as i32,
                    floor_mm: r.floor,
                    road_mm: r.radius as i32 - cycle,
                    island_mm: match &r.turbo {
                        Some(t) => BLOCKS.get(t.block).map_or(MIN_ISLAND_MM as i32, |b| b.r1_mm),
                        None => (r.radius - cycle as f64 - RING_WIDTH_MM).max(MIN_ISLAND_MM) as i32,
                    },
                    cycle_mm: s.cycle,
                    circulation: if side == Side::Right { "anticlockwise" } else { "clockwise" },
                    lanes,
                    dividers,
                    island_edge,
                    road_edge,
                    outer_edge,
                }
            }),
            ring_extra_mm: s.ring_extra_mm,
            turbo: s.turbo.as_ref().and_then(|t| {
                let (spec, block) = (t.spec()?, t.block()?);
                Some(TurboView {
                    layout: t.layout,
                    size: t.size,
                    layout_id: spec.id,
                    layout_key: spec.key,
                    label_key: spec.label_key,
                    size_key: block.key,
                    main: t.main.clone(),
                    can_switch_main: next_main(s, region).is_some_and(|(_, accepted)| accepted),
                    main_switch_moves_lanes: next_main(s, region).is_some_and(|(n, _)| n.len() == 2 && n[0] == t.main[1] && n[1] == t.main[0]),
                    can_smaller: t.size > 0,
                    can_larger: t.size + 1 < BLOCKS.len(),
                })
            }),
            turbo_options: if s.control == ROUNDABOUT {
                layouts_for(s.arms.len())
                    .into_iter()
                    .filter(|&i| choose_turbo(s, region, i).is_some_and(|(_, fits)| fits))
                    .map(|i| TurboOption { layout: i, id: LAYOUTS[i].id, key: LAYOUTS[i].key, on: s.turbo.as_ref().is_some_and(|t| t.layout == i) })
                    .collect()
            } else {
                Vec::new()
            },
            raised,
            bus: s.bus.and_then(|(a, b)| {
                let ends = |u: u32| s.arms.iter().position(|x| x.uid == u).map(|i| at(lay.arms[i].bearing, s.arms[i].offset_mm as f64, lay.arms[i].mouth));
                let (pa, pb) = (ends(a)?, ends(b)?);
                let d = sub(pb, pa);
                let len = (d.0 * d.0 + d.1 * d.1).sqrt().max(1.0);
                let n = (-d.1 / len * BUS_LANE_MM as f64 / 2.0, d.0 / len * BUS_LANE_MM as f64 / 2.0);
                Some(BusView { from: a, to: b, poly: poly(&[add(pa, n), add(pb, n), sub(pb, n), sub(pa, n)]), width_mm: BUS_LANE_MM })
            }),
            bus_options: bus_options(s),
            core,
            arms,
            corners,
            movements,
            conflicts,
            checks,
            signal,
            revisions: self.revisions().enumerate().map(|(i, l)| Revision { step: i + 1, label: l.clone() }).collect(),
            selected,
            bounds: [mins.0, mins.1, maxs.0, maxs.1],
            can_undo: self.can_undo(),
            can_redo: self.can_redo(),
            changed: self.changed(),
            source: s.source.clone(),
        }
    }
}

/// Whether the piece next to a curb is a sidewalk.
fn is_walk(kind: Option<usize>) -> bool {
    kind.is_some_and(|k| KINDS[k].id == "sidewalk")
}

/// How far out the plain pavement of the corner runs: past the crossing, where
/// planting and the median take over.
fn clear_far(a: &Arm, l: &ArmLayout) -> f64 {
    let crossing = a.crossing.map_or(0.0, |c| (c.setback_mm + c.width_mm) as f64);
    (l.mouth + crossing + 1_500.0).max(l.mouth + WEDGE_MM)
}

/// Judges the street's stops on an arm: those too near what reaches out from the centre (`edge` names it), and the
/// near-side stops whose lane would hold the buses behind a turn (`lanes` are the arm's entering lanes, `classes` the
/// turns it has, `side` the side of the road traffic keeps to). Sets `bad` on each stop a finding names.
fn judge_stops(stops: &mut [StopView], arm: &Said, edge: &'static str, lanes: &[LaneView], classes: u8, side: Side) -> (Vec<Said>, Vec<Said>) {
    let (mut clear, mut lane) = (Vec::new(), Vec::new());
    let curb_turn = if side == Side::Right { RIGHT } else { LEFT };
    for st in stops {
        let said = |key: &'static str| Said::new(key).with("kind", Arg::MsgLower(st.kind.name_key())).with("arm", Arg::Said(Box::new(arm.clone())));
        if st.gap_mm < STOP_CLEAR_MM {
            clear.push(said("jn-stop-too-near").with("gap", Arg::Length(st.gap_mm)).with("need", Arg::Length(STOP_CLEAR_MM)).with("edge", Arg::Msg(edge)));
            st.bad = true;
        }
        // A bus piece has no lane here, and a one-lane approach has no other lane to be in.
        let Some((n, uses)) = st.lane.filter(|_| st.enters && lanes.len() >= 2).and_then(|n| lanes.get(n).map(|l| (n, l.uses))) else { continue };
        let finding = if uses != 0 && uses & THROUGH == 0 && classes & THROUGH != 0 {
            stops::turns_said(uses).map(|t| said("jn-stop-turn-only").with("turn", Arg::Said(Box::new(t))))
        } else if uses & THROUGH != 0 && uses & curb_turn != 0 {
            stops::turns_said(curb_turn).map(|t| said("jn-stop-shared-turn").with("turn", Arg::Said(Box::new(t))))
        } else {
            None
        };
        if let Some(f) = finding {
            lane.push(f.with("n", Arg::Num(n as i64 + 1)));
            st.bad = true;
        }
    }
    (clear, lane)
}

/// Places the transit measures on an arm and lists what each one is missing.
fn transit_view(s: &State, a: &Arm, l: &ArmLayout, off: f64, far: f64, has_crossing: bool, side: Side) -> TransitView {
    let len = ARM_LENGTH_MM as f64;
    let kerb_side = usize::from(side == Side::Left);
    let (ke, sg) = if kerb_side == 0 { (l.cl, 1.0) } else { (l.cr, -1.0) };
    let park = l.prof.park[kerb_side] as f64;
    let lane_w = TRANSIT_LANE_MM as f64;
    let span = |from: f64, to: f64| (ke + sg * from).min(ke + sg * to)..(ke + sg * from).max(ke + sg * to);
    let range = |from: f64, to: f64| {
        let r = span(from, to);
        (r.start, r.end)
    };
    let stop_t = far + 600.0;
    // However far a measure reaches, it stays on the arm as drawn.
    let reach = |t: f64| t.min(len - 1_500.0);
    let mut problems = Vec::new();
    let problem = |key: &'static str| Said::new(key).with("arm", Arg::Said(Box::new(arm_name(a))));

    let (lo, hi) = range(0.0, lane_w);
    let bus = a.bus_lane.then(|| poly(&strip(l.bearing, lo, hi, l.mouth, len)));

    let queue_len = a.approach_mm as f64;
    let queue_pts = match a.approach {
        Q_OFFSET => {
            let (lo, hi) = range(park, park + lane_w);
            if park == 0.0 {
                problems.push(problem("jn-problem-offset-parking"));
            }
            Some(strip(l.bearing, lo, hi, l.mouth, reach(stop_t + queue_len)))
        }
        Q_CURB => {
            let (lo, hi) = range(0.0, lane_w);
            Some(strip(l.bearing, lo, hi, l.mouth, reach(stop_t + queue_len)))
        }
        _ => None,
    };
    let queue = queue_pts.as_ref().map(|p| poly(p));
    let loop_pts = (a.approach == Q_VIRTUAL).then(|| {
        if s.control != SIGNAL {
            problems.push(problem("jn-problem-virtual-signal"));
        }
        let (lo, hi) = range(0.0, lane_w);
        strip(l.bearing, lo, hi, stop_t + 2_000.0, stop_t + 5_000.0)
    });
    let virtual_loop = loop_pts.as_ref().map(|p| poly(p));
    let gate = matches!(a.approach, GATE_SIGNAL | GATE_YIELD).then(|| {
        if !a.bus_lane {
            problems.push(problem("jn-problem-gate-bus-lane"));
        }
        if l.prof.enter_x.len() < 2 && a.bus_lane {
            problems.push(problem("jn-problem-gate-lane"));
        }
        let t = reach(stop_t + queue_len);
        [at(l.bearing, l.lat(off, l.prof.enter_span.0), t), at(l.bearing, l.lat(off, l.prof.enter_span.1), t)]
    });

    let (stop_poly, stop_at) = match a.stop {
        STOP_BULB => {
            if park == 0.0 {
                problems.push(problem("jn-problem-bulb-parking"));
            }
            let w = if park > 0.0 { park } else { 2_400.0 };
            let (lo, hi) = range(0.0, w);
            (Some(poly(&strip(l.bearing, lo, hi, far + 3_000.0, far + 11_000.0))), Some(at(l.bearing, (lo + hi) / 2.0, far + 7_000.0)))
        }
        STOP_PLATFORM => {
            if s.control != SIGNAL {
                problems.push(problem("jn-problem-platform-signal"));
            }
            if !has_crossing {
                problems.push(problem("jn-problem-platform-crossing"));
            }
            let (lo, hi) = range(lane_w, lane_w + 2_500.0);
            (Some(poly(&strip(l.bearing, lo, hi, far + 3_000.0, far + 11_000.0))), Some(at(l.bearing, (lo + hi) / 2.0, far + 7_000.0)))
        }
        _ => (None, None),
    };

    // A measure on the kerb side and a stop of the street on that side are two stops where there should be one.
    let kerb = if kerb_side == 0 { Side::Left } else { Side::Right };
    if a.stop != 0 && s.turbo.is_none() && !a.is_pedestrian() && a.stops.iter().any(|e| e.side == kerb && f64::from(e.near_mm) < len) {
        problems.push(problem("jn-problem-stop-twice").with("measure", Arg::Measure(STOPS[a.stop].code)));
    }

    let mid = (l.cl + l.cr) / 2.0;
    let bollards = if a.filter {
        if !a.bus_lane {
            problems.push(problem("jn-problem-filter-bus-lane"));
        }
        let mut v = Vec::new();
        let mut x = l.cl + 900.0;
        while x < l.cr {
            let in_bus = a.bus_lane && x >= lo.min(hi) - 100.0 && x <= lo.max(hi) + 100.0;
            if !in_bus {
                v.push(at(l.bearing, x, l.mouth + 10_000.0));
            }
            x += 1_800.0;
        }
        v
    } else {
        Vec::new()
    };
    let cap = (a.rule == RULE_DEAD_END).then(|| [at(l.bearing, l.cl, l.mouth + 6_000.0), at(l.bearing, l.cr, l.mouth + 6_000.0)]);
    let island = (a.rule == RULE_RIRO).then(|| poly(&strip(l.bearing, mid - 900.0, mid + 900.0, l.mouth + 1_500.0, l.mouth + 9_000.0)));
    let _ = (&queue_len, off);
    let centre = |pts: &[P]| scale(pts.iter().fold((0.0, 0.0), |acc, p| add(acc, *p)), 1.0 / pts.len() as f64);
    let icon_at = at(l.bearing, mid, l.mouth + 14_000.0);
    // Where each measure is labelled, in the order they are drawn.
    let mut tags = Vec::new();
    let approach_code = APPROACHES[a.approach].code;
    if let Some(p) = &queue_pts {
        tags.push(MeasureTag { code: approach_code, at: centre(p) });
    }
    if let Some(p) = &loop_pts {
        tags.push(MeasureTag { code: approach_code, at: centre(p) });
    }
    if let Some(g) = &gate {
        tags.push(MeasureTag { code: approach_code, at: scale(add(g[0], g[1]), 0.5) });
    }
    if let (Some(_), Some(p)) = (&stop_poly, stop_at) {
        tags.push(MeasureTag { code: STOPS[a.stop].code, at: p });
    }
    if let (true, Some(p)) = (a.filter, bollards.get(bollards.len() / 2)) {
        tags.push(MeasureTag { code: FILTER_CODE, at: *p });
    }
    if cap.is_some() || island.is_some() || matches!(a.rule, RULE_AROUND | RULE_WITHIN) {
        tags.push(MeasureTag { code: RULES[a.rule].code, at: icon_at });
    }
    let mut codes = Vec::new();
    for (on, code) in
        [(a.approach != 0, APPROACHES[a.approach].code), (a.stop != 0, STOPS[a.stop].code), (a.rule != 0, RULES[a.rule].code), (a.filter, FILTER_CODE)]
    {
        if on {
            codes.push(code);
        }
    }
    let approach_kind = match a.approach {
        Q_OFFSET | Q_CURB => "queue",
        Q_VIRTUAL => "virtual",
        GATE_SIGNAL | GATE_YIELD => "gate",
        _ => "none",
    };
    TransitView {
        bus_lane: a.bus_lane,
        approach: a.approach,
        approaches_out: if s.turbo.is_some() { vec![Q_OFFSET, Q_CURB, Q_VIRTUAL] } else { Vec::new() },
        approach_mm: a.approach_mm,
        stop: a.stop,
        rule: a.rule,
        filter: a.filter,
        kerb_side,
        bus,
        queue,
        virtual_loop,
        gate,
        stop_poly,
        stop_at,
        bollards,
        cap,
        island,
        icon_at,
        problems,
        approach_kind,
        gate_yields: a.approach == GATE_YIELD,
        dead: a.rule == RULE_DEAD_END,
        tags,
        codes,
    }
}

fn l_(p: P) -> Value {
    l(p)
}

fn span_mid(span: (i32, i32)) -> Option<i32> {
    (span != (0, 0)).then_some((span.0 + span.1) / 2)
}

/// A centreline for one movement: a curve tangent to the way the traffic
/// enters and to the way it leaves, or, round a roundabout, along its ring.
#[allow(clippy::too_many_arguments)]
fn movement_path(bear_a: f64, pe: P, bear_b: f64, px: P, ring: Option<f64>, side: Side, ba: i32, bb: i32) -> Vec<Value> {
    if let Some(r) = ring {
        let mid = r - RING_WIDTH_MM / 2.0;
        let (ra, rb) = (at(ba as f64, 0.0, mid), at(bb as f64, 0.0, mid));
        // Traffic circulates anticlockwise where it keeps right.
        let ccw = side == Side::Right;
        let sweep_deg = if ccw { gap(bb, ba) } else { gap(ba, bb) } as f64;
        return vec![m(pe), l(ra), arc(mid, sweep_deg > 180.0, !ccw, rb), l(px)];
    }
    // Entry heading is inward, exit heading is outward; they meet where the
    // turn bends.
    let (di, dx) = (scale(dir(bear_a), -1.0), dir(bear_b));
    let d = cross(di, dx);
    let control = if d.abs() < 1e-6 {
        scale(add(pe, px), 0.5)
    } else {
        let q = sub(px, pe);
        let (u, v) = (cross(q, dx) / d, -cross(di, q) / d);
        if u < 0.0 || v > 0.0 || u > 40_000.0 || v < -40_000.0 { scale(add(pe, px), 0.5) } else { add(pe, scale(di, u)) }
    };
    vec![m(pe), quad(control, px)]
}

/// The lane of an arm that takes traffic to street `to`: of the lanes that serve it, the one nearest the island (the
/// left one for right-hand traffic, the right one for left-hand), as that is the one a driver bound there is in.
fn serving_lane(a: &Arm, to: u32, right_hand: bool) -> Option<usize> {
    let mut serving = a.lanes.iter().enumerate().filter(|(_, l)| l.to.contains(&to)).map(|(k, _)| k);
    if right_hand { serving.next() } else { serving.last() }
}

/// A movement round a turbo's ring: in along the entry street to where it meets ring lane `k`, along the spiral (one
/// lane further out each time it crosses the translation axis) to the street it leaves by, and out along it. A
/// movement that runs out of lanes at a tip leaves the ring there, for the first street after the tip, and goes straight to it (UNVERIFIED, J4).
#[allow(clippy::too_many_arguments)]
fn turbo_path(la: &ArmLayout, lb: &ArmLayout, pe: P, px: P, enter_lat: f64, exit_lat: f64, k: usize, shape: &RingShape) -> Vec<Value> {
    // The lane the entry feeds, or failing that a lane further in: on an egg or a knee a lone entry lane feeds the outer ring
    // lane (`ring_lane`), from which a side street's left turn and through cannot be reached (UNVERIFIED).
    let Some(walk) = (0..=k).rev().find_map(|k| spiral_walk(shape, k, (la.bearing, enter_lat), (lb.bearing, exit_lat))) else {
        return vec![m(pe), l(px)];
    };
    let mut path = vec![m(pe)];
    path.extend(walk.points.into_iter().map(l));
    path.push(l(px));
    path
}

fn conflicts(s: &State, moves: &[MoveView], lay: &Layout, side: Side) -> Conflicts {
    let live: Vec<&MoveView> = moves.iter().filter(|m| m.allowed).collect();
    let by_phase = s.control == SIGNAL;
    if lay.ring.is_some() {
        let enters = s.arms.iter().filter(|a| live.iter().any(|m| m.from == a.uid)).count();
        let leaves = s.arms.iter().filter(|a| live.iter().any(|m| m.to == a.uid)).count();
        let _ = side;
        // A bus lane across the middle crosses the circulating traffic going in and coming out.
        // Cyclists on a track round the outside cross the way in and the way out of every street.
        let crossing = if s.bus.is_some() { 2 } else { 0 } + if s.cycle.is_some() { 2 * s.arms.len() } else { 0 };
        return Conflicts { crossing, merging: enters, diverging: leaves, by_phase };
    }
    let mut diverging = 0;
    let mut merging = 0;
    for a in &s.arms {
        diverging += live.iter().filter(|m| m.from == a.uid).count().saturating_sub(1);
        merging += live.iter().filter(|m| m.to == a.uid).count().saturating_sub(1);
    }
    let polys: Vec<Vec<P>> = live.iter().map(|m| sample_path(&m.path)).collect();
    let mut crossing = 0;
    for i in 0..live.len() {
        for j in i + 1..live.len() {
            if live[i].from != live[j].from && live[i].to != live[j].to && polylines_cross(&polys[i], &polys[j]) {
                crossing += 1;
            }
        }
    }
    Conflicts { crossing, merging, diverging, by_phase }
}

/// A movement's path as points, for crossing tests.
fn sample_path(path: &[Value]) -> Vec<P> {
    let num = |v: &Value, i: usize| v[i].as_f64().unwrap_or(0.0);
    let mut out: Vec<P> = Vec::new();
    for c in path {
        match c[0].as_str() {
            Some("M") | Some("L") => out.push((num(c, 1), num(c, 2))),
            Some("Q") => {
                let from = *out.last().unwrap_or(&(0.0, 0.0));
                out.extend(sample_quad(from, (num(c, 1), num(c, 2)), (num(c, 3), num(c, 4)), 16).into_iter().skip(1));
            }
            _ => {}
        }
    }
    out
}

/// How far past the biggest arrow's centre the steady part of the end zone has to reach, so the stop line and the
/// arrows stand in it (half the longest arrow).
const ARROW_CLEAR_MM: f64 = 3_000.0;

/// Where the end zone's steady part ends, from the centre: where the taper starts, else the boundary.
fn steady_mm(b: &Beyond) -> i32 {
    b.taper.as_ref().map_or(b.boundary_mm, |t| t.from_mm)
}

/// Why an arm's end zone cannot hold its stop line and arrows, or its transition cannot be built. `arrows_mm` is where
/// the arm's arrows stand, with a bike box's depth, from the centre.
fn approach_problems(arm: &Said, b: &Beyond, arrows_mm: f64) -> Vec<Said> {
    let about = |key: &'static str| Said::new(key).with("arm", Arg::Said(Box::new(arm.clone())));
    let mut out = Vec::new();
    let need = (arrows_mm + ARROW_CLEAR_MM).round() as i32;
    if f64::from(steady_mm(b)) < arrows_mm + ARROW_CLEAR_MM {
        out.push(about("jn-approach-short").with("reach", Arg::Length(steady_mm(b))).with("need", Arg::Length(need)));
    }
    if let Some(t) = &b.taper {
        if t.room_mm < t.need_mm {
            out.push(about("jn-approach-room").with("need", Arg::Length(t.need_mm)).with("room", Arg::Length(t.room_mm)));
        }
        if !t.merges {
            out.push(about("jn-approach-drop"));
        }
    }
    out
}

fn checks(s: &State, region: usize, arms: &[ArmView], corners: &[CornerView], moves: &[MoveView], lay: &Layout) -> Vec<Check> {
    let name = |uid: u32| arms.iter().find(|a| a.uid == uid).map_or_else(|| crate::street::model::named_said(""), |a| a.label.clone());
    let arms_of = |uids: Vec<u32>| list(uids.into_iter().map(name).collect());
    let check = |id: &'static str, label: &'static str, ok: Said, bad: Option<Said>| Check {
        id,
        ok: bad.is_none(),
        amount_mm: 0,
        label: Said::new(label),
        detail: bad.unwrap_or(ok),
    };
    let mut out = Vec::new();

    // Every allowed turn has a lane that serves it.
    let no_lane = list(
        moves
            .iter()
            .filter(|m| m.allowed && !m.lane)
            .map(|m| {
                let key = match m.class {
                    LEFT => "jn-no-lane-left",
                    THROUGH => "jn-no-lane-through",
                    _ => "jn-no-lane-right",
                };
                Said::new(key).with("from", Arg::Said(Box::new(name(m.from)))).with("to", Arg::Said(Box::new(name(m.to))))
            })
            .collect(),
    );
    out.push(check(
        "lanes-cover",
        "jn-check-lanes-cover",
        Said::new("jn-check-lanes-cover-ok"),
        no_lane.map(|l| Said::new("jn-check-lanes-cover-bad").with("turns", Arg::Said(Box::new(l)))),
    ));

    // Every lane points at a turn that is allowed.
    let mut dead = Vec::new();
    for a in arms {
        for (i, l) in a.lanes.iter().enumerate() {
            if l.bad {
                dead.push(Said::new("jn-lane-of").with("arm", Arg::Said(Box::new(a.label.clone()))).with("n", Arg::Num(i as i64 + 1)));
            }
        }
    }
    out.push(check(
        "lanes-follow",
        "jn-check-lanes-follow",
        Said::new("jn-check-lanes-follow-ok"),
        list(dead).map(|l| Said::new("jn-check-lanes-follow-bad").with("lanes", Arg::Said(Box::new(l)))),
    ));

    // Crossing distance.
    let longest = arms.iter().filter_map(|a| a.crossing.as_ref().map(|c| (c, a))).max_by_key(|(c, _)| c.stage_mm);
    out.push(match longest {
        None => check("crossing", "jn-check-crossing", Said::new("jn-check-crossing-none"), None),
        Some((c, a)) => {
            let d = c.stage_mm;
            let key = if d <= MAX_STAGE_MM {
                "jn-check-crossing-ok"
            } else if a.can_island && !c.island && !c.continuous && (c.distance_mm - ISLAND_MM) / 2 <= MAX_STAGE_MM {
                // The island is the editor's to suggest, not to place: where it would bring each stage within reach, say so.
                "jn-check-crossing-island"
            } else {
                "jn-check-crossing-far"
            };
            let detail = Said::new(key).with("arm", Arg::Said(Box::new(a.label.clone())));
            Check { id: "crossing", ok: d <= MAX_STAGE_MM, amount_mm: d, label: Said::new("jn-check-crossing"), detail }
        }
    });

    // Turning speed across a marked crossing.
    let fast = arms_of(corners.iter().filter(|c| c.fast).map(|c| c.uid).collect());
    out.push(check(
        "turning-speed",
        "jn-check-turning-speed",
        Said::new("jn-check-turning-speed-ok"),
        fast.map(|l| Said::new("jn-check-turning-speed-bad").with("arms", Arg::Said(Box::new(l)))),
    ));

    // Sidewalk left at the corner.
    let thin = arms_of(corners.iter().filter(|c| !c.ok).map(|c| c.uid).collect());
    out.push(check(
        "corner-room",
        "jn-check-corner-room",
        Said::new("jn-check-corner-room-ok"),
        thin.map(|l| Said::new("jn-check-corner-room-bad").with("arms", Arg::Said(Box::new(l)))),
    ));

    // Transit priority measures have what they need.
    let problems = list(arms.iter().flat_map(|a| a.transit.problems.iter().cloned()).collect());
    out.push(check("transit", "jn-check-transit", Said::new("jn-check-transit-ok"), problems));

    // The street's bus stops stand clear of the junction and do not hold the buses behind a turn.
    let problems = list(arms.iter().flat_map(|a| a.stop_clear.iter().cloned()).collect());
    out.push(check(
        "stop-clear",
        "jn-check-stop-clear",
        Said::new("jn-check-stop-clear-ok"),
        problems.map(|p| Said::new("jn-check-stop-clear-bad").with("stops", Arg::Said(Box::new(p)))),
    ));
    let problems = list(arms.iter().flat_map(|a| a.stop_lane.iter().cloned()).collect());
    out.push(check(
        "stop-lane",
        "jn-check-stop-lane",
        Said::new("jn-check-stop-lane-ok"),
        problems.map(|p| Said::new("jn-check-stop-lane-bad").with("stops", Arg::Said(Box::new(p)))),
    ));

    // The end zone reaches past its stop line and arrows, and its transition fits.
    let problems = list(arms.iter().flat_map(|a| a.beyond_problems.iter().cloned()).collect());
    out.push(check(
        "approach",
        "jn-check-approach",
        Said::new("jn-check-approach-ok"),
        problems.map(|p| Said::new("jn-check-approach-bad").with("problems", Arg::Said(Box::new(p)))),
    ));

    // Signals.
    let signal_ok = s.control != SIGNAL || traffic_arms(s) <= MAX_SIGNAL_ARMS;
    out.push(check(
        "signal",
        "jn-check-signal",
        Said::new("jn-check-signal-ok"),
        (!signal_ok).then(|| Said::new("jn-check-signal-bad").with("n", Arg::Num(traffic_arms(s) as i64))),
    ));
    // Signal timing: time to cross, and the cycle.
    if let Some(plan) = signal::plan(s, region).filter(|p| !p.phases.is_empty()) {
        let short: Vec<u32> = arms
            .iter()
            .filter_map(|a| {
                let needed = (a.crossing.as_ref()?.distance_mm as f64 / 1000.0 / WALK_SPEED_MPS).ceil() as u32;
                (signal::walk_window_s(&plan, a.uid) < needed).then_some(a.uid)
            })
            .collect();
        out.push(check(
            "signal-walk",
            "jn-check-signal-walk",
            Said::new("jn-check-signal-walk-ok"),
            arms_of(short).map(|l| Said::new("jn-check-signal-walk-bad").with("arms", Arg::Said(Box::new(l)))),
        ));
        // Every street with a movement wants a phase that serves it: an edited plan can leave one out.
        let unserved: Vec<u32> = s
            .arms
            .iter()
            .filter(|a| a.lanes.iter().any(|l| l.to.iter().any(|&t| t != a.uid)))
            .filter(|a| !plan.phases.iter().any(|p| p.moves.iter().any(|m| m.from == a.uid)))
            .map(|a| a.uid)
            .collect();
        out.push(check(
            "signal-phases",
            "jn-check-signal-phases",
            Said::new("jn-check-signal-phases-ok"),
            arms_of(unserved).map(|l| Said::new("jn-check-signal-phases-bad").with("arms", Arg::Said(Box::new(l)))),
        ));
        let seconds = i64::from(plan.cycle_s);
        out.push(check(
            "signal-cycle",
            "jn-check-signal-cycle",
            Said::new("jn-check-signal-cycle-ok").with("seconds", Arg::Num(seconds)),
            (plan.cycle_s > MAX_CYCLE_S)
                .then(|| Said::new("jn-check-signal-cycle-bad").with("seconds", Arg::Num(seconds)).with("max", Arg::Num(i64::from(MAX_CYCLE_S)))),
        ));
    }
    // A turbo wants its streets to meet the ring square.
    if let Some((uid, deg)) = s.turbo.as_ref().and_then(|t| t.worst_skew(&s.arms)) {
        let bad = deg >= WARN_SKEW_DEG;
        let detail = if bad {
            Said::new("jn-check-turbo-square-bad").with("arm", Arg::Said(Box::new(name(uid)))).with("degrees", Arg::Num(deg.round() as i64))
        } else {
            Said::new("jn-check-turbo-square-ok")
        };
        out.push(Check { id: "turbo-square", ok: !bad, amount_mm: 0, label: Said::new("jn-check-turbo-square"), detail });
    }
    let _ = lay;
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::junction::model::{DEFAULT_SETBACK_MM, JUNCTION_SAMPLES, MIN_SETBACK_MM, junction_of, junction_two_way};
    use crate::shared::testing::{en, fr};

    /// How many things an `and_list` says.
    fn items(s: &Said) -> usize {
        match (s.key, s.args.as_slice()) {
            ("jn-ph-and" | "jn-list-comma", [(_, Arg::Said(head)), (_, Arg::Said(tail))]) => items(head) + items(tail),
            _ => 1,
        }
    }

    #[test]
    fn a_phases_name_and_summary_name_the_same_streets() {
        let j = Junction::new(0);
        let v = j.view();
        let (a, b, c) = (v.arms[0].uid, v.arms[1].uid, v.arms[2].uid);
        let mut p = signal::plan(j.current(), 0).unwrap().phases[0].clone();
        p.moves = vec![signal::Move { from: a, to: b }, signal::Move { from: c, to: b }, signal::Move { from: a, to: c }];
        p.walks.clear();
        p.rides.clear();
        let find = |uid: u32| v.arms.iter().find(|x| x.uid == uid);
        let name = |uid: u32| find(uid).unwrap().label.clone();
        let named = phase_name(0, &p, &find, &name);
        let Some((_, Arg::Said(streets))) = named.args.iter().find(|(k, _)| *k == "streets") else { panic!("{named:?}") };
        let said = phase_summary(&p, &v.arms);
        let Some((_, Arg::Said(cars))) = said.args.iter().find(|(k, _)| *k == "arms") else { panic!("{said:?}") };
        assert_eq!((items(streets), items(cars)), (2, 2), "the two streets traffic goes from, once each: {named:?} {said:?}");
    }

    #[test]
    fn arm_reaches_match_the_drawn_crossing() {
        for i in 0..JUNCTION_SAMPLES.len() {
            let j = Junction::new(i);
            let reaches = arm_reaches(j.current(), 0);
            let view = j.view();
            assert_eq!(reaches.len(), view.arms.len(), "sample {i}");
            let mut crossings = 0;
            for (arm, drawn) in j.current().arms.iter().zip(&view.arms) {
                let reach = reaches.iter().find(|r| r.0 == arm.uid).unwrap().1;
                if let Some(c) = &drawn.crossing {
                    // the handle stands on the crossing's far edge: its distance along the arm from the centre
                    let d = crate::junction::geometry::dir(drawn.bearing as f64);
                    let far = c.handle.0 * d.0 + c.handle.1 * d.1;
                    assert!((far - reach as f64).abs() <= 1.0, "sample {i}: reach {reach}, far edge {far}");
                    crossings += 1;
                } else {
                    assert!(reach > 0, "sample {i}: without a crossing the reach is the mouth");
                }
            }
            assert!(crossings > 0, "sample {i} has a crossing to compare");
        }
    }

    #[test]
    fn a_junction_with_one_walkway_arm_lays_out_and_draws() {
        use crate::junction::model::junction_with_walkways;
        let j = junction_with_walkways(&[90], &[("sidewalk", 3000), ("terrace", 2000), ("sidewalk", 3000)]);
        assert!(layout(j.current(), 0).is_some(), "the junction can be laid out");
        let v = j.view();
        assert_eq!(v.arms.len(), 4);
        let walk = v.arms.iter().find(|a| a.bearing == 90).unwrap();
        assert!(walk.pedestrian && !walk.enters && !walk.leaves && walk.lanes.is_empty());
        assert_eq!(v.arms.iter().filter(|a| a.pedestrian).count(), 1);
        assert_eq!(v.traffic_arms, 3);
        let i18n = crate::i18n_for(crate::shared::i18n::Locale::En);
        let plan = crate::junction::plan_svg::plan_svg(&v, &i18n, 640.0, 640.0, crate::shared::units::Units::Metres);
        assert!(plan.markup.matches("data-role=\"arm\"").count() >= 4);
        let iso = crate::junction::iso::iso_svg(&v, &i18n, 640.0, 640.0, crate::shared::units::Units::Metres);
        assert!(iso.markup.contains("iso-"));
    }

    const WALKWAY: [(&str, i32); 3] = [("sidewalk", 3000), ("terrace", 2000), ("sidewalk", 3000)];

    #[test]
    fn a_walkway_arm_s_pieces_start_at_its_mouth_with_no_road_before_them_and_no_table_of_its_own() {
        use crate::junction::iso::flatten;
        use crate::junction::model::junction_with_walkways;
        let mut j = junction_with_walkways(&[90], &WALKWAY);
        assert!(j.set_raised(true));
        let v = j.view();
        let walk = v.arms.iter().find(|a| a.pedestrian).unwrap();
        assert!(walk.gaps.is_empty(), "no road lies between the junction and the walkway");
        assert!(walk.raised.is_none(), "a walkway is at sidewalk height already");
        assert!(v.arms.iter().filter(|a| !a.pedestrian).all(|a| a.raised.is_some()), "the other streets are on the table");
        let h = crate::junction::geometry::dir(90.0);
        let near = |poly: &[Value]| flatten(poly).iter().map(|p| p.0 * h.0 + p.1 * h.1).fold(f64::MAX, f64::min);
        let mouth = near(&walk.outline);
        assert_eq!(walk.pieces.len(), 3);
        for p in &walk.pieces {
            assert!((near(&p.poly) - mouth).abs() < 1.0, "a piece starts at the mouth: {} against {mouth}", near(&p.poly));
        }
    }

    #[test]
    fn the_lanes_of_the_other_streets_do_not_offer_a_walkway_as_a_place_to_go() {
        use crate::junction::model::junction_with_walkways;
        let v = junction_with_walkways(&[90], &WALKWAY).view();
        let walk = v.arms.iter().find(|a| a.pedestrian).unwrap().uid;
        for a in v.arms.iter().filter(|a| !a.pedestrian) {
            assert!(!a.lanes.is_empty());
            for lane in &a.lanes {
                assert!(!lane.dests.is_empty() && lane.dests.iter().all(|d| d.uid != walk));
            }
        }
    }

    #[test]
    fn no_turn_is_too_fast_at_a_corner_beside_a_walkway_and_the_same_radius_elsewhere_is() {
        use crate::junction::model::junction_with_walkways;
        let mut j = junction_with_walkways(&[90], &WALKWAY);
        let (north, south) = (j.current().arms.iter().find(|a| a.bearing == 0).unwrap().uid, j.current().arms.iter().find(|a| a.bearing == 180).unwrap().uid);
        assert!(j.set_corner(north, 12_000) && j.set_corner(south, 12_000));
        let v = j.view();
        let fast = |uid: u32| v.corners.iter().find(|c| c.uid == uid).unwrap().fast;
        assert!(!fast(north), "the corner between the north street and the walkway has no turn to flag");
        assert!(fast(south), "the same radius between two streets of traffic is flagged");
    }

    #[test]
    fn a_walkway_arm_adds_nothing_to_the_checks() {
        use crate::junction::model::junction_with_walkways;
        let v = junction_with_walkways(&[90], &WALKWAY).view();
        let walk = v.arms.iter().find(|a| a.pedestrian).unwrap();
        assert!(walk.crossing.is_none() && walk.stop_line.is_none() && walk.lanes.is_empty());
        for id in ["lanes-cover", "lanes-follow", "crossing", "turning-speed"] {
            assert!(v.checks.iter().find(|c| c.id == id).unwrap().ok, "{id}");
        }
    }

    #[test]
    fn a_walkway_wider_than_the_cross_street_does_not_take_the_priority() {
        use crate::junction::model::{PRIORITY, junction_with_walkways};
        let mut j = junction_with_walkways(&[90], &[("sidewalk", 12_000), ("terrace", 8_000), ("sidewalk", 12_000)]);
        assert!(j.set_control(PRIORITY));
        let s = j.current();
        let role = |bearing: i32| control_role(s, s.arms.iter().position(|a| a.bearing == bearing).unwrap());
        assert_eq!((role(0), role(180), role(270)), ("free", "free", "stop"), "the through street keeps the priority");
    }

    #[test]
    fn a_five_arm_signal_with_one_walkway_passes_the_signal_check() {
        use crate::junction::model::SIGNAL;
        use crate::shared::catalogue::{Side, StreetClass, kind_index};
        use crate::street::model::{Piece, Street};
        let pieces: Vec<Piece> =
            WALKWAY.iter().map(|&(id, width_mm)| Piece { kind: kind_index(id).unwrap(), width_mm, direction: None, variants: Vec::new() }).collect();
        let walkway = Street::imported(StreetClass::Pedestrian, Side::Right, &pieces);
        let mut state = Junction::new(3).current().clone(); // the five-way sample
        let street = Street::sample(0, Side::Right);
        for (i, a) in state.arms.iter_mut().enumerate() {
            a.section = Some(if i == 0 { walkway.clone() } else { street.clone() });
            a.edge = a.uid;
        }
        let mut j = Junction::from_city(Said::new("city-name").with("name", Arg::Text("Test".into())), &state, &state, 0).expect("draws");
        assert_eq!(j.current().arms.len(), 5);
        assert!(j.set_control(SIGNAL));
        assert!(j.view().checks.iter().find(|c| c.id == "signal").unwrap().ok, "four streets carry traffic");
    }

    #[test]
    fn the_sheet_says_whether_the_plan_is_suggested_and_flags_a_street_left_without_a_phase() {
        let mut j = Junction::new(0);
        let v = j.view();
        assert!(v.signal.as_ref().unwrap().suggested);
        assert!(v.checks.iter().find(|c| c.id == "signal-phases").unwrap().ok);
        // Four phases, one per street: taking one out leaves that street's lanes with none.
        assert!(j.remove_phase(0));
        let v = j.view();
        assert!(!v.signal.as_ref().unwrap().suggested);
        assert_eq!(v.signal.as_ref().unwrap().phases.len(), 3);
        let check = v.checks.iter().find(|c| c.id == "signal-phases").unwrap();
        assert!(!check.ok, "a street with no phase is flagged");
        assert!(en(&check.detail).contains("No phase serves the traffic entering from"), "{}", en(&check.detail));
        assert!(j.restore_suggestion());
        assert!(j.view().checks.iter().find(|c| c.id == "signal-phases").unwrap().ok);
    }

    #[test]
    fn a_junction_with_one_street_of_traffic_runs_uncontrolled_passes_the_lane_checks_and_draws() {
        use crate::junction::model::{SIGNAL, UNCONTROLLED, junction_with_walkways};
        let j = junction_with_walkways(&[90, 180, 270], &[("sidewalk", 4000), ("sidewalk", 4000)]);
        let v = j.view();
        assert_eq!(v.traffic_arms, 1);
        assert_eq!((v.control, v.control_index), ("uncontrolled", UNCONTROLLED));
        assert_eq!(j.current().control, SIGNAL, "the kept control is not touched");
        let follow = v.checks.iter().find(|c| c.id == "lanes-follow").unwrap();
        assert!(follow.ok, "{:?}", follow.detail);
        assert!(
            v.arms.iter().find(|a| !a.pedestrian).unwrap().lanes.iter().all(|l| !l.bad),
            "a lane with nowhere to go is not a lane that points at a banned turn"
        );
        let i18n = crate::i18n_for(crate::shared::i18n::Locale::En);
        let units = crate::shared::units::Units::Metres;
        assert!(!crate::junction::plan_svg::plan_svg(&v, &i18n, 640.0, 640.0, units).markup.is_empty());
        assert!(!crate::junction::iso::iso_svg(&v, &i18n, 640.0, 640.0, units).markup.is_empty());
    }

    #[test]
    fn a_junction_where_every_street_is_a_walkway_still_draws() {
        use crate::junction::model::junction_with_walkways;
        let v = junction_with_walkways(&[0, 90, 180, 270], &WALKWAY).view();
        assert_eq!((v.traffic_arms, v.arms.iter().filter(|a| a.pedestrian).count()), (0, 4));
        assert_eq!(v.control, "uncontrolled");
        let i18n = crate::i18n_for(crate::shared::i18n::Locale::En);
        let units = crate::shared::units::Units::Metres;
        assert!(!crate::junction::plan_svg::plan_svg(&v, &i18n, 640.0, 640.0, units).markup.is_empty());
        assert!(!crate::junction::iso::iso_svg(&v, &i18n, 640.0, 640.0, units).markup.is_empty());
    }

    #[test]
    fn a_corner_has_a_sidewalk_where_an_arm_has_one_beside_its_curb() {
        let with = junction_of(&[("sidewalk", 2000), ("travel", 3200), ("travel", 3200), ("sidewalk", 2000)]).view();
        assert!(with.corners.iter().all(|c| c.walk), "every arm has a sidewalk");
        let without = junction_of(&[("travel", 3200), ("travel", 3200)]).view();
        assert!(without.corners.iter().all(|c| !c.walk), "no arm has one, and the carriageway reaches the property line");
    }

    #[test]
    fn a_planting_strip_at_the_curb_is_not_a_sidewalk_corner() {
        let v = junction_of(&[("sidewalk", 2000), ("planting", 1500), ("travel", 3200), ("travel", 3200), ("planting", 1500), ("sidewalk", 2000)]).view();
        assert!(v.corners.iter().all(|c| !c.walk));
    }

    #[test]
    fn one_arm_with_a_sidewalk_is_enough_for_its_corners() {
        use crate::shared::catalogue::{StreetClass, kind_index};
        use crate::street::model::{Piece, Street};
        let piece = |id: &str, width_mm| Piece { kind: kind_index(id).unwrap(), width_mm, direction: (id == "travel").then_some(1), variants: Vec::new() };
        let walked = Street::imported(
            StreetClass::Local,
            Side::Right,
            &[piece("sidewalk", 2000), piece("travel", 3200), piece("travel", 3200), piece("sidewalk", 2000)],
        );
        let bare = Street::imported(StreetClass::Local, Side::Right, &[piece("travel", 3200), piece("travel", 3200)]);
        let mut state = Junction::new(0).current().clone();
        for a in &mut state.arms {
            a.edge = a.uid;
            a.section = Some(if a.bearing == 0 { walked.clone() } else { bare.clone() });
        }
        let j = Junction::from_city(Said::new("city-name").with("name", Arg::Text("Test".into())), &state, &state, 0).expect("draws");
        let walks: Vec<bool> = j.view().corners.iter().map(|c| c.walk).collect();
        assert_eq!(walks.iter().filter(|w| **w).count(), 2, "the two corners beside the walked arm: {walks:?}");
    }

    const BIKED: [(&str, i32); 6] = [("sidewalk", 2000), ("bike", 1500), ("travel", 3200), ("travel", 3200), ("bike", 1500), ("sidewalk", 2000)];

    #[test]
    fn an_outer_bike_lane_continues_into_the_core() {
        let v = junction_of(&BIKED).view();
        assert!(v.arms.iter().all(|a| a.core_bike.len() == 2), "one strip for each side");
        let none = junction_of(&[("sidewalk", 2000), ("travel", 3200), ("travel", 3200), ("sidewalk", 2000)]).view();
        assert!(none.arms.iter().all(|a| a.core_bike.is_empty()));
    }

    #[test]
    fn a_bike_lane_between_travel_lanes_is_not_continued() {
        let v = junction_of(&[("sidewalk", 2000), ("travel", 3200), ("bike", 1500), ("travel", 3200), ("sidewalk", 2000)]).view();
        assert!(v.arms.iter().all(|a| a.core_bike.is_empty()));
    }

    #[test]
    fn a_road_of_one_bike_lane_has_one_strip() {
        let v = junction_of(&[("sidewalk", 2000), ("bike", 2500), ("sidewalk", 2000)]).view();
        assert!(v.arms.iter().all(|a| a.core_bike.len() == 1), "the first and the last roadway piece are the same piece");
    }

    #[test]
    fn a_bike_lane_outside_parking_is_continued() {
        let v = junction_of(&[
            ("sidewalk", 2000),
            ("parking", 2000),
            ("bike", 1500),
            ("travel", 3200),
            ("travel", 3200),
            ("bike", 1500),
            ("parking", 2000),
            ("sidewalk", 2000),
        ])
        .view();
        assert!(v.arms.iter().all(|a| a.core_bike.len() == 2), "the parking lanes give way to the road at the mouth, the bike lanes arrive");
    }

    #[test]
    fn a_roundabout_continues_no_bike_lane() {
        let mut j = junction_of(&BIKED);
        assert!(j.view().arms.iter().all(|a| !a.core_bike.is_empty()));
        j.set_control(ROUNDABOUT);
        let v = j.view();
        assert!(v.ring.is_some());
        assert!(v.arms.iter().all(|a| a.core_bike.is_empty()));
    }

    #[test]
    fn a_raised_junction_has_a_top_and_a_ramp_on_every_arm() {
        let mut j = Junction::new(0);
        assert!(j.view().arms.iter().all(|a| a.raised.is_none()) && !j.view().raised);
        assert!(j.set_raised(true));
        let v = j.view();
        assert!(v.raised);
        assert!(v.arms.iter().all(|a| a.raised.as_ref().is_some_and(|r| r.top.len() > 2 && r.ramp.len() > 2)));
    }

    #[test]
    fn a_raised_junction_without_crossings_ramps_from_the_mouth() {
        let mut j = Junction::new(0);
        for a in j.current().arms.iter().map(|a| a.uid).collect::<Vec<_>>() {
            assert!(j.set_crossing(a, false));
        }
        assert!(j.set_raised(true));
        assert!(j.view().arms.iter().all(|a| a.raised.is_some()));
    }

    #[test]
    fn a_roundabout_draws_no_table_even_if_its_save_says_raised() {
        let mut state = Junction::new(0).current().clone();
        state.control = ROUNDABOUT;
        state.raised = true;
        for a in &mut state.arms {
            a.edge = a.uid;
        }
        let j = Junction::from_city(Said::new("city-name").with("name", Arg::Text("Test".into())), &state, &state, 0).expect("draws");
        assert!(!j.view().raised);
        assert!(j.view().arms.iter().all(|a| a.raised.is_none()));
        assert!(j.view().corners.iter().all(|c| !c.fast));
    }

    #[test]
    fn a_raised_table_allows_a_faster_turn_beside_a_crossing_but_not_a_much_faster_one() {
        let fast = |radius: i32, raised: bool| {
            let mut j = Junction::new(0);
            let n = arm_of(&j, 0);
            assert!(j.set_corner(n, radius));
            if raised {
                assert!(j.set_raised(true));
            }
            j.view().corners.iter().find(|c| c.uid == n).unwrap().fast
        };
        assert!(fast(12_000, false), "21 km/h is over the 20 km/h limit");
        assert!(!fast(12_000, true), "and under the 23 km/h one of a table");
        assert!(fast(15_000, true), "24 km/h is over it");
    }

    /// How far along an arm at `bearing` (its lateral offset 0) a polygon reaches, from the centre of the junction.
    fn along(poly: &[Value], bearing: f64) -> (f64, f64) {
        let d = crate::junction::geometry::dir(bearing);
        let ts: Vec<f64> = poly.iter().filter_map(|v| Some(v.get(1)?.as_f64()? * d.0 + v.get(2)?.as_f64()? * d.1)).collect();
        (ts.iter().cloned().fold(f64::MAX, f64::min), ts.iter().cloned().fold(f64::MIN, f64::max))
    }

    #[test]
    fn a_continuous_crossing_has_a_ramp_on_each_side_and_a_plain_one_has_none() {
        let mut j = Junction::new(0);
        let n = arm_of(&j, 0);
        // (continuous, setback, width, ramps)
        let cx = |j: &Junction| {
            let v = j.view();
            let c = v.arms.iter().find(|a| a.uid == n).unwrap().crossing.as_ref().unwrap();
            (c.continuous, c.setback_mm, c.width_mm, c.ramps.clone())
        };
        let plain = cx(&j);
        assert!(plain.3.is_empty() && !plain.0);
        assert!(j.set_continuous(n, true));
        let (continuous, setback_mm, width_mm, ramps) = cx(&j);
        assert!(continuous);
        assert_eq!(ramps.len(), 2);
        let mouth = layout(j.current(), 0).unwrap().arms[0].mouth;
        let (t0, t1) = (mouth + setback_mm as f64, mouth + (setback_mm + width_mm) as f64);
        let (inner, outer) = (along(&ramps[0], 0.0), along(&ramps[1], 0.0));
        assert!((inner.0 - (t0 - 2_400.0)).abs() < 2.0 && (inner.1 - t0).abs() < 2.0, "{inner:?} vs {t0}");
        assert!((outer.0 - t1).abs() < 2.0 && (outer.1 - (t1 + 2_400.0)).abs() < 2.0, "{outer:?} vs {t1}");
    }

    #[test]
    fn a_continuous_crossing_on_a_raised_table_draws_no_ramps_of_its_own() {
        let mut j = Junction::new(0);
        let n = arm_of(&j, 0);
        assert!(j.set_continuous(n, true));
        assert!(j.set_raised(true));
        let v = j.view();
        let c = v.arms.iter().find(|a| a.uid == n).unwrap().crossing.as_ref().unwrap();
        assert!(c.continuous && c.ramps.is_empty(), "the table's own ramps are the steps");
    }

    #[test]
    fn a_continuous_crossing_is_not_advised_an_island_it_cannot_have() {
        let mut street = vec![("sidewalk", 2000)];
        street.extend((0..5).map(|_| ("travel", 3500)));
        street.push(("sidewalk", 2000));
        let mut j = junction_of(&street);
        let detail = |j: &Junction| en(&j.view().checks.iter().find(|c| c.id == "crossing").unwrap().detail);
        assert!(detail(&j).contains("refuge island"), "an island would help a 17.5 m crossing: {}", detail(&j));
        for uid in j.current().arms.iter().map(|a| a.uid).collect::<Vec<_>>() {
            assert!(j.set_continuous(uid, true));
        }
        let said = detail(&j);
        assert!(said.contains("too far to cross in one go") && !said.contains("refuge island"), "{said}");
    }

    #[test]
    fn the_inner_ramp_stops_at_the_mouth_when_the_crossing_is_set_back_least() {
        let mut j = Junction::new(0);
        let n = arm_of(&j, 0);
        assert!(j.set_setback(n, MIN_SETBACK_MM));
        assert!(j.set_continuous(n, true));
        let v = j.view();
        let c = v.arms.iter().find(|a| a.uid == n).unwrap().crossing.as_ref().unwrap();
        let mouth = layout(j.current(), 0).unwrap().arms[0].mouth;
        let inner = along(&c.ramps[0], 0.0);
        assert!((inner.0 - mouth).abs() < 2.0, "{inner:?} from {mouth}");
    }

    #[test]
    fn a_corner_beside_only_continuous_crossings_may_turn_at_the_table_speed() {
        // a 12 m corner turns at 21 km/h: over 20, under 23
        let fast = |continuous: &[bool; 2], crossing: &[bool; 2]| {
            let mut j = Junction::new(0);
            let n = arm_of(&j, 0);
            assert!(j.set_corner(n, 12_000));
            let next = j.view().corners.iter().find(|c| c.uid == n).unwrap().next_uid;
            for (i, uid) in [n, next].into_iter().enumerate() {
                if !crossing[i] {
                    assert!(j.set_crossing(uid, false));
                } else if continuous[i] {
                    assert!(j.set_continuous(uid, true));
                }
            }
            j.view().corners.iter().find(|c| c.uid == n).unwrap().fast
        };
        assert!(fast(&[false, false], &[true, true]), "zebras on both: 20 km/h");
        assert!(fast(&[true, false], &[true, true]), "one continuous, one zebra: still 20");
        assert!(!fast(&[true, true], &[true, true]), "both continuous");
        assert!(!fast(&[true, false], &[true, false]), "one continuous, the other street has no crossing");
    }

    #[test]
    fn a_bike_box_spans_the_entering_lanes_ahead_of_the_stop_line_and_moves_the_stop_line_back() {
        let mut j = junction_of(&BIKED);
        let n = arm_of(&j, 0);
        let arm = |j: &Junction| {
            let v = j.view();
            let a = v.arms.iter().find(|a| a.uid == n).unwrap();
            (a.bike_box.clone(), a.stop_line, a.can_bike_box, a.crossing.as_ref().map(|c| (c.setback_mm, c.width_mm)))
        };
        let (none, stop0, can, cx) = arm(&j);
        assert!(none.is_none() && can);
        assert!(j.set_bike_box(n, true));
        let (boxed, stop1, ..) = arm(&j);
        let mouth = layout(j.current(), 0).unwrap().arms[0].mouth;
        let (setback, width) = cx.unwrap();
        let far = mouth + (setback + width) as f64;
        let (t0, t1) = along(&boxed.unwrap(), 0.0);
        assert!((t0 - (far + 600.0)).abs() < 2.0 && (t1 - (far + 3_600.0)).abs() < 2.0, "{t0}..{t1} from {far}");
        let d = crate::junction::geometry::dir(0.0);
        let t = |s: Option<[P; 2]>| {
            let p = s.unwrap()[0];
            p.0 * d.0 + p.1 * d.1
        };
        assert!((t(stop1) - t(stop0) - 3_000.0).abs() < 2.0, "the stop line moves back by the box's depth");
    }

    #[test]
    fn a_street_with_no_crossing_has_its_box_from_the_mouth() {
        let mut j = junction_of(&BIKED);
        let n = arm_of(&j, 0);
        assert!(j.set_crossing(n, false));
        assert!(j.set_bike_box(n, true));
        let v = j.view();
        let boxed = v.arms.iter().find(|a| a.uid == n).unwrap().bike_box.clone().unwrap();
        let mouth = layout(j.current(), 0).unwrap().arms[0].mouth;
        assert!((along(&boxed, 0.0).0 - (mouth + 600.0)).abs() < 2.0);
    }

    #[test]
    fn a_street_that_cannot_have_a_bike_box_says_why() {
        let why = |j: &Junction| j.view().arms.iter().map(|a| a.bike_box_why).collect::<Vec<_>>();
        assert!(why(&junction_of(&BIKED)).iter().all(|w| w.is_none()));
        let plain = junction_of(&[("sidewalk", 2000), ("travel", 3200), ("travel", 3200), ("sidewalk", 2000)]);
        assert!(why(&plain).iter().all(|w| *w == Some("jn-insp-bike-box-no-lane")));
        let mut ring = junction_of(&BIKED);
        assert!(ring.set_control(ROUNDABOUT));
        assert!(why(&ring).iter().all(|w| *w == Some("jn-insp-bike-box-roundabout")));
        let mut priority = junction_of(&BIKED);
        assert!(priority.set_control(PRIORITY));
        let reasons = why(&priority);
        assert!(reasons.contains(&Some("jn-insp-bike-box-free")) && reasons.contains(&None), "{reasons:?}");
    }

    /// The points a path's commands end at (M and L carry x y, A carries its end last).
    fn ends(path: &[Value]) -> Vec<P> {
        path.iter()
            .filter_map(|v| {
                let a = v.as_array()?;
                match a.first()?.as_str()? {
                    "M" | "L" => Some((a[1].as_f64()?, a[2].as_f64()?)),
                    "A" => Some((a[4].as_f64()?, a[5].as_f64()?)),
                    _ => None,
                }
            })
            .collect()
    }

    #[test]
    fn a_protected_corner_has_a_bike_track_and_a_corner_island_round_its_curb() {
        let mut j = junction_of(&BIKED);
        let n = arm_of(&j, 0);
        let corner = |j: &Junction| {
            let v = j.view();
            let c = v.corners.iter().find(|c| c.uid == n).unwrap();
            (c.protected.as_ref().map(|p| (p.track.clone(), p.island.clone())), c.protected_why)
        };
        assert!(corner(&j).0.is_none());
        assert!(j.set_protected(n, true));
        let (pv, why) = corner(&j);
        let (track, island) = pv.unwrap();
        assert!(why.is_none());
        let lay = layout(j.current(), 0).unwrap();
        let f = lay.corners[j.current().arms.iter().position(|a| a.uid == n).unwrap()].fillet.as_ref().unwrap();
        let (w, i) = (1_500.0, 2_000.0);
        let d = |p: &P| ((p.0 - f.o.0).powi(2) + (p.1 - f.o.1).powi(2)).sqrt();
        for p in ends(&track) {
            assert!(d(&p) >= f.r - 2.0 && d(&p) <= f.r + w + 2.0, "track point {p:?} at {} (r {})", d(&p), f.r);
        }
        for p in ends(&island) {
            assert!(d(&p) >= f.r + w - 2.0 && d(&p) <= f.r + w + i + 2.0, "island point {p:?} at {}", d(&p));
        }
        assert!(ends(&track).iter().any(|p| (d(p) - (f.r + w)).abs() < 2.0), "the track reaches the island's side");
    }

    #[test]
    fn the_island_leaves_the_street_a_lane_and_the_arcs_turn_the_way_the_curb_does() {
        let mut j = junction_of(&BIKED);
        for uid in j.current().arms.iter().map(|a| a.uid).collect::<Vec<_>>() {
            assert!(j.set_protected(uid, true));
        }
        let v = j.view();
        let lay = layout(j.current(), 0).unwrap();
        for (i, c) in v.corners.iter().enumerate() {
            let p = c.protected.as_ref().expect("protected");
            let f = lay.corners[i].fillet.as_ref().unwrap();
            let d = |q: &P| ((q.0 - f.o.0).powi(2) + (q.1 - f.o.1).powi(2)).sqrt();
            // 9.4 m of road, 1.5 m lanes at the curbs, 3 m kept clear: the island is at most (9.4 - 3) / 2 - 1.5 = 1.7 m deep
            let reach = ends(&p.island).iter().map(d).fold(0.0, f64::max) - f.r - 1_500.0;
            assert!((reach - 1_700.0).abs() < 2.0, "corner {i}: the island is {reach} deep");
            // the arcs sweep the way the curb's arc does, so the crescents lie on the carriageway side of it
            let sweep = |path: &[Value]| path.iter().find(|v| v[0] == "A").map(|v| v[3].clone());
            assert_eq!(sweep(&p.track), sweep(&c.curb), "corner {i}");
            assert_eq!(sweep(&p.island), sweep(&c.curb), "corner {i}");
        }
    }

    #[test]
    fn the_track_follows_the_curb_on_a_five_way_s_obtuse_and_acute_corners_too() {
        use crate::shared::catalogue::{Side, StreetClass, kind_index};
        use crate::street::model::{Piece, Street};
        let pieces: Vec<Piece> = BIKED
            .iter()
            .map(|&(id, w)| Piece { kind: kind_index(id).unwrap(), width_mm: w, direction: (id == "travel").then_some(1), variants: Vec::new() })
            .collect();
        let street = Street::imported(StreetClass::Local, Side::Right, &pieces);
        let mut state = Junction::new(3).current().clone(); // the five-way sample
        for a in &mut state.arms {
            a.section = Some(street.clone());
            a.edge = a.uid;
        }
        let mut j = Junction::from_city(Said::new("city-name").with("name", Arg::Text("Test".into())), &state, &state, 0).expect("draws");
        let lay = layout(j.current(), 0).unwrap();
        for (i, uid) in j.current().arms.iter().map(|a| a.uid).collect::<Vec<_>>().into_iter().enumerate() {
            assert!(j.set_protected(uid, true), "corner {i}");
            let v = j.view();
            let p = v.corners[i].protected.as_ref().expect("protected");
            let f = lay.corners[i].fillet.as_ref().unwrap();
            let d = |q: &P| ((q.0 - f.o.0).powi(2) + (q.1 - f.o.1).powi(2)).sqrt();
            assert!(ends(&p.track).iter().all(|q| d(q) >= f.r - 2.0 && d(q) <= f.r + 1_500.0 + 2.0), "corner {i}");
            assert!(ends(&p.island).iter().all(|q| d(q) >= f.r + 1_500.0 - 2.0 && d(q) <= f.r + 3_500.0 + 2.0), "corner {i}");
        }
    }

    #[test]
    fn a_corner_that_cannot_be_protected_says_why() {
        let why = |j: &Junction| j.view().corners.iter().map(|c| c.protected_why).collect::<Vec<_>>();
        assert!(why(&junction_of(&BIKED)).iter().all(|w| w.is_none()));
        let plain = junction_of(&[("sidewalk", 2000), ("travel", 3200), ("travel", 3200), ("sidewalk", 2000)]);
        assert!(why(&plain).iter().all(|w| *w == Some("jn-insp-protected-no-lane")));
        let mut ring = junction_of(&BIKED);
        assert!(ring.set_control(ROUNDABOUT));
        assert!(why(&ring).iter().all(|w| *w == Some("jn-insp-protected-roundabout")));
        let mut state = junction_of(&BIKED).current().clone();
        state.arms.retain(|a| a.bearing != 270);
        let t = Junction::from_city(Said::new("city-name").with("name", Arg::Text("Test".into())), &state, &state, 0).expect("draws");
        let reasons = why(&t);
        assert!(reasons.contains(&Some("jn-insp-protected-straight")) && reasons.contains(&None), "{reasons:?}");
    }

    #[test]
    fn a_protected_corner_replaces_the_two_core_bike_stubs_on_its_sides() {
        let mut j = junction_of(&BIKED);
        let stubs = |j: &Junction| j.view().arms.iter().map(|a| a.core_bike.len()).sum::<usize>();
        assert_eq!(stubs(&j), 8);
        let n = arm_of(&j, 0);
        assert!(j.set_protected(n, true));
        assert_eq!(stubs(&j), 6, "arm 0's right and the next arm's left");
        let v = j.view();
        let next = v.corners.iter().find(|c| c.uid == n).unwrap().next_uid;
        assert_eq!(v.arms.iter().find(|a| a.uid == n).unwrap().core_bike.len(), 1);
        assert_eq!(v.arms.iter().find(|a| a.uid == next).unwrap().core_bike.len(), 1);
    }

    #[test]
    fn the_bike_lane_reaches_the_track_where_the_two_corners_of_a_street_meet_it_at_different_distances() {
        let mut j = junction_of(&BIKED);
        let n = arm_of(&j, 0);
        let before = arm_of(&j, 270);
        // the corner before arm 0 is wide and sets its mouth back; arm 0's own corner is tight
        assert!(j.set_corner(before, 9_000));
        assert!(j.set_corner(n, 3_000));
        assert!(j.set_protected(n, true));
        let lay = layout(j.current(), 0).unwrap();
        let mouth = lay.arms[0].mouth;
        let v = j.view();
        let arm = v.arms.iter().find(|a| a.uid == n).unwrap();
        assert_eq!(arm.core_bike.len(), 2, "the left stub, and a lead-in where the right one was");
        let f = lay.corners[0].fillet.as_ref().unwrap();
        let d = crate::junction::geometry::dir(0.0);
        let tangent = f.a.0 * d.0 + f.a.1 * d.1;
        assert!(mouth - tangent > 500.0, "the corners differ: tangent {tangent}, mouth {mouth}");
        let lead = arm.core_bike.iter().map(|s| along(s, 0.0)).find(|(lo, _)| (lo - tangent).abs() < 2.0).expect("a lead-in from the tangent point");
        assert!((lead.1 - mouth).abs() < 2.0, "to the arm's lane at the mouth: {lead:?}");
    }

    #[test]
    fn a_roundabout_loaded_with_a_protected_corner_draws_none() {
        let mut state = junction_of(&BIKED).current().clone();
        state.control = ROUNDABOUT;
        for a in &mut state.arms {
            a.protected = true;
            a.edge = a.uid;
        }
        let j = Junction::from_city(Said::new("city-name").with("name", Arg::Text("Test".into())), &state, &state, 0).expect("draws");
        assert!(j.view().corners.iter().all(|c| c.protected.is_none()));
    }

    #[test]
    fn a_free_approach_is_offered_no_bike_box() {
        let mut j = junction_of(&BIKED);
        assert!(j.set_control(PRIORITY));
        let v = j.view();
        for a in &v.arms {
            assert_eq!(a.can_bike_box, a.role != "free", "{} is {}", a.bearing, a.role);
        }
        assert!(v.arms.iter().any(|a| a.role == "free") && v.arms.iter().any(|a| a.role != "free"));
    }

    #[test]
    fn a_street_with_no_crossing_still_has_a_stop_line_behind_its_box() {
        let mut j = junction_of(&BIKED);
        let n = arm_of(&j, 0);
        assert!(j.set_crossing(n, false));
        let stop = |j: &Junction| j.view().arms.iter().find(|a| a.uid == n).unwrap().stop_line;
        assert!(stop(&j).is_none());
        assert!(j.set_bike_box(n, true));
        assert!(stop(&j).is_some());
    }

    #[test]
    fn the_box_is_as_wide_as_the_stop_line_and_the_arrows_stay_clear_of_it() {
        let mut j = junction_of(&BIKED);
        let n = arm_of(&j, 0);
        let lane = |j: &Junction| {
            let v = j.view();
            let a = v.arms.iter().find(|a| a.uid == n).unwrap();
            (a.bike_box.clone(), a.stop_line, a.lanes[0].at)
        };
        let (_, _, arrow0) = lane(&j);
        assert!(j.set_bike_box(n, true));
        let (boxed, stop, arrow1) = lane(&j);
        let xs: Vec<f64> = boxed.unwrap().iter().filter_map(|v| v.get(1)?.as_f64()).collect();
        let (lo, hi) = (xs.iter().cloned().fold(f64::MAX, f64::min), xs.iter().cloned().fold(f64::MIN, f64::max));
        let stop = stop.unwrap();
        let (a, b) = (stop[0].0.min(stop[1].0), stop[0].0.max(stop[1].0));
        assert!((lo - a).abs() < 2.0 && (hi - b).abs() < 2.0, "box {lo}..{hi}, stop line {a}..{b}");
        let d = crate::junction::geometry::dir(0.0);
        let moved = (arrow1.0 - arrow0.0) * d.0 + (arrow1.1 - arrow0.1) * d.1;
        assert!((moved - 3_000.0).abs() < 2.0, "the entering arrows move back with the stop line: {moved}");
    }

    #[test]
    fn no_bike_box_is_offered_without_a_lane_or_on_a_roundabout_and_a_loaded_roundabout_draws_none() {
        let plain = junction_of(&[("sidewalk", 2000), ("travel", 3200), ("travel", 3200), ("sidewalk", 2000)]).view();
        assert!(plain.arms.iter().all(|a| !a.can_bike_box && a.bike_box.is_none()));
        let mut ring = junction_of(&BIKED);
        assert!(ring.set_control(ROUNDABOUT));
        assert!(ring.view().arms.iter().all(|a| !a.can_bike_box && a.bike_box.is_none()));
        let mut state = junction_of(&BIKED).current().clone();
        state.control = ROUNDABOUT;
        for a in &mut state.arms {
            a.bike_box = true;
            a.edge = a.uid;
        }
        let loaded = Junction::from_city(Said::new("city-name").with("name", Arg::Text("Test".into())), &state, &state, 0).expect("draws");
        assert!(loaded.view().arms.iter().all(|a| a.bike_box.is_none()));
    }

    fn arm_of(j: &Junction, bearing: i32) -> u32 {
        j.current().arms.iter().find(|a| a.bearing == bearing).unwrap().uid
    }

    #[test]
    fn the_four_way_has_the_textbook_thirty_two_conflicts() {
        let j = Junction::new(0);
        let v = j.view();
        assert_eq!(v.conflicts.crossing, 16, "{:?}", v.conflicts.crossing);
        assert_eq!(v.conflicts.merging, 8);
        assert_eq!(v.conflicts.diverging, 8);
        assert!(v.conflicts.by_phase);
    }

    #[test]
    fn a_crossing_that_is_too_far_suggests_a_refuge_island_where_one_would_fix_it() {
        let mut j = Junction::new(0);
        let n = j.current().arms.iter().find(|a| a.bearing == 0).unwrap().uid;
        assert!(j.set_island(n, false));
        let v = j.view();
        let check = v.checks.iter().find(|c| c.id == "crossing").unwrap();
        assert!(!check.ok);
        assert_eq!(en(&check.detail), "Sample Avenue 2 (north) is too far to cross in one go. A refuge island in the middle would split it in two");
        assert_eq!(
            fr(&check.detail),
            "Sample Avenue 2 (nord)\u{a0}: trop long à traverser d’un seul coup. Un îlot refuge au centre couperait la traversée en deux"
        );
        assert_eq!((en(&check.label), fr(&check.label)), ("Crossing distance".into(), "Distance de traversée".into()));
        // With the island on, there is nothing to suggest.
        assert!(j.set_island(n, true));
        let v = j.view();
        let check = v.checks.iter().find(|c| c.id == "crossing").unwrap();
        assert!(check.ok);
        assert_eq!(en(&check.detail), "Longest crossing in one go");
    }

    #[test]
    fn every_sample_starts_sound() {
        for i in 0..JUNCTION_SAMPLES.len() {
            let v = Junction::new(i).view();
            let failing: Vec<_> = v.checks.iter().filter(|c| !c.ok).map(|c| format!("{}: {}", c.id, en(&c.detail))).collect();
            assert!(failing.is_empty(), "{}: {failing:?}", JUNCTION_SAMPLES[i].name);
        }
    }

    #[test]
    fn a_bus_lane_across_the_middle_is_drawn_between_its_streets_and_crosses_the_ring_twice() {
        let mut j = Junction::new(0);
        j.set_control(ROUNDABOUT);
        let opts = j.view().bus_options;
        assert_eq!(opts.len(), 6);
        let first = en(&opts[0].label);
        assert_eq!(first.contains("north") && first.contains("south") || first.contains("east") && first.contains("west"), true, "straightest first");
        assert!(j.set_bus(Some((opts[0].a, opts[0].b))));
        let v = j.view();
        let bus = v.bus.unwrap();
        assert_eq!(bus.poly.len(), 5);
        assert_eq!(v.conflicts.crossing, 2);
        assert!(j.set_bus(None));
        assert!(j.view().bus.is_none());
    }

    #[test]
    fn a_cycle_track_shrinks_the_carriageway_not_the_footprint_and_adds_crossings() {
        let mut j = Junction::new(0);
        j.set_control(ROUNDABOUT);
        let plain = j.view().ring.unwrap();
        assert!(j.set_cycle(Some(2000)));
        let v = j.view();
        let r = v.ring.unwrap();
        assert_eq!(r.radius_mm, plain.radius_mm);
        assert_eq!(r.road_mm, plain.radius_mm - 2000);
        assert_eq!(r.cycle_mm, Some(2000));
        assert!(r.island_mm < plain.island_mm);
        assert_eq!(v.conflicts.crossing, 8, "two for each of four streets");
    }

    #[test]
    fn transit_measures_flag_what_they_lack_and_stop_turns() {
        let mut j = Junction::new(0);
        let (n, e) = (arm_of(&j, 0), arm_of(&j, 90));
        let transit = |j: &Junction| j.view().checks.into_iter().find(|c| c.id == "transit").unwrap();
        assert!(transit(&j).ok);
        // A gate with no bus lane has nothing to let through.
        assert!(j.set_approach(n, GATE_SIGNAL));
        assert!(!transit(&j).ok);
        assert!(j.set_bus_lane(n, true));
        assert!(transit(&j).ok, "{}", en(&transit(&j).detail));
        // A virtual queue jump needs a signal; take the signal away.
        assert!(j.set_approach(e, Q_VIRTUAL));
        assert!(transit(&j).ok);
        assert!(j.set_control(ALL_WAY_STOP));
        assert!(!transit(&j).ok);
        // A bulb extends into parking; a lane street has none beside its curb? it does here.
        assert!(j.set_approach(e, 0) && j.set_control(SIGNAL));
        assert!(j.set_stop(n, STOP_BULB));
        assert!(transit(&j).ok);
        assert!(j.set_stop(n, STOP_PLATFORM));
        assert!(transit(&j).ok, "signal and crossing are there");
        assert!(j.set_crossing(n, false));
        assert!(!transit(&j).ok);
    }

    #[test]
    fn turn_management_blocks_movements_and_the_lanes_that_only_go_there() {
        let mut j = Junction::new(0);
        let (n, e, s, w) = (arm_of(&j, 0), arm_of(&j, 90), arm_of(&j, 180), arm_of(&j, 270));
        let moves = |j: &Junction| j.view().movements;
        let allowed = |j: &Junction, a: u32, b: u32| moves(j).iter().find(|m| m.from == a && m.to == b).unwrap().allowed;
        // L3: the east street is right-in/right-out: only right turns leave it, only right turns enter.
        assert!(j.set_rule(e, RULE_RIRO));
        assert!(allowed(&j, e, n) == false || allowed(&j, e, s) == false, "one of its turns is a left, so it is blocked");
        let left_from_e = if turn_class(90, 0) == LEFT { n } else { s };
        assert!(!allowed(&j, e, left_from_e));
        assert!(!allowed(&j, e, w), "through is blocked");
        assert!(moves(&j).iter().filter(|m| m.from == e && m.allowed).all(|m| m.class == RIGHT));
        assert!(moves(&j).iter().filter(|m| m.to == e && m.allowed).all(|m| m.class == RIGHT));
        // L4: a dead end has no movements either way.
        assert!(j.set_rule(e, RULE_DEAD_END));
        assert!(moves(&j).iter().filter(|m| m.from == e || m.to == e).all(|m| !m.allowed));
        assert!(moves(&j).iter().filter(|m| m.blocked.is_some()).count() >= 6);
        // N1: a filter does the same for general traffic.
        assert!(j.set_rule(e, 0) && j.set_filter(w, true));
        assert!(moves(&j).iter().filter(|m| m.from == w || m.to == w).all(|m| !m.allowed));
        assert!(j.set_filter(w, false));
        // L1: left turns from the south street are sent round the block.
        assert!(j.set_rule(s, RULE_AROUND));
        assert!(moves(&j).iter().filter(|m| m.from == s && m.class == LEFT).all(|m| !m.allowed));
        assert!(moves(&j).iter().filter(|m| m.from == s && m.class != LEFT).all(|m| m.allowed));
        // The lane checks still hold together with the measures.
        let v = j.view();
        assert!(v.checks.iter().find(|c| c.id == "lanes-cover").unwrap().ok);
    }

    #[test]
    fn l2_leaves_the_left_turn_allowed_but_routes_it_through_the_middle() {
        let mut j = Junction::new(0);
        let n = arm_of(&j, 0);
        assert!(j.set_rule(n, RULE_WITHIN));
        let v = j.view();
        let left = v.movements.iter().find(|m| m.from == n && m.class == LEFT).unwrap();
        assert!(left.allowed && left.indirect);
        assert_eq!(left.path.len(), 3);
        assert_eq!(v.movements.iter().filter(|m| m.indirect).count(), 1);
    }

    #[test]
    fn the_measures_are_drawn_where_they_belong() {
        let mut j = Junction::new(0);
        let e = arm_of(&j, 90);
        j.set_bus_lane(e, true);
        j.set_approach(e, Q_CURB);
        j.set_stop(e, STOP_BULB);
        j.set_filter(e, true);
        j.set_rule(e, RULE_DEAD_END);
        let v = j.view();
        let t = &v.arms.iter().find(|a| a.uid == e).unwrap().transit;
        assert!(t.bus.is_some() && t.queue.is_some() && t.stop_poly.is_some() && t.cap.is_some());
        assert!(!t.bollards.is_empty());
        let n = v.arms.iter().find(|a| a.bearing == 0).unwrap();
        assert!(n.transit.bus.is_none() && n.transit.cap.is_none());
    }

    #[test]
    fn a_roundabout_has_no_crossing_conflicts() {
        let mut j = Junction::new(0);
        assert!(j.set_control(ROUNDABOUT));
        let v = j.view();
        assert_eq!(v.conflicts.crossing, 0);
        assert_eq!(v.conflicts.merging, 4);
        assert!(v.ring.is_some());
    }

    #[test]
    fn a_t_junction_has_nine_conflicts() {
        // The textbook count for a T: 3 crossing, 3 merging, 3 diverging.
        let j = Junction::new(1);
        let v = j.view();
        assert_eq!(v.conflicts.crossing, 3);
        assert_eq!(v.conflicts.merging, 3);
        assert_eq!(v.conflicts.diverging, 3);
    }

    #[test]
    fn corner_fillets_are_tangent_and_touch_neither_arm_end() {
        let j = Junction::new(0);
        let lay = layout(j.current(), 0).unwrap();
        for c in &lay.corners {
            let f = c.fillet.as_ref().unwrap();
            assert!((dist(f.a, f.o) - f.r).abs() < 1e-6 && (dist(f.b, f.o) - f.r).abs() < 1e-6);
            assert!(lay.arms[c.a].mouth >= 0.0 && lay.arms[c.a].mouth < ARM_LENGTH_MM as f64);
        }
    }

    #[test]
    fn a_bigger_corner_sets_the_mouth_back_and_speeds_the_turn() {
        let mut j = Junction::new(0);
        let n = arm_of(&j, 0);
        let before = layout(j.current(), 0).unwrap().arms[0].mouth;
        assert!(j.set_corner(n, 12_000));
        let after = layout(j.current(), 0).unwrap().arms[0].mouth;
        assert!(after > before);
        let v = j.view();
        let c = v.corners.iter().find(|c| c.uid == n).unwrap();
        assert!(c.speed_kmh > 20.0);
        // A crossing sits beside it, so the check now fails.
        assert!(!v.checks.iter().find(|c| c.id == "turning-speed").unwrap().ok);
    }

    #[test]
    fn crossing_distance_depends_on_the_road_bulbs_and_island() {
        let mut j = Junction::new(0);
        let st = arm_of(&j, 90); // Sample Street 1, road 11.4 m
        let d = |j: &Junction| j.view().arms.iter().find(|a| a.uid == st).unwrap().crossing.as_ref().unwrap().distance_mm;
        assert_eq!(d(&j), 11_400);
        assert!(j.set_bulb(st, 0, true));
        assert_eq!(d(&j), 11_400 - 2_400);
        assert!(j.set_bulb(st, 1, true));
        assert_eq!(d(&j), 11_400 - 4_800);
        let av = arm_of(&j, 0); // Sample Avenue 2, road 20 m, starts with an island
        let cv = |j: &Junction| j.view().arms.iter().find(|a| a.uid == av).unwrap().crossing.as_ref().map(|c| (c.stage_mm, c.stages)).unwrap();
        assert_eq!(cv(&j), (9_000, 2));
        assert!(j.set_island(av, false));
        assert_eq!(cv(&j), (20_000, 1));
    }

    #[test]
    fn the_avenue_is_too_wide_to_cross_without_its_islands() {
        let mut j = Junction::new(0);
        let (n, s) = (arm_of(&j, 0), arm_of(&j, 180));
        let check = |j: &Junction| j.view().checks.iter().find(|c| c.id == "crossing").unwrap().ok;
        assert!(check(&j), "two stages of 9 m");
        assert!(j.set_island(n, false));
        assert!(!check(&j), "20 m in one go");
        assert!(j.set_island(s, false));
        assert!(!check(&j));
        assert!(j.set_island(n, true) && j.set_island(s, true));
        assert!(check(&j));
    }

    #[test]
    fn a_lane_going_only_where_it_is_banned_is_flagged_until_changed() {
        let mut j = Junction::new(0);
        let av = arm_of(&j, 0);
        let (east, south) = (arm_of(&j, 90), arm_of(&j, 180));
        let follows = |j: &Junction| j.view().checks.iter().find(|c| c.id == "lanes-follow").unwrap().ok;
        assert!(follows(&j));
        assert!(j.set_turn(av, east, false));
        assert!(follows(&j), "the left lane still goes straight on");
        assert!(j.set_lane_dest(av, 0, south, false));
        assert!(!follows(&j), "the left lane now only goes east, which is banned");
        assert!(j.set_lane_dest(av, 0, south, true));
        assert!(follows(&j));
    }

    #[test]
    fn an_allowed_turn_with_no_lane_is_flagged() {
        let mut j = Junction::new(0);
        let av = arm_of(&j, 0);
        let east = arm_of(&j, 90);
        // The avenue's left turn is only in its first lane; take it off.
        assert!(j.set_lane_dest(av, 0, east, false));
        let v = j.view();
        assert!(!v.checks.iter().find(|c| c.id == "lanes-cover").unwrap().ok);
        assert!(j.set_lane_dest(av, 1, east, true));
        assert!(j.view().checks.iter().find(|c| c.id == "lanes-cover").unwrap().ok);
    }

    #[test]
    fn a_signal_has_room_for_four_streets() {
        let mut j = Junction::new(3);
        assert!(j.view().checks.iter().find(|c| c.id == "signal").unwrap().ok);
        assert!(j.set_control(SIGNAL));
        assert!(!j.view().checks.iter().find(|c| c.id == "signal").unwrap().ok);
    }

    #[test]
    fn the_priority_street_is_the_pair_nearest_a_straight_line() {
        let j = Junction::new(1);
        let v = j.view();
        let roles: Vec<(i32, &str)> = v.arms.iter().map(|a| (a.bearing, a.role)).collect();
        assert_eq!(roles, vec![(90, "free"), (180, "stop"), (270, "free")]);
    }

    #[test]
    fn a_roundabout_floor_grows_with_the_arms() {
        let mut j = Junction::new(3);
        assert!(j.set_control(ROUNDABOUT));
        let five = j.view().ring.unwrap().floor_mm;
        let mut k = Junction::new(1);
        assert!(k.set_control(ROUNDABOUT));
        let three = k.view().ring.unwrap().floor_mm;
        assert!(five >= three);
        assert!(j.set_ring(4000));
        assert_eq!(j.view().ring.unwrap().radius_mm, five + 4000);
    }

    #[test]
    fn offset_crossings_draw_and_keep_their_arms_apart() {
        let j = Junction::new(2);
        let lay = layout(j.current(), 0).unwrap();
        assert!(lay.arms.iter().all(|a| a.mouth > 0.0));
        let v = j.view();
        assert_eq!(v.arms.len(), 4);
        assert!(v.bounds[0] < -30_000.0 && v.bounds[2] > 30_000.0);
    }

    #[test]
    fn every_sample_draws_in_every_region_and_control() {
        for s in 0..JUNCTION_SAMPLES.len() {
            for region in 0..REGIONS.len() {
                for control in 0..CONTROLS.len() {
                    let mut j = Junction::new(s);
                    j.set_region(region);
                    j.set_control(control);
                    let v = j.view();
                    assert!(!v.arms.is_empty());
                    assert!(v.movements.iter().all(|m| !m.path.is_empty()));
                }
            }
        }
    }

    #[test]
    fn left_hand_traffic_circulates_the_other_way() {
        let mut j = Junction::new(0);
        j.set_control(ROUNDABOUT);
        assert_eq!(j.view().ring.unwrap().circulation, "anticlockwise");
        j.set_region(3);
        assert_eq!(j.view().ring.unwrap().circulation, "clockwise");
    }

    #[test]
    fn dragging_an_arm_turns_it_to_face_the_pointer() {
        let mut j = Junction::new(1); // 90, 180, 270
        let west = arm_of(&j, 270);
        // The plan has north up and y growing down: up and to the left is 315°.
        assert!(j.drag_arm_to(west, -7_071.0, -7_071.0));
        assert_eq!(j.arm(west).unwrap().bearing, 315);
    }

    #[test]
    fn dragging_an_arm_snaps_to_the_step() {
        let mut j = Junction::new(1);
        let west = arm_of(&j, 270);
        // 322.6° is nearer 325° than 320°.
        assert!(j.drag_arm_to(west, -7_650.0, -10_000.0));
        assert_eq!(j.arm(west).unwrap().bearing, 325);
    }

    #[test]
    fn dragging_an_arm_just_short_of_north_wraps_to_north() {
        let mut j = Junction::new(1);
        let west = arm_of(&j, 270);
        // 359.9° rounds to 360°, which is 0°.
        assert!(j.drag_arm_to(west, -10.0, -10_000.0));
        assert_eq!(j.arm(west).unwrap().bearing, 0);
    }

    #[test]
    fn dragging_an_arm_onto_its_neighbour_is_refused() {
        let mut j = Junction::new(1);
        let west = arm_of(&j, 270);
        let before = j.current().clone();
        // Straight down is 180°, where another street already runs.
        assert!(!j.drag_arm_to(west, 0.0, 10_000.0));
        assert_eq!(*j.current(), before);
    }

    #[test]
    fn dragging_an_unknown_arm_does_nothing() {
        let mut j = Junction::new(1);
        assert!(!j.drag_arm_to(9_999, 0.0, -10_000.0));
    }

    #[test]
    fn dragging_the_corner_handle_sets_the_radius_from_the_pointer() {
        let mut j = Junction::new(0);
        let n = arm_of(&j, 0);
        // The handle stands at the arc's apex: `apex_per_radius` times the
        // radius from where the curbs would meet, along the bisector.
        let lay = layout(j.current(), 0).unwrap();
        let i = j.current().arms.iter().position(|a| a.uid == n).unwrap();
        let c = lay.corners.iter().find(|c| c.a == i).unwrap();
        let f = c.fillet.as_ref().unwrap();
        let k = 1.0 / (c.delta as f64 / 2.0).to_radians().sin() - 1.0;
        let apex = |r: f64| add(f.c, scale(f.u, k * r));
        let (x, y) = apex(9_000.0);
        assert!(j.drag_corner_to(n, x, y));
        assert_eq!(j.arm(n).unwrap().corner_mm, 9_000);
    }

    #[test]
    fn dragging_the_corner_handle_beyond_the_limits_is_refused() {
        let mut j = Junction::new(0);
        let n = arm_of(&j, 0);
        let before = j.arm(n).unwrap().corner_mm;
        // On the sharp point itself the radius would be nothing.
        let lay = layout(j.current(), 0).unwrap();
        let i = j.current().arms.iter().position(|a| a.uid == n).unwrap();
        let sharp = lay.corners.iter().find(|c| c.a == i).unwrap().fillet.as_ref().unwrap().c;
        assert!(!j.drag_corner_to(n, sharp.0, sharp.1));
        assert_eq!(j.arm(n).unwrap().corner_mm, before);
    }

    #[test]
    fn a_straight_curb_has_no_corner_handle_to_drag() {
        let mut j = Junction::new(1); // a T: the corner clockwise of the west arm is a straight curb
        let west = arm_of(&j, 270);
        let before = j.arm(west).unwrap().corner_mm;
        assert!(!j.drag_corner_to(west, 1_000.0, 1_000.0));
        assert_eq!(j.arm(west).unwrap().corner_mm, before);
    }

    #[test]
    fn dragging_the_crossing_handle_sets_the_setback_from_the_pointer() {
        let mut j = Junction::new(0);
        let e = arm_of(&j, 90);
        let width = j.view().arms.iter().find(|a| a.uid == e).unwrap().crossing.as_ref().unwrap().width_mm;
        let a = j.view().arms.iter().find(|a| a.uid == e).unwrap().mouth_at;
        // The handle is on the crossing's far edge: setback plus width out
        // along the arm from the mouth.
        let (x, y) = add(a, scale(dir(90.0), 5_000.0 + width as f64));
        assert!(j.drag_crossing_to(e, x, y));
        assert_eq!(j.arm(e).unwrap().crossing.as_ref().unwrap().setback_mm, 5_000);
    }

    #[test]
    fn dragging_the_crossing_handle_beyond_the_limits_is_refused() {
        let mut j = Junction::new(0);
        let e = arm_of(&j, 90);
        let a = j.view().arms.iter().find(|a| a.uid == e).unwrap().mouth_at;
        let (x, y) = add(a, scale(dir(90.0), 40_000.0));
        assert!(!j.drag_crossing_to(e, x, y));
        assert_eq!(j.arm(e).unwrap().crossing.as_ref().unwrap().setback_mm, DEFAULT_SETBACK_MM);
    }

    #[test]
    fn dragging_a_crossing_that_is_not_there_does_nothing() {
        let mut j = Junction::new(0);
        let e = arm_of(&j, 90);
        j.set_crossing(e, false);
        assert!(!j.drag_crossing_to(e, 5_000.0, 0.0));
    }

    #[test]
    fn a_typed_roundabout_size_is_taken_from_its_least_size() {
        let mut j = Junction::new(0);
        assert!(j.set_control(ROUNDABOUT));
        let floor = j.view().ring.unwrap().floor_mm;
        assert!(j.set_ring_radius(floor + 3_000));
        assert_eq!(j.current().ring_extra_mm, 3_000);
        // Smaller than the least size is the least size, not a refusal.
        assert!(j.set_ring_radius(floor - 1_000));
        assert_eq!(j.current().ring_extra_mm, 0);
    }

    #[test]
    fn only_a_roundabout_has_a_size_to_type() {
        let mut j = Junction::new(0);
        assert!(!j.set_ring_radius(10_000));
    }

    #[test]
    fn a_street_can_be_removed_while_the_junction_keeps_three_and_is_not_the_city_s() {
        let mut j = Junction::new(0);
        assert!(j.view().can_remove);
        let uid = j.current().arms[0].uid;
        assert!(j.remove_arm(uid));
        assert!(!j.view().can_remove, "three streets are left");
    }

    #[test]
    fn a_city_junction_cannot_remove_streets() {
        let mut state = Junction::new(0).current().clone();
        for a in &mut state.arms {
            a.edge = a.uid;
        }
        let j = Junction::from_city(Said::new("city-name").with("name", Arg::Text("Test".into())), &state, &state, 0).unwrap();
        assert!(!j.view().can_remove);
    }

    #[test]
    fn a_curb_bulge_is_offered_only_beside_parking() {
        let j = Junction::new(1); // Sample Street 1 has parking both sides; Sample Lane 3 on one
        let v = j.view();
        let by = |sample: usize| v.arms.iter().find(|a| en(&a.street) == crate::street::model::SAMPLES[sample].name).unwrap().can_bulb;
        assert_eq!(by(0), [true, true]);
        assert_eq!(by(2), [false, true]);
    }

    #[test]
    fn turn_classes_and_lane_uses_are_sent_as_words() {
        let j = Junction::new(0);
        let v = serde_json::to_value(j.view()).unwrap();
        let north = v["arms"].as_array().unwrap().iter().find(|a| a["bearing"] == 0).unwrap();
        let uses: Vec<Vec<&str>> =
            north["lanes"].as_array().unwrap().iter().map(|l| l["uses"].as_array().unwrap().iter().map(|u| u.as_str().unwrap()).collect()).collect();
        assert_eq!(uses, vec![vec!["left", "through"], vec!["through", "right"]]);
        // Entering from the north and heading east is a left turn.
        let east = v["arms"].as_array().unwrap().iter().find(|a| a["bearing"] == 90).unwrap();
        let m = v["movements"].as_array().unwrap().iter().find(|m| m["from"] == north["uid"] && m["to"] == east["uid"]).unwrap();
        assert_eq!(m["class"], "left");
        let dests = north["lanes"][0]["dests"].as_array().unwrap();
        assert!(dests.iter().all(|d| ["left", "through", "right"].contains(&d["class"].as_str().unwrap())));
        assert!(dests.iter().any(|d| d["class"] == "through"));
    }

    fn transit_json(j: &Junction, uid: u32) -> serde_json::Value {
        let v = serde_json::to_value(j.view()).unwrap();
        v["arms"].as_array().unwrap().iter().find(|a| a["uid"] == uid).unwrap()["transit"].clone()
    }

    fn tag_codes(t: &serde_json::Value) -> Vec<&str> {
        t["tags"].as_array().unwrap().iter().map(|g| g["code"].as_str().unwrap()).collect()
    }

    #[test]
    fn an_arm_with_no_measures_has_no_tags_or_codes() {
        let j = Junction::new(0);
        let t = transit_json(&j, arm_of(&j, 90));
        assert!(tag_codes(&t).is_empty());
        assert_eq!(t["codes"], serde_json::json!([]));
        assert_eq!(t["approach_kind"], "none");
        assert_eq!(t["dead"], false);
        assert_eq!(t["gate_yields"], false);
    }

    #[test]
    fn each_measure_is_tagged_with_its_atlas_code_where_it_is_drawn() {
        let mut j = Junction::new(0);
        let e = arm_of(&j, 90);
        j.set_bus_lane(e, true);
        j.set_approach(e, Q_CURB);
        j.set_stop(e, STOP_BULB);
        j.set_filter(e, true);
        j.set_rule(e, RULE_WITHIN);
        let t = transit_json(&j, e);
        assert_eq!(tag_codes(&t), vec!["G2", "M1", "N1", "L2"]);
        assert_eq!(t["codes"], serde_json::json!(["G2", "M1", "L2", "N1"]));
        assert_eq!(t["approach_kind"], "queue");
        // The stop's tag sits at the stop; the rule's at the arm's icon spot.
        assert_eq!(t["tags"][1]["at"], t["stop_at"]);
        assert_eq!(t["tags"][3]["at"], t["icon_at"]);
    }

    #[test]
    fn a_queue_jump_is_tagged_at_the_middle_of_its_lane() {
        let mut j = Junction::new(0);
        let e = arm_of(&j, 90);
        j.set_approach(e, Q_CURB);
        let t = transit_json(&j, e);
        let queue = t["queue"].as_array().unwrap();
        let pts: Vec<(f64, f64)> = queue.iter().filter(|c| c[0] != "Z").map(|c| (c[1].as_f64().unwrap(), c[2].as_f64().unwrap())).collect();
        let n = pts.len() as f64;
        let mean = (pts.iter().map(|p| p.0).sum::<f64>() / n, pts.iter().map(|p| p.1).sum::<f64>() / n);
        let at = (t["tags"][0]["at"][0].as_f64().unwrap(), t["tags"][0]["at"][1].as_f64().unwrap());
        assert!((at.0 - mean.0).abs() < 1e-6 && (at.1 - mean.1).abs() < 1e-6, "{at:?} vs {mean:?}");
    }

    #[test]
    fn a_virtual_queue_jump_is_tagged_g3_and_a_gate_h1_or_h2() {
        let mut j = Junction::new(0);
        let e = arm_of(&j, 90);
        assert!(j.set_approach(e, Q_VIRTUAL));
        let t = transit_json(&j, e);
        assert_eq!(tag_codes(&t), vec!["G3"]);
        assert_eq!(t["approach_kind"], "virtual");
        j.set_bus_lane(e, true);
        j.set_approach(e, GATE_SIGNAL);
        let t = transit_json(&j, e);
        assert_eq!((tag_codes(&t), t["approach_kind"].as_str(), t["gate_yields"].as_bool()), (vec!["H1"], Some("gate"), Some(false)));
        j.set_approach(e, GATE_YIELD);
        let t = transit_json(&j, e);
        assert_eq!((tag_codes(&t), t["gate_yields"].as_bool()), (vec!["H2"], Some(true)));
    }

    #[test]
    fn turn_rules_are_tagged_and_a_dead_end_marks_its_arm() {
        let mut j = Junction::new(0);
        let e = arm_of(&j, 90);
        for (rule, code, dead) in [(RULE_AROUND, "L1", false), (RULE_RIRO, "L3", false), (RULE_DEAD_END, "L4", true)] {
            assert!(j.set_rule(e, rule));
            let t = transit_json(&j, e);
            assert_eq!(tag_codes(&t), vec![code], "rule {rule}");
            assert_eq!(t["dead"], dead);
        }
    }

    #[test]
    fn a_selection_named_in_the_view_selects_the_same_target_when_sent_back() {
        let mut j = Junction::new(0);
        assert!(j.set_control(ROUNDABOUT));
        let (a, b) = (arm_of(&j, 0), arm_of(&j, 180));
        assert!(j.set_bus(Some((a, b))) && j.set_cycle(Some(2_000)));
        let targets = j.targets();
        assert!(targets.contains(&Target::Bus) && targets.contains(&Target::Cycle));
        for t in targets {
            j.select(t);
            let s = j.view().selected;
            assert_eq!(Target::named(s.kind.unwrap(), s.uid, s.lane), t);
        }
        j.select(Target::None);
        assert_eq!(j.view().selected.kind, None);
        assert_eq!(Target::named("", 0, 0), Target::None);
    }

    #[test]
    fn an_unknown_selection_name_selects_nothing() {
        assert_eq!(Target::named("wedge", 3, 0), Target::None);
    }

    // ---- signal checks ----

    fn check<'a>(j: &'a JView, id: &str) -> Option<&'a Check> {
        j.checks.iter().find(|c| c.id == id)
    }

    fn plain(said: &str) -> String {
        said.replace(['\u{a0}', '\u{202f}'], " ")
    }

    /// The four-way on a signal where every street's lanes go straight on only, so each crossing is free in two phases.
    fn straight_on() -> Junction {
        let mut j = Junction::new(0);
        let arms: Vec<(i32, u32, usize)> = j.current().arms.iter().map(|a| (a.bearing, a.uid, a.lanes.len())).collect();
        let uid = |b: i32| arms.iter().find(|(x, ..)| *x == b).unwrap().1;
        for &(bearing, from, lanes) in &arms {
            let straight = uid((bearing + 180) % 360);
            for lane in 0..lanes {
                for &(_, other, _) in &arms {
                    if other != from && other != straight {
                        let _ = j.set_lane_dest(from, lane, other, false);
                    }
                }
                let _ = j.set_lane_dest(from, lane, straight, true);
            }
        }
        // The invariant the tests lean on: every lane of every street goes straight on and nowhere else.
        for &(bearing, from, _) in &arms {
            let straight = uid((bearing + 180) % 360);
            assert!(j.arm(from).unwrap().lanes.iter().all(|l| l.to == vec![straight]), "street at {bearing} goes straight only");
        }
        j
    }

    /// The most seconds any crossing needs at the walking speed.
    fn needed_s(v: &JView) -> u32 {
        v.arms.iter().filter_map(|a| a.crossing.as_ref()).map(|c| (c.distance_mm as f64 / 1000.0 / WALK_SPEED_MPS).ceil() as u32).max().unwrap()
    }

    #[test]
    fn there_are_no_signal_timing_checks_off_a_signal() {
        let v = Junction::new(1).view();
        assert!(check(&v, "signal-walk").is_none() && check(&v, "signal-cycle").is_none());
    }

    #[test]
    fn the_cycle_is_checked_against_its_limit_and_named() {
        use crate::junction::model::TimingField::Green;
        let mut j = Junction::new(0);
        let v = j.view();
        let c = check(&v, "signal-cycle").unwrap();
        assert!(c.ok, "four streets at the default 30 s make 120 s, which is the limit and not over it");
        assert!(plain(&en(&c.detail)).contains("The cycle is 120 s"), "{}", en(&c.detail));
        for a in j.current().arms.iter().map(|a| a.uid).collect::<Vec<_>>() {
            assert!(j.set_timing(a, Green, 40));
        }
        let v = j.view();
        let c = check(&v, "signal-cycle").unwrap();
        assert!(!c.ok);
        assert!(plain(&en(&c.detail)).contains("The cycle is 180 s, over the 120 s"), "{}", en(&c.detail));
        assert!(plain(&fr(&c.detail)).contains("Le cycle dure 180 s"), "{}", fr(&c.detail));
    }

    #[test]
    fn a_stock_signal_gives_time_to_cross_because_turning_drivers_give_way() {
        let v = Junction::new(0).view(); // the sample's lanes serve every turn
        let c = check(&v, "signal-walk").unwrap();
        assert!(c.ok, "{}", en(&c.detail));
    }

    #[test]
    fn a_crossing_too_long_for_its_window_fails_and_names_the_streets() {
        use crate::junction::model::{TimingField::Green, junction_of};
        // Six lanes of 3.5 m, a street taking about 21 s to cross: a whole 30 s phase covers it, a 10 s one does not.
        let lanes = [
            ("sidewalk", 2000),
            ("travel", 3500),
            ("travel", 3500),
            ("travel", 3500),
            ("travel", 3500),
            ("travel", 3500),
            ("travel", 3500),
            ("sidewalk", 2000),
        ];
        let mut j = junction_of(&lanes);
        let v = j.view();
        assert!(needed_s(&v) > 10 && needed_s(&v) <= 30, "the street takes {} s to cross", needed_s(&v));
        assert!(check(&v, "signal-walk").unwrap().ok, "{}", en(&check(&v, "signal-walk").unwrap().detail));
        for a in j.current().arms.iter().map(|a| a.uid).collect::<Vec<_>>() {
            assert!(j.set_timing(a, Green, 5));
        }
        let v = j.view();
        let c = check(&v, "signal-walk").unwrap();
        assert!(!c.ok);
        assert!(plain(&en(&c.detail)).starts_with("Not enough time to cross at"), "{}", en(&c.detail));
    }

    #[test]
    fn a_crossing_needs_a_window_as_long_as_the_street_takes_to_cross() {
        use crate::junction::model::TimingField::Green;
        let mut j = straight_on();
        let v = j.view();
        // At the default 30 s a phase, each crossing is free for one whole phase (its two free phases are not next to
        // each other).
        assert!(needed_s(&v) <= 30, "a street of the samples is crossed in {} s", needed_s(&v));
        assert!(check(&v, "signal-walk").unwrap().ok, "{}", en(&check(&v, "signal-walk").unwrap().detail));
        for a in j.current().arms.iter().map(|a| a.uid).collect::<Vec<_>>() {
            assert!(j.set_timing(a, Green, 5));
        }
        let v = j.view();
        let window = 5 + 3 + 2;
        assert_eq!(check(&v, "signal-walk").unwrap().ok, needed_s(&v) <= window, "a {window} s window against {} s", needed_s(&v));
    }

    #[test]
    fn the_sheet_reads_the_cycle_and_each_phase_with_what_runs_beside_it() {
        let mut j = Junction::new(0);
        let n = j.current().arms.iter().find(|a| a.bearing == 0).unwrap().uid;
        assert!(j.set_timing(n, crate::junction::model::TimingField::Green, 30));
        let v = j.view();
        let s = v.signal.as_ref().expect("a signal has a plan");
        assert_eq!(s.cycle_s, 4 * 30 + 5);
        assert_eq!(s.phases.len(), 4);
        assert_eq!(s.phases[0].uid, n);
        assert_eq!(s.phases[0].timing.green_s, 30);
        assert!(Junction::new(1).view().signal.is_none(), "no plan off a signal");
    }

    #[test]
    fn each_phase_carries_its_moves_walks_rides_and_a_summary_by_compass_point() {
        use crate::shared::testing::en;
        let j = Junction::new(0);
        let v = j.view();
        let s = v.signal.as_ref().unwrap();
        let arm = |bearing: i32| v.arms.iter().find(|a| a.bearing == bearing).unwrap();
        let p = &s.phases[0];
        assert_eq!(p.uid, arm(0).uid);
        assert!(!p.moves.is_empty() && p.moves.iter().all(|m| m.from == arm(0).uid), "the north phase serves what enters from the north");
        assert_eq!(p.walk_arms, vec![arm(90).uid, arm(270).uid], "people cross east and west while the north goes");
        assert!(p.ride_arms.is_empty(), "the sample has no bike lanes");
        assert_eq!(en(&p.name), format!("Phase 1: {}", en(&arm(0).street)));
        assert_eq!(en(&p.summary), "Cars go: north. People cross: east and west.");
        assert!(s.phases.iter().all(|p| en(&p.summary).starts_with("Cars go: ")));
        assert!(en(&s.phases[3].name).starts_with("Phase 4: "));
    }

    #[test]
    fn each_phase_says_its_pattern_and_the_patterns_that_fit_and_the_summary_and_drawing_follow_a_choice() {
        use crate::shared::testing::en;
        let mut j = Junction::new(0);
        let v = j.view();
        let p = &v.signal.as_ref().unwrap().phases[0];
        assert_eq!(p.pattern, Pattern::All, "the suggestion serves every movement");
        let offered: Vec<Pattern> = p.patterns.iter().map(|o| o.pattern).collect();
        assert_eq!(offered, vec![Pattern::ThroughOnly, Pattern::All, Pattern::ProtectedLeftThenThrough, Pattern::Scramble]);
        assert_eq!((p.patterns[0].name, p.patterns[0].description), ("jn-pattern-through", "jn-pattern-through-desc"));
        let drawings = crate::junction::phase_svg::phase_drawings(&v);
        assert!(j.set_phase_pattern(0, Pattern::Scramble));
        let v2 = j.view();
        let p2 = &v2.signal.as_ref().unwrap().phases[0];
        assert_eq!(p2.pattern, Pattern::Scramble);
        assert!(p2.moves.is_empty());
        assert_eq!(en(&p2.summary), "People cross: north, east, south and west.");
        assert_ne!(crate::junction::phase_svg::phase_drawings(&v2)[0], drawings[0], "the drawing follows the choice");
        assert_eq!(crate::junction::phase_svg::phase_drawings(&v2)[1], drawings[1], "the other phases are drawn as they were");
        // Taking a street's straight-on movement away leaves a stored phase that matches no pattern: Custom, never offered.
        let mut k = Junction::new(0);
        assert!(k.set_phase_pattern(0, Pattern::ProtectedLeftThenThrough));
        let n = k.view().signal.as_ref().unwrap().phases[0].uid;
        let south = k.view().arms.iter().find(|a| a.bearing == 180).unwrap().uid;
        for lane in 0..k.arm(n).unwrap().lanes.len() {
            k.set_lane_dest(n, lane, south, false);
        }
        let v3 = k.view();
        let p3 = &v3.signal.as_ref().unwrap().phases[0];
        assert_eq!(p3.pattern, Pattern::Custom);
        assert!(!p3.patterns.is_empty() && p3.patterns.iter().all(|o| o.pattern != Pattern::Custom));
    }

    #[test]
    fn a_phase_with_bikes_says_so() {
        use crate::junction::model::junction_of;
        use crate::shared::testing::en;
        let j = junction_of(&[("sidewalk", 2000), ("bike", 1500), ("travel", 3200), ("travel", 3200), ("bike", 1500), ("sidewalk", 2000)]);
        let v = j.view();
        let p = &v.signal.as_ref().unwrap().phases[0];
        assert!(!p.ride_arms.is_empty());
        assert!(en(&p.summary).contains("Bikes go: "), "{}", en(&p.summary));
    }

    #[test]
    fn suggested_goes_with_an_edit_and_comes_back_with_the_restore() {
        use crate::shared::testing::en;
        let mut j = Junction::new(0);
        assert!(j.view().signal.unwrap().suggested);
        assert!(j.remove_phase(0));
        let s = j.view().signal.unwrap();
        assert!(!s.suggested);
        assert_eq!(s.phases.len(), 3);
        assert!(en(&s.phases[0].name).starts_with("Phase 1: "), "numbered again after the removal");
        assert!(j.restore_suggestion());
        let s = j.view().signal.unwrap();
        assert!(s.suggested && s.phases.len() == 4);
    }

    // ---- turbo ring ----

    use crate::junction::turbo::Turbo;

    /// The Avenue and street (sample 0) as a roundabout with a turbo: uids 1 and 3 are the avenue (north, south), 2 and 4 the street.
    fn turbo_state(layout: usize, size: usize) -> State {
        let mut s = Junction::new(0).current().clone();
        s.control = ROUNDABOUT;
        s.turbo = Some(Turbo { layout, size, main: vec![1, 3] });
        s
    }

    /// How far `p` is from the closed outline.
    fn off(outline: &[P], p: P) -> f64 {
        (0..outline.len())
            .map(|i| {
                let (a, b) = (outline[i], outline[(i + 1) % outline.len()]);
                let ab = sub(b, a);
                let t = (((p.0 - a.0) * ab.0 + (p.1 - a.1) * ab.1) / (ab.0 * ab.0 + ab.1 * ab.1).max(1e-9)).clamp(0.0, 1.0);
                dist(p, add(a, scale(ab, t)))
            })
            .fold(f64::MAX, f64::min)
    }

    #[test]
    fn a_turbo_ring_is_the_blocks_outer_edge_and_fits_the_streets_that_meet_it() {
        // The mini block (R4 = 21 150 mm) is the tightest. After widening, the curbs are the Avenue's +-10 000 and the Street's -7 000 and +5 700;
        // a curb at lateral `c` meets the ring at asin(c / R4) degrees from its arm's axis. North to east: the Avenue's right curb at asin(10 000 / 21 150) = 28.2 degrees,
        // the Street's left at 90 - asin(7 000 / 21 150) = 90 - 19.3 = 70.7: an arc of 42.5 degrees. East to south: 90 + asin(5 700 / 21 150) = 105.6 to
        // 180 - 28.2 = 151.8: 46.2 degrees. `ring_fits` asks for 8 000 mm of arc, 8 000 / 21 150 rad = 21.7 degrees, so mini fits and every larger block does.
        for (size, outer) in [21_150.0, 22_450.0, 25_200.0, 29_900.0].into_iter().enumerate() {
            let lay = layout(&turbo_state(0, size), 0).expect("the streets of sample 0 fit every block");
            let ring = lay.ring.expect("a ring");
            assert_eq!((ring.radius, ring.floor as f64), (outer, outer), "block {size}");
            let turbo = ring.turbo.expect("a turbo ring");
            assert_eq!((turbo.block, turbo.lanes, turbo.axis_deg), (size, 2, 57.5));
            assert_eq!(turbo.outline.len(), 2 * (crate::junction::turbo_spiral::STEPS + 1));
        }
    }

    #[test]
    fn a_turbo_with_a_third_lane_is_laid_on_the_larger_outer_radius() {
        let lay = layout(&turbo_state(3, 0), 0).expect("a spiral fits the avenue and street");
        assert_eq!(lay.ring.as_ref().unwrap().radius, 26_450.0, "R4 + 300 + the lane");
        assert_eq!(lay.ring.unwrap().turbo.unwrap().lanes, 3);
    }

    #[test]
    fn a_cycle_track_makes_the_turbo_ring_larger_by_its_width() {
        let mut s = turbo_state(0, 0);
        s.cycle = Some(2_000);
        let ring = layout(&s, 0).unwrap().ring.unwrap();
        assert_eq!(ring.radius, 23_150.0);
    }

    #[test]
    fn the_arms_meet_the_ring_on_its_outline() {
        let lay = layout(&turbo_state(0, 0), 0).unwrap();
        let outline = lay.ring.unwrap().turbo.unwrap().outline;
        for a in &lay.arms {
            assert!(a.mouth >= a.strip0 && a.strip0 > 0.0, "{} {}", a.mouth, a.strip0);
            for t in [a.mouth, a.strip0] {
                assert!([a.cl, a.cr].iter().any(|&lat| off(&outline, at(a.bearing, lat, t)) < 1.0), "the curb meets the ring at {t}");
            }
        }
    }

    #[test]
    fn an_entry_is_widened_to_the_layouts_lanes_away_from_the_middle_of_the_road() {
        let lay = layout(&turbo_state(0, 0), 0).unwrap();
        // Sample Street 1 (east, uid 2) is 18 m across and centred on its arm, so a lateral position is x - 9000. Two lanes of 3.5 m
        // grow its one 3.3 m lane to the span (2000, 9000) (see `widen_entry` in Task 1): laterally (2000 - 9000, 9000 - 9000) = (-7000, 0),
        // where the street had (5700 - 9000, 9000 - 9000) = (-3300, 0). Its own curbs are (3300 - 9000, 14700 - 9000) = (-5700, 5700):
        // the left one moves out to -7000, the right one stays.
        let street = &lay.arms[1];
        assert_eq!(street.prof.enter_x, [7_250, 3_750]);
        assert_eq!(street.prof.enter_span, (2_000, 9_000));
        assert_eq!((street.cl, street.cr), (-7_000.0, 5_700.0), "the left curb moves out to the wider entry; the right is the street's");
        assert_eq!(street.lane_mm, Some(3_500), "(9000 - 2000) / 2");
        assert_eq!(street.entry, Some(EntryWiden { new: (-7_000.0, 0.0), old: (-3_300.0, 0.0) }));
        // Sample Avenue 2 (north, uid 1) is 30 m across (lateral = x - 15000) and has two entering lanes of 3.3 m at 7400..14000. Two of
        // 3.5 m make width = max(7000, 6600) = 7000: the span (14000 - 7000, 14000) = (7000, 14000), the lanes centred at 14000 - 1750 = 12250 and
        // 8750; laterally (7000 - 15000, 14000 - 15000) = (-8000, -1000), where the street had (7400 - 15000, -1000) = (-7600, -1000). Its own
        // curbs (5000 - 15000, 25000 - 15000) = (-10000, 10000) are farther out, so they stay.
        let avenue = &lay.arms[0];
        assert_eq!(avenue.prof.enter_x, [12_250, 8_750]);
        assert_eq!(avenue.prof.enter_span, (7_000, 14_000));
        assert_eq!((avenue.cl, avenue.cr), (-10_000.0, 10_000.0));
        assert_eq!(avenue.lane_mm, Some(3_500));
        assert_eq!(avenue.entry, Some(EntryWiden { new: (-8_000.0, -1_000.0), old: (-7_600.0, -1_000.0) }));
        for a in &lay.arms {
            let e = a.entry.expect("every street of sample 0 is entered");
            assert!(a.pl <= e.new.0 && e.new.1 <= a.pr, "the widened entry stays inside the street's property lines: {e:?} in {}..{}", a.pl, a.pr);
        }
    }

    #[test]
    fn a_plain_roundabout_widens_nothing() {
        let mut s = turbo_state(0, 0);
        s.turbo = None;
        let lay = layout(&s, 0).unwrap();
        assert!(lay.ring.as_ref().unwrap().turbo.is_none());
        assert!(lay.arms.iter().all(|a| a.entry.is_none() && a.lane_mm.is_none()));
    }

    #[test]
    fn a_turbo_that_does_not_suit_its_arms_has_no_layout() {
        let mut s = turbo_state(0, 0);
        s.turbo = Some(Turbo { layout: 0, size: 0, main: vec![1, 9] });
        assert!(layout(&s, 0).is_none(), "a main arm the junction does not have");
        s.turbo = Some(Turbo { layout: 9, size: 0, main: vec![1, 3] });
        assert!(layout(&s, 0).is_none(), "a layout that does not exist");
        s.turbo = Some(Turbo { layout: 0, size: 9, main: vec![1, 3] });
        assert!(layout(&s, 0).is_none(), "a block that does not exist");
        let mut three = Junction::new(1).current().clone();
        three.control = ROUNDABOUT;
        three.turbo = Some(Turbo { layout: 0, size: 0, main: vec![1, 3] });
        assert!(layout(&three, 0).is_none(), "basic is for four arms");
    }

    #[test]
    fn streets_too_wide_for_the_block_leave_no_layout_though_the_plain_roundabout_fits() {
        let mut j = junction_of(&[("travel", 3_500); 11]);
        assert!(j.set_control(ROUNDABOUT), "the plain roundabout fits");
        let mut s = j.current().clone();
        s.turbo = Some(Turbo { layout: 0, size: 3, main: vec![1, 3] });
        assert!(layout(&s, 0).is_none(), "even the large block is too small for streets 38.5 m wide");
    }

    #[test]
    fn eight_travel_lanes_in_a_row_need_the_medium_block() {
        let mut j = junction_of(&[("travel", 3_500); 8]);
        assert!(j.set_control(ROUNDABOUT));
        let mut s = j.current().clone();
        for (size, fits) in [false, false, true, true].into_iter().enumerate() {
            s.turbo = Some(Turbo { layout: 0, size, main: vec![1, 3] });
            assert_eq!(layout(&s, 0).is_some(), fits, "block {size}");
        }
    }

    #[test]
    fn a_three_arm_egg_turns_its_axis_65_degrees_and_left_hand_traffic_turns_it_the_other_way() {
        let mut egg = Junction::new(1).current().clone();
        egg.control = ROUNDABOUT;
        egg.turbo = Some(Turbo { layout: 1, size: 0, main: vec![1, 3] });
        let lay = layout(&egg, 0).expect("an egg fits the three streets");
        assert_eq!(lay.ring.unwrap().turbo.unwrap().axis_deg, 155.0);
        let lay = layout(&turbo_state(0, 0), 3).expect("and the same junction in a country that drives on the left");
        assert_eq!(lay.ring.unwrap().turbo.unwrap().axis_deg, 302.5);
    }

    #[test]
    fn the_plain_roundabout_is_laid_out_as_before() {
        let mut s = turbo_state(0, 0);
        s.turbo = None;
        let ring = layout(&s, 0).unwrap().ring.unwrap();
        assert_eq!(ring.radius, ring.floor as f64);
        assert!(ring.turbo.is_none());
    }

    #[test]
    fn arms_that_cars_do_not_enter_are_not_widened_and_the_layout_still_draws() {
        use crate::shared::catalogue::{Side, StreetClass, kind_index};
        use crate::street::model::{Piece, Street};
        let out: Vec<Piece> = [("sidewalk", 2000), ("travel", 3200), ("sidewalk", 2000)]
            .iter()
            .map(|&(id, width_mm)| Piece { kind: kind_index(id).unwrap(), width_mm, direction: (id == "travel").then_some(0), variants: Vec::new() })
            .collect();
        let mut s = turbo_state(0, 0);
        for a in &mut s.arms {
            match a.bearing {
                90 => a.section = Some(Street::imported(StreetClass::Local, Side::Right, &out)), // goes only one way out
                270 => a.section = Some(Street::imported(StreetClass::Pedestrian, Side::Right, &[])), // a walkway
                _ => {}
            }
        }
        let lay = layout(&s, 0).expect("it draws");
        assert!(lay.ring.unwrap().turbo.is_some());
        for a in &lay.arms {
            let entered = a.bearing == 0.0 || a.bearing == 180.0;
            assert_eq!((a.entry.is_some(), a.lane_mm.is_some()), (entered, entered), "arm {}", a.bearing);
        }
        for (arm, laid) in s.arms.iter().zip(&lay.arms).filter(|(a, _)| a.bearing == 90 || a.bearing == 270) {
            assert_eq!(laid.prof.enter_span, arm.profile(0).enter_span, "the span of arm {} is not widened", arm.bearing);
        }
    }

    #[test]
    fn a_turbo_on_a_junction_with_at_most_one_street_of_traffic_has_no_ring() {
        use crate::shared::catalogue::{Side, StreetClass};
        use crate::street::model::Street;
        let mut s = turbo_state(0, 0);
        for a in s.arms.iter_mut().skip(1) {
            a.section = Some(Street::imported(StreetClass::Pedestrian, Side::Right, &[]));
        }
        assert_eq!(crate::junction::model::traffic_arms(&s), 1);
        assert!(layout(&s, 0).is_none_or(|l| l.ring.is_none()), "it runs uncontrolled, so there is no roundabout, turbo or not");
    }

    #[test]
    fn left_hand_traffic_widens_an_entry_on_the_mirrored_side() {
        let (right, left) = (layout(&turbo_state(0, 0), 0).unwrap(), layout(&turbo_state(0, 0), 3).unwrap());
        let (r, l) = (right.arms[1].entry.expect("the east street is entered"), left.arms[1].entry.expect("and in the left-hand region too"));
        assert_eq!(r.new, (-7_000.0, 0.0), "right-hand: the entry is left of the road's middle");
        assert_eq!(l.new, (0.0, 7_000.0), "left-hand: mirrored, right of it");
        assert_eq!(left.arms[1].lane_mm, Some(3_500));
    }

    fn turbo_junction(layout: usize) -> Junction {
        let mut j = Junction::new(0);
        assert!(j.set_control(ROUNDABOUT));
        assert!(j.set_turbo(Some(layout)));
        j
    }

    #[test]
    fn a_turbo_ring_is_viewed_as_lanes_and_dividers_on_its_block() {
        let v = turbo_junction(0).view();
        let ring = v.ring.as_ref().unwrap();
        assert_eq!((ring.radius_mm, ring.road_mm, ring.island_mm, ring.floor_mm), (21_150, 21_150, 10_500, 21_150));
        assert_eq!(ring.lanes.len(), 4, "a lane on each half of the ring");
        assert!(ring.dividers.len() >= 2, "the divider of each half, cut at the streets: {}", ring.dividers.len());
        assert!(!ring.island_edge.is_empty() && !ring.road_edge.is_empty() && !ring.outer_edge.is_empty());
        let t = v.turbo.as_ref().unwrap();
        assert_eq!((t.layout, t.size, t.layout_id, t.size_key), (0, 0, "basic", "jn-turbo-size-mini"));
        assert_eq!(t.main, [1, 3]);
        assert!(t.can_switch_main && !t.can_smaller && t.can_larger);
    }

    #[test]
    fn a_plain_roundabout_has_no_lanes_dividers_or_turbo_and_is_offered_the_layouts_that_suit_it() {
        let mut j = Junction::new(0);
        j.set_control(ROUNDABOUT);
        let v = j.view();
        let ring = v.ring.as_ref().unwrap();
        assert!(ring.lanes.is_empty() && ring.dividers.is_empty() && ring.island_edge.is_empty());
        assert!(v.turbo.is_none());
        assert_eq!(
            v.turbo_options.iter().map(|o| o.id).collect::<Vec<_>>(),
            ["basic", "egg", "knee", "spiral"],
            "the rotor is not offered: its three lanes on Sample Street 1 would pass the property line"
        );
        assert!(v.turbo_options.iter().all(|o| !o.on));
        assert!(Junction::new(0).view().turbo_options.is_empty(), "a signal offers no turbo");
    }

    #[test]
    fn a_layout_is_offered_only_when_a_main_road_makes_a_turbo_that_stands() {
        let ids = |j: &Junction| j.view().turbo_options.iter().map(|o| o.id).collect::<Vec<_>>();
        let mut j = junction_of(&[("sidewalk", 3_000), ("travel", 3_500), ("travel", 3_500), ("sidewalk", 3_000)]);
        j.set_control(ROUNDABOUT);
        assert_eq!(ids(&j), ["basic", "egg", "knee"], "two lanes of 3.5 m: a spiral's or rotor's three would pass the property line");
        let east = j.current().arms[1].uid;
        assert!(j.set_bearing(east, 105));
        assert_eq!(ids(&j), ["basic", "egg", "knee"], "15 degrees off square is allowed (and reported by a check)");
        assert!(j.set_bearing(east, 115));
        assert!(ids(&j).is_empty(), "25 degrees off the north-south road, and east and west (155 degrees apart) are not opposite: no layout fits");
    }

    #[test]
    fn the_main_road_switch_says_when_it_only_moves_the_two_entry_lanes() {
        let mut j = turbo_junction(2);
        assert!(j.view().turbo.unwrap().main_switch_moves_lanes, "a knee: the next candidate is the same pair reversed");
        assert!(j.switch_turbo_main());
        assert!(!j.view().turbo.unwrap().main_switch_moves_lanes, "and then the other pair");
        assert!(!turbo_junction(0).view().turbo.unwrap().main_switch_moves_lanes, "a basic turbo has no lane to move");
    }

    #[test]
    fn three_streets_are_offered_the_egg_and_the_knee_and_a_y_none() {
        let mut three = Junction::new(1);
        three.set_control(ROUNDABOUT);
        assert_eq!(three.view().turbo_options.iter().map(|o| o.id).collect::<Vec<_>>(), ["egg", "knee"]);
        let (a, b) = (three.current().arms[1].uid, three.current().arms[2].uid);
        three.set_bearing(a, 210);
        three.set_bearing(b, 330);
        assert!(three.view().turbo_options.is_empty(), "no two streets are opposite");
    }

    #[test]
    fn the_layout_chosen_is_the_one_on_and_a_third_lane_makes_three_ring_lanes() {
        let v = turbo_junction(3).view();
        assert_eq!(v.turbo_options.iter().filter(|o| o.on).map(|o| o.id).collect::<Vec<_>>(), ["spiral"]);
        assert_eq!(v.ring.unwrap().lanes.len(), 6);
    }

    #[test]
    fn a_cycle_track_surrounds_the_turbo_road() {
        let mut j = turbo_junction(0);
        assert!(j.set_cycle_track(true));
        let ring = j.view().ring.unwrap();
        assert_eq!((ring.radius_mm, ring.road_mm, ring.cycle_mm), (23_150, 21_150, Some(2_000)));
        assert!(ring.outer_edge != ring.road_edge);
    }

    #[test]
    fn the_entering_lanes_of_a_turbo_are_the_layouts_fixed_and_3_5_m_wide() {
        let v = turbo_junction(0).view();
        let street = v.arms.iter().find(|a| a.uid == 2).unwrap();
        assert_eq!(street.lanes.len(), 2);
        assert!(street.lanes.iter().all(|l| l.width_mm == 3_500 && l.fixed), "two lanes of 3.5 m on Sample Street 1: (9000 - 2000) / 2");
        assert_eq!(street.leave_arrows.len(), 1, "the way out is the street's own");
        assert!(street.leave_arrows.iter().all(|l| !l.fixed));
        let mut plain = Junction::new(0);
        plain.set_control(ROUNDABOUT);
        assert!(plain.view().arms.iter().flat_map(|a| &a.lanes).all(|l| !l.fixed));
    }

    #[test]
    fn a_widened_entry_is_a_flaring_carriageway_and_a_street_that_needs_no_more_has_none() {
        let v = turbo_junction(0).view();
        assert!(v.arms.iter().all(|a| a.turbo_entry.as_ref().is_some_and(|e| e.len() == 7)), "six corners and a close, on every arm of sample 0");
        let mut plain = Junction::new(0);
        plain.set_control(ROUNDABOUT);
        assert!(plain.view().arms.iter().all(|a| a.turbo_entry.is_none()));
        let mut wide = junction_of(&[("travel", 3_500); 8]);
        wide.set_control(ROUNDABOUT);
        assert!(wide.set_turbo(Some(0)));
        assert!(wide.view().arms.iter().all(|a| a.turbo_entry.is_none()), "the layout wants no more than the street has");
    }

    #[test]
    fn an_arm_that_cars_do_not_enter_draws_no_turbo_entry_and_nothing_breaks() {
        use crate::junction::model::junction_with_walkways;
        // A turbo needs streets of traffic (a junction with at most one runs uncontrolled), so one walkway among four.
        let mut j = junction_with_walkways(&[0], &[("sidewalk", 3_000), ("bike", 2_000), ("sidewalk", 3_000)]);
        assert!(j.set_control(ROUNDABOUT));
        assert!(j.set_turbo(Some(0)));
        let v = j.view();
        let walkway = v.arms.iter().find(|a| a.bearing == 0).unwrap();
        assert!(walkway.pedestrian && walkway.turbo_entry.is_none() && walkway.lanes.is_empty());
        assert!(v.arms.iter().filter(|a| a.bearing != 0).all(|a| a.turbo_entry.is_some() && !a.lanes.is_empty()));
        assert_eq!(v.ring.unwrap().lanes.len(), 4);
    }

    #[test]
    fn a_turbo_s_corners_follow_the_ring_so_the_wedges_are_closed_paths() {
        let v = turbo_junction(0).view();
        for c in &v.corners {
            assert!(c.curb.len() > 4, "the curb walks along the ring between the two streets: {}", c.curb.len());
            assert!(c.wedge.last().is_some_and(|z| z[0] == "Z"));
        }
    }

    #[test]
    fn no_movement_on_a_turbo_crosses_a_divider_in_any_layout_block_or_side_of_the_road() {
        // Streets of three lanes in and one out (3.5 m each), so that every layout, the spiral and the rotor included, stands, and every
        // street has a way in and a way out: `junction_of` makes every lane come in, and a four-way of it has no movement to check.
        let (mut checked, mut checked_three) = (0, 0);
        for region in [0usize, 3] {
            for (three_arms, layouts) in [(false, vec![0, 1, 2, 3, 4]), (true, vec![1, 2])] {
                let mut j = if three_arms { Junction::new(1) } else { junction_two_way(3, 1, region) };
                if three_arms && region != 0 {
                    assert!(j.set_region(region));
                }
                assert!(j.set_control(ROUNDABOUT));
                for layout in layouts {
                    assert!(j.set_turbo(Some(layout)), "layout {layout}, region {region}, three arms {three_arms}");
                    for size in 0..4 {
                        let _ = j.set_turbo_size(size); // the block it already has is no change
                        let v = j.view();
                        let ring = v.ring.as_ref().unwrap();
                        assert!(!ring.dividers.is_empty());
                        for m in &v.movements {
                            let path = sample_path(&m.path);
                            for d in &ring.dividers {
                                assert!(
                                    !polylines_cross(&path, d),
                                    "layout {layout} block {size} region {region} three arms {three_arms}: {} to {} crosses a divider",
                                    m.from,
                                    m.to
                                );
                            }
                            checked += 1;
                            if three_arms {
                                checked_three += 1;
                            }
                        }
                    }
                }
            }
        }
        // Four arms: 12 movements, 5 layouts, 4 blocks, 2 sides = 480; three arms (the 65 degree axis): 6 movements, 2 layouts, 4 blocks, 2 sides = 96.
        assert_eq!(checked_three, 96);
        assert_eq!(checked, 576);
    }

    #[test]
    fn a_street_15_degrees_off_square_and_a_cycle_track_still_meet_the_ring_and_cross_no_divider() {
        let mut j = Junction::new(0);
        assert!(j.set_control(ROUNDABOUT));
        assert!(j.set_turbo(Some(0)));
        let east = j.current().arms[1].uid;
        assert!(j.set_bearing(east, 105));
        assert!(j.set_cycle_track(true));
        let v = j.view();
        let ring = v.ring.as_ref().unwrap();
        assert!(!ring.dividers.is_empty());
        assert!(v.movements.iter().count() > 0);
        for m in &v.movements {
            let path = sample_path(&m.path);
            assert!(path.iter().all(|p| p.0.is_finite() && p.1.is_finite()), "{} to {}", m.from, m.to);
            assert!(ring.dividers.iter().all(|d| !polylines_cross(&path, d)), "{} to {} crosses a divider", m.from, m.to);
        }
    }

    #[test]
    fn a_street_at_bearing_zero_is_walked_to_the_same_as_one_at_bearing_360() {
        use crate::junction::turbo_spiral::{Axes, Mouth, ring_shape, spiral_walk};
        let block = &BLOCKS[1];
        let axes = Axes::new(57.5, true);
        let mouths = |north: f64| -> Vec<Mouth> { [north, 90.0, 180.0, 270.0].iter().map(|&bearing| Mouth { bearing, cl: -3_500.0, cr: 3_500.0 }).collect() };
        for k in 0..2 {
            let a = ring_shape(block, 2, 0, &axes, true, &mouths(0.0));
            let b = ring_shape(block, 2, 0, &axes, true, &mouths(360.0));
            for from in [90.0, 180.0, 270.0] {
                let (wa, wb) = (spiral_walk(&a, k, (from, -1_750.0), (0.0, 1_750.0)), spiral_walk(&b, k, (from, -1_750.0), (360.0, 1_750.0)));
                assert_eq!(wa.is_some(), wb.is_some(), "lane {k} from {from}");
                let (wc, wd) = (spiral_walk(&b, k, (from, -1_750.0), (0.0, 1_750.0)), spiral_walk(&a, k, (from, -1_750.0), (360.0, 1_750.0)));
                assert_eq!(wa.is_some(), wc.is_some(), "lane {k} from {from}");
                assert_eq!(wa.is_some(), wd.is_some(), "lane {k} from {from}");
            }
        }
    }

    #[test]
    fn a_turbo_ring_is_a_spiral_not_two_closed_loops() {
        let v = turbo_junction(0).view();
        let ring = v.ring.as_ref().unwrap();
        assert!(ring.dividers.iter().all(|d| d.len() >= 2 && d.first() != d.last()), "every divider is an open piece");
        assert!(ring.lanes.iter().all(|l| l.centre.first() != l.centre.last()), "every lane is open");
        // The island's edge steps along the translation axis (57.5 degrees, right-hand) at the two tips: two long sides, and they lie ON the
        // axis. A stadium's long sides ran beside it, a radius off.
        let axes = Axes::new(57.5, true);
        let island = &ring.island_edge;
        let n = island.len();
        let steps: Vec<P> =
            (0..n).filter(|&i| dist(island[i], island[(i + 1) % n]) > 4_000.0).map(|i| scale(add(island[i], island[(i + 1) % n]), 0.5)).collect();
        assert_eq!(steps.len(), 2, "one step at each tip");
        assert!(steps.iter().all(|m| (m.0 * axes.side.0 + m.1 * axes.side.1).abs() < 1.0), "on the axis: {steps:?}");
    }

    #[test]
    fn the_two_way_fixture_has_streets_cars_enter_and_leave_by() {
        for region in [0usize, 3] {
            let mut j = junction_two_way(3, 1, region);
            assert!(j.set_control(ROUNDABOUT), "region {region}");
            assert!(j.set_turbo(Some(0)), "the basic turbo stands on it, region {region}");
            let v = j.view();
            assert_eq!(v.movements.len(), 12, "four streets, each to the three others (`junction_of`'s streets are all one-way in: no movement at all)");
            assert!(v.arms.iter().all(|a| a.lanes.len() == 2 && a.leaves), "region {region}");
        }
    }

    #[test]
    fn every_layout_and_block_draws_without_a_missing_number() {
        // Four streets of three entering lanes of 3.5 m: a spiral's or rotor's three lanes need no more room than the street has.
        let mut j = junction_of(&[("sidewalk", 3_000), ("travel", 3_500), ("travel", 3_500), ("travel", 3_500), ("sidewalk", 3_000)]);
        assert!(j.set_control(ROUNDABOUT));
        let finite = |v: &JView| {
            let ring = v.ring.as_ref().unwrap();
            let pts = ring
                .lanes
                .iter()
                .flat_map(|l| l.inner.iter().chain(&l.outer).chain(&l.centre))
                .chain(ring.dividers.iter().flatten())
                .chain(&ring.island_edge)
                .chain(&ring.road_edge)
                .chain(&ring.outer_edge);
            let entries = v.arms.iter().filter_map(|a| a.turbo_entry.as_ref()).flatten().flat_map(|c| c.as_array().unwrap().iter().skip(1));
            let moves = v.movements.iter().flat_map(|m| sample_path(&m.path));
            pts.into_iter().all(|p| p.0.is_finite() && p.1.is_finite())
                && entries.into_iter().all(|n| n.is_number())
                && moves.into_iter().all(|p| p.0.is_finite() && p.1.is_finite())
        };
        for layout in 0..5 {
            assert!(j.set_turbo(Some(layout)), "layout {layout}");
            for size in 0..4 {
                let _ = j.set_turbo_size(size); // the block it already has is no change
                assert_eq!(j.current().turbo.as_ref().unwrap().size, size, "layout {layout}");
                assert!(finite(&j.view()), "layout {layout} block {size}");
            }
        }
    }

    /// The ring lanes (their index, 0 the innermost) a movement runs along on the ring, in order, each once in a row: the middle of its path, all but the way in and the way out.
    fn lanes_followed(v: &JView, from: u32, to: u32) -> Vec<usize> {
        let ring = v.ring.as_ref().unwrap();
        let m = v.movements.iter().find(|m| m.from == from && m.to == to).unwrap();
        let pts = sample_path(&m.path);
        let mut seen: Vec<usize> = Vec::new();
        for p in &pts[2..pts.len() - 2] {
            let k = ring
                .lanes
                .iter()
                .find(|l| l.centre.iter().any(|c| (c.0 - p.0).abs() < 1.5 && (c.1 - p.1).abs() < 1.5))
                .map(|l| l.k)
                .expect("every point of the run round the ring lies on a lane's centre line");
            if seen.last() != Some(&k) {
                seen.push(k);
            }
        }
        seen
    }

    #[test]
    fn a_movement_on_a_turbo_follows_its_lane_and_steps_out_one_lane_at_the_axis() {
        // The basic turbo on sample 0, right-hand: the axis points 57.5 degrees, so the east and south streets are on the ahead half and
        // the north and west streets on the other, and traffic goes anticlockwise (north, west, south, east): see the spec's J1.
        let v = turbo_junction(0).view();
        assert_eq!(
            lanes_followed(&v, 1, 2),
            [0, 1],
            "north to east is a left turn, three quarters round: the inner lane to the back tip, where it steps out to the outer lane, which leaves at east"
        );
        assert_eq!(lanes_followed(&v, 1, 4), [1], "north to west is a right turn: the outer lane, and it never reaches a tip");
        assert_eq!(lanes_followed(&v, 1, 3), [0, 1], "straight on takes the first lane that serves it, the inner one, and steps out at the back tip");
    }

    #[test]
    fn left_hand_traffic_circulates_the_other_way_and_feeds_the_mirrored_ring_lane() {
        let mut j = Junction::new(0);
        assert!(j.set_control(ROUNDABOUT) && j.set_region(3) && j.set_turbo(Some(0)));
        let v = j.view();
        assert_eq!(v.ring.as_ref().unwrap().circulation, "clockwise");
        assert_eq!(
            lanes_followed(&v, 1, 2),
            [1],
            "the driver's left lane is now the outside: a left turn takes the outer ring lane, and east comes soon after north on the same half"
        );
    }

    #[test]
    fn traffic_on_a_turbo_only_merges_and_diverges_and_the_lane_checks_run_on_the_layouts_lanes() {
        let v = turbo_junction(0).view();
        assert_eq!((v.conflicts.crossing, v.conflicts.merging, v.conflicts.diverging), (0, 4, 4));
        assert!(v.checks.iter().find(|c| c.id == "lanes-cover").unwrap().ok);
        assert!(v.checks.iter().find(|c| c.id == "lanes-follow").unwrap().ok);
    }

    #[test]
    fn the_turbo_square_check_reports_the_worst_arm_from_10_degrees() {
        let mut j = junction_of(&[("sidewalk", 3_000), ("travel", 3_500), ("travel", 3_500), ("sidewalk", 3_000)]);
        j.set_control(ROUNDABOUT);
        assert!(j.view().checks.iter().all(|c| c.id != "turbo-square"), "no turbo, no check");
        assert!(j.set_turbo(Some(0)));
        let find = |j: &Junction| j.view().checks.into_iter().find(|c| c.id == "turbo-square").unwrap();
        let ok = find(&j);
        assert!(ok.ok);
        assert_eq!(en(&ok.label), "Arms meet the ring square");
        assert_eq!(fr(&ok.label), "Les rues abordent l’anneau d’équerre");
        let east = j.current().arms[1].uid;
        assert!(j.set_bearing(east, 95));
        assert!(find(&j).ok, "5 degrees off is square enough");
        assert!(j.set_bearing(east, 100));
        let bad = find(&j);
        assert!(!bad.ok, "10 degrees or more is reported");
        assert!(en(&bad.detail).ends_with(" is 10° off square."), "{}", en(&bad.detail));
        assert!(fr(&bad.detail).ends_with(" s’écarte de 10° de l’équerre."), "{}", fr(&bad.detail));
    }

    #[test]
    fn a_turbo_layout_no_block_can_hold_is_not_offered() {
        let mut j = junction_of(&[("travel", 3_500); 11]);
        assert!(j.set_control(ROUNDABOUT), "the plain roundabout fits");
        let offered: Vec<usize> = j.view().turbo_options.iter().map(|o| o.layout).collect();
        for layout in 0..LAYOUTS.len() {
            let mut fresh = junction_of(&[("travel", 3_500); 11]);
            assert!(fresh.set_control(ROUNDABOUT));
            let accepted = fresh.set_turbo(Some(layout));
            assert_eq!(offered.contains(&layout), accepted, "layout {layout}: offered {offered:?}, set_turbo {accepted}");
        }
        assert!(!offered.contains(&0), "the basic turbo is refused as too big here, so it is not offered");
    }

    #[test]
    fn what_is_offered_is_what_set_turbo_accepts_whatever_the_turbo_already_on() {
        let streets: [&[(&str, i32)]; 3] =
            [&[("travel", 3_500); 11], &[("sidewalk", 3_000), ("travel", 3_500), ("travel", 3_500), ("sidewalk", 3_000)], &[("travel", 3_500); 5]];
        for street in streets {
            for start in [None, Some((0, 3)), Some((2, 1)), Some((3, 3))] {
                let build = || {
                    let mut j = junction_of(street);
                    assert!(j.set_control(ROUNDABOUT));
                    if let Some((layout, size)) = start {
                        // Whatever the junction takes of it: the turbo may be refused here, and the junction then has none.
                        let _ = j.set_turbo(Some(layout)) && j.set_turbo_size(size);
                    }
                    j
                };
                let offered: Vec<usize> = build().view().turbo_options.iter().map(|o| o.layout).collect();
                for layout in 0..LAYOUTS.len() {
                    let accepted = build().set_turbo(Some(layout));
                    let on_already = build().current().turbo.as_ref().is_some_and(|t| t.layout == layout);
                    assert!(
                        offered.contains(&layout) == accepted || on_already,
                        "{street:?} from {start:?}, layout {layout}: offered {offered:?}, accepted {accepted}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_steady_part_exactly_as_long_as_the_arrows_need_passes() {
        let arrows = 23_000.0;
        let beyond = |steady: i32| Beyond { boundary_mm: steady, next: Vec::new(), zone: Said::new("zone-unnamed-mid"), taper: None };
        let arm = Said::new("city-name");
        let need = (arrows + ARROW_CLEAR_MM) as i32;
        assert!(approach_problems(&arm, &beyond(need), arrows).is_empty(), "equal passes");
        assert_eq!(approach_problems(&arm, &beyond(need - 1), arrows).len(), 1, "a millimetre short fails");
    }
}
