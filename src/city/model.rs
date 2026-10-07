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

use crate::junction::model::{self as junction, Arm, Junction, State};
#[cfg(test)]
use crate::junction::model::{ALL_WAY_STOP, PRIORITY, SIGNAL};
use crate::shared::catalogue::{KINDS, REGIONS, SAMPLES, Side};
use crate::street::model::{Editor, Street};

/// Bump when what is saved changes shape; an older save is then left behind.
const SAVE_VERSION: u32 = 1;

pub(super) struct NodeDef {
    pub(super) x_mm: i32,
    pub(super) y_mm: i32,
    /// Where three to five streets meet. Otherwise the street runs off the map.
    pub(super) junction: bool,
    pub(super) control: usize,
    pub(super) corner_mm: i32,
}

#[cfg(test)]
const fn junction_at(x_m: i32, y_m: i32, control: usize, corner_mm: i32) -> NodeDef {
    NodeDef { x_mm: x_m * 1000, y_mm: y_m * 1000, junction: true, control, corner_mm }
}

#[cfg(test)]
const fn gate_at(x_m: i32, y_m: i32) -> NodeDef {
    NodeDef { x_mm: x_m * 1000, y_mm: y_m * 1000, junction: false, control: 0, corner_mm: 0 }
}

pub(super) struct EdgeDef {
    /// Node indices. The street editor shows the street looking from `a` to `b`.
    pub(super) a: usize,
    pub(super) b: usize,
    /// Index into `SAMPLES`: what sort of street it is.
    pub(super) street: usize,
    /// Its name, where it is not just the sort of street it is.
    pub(super) name: Option<String>,
    /// The street as it first stands, where it is not the sample.
    pub(super) section: Option<Street>,
    /// The way the street leaves node `a` and node `b`, in degrees clockwise from north, where its real
    /// shape is known. Otherwise it leaves along the straight line to its other end.
    pub(super) headings: Option<(f64, f64)>,
    /// The street's centreline in layout millimetres, from node `a` to node `b`, where its real shape is
    /// known. Otherwise it runs along the straight line between them.
    pub(super) shape: Option<Vec<(i32, i32)>>,
}

#[cfg(test)]
const fn street(a: usize, b: usize, street: usize) -> EdgeDef {
    EdgeDef { a, b, street, name: None, section: None, headings: None, shape: None }
}

#[cfg(test)]
const STREET: usize = 0;
#[cfg(test)]
const AVENUE: usize = 1;
#[cfg(test)]
const LANE: usize = 2;
#[cfg(test)]
const FREEWAY: usize = 3;

#[cfg(test)]
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

#[cfg(test)]
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
#[cfg(test)]
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

/// Where the junctions and street ends are, and which street joins which. The
/// network is fixed for a city; what a resident edits is kept apart from it.
pub struct Layout {
    pub(super) name: String,
    /// The side traffic keeps to.
    pub(super) side: Side,
    pub(super) nodes: Vec<NodeDef>,
    pub(super) edges: Vec<EdgeDef>,
    /// Where the layout's origin lies in the network it was made from, in metres east and north of the
    /// network's own origin. The sample city is on no network, so has none.
    pub(super) origin_m: Option<(f64, f64)>,
}

/// The widest a gap between neighbouring arms can be made by moving them, in degrees: a bend with a
/// side street on its inside has a gap a little over 180, which is a junction; a fan is not one.
const MOST_CLOSED: i32 = 60;

/// Moves bearings (sorted, in degrees) to what the junction editor allows: apart where two are closer than
/// it allows, each by half of what is missing, and together where a gap is wider than a straight line (by up to
/// `MOST_CLOSED`), in the editor's steps. Bearings that cannot all be made to fit are left.
fn spread(b: &mut [i32]) {
    use crate::junction::model::BEARING_STEP;
    let n = b.len();
    let gap = |b: &[i32], i: usize| (b[(i + 1) % n] - b[i]).rem_euclid(360);
    if let Some(i) = (0..n).find(|&i| (181..=180 + MOST_CLOSED).contains(&gap(b, i))) {
        // the two arms either side of the wide gap each move into it by half the excess
        let take = (((gap(b, i) - 180) as f64 / 2.0 / BEARING_STEP as f64).ceil() as i32) * BEARING_STEP;
        b[i] = (b[i] + take).rem_euclid(360);
        b[(i + 1) % n] = (b[(i + 1) % n] - take).rem_euclid(360);
    }
    separate(b);
}

