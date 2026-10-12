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

use super::edits::{ArmStreet, Edits, Ids, JunctionRecord, SAME_WAY_DEG, StreetRecord, assign, degrees_apart};
use crate::junction::model::{self as junction, Arm, Junction, State};
#[cfg(test)]
use crate::junction::model::{ALL_WAY_STOP, PRIORITY, SIGNAL};
use crate::shared::catalogue::{KINDS, REGIONS, Side, StreetClass};
use crate::shared::provenance::OsmRef;
use crate::shared::said::{Arg, Said};
#[cfg(test)]
use crate::street::model::SAMPLES;
use crate::street::model::{Editor, Street, named_said, unnamed_said};

/// Bump when what is saved changes shape; an older save is then left behind.
const SAVE_VERSION: u32 = 2;

pub(super) struct NodeDef {
    pub(super) x_mm: i32,
    pub(super) y_mm: i32,
    /// Where three to five streets meet. Otherwise the street runs off the map.
    pub(super) junction: bool,
    pub(super) control: usize,
    pub(super) corner_mm: i32,
    /// The OSM nodes it was made from.
    pub(super) source: Vec<OsmRef>,
}

#[cfg(test)]
const fn junction_at(x_m: i32, y_m: i32, control: usize, corner_mm: i32) -> NodeDef {
    NodeDef { x_mm: x_m * 1000, y_mm: y_m * 1000, junction: true, control, corner_mm, source: Vec::new() }
}

#[cfg(test)]
const fn gate_at(x_m: i32, y_m: i32) -> NodeDef {
    NodeDef { x_mm: x_m * 1000, y_mm: y_m * 1000, junction: false, control: 0, corner_mm: 0, source: Vec::new() }
}

pub(super) struct EdgeDef {
    /// Node indices. The street editor shows the street looking from `a` to `b`.
    pub(super) a: usize,
    pub(super) b: usize,
    /// What sort of street it is.
    pub(super) class: StreetClass,
    /// Its name, where the network gives it one.
    pub(super) name: Option<String>,
    /// The street as it first stands.
    pub(super) section: Street,
    /// The way the street leaves node `a` and node `b`, in degrees clockwise from north, where its real
    /// shape is known. Otherwise it leaves along the straight line to its other end.
    pub(super) headings: Option<(f64, f64)>,
    /// The street's centreline in layout millimetres, from node `a` to node `b`, where its real shape is
    /// known. Otherwise it runs along the straight line between them.
    pub(super) shape: Option<Vec<(i32, i32)>>,
}

/// A street of the hand-made test city: between two nodes, laid out like one of the sample streets.
#[cfg(test)]
struct TestEdge {
    a: usize,
    b: usize,
    fixture: usize,
}

#[cfg(test)]
const fn street(a: usize, b: usize, fixture: usize) -> TestEdge {
    TestEdge { a, b, fixture }
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
const EDGES: [TestEdge; 23] = [
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

/// A place of the layout, by its index: a node, or an edge.
#[derive(Clone, Copy, Debug)]
pub(super) enum At {
    Node(usize),
    Edge(usize),
}

/// What the city kept between visits before the edits document: only the migration reads it.
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

/// The messages that name a place that is not a junction, as a title or in a sentence. The end of an unnamed
/// road has its own, because French says `d’une rue` where it says `de Rue Rachel`.
struct PlaceForms {
    end_of: &'static str,
    end_of_unnamed: &'static str,
    connection_on: &'static str,
    map_edge: &'static str,
}

const TITLE: PlaceForms =
    PlaceForms { end_of: "city-end-of", end_of_unnamed: "city-end-of-unnamed", connection_on: "city-connection-on", map_edge: "city-map-edge" };

const IN_SENTENCE: PlaceForms = PlaceForms {
    end_of: "city-the-end-of",
    end_of_unnamed: "city-the-end-of-unnamed",
    connection_on: "city-a-connection-on",
    map_edge: "city-the-edge-of-the-map",
};

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
        let edges = EDGES
            .iter()
            .enumerate()
            .map(|(i, e)| EdgeDef {
                a: e.a,
                b: e.b,
                class: SAMPLES[e.fixture].class,
                name: None,
                section: Street { source: vec![OsmRef { id: 2000 + i as i64, version: None }], ..Street::sample(e.fixture, Side::Right) },
                headings: None,
                shape: None,
            })
            .collect();
        Layout {
            name: NAME.to_string(),
            side: Side::Right,
            nodes: NODES.into_iter().enumerate().map(|(i, n)| NodeDef { source: vec![OsmRef { id: 1000 + i as i64, version: None }], ..n }).collect(),
            edges,
            origin_m: None,
        }
    }
}

pub struct City {
    layout: Layout,
    streets: BTreeMap<u32, Street>,
    junctions: BTreeMap<u32, State>,
    today_streets: BTreeMap<u32, Street>,
    today_junctions: BTreeMap<u32, State>,
    /// The key of each street, by edge index; None where it has none or another street has the same one.
    street_keys: Vec<Option<String>>,
    /// The same for the junctions, by node index (None for a node that is not a junction).
    junction_keys: Vec<Option<String>>,
    /// The key of the record each street (by uid) took its edit from, where it took one.
    took_streets: BTreeMap<u32, String>,
    /// The same for the junctions.
    took_junctions: BTreeMap<u32, String>,
}

/// The keys, each kept only where no other place has it: a key two places share would bring back one place's
/// edit on the other, or on neither, so such a place is not keyed (its edits are refused, not lost).
fn alone(keys: Vec<Option<String>>) -> Vec<Option<String>> {
    let mut count: BTreeMap<String, usize> = BTreeMap::new();
    for k in keys.iter().flatten() {
        *count.entry(k.clone()).or_default() += 1;
    }
    keys.into_iter().map(|k| k.filter(|k| count[k] == 1)).collect()
}

/// How long a line through these points is, in millimetres.
fn path_mm(points: &[(i32, i32)]) -> f64 {
    points.windows(2).map(|w| ((w[1].0 - w[0].0) as f64).hypot((w[1].1 - w[0].1) as f64)).sum()
}

impl Layout {
    /// The key of the junction at `node`, from the OpenStreetMap nodes it was made from.
    pub(super) fn node_key(&self, node: usize) -> Option<String> {
        super::keys::junction_key(&self.nodes[node].source)
    }

    /// The way street `edge` leaves `node`, in degrees clockwise from north: from the layout node to the first point
    /// at least a metre along its centreline (the last, where it is shorter). The centreline is osm2streets' trimmed
    /// one and the node is its junction's point, both of which move when the junction's cluster changes with the
    /// area's box: the same street measured in two areas has differed by up to 60 degrees (43 the worst among 5442
    /// pairs with the same key; 90 pairs 20 degrees or more). See `keys::HEADINGS` for what that risks.
    pub(super) fn leaving_deg(&self, edge: usize, node: usize) -> f64 {
        let mut shape = self.shape_of(edge);
        if self.edges[edge].a != node {
            shape.reverse();
        }
        let from = shape[0];
        let far = |p: &&(i32, i32)| ((p.0 - from.0) as f64).hypot((p.1 - from.1) as f64) >= 1_000.0;
        let to = shape.iter().skip(1).find(far).or(shape.last()).copied().unwrap_or(from);
        // layout millimetres run east and south
        ((to.0 - from.0) as f64).atan2(-(to.1 - from.1) as f64).to_degrees().rem_euclid(360.0)
    }

    /// The key of each street, from its ways, the nodes at its ends and, where an end is cut at the area's box or
    /// another street of the same way joins the same two ends, the way it leaves its first end (`keys::street_key`).
    pub(super) fn edge_keys(&self) -> Vec<Option<String>> {
        let key = |e: usize, twinned: bool| {
            let d = &self.edges[e];
            let end = |n: usize| super::keys::End { source: &self.nodes[n].source, leaving_deg: self.leaving_deg(e, n) };
            super::keys::street_key(&d.section.source, end(d.a), end(d.b), twinned)
        };
        let plain: Vec<Option<String>> = (0..self.edges.len()).map(|e| key(e, false)).collect();
        let mut count: BTreeMap<&str, usize> = BTreeMap::new();
        for k in plain.iter().flatten() {
            *count.entry(k).or_default() += 1;
        }
        let twinned: Vec<bool> = plain.iter().map(|k| k.as_deref().is_some_and(|k| count[k] > 1)).collect();
        (0..self.edges.len()).map(|e| if twinned[e] { key(e, true) } else { plain[e].clone() }).collect()
    }

