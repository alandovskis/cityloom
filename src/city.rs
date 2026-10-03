//! The city: a network of junctions joined by streets, and what the street and
//! junction editors keep in it. Pure Rust like the other models.
//!
//! The network itself (where the junctions are and which street joins which)
//! is fixed. What a resident can change is each street's section and each
//! junction's plan, which the city holds as snapshots and hands to the editors.
//! A junction reads the streets it meets from the city, so an edit to a street
//! shows in the junctions at its ends. Everything here is a synthetic
//! placeholder: the layout, the names and the widths.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::shared::catalogue::{KINDS, REGIONS, SAMPLES, Side};
use crate::junction::{self, ALL_WAY_STOP, Arm, Junction, PRIORITY, SIGNAL, State};
use crate::model::{Editor, Street};

/// Bump when what is saved changes shape; an older save is then left behind.
const SAVE_VERSION: u32 = 1;

/// Streets wider than this get a refuge island in their crossings to start with.
const ISLAND_ROW_MM: i32 = 24_000;

struct NodeDef {
    x_m: i32,
    y_m: i32,
    /// Where three to five streets meet. Otherwise the street runs off the map.
    junction: bool,
    control: usize,
    corner_mm: i32,
}

const fn junction_at(x_m: i32, y_m: i32, control: usize, corner_mm: i32) -> NodeDef {
    NodeDef { x_m, y_m, junction: true, control, corner_mm }
}

const fn gate_at(x_m: i32, y_m: i32) -> NodeDef {
    NodeDef { x_m, y_m, junction: false, control: 0, corner_mm: 0 }
}

struct EdgeDef {
    /// Node indices. The street editor shows the street looking from `a` to `b`.
    a: usize,
    b: usize,
    /// Index into `SAMPLES`.
    street: usize,
}

const fn street(a: usize, b: usize, street: usize) -> EdgeDef {
    EdgeDef { a, b, street }
}

const STREET: usize = 0;
const AVENUE: usize = 1;
const LANE: usize = 2;
const FREEWAY: usize = 3;

#[rustfmt::skip]
const NODES: [NodeDef; 21] = [
    gate_at(20, 340),                           //  0 avenue, west
    junction_at(230, 340, SIGNAL, 6_000),       //  1
    junction_at(230, 160, PRIORITY, 3_000),     //  2
    gate_at(20, 160),                           //  3 lane, west
    junction_at(460, 160, ALL_WAY_STOP, 4_000), //  4
    gate_at(460, 20),                           //  5 street, north
    junction_at(460, 340, SIGNAL, 6_000),       //  6
    junction_at(700, 160, PRIORITY, 4_000),     //  7
    gate_at(900, 160),                          //  8 lane, east
    junction_at(700, 340, SIGNAL, 6_000),       //  9
    gate_at(900, 340),                          // 10 avenue, east
    junction_at(460, 500, ALL_WAY_STOP, 4_000), // 11
    gate_at(460, 580),                          // 12 street, south
    junction_at(700, 500, PRIORITY, 4_000),     // 13
    gate_at(900, 500),                          // 14 lane, east
    junction_at(230, 500, ALL_WAY_STOP, 2_000), // 15 five ways
    gate_at(20, 500),                           // 16 lane, west
    gate_at(180, 590),                          // 17 lane, south-west
    gate_at(280, 590),                          // 18 lane, south-east
    gate_at(20, 660),                           // 19 freeway, west
    gate_at(900, 660),                          // 20 freeway, east
];

#[rustfmt::skip]
const EDGES: [EdgeDef; 23] = [
    street(0, 1, AVENUE),   //  1
    street(1, 6, AVENUE),   //  2
    street(6, 9, AVENUE),   //  3
    street(9, 10, AVENUE),  //  4
    street(2, 1, STREET),   //  5
    street(1, 15, STREET),  //  6
    street(3, 2, LANE),     //  7
    street(2, 4, LANE),     //  8
    street(5, 4, STREET),   //  9
    street(4, 6, STREET),   // 10
    street(4, 7, STREET),   // 11
    street(6, 11, STREET),  // 12
    street(7, 8, LANE),     // 13
    street(7, 9, STREET),   // 14
    street(9, 13, STREET),  // 15
    street(11, 12, STREET), // 16
    street(15, 11, LANE),   // 17
    street(11, 13, STREET), // 18
    street(13, 14, LANE),   // 19
    street(16, 15, LANE),   // 20
    street(15, 17, LANE),   // 21
    street(15, 18, LANE),   // 22
    street(19, 20, FREEWAY),// 23
];