fn separate(b: &mut [i32]) {
    use crate::junction::model::{BEARING_STEP, MIN_SEPARATION};
    let n = b.len();
    for _ in 0..40 {
        let mut moved = false;
        for i in 0..n {
            let j = (i + 1) % n;
            let gap = (b[j] - b[i]).rem_euclid(360);
            if n > 1 && gap < MIN_SEPARATION {
                let give = (((MIN_SEPARATION - gap) as f64 / 2.0 / BEARING_STEP as f64).ceil() as i32) * BEARING_STEP;
                b[i] = (b[i] - give).rem_euclid(360);
                b[j] = (b[j] + give).rem_euclid(360);
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }
}

impl Layout {
    /// The hand-made city the tests are written against. It is not part of the app.
    #[cfg(test)]
    pub fn sample() -> Layout {
        Layout { name: NAME.to_string(), side: Side::Right, nodes: NODES.into_iter().collect(), edges: EDGES.into_iter().collect(), origin_m: None }
    }
}

pub struct City {
    layout: Layout,
    streets: BTreeMap<u32, Street>,
    junctions: BTreeMap<u32, State>,
    today_streets: BTreeMap<u32, Street>,
    today_junctions: BTreeMap<u32, State>,
}

/// How long a line through these points is, in millimetres.
fn path_mm(points: &[(i32, i32)]) -> f64 {
    points.windows(2).map(|w| ((w[1].0 - w[0].0) as f64).hypot((w[1].1 - w[0].1) as f64)).sum()
}

impl Layout {
    /// The street's centreline from node `a` to node `b`, in millimetres: its own shape, or the straight line.
    fn shape_of(&self, edge: usize) -> Vec<(i32, i32)> {
        let e = &self.edges[edge];
        e.shape.clone().unwrap_or_else(|| {
            let (a, b) = (&self.nodes[e.a], &self.nodes[e.b]);
            vec![(a.x_mm, a.y_mm), (b.x_mm, b.y_mm)]
        })
    }

    /// The bearing at which street `edge` leaves `node`, to the nearest step the junction editor uses.
    fn leaving(&self, edge: usize, node: usize) -> i32 {
        let e = &self.edges[edge];
        match e.headings {
            Some((a, b)) => {
                let deg = if e.a == node { a } else { b };
                ((deg / junction::BEARING_STEP as f64).round() as i32 * junction::BEARING_STEP).rem_euclid(360)
            }
            None => self.bearing(node, self.other_end(edge, node)),
        }
    }

    /// The bearing from node `from` toward node `to`, clockwise from north (up),
    /// to the nearest step the junction editor uses.
    fn bearing(&self, from: usize, to: usize) -> i32 {
        let dx = (self.nodes[to].x_mm - self.nodes[from].x_mm) as f64;
        let dy = (self.nodes[to].y_mm - self.nodes[from].y_mm) as f64;
        let deg = dx.atan2(-dy).to_degrees();
        ((deg / junction::BEARING_STEP as f64).round() as i32 * junction::BEARING_STEP).rem_euclid(360)
    }

    pub(super) fn edges_at(&self, node: usize) -> Vec<usize> {
        (0..self.edges.len()).filter(|&e| self.edges[e].a == node || self.edges[e].b == node).collect()
    }

    fn other_end(&self, edge: usize, node: usize) -> usize {
        if self.edges[edge].a == node { self.edges[edge].b } else { self.edges[edge].a }
    }

    /// Junctions are numbered in reading order, so the names run across the map.
    fn junction_number(&self, node: usize) -> usize {
        let mut order: Vec<usize> = (0..self.nodes.len()).filter(|&n| self.nodes[n].junction).collect();
        order.sort_by_key(|&n| (self.nodes[n].y_mm, self.nodes[n].x_mm));
        order.iter().position(|&n| n == node).map_or(0, |p| p + 1)
    }

    /// The distinct names of the streets meeting at `node`, in the order the edges are held.
    fn street_names_at(&self, node: usize) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for e in self.edges_at(node) {
            if let Some(n) = &self.edges[e].name
                && !names.contains(n)
            {
                names.push(n.clone());
            }
        }
        names
    }

    /// Who a junction's streets are, in a title: "Main Street and Side Road".
    fn joined(names: &[String]) -> String {
        match names {
            [] => String::new(),
            [one] => format!("{one} junction"),
            [a, b] => format!("{a} and {b}"),
            [a, b, c] => format!("{a}, {b} and {c}"),
            [a, b, rest @ ..] => format!("{a}, {b} and {} more", rest.len()),
        }
    }

    /// A place as a title.
    pub(super) fn node_name(&self, node: usize) -> String {
        let names = self.street_names_at(node);
        match (self.nodes[node].junction, names.first()) {
            (true, Some(_)) => Self::joined(&names),
            (true, None) => format!("Junction {}", self.junction_number(node)),
            (false, Some(street)) if self.edges_at(node).len() == 1 => format!("End of {street}"),
            (false, Some(street)) => format!("Connection on {street}"),
            (false, None) => "Edge of the map".to_string(),
        }
    }

    /// A place in a sentence.
    pub(super) fn end_name(&self, node: usize) -> String {
        let names = self.street_names_at(node);
        match (self.nodes[node].junction, names.first()) {
            (true, Some(_)) => Self::joined(&names),
            (true, None) => format!("Junction {}", self.junction_number(node)),
            (false, Some(street)) if self.edges_at(node).len() == 1 => format!("the end of {street}"),
            (false, Some(street)) => format!("a connection on {street}"),
            (false, None) => "the edge of the map".to_string(),
        }
    }

    fn edge_name(&self, edge: usize) -> String {
        let e = &self.edges[edge];
        let kind = e.name.as_deref().unwrap_or(SAMPLES[e.street].name);
        if !self.nodes[e.a].junction && !self.nodes[e.b].junction {
            format!("{kind} · through the city")
        } else {
            format!("{kind} · {} to {}", self.end_name(e.a), self.end_name(e.b))
        }
    }

    /// A junction as first laid out, from the streets that meet there.
    fn generate(&self, node: usize, streets: &BTreeMap<u32, Street>) -> State {
        let def = &self.nodes[node];
        let mut incident: Vec<(i32, usize)> = self.edges_at(node).into_iter().map(|e| (self.leaving(e, node), e)).collect();
        incident.sort_by_key(|(b, _)| *b);
        let mut bearings: Vec<i32> = incident.iter().map(|(b, _)| *b).collect();
        spread(&mut bearings);
        for (i, b) in bearings.into_iter().enumerate() {
            incident[i].0 = b;
        }
        incident.sort_by_key(|(b, _)| *b);
        let mut arms: Vec<Arm> = incident
            .iter()
            .enumerate()
            .map(|(i, &(b, e))| {
                let mut arm = Arm::new(i as u32 + 1, self.edges[e].street, b, 0);
                arm.edge = e as u32 + 1;
                arm.corner_mm = def.corner_mm;
                arm.section = streets.get(&arm.edge).map(|s| seen_from(self, e, node, s));
                arm
            })
            .collect();
        junction::normalize(&mut arms, 0);
        tune_corners(&mut arms, def.control, def.corner_mm);
        let mut s = State { label: "Junction today".into(), arms, control: def.control, ring_extra_mm: 0, bus: None, cycle: None };
        forget_streets(&mut s);
        s
    }

    /// Whether a saved junction can stand for this node: the same streets, with
    /// every index and length inside what the editor allows.
    fn junction_fits(&self, s: &State, node: usize) -> bool {
        use crate::junction::model::*;
        let mut want: Vec<u32> = self.edges_at(node).iter().map(|&e| e as u32 + 1).collect();
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
                    && a.crossing
                        .is_none_or(|c| (MIN_SETBACK_MM..=MAX_SETBACK_MM).contains(&c.setback_mm) && (MIN_CROSSING_MM..=MAX_CROSSING_MM).contains(&c.width_mm))
            })
    }
}

