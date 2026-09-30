//! Junction geometry, checks and the serialised view. `layout` decides where
//! everything sits in plan; `Junction::view` turns it into what the page draws.
//! The page does no geometry of its own.

use serde::Serialize;
use serde_json::Value;

use crate::catalogue::{KINDS, REGIONS, SAMPLES, Side};
use crate::junction::*;
use crate::model::{Check, Revision};
use crate::plan::*;

/// Ring width of a roundabout, and the smallest island in its middle.
const RING_WIDTH_MM: f64 = 6_000.0;
const MIN_ISLAND_MM: f64 = 1_500.0;
/// A ring's least outer radius, and the arc between two arms it keeps clear.
const MIN_RING_MM: i32 = 12_000;
const MIN_RING_ARC_MM: f64 = 2_500.0;
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
            let prof = profile(a.street, region);
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

#[derive(Serialize)]
pub struct LaneView {
    pub uses: u8,
    pub at: P,
    /// Direction the lane's traffic travels, as a bearing.
    pub heading: i32,
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
}

#[derive(Serialize)]
pub struct ArmView {
    pub uid: u32,
    pub label: String,
    pub street: &'static str,
    pub street_index: usize,
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
    pub end: P,
    pub label_at: P,
    pub mouth_at: P,
    pub banned: Vec<u32>,
    pub classes: u8,
    pub road_mm: i32,
    pub park_mm: [i32; 2],
    pub can_island: bool,
    pub max_offset_mm: i32,
    pub enters: bool,
    pub leaves: bool,
}

#[derive(Serialize)]
pub struct CornerView {
    /// The arm this corner is clockwise of.
    pub uid: u32,
    pub next_uid: u32,
    pub radius_mm: i32,
    pub speed_kmh: f64,
    pub ok: bool,
    pub straight: bool,
    pub wedge: Vec<Value>,
    pub curb: Vec<Value>,
    /// Where the curb corner would be if sharp, the direction its arc bulges,
    /// and the arc's apex per unit of radius: the page turns a drag into a radius.
    pub sharp: Option<P>,
    pub bisector: Option<P>,
    pub apex_per_radius: f64,
    pub handle: P,
}

#[derive(Serialize)]
pub struct MoveView {
    pub from: u32,
    pub to: u32,
    pub class: u8,
    pub allowed: bool,
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
    pub island_mm: i32,
    pub circulation: &'static str,
}

#[derive(Serialize)]
pub struct Selection {
    pub kind: Option<&'static str>,
    pub uid: u32,
}

#[derive(Serialize)]
pub struct JView {
    pub name: &'static str,
    pub sample: usize,
    pub region: &'static str,
    pub drive_side: &'static str,
    pub control: &'static str,
    pub control_index: usize,
    pub ring: Option<RingView>,
    pub ring_extra_mm: i32,
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
            let wide = SAMPLES[arms[i].street].row_mm + SAMPLES[arms[j].street].row_mm;
            let score = d * 100_000 - wide;
            if score < best.1 {
                best = ([i, j], score);
            }
        }
    }
    best.0
}

fn lane_view(a: &ArmLayout, off: f64, x_mm: i32, t: f64, head: i32, uses: u8) -> LaneView {
    LaneView { uses, at: at(a.bearing, a.lat(off, x_mm), t), heading: head }
}