/// The map name of the city.
pub const NAME: &str = "Sample city";

/// Node and street uids are their place in the tables above, counting from 1.
fn node_uid(i: usize) -> u32 {
    i as u32 + 1
}

/// What the city keeps between visits.
#[derive(Serialize, Deserialize)]
struct Saved {
    version: u32,
    streets: BTreeMap<u32, Street>,
    junctions: BTreeMap<u32, State>,
}

pub struct City {
    streets: BTreeMap<u32, Street>,
    junctions: BTreeMap<u32, State>,
    today_streets: BTreeMap<u32, Street>,
    today_junctions: BTreeMap<u32, State>,
}

fn dist_mm(a: usize, b: usize) -> f64 {
    let (dx, dy) = ((NODES[a].x_m - NODES[b].x_m) as f64, (NODES[a].y_m - NODES[b].y_m) as f64);
    dx.hypot(dy) * 1000.0
}

/// The bearing from node `from` toward node `to`, clockwise from north (up),
/// to the nearest step the junction editor uses.
fn bearing(from: usize, to: usize) -> i32 {
    let dx = (NODES[to].x_m - NODES[from].x_m) as f64;
    let dy = (NODES[to].y_m - NODES[from].y_m) as f64;
    let deg = dx.atan2(-dy).to_degrees();
    ((deg / junction::BEARING_STEP as f64).round() as i32 * junction::BEARING_STEP).rem_euclid(360)
}

fn edges_at(node: usize) -> Vec<usize> {
    (0..EDGES.len()).filter(|&e| EDGES[e].a == node || EDGES[e].b == node).collect()
}

fn other_end(edge: usize, node: usize) -> usize {
    if EDGES[edge].a == node { EDGES[edge].b } else { EDGES[edge].a }
}

/// Junctions are numbered in reading order, so the names run across the map.
fn junction_number(node: usize) -> usize {
    let mut order: Vec<usize> = (0..NODES.len()).filter(|&n| NODES[n].junction).collect();
    order.sort_by_key(|&n| (NODES[n].y_m, NODES[n].x_m));
    order.iter().position(|&n| n == node).map_or(0, |p| p + 1)
}

fn node_name(node: usize) -> String {
    if NODES[node].junction { format!("Junction {}", junction_number(node)) } else { "Edge of the map".to_string() }
}

fn end_name(node: usize) -> String {
    if NODES[node].junction { format!("Junction {}", junction_number(node)) } else { "the edge of the map".to_string() }
}

fn edge_name(edge: usize) -> String {
    let e = &EDGES[edge];
    let kind = SAMPLES[e.street].name;
    if !NODES[e.a].junction && !NODES[e.b].junction {
        format!("{kind} · through the city")
    } else {
        format!("{kind} · {} to {}", end_name(e.a), end_name(e.b))
    }
}

/// The street an arm reads: the city's street as seen looking out from `node`.
fn seen_from(edge: usize, node: usize, street: &Street) -> Street {
    if EDGES[edge].a == node { street.clone() } else { street.reversed() }
}

/// A junction as first laid out, from the streets that meet there.
fn generate(node: usize, streets: &BTreeMap<u32, Street>) -> State {
    let def = &NODES[node];
    let mut incident: Vec<(i32, usize)> = edges_at(node).into_iter().map(|e| (bearing(node, other_end(e, node)), e)).collect();
    incident.sort_by_key(|(b, _)| *b);
    let mut arms: Vec<Arm> = incident
        .iter()
        .enumerate()
        .map(|(i, &(b, e))| {
            let mut arm = Arm::new(i as u32 + 1, EDGES[e].street, b, 0);
            arm.edge = e as u32 + 1;
            arm.corner_mm = def.corner_mm;
            arm.section = streets.get(&arm.edge).map(|s| seen_from(e, node, s));
            if arm.row_mm() >= ISLAND_ROW_MM
                && let Some(c) = arm.crossing.as_mut()
            {
                c.island = true;
            }
            arm
        })
        .collect();
    junction::normalize(&mut arms, 0);
    tune_corners(&mut arms, def.control, def.corner_mm);
    let mut s = State { label: "Junction today".into(), arms, control: def.control, ring_extra_mm: 0, bus: None, cycle: None };
    forget_streets(&mut s);
    s
}