/// Gives each corner the radius nearest `preferred` that leaves its sidewalk
/// and keeps its turn slow. Streets differ in the pavement they have beside
/// the curb, so one radius does not suit every corner of a junction. A corner
/// nothing suits keeps `preferred`.
fn tune_corners(arms: &mut [Arm], control: usize, preferred: i32) {
    use crate::junction::model::{MAX_CORNER_MM, MIN_CORNER_MM, RING_STEP_MM};
    let state = |arms: &[Arm]| State { label: String::new(), arms: arms.to_vec(), control, ring_extra_mm: 0, bus: None, cycle: None };
    let mut radii: Vec<i32> = (MIN_CORNER_MM..=MAX_CORNER_MM).step_by(RING_STEP_MM as usize).collect();
    radii.sort_by_key(|r| ((r - preferred).abs(), *r));
    for i in 0..arms.len() {
        let uid = arms[i].uid;
        let found = radii.iter().any(|&r| {
            arms[i].corner_mm = r;
            let s = state(arms);
            Junction::from_city("", &s, &s, 0).is_some_and(|j| j.view().corners.iter().any(|c| c.uid == uid && c.ok && !c.fast))
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

fn same_junction(a: &State, b: &State) -> bool {
    State { label: String::new(), ..a.clone() } == State { label: String::new(), ..b.clone() }
}

/// The street an arm reads: the city's street as seen looking out from `node`.
fn seen_from(layout: &Layout, edge: usize, node: usize, street: &Street) -> Street {
    if layout.edges[edge].a == node { street.clone() } else { street.reversed() }
}

#[cfg(test)]
impl Default for City {
    fn default() -> Self {
        City::new()
    }
}

impl City {
    /// The hand-made city the tests are written against, as first laid out: every street a sample, every
    /// junction generated from the streets that meet there. It is not part of the app.
    #[cfg(test)]
    pub fn new() -> City {
        City::on(Layout::sample())
    }

    /// The same for any network.
    pub fn on(mut layout: Layout) -> City {
        let today_streets: BTreeMap<u32, Street> =
            layout.edges.iter().enumerate().map(|(i, e)| (i as u32 + 1, e.section.clone().unwrap_or_else(|| Street::sample(e.street, layout.side)))).collect();
        let region = REGIONS.iter().position(|r| r.drive_side == layout.side).unwrap_or(0);
        let mut today_junctions: BTreeMap<u32, State> = BTreeMap::new();
        for n in 0..layout.nodes.len() {
            if !layout.nodes[n].junction {
                continue;
            }
            let mut s = layout.generate(n, &today_streets);
            for a in &mut s.arms {
                let e = (a.edge as usize).saturating_sub(1);
                a.section = today_streets.get(&a.edge).map(|st| seen_from(&layout, e, n, st));
            }
            // A meeting the junction editor cannot draw (every road leaving to one side, say) is left as
            // a plain connection of streets.
            if Junction::from_city("", &s, &s, region).is_none() {
                layout.nodes[n].junction = false;
                continue;
            }
            forget_streets(&mut s);
            today_junctions.insert(node_uid(n), s);
        }
        City { layout, streets: today_streets.clone(), junctions: today_junctions.clone(), today_streets, today_junctions }
    }

    /// The hand-made city as saved, or as first laid out for any part of the save that
    /// cannot be used. Not an error: a stale or damaged save just starts over.
    #[cfg(test)]
    pub fn load(json: &str) -> City {
        City::load_on(Layout::sample(), json)
    }

    /// The same for any network.
    pub fn load_on(layout: Layout, json: &str) -> City {
        let mut city = City::on(layout);
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
            let node = (uid as usize).checked_sub(1).filter(|&n| n < city.layout.nodes.len() && city.layout.nodes[n].junction);
            if node.is_some_and(|n| city.layout.junction_fits(&state, n)) {
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

    fn edge_index(&self, uid: u32) -> Option<usize> {
        (uid as usize).checked_sub(1).filter(|&e| e < self.layout.edges.len())
    }

    fn node_index(&self, uid: u32) -> Option<usize> {
        (uid as usize).checked_sub(1).filter(|&n| n < self.layout.nodes.len())
    }

    pub fn street_name(&self, edge: u32) -> Option<String> {
        self.edge_index(edge).map(|e| self.layout.edge_name(e))
    }

    pub fn junction_name(&self, node: u32) -> Option<String> {
        self.node_index(node).filter(|&n| self.layout.nodes[n].junction).map(|n| self.layout.node_name(n))
    }

    /// The two ends of a street, first the one the street editor looks from.
    pub fn street_ends(&self, edge: u32) -> Vec<EndView> {
        let Some(e) = self.edge_index(edge) else { return Vec::new() };
        [self.layout.edges[e].a, self.layout.edges[e].b]
            .into_iter()
            .map(|n| EndView { uid: node_uid(n), name: self.layout.end_name(n), junction: self.layout.nodes[n].junction })
            .collect()
    }

    /// The street editor on one street of the city.
    pub fn street_editor(&self, edge: u32, region: usize) -> Option<Editor> {
        self.edge_index(edge)?;
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
        self.with_streets_of(&self.streets, node, s)
    }

    fn with_streets_of(&self, streets: &BTreeMap<u32, Street>, node: usize, s: &State) -> State {
        let mut s = s.clone();
        for a in &mut s.arms {
            let edge = a.edge;
            a.section = self.edge_index(edge).and_then(|e| streets.get(&edge).map(|st| seen_from(&self.layout, e, node, st)));
        }
        s
    }

    /// The checks a junction fails as the city first laid it out, by label. A real city's streets
    /// fall short of the rules as they stand; only what a change adds is flagged.
    fn failing_today(&self, node: u32, region: usize) -> Vec<String> {
        let Some(n) = self.node_index(node) else { return Vec::new() };
        let Some(state) = self.today_junctions.get(&node) else { return Vec::new() };
        let today = self.with_streets_of(&self.today_streets, n, state);
        Junction::from_city("", &today, &today, region)
            .map_or_else(Vec::new, |j| j.view().checks.iter().filter(|c| !c.ok).map(|c| c.label.to_string()).collect())
    }

    /// The junction editor on one junction of the city, reading the streets as
    /// they now stand. None when it cannot be drawn with them.
    pub fn junction_editor(&self, node: u32, region: usize) -> Option<Junction> {
        let n = self.node_index(node).filter(|&n| self.layout.nodes[n].junction)?;
        let today = self.with_streets(n, self.today_junctions.get(&node)?);
        let now = self.with_streets(n, self.junctions.get(&node)?);
        Junction::from_city(&self.layout.node_name(n), &today, &now, region)
    }

    /// Keeps what the junction editor has made of a junction.
    pub fn keep_junction(&mut self, node: u32, state: State) -> bool {
        let fits = self.node_index(node).filter(|&n| self.layout.nodes[n].junction).is_some_and(|n| self.layout.junction_fits(&state, n));
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
            if !self.layout.nodes[n].junction {
                return 0;
            }
            self.layout.edges_at(n).iter().map(|&e| SAMPLES[self.layout.edges[e].street].row_mm / 2).max().unwrap_or(0)
        };
        for (i, e) in self.layout.edges.iter().enumerate() {
            let uid = i as u32 + 1;
            let today = &self.today_streets[&uid];
            let now = &self.streets[&uid];
            let editor = Editor::from_street(today, now, region);
            let v = editor.view();
            let at_first: Vec<String> = Editor::from_street(today, today, region).view().checks.iter().filter(|c| !c.ok).map(|c| c.label.to_string()).collect();
            let failing: Vec<String> = v.checks.iter().filter(|c| !c.ok && !at_first.iter().any(|l| l == c.label)).map(|c| c.label.to_string()).collect();
            let row = v.row_mm;
            let shape = self.layout.shape_of(i);
            edges.push(EdgeView {
                uid,
                a: node_uid(e.a),
                b: node_uid(e.b),
                name: self.layout.edge_name(i),
                kind: now.title(),
                row_mm: row,
                total_mm: v.total_mm,
                length_mm: path_mm(&shape).round() as i32,
                shape_mm: shape.iter().map(|&(x, y)| [x, y]).collect(),
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
        for (i, def) in self.layout.nodes.iter().enumerate() {
            let uid = node_uid(i);
            let mut v = NodeView {
                uid,
                name: self.layout.node_name(i),
                number: if def.junction { self.layout.junction_number(i) as u32 } else { 0 },
                x_mm: def.x_mm,
                y_mm: def.y_mm,
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
                        let at_first = self.failing_today(uid, region);
                        v.failing = j.view().checks.iter().filter(|c| !c.ok && !at_first.iter().any(|l| l == c.label)).map(|c| c.label.to_string()).collect();
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
        // A city of no places lies at its origin, not in a box turned inside out.
        let (x0, y0, x1, y1) = if nodes.is_empty() {
            (0, 0, 0, 0)
        } else {
            nodes.iter().fold((i32::MAX, i32::MAX, i32::MIN, i32::MIN), |(x0, y0, x1, y1), n| (x0.min(n.x_mm), y0.min(n.y_mm), x1.max(n.x_mm), y1.max(n.y_mm)))
        };
        let places = nodes.iter().filter(|n| n.junction).count() + edges.len();
        let failing = nodes.iter().filter(|n| !n.ok).count() + edges.iter().filter(|e| !e.ok).count();
        let edited = nodes.iter().filter(|n| n.edited).count() + edges.iter().filter(|e| e.edited).count();
        CityView {
            name: self.layout.name.clone(),
            nodes,
            edges,
            bounds_mm: [x0, y0, x1, y1],
            places,
            failing,
            edited,
            origin_m: self.layout.origin_m.map(|(x, y)| [x, y]),
        }
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
    pub kind: String,
    pub row_mm: i32,
    pub total_mm: i32,
    pub length_mm: i32,
    /// The street's centreline from node `a` to node `b`, in millimetres: two points or more.
    pub shape_mm: Vec<[i32; 2]>,
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
    /// Which junction it is, counting in reading order from 1; 0 where a street leaves the map.
    pub number: u32,
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
    pub name: String,
    pub nodes: Vec<NodeView>,
    pub edges: Vec<EdgeView>,
    pub bounds_mm: [i32; 4],
    /// Junctions and streets together.
    pub places: usize,
    pub failing: usize,
    pub edited: usize,
    /// Where the layout's origin lies in the network, in metres east and north of its corner; None for the sample city.
    pub origin_m: Option<[f64; 2]>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn junction_number(node: usize) -> usize {
        Layout::sample().junction_number(node)
    }

    fn edges_at(node: usize) -> Vec<usize> {
        Layout::sample().edges_at(node)
    }

    fn node_name(node: usize) -> String {
        Layout::sample().node_name(node)
    }

    fn edge_name(edge: usize) -> String {
        Layout::sample().edge_name(edge)
    }

    fn gaps(b: &[i32]) -> Vec<i32> {
        (0..b.len()).map(|i| (b[(i + 1) % b.len()] - b[i]).rem_euclid(360)).collect()
    }

    #[test]
    fn a_bend_with_a_side_street_on_its_inside_is_made_a_junction_and_a_fan_is_not() {
        // 185 degrees across the outside of the bend
        let mut b = [0, 185, 270];
        spread(&mut b);
        assert!(gaps(&b).iter().all(|g| (30..=180).contains(g)), "{b:?}");
        // a larger excess is closed a step at a time on each side
        let mut b = [0, 80, 150];
        spread(&mut b);
        assert!(gaps(&b).iter().all(|g| (30..=180).contains(g)), "{b:?}");
        // three streets all within a quarter of the compass are a fan, which no junction is
        let mut b = [0, 50, 100];
        let before = b;
        spread(&mut b);
        assert_eq!(b, before);
        // what was already a junction is left as it is
        let mut b = [0, 90, 180, 270];
        spread(&mut b);
        assert_eq!(b, [0, 90, 180, 270]);
    }

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
        let bad: Vec<_> =
            v.nodes.iter().filter(|n| !n.ok).map(|n| (&n.name, &n.failing)).chain(v.edges.iter().filter(|e| !e.ok).map(|e| (&e.name, &e.failing))).collect();
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
    fn a_junction_is_laid_out_without_refuge_islands_whatever_the_width() {
        let city = City::new();
        let mut crossings = 0;
        for n in 0..city.layout.nodes.len() {
            let Some(j) = city.junction_editor(node_uid(n), 0) else { continue };
            for c in j.current().arms.iter().filter_map(|a| a.crossing) {
                crossings += 1;
                assert!(!c.island, "node {n} starts with an island");
            }
        }
        assert!(crossings > 0);
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