    /// What identifies street `edge` when an edit is matched to it: its key, its ways, the nodes of both ends and
    /// the way it leaves each end that is an OpenStreetMap node.
    fn street_ids(&self, edge: usize, key: Option<String>) -> Ids {
        let e = &self.edges[edge];
        Ids {
            key,
            ways: super::keys::ids(&e.section.source),
            nodes: &super::keys::ids(&self.nodes[e.a].source) | &super::keys::ids(&self.nodes[e.b].source),
            leaving: [e.a, e.b]
                .into_iter()
                .filter_map(|n| Some((self.nodes[n].source.iter().map(|r| r.id).min()?, self.leaving_deg(edge, n).round() as i32 % 360)))
                .collect(),
        }
    }

    /// What identifies the junction at `node` when an edit is matched to it: its key and its nodes.
    fn junction_ids(&self, node: usize, key: Option<String>) -> Ids {
        Ids { key, nodes: super::keys::ids(&self.nodes[node].source), ..Ids::default() }
    }

    /// How far, in metres, a place lies from the centre of the area box its network was cut to (a street is
    /// taken to be where the middle of its centreline is). Network metres are measured from that box's
    /// south-west corner, and `origin_m` is where the layout's origin lies in them. A layout on no network has
    /// no such centre, so its places are infinitely far.
    pub(super) fn centre_distance_m(&self, at: At) -> f64 {
        let Some((ox, oy)) = self.origin_m else { return f64::MAX };
        let (x_mm, y_mm) = match at {
            At::Node(n) => (self.nodes[n].x_mm as f64, self.nodes[n].y_mm as f64),
            At::Edge(e) => {
                let shape = self.shape_of(e);
                let (a, b) = (shape[(shape.len() - 1) / 2], shape[shape.len() / 2]);
                ((a.0 as f64 + b.0 as f64) / 2.0, (a.1 as f64 + b.1 as f64) / 2.0)
            }
        };
        let (x, y) = (ox + x_mm / 1000.0, oy - y_mm / 1000.0);
        (x - crate::place::area::HALF_M).hypot(y - crate::place::area::HALF_M)
    }

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

    /// A junction, by the streets that meet there ("Main Street and Side Road"), or by its number where
    /// none of them has a name.
    fn junction_title(&self, node: usize) -> Said {
        let names = self.street_names_at(node);
        let name = |i: usize| Arg::Text(names[i].clone());
        match names.len() {
            0 => Said::new("city-junction-number").with("n", Arg::Num(self.junction_number(node) as i64)),
            1 => Said::new("city-junction-of-one").with("a", name(0)),
            2 => Said::new("city-junction-of-two").with("a", name(0)).with("b", name(1)),
            3 => Said::new("city-junction-of-three").with("a", name(0)).with("b", name(1)).with("c", name(2)),
            n => Said::new("city-junction-of-many").with("a", name(0)).with("b", name(1)).with("rest", Arg::Num(n as i64 - 2)),
        }
    }

    /// A road at `node` that has no name at all, said as one in a sentence ("an unnamed local street"); None
    /// where every road there has a name, of its own or in its section (the sample city's streets).
    fn unnamed_at(&self, node: usize) -> Option<Said> {
        let e = self.edges_at(node).into_iter().map(|e| &self.edges[e]).find(|e| e.name.is_none() && e.section.name.is_none())?;
        Some(unnamed_said(e.section.class, true))
    }

    /// A place that is not a junction, in the words of `forms`: the end of a street, a connection on one, or
    /// the edge of the map where no road there has any name to lend it.
    fn place_name(&self, node: usize, forms: &PlaceForms) -> Said {
        let dead_end = self.edges_at(node).len() == 1;
        if let Some(street) = self.street_names_at(node).first() {
            let key = if dead_end { forms.end_of } else { forms.connection_on };
            return Said::new(key).with("street", Arg::Text(street.clone()));
        }
        match self.unnamed_at(node) {
            Some(street) => {
                let key = if dead_end { forms.end_of_unnamed } else { forms.connection_on };
                Said::new(key).with("street", Arg::Said(Box::new(street)))
            }
            None => Said::new(forms.map_edge),
        }
    }

    /// A place as a title.
    pub(super) fn node_name(&self, node: usize) -> Said {
        if self.nodes[node].junction { self.junction_title(node) } else { self.place_name(node, &TITLE) }
    }

    /// A place in a sentence.
    pub(super) fn end_name(&self, node: usize) -> Said {
        if self.nodes[node].junction { self.junction_title(node) } else { self.place_name(node, &IN_SENTENCE) }
    }

    /// What a street is called: the name the network gives it, or else its section's title.
    pub(super) fn edge_title(&self, edge: usize) -> Said {
        let e = &self.edges[edge];
        e.name.as_ref().map_or_else(|| e.section.title(), |name| named_said(name))
    }