/// Gives each corner the radius nearest `preferred` that leaves its sidewalk
/// and keeps its turn slow. Streets differ in the pavement they have beside
/// the curb, so one radius does not suit every corner of a junction. A corner
/// nothing suits keeps `preferred`.
fn tune_corners(arms: &mut [Arm], control: usize, preferred: i32) {
    use junction::{MAX_CORNER_MM, MIN_CORNER_MM, RING_STEP_MM};
    let state = |arms: &[Arm]| State { label: String::new(), arms: arms.to_vec(), control, ring_extra_mm: 0, bus: None, cycle: None };
    let mut radii: Vec<i32> = (MIN_CORNER_MM..=MAX_CORNER_MM).step_by(RING_STEP_MM as usize).collect();
    radii.sort_by_key(|r| ((r - preferred).abs(), *r));
    for i in 0..arms.len() {
        let uid = arms[i].uid;
        let found = radii.iter().any(|&r| {
            arms[i].corner_mm = r;
            let s = state(arms);
            Junction::from_city("", &s, &s, 0)
                .is_some_and(|j| j.view().corners.iter().any(|c| c.uid == uid && c.ok && !c.fast))
        });
        if !found {
            arms[i].corner_mm = preferred;
        }
    }
}

fn forget_streets(s: &mut State) {
    for a in &mut s.arms {
        a.section = None;
    }
}

/// Whether a saved junction can stand for this node: the same streets, with
/// every index and length inside what the editor allows.
fn junction_fits(s: &State, node: usize) -> bool {
    use junction::*;
    let mut want: Vec<u32> = edges_at(node).iter().map(|&e| e as u32 + 1).collect();
    let mut have: Vec<u32> = s.arms.iter().map(|a| a.edge).collect();
    want.sort_unstable();
    have.sort_unstable();
    let mut uids: Vec<u32> = s.arms.iter().map(|a| a.uid).collect();
    uids.sort_unstable();
    uids.dedup();
    want == have
        && uids.len() == s.arms.len()
        && uids.first().is_some_and(|&u| u != 0)
        && s.control < CONTROLS.len()
        && (0..=MAX_RING_MM).contains(&s.ring_extra_mm)
        && s.cycle.is_none_or(|c| (CYCLE_MIN_MM..=CYCLE_MAX_MM).contains(&c))
        && s.arms.iter().all(|a| {
            a.street < SAMPLES.len()
                && a.bearing % BEARING_STEP == 0
                && (0..360).contains(&a.bearing)
                && (MIN_CORNER_MM..=MAX_CORNER_MM).contains(&a.corner_mm)
                && a.offset_mm.abs() <= 30_000
                && a.approach < APPROACHES.len()
                && (APPROACH_MIN_MM..=APPROACH_MAX_MM).contains(&a.approach_mm)
                && a.stop < STOPS.len()
                && a.rule < RULES.len()
                && a.crossing.is_none_or(|c| {
                    (MIN_SETBACK_MM..=MAX_SETBACK_MM).contains(&c.setback_mm) && (MIN_CROSSING_MM..=MAX_CROSSING_MM).contains(&c.width_mm)
                })
        })
}

fn same_junction(a: &State, b: &State) -> bool {
    State { label: String::new(), ..a.clone() } == State { label: String::new(), ..b.clone() }
}

impl Default for City {
    fn default() -> Self {
        City::new()
    }
}

impl City {
    /// The city as first laid out: every street a sample, every junction
    /// generated from the streets that meet there.
    pub fn new() -> City {
        let today_streets: BTreeMap<u32, Street> =
            EDGES.iter().enumerate().map(|(i, e)| (i as u32 + 1, Street::sample(e.street, Side::Right))).collect();
        let today_junctions: BTreeMap<u32, State> =
            (0..NODES.len()).filter(|&n| NODES[n].junction).map(|n| (node_uid(n), generate(n, &today_streets))).collect();
        City { streets: today_streets.clone(), junctions: today_junctions.clone(), today_streets, today_junctions }
    }

