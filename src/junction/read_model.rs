//! Junction geometry, checks and the serialised view. `layout` decides where
//! everything sits in plan; `Junction::view` turns it into what the page draws.
//! The page does no geometry of its own.

use serde::Serialize;
use serde_json::Value;

use crate::junction::geometry::*;
use crate::junction::model::*;
use crate::shared::catalogue::{KINDS, REGIONS, Side, is_roadway};
use crate::street::model::{Check, Revision};

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
/// Synthetic thresholds for the checks.
const MAX_STAGE_MM: i32 = 15_000;
const MAX_TURN_KMH: f64 = 20.0;
const MIN_SIDEWALK_AT_CORNER_MM: f64 = 1_800.0;
const MAX_SIGNAL_ARMS: usize = 4;

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
}

pub struct Layout {
    pub arms: Vec<ArmLayout>,
    pub corners: Vec<CornerLayout>,
    pub ring: Option<RingLayout>,
}

/// Puts the junction on the sheet, or says it cannot be drawn: a corner whose
/// curbs meet behind the junction, a fillet longer than its arm, a roundabout
/// that will not fit its arms in the largest ring.
pub fn layout(s: &State, region: usize) -> Option<Layout> {
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
            }
        })
        .collect();
    let limit = ARM_LENGTH_MM as f64 * 0.75;
    let mut corners = Vec::new();

    if s.control == ROUNDABOUT {
        let floor = ring_floor(&arms)?;
        let radius = (floor + s.ring_extra_mm).min(MAX_RING_MM) as f64;
        for a in arms.iter_mut() {
            let (tl, tr) = (on_circle(a.cl, radius)?, on_circle(a.cr, radius)?);
            a.mouth = tl.max(tr);
            a.strip0 = tl.min(tr);
        }
        for i in 0..n {
            let j = (i + 1) % n;
            corners.push(CornerLayout { a: i, b: j, delta: gap(s.arms[i].bearing, s.arms[j].bearing) as i32, fillet: None });
        }
        return Some(Layout { arms, corners, ring: Some(RingLayout { radius, floor }) });
    }

    let mut mouths = vec![0.0f64; n];
    for i in 0..n {
        let j = (i + 1) % n;
        let delta = gap(s.arms[i].bearing, s.arms[j].bearing) as i32;
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

/// The least outer radius that keeps each arm's curbs on the ring and leaves
/// room between neighbouring arms.
fn ring_floor(arms: &[ArmLayout]) -> Option<i32> {
    let n = arms.len();
    let mut r = MIN_RING_MM;
    while r <= MAX_RING_MM {
        let rf = r as f64;
        let ok = (0..n).all(|i| {
            let (a, b) = (&arms[i], &arms[(i + 1) % n]);
            if a.cr.abs() >= rf || b.cl.abs() >= rf {
                return false;
            }
            let pa = a.bearing + (a.cr / rf).asin().to_degrees();
            let qb = b.bearing + (b.cl / rf).asin().to_degrees();
            let arc = (qb - pa).rem_euclid(360.0);
            arc >= (MIN_RING_ARC_MM / rf).to_degrees() && arc < 360.0 - 1e-6 && arc <= 180.0 + 1e-6 + (b.bearing - a.bearing).rem_euclid(360.0)
        }) && arms.iter().all(|a| a.cl.abs() < rf && a.cr.abs() < rf);
        if ok {
            return Some(r);
        }
        r += RING_STEP_MM;
    }
    None
}

// ---- view -------------------------------------------------------------------

#[derive(Serialize)]
pub struct PieceView {
    pub kind: usize,
    pub material: &'static str,
    pub direction: Option<&'static str>,
    pub poly: Vec<Value>,
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
}

#[derive(Serialize)]
pub struct DestView {
    pub uid: u32,
    pub label: String,
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
    pub problems: Vec<String>,
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
pub struct ArmView {
    pub uid: u32,
    /// The city street this arm is, or 0 outside a city.
    pub edge: u32,
    pub label: String,
    pub street: String,
    pub bearing: i32,
    pub offset_mm: i32,
    pub corner_mm: i32,
    pub pieces: Vec<PieceView>,
    /// No-parking stretches beside the curb, drawn as plain road.
    pub gaps: Vec<Vec<Value>>,
    pub bulbs: [Option<Vec<Value>>; 2],
    pub crossing: Option<CrossingView>,
    pub stop_line: Option<[P; 2]>,
    pub lanes: Vec<LaneView>,
    pub leave_arrows: Vec<LaneView>,
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
    pub transit: TransitView,
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
    pub blocked: Option<&'static str>,
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
    pub label: String,
}

#[derive(Serialize)]
pub struct Selection {
    pub kind: Option<&'static str>,
    pub uid: u32,
    /// For a lane, its place among the arm's entering lanes.
    pub lane: usize,
}

#[derive(Serialize)]
pub struct JView {
    pub name: String,
    /// A place in a city: its streets are the city's.
    pub linked: bool,
    /// A street can be taken away: the junction is not the city's and keeps three.
    pub can_remove: bool,
    pub region: &'static str,
    pub drive_side: &'static str,
    pub control: &'static str,
    pub control_index: usize,
    pub ring: Option<RingView>,
    pub ring_extra_mm: i32,
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
    pub revisions: Vec<Revision>,
    pub selected: Selection,
    pub bounds: [f64; 4],
    pub can_undo: bool,
    pub can_redo: bool,
    pub changed: bool,
}

/// Every pair of streets a bus lane could join across a roundabout, the one
/// nearest a straight line first.
fn bus_options(s: &State) -> Vec<BusOption> {
    if s.control != ROUNDABOUT {
        return Vec::new();
    }
    let mut v: Vec<(i32, BusOption)> = Vec::new();
    for (i, a) in s.arms.iter().enumerate() {
        for b in &s.arms[i + 1..] {
            let off = (gap(a.bearing, b.bearing) as i32 - 180).abs();
            v.push((off, BusOption { a: a.uid, b: b.uid, label: format!("{} to {}", arm_name(a), arm_name(b)) }));
        }
    }
    v.sort_by_key(|(off, o)| (*off, o.a, o.b));
    v.into_iter().map(|(_, o)| o).collect()
}

fn heading(bearing: f64) -> i32 {
    (bearing + 180.0).rem_euclid(360.0).round() as i32
}

fn control_role(s: &State, i: usize) -> &'static str {
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
    let mut best = ([0, 1], i32::MAX);
    for i in 0..n {
        for j in i + 1..n {
            let d = (gap(arms[i].bearing, arms[j].bearing) as i32 - 180).abs();
            let wide = arms[i].row_mm() + arms[j].row_mm();
            let score = d * 100_000 - wide;
            if score < best.1 {
                best = ([i, j], score);
            }
        }
    }
    best.0
}

fn lane_view(a: &ArmLayout, off: f64, x_mm: i32, t: f64, head: i32, uses: u8) -> LaneView {
    let w = a.prof.pieces.iter().find(|p| p.x_mm + p.width_mm / 2 == x_mm && KINDS[p.kind].id == "travel").map_or(3_000, |p| p.width_mm);
    let c = a.lat(off, x_mm);
    LaneView {
        uses,
        dests: Vec::new(),
        bad: false,
        at: at(a.bearing, c, t),
        heading: head,
        poly: poly(&strip(a.bearing, c - w as f64 / 2.0, c + w as f64 / 2.0, a.mouth, ARM_LENGTH_MM as f64)),
        width_mm: w,
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
        let s = self.current();
        let region = self.region;
        // The current state is always valid; the layout it was checked with is
        // rebuilt here. A gesture may hold a state the layout rejects only if a
        // rule above was skipped, so fall back to the last good one.
        let lay = layout(s, region).expect("a junction state is always drawable");
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
            let far = l.mouth + cx.map_or(0.0, |c| (c.setback_mm + c.width_mm) as f64);
            let park_t = far + if cx.is_some() { PARK_CLEAR_MM } else { NO_CROSSING_CLEAR_MM };
            let clear_far = clear_far(a, l);
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
                if inside && !road {
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
                pieces.push(PieceView { kind: p.kind, material: p.material, direction: p.direction, poly: poly(&strip(l.bearing, x0, x1, t0, len)) });
            }
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
                    handle: at(l.bearing, (cl + cr) / 2.0 - (cr - cl) * 0.28, t1),
                    distance_mm: distance,
                    stage_mm: if c.island { (distance - ISLAND_MM) / 2 } else { distance },
                    stages: if c.island { 2 } else { 1 },
                    too_far: (if c.island { (distance - ISLAND_MM) / 2 } else { distance }) > MAX_STAGE_MM,
                }
            });
            let enters = !l.prof.enter_x.is_empty();
            let leaves = !l.prof.leave_x.is_empty();
            let stop_line = (cx.is_some() && enters).then(|| {
                let t = far + 600.0;
                [at(l.bearing, l.lat(off, l.prof.enter_span.0), t), at(l.bearing, l.lat(off, l.prof.enter_span.1), t)]
            });
            let arrow_t = if cx.is_some() { far + 5_000.0 } else { l.mouth + 6_000.0 };
            let lanes = l
                .prof
                .enter_x
                .iter()
                .zip(&a.lanes)
                .map(|(&x, lane)| {
                    let uses = lane.to.iter().filter_map(|t| s.arms.iter().find(|b| b.uid == *t)).fold(0, |m, b| m | turn_class(a.bearing, b.bearing));
                    let mut v = lane_view(l, off, x, arrow_t, heading(l.bearing), uses);
                    let mut dests: Vec<(i32, DestView)> = s
                        .arms
                        .iter()
                        .zip(&lay.arms)
                        .enumerate()
                        .filter(|(_, (b, _))| b.uid != a.uid)
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
            let leave_arrows = l.prof.leave_x.iter().map(|&x| lane_view(l, off, x, arrow_t, l.bearing.round() as i32, 0)).collect();
            let road_mm = l.prof.road_mm();
            arms.push(ArmView {
                uid: a.uid,
                edge: a.edge,
                label: arm_name(a),
                street: l.prof.name.clone(),
                bearing: a.bearing,
                offset_mm: a.offset_mm,
                corner_mm: a.corner_mm,
                pieces,
                gaps,
                bulbs,
                crossing,
                stop_line,
                lanes,
                leave_arrows,
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
                transit: transit_view(s, a, l, off, far, crossing_present, side),
            });
        }

        // Corners, the pavement wedges and the carriageway core.
        let mut corners = Vec::new();
        let mut core = Vec::new();
        let ring = lay.ring.as_ref();
        for c in &lay.corners {
            let (la, lb) = (&lay.arms[c.a], &lay.arms[c.b]);
            let (fa, fb) = (clear_far(&s.arms[c.a], la), clear_far(&s.arms[c.b], lb));
            // The curb's two ends, where it leaves each arm's straight edge.
            let (ca_in, cb_in, curb_mid): (P, P, Vec<Value>) = match (&c.fillet, ring) {
                (Some(f), _) => (f.a, f.b, vec![arc(f.r, false, cross(sub(f.a, f.o), sub(f.b, f.o)) > 0.0, f.b)]),
                (None, Some(r)) => {
                    let pa = at(la.bearing, la.cr, on_circle(la.cr, r.radius).unwrap_or(0.0));
                    let qb = at(lb.bearing, lb.cl, on_circle(lb.cl, r.radius).unwrap_or(0.0));
                    (pa, qb, vec![ring_arc(r.radius, pa, qb)])
                }
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
            corners.push(CornerView {
                uid: ua,
                next_uid: ub,
                radius_mm: radius,
                speed_kmh: if c.fillet.is_some() { speed_kmh(radius) } else { 0.0 },
                ok,
                fast: c.fillet.is_some() && speed_kmh(radius) > MAX_TURN_KMH && (s.arms[c.a].crossing.is_some() || s.arms[c.b].crossing.is_some()),
                straight: c.fillet.is_none(),
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
        for (i, a) in s.arms.iter().enumerate() {
            for (j, b) in s.arms.iter().enumerate() {
                if i == j {
                    continue;
                }
                let (la, lb) = (&lay.arms[i], &lay.arms[j]);
                let (Some(ea), Some(xb)) = (span_mid(la.prof.enter_span), span_mid(lb.prof.leave_span)) else { continue };
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
                } else {
                    movement_path(la.bearing, pe, lb.bearing, px, rad, side, a.bearing, b.bearing)
                };
                movements.push(MoveView { from: a.uid, to: b.uid, class, allowed, blocked, indirect, lane, path });
            }
        }
        for (a, arm) in arms.iter_mut().zip(&s.arms) {
            for (lane, l) in a.lanes.iter_mut().zip(&arm.lanes) {
                lane.bad = !l.to.iter().any(|t| movements.iter().any(|m| m.from == arm.uid && m.to == *t && m.allowed));
            }
        }
        let conflicts = conflicts(s, &movements, &lay, side);

        let checks = checks(s, &arms, &corners, &movements, &lay);
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
            name: self.name().to_string(),
            linked: self.is_linked(),
            can_remove: !self.is_linked() && n > MIN_ARMS,
            region: REGIONS[region].id,
            drive_side: if side == Side::Left { "left" } else { "right" },
            control: CONTROLS[s.control].id,
            control_index: s.control,
            ring: ring.map(|r| RingView {
                radius_mm: r.radius as i32,
                floor_mm: r.floor,
                road_mm: r.radius as i32 - s.cycle.unwrap_or(0),
                island_mm: (r.radius - s.cycle.unwrap_or(0) as f64 - RING_WIDTH_MM).max(MIN_ISLAND_MM) as i32,
                cycle_mm: s.cycle,
                circulation: if side == Side::Right { "anticlockwise" } else { "clockwise" },
            }),
            ring_extra_mm: s.ring_extra_mm,
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
            revisions: self.revisions().enumerate().map(|(i, l)| Revision { step: i + 1, label: l.to_string() }).collect(),
            selected,
            bounds: [mins.0, mins.1, maxs.0, maxs.1],
            can_undo: self.can_undo(),
            can_redo: self.can_redo(),
            changed: self.changed(),
        }
    }
}

/// How far out the plain pavement of the corner runs: past the crossing, where
/// planting and the median take over.
fn clear_far(a: &Arm, l: &ArmLayout) -> f64 {
    let crossing = a.crossing.map_or(0.0, |c| (c.setback_mm + c.width_mm) as f64);
    (l.mouth + crossing + 1_500.0).max(l.mouth + WEDGE_MM)
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
    let name = arm_name(a);

    let (lo, hi) = range(0.0, lane_w);
    let bus = a.bus_lane.then(|| poly(&strip(l.bearing, lo, hi, l.mouth, len)));

    let queue_len = a.approach_mm as f64;
    let queue_pts = match a.approach {
        Q_OFFSET => {
            let (lo, hi) = range(park, park + lane_w);
            if park == 0.0 {
                problems.push(format!("{name}: an offset queue jump needs parking beside the curb to sit beside"));
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
            problems.push(format!("{name}: a virtual queue jump needs a traffic signal"));
        }
        let (lo, hi) = range(0.0, lane_w);
        strip(l.bearing, lo, hi, stop_t + 2_000.0, stop_t + 5_000.0)
    });
    let virtual_loop = loop_pts.as_ref().map(|p| poly(p));
    let gate = matches!(a.approach, GATE_SIGNAL | GATE_YIELD).then(|| {
        if !a.bus_lane {
            problems.push(format!("{name}: a bus gate lets a bus lane through, so it needs a bus lane"));
        }
        if l.prof.enter_x.len() < 2 && a.bus_lane {
            problems.push(format!("{name}: a bus gate needs a traffic lane to hold back"));
        }
        let t = reach(stop_t + queue_len);
        [at(l.bearing, l.lat(off, l.prof.enter_span.0), t), at(l.bearing, l.lat(off, l.prof.enter_span.1), t)]
    });

    let (stop_poly, stop_at) = match a.stop {
        STOP_BULB => {
            if park == 0.0 {
                problems.push(format!("{name}: a bus bulb extends the curb into parking, and there is none"));
            }
            let w = if park > 0.0 { park } else { 2_400.0 };
            let (lo, hi) = range(0.0, w);
            (Some(poly(&strip(l.bearing, lo, hi, far + 3_000.0, far + 11_000.0))), Some(at(l.bearing, (lo + hi) / 2.0, far + 7_000.0)))
        }
        STOP_PLATFORM => {
            if s.control != SIGNAL {
                problems.push(format!("{name}: a signal-protected platform needs a traffic signal"));
            }
            if !has_crossing {
                problems.push(format!("{name}: a platform in the street needs a crossing to reach it"));
            }
            let (lo, hi) = range(lane_w, lane_w + 2_500.0);
            (Some(poly(&strip(l.bearing, lo, hi, far + 3_000.0, far + 11_000.0))), Some(at(l.bearing, (lo + hi) / 2.0, far + 7_000.0)))
        }
        _ => (None, None),
    };

    let mid = (l.cl + l.cr) / 2.0;
    let bollards = if a.filter {
        if !a.bus_lane {
            problems.push(format!("{name}: a modal filter needs a bus lane for the buses that pass"));
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

fn checks(s: &State, arms: &[ArmView], corners: &[CornerView], moves: &[MoveView], lay: &Layout) -> Vec<Check> {
    let name = |uid: u32| arms.iter().find(|a| a.uid == uid).map_or(String::new(), |a| a.label.clone());
    let mut out = Vec::new();

    // Every allowed turn has a lane that serves it.
    let mut no_lane = Vec::new();
    for m in moves.iter().filter(|m| m.allowed && !m.lane) {
        no_lane.push(format!("{} to {} ({} turn)", name(m.from), name(m.to), class_word(m.class)));
    }
    no_lane.sort();
    no_lane.dedup();
    out.push(Check {
        id: "lanes-cover",
        ok: no_lane.is_empty(),
        amount_mm: 0,
        label: "Lanes cover every turn",
        detail: if no_lane.is_empty() { "Each allowed turn has a lane".into() } else { format!("No lane for {}", no_lane.join("; ")) },
    });

    // Every lane points at a turn that is allowed.
    let mut dead = Vec::new();
    for a in arms {
        for (i, l) in a.lanes.iter().enumerate() {
            if l.bad {
                dead.push(format!("{} lane {}", a.label, i + 1));
            }
        }
    }
    out.push(Check {
        id: "lanes-follow",
        ok: dead.is_empty(),
        amount_mm: 0,
        label: "Lanes follow the turn bans",
        detail: if dead.is_empty() { "No lane points at a banned turn".into() } else { format!("Points at banned turns: {}", dead.join("; ")) },
    });

    // Crossing distance.
    let longest = arms.iter().filter_map(|a| a.crossing.as_ref().map(|c| (c, a))).max_by_key(|(c, _)| c.stage_mm);
    out.push(match longest {
        None => Check { id: "crossing", ok: true, amount_mm: 0, label: "Crossing distance", detail: "No crossings marked".into() },
        Some((c, a)) => {
            let d = c.stage_mm;
            let detail = if d <= MAX_STAGE_MM {
                "Longest crossing in one go".into()
            } else if a.can_island && !c.island && (c.distance_mm - ISLAND_MM) / 2 <= MAX_STAGE_MM {
                // The island is the editor's to suggest, not to place: where it would bring each stage within reach, say so.
                format!("{} is too far to cross in one go. A refuge island in the middle would split it in two", a.label)
            } else {
                format!("{} is too far to cross in one go", a.label)
            };
            Check { id: "crossing", ok: d <= MAX_STAGE_MM, amount_mm: d, label: "Crossing distance", detail }
        }
    });

    // Turning speed across a marked crossing.
    let fast: Vec<String> = corners.iter().filter(|c| c.fast).map(|c| name(c.uid)).collect();
    out.push(Check {
        id: "turning-speed",
        ok: fast.is_empty(),
        amount_mm: 0,
        label: "Slow turns at crossings",
        detail: if fast.is_empty() { "Turns are slow enough beside every crossing".into() } else { format!("Fast corner after {}", fast.join("; ")) },
    });

    // Sidewalk left at the corner.
    let thin: Vec<String> = corners.iter().filter(|c| !c.ok).map(|c| name(c.uid)).collect();
    out.push(Check {
        id: "corner-room",
        ok: thin.is_empty(),
        amount_mm: 0,
        label: "Sidewalk survives the corner",
        detail: if thin.is_empty() { "Every corner leaves room to stand".into() } else { format!("Corner after {} eats the sidewalk", thin.join("; ")) },
    });

    // Transit priority measures have what they need.
    let problems: Vec<&str> = arms.iter().flat_map(|a| a.transit.problems.iter().map(String::as_str)).collect();
    out.push(Check {
        id: "transit",
        ok: problems.is_empty(),
        amount_mm: 0,
        label: "Transit measures work",
        detail: if problems.is_empty() { "Each measure has what it needs".into() } else { problems.join("; ") },
    });

    // Signals.
    let signal_ok = s.control != SIGNAL || s.arms.len() <= MAX_SIGNAL_ARMS;
    out.push(Check {
        id: "signal",
        ok: signal_ok,
        amount_mm: 0,
        label: "Signal has room",
        detail: if signal_ok { "Few enough streets for a signal".into() } else { format!("{} streets is too many for one signal", s.arms.len()) },
    });
    let _ = lay;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(check.detail, "Sample Avenue 2 (north) is too far to cross in one go. A refuge island in the middle would split it in two");
        // With the island on, there is nothing to suggest.
        assert!(j.set_island(n, true));
        let v = j.view();
        let check = v.checks.iter().find(|c| c.id == "crossing").unwrap();
        assert!(check.ok);
        assert_eq!(check.detail, "Longest crossing in one go");
    }

    #[test]
    fn every_sample_starts_sound() {
        for i in 0..JUNCTION_SAMPLES.len() {
            let v = Junction::new(i).view();
            let failing: Vec<_> = v.checks.iter().filter(|c| !c.ok).map(|c| format!("{}: {}", c.id, c.detail)).collect();
            assert!(failing.is_empty(), "{}: {failing:?}", JUNCTION_SAMPLES[i].name);
        }
    }

    #[test]
    fn a_bus_lane_across_the_middle_is_drawn_between_its_streets_and_crosses_the_ring_twice() {
        let mut j = Junction::new(0);
        j.set_control(ROUNDABOUT);
        let opts = j.view().bus_options;
        assert_eq!(opts.len(), 6);
        assert_eq!(
            opts[0].label.contains("north") && opts[0].label.contains("south") || opts[0].label.contains("east") && opts[0].label.contains("west"),
            true,
            "straightest first"
        );
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
        assert!(transit(&j).ok, "{}", transit(&j).detail);
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
        let j = Junction::from_city("Test", &state, &state, 0).unwrap();
        assert!(!j.view().can_remove);
    }

    #[test]
    fn a_curb_bulge_is_offered_only_beside_parking() {
        let j = Junction::new(1); // Sample Street 1 has parking both sides; Sample Lane 3 on one
        let v = j.view();
        let by = |sample: usize| v.arms.iter().find(|a| a.street == crate::shared::catalogue::SAMPLES[sample].name).unwrap().can_bulb;
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
}