    /// Where a street runs: from one place to another, or through the city where neither end is a junction.
    pub(super) fn edge_ends(&self, edge: usize) -> Said {
        let e = &self.edges[edge];
        if !self.nodes[e.a].junction && !self.nodes[e.b].junction {
            Said::new("city-edge-through")
        } else {
            Said::new("city-edge-between").with("from", Arg::Said(Box::new(self.end_name(e.a)))).with("to", Arg::Said(Box::new(self.end_name(e.b))))
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
                let mut arm = Arm::new(i as u32 + 1, b, 0);
                arm.edge = e as u32 + 1;
                arm.corner_mm = def.corner_mm;
                arm.section = streets.get(&arm.edge).map(|s| seen_from(self, e, node, s));
                arm
            })
            .collect();
        junction::normalize(&mut arms, 0);
        tune_corners(&mut arms, def.control, def.corner_mm);
        let mut s =
            State { label: junction::today(), arms, control: def.control, ring_extra_mm: 0, bus: None, cycle: None, raised: false, source: def.source.clone() };
        forget_streets(&mut s);
        s
    }

    /// Whether a saved junction can stand for this node: the same streets, with
    /// every index and length inside what the editor allows.
    fn junction_fits(&self, s: &State, node: usize) -> bool {
        use crate::junction::model::{
            APPROACH_MAX_MM, APPROACH_MIN_MM, APPROACHES, BEARING_STEP, CONTROLS, CYCLE_MAX_MM, CYCLE_MIN_MM, MAX_CORNER_MM, MAX_CROSSING_MM, MAX_RING_MM,
            MAX_SETBACK_MM, MIN_CORNER_MM, MIN_CROSSING_MM, MIN_SETBACK_MM, RULES, STOPS,
        };
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
                a.bearing % BEARING_STEP == 0
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
    let state = |arms: &[Arm]| State {
        label: junction::unlabelled(),
        arms: arms.to_vec(),
        control,
        ring_extra_mm: 0,
        bus: None,
        cycle: None,
        raised: false,
        source: Vec::new(),
    };
    let mut radii: Vec<i32> = (MIN_CORNER_MM..=MAX_CORNER_MM).step_by(RING_STEP_MM as usize).collect();
    radii.sort_by_key(|r| ((r - preferred).abs(), *r));
    for i in 0..arms.len() {
        let uid = arms[i].uid;
        let found = radii.iter().any(|&r| {
            arms[i].corner_mm = r;
            let s = state(arms);
            // `from_city` wants a name for the junction; `unlabelled()` is a throwaway here, nothing shows it.
            Junction::from_city(junction::unlabelled(), &s, &s, 0).is_some_and(|j| j.view().corners.iter().any(|c| c.uid == uid && c.ok && !c.fast))
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
    State { label: junction::unlabelled(), ..a.clone() } == State { label: junction::unlabelled(), ..b.clone() }
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
        let today_streets: BTreeMap<u32, Street> = layout.edges.iter().enumerate().map(|(i, e)| (i as u32 + 1, e.section.clone())).collect();
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
            // As in `tune_corners`, the name is a throwaway: this only asks whether the junction can be drawn.
            if Junction::from_city(junction::unlabelled(), &s, &s, region).is_none() {
                layout.nodes[n].junction = false;
                continue;
            }
            forget_streets(&mut s);
            today_junctions.insert(node_uid(n), s);
        }
        let street_keys = alone(layout.edge_keys());
        let junction_keys = alone((0..layout.nodes.len()).map(|n| layout.nodes[n].junction.then(|| layout.node_key(n)).flatten()).collect());
        City {
            layout,
            streets: today_streets.clone(),
            junctions: today_junctions.clone(),
            today_streets,
            today_junctions,
            street_keys,
            junction_keys,
            took_streets: BTreeMap::new(),
            took_junctions: BTreeMap::new(),
        }
    }

    /// The hand-made city with `edits` applied. It is not part of the app.
    #[cfg(test)]
    pub fn load(edits: &Edits) -> City {
        City::load_on(Layout::sample(), edits)
    }

    /// The city of any network with the resident's `edits` applied: each edit goes to the place with its key, or
    /// failing that the one made from the same OpenStreetMap ids, and what no place takes, or what a place no
    /// longer fits, is left as first laid out. Not an error: the records stay in the document.
    pub fn load_on(layout: Layout, edits: &Edits) -> City {
        let mut city = City::on(layout);
        city.apply_streets(edits);
        city.apply_junctions(edits);
        city
    }

    fn apply_streets(&mut self, edits: &Edits) {
        let places: Vec<Ids> = (0..self.layout.edges.len()).map(|e| self.layout.street_ids(e, self.street_keys[e].clone())).collect();
        let records: Vec<(&String, &StreetRecord)> = edits.streets.iter().collect();
        let ids: Vec<Ids> = records.iter().map(|(k, r)| Ids::from_street_record(k, r)).collect();
        for (e, taken) in assign(&places, &ids).into_iter().enumerate() {
            let Some(ri) = taken else { continue };
            let uid = e as u32 + 1;
            let street = &records[ri].1.street;
            let fits = self.today_streets.get(&uid).is_some_and(|t| t.class == street.class && t.row_mm == street.row_mm);
            if fits && street.is_sound() {
                // where a street came from and what it is called are not something an edit changes
                let today = &self.today_streets[&uid];
                let (source, name) = (today.source.clone(), today.name.clone());
                self.streets.insert(uid, Street { source, name, ..street.clone() });
                self.took_streets.insert(uid, records[ri].0.clone());
            }
        }
    }

    fn apply_junctions(&mut self, edits: &Edits) {
        let nodes: Vec<usize> = (0..self.layout.nodes.len()).filter(|&n| self.layout.nodes[n].junction).collect();
        let places: Vec<Ids> = nodes.iter().map(|&n| self.layout.junction_ids(n, self.junction_keys[n].clone())).collect();
        let records: Vec<(&String, &JunctionRecord)> = edits.junctions.iter().collect();
        let ids: Vec<Ids> = records.iter().map(|(k, r)| Ids::from_junction_record(k, r)).collect();
        for (i, taken) in assign(&places, &ids).into_iter().enumerate() {
            let Some(ri) = taken else { continue };
            let n = nodes[i];
            let record = records[ri].1;
            let mut state = record.state.clone();
            // each arm goes back on the street at the junction it was kept with, now where it stands in the city
            let on_streets = state.arms.iter_mut().all(|arm| {
                let Some(e) = record.arm_streets.get(&arm.uid).and_then(|street| self.arm_street(n, street)) else { return false };
                arm.edge = e as u32 + 1;
                true
            });
            if on_streets && self.layout.junction_fits(&state, n) {
                let uid = node_uid(n);
                let source = self.today_junctions[&uid].source.clone();
                self.junctions.insert(uid, State { source, label: junction::earlier(), ..state });
                self.took_junctions.insert(uid, records[ri].0.clone());
            }
        }
    }

    /// The city as an older version kept it, by position: only for moving that save to the edits.
    pub(super) fn load_old(layout: Layout, json: &str) -> City {
        let mut city = City::on(layout);
        let Ok(saved) = serde_json::from_str::<Saved>(json) else { return city };
        if saved.version != SAVE_VERSION {
            return city;
        }
        for (uid, street) in saved.streets {
            let ok = city.today_streets.get(&uid).is_some_and(|t| t.class == street.class && t.row_mm == street.row_mm);
            if ok && street.is_sound() {
                // where a street came from and what it is called are not something an edit changes; older
                // saves did not say where, and named a street without a name in English
                let today = &city.today_streets[&uid];
                let (source, name) = (today.source.clone(), today.name.clone());
                city.streets.insert(uid, Street { source, name, ..street });
            }
        }
        for (uid, state) in saved.junctions {
            let node = (uid as usize).checked_sub(1).filter(|&n| n < city.layout.nodes.len() && city.layout.nodes[n].junction);
            if node.is_some_and(|n| city.layout.junction_fits(&state, n)) {
                let source = city.today_junctions[&uid].source.clone();
                city.junctions.insert(uid, State { source, ..state });
            }
        }
        city
    }

    /// The same, or None unless `json` is a save of the version this reads (a save that is skipped can then be
    /// told from one that changed nothing).
    pub(super) fn load_old_checked(layout: Layout, json: &str) -> Option<City> {
        serde_json::from_str::<Saved>(json).ok().filter(|s| s.version == SAVE_VERSION)?;
        Some(City::load_old(layout, json))
    }

    /// Each street changed from how it was first laid out, with its distance from the centre of its area and the
    /// key and record to keep it by; a street with no key is not here.
    pub(super) fn edited_streets(&self) -> Vec<(f64, String, StreetRecord)> {
        let changed = self.streets.iter().filter(|(uid, s)| self.today_streets.get(uid) != Some(s));
        changed
            .filter_map(|(&uid, _)| {
                let (key, record) = self.street_record(uid)?;
                Some((self.layout.centre_distance_m(At::Edge(self.edge_index(uid)?)), key, record))
            })
            .collect()
    }

    /// The same for junctions (the label is not an edit).
    pub(super) fn edited_junctions(&self) -> Vec<(f64, String, JunctionRecord)> {
        let changed = self.junctions.iter().filter(|(uid, s)| self.today_junctions.get(uid).is_none_or(|t| !same_junction(t, s)));
        changed
            .filter_map(|(&uid, _)| {
                let (key, record) = self.junction_record(uid)?;
                Some((self.layout.centre_distance_m(At::Node(self.node_index(uid)?)), key, record))
            })
            .collect()
    }

    /// What an older version kept the city as, to build old saves in tests.
    #[cfg(test)]
    pub(super) fn legacy_save(&self) -> String {
        let saved = Saved { version: SAVE_VERSION, streets: self.streets.clone(), junctions: self.junctions.clone() };
        serde_json::to_string(&saved).expect("the city serialises")
    }

    /// Which of the streets at the junction at `node` is the one a record's arm names: the one with its key, or
    /// failing that the only one made from one of its ways that leaves the junction the same way. None when no
    /// street, or more than one, is.
    fn arm_street(&self, node: usize, street: &ArmStreet) -> Option<usize> {
        let at = self.layout.edges_at(node);
        if let Some(key) = &street.key {
            let mut exact = at.iter().copied().filter(|&e| self.street_keys[e].as_ref() == Some(key));
            if let (Some(e), None) = (exact.next(), exact.next()) {
                return Some(e);
            }
        }
        let mut near = at.iter().copied().filter(|&e| {
            !super::keys::ids(&self.layout.edges[e].section.source).is_disjoint(&street.ways)
                && degrees_apart(self.layout.leaving_deg(e, node).round() as i32, street.leaving) < SAME_WAY_DEG
        });
        match (near.next(), near.next()) {
            (Some(e), None) => Some(e),
            _ => None,
        }
    }

    /// The key and the record to keep for street `edge` as it now stands; None where the street has no key, or one
    /// another place shares, so that what is written is always what comes back.
    pub fn street_record(&self, edge: u32) -> Option<(String, StreetRecord)> {
        let e = self.edge_index(edge)?;
        let key = self.street_keys[e].clone()?;
        let ids = self.layout.street_ids(e, None);
        Some((key, StreetRecord { ways: ids.ways, nodes: ids.nodes, leaving: ids.leaving, street: self.streets.get(&edge)?.clone() }))
    }

    /// The key and the record to keep for the junction at `node` as it now stands; None where it has no key of its
    /// own, or an arm's street could not be found again from what the record says of it.
    pub fn junction_record(&self, node: u32) -> Option<(String, JunctionRecord)> {
        let n = self.node_index(node).filter(|&n| self.layout.nodes[n].junction)?;
        let key = self.junction_keys[n].clone()?;
        let mut state = self.junctions.get(&node)?.clone();
        let mut arm_streets = BTreeMap::new();
        for arm in &mut state.arms {
            let e = self.edge_index(arm.edge)?;
            let ways = super::keys::ids(&self.layout.edges[e].section.source);
            let street = ArmStreet { key: self.street_keys[e].clone(), ways, leaving: self.layout.leaving_deg(e, n).round() as i32 % 360 };
            // the arm must find this street again, and no other, when the record is read
            if self.arm_street(n, &street) != Some(e) {
                return None;
            }
            arm_streets.insert(arm.uid, street);
            arm.edge = 0;
        }
        Some((key, JunctionRecord { nodes: self.layout.junction_ids(n, None).nodes, state, arm_streets }))
    }

    /// The key of the record street `edge` took its edit from when the city was loaded, if it took one. When the
    /// street is written under a key of its own that differs (the record was found by its ids), that record is
    /// the one it replaces.
    pub fn took_street(&self, edge: u32) -> Option<&str> {
        self.took_streets.get(&edge).map(String::as_str)
    }

    /// The same for the junction at `node`.
    pub fn took_junction(&self, node: u32) -> Option<&str> {
        self.took_junctions.get(&node).map(String::as_str)
    }

    fn edge_index(&self, uid: u32) -> Option<usize> {
        (uid as usize).checked_sub(1).filter(|&e| e < self.layout.edges.len())
    }

    fn node_index(&self, uid: u32) -> Option<usize> {
        (uid as usize).checked_sub(1).filter(|&n| n < self.layout.nodes.len())
    }

    pub fn junction_name(&self, node: u32) -> Option<Said> {
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

    /// The centreline of a street, in layout millimetres: its own shape, or the straight line between its ends.
    pub fn street_shape(&self, edge: u32) -> Option<Vec<[i32; 2]>> {
        let e = self.edge_index(edge)?;
        Some(self.layout.shape_of(e).into_iter().map(|(x, y)| [x, y]).collect())
    }

    /// Where the layout's origin lies in the network it was made from (`CityView::origin_m`), without drawing the city.
    pub fn origin_m(&self) -> Option<[f64; 2]> {
        self.layout.origin_m.map(|(x, y)| [x, y])
    }

    /// The street editor on one street of the city.
    pub fn street_editor(&self, edge: u32, region: usize) -> Option<Editor> {
        self.edge_index(edge)?;
        Some(Editor::from_street(self.today_streets.get(&edge)?, self.streets.get(&edge)?, region))
    }

    /// Keeps what the street editor has made of a street.
    pub fn keep_street(&mut self, edge: u32, street: Street) -> bool {
        let ok = self.today_streets.get(&edge).is_some_and(|t| t.class == street.class && t.row_mm == street.row_mm);
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

    /// The checks a junction fails as the city first laid it out, by id. A real city's streets
    /// fall short of the rules as they stand; only what a change adds is flagged.
    fn failing_today(&self, node: u32, region: usize) -> Vec<&'static str> {
        let Some(n) = self.node_index(node) else { return Vec::new() };
        let Some(state) = self.today_junctions.get(&node) else { return Vec::new() };
        let today = self.with_streets_of(&self.today_streets, n, state);
        Junction::from_city(junction::unlabelled(), &today, &today, region)
            .map_or_else(Vec::new, |j| j.view().checks.iter().filter(|c| !c.ok).map(|c| c.id).collect())
    }

    /// The junction editor on one junction of the city, reading the streets as
    /// they now stand. None when it cannot be drawn with them.
    pub fn junction_editor(&self, node: u32, region: usize) -> Option<Junction> {
        let n = self.node_index(node).filter(|&n| self.layout.nodes[n].junction)?;
        let today = self.with_streets(n, self.today_junctions.get(&node)?);
        let now = self.with_streets(n, self.junctions.get(&node)?);
        Junction::from_city(self.layout.node_name(n), &today, &now, region)
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
            self.layout.edges_at(n).iter().map(|&e| self.today_streets[&(e as u32 + 1)].row_mm / 2).max().unwrap_or(0)
        };
        for (i, e) in self.layout.edges.iter().enumerate() {
            let uid = i as u32 + 1;
            let today = &self.today_streets[&uid];
            let now = &self.streets[&uid];
            let editor = Editor::from_street(today, now, region);
            let v = editor.view();
            let at_first: Vec<&str> = Editor::from_street(today, today, region).view().checks.iter().filter(|c| !c.ok).map(|c| c.id).collect();
            let failing: Vec<Said> = v.checks.iter().filter(|c| !c.ok && !at_first.contains(&c.id)).map(|c| c.label.clone()).collect();
            let row = v.row_mm;
            let shape = self.layout.shape_of(i);
            edges.push(EdgeView {
                uid,
                a: node_uid(e.a),
                b: node_uid(e.b),
                called: self.layout.edge_title(i),
                ends: self.layout.edge_ends(i),
                row_mm: row,
                total_mm: v.total_mm,
                length_mm: path_mm(&shape).round() as i32,
                shape_mm: shape.iter().map(|&(x, y)| [x, y]).collect(),
                trim_a_mm: trim_at(e.a),
                trim_b_mm: trim_at(e.b),
                freeway: e.class.is_freeway(),
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
                v.control = Some(junction::control_key(now.control));
                match self.junction_editor(uid, region) {
                    Some(j) => {
                        let at_first = self.failing_today(uid, region);
                        v.failing = j.view().checks.iter().filter(|c| !c.ok && !at_first.contains(&c.id)).map(|c| c.label.clone()).collect();
                        v.ok = v.failing.is_empty();
                    }
                    None => {
                        v.ok = false;
                        v.failing = vec![Said::new("city-cannot-draw")];
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
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct EndView {
    pub uid: u32,
    pub name: Said,
    pub junction: bool,
}

#[derive(Serialize)]
pub struct EdgeView {
    pub uid: u32,
    pub a: u32,
    pub b: u32,
    /// What the street is called: its own name, or what sort of street it is.
    pub called: Said,
    /// Where it runs: from one place to another, or through the city.
    pub ends: Said,
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
    pub failing: Vec<Said>,
}

impl EdgeView {
    /// The street's whole name: what it is called and where it runs.
    pub fn name(&self) -> Said {
        Said::new("city-edge-name").with("street", Arg::Said(Box::new(self.called.clone()))).with("ends", Arg::Said(Box::new(self.ends.clone())))
    }
}

#[derive(Serialize)]
pub struct NodeView {
    pub uid: u32,
    pub name: Said,
    /// Which junction it is, counting in reading order from 1; 0 where a street leaves the map.
    pub number: u32,
    pub x_mm: i32,
    pub y_mm: i32,
    /// A junction of streets, or else where a street leaves the map.
    pub junction: bool,
    pub radius_mm: i32,
    /// The message that names the junction's control (`junction::control_key`).
    pub control: Option<&'static str>,
    pub arms: usize,
    pub edited: bool,
    pub ok: bool,
    pub failing: Vec<Said>,
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
    use crate::shared::said::Arg;

    #[test]
    fn every_place_of_the_sample_city_has_a_key_and_no_two_places_share_one() {
        let layout = Layout::sample();
        let mut keys: Vec<Option<String>> = (0..layout.nodes.len()).filter(|&n| layout.nodes[n].junction).map(|n| layout.node_key(n)).collect();
        keys.extend(layout.edge_keys());
        assert!(keys.iter().all(Option::is_some));
        let distinct: std::collections::BTreeSet<_> = keys.iter().collect();
        assert_eq!(distinct.len(), keys.len());
    }

    fn junction_number(node: usize) -> usize {
        Layout::sample().junction_number(node)
    }

    fn edges_at(node: usize) -> Vec<usize> {
        Layout::sample().edges_at(node)
    }

    use crate::shared::testing::{en, fr};

    /// The sample layout with names given to some of its streets, by edge index.
    fn named(names: &[(usize, &str)]) -> Layout {
        let mut layout = Layout::sample();
        for &(e, name) in names {
            layout.edges[e].name = Some(name.to_string());
        }
        layout
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
        let bad: Vec<_> = v
            .nodes
            .iter()
            .filter(|n| !n.ok)
            .map(|n| (en(&n.name), &n.failing))
            .chain(v.edges.iter().filter(|e| !e.ok).map(|e| (en(&e.name()), &e.failing)))
            .collect();
        assert!(bad.is_empty(), "{bad:?}");
        assert_eq!((v.failing, v.edited), (0, 0));
        assert_eq!(v.places, 9 + 23);
    }

    #[test]
    fn the_first_city_works_on_the_other_side_of_the_road_too() {
        let left = REGIONS.iter().position(|r| r.drive_side == Side::Left).unwrap();
        let v = City::new().view(left);
        assert_eq!(v.failing, 0, "{:?}", v.nodes.iter().filter(|n| !n.ok).map(|n| (en(&n.name), &n.failing)).collect::<Vec<_>>());
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

    /// What an older version kept, read back as the migration reads it.
    fn old_load(json: &str) -> City {
        City::load_old(Layout::sample(), json)
    }

    /// A copy of street `uid` as first laid out with 100 mm moved from its first piece to its second, so that its
    /// class and total width, which an edit must keep, are unchanged.
    fn narrowed(city: &City, uid: u32) -> Street {
        let mut s = city.today_streets[&uid].clone();
        s.segments[0].width_mm -= 100;
        s.segments[1].width_mm += 100;
        s
    }

    /// Every place of `city` that differs from first laid out, as the store would keep it.
    fn edits_of(city: &City) -> Edits {
        let mut e = Edits::new();
        for uid in 1..=city.layout.edges.len() as u32 {
            if city.streets[&uid] != city.today_streets[&uid]
                && let Some((key, record)) = city.street_record(uid)
            {
                e.streets.insert(key, record);
            }
        }
        for (&uid, state) in &city.junctions {
            if !same_junction(state, &city.today_junctions[&uid])
                && let Some((key, record)) = city.junction_record(uid)
            {
                e.junctions.insert(key, record);
            }
        }
        e
    }

    /// The sample layout with its streets and its places in the other order, as a refreshed tile or importer may
    /// give them: the same roads, other positions.
    fn reordered() -> Layout {
        let mut layout = Layout::sample();
        let last = layout.nodes.len() - 1;
        layout.nodes.reverse();
        for e in &mut layout.edges {
            (e.a, e.b) = (last - e.a, last - e.b);
        }
        layout.edges.reverse();
        layout
    }

    #[test]
    fn what_the_editors_keep_comes_back_through_the_edits() {
        let mut city = City::new();
        let street = narrowed(&city, 1);
        assert!(city.keep_street(1, street.clone()));
        let mut state = city.junctions[&node_uid(1)].clone();
        state.ring_extra_mm = 500;
        assert!(city.keep_junction(node_uid(1), state));
        let back = City::load(&edits_of(&city));
        assert_eq!(back.streets[&1], street);
        assert_eq!(back.junctions[&node_uid(1)].ring_extra_mm, 500);
        assert_eq!(back.streets[&2], back.today_streets[&2], "what was not edited is as first laid out");
        assert_eq!(back.view(0).edited, 2);
    }

    #[test]
    fn an_edit_stays_with_its_street_when_the_layout_is_reordered() {
        // The defect: positions shift when a tile or the importer is refreshed. The old positional load would
        // give the edit to whatever street now has uid 1.
        let mut city = City::new();
        let street = narrowed(&city, 1);
        assert!(city.keep_street(1, street.clone()));
        let edits = edits_of(&city);
        let mut layout = Layout::sample();
        layout.edges.reverse();
        let back = City::load_on(layout, &edits);
        let last = back.layout.edges.len() as u32; // street 1 is now the last
        assert_eq!(back.streets[&last], street, "the edit followed its street");
        for other in 1..last {
            assert_eq!(back.streets[&other], back.today_streets[&other], "no other street took an edit (street {other})");
        }
    }

    #[test]
    fn an_edit_whose_street_is_gone_is_not_applied_and_the_record_is_still_there_to_try_again() {
        let mut city = City::new();
        assert!(city.keep_street(1, narrowed(&city, 1)));
        let edits = edits_of(&city);
        let mut layout = Layout::sample();
        layout.edges[0].section.source = vec![OsmRef { id: 9_999_999, version: None }]; // another way now
        let back = City::load_on(layout, &edits);
        assert_eq!(back.streets[&1], back.today_streets[&1]);
        assert_eq!(edits.streets.len(), 1, "loading does not take a record out of the document");
        // and the street comes back: the record applies again
        assert_eq!(City::load_on(Layout::sample(), &edits).streets[&1], narrowed(&City::new(), 1));
    }

    #[test]
    fn a_street_with_the_same_key_but_another_width_does_not_take_the_edit() {
        let mut city = City::new();
        assert!(city.keep_street(1, narrowed(&city, 1)));
        let mut edits = edits_of(&city);
        for record in edits.streets.values_mut() {
            record.street.row_mm += 100; // the way is wider now: the saved section no longer fits it
        }
        let back = City::load(&edits);
        assert_eq!(back.streets[&1], back.today_streets[&1]);
    }

    #[test]
    fn a_street_of_another_class_does_not_take_the_edit() {
        let mut city = City::new();
        assert!(city.keep_street(1, narrowed(&city, 1)));
        let mut edits = edits_of(&city);
        for record in edits.streets.values_mut() {
            record.street.class = StreetClass::ALL.into_iter().find(|&c| c != record.street.class).unwrap(); // only the class differs
        }
        let back = City::load(&edits);
        assert_eq!(back.streets[&1], back.today_streets[&1]);
    }

    /// The sample city with an edited junction 1, its edits, and the junction's node index.
    fn edited_junction() -> (Edits, usize) {
        let mut city = City::new();
        let mut state = city.junctions[&node_uid(1)].clone();
        state.ring_extra_mm = 500;
        assert!(city.keep_junction(node_uid(1), state));
        let edits = edits_of(&city);
        assert_eq!(edits.junctions.len(), 1);
        (edits, 1)
    }

    fn junction_is_as_first_laid_out(back: &City) -> bool {
        back.junctions[&node_uid(1)].ring_extra_mm == back.today_junctions[&node_uid(1)].ring_extra_mm
    }

    #[test]
    fn a_junction_record_that_does_not_say_which_street_an_arm_is_on_is_not_applied() {
        let (mut edits, _) = edited_junction();
        for record in edits.junctions.values_mut() {
            let uid = *record.arm_streets.keys().next().unwrap();
            record.arm_streets.remove(&uid);
        }
        assert!(junction_is_as_first_laid_out(&City::load(&edits)));
    }

    /// The sample layout with a copy of street 1 somewhere that is not junction 1 (the same way and end nodes).
    fn with_a_twin_of_street_1_elsewhere() -> Layout {
        let mut layout = Layout::sample();
        let (a, b) = (layout.edges[0].a, layout.edges[0].b);
        let copy = |n: &NodeDef| NodeDef { x_mm: n.x_mm, y_mm: n.y_mm, junction: false, control: 0, corner_mm: 0, source: n.source.clone() };
        let (na, nb) = (copy(&layout.nodes[a]), copy(&layout.nodes[b]));
        layout.nodes.push(na);
        layout.nodes.push(nb);
        let n = layout.nodes.len();
        let e = &layout.edges[0];
        let twin =
            EdgeDef { a: n - 2, b: n - 1, class: e.class, name: e.name.clone(), section: e.section.clone(), headings: e.headings, shape: e.shape.clone() };
        layout.edges.push(twin);
        layout
    }

    fn clone_layout(l: &Layout) -> Layout {
        let node = |n: &NodeDef| NodeDef { source: n.source.clone(), ..*n };
        let edge = |e: &EdgeDef| EdgeDef { name: e.name.clone(), section: e.section.clone(), shape: e.shape.clone(), ..*e };
        Layout {
            name: l.name.clone(),
            side: l.side,
            nodes: l.nodes.iter().map(node).collect(),
            edges: l.edges.iter().map(edge).collect(),
            origin_m: l.origin_m,
        }
    }

    #[test]
    fn two_streets_with_the_same_key_have_none_and_are_neither_written_nor_given_an_edit() {
        let mut city = City::new();
        assert!(city.keep_street(1, narrowed(&city, 1)));
        let edits = edits_of(&city);
        let layout = with_a_twin_of_street_1_elsewhere();
        let keys = layout.edge_keys();
        let last = keys.len() - 1;
        assert_eq!(keys[0], keys[last], "the twin has the same key, the way it leaves its first end too");
        let mut back = City::load_on(layout, &edits);
        assert_eq!(back.streets[&1], back.today_streets[&1], "neither takes the edit by its key");
        assert!(back.street_record(1).is_none() && back.street_record(last as u32 + 1).is_none(), "and neither can be written");
        assert!(back.keep_street(1, narrowed(&back, 1)));
        assert!(back.street_record(1).is_none(), "an edit that could not come back is refused, not lost");
    }

    #[test]
    fn a_junction_arm_finds_its_street_among_those_at_the_junction_whatever_other_street_has_its_key() {
        let (edits, _) = edited_junction();
        assert!(!junction_is_as_first_laid_out(&City::load_on(with_a_twin_of_street_1_elsewhere(), &edits)));
    }

    #[test]
    fn a_junction_arm_whose_street_key_changed_finds_its_street_by_its_way_and_the_way_it_leaves() {
        // kept where the arm's street was `w2000~n1000~n1001`; here the box cuts it, so its key is another
        let (edits, node) = edited_junction();
        let mut layout = Layout::sample();
        let far = layout.edges[0].a;
        assert_ne!(far, node);
        layout.nodes[far].source.clear();
        assert!(layout.edge_keys()[0].as_deref().is_some_and(|k| k.contains("~x~")), "{:?}", layout.edge_keys()[0]);
        assert!(!junction_is_as_first_laid_out(&City::load_on(layout, &edits)));
    }

    #[test]
    fn a_junction_kept_where_an_arm_street_was_cut_at_the_box_comes_back_where_the_street_is_whole() {
        let mut cut = Layout::sample();
        let far = cut.edges[0].a;
        cut.nodes[far].source.clear();
        let mut city = City::on(cut);
        let mut state = city.junctions[&node_uid(1)].clone();
        state.ring_extra_mm = 500;
        assert!(city.keep_junction(node_uid(1), state));
        let edits = edits_of(&city);
        let arms: Vec<&ArmStreet> = edits.junctions.values().flat_map(|r| r.arm_streets.values()).collect();
        assert!(arms.iter().any(|a| a.key.as_deref().is_some_and(|k| k.contains("~x~"))), "{arms:?}");
        assert!(!junction_is_as_first_laid_out(&City::load(&edits)));
    }

    #[test]
    fn a_junction_arm_without_a_key_is_found_by_its_way_and_bearing_and_not_on_a_street_that_leaves_another_way() {
        let (mut edits, _) = edited_junction();
        for record in edits.junctions.values_mut() {
            for street in record.arm_streets.values_mut() {
                street.key = None; // only the way and the bearing say which street it is
            }
        }
        assert!(!junction_is_as_first_laid_out(&City::load(&edits)), "found again by way and bearing");
        let mut turned = edits.clone();
        for record in turned.junctions.values_mut() {
            let street = record.arm_streets.values_mut().next().unwrap();
            street.leaving = (street.leaving + 90) % 360;
        }
        assert!(junction_is_as_first_laid_out(&City::load(&turned)), "the street of the way leaves the junction another way");
    }

    #[test]
    fn of_two_streets_of_one_way_at_a_junction_an_arm_takes_the_one_leaving_its_way_and_neither_if_both_do() {
        let node = 1;
        let mut layout = Layout::sample();
        let at = layout.edges_at(node);
        let way = layout.edges[at[0]].section.source.clone();
        layout.edges[at[1]].section.source = way.clone(); // the way runs on through the junction
        let leaving = layout.leaving_deg(at[0], node).round() as i32;
        let street = ArmStreet { key: None, ways: super::super::keys::ids(&way), leaving };
        assert_eq!(City::on(clone_layout(&layout)).arm_street(node, &street), Some(at[0]));
        // the other street of the way now leaves the junction almost as the first does
        let far = layout.other_end(at[1], node);
        let near = &layout.nodes[layout.other_end(at[0], node)];
        (layout.nodes[far].x_mm, layout.nodes[far].y_mm) = (near.x_mm + 1_000, near.y_mm - 5_000);
        assert_eq!(City::on(layout).arm_street(node, &street), None);
    }

    #[test]
    fn a_junction_arm_whose_street_key_names_a_street_that_does_not_meet_it_is_not_applied() {
        let (mut edits, node) = edited_junction();
        let layout = Layout::sample();
        let at = layout.edges_at(node);
        let away = (0..layout.edges.len()).find(|e| !at.contains(e)).expect("a street that does not meet the junction");
        let key = layout.edge_keys()[away].clone();
        let ways = super::super::keys::ids(&layout.edges[away].section.source);
        for record in edits.junctions.values_mut() {
            let street = record.arm_streets.values_mut().next().unwrap();
            (street.key, street.ways) = (key.clone(), ways.clone());
        }
        assert!(junction_is_as_first_laid_out(&City::load_on(layout, &edits)));
    }

    #[test]
    fn a_street_without_a_key_stays_as_first_laid_out_while_the_others_take_their_edits() {
        let mut city = City::new();
        assert!(city.keep_street(2, narrowed(&city, 2)));
        let mut state = city.junctions[&node_uid(1)].clone();
        state.ring_extra_mm = 500;
        assert!(city.keep_junction(node_uid(1), state));
        let edits = edits_of(&city);
        let mut layout = Layout::sample();
        layout.edges[0].section.source.clear(); // street 1 has no way
        let back = City::load_on(layout, &edits);
        assert_eq!(back.streets[&1], back.today_streets[&1], "the street with no key is left as it was");
        assert_eq!(back.streets[&2], narrowed(&City::new(), 2), "another street takes its edit");
        assert!(junction_is_as_first_laid_out(&back), "a junction with an arm on a street that has no key cannot find it again");
    }

    #[test]
    fn a_junction_comes_back_from_the_edits_document_with_the_label_a_saved_state_has() {
        let (edits, _) = edited_junction();
        // what is kept comes back as the earlier changes: the label is not stored, and loading says the same
        let text = edits.to_json();
        let crate::city::edits::Document::Current(read) = crate::city::edits::parse(Some(&text)) else { panic!("not read back") };
        assert_eq!(read.junctions.values().next().unwrap().state.label, junction::earlier());
        assert_eq!(City::load(&read).junctions[&node_uid(1)].label, junction::earlier());
    }

    #[test]
    fn a_junction_edit_comes_back_with_its_arms_on_the_right_streets() {
        let mut city = City::new();
        let mut state = city.junctions[&node_uid(1)].clone();
        state.ring_extra_mm = 500;
        assert!(city.keep_junction(node_uid(1), state));
        let edits = edits_of(&city);
        let back = City::load_on(reordered(), &edits);
        // node index 1 is now at the other end of the table
        let node = back.layout.nodes.len() - 2;
        let uid = node_uid(node);
        assert_eq!(back.junctions[&uid].ring_extra_mm, 500);
        let mut arms: Vec<u32> = back.junctions[&uid].arms.iter().map(|a| a.edge).collect();
        arms.sort_unstable();
        let mut want: Vec<u32> = back.layout.edges_at(node).iter().map(|&e| e as u32 + 1).collect();
        want.sort_unstable();
        assert_eq!(arms, want, "every arm is on a street that meets the junction");
        // and each arm is on the street it was on: the same key
        let was: BTreeMap<u32, ArmStreet> = edits.junctions.values().next().unwrap().arm_streets.clone();
        let keys = back.layout.edge_keys();
        for arm in &back.junctions[&uid].arms {
            assert_eq!(keys[arm.edge as usize - 1], was[&arm.uid].key);
        }
    }

    #[test]
    fn a_junction_whose_arm_street_is_no_longer_there_is_not_applied() {
        let mut city = City::new();
        let mut state = city.junctions[&node_uid(1)].clone();
        state.ring_extra_mm = 500;
        assert!(city.keep_junction(node_uid(1), state));
        let mut edits = edits_of(&city);
        for record in edits.junctions.values_mut() {
            if let Some(street) = record.arm_streets.values_mut().next() {
                // a street the city does not have
                (street.key, street.ways) = (Some("w1~n1~n2".to_string()), [1].into());
            }
        }
        let back = City::load(&edits);
        assert_eq!(back.junctions[&node_uid(1)].ring_extra_mm, back.today_junctions[&node_uid(1)].ring_extra_mm);
    }

    #[test]
    fn a_place_without_a_key_has_no_record() {
        let mut layout = Layout::sample();
        layout.edges[0].section.source.clear();
        let mut city = City::on(layout);
        assert!(city.keep_street(1, narrowed(&city, 1)));
        assert!(city.street_record(1).is_none(), "a street with no way has no key");
        assert!(city.street_record(2).is_some());
        assert!(city.junction_record(node_uid(0)).is_none(), "an end of a street is not a junction");
    }

    #[test]
    fn a_junction_record_stores_arms_without_edges_and_names_their_streets() {
        let city = City::new();
        let (key, record) = city.junction_record(node_uid(1)).unwrap();
        assert_eq!(Some(key), city.layout.node_key(1));
        assert!(record.state.arms.iter().all(|a| a.edge == 0));
        assert_eq!(record.arm_streets.len(), record.state.arms.len());
        assert_eq!(record.nodes, [1001].into());
    }

    #[test]
    fn a_street_record_carries_the_ids_of_the_street_and_of_both_its_ends() {
        let city = City::new();
        let (key, record) = city.street_record(1).unwrap();
        assert_eq!(key, "w2000~n1000~n1001");
        assert_eq!((record.ways, record.nodes), ([2000].into(), [1000, 1001].into()));
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

        let back = old_load(&city.legacy_save());
        assert_eq!(back.streets, city.streets);
        // A junction comes back the same but for its history label, which is not kept.
        assert_eq!(back.junctions.keys().collect::<Vec<_>>(), city.junctions.keys().collect::<Vec<_>>());
        assert!(back.junctions.iter().all(|(k, s)| same_junction(s, &city.junctions[k]) && s.label == junction::earlier()));
        let v = back.view(0);
        assert!(v.edges[0].edited && v.nodes[1].edited);
        assert_eq!(v.edited, 2);
        // The editors reopen with the earlier changes as one revision, and Start over undoes them, in each editor.
        let mut e = back.street_editor(1, 0).unwrap();
        assert!(e.view().changed);
        assert!(e.reset());
        let mut j = back.junction_editor(node_uid(1), 0).unwrap();
        assert!(j.changed());
        assert!(j.reset());
    }

    #[test]
    fn a_damaged_or_foreign_save_starts_the_city_over() {
        let fresh = City::new().legacy_save();
        for bad in ["", "not json", "{}", "[]", r#"{"version":99,"streets":{},"junctions":{}}"#] {
            assert_eq!(old_load(bad).legacy_save(), fresh, "{bad:?}");
        }
        // A street with an index outside the catalogue is dropped; the rest is kept.
        let mut city = City::new();
        let mut s = city.streets[&3].clone();
        s.segments[0].width_mm = 3_000;
        city.keep_street(3, s);
        let mut saved: serde_json::Value = serde_json::from_str(&city.legacy_save()).unwrap();
        saved["streets"]["1"]["segments"][0]["kind"] = serde_json::json!(99);
        saved["junctions"]["2"]["control"] = serde_json::json!(99);
        let back = old_load(&saved.to_string());
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
        assert_eq!(
            ends.iter().map(|e| (en(&e.name), e.junction)).collect::<Vec<_>>(),
            [("the edge of the map".to_string(), false), ("Junction 4".to_string(), true)]
        );
        assert!(City::new().street_ends(99).is_empty());
    }

    #[test]
    fn a_save_from_before_street_classes_is_left_behind() {
        let mut city = City::new();
        let uid = city.view(0).edges[0].uid;
        let mut e = city.street_editor(uid, 0).unwrap();
        let piece = e.view().segments[0].uid;
        e.nudge_width(piece, 100);
        assert!(city.keep_street(uid, e.snapshot()));
        let edited = city.view(0).edited;
        assert!(edited > 0);
        let json = city.legacy_save();
        assert_eq!(old_load(&json).view(0).edited, edited, "a save of this version is kept");
        let old = json.replace(&format!("\"version\":{SAVE_VERSION}"), "\"version\":1");
        assert_eq!(old_load(&old).view(0).edited, 0, "a version 1 save is not opened");
    }

    #[test]
    fn names_say_where_a_street_goes() {
        let layout = Layout::sample();
        assert_eq!(en(&layout.edge_title(0)), "Sample Avenue 2");
        assert_eq!(en(&layout.edge_ends(0)), "the edge of the map to Junction 4");
        assert_eq!(fr(&layout.edge_ends(0)), "entre la limite de la carte et Jonction 4");
        assert_eq!(en(&layout.edge_ends(22)), "through the city");
        assert_eq!(fr(&layout.edge_ends(22)), "à travers la ville");
        assert_eq!(layout.edge_ends(22), Said::new("city-edge-through"));
        let v = City::new().view(0);
        assert_eq!(en(&v.edges[0].name()), "Sample Avenue 2 · the edge of the map to Junction 4");
        assert_eq!(en(&v.edges[22].name()), "Sample Freeway 4 · through the city");
        assert_eq!(fr(&v.edges[22].name()), "Sample Freeway 4 · à travers la ville");
    }

    #[test]
    fn a_place_is_titled_by_the_streets_that_meet_there_in_each_language() {
        // node 15 meets edges 5, 16, 19, 20 and 21; the sample city's streets have no names of their own
        let five = [5, 16, 19, 20, 21];
        let names = ["A", "B", "C", "D", "E"];
        let title = |k: usize| named(&five.iter().zip(names).take(k).map(|(&e, n)| (e, n)).collect::<Vec<_>>()).node_name(15);
        assert_eq!(title(0), Said::new("city-junction-number").with("n", Arg::Num(7)));
        assert_eq!(title(1), Said::new("city-junction-of-one").with("a", Arg::Text("A".into())));
        assert_eq!(title(2).key, "city-junction-of-two");
        assert_eq!(title(3).key, "city-junction-of-three");
        assert_eq!(title(5), Said::new("city-junction-of-many").with("a", Arg::Text("A".into())).with("b", Arg::Text("B".into())).with("rest", Arg::Num(3)));
        let english: Vec<String> = [0, 1, 2, 3, 5].into_iter().map(|k| en(&title(k))).collect();
        assert_eq!(english, ["Junction 7", "A junction", "A and B", "A, B and C", "A, B and 3 more"]);
        let french: Vec<String> = [0, 1, 2, 3, 5].into_iter().map(|k| fr(&title(k))).collect();
        assert_eq!(french, ["Jonction 7", "Jonction A", "A et B", "A, B et C", "A, B et 3 autres"]);
        assert!(french.iter().all(|t| !t.contains("Junction") && !t.contains(" and ")), "{french:?}");
        // a junction is said the same in a sentence
        assert_eq!(named(&[(5, "Main Street"), (16, "Side Road")]).end_name(15), named(&[(5, "Main Street"), (16, "Side Road")]).node_name(15));
        assert_eq!(en(&named(&[(5, "Main Street"), (16, "Side Road")]).node_name(15)), "Main Street and Side Road");

        // the end of a named street, where it leaves the map
        let lane = named(&[(19, "Lane X")]);
        assert_eq!(lane.node_name(16), Said::new("city-end-of").with("street", Arg::Text("Lane X".into())));
        assert_eq!((en(&lane.node_name(16)), fr(&lane.node_name(16))), ("End of Lane X".into(), "Bout de Lane X".into()));
        assert_eq!((en(&lane.end_name(16)), fr(&lane.end_name(16))), ("the end of Lane X".into(), "le bout de Lane X".into()));
        // a meeting the junction editor cannot draw is a connection on its street
        let mut meeting = named(&[(0, "Avenue Y")]);
        meeting.nodes[1].junction = false;
        assert_eq!((en(&meeting.node_name(1)), fr(&meeting.node_name(1))), ("Connection on Avenue Y".into(), "Raccordement sur Avenue Y".into()));
        assert_eq!(en(&meeting.end_name(1)), "a connection on Avenue Y");
        // an end where no road has wording of its own to lend (the sample city's streets are named in their
        // sections, not by the network) is the edge of the map
        let layout = Layout::sample();
        assert_eq!(layout.node_name(0), Said::new("city-map-edge"));
        assert_eq!((en(&layout.node_name(0)), fr(&layout.node_name(0))), ("Edge of the map".into(), "Limite de la carte".into()));
        assert_eq!(en(&layout.end_name(0)), "the edge of the map");
        assert_eq!(City::new().junction_name(node_uid(1)).map(|n| en(&n)), Some("Junction 4".to_string()));
        assert_eq!(City::new().junction_name(node_uid(0)), None, "an end of a street is not a junction");
    }

    #[test]
    fn what_fails_is_said_in_the_language_of_the_map() {
        let mut city = City::new();
        let mut s = city.streets[&2].clone();
        s.segments.retain(|g| KINDS[g.kind].id == "sidewalk");
        assert!(city.keep_street(2, s));
        let v = city.view(0);
        let failing = &v.edges[1].failing;
        assert!(!failing.is_empty());
        assert!(failing.iter().all(|f| en(f) != fr(f)), "{failing:?}");
    }

    #[test]
    fn a_saved_junction_without_a_raised_table_loads_and_one_with_it_keeps_it() {
        let mut city = City::new();
        let mut j = city.junction_editor(node_uid(1), 0).unwrap();
        assert!(j.set_raised(true));
        assert!(city.keep_junction(node_uid(1), j.snapshot()));
        let saved: serde_json::Value = serde_json::from_str(&city.legacy_save()).unwrap();
        assert_eq!(saved["junctions"]["2"]["raised"], serde_json::json!(true));
        let back = old_load(&saved.to_string());
        assert!(back.junctions[&node_uid(1)].raised, "a table is kept");

        let mut older = saved.clone();
        older["junctions"]["2"].as_object_mut().unwrap().remove("raised");
        let back = old_load(&older.to_string());
        assert!(back.junctions.contains_key(&node_uid(1)), "the save is not discarded");
        assert!(!back.junctions[&node_uid(1)].raised, "a save made before the table has none");
    }

    #[test]
    fn a_saved_crossing_without_the_continuous_field_loads_and_one_with_it_keeps_it() {
        let mut city = City::new();
        let mut j = city.junction_editor(node_uid(1), 0).unwrap();
        let arm = j.current().arms.iter().find(|a| a.crossing.is_some()).expect("a street with a crossing").uid;
        assert!(j.set_continuous(arm, true));
        assert!(city.keep_junction(node_uid(1), j.snapshot()));
        let saved: serde_json::Value = serde_json::from_str(&city.legacy_save()).unwrap();
        let continuous: Vec<bool> = saved["junctions"]["2"]["arms"].as_array().unwrap().iter().filter_map(|a| a["crossing"]["continuous"].as_bool()).collect();
        assert!(continuous.contains(&true), "it is saved: {continuous:?}");
        let back = old_load(&saved.to_string());
        assert!(back.junctions[&node_uid(1)].arms.iter().any(|a| a.crossing.is_some_and(|c| c.continuous)));

        let mut older = saved.clone();
        for a in older["junctions"]["2"]["arms"].as_array_mut().unwrap() {
            if let Some(c) = a["crossing"].as_object_mut() {
                c.remove("continuous");
            }
        }
        let back = old_load(&older.to_string());
        assert!(back.junctions.contains_key(&node_uid(1)), "the save is not discarded");
        assert!(back.junctions[&node_uid(1)].arms.iter().all(|a| !a.crossing.is_some_and(|c| c.continuous)));
    }

    #[test]
    fn a_save_made_before_the_bike_box_loads_with_none() {
        let mut city = City::new();
        let mut j = city.junction_editor(node_uid(1), 0).unwrap();
        let arm = j.current().arms[0].uid;
        assert!(j.set_corner(arm, 4_500));
        assert!(city.keep_junction(node_uid(1), j.snapshot()));
        let mut saved: serde_json::Value = serde_json::from_str(&city.legacy_save()).unwrap();
        let arms = saved["junctions"]["2"]["arms"].as_array_mut().unwrap();
        assert!(arms.iter().all(|a| a["bike_box"] == serde_json::json!(false)), "the field is saved");
        for a in arms.iter_mut() {
            a.as_object_mut().unwrap().remove("bike_box");
        }
        let back = old_load(&saved.to_string());
        assert!(back.view(0).nodes[1].edited, "the junction is kept, the save is not discarded");
        assert!(back.junctions[&node_uid(1)].arms.iter().all(|a| !a.bike_box));
    }

    #[test]
    fn a_saved_junction_from_the_earlier_version_with_a_label_still_loads() {
        let mut city = City::new();
        let mut j = city.junction_editor(node_uid(1), 0).unwrap();
        let arm = j.current().arms[0].uid;
        assert!(j.set_corner(arm, 4_500));
        assert!(city.keep_junction(node_uid(1), j.snapshot()));
        let mut saved: serde_json::Value = serde_json::from_str(&city.legacy_save()).unwrap();
        assert!(saved["junctions"]["2"].is_object(), "the edited junction is saved");
        assert_eq!(saved["junctions"]["2"].get("label"), None, "a junction's history label is not kept: {}", saved["junctions"]["2"]);
        saved["junctions"]["2"]["label"] = serde_json::json!("Remove Main (N)");
        let back = old_load(&saved.to_string());
        assert!(back.view(0).nodes[1].edited, "the edited junction is kept");
        assert!(same_junction(&back.junctions[&node_uid(1)], &city.junctions[&node_uid(1)]));
    }

    #[test]
    fn a_street_s_shape_and_where_the_layout_lies_can_be_read_without_drawing_the_city() {
        let city = City::from_network(&crate::restriction::fixtures::bent_network(), "Bendville");
        assert_eq!(city.street_shape(1), Some(city.view(0).edges[0].shape_mm.clone()));
        assert_eq!(city.street_shape(99), None);
        assert_eq!(city.origin_m(), city.view(0).origin_m);
        assert_eq!(City::new().origin_m(), None, "the sample city lies nowhere");
    }
}