    /// The city as saved, or as first laid out for any part of the save that
    /// cannot be used. Not an error: a stale or damaged save just starts over.
    pub fn load(json: &str) -> City {
        let mut city = City::new();
        let Ok(saved) = serde_json::from_str::<Saved>(json) else { return city };
        if saved.version != SAVE_VERSION {
            return city;
        }
        for (uid, street) in saved.streets {
            let ok = city.today_streets.get(&uid).is_some_and(|t| t.sample == street.sample && t.row_mm == street.row_mm);
            if ok && street.is_sound() {
                city.streets.insert(uid, street);
            }
        }
        for (uid, state) in saved.junctions {
            let node = (uid as usize).checked_sub(1).filter(|&n| n < NODES.len() && NODES[n].junction);
            if node.is_some_and(|n| junction_fits(&state, n)) {
                city.junctions.insert(uid, state);
            }
        }
        city
    }

    pub fn save(&self) -> String {
        let saved = Saved { version: SAVE_VERSION, streets: self.streets.clone(), junctions: self.junctions.clone() };
        serde_json::to_string(&saved).expect("the city serialises")
    }

    /// Puts every street and junction back as first laid out.
    pub fn reset(&mut self) {
        self.streets = self.today_streets.clone();
        self.junctions = self.today_junctions.clone();
    }

    fn edge_index(uid: u32) -> Option<usize> {
        (uid as usize).checked_sub(1).filter(|&e| e < EDGES.len())
    }

    fn node_index(uid: u32) -> Option<usize> {
        (uid as usize).checked_sub(1).filter(|&n| n < NODES.len())
    }

    pub fn street_name(&self, edge: u32) -> Option<String> {
        Self::edge_index(edge).map(edge_name)
    }

    pub fn junction_name(&self, node: u32) -> Option<String> {
        Self::node_index(node).filter(|&n| NODES[n].junction).map(node_name)
    }

    /// The two ends of a street, first the one the street editor looks from.
    pub fn street_ends(&self, edge: u32) -> Vec<EndView> {
        let Some(e) = Self::edge_index(edge) else { return Vec::new() };
        [EDGES[e].a, EDGES[e].b]
            .into_iter()
            .map(|n| EndView { uid: node_uid(n), name: end_name(n), junction: NODES[n].junction })
            .collect()
    }

    /// The street editor on one street of the city.
    pub fn street_editor(&self, edge: u32, region: usize) -> Option<Editor> {
        Self::edge_index(edge)?;
        Some(Editor::from_street(self.today_streets.get(&edge)?, self.streets.get(&edge)?, region))
    }

    /// Keeps what the street editor has made of a street.
    pub fn keep_street(&mut self, edge: u32, street: Street) -> bool {
        let ok = self.today_streets.get(&edge).is_some_and(|t| t.sample == street.sample && t.row_mm == street.row_mm);
        if ok && street.is_sound() {
            self.streets.insert(edge, street);
        }
        ok
    }

    fn with_streets(&self, node: usize, s: &State) -> State {
        let mut s = s.clone();
        for a in &mut s.arms {
            let edge = a.edge;
            a.section = Self::edge_index(edge).and_then(|e| self.streets.get(&edge).map(|st| seen_from(e, node, st)));
        }
        s
    }

    /// The junction editor on one junction of the city, reading the streets as
    /// they now stand. None when it cannot be drawn with them.
    pub fn junction_editor(&self, node: u32, region: usize) -> Option<Junction> {
        let n = Self::node_index(node).filter(|&n| NODES[n].junction)?;
        let today = self.with_streets(n, self.today_junctions.get(&node)?);
        let now = self.with_streets(n, self.junctions.get(&node)?);
        Junction::from_city(&node_name(n), &today, &now, region)
    }

    /// Keeps what the junction editor has made of a junction.
    pub fn keep_junction(&mut self, node: u32, state: State) -> bool {
        let fits = Self::node_index(node).filter(|&n| NODES[n].junction).is_some_and(|n| junction_fits(&state, n));
        if fits {
            self.junctions.insert(node, state);
        }
        fits
    }

    // ---- the map ----------------------------------------------------------