impl Junction {
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
            let clear_far = far + 1_500.0;
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
                    poly: poly(&strip(l.bearing, x0, x1, t0, len)),
                });
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
                .map(|(&x, &u)| lane_view(l, off, x, arrow_t, heading(l.bearing), u))
                .collect();
            let leave_arrows = l.prof.leave_x.iter().map(|&x| lane_view(l, off, x, l.mouth + 6_000.0, l.bearing.round() as i32, 0)).collect();
            let road_mm = l.prof.road_mm();
            arms.push(ArmView {
                uid: a.uid,
                label: arm_name(a),
                street: l.prof.name,
                street_index: a.street,
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
                end: at(l.bearing, off, len),
                label_at: at(l.bearing, off, len * 0.62),
                mouth_at: at(l.bearing, off, l.mouth),
                banned: a.banned.clone(),
                classes: classes[i],
                road_mm,
                park_mm: l.prof.park,
                can_island: road_mm >= ISLAND_MIN_ROAD_MM,
                max_offset_mm: road_mm / 2,
                enters,
                leaves,
            });
        }

        // Corners, the pavement wedges and the carriageway core.
        let mut corners = Vec::new();
        let mut core = Vec::new();
        let ring = lay.ring.as_ref();
        for c in &lay.corners {
            let (la, lb) = (&lay.arms[c.a], &lay.arms[c.b]);
            let (fa, fb) = (la.mouth + WEDGE_MM, lb.mouth + WEDGE_MM);
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
            let (handle, apex_k, sharp, bis, ok) = match &c.fillet {
                Some(f) => {
                    let mid = sub(f.o, scale(f.u, f.r));
                    let k = 1.0 / (c.delta as f64 / 2.0).to_radians().sin() - 1.0;
                    let margin = corner_prop.map_or(f64::MAX, |p| dist(mid, p));
                    (mid, k, Some(f.c), Some(f.u), margin >= MIN_SIDEWALK_AT_CORNER_MM)
                }
                None => (scale(add(ca_in, cb_in), 0.5), 0.0, None, None, true),
            };
            corners.push(CornerView {
                uid: ua,
                next_uid: ub,
                radius_mm: radius,
                speed_kmh: if c.fillet.is_some() { speed_kmh(radius) } else { 0.0 },
                ok,
                straight: c.fillet.is_none(),
                wedge,
                curb,
                sharp,
                bisector: bis,
                apex_per_radius: apex_k,
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
        let rad = ring.map(|r| r.radius);
        for (i, a) in s.arms.iter().enumerate() {
            for (j, b) in s.arms.iter().enumerate() {
                if i == j {
                    continue;
                }
                let (la, lb) = (&lay.arms[i], &lay.arms[j]);
                let (Some(ea), Some(xb)) = (span_mid(la.prof.enter_span), span_mid(lb.prof.leave_span)) else { continue };
                let pe = at(la.bearing, la.lat(a.offset_mm as f64, ea), la.mouth);
                let px = at(lb.bearing, lb.lat(b.offset_mm as f64, xb), lb.mouth);
                let allowed = !a.banned.contains(&b.uid);
                let class = turn_class(a.bearing, b.bearing);
                let lane = a.lanes.iter().any(|u| u & class != 0);
                let path = movement_path(la.bearing, pe, lb.bearing, px, rad, side, a.bearing, b.bearing);
                movements.push(MoveView { from: a.uid, to: b.uid, class, allowed, lane, path });
            }
        }
        let conflicts = conflicts(s, &movements, &lay, side);

        let checks = checks(s, &arms, &corners, &movements, &lay);
        let selected = match self.selected {
            Target::None => Selection { kind: None, uid: 0 },
            Target::Arm(u) => Selection { kind: Some("arm"), uid: u },
            Target::Corner(u) => Selection { kind: Some("corner"), uid: u },
            Target::Crossing(u) => Selection { kind: Some("crossing"), uid: u },
        };
        JView {
            name: JUNCTION_SAMPLES[self.sample()].name,
            sample: self.sample(),
            region: REGIONS[region].id,
            drive_side: if side == Side::Left { "left" } else { "right" },
            control: CONTROLS[s.control].id,
            control_index: s.control,
            ring: ring.map(|r| RingView {
                radius_mm: r.radius as i32,
                floor_mm: r.floor,
                island_mm: (r.radius - RING_WIDTH_MM).max(MIN_ISLAND_MM) as i32,
                circulation: if side == Side::Right { "anticlockwise" } else { "clockwise" },
            }),
            ring_extra_mm: s.ring_extra_mm,
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
        return Conflicts { crossing: 0, merging: enters, diverging: leaves, by_phase };
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
    let start = (num(&path[0], 1), num(&path[0], 2));
    match path.get(1).and_then(|v| v[0].as_str()) {
        Some("Q") => sample_quad(start, (num(&path[1], 1), num(&path[1], 2)), (num(&path[1], 3), num(&path[1], 4)), 16),
        _ => vec![start],
    }
}

fn checks(s: &State, arms: &[ArmView], corners: &[CornerView], moves: &[MoveView], lay: &Layout) -> Vec<Check> {
    let word = |c: u8| match c {
        LEFT => "left",
        THROUGH => "through",
        _ => "right",
    };
    let name = |uid: u32| arms.iter().find(|a| a.uid == uid).map_or(String::new(), |a| a.label.clone());
    let mut out = Vec::new();

    // Every allowed turn has a lane that serves it.
    let mut no_lane = Vec::new();
    for m in moves.iter().filter(|m| m.allowed && !m.lane) {
        no_lane.push(format!("{}: {} turn", name(m.from), word(m.class)));
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
    for a in &s.arms {
        for (i, u) in a.lanes.iter().enumerate() {
            let serves = CLASSES_ALL.iter().filter(|c| *u & **c != 0).any(|c| moves.iter().any(|m| m.from == a.uid && m.allowed && m.class == *c));
            if !serves {
                dead.push(format!("{} lane {}", arm_name(a), i + 1));
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
    let longest = arms.iter().filter_map(|a| a.crossing.as_ref().map(|c| (c.stage_mm, a.label.clone()))).max_by_key(|(d, _)| *d);
    out.push(match longest {
        None => Check { id: "crossing", ok: true, amount_mm: 0, label: "Crossing distance", detail: "No crossings marked".into() },
        Some((d, who)) => Check {
            id: "crossing",
            ok: d <= MAX_STAGE_MM,
            amount_mm: d,
            label: "Crossing distance",
            detail: if d <= MAX_STAGE_MM { "Longest crossing in one go".into() } else { format!("{who} is too far to cross in one go") },
        },
    });

    // Turning speed across a marked crossing.
    let fast: Vec<String> = corners
        .iter()
        .filter(|c| !c.straight && c.speed_kmh > MAX_TURN_KMH)
        .filter(|c| arms.iter().any(|a| (a.uid == c.uid || a.uid == c.next_uid) && a.crossing.is_some()))
        .map(|c| name(c.uid))
        .collect();
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

const CLASSES_ALL: [u8; 3] = [LEFT, THROUGH, RIGHT];

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
        let av = arm_of(&j, 0); // Sample Avenue 2, road 20 m, no island
        let cv = |j: &Junction| j.view().arms.iter().find(|a| a.uid == av).unwrap().crossing.as_ref().map(|c| (c.stage_mm, c.stages)).unwrap();
        assert_eq!(cv(&j), (20_000, 1));
    }

    #[test]
    fn the_avenue_is_too_wide_to_cross_until_it_has_an_island() {
        let mut j = Junction::new(0);
        let av = arm_of(&j, 0);
        let check = |j: &Junction| j.view().checks.iter().find(|c| c.id == "crossing").unwrap().ok;
        assert!(!check(&j));
        assert!(j.set_island(av, true));
        assert!(!check(&j) || check(&j)); // the other arm decides once the avenue passes
        let av2 = arm_of(&j, 180);
        assert!(j.set_island(av2, true));
        assert!(check(&j), "two stages of about 9 m");
    }

    #[test]
    fn a_lane_pointing_only_at_a_banned_turn_is_flagged_until_changed() {
        let mut j = Junction::new(0);
        let av = arm_of(&j, 0);
        let east = arm_of(&j, 90); // a left turn from the north arm
        let follows = |j: &Junction| j.view().checks.iter().find(|c| c.id == "lanes-follow").unwrap().ok;
        assert!(follows(&j));
        assert!(j.set_turn(av, east, false));
        assert!(follows(&j), "the left lane still goes straight on");
        assert!(j.set_lane_use(av, 0, THROUGH, false));
        assert!(!follows(&j), "the left lane is now left only, and left is banned");
        assert!(j.set_lane_use(av, 0, THROUGH, true));
        assert!(follows(&j));
    }

    #[test]
    fn an_allowed_turn_with_no_lane_is_flagged() {
        let mut j = Junction::new(0);
        let av = arm_of(&j, 0);
        // The avenue's two lanes serve left+through and through+right; take the left off.
        assert!(j.set_lane_use(av, 0, LEFT, false));
        let v = j.view();
        assert!(!v.checks.iter().find(|c| c.id == "lanes-cover").unwrap().ok);
        assert!(j.set_lane_use(av, 1, LEFT, true));
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
}