    /// Everything the map draws, and the state of each place.
    pub fn view(&self, region: usize) -> CityView {
        let region = region.min(REGIONS.len() - 1);
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let trim_at = |n: usize| -> i32 {
            if !NODES[n].junction {
                return 0;
            }
            edges_at(n).iter().map(|&e| SAMPLES[EDGES[e].street].row_mm / 2).max().unwrap_or(0)
        };
        for (i, e) in EDGES.iter().enumerate() {
            let uid = i as u32 + 1;
            let today = &self.today_streets[&uid];
            let now = &self.streets[&uid];
            let editor = Editor::from_street(today, now, region);
            let v = editor.view();
            let failing: Vec<String> = v.checks.iter().filter(|c| !c.ok).map(|c| c.label.to_string()).collect();
            let row = v.row_mm;
            edges.push(EdgeView {
                uid,
                a: node_uid(e.a),
                b: node_uid(e.b),
                name: edge_name(i),
                kind: SAMPLES[e.street].name,
                row_mm: row,
                total_mm: v.total_mm,
                length_mm: dist_mm(e.a, e.b).round() as i32,
                trim_a_mm: trim_at(e.a),
                trim_b_mm: trim_at(e.b),
                freeway: SAMPLES[e.street].freeway,
                pieces: v
                    .segments
                    .iter()
                    .map(|s| PieceView { kind: KINDS[s.kind].id, offset_mm: s.x_mm + s.width_mm / 2 - row / 2, width_mm: s.width_mm })
                    .collect(),
                edited: now != today,
                ok: failing.is_empty(),
                failing,
            });
        }
        for (i, def) in NODES.iter().enumerate() {
            let uid = node_uid(i);
            let mut v = NodeView {
                uid,
                name: node_name(i),
                x_mm: def.x_m * 1000,
                y_mm: def.y_m * 1000,
                junction: def.junction,
                radius_mm: trim_at(i),
                control: None,
                arms: 0,
                edited: false,
                ok: true,
                failing: Vec::new(),
            };
            if def.junction {
                let now = &self.junctions[&uid];
                v.edited = !same_junction(now, &self.today_junctions[&uid]);
                v.arms = now.arms.len();
                v.control = Some(junction::CONTROLS[now.control].name);
                match self.junction_editor(uid, region) {
                    Some(j) => {
                        v.failing = j.view().checks.iter().filter(|c| !c.ok).map(|c| c.label.to_string()).collect();
                        v.ok = v.failing.is_empty();
                    }
                    None => {
                        v.ok = false;
                        v.failing = vec!["Cannot be drawn with the streets as they are".to_string()];
                    }
                }
            }
            nodes.push(v);
        }
        let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for n in &nodes {
            (x0, y0, x1, y1) = (x0.min(n.x_mm), y0.min(n.y_mm), x1.max(n.x_mm), y1.max(n.y_mm));
        }
        let places = nodes.iter().filter(|n| n.junction).count() + edges.len();
        let failing = nodes.iter().filter(|n| !n.ok).count() + edges.iter().filter(|e| !e.ok).count();
        let edited = nodes.iter().filter(|n| n.edited).count() + edges.iter().filter(|e| e.edited).count();
        CityView { name: NAME, nodes, edges, bounds_mm: [x0, y0, x1, y1], places, failing, edited }
    }
}

// ---- serialised view ------------------------------------------------------

#[derive(Serialize)]
pub struct PieceView {
    pub kind: &'static str,
    /// Where the piece's middle lies across the street, to the right of its
    /// axis when looking from the street's first end to its second.
    pub offset_mm: i32,
    pub width_mm: i32,
}

/// One end of a street: a junction, or where it leaves the map.
#[derive(Serialize)]
pub struct EndView {
    pub uid: u32,
    pub name: String,
    pub junction: bool,
}

#[derive(Serialize)]
pub struct EdgeView {
    pub uid: u32,
    pub a: u32,
    pub b: u32,
    pub name: String,
    /// The kind of street it began as.
    pub kind: &'static str,
    pub row_mm: i32,
    pub total_mm: i32,
    pub length_mm: i32,
    /// How far in from each end the street's details are left out, because
    /// the junction there is drawn over them.
    pub trim_a_mm: i32,
    pub trim_b_mm: i32,
    pub freeway: bool,
    pub pieces: Vec<PieceView>,
    pub edited: bool,
    pub ok: bool,
    pub failing: Vec<String>,
}

#[derive(Serialize)]
pub struct NodeView {
    pub uid: u32,
    pub name: String,
    pub x_mm: i32,
    pub y_mm: i32,
    /// A junction of streets, or else where a street leaves the map.
    pub junction: bool,
    pub radius_mm: i32,
    pub control: Option<&'static str>,
    pub arms: usize,
    pub edited: bool,
    pub ok: bool,
    pub failing: Vec<String>,
}

#[derive(Serialize)]
pub struct CityView {
    pub name: &'static str,
    pub nodes: Vec<NodeView>,
    pub edges: Vec<EdgeView>,
    pub bounds_mm: [i32; 4],
    /// Junctions and streets together.
    pub places: usize,
    pub failing: usize,
    pub edited: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_network_is_well_formed() {
        for (i, e) in EDGES.iter().enumerate() {
            assert_ne!(e.a, e.b, "edge {i}");
            assert!(e.a < NODES.len() && e.b < NODES.len());
        }
        for (n, def) in NODES.iter().enumerate() {
            let degree = edges_at(n).len();
            if def.junction {
                assert!((junction::MIN_ARMS..=junction::MAX_ARMS).contains(&degree), "node {n} has {degree} streets");
            } else {
                assert_eq!(degree, 1, "gate {n}");
            }
        }
        // Reading order: the first junction is the top left one.
        assert_eq!(junction_number(2), 1);
        assert_eq!(junction_number(15), 7);
    }

    #[test]
    fn every_place_in_the_first_city_works() {
        let v = City::new().view(0);
        let bad: Vec<_> = v.nodes.iter().filter(|n| !n.ok).map(|n| (&n.name, &n.failing)).chain(v.edges.iter().filter(|e| !e.ok).map(|e| (&e.name, &e.failing))).collect();
        assert!(bad.is_empty(), "{bad:?}");
        assert_eq!((v.failing, v.edited), (0, 0));
        assert_eq!(v.places, 9 + 23);
    }

    #[test]
    fn the_first_city_works_on_the_other_side_of_the_road_too() {
        let left = REGIONS.iter().position(|r| r.drive_side == Side::Left).unwrap();
        let v = City::new().view(left);
        assert_eq!(v.failing, 0, "{:?}", v.nodes.iter().filter(|n| !n.ok).map(|n| (&n.name, &n.failing)).collect::<Vec<_>>());
    }

    #[test]
    fn a_junction_is_its_streets_seen_from_the_junction() {
        let city = City::new();
        // Junction 4 is node 1: the avenue arrives from the west and leaves to the east.
        let j = city.junction_editor(node_uid(1), 0).unwrap();
        let arms = &j.current().arms;
        assert_eq!(arms.len(), 4);
        assert_eq!(arms.iter().map(|a| a.bearing).collect::<Vec<_>>(), [0, 90, 180, 270]);
        assert!(arms.iter().all(|a| a.edge != 0));
        // Every arm of the sample city brings the lanes its street has.
        for a in arms {
            assert_eq!(a.lanes.len(), a.profile(0).enter_x.len());
        }
    }

    #[test]
    fn a_one_way_street_enters_one_junction_and_leaves_the_other() {
        let mut city = City::new();
        // Edge 2 joins node 1 to node 6. Make every lane run toward whoever looks from node 1.
        let mut s = city.streets[&2].clone();
        for seg in &mut s.segments {
            if seg.direction.is_some() {
                seg.direction = Some(1);
            }
        }
        assert!(city.keep_street(2, s));
        let arm_at = |node: usize| {
            let j = city.junction_editor(node_uid(node), 0).unwrap();
            let a = j.current().arms.iter().find(|a| a.edge == 2).unwrap().clone();
            a.profile(0)
        };
        let (at_a, at_b) = (arm_at(1), arm_at(6));
        assert!(!at_a.enter_x.is_empty() && at_a.leave_x.is_empty());
        assert!(at_b.enter_x.is_empty() && !at_b.leave_x.is_empty());
    }

    #[test]
    fn a_change_to_a_street_shows_in_the_junctions_at_its_ends() {
        let mut city = City::new();
        let mut s = city.streets[&5].clone();
        s.segments.retain(|g| KINDS[g.kind].id != "parking");
        assert!(city.keep_street(5, s));
        let v = city.view(0);
        assert!(v.edges[4].edited && v.edges[4].ok);
        let j = city.junction_editor(node_uid(1), 0).unwrap();
        let arm = j.current().arms.iter().find(|a| a.edge == 5).unwrap();
        assert_eq!(arm.profile(0).park, [0, 0]);
        // Edge 5 joins node 2 to node 1; the other arms still have their parking.
        assert!(j.current().arms.iter().filter(|a| a.edge != 5).any(|a| a.profile(0).park != [0, 0]));
    }

    #[test]
    fn what_the_editors_keep_is_saved_and_comes_back() {
        let mut city = City::new();
        let mut e = city.street_editor(1, 0).unwrap();
        let uid = e.view().segments[2].uid;
        assert!(e.set_width(uid, 2_700));
        assert!(city.keep_street(1, e.snapshot()));
        let mut j = city.junction_editor(node_uid(1), 0).unwrap();
        let arm = j.current().arms[0].uid;
        assert!(j.set_corner(arm, 4_500));
        assert!(city.keep_junction(node_uid(1), j.snapshot()));

        let back = City::load(&city.save());
        assert_eq!(back.streets, city.streets);
        assert_eq!(back.junctions, city.junctions);
        let v = back.view(0);
        assert!(v.edges[0].edited && v.nodes[1].edited);
        assert_eq!(v.edited, 2);
        // The editors reopen with the earlier changes as one revision, and Start over undoes them.
        let mut e = back.street_editor(1, 0).unwrap();
        assert!(e.view().changed);
        assert!(e.reset());
        let mut j = back.junction_editor(node_uid(1), 0).unwrap();
        assert!(j.changed());
        assert!(j.reset());
        let mut city = back;
        city.reset();
        assert_eq!(city.view(0).edited, 0);
    }

    #[test]
    fn a_damaged_or_foreign_save_starts_the_city_over() {
        let fresh = City::new().save();
        for bad in ["", "not json", "{}", "[]", r#"{"version":99,"streets":{},"junctions":{}}"#] {
            assert_eq!(City::load(bad).save(), fresh, "{bad:?}");
        }
        // A street with an index outside the catalogue is dropped; the rest is kept.
        let mut city = City::new();
        let mut s = city.streets[&3].clone();
        s.segments[0].width_mm = 3_000;
        city.keep_street(3, s);
        let mut saved: serde_json::Value = serde_json::from_str(&city.save()).unwrap();
        saved["streets"]["1"]["segments"][0]["kind"] = serde_json::json!(99);
        saved["junctions"]["2"]["control"] = serde_json::json!(99);
        let back = City::load(&saved.to_string());
        assert_eq!(back.streets[&1], back.today_streets[&1]);
        assert_eq!(back.junctions[&2], back.today_junctions[&2]);
        assert_ne!(back.streets[&3], back.today_streets[&3]);
    }

    #[test]
    fn a_save_whose_junction_has_other_streets_is_not_used() {
        let mut city = City::new();
        let mut state = city.junctions[&node_uid(1)].clone();
        state.arms[0].edge = 99;
        assert!(!city.keep_junction(node_uid(1), state));
        assert!(!city.keep_junction(node_uid(0), city.junctions[&node_uid(1)].clone()));
        assert!(!city.keep_junction(999, city.junctions[&node_uid(1)].clone()));
    }

    #[test]
    fn a_street_that_loses_its_room_marks_the_places_it_breaks() {
        let mut city = City::new();
        // Strip a street of everything but a sidewalk: it no longer works, and a
        // junction that meets it either still draws or says it cannot.
        let mut s = city.streets[&2].clone();
        s.segments.retain(|g| KINDS[g.kind].id == "sidewalk");
        assert!(city.keep_street(2, s));
        let v = city.view(0);
        assert!(!v.edges[1].ok);
        assert!(v.failing >= 1);
        for n in v.nodes.iter().filter(|n| n.junction) {
            // Whatever happens, asking for the junction never panics.
            let _ = city.junction_editor(n.uid, 0).map(|j| j.view());
        }
    }

    #[test]
    fn a_street_knows_its_ends() {
        let ends = City::new().street_ends(1);
        assert_eq!(ends.iter().map(|e| (e.name.as_str(), e.junction)).collect::<Vec<_>>(), [("the edge of the map", false), ("Junction 4", true)]);
        assert!(City::new().street_ends(99).is_empty());
    }

    #[test]
    fn names_say_where_a_street_goes() {
        assert_eq!(edge_name(0), "Sample Avenue 2 · the edge of the map to Junction 4");
        assert_eq!(edge_name(22), "Sample Freeway 4 · through the city");
        assert_eq!(node_name(15), "Junction 7");
        assert_eq!(node_name(0), "Edge of the map");
    }
}
