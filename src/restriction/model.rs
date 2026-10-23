//! An area restriction: a traffic rule over an outline the resident draws, and which streets it holds. Pure: no
//! DOM, no signals, no storage. The outline is in longitude and latitude, so it does not depend on which tile or
//! area is loaded; the geometry is done in whole micro-degrees, where it is exact.

use serde::{Deserialize, Serialize};

use crate::city::model::EdgeView;
use crate::city::projection::Projection;
use crate::shared::catalogue::{CAR_KINDS, KINDS};
use crate::shared::said::{Arg, Said};
use crate::street::model::Street;

/// A position on the earth: longitude, then latitude, in degrees (GeoJSON's order).
pub type LonLat = [f64; 2];

/// A position in whole micro-degrees, where the geometry is exact.
type Micro = (i64, i64);

fn micro([lon, lat]: LonLat) -> Micro {
    ((lon * 1e6).round() as i64, (lat * 1e6).round() as i64)
}

/// Whether two positions are the same to the micro-degree.
pub fn same_spot(a: LonLat, b: LonLat) -> bool {
    micro(a) == micro(b)
}

/// Twice the signed area of the triangle a, b, c: positive when they turn left.
fn cross(a: Micro, b: Micro, c: Micro) -> i128 {
    (b.0 - a.0) as i128 * (c.1 - a.1) as i128 - (b.1 - a.1) as i128 * (c.0 - a.0) as i128
}

/// Whether `p` lies on the segment a-b, its ends included.
fn on_segment(p: Micro, a: Micro, b: Micro) -> bool {
    cross(a, b, p) == 0 && (a.0.min(b.0)..=a.0.max(b.0)).contains(&p.0) && (a.1.min(b.1)..=a.1.max(b.1)).contains(&p.1)
}

/// The ring's edges, the last joining the first.
fn edges(ring: &[Micro]) -> impl Iterator<Item = (Micro, Micro)> + '_ {
    (0..ring.len()).map(move |i| (ring[i], ring[(i + 1) % ring.len()]))
}

/// Whether `p` is inside the ring, by the even-odd rule; a point on the border counts as inside. The ring is not
/// closed explicitly. A ring across the antimeridian is not supported: it is read the long way round.
pub fn contains(ring: &[LonLat], p: LonLat) -> bool {
    if ring.len() < 3 {
        return false;
    }
    let ring: Vec<Micro> = ring.iter().copied().map(micro).collect();
    let p = micro(p);
    let mut inside = false;
    for (a, b) in edges(&ring) {
        if on_segment(p, a, b) {
            return true;
        }
        // An edge that spans p's latitude (half-open, so a corner on it is counted once) crosses the ray east of
        // p when p is on its left going up, or on its right going down.
        if (a.1 > p.1) != (b.1 > p.1) && (cross(a, b, p) > 0) == (b.1 > a.1) {
            inside = !inside;
        }
    }
    inside
}

/// Whether two segments meet at all: cross, touch or overlap.
fn meet(a: Micro, b: Micro, c: Micro, d: Micro) -> bool {
    let opposite = |x: i128, y: i128| (x > 0 && y < 0) || (x < 0 && y > 0);
    (opposite(cross(c, d, a), cross(c, d, b)) && opposite(cross(a, b, c), cross(a, b, d)))
        || on_segment(a, c, d)
        || on_segment(b, c, d)
        || on_segment(c, a, b)
        || on_segment(d, a, b)
}

/// Whether the ring can be an area: at least three corners not all in a line, no corner used twice, and no edge
/// meeting another except where neighbours share their corner. A refusal says why. O(n²), which outlines drawn
/// by hand can afford on every drop.
pub fn valid(ring: &[LonLat]) -> Result<(), Said> {
    let crossing = || Err(Said::new("restriction-refused-crossing"));
    let r: Vec<Micro> = ring.iter().copied().map(micro).collect();
    let mut distinct = r.clone();
    distinct.sort_unstable();
    distinct.dedup();
    // All in a line: every corner is on the line through the first two distinct ones. (The signed area is no
    // test of that: a bow tie's two lobes cancel to zero.)
    let in_a_line = distinct.len() < 3 || distinct.iter().all(|&p| cross(distinct[0], distinct[1], p) == 0);
    if in_a_line {
        return Err(Said::new("restriction-refused-too-few"));
    }
    if distinct.len() < r.len() {
        return crossing();
    }
    let n = r.len();
    for i in 0..n {
        for j in i + 1..n {
            let (a, b) = (r[i], r[(i + 1) % n]);
            let (c, d) = (r[j], r[(j + 1) % n]);
            if j == i + 1 || (i == 0 && j == n - 1) {
                // Neighbours share one corner; one folding back along the other overlaps it.
                let (shared, far_a, far_b) = if j == i + 1 { (b, a, d) } else { (a, b, c) };
                if cross(shared, far_a, far_b) == 0 && (on_segment(far_b, shared, far_a) || on_segment(far_a, shared, far_b)) {
                    return crossing();
                }
            } else if meet(a, b, c, d) {
                return crossing();
            }
        }
    }
    Ok(())
}

/// The point halfway along a line, by its length, in the line's own units.
pub fn midpoint(line: &[[i32; 2]]) -> Option<[f64; 2]> {
    let pts: Vec<[f64; 2]> = line.iter().map(|p| [p[0] as f64, p[1] as f64]).collect();
    let last = *pts.last()?;
    let len = |a: [f64; 2], b: [f64; 2]| (b[0] - a[0]).hypot(b[1] - a[1]);
    let mut left = pts.windows(2).map(|w| len(w[0], w[1])).sum::<f64>() / 2.0;
    for w in pts.windows(2) {
        let l = len(w[0], w[1]);
        if l > 0.0 && left <= l {
            let t = left / l;
            return Some([w[0][0] + (w[1][0] - w[0][0]) * t, w[0][1] + (w[1][1] - w[0][1]) * t]);
        }
        left -= l;
    }
    Some(last)
}

/// Whether the street along `shape_mm` (layout millimetres) is inside the ring: its midpoint is.
pub fn holds(ring: &[LonLat], shape_mm: &[[i32; 2]], projection: &Projection) -> bool {
    midpoint(shape_mm).is_some_and(|[x, y]| contains(ring, projection.lon_lat(x.round() as i32, y.round() as i32)))
}

/// The streets of a city view inside the ring, by uid.
pub fn inside(ring: &[LonLat], edges: &[EdgeView], projection: &Projection) -> Vec<u32> {
    edges.iter().filter(|e| holds(ring, &e.shape_mm, projection)).map(|e| e.uid).collect()
}

/// The speeds a limit can be set to, in km/h, and the one a new limit starts at.
pub const SPEEDS: [u8; 7] = [10, 20, 30, 40, 50, 60, 70];
pub const DEFAULT_SPEED: u8 = 30;

/// The traffic rule over an area.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Rule {
    CarFree,
    BusAndBikeOnly,
    SpeedLimit { kmh: u8 },
    NoParkingOrLoading,
}

impl Rule {
    /// Every rule's id, in the order a choice lists them.
    pub const IDS: [&'static str; 4] = ["car_free", "bus_and_bike_only", "speed_limit", "no_parking_or_loading"];

    pub fn id(self) -> &'static str {
        match self {
            Rule::CarFree => Rule::IDS[0],
            Rule::BusAndBikeOnly => Rule::IDS[1],
            Rule::SpeedLimit { .. } => Rule::IDS[2],
            Rule::NoParkingOrLoading => Rule::IDS[3],
        }
    }

    /// The rule an id names; a speed limit is at `kmh`.
    pub fn from_id(id: &str, kmh: u8) -> Option<Rule> {
        match Rule::IDS.iter().position(|i| *i == id)? {
            0 => Some(Rule::CarFree),
            1 => Some(Rule::BusAndBikeOnly),
            2 => Some(Rule::SpeedLimit { kmh }),
            _ => Some(Rule::NoParkingOrLoading),
        }
    }

    /// The rule's name.
    pub fn said(self) -> Said {
        match self {
            Rule::CarFree => Said::new("restriction-rule-car-free"),
            Rule::BusAndBikeOnly => Said::new("restriction-rule-bus-and-bike"),
            Rule::SpeedLimit { kmh } => Said::new("restriction-rule-speed-limit").with("kmh", Arg::Num(kmh.into())),
            Rule::NoParkingOrLoading => Said::new("restriction-rule-no-parking"),
        }
    }

    /// The message that names the rule `id` among the choices, a speed limit without its speed.
    pub fn choice_key(id: &str) -> &'static str {
        match Rule::IDS.iter().position(|i| *i == id) {
            Some(0) => "restriction-rule-car-free",
            Some(1) => "restriction-rule-bus-and-bike",
            Some(2) => "restriction-rule-speed-limit-choice",
            _ => "restriction-rule-no-parking",
        }
    }

    /// The catalogue kinds of piece the rule does not allow.
    fn forbids(self) -> &'static [&'static str] {
        match self {
            Rule::CarFree => &CAR_KINDS,
            Rule::BusAndBikeOnly => &["travel"],
            Rule::NoParkingOrLoading => &["parking", "loading"],
            Rule::SpeedLimit { .. } => &[],
        }
    }

    /// Whether a street inside can only be listed under it, not checked: streets carry no speed.
    pub fn informational(self) -> bool {
        matches!(self, Rule::SpeedLimit { .. })
    }
}

/// Pieces of one kind a street has that a rule does not allow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Conflict {
    /// The catalogue id of the kind.
    pub kind: &'static str,
    pub pieces: usize,
}

impl Conflict {
    /// What the street has, as a sentence.
    pub fn said(&self) -> Said {
        let key = match self.kind {
            "travel" => "restriction-conflict-travel",
            "parking" => "restriction-conflict-parking",
            _ => "restriction-conflict-loading",
        };
        Said::new(key).with("n", Arg::Num(self.pieces as i64))
    }
}

/// What a street has that `rule` does not allow, a kind of piece at a time in the order the rule names them. A
/// piece counts for a kind when it is that kind at any time of day. Every zone is read; a kind counts as the
/// most pieces of it in any one section (never a sum), so a conflict in any zone is the street's.
pub fn conflicts(rule: Rule, street: &Street) -> Vec<Conflict> {
    let sections = street.sections();
    rule.forbids()
        .iter()
        .filter_map(|&kind| {
            let pieces = sections
                .iter()
                .map(|section| section.iter().filter(|s| std::iter::once(s.kind).chain(s.variants.iter().map(|v| v.kind)).any(|k| KINDS[k].id == kind)).count())
                .max()
                .unwrap_or(0);
            (pieces > 0).then_some(Conflict { kind, pieces })
        })
        .collect()
}

/// A rule over an outline, as the resident declared it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Restriction {
    /// `r<n>`: the key the document keeps the record under, so not written inside it.
    #[serde(skip)]
    pub id: String,
    /// The resident's own words, kept as typed.
    pub name: String,
    pub rule: Rule,
    /// The outline's corners, not closed explicitly.
    pub ring: Vec<LonLat>,
}

impl Restriction {
    /// What it is called: its name, or the rule's name when it has none.
    pub fn title(&self) -> Said {
        if self.name.trim().is_empty() { self.rule.said() } else { Said::new("restriction-named").with("name", Arg::Text(self.name.clone())) }
    }

    /// Whether the street along `shape_mm` is inside it.
    pub fn holds(&self, shape_mm: &[[i32; 2]], projection: &Projection) -> bool {
        holds(&self.ring, shape_mm, projection)
    }
}

/// The number in a restriction's id, `r12` is 12; None for an id of another shape.
pub fn number_of(id: &str) -> Option<u32> {
    id.strip_prefix('r')?.parse().ok()
}

/// How a street stands against one restriction it is inside.
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    /// The restriction's title.
    pub area: Said,
    pub rule: Rule,
    pub conflicts: Vec<Conflict>,
}

impl Finding {
    pub fn of(area: &Restriction, street: &Street) -> Finding {
        Finding { area: area.title(), rule: area.rule, conflicts: conflicts(area.rule, street) }
    }

    /// Whether the street keeps to the rule (a speed limit is always kept: it is only listed).
    pub fn ok(&self) -> bool {
        self.conflicts.is_empty()
    }

    pub fn label(&self) -> Said {
        Said::new("restriction-check").with("area", Arg::Said(Box::new(self.area.clone())))
    }

    /// What it says, sentence by sentence: how the street stands, then each thing it has that the rule does not allow.
    pub fn detail(&self) -> Vec<Said> {
        let key = if self.rule.informational() {
            "restriction-check-info"
        } else if self.ok() {
            "restriction-check-keeps"
        } else {
            "restriction-check-breaks"
        };
        let head = Said::new(key).with("rule", Arg::Said(Box::new(self.rule.said())));
        std::iter::once(head).chain(self.conflicts.iter().map(Conflict::said)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::restriction::fixtures::{bent_midpoint, bent_store, square};
    use crate::shared::testing::{en, fr};

    const SQUARE: [LonLat; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    /// A U: the notch between x 1 and 2 reaches down to y 1.
    const U: [LonLat; 8] = [[0.0, 0.0], [3.0, 0.0], [3.0, 3.0], [2.0, 3.0], [2.0, 1.0], [1.0, 1.0], [1.0, 3.0], [0.0, 3.0]];

    #[test]
    fn a_point_is_inside_outside_or_on_the_border_which_counts_as_inside() {
        assert!(contains(&SQUARE, [0.5, 0.5]));
        assert!(!contains(&SQUARE, [1.5, 0.5]));
        assert!(!contains(&SQUARE, [0.5, -0.5]));
        assert!(contains(&SQUARE, [1.0, 0.5]), "on an edge");
        assert!(contains(&SQUARE, [0.5, 1.0]), "on the top edge");
        assert!(contains(&SQUARE, [0.0, 0.0]), "on a corner");
    }

    #[test]
    fn a_concave_outline_leaves_its_notch_out() {
        assert!(!contains(&U, [1.5, 2.0]), "in the notch");
        assert!(contains(&U, [0.5, 2.0]) && contains(&U, [2.5, 2.0]) && contains(&U, [1.5, 0.5]));
    }

    #[test]
    fn a_ray_through_corners_is_counted_once_at_each_crossing() {
        let diamond = [[1.0, 0.0], [2.0, 1.0], [1.0, 2.0], [0.0, 1.0]];
        assert!(contains(&diamond, [0.5, 1.0]) && contains(&diamond, [1.0, 1.0]));
        assert!(!contains(&diamond, [-0.5, 1.0]) && !contains(&diamond, [2.5, 1.0]));
    }

    #[test]
    fn the_geometry_is_exact_to_the_micro_degree() {
        assert!(contains(&SQUARE, [1.000_000_4, 0.5]), "rounds onto the border");
        assert!(!contains(&SQUARE, [1.000_001, 0.5]), "a micro-degree out is out");
    }

    #[test]
    fn an_outline_across_the_antimeridian_is_not_supported_and_reads_as_the_long_way_round() {
        let across = [[179.0, 0.0], [-179.0, 0.0], [-179.0, 1.0], [179.0, 1.0]];
        assert!(!contains(&across, [179.5, 0.5]));
        assert!(contains(&across, [0.0, 0.5]));
    }

    #[test]
    fn fewer_than_three_corners_hold_nothing() {
        assert!(!contains(&[[0.0, 0.0], [1.0, 1.0]], [0.5, 0.5]));
        assert!(!contains(&[], [0.0, 0.0]));
    }

    #[test]
    fn an_outline_is_valid_with_three_corners_not_in_a_line_and_no_crossing() {
        assert_eq!(valid(&SQUARE), Ok(()));
        assert_eq!(valid(&U), Ok(()));
        assert_eq!(valid(&[[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]), Ok(()));
    }

    #[test]
    fn too_few_corners_or_all_in_a_line_are_refused_with_the_reason() {
        let too_few = Said::new("restriction-refused-too-few");
        assert_eq!(valid(&[[0.0, 0.0], [1.0, 0.0]]), Err(too_few.clone()));
        assert_eq!(valid(&[[0.0, 0.0], [1.0, 0.0], [2.0, 0.0]]), Err(too_few.clone()), "in a line");
        assert_eq!(valid(&[[0.0, 0.0], [1.0, 0.0], [0.0, 0.0]]), Err(too_few), "two distinct");
        assert_eq!(en(&Said::new("restriction-refused-too-few")), "An area needs at least three corners, not all in a line.");
        assert_eq!(fr(&Said::new("restriction-refused-too-few")), "Une zone a besoin d’au moins trois coins, pas tous alignés.");
    }

    #[test]
    fn crossing_touching_folding_back_and_repeated_corners_are_refused_as_a_crossing() {
        let crossing = Err(Said::new("restriction-refused-crossing"));
        assert_eq!(valid(&[[0.0, 0.0], [1.0, 1.0], [1.0, 0.0], [0.0, 1.0]]), crossing, "a bow tie");
        assert_eq!(valid(&[[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [2.0, 0.0]]), crossing, "the last edge folds back along the first");
        assert_eq!(valid(&[[0.0, 0.0], [4.0, 0.0], [2.0, 2.0], [4.0, 4.0], [0.0, 4.0], [2.0, 2.0]]), crossing, "a corner used twice");
        assert_eq!(valid(&[[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [2.0, 0.0], [0.0, 4.0]]), crossing, "a corner touching an edge");
        assert_eq!(en(&Said::new("restriction-refused-crossing")), "The outline crosses itself. Move or remove a corner.");
    }

    #[test]
    fn the_midpoint_is_halfway_along_the_line_by_its_length() {
        assert_eq!(midpoint(&[[0, 0], [10, 0]]), Some([5.0, 0.0]));
        let [x, y] = midpoint(&[[0, 0], [80_000, 0], [100_000, 50_000]]).unwrap();
        let half = (80_000.0 + 20_000f64.hypot(50_000.0)) / 2.0;
        assert!((x - half).abs() < 1e-6 && y == 0.0, "{x} {y}");
        assert_eq!(midpoint(&[[3, 4]]), Some([3.0, 4.0]));
        assert_eq!(midpoint(&[]), None);
    }

    #[test]
    fn the_streets_inside_are_those_whose_midpoint_is() {
        let (ports, ..) = crate::shared::ports::test_ports();
        let store = bent_store(ports.storage);
        let v = store.open().view(0);
        let (p, mid) = bent_midpoint(&store);
        assert_eq!(inside(&square(mid, 0.0005), &v.edges, &p), vec![1]);
        assert_eq!(inside(&square([mid[0] + 0.01, mid[1]], 0.0005), &v.edges, &p), Vec::<u32>::new());
        let start = p.lon_lat(v.edges[0].shape_mm[0][0], v.edges[0].shape_mm[0][1]);
        assert_eq!(inside(&square(start, 0.0002), &v.edges, &p), Vec::<u32>::new(), "an end inside is not the street inside");
        assert!(holds(&square(mid, 0.0005), &v.edges[0].shape_mm, &p));
    }

    use crate::shared::catalogue::{KINDS, Side, StreetClass, kind_index};
    use crate::street::model::{Piece, Street, Window};

    fn piece(id: &str) -> Piece {
        let kind = kind_index(id).unwrap();
        Piece { kind, width_mm: KINDS[kind].min_mm, direction: None, variants: Vec::new() }
    }

    fn street(ids: &[&str]) -> Street {
        Street::imported(StreetClass::Local, Side::Right, &ids.iter().map(|id| piece(id)).collect::<Vec<_>>())
    }

    fn c(kind: &'static str, pieces: usize) -> Conflict {
        Conflict { kind, pieces }
    }

    #[test]
    fn an_area_rule_reads_every_zone_and_counts_the_most_pieces_of_one_kind_in_a_zone() {
        use crate::street::model::{Segment, Zone, zoned};
        let kind = |id: &str| kind_index(id).unwrap();
        let piece = |uid: u32, id: &str| Segment::new(uid, kind(id), 3_000);
        let base = vec![piece(1, "sidewalk"), piece(2, "travel"), piece(3, "sidewalk")];
        let mut street = Street::sample(0, Side::Right);
        street.segments = base;
        let mut street = zoned(street, &[0, 20_000], 40_000);
        street.zones[0].segments = Some(vec![piece(1, "sidewalk"), piece(2, "travel"), piece(4, "travel"), piece(3, "sidewalk")]);
        street.zones[1] = Zone { from_mm: 20_000, segments: None, name: None };
        assert_eq!(conflicts(Rule::CarFree, &street), vec![c("travel", 2)], "the most in one zone, not the sum (3)");
        let finding = Finding::of(&Restriction { id: "r1".into(), name: String::new(), rule: Rule::CarFree, ring: SQUARE.to_vec() }, &street);
        assert!(!finding.ok());
        assert_eq!(finding.conflicts, vec![c("travel", 2)]);

        let mut mixed = street.clone();
        mixed.zones[0].segments = Some(vec![piece(1, "sidewalk"), piece(5, "planting"), piece(3, "sidewalk")]);
        assert_eq!(conflicts(Rule::CarFree, &mixed), vec![c("travel", 1)], "a pedestrian zone beside a car zone conflicts");

        let mut calm = mixed.clone();
        calm.segments = vec![piece(1, "sidewalk"), piece(5, "planting"), piece(3, "sidewalk")];
        calm.zones[1].segments = None;
        assert_eq!(conflicts(Rule::CarFree, &calm), vec![]);
        assert!(Finding::of(&Restriction { id: "r1".into(), name: String::new(), rule: Rule::CarFree, ring: SQUARE.to_vec() }, &calm).ok());
    }

    #[test]
    fn a_street_is_pedestrian_exactly_when_it_has_no_motor_piece_and_breaks_the_car_free_rule_only_for_cars() {
        // (pieces, breaks the Car-free rule, is pedestrian)
        for (ids, car_free_breaks, pedestrian) in [
            (&["sidewalk"][..], false, true),
            (&["sidewalk", "bike"], false, true),
            (&["sidewalk", "bus", "bike"], false, false),
            (&["sidewalk", "shoulder"], false, false),
            (&["sidewalk", "travel"], true, false),
            (&["parking"], true, false),
            (&["bike", "loading"], true, false),
        ] {
            let s = street(ids);
            assert_eq!(!conflicts(Rule::CarFree, &s).is_empty(), car_free_breaks, "{ids:?}");
            assert_eq!(s.class == StreetClass::Pedestrian, pedestrian, "{ids:?}");
            assert_eq!(s.has_motor(), !pedestrian, "{ids:?}");
            assert!(!car_free_breaks || s.has_motor(), "a car piece is a motor piece: {ids:?}");
        }
    }

    #[test]
    fn car_free_allows_no_driving_parking_or_loading() {
        let busy = street(&["sidewalk", "travel", "travel", "parking", "loading", "sidewalk"]);
        assert_eq!(conflicts(Rule::CarFree, &busy), vec![c("travel", 2), c("parking", 1), c("loading", 1)]);
        let calm = street(&["sidewalk", "bike", "bus", "planting", "sidewalk"]);
        assert_eq!(conflicts(Rule::CarFree, &calm), vec![]);
    }

    #[test]
    fn car_free_forbids_what_the_catalogue_calls_cars_and_still_allows_buses() {
        use crate::shared::catalogue::CAR_KINDS;
        assert_eq!(Rule::CarFree.forbids(), &CAR_KINDS[..]);
        let calm = street(&["sidewalk", "bus", "bike", "shoulder"]);
        assert_eq!(conflicts(Rule::CarFree, &calm), vec![], "a bus lane, a bike lane and a shoulder are not cars to the rule");
    }

    #[test]
    fn bus_and_bike_only_allows_no_driving_lane_and_leaves_parking_alone() {
        let s = street(&["sidewalk", "travel", "bus", "bike", "parking"]);
        assert_eq!(conflicts(Rule::BusAndBikeOnly, &s), vec![c("travel", 1)]);
    }

    #[test]
    fn no_parking_or_loading_allows_neither() {
        let s = street(&["sidewalk", "travel", "loading", "parking"]);
        assert_eq!(conflicts(Rule::NoParkingOrLoading, &s), vec![c("parking", 1), c("loading", 1)]);
    }

    #[test]
    fn a_speed_limit_checks_nothing_and_is_for_information() {
        let s = street(&["travel", "travel"]);
        let limit = Rule::SpeedLimit { kmh: 30 };
        assert_eq!(conflicts(limit, &s), vec![]);
        assert!(limit.informational() && !Rule::CarFree.informational());
    }

    #[test]
    fn a_piece_counts_for_every_kind_it_takes_during_the_day() {
        let parking = kind_index("parking").unwrap();
        let travel = kind_index("travel").unwrap();
        let rush = Piece { kind: parking, width_mm: 3000, direction: None, variants: vec![Window { kind: travel, from_min: 420, to_min: 540, days: None }] };
        let s = Street::imported(StreetClass::Local, Side::Right, &[piece("sidewalk"), rush]);
        assert_eq!(conflicts(Rule::CarFree, &s), vec![c("travel", 1), c("parking", 1)]);
        assert_eq!(conflicts(Rule::BusAndBikeOnly, &s), vec![c("travel", 1)], "a driving lane at rush hour");
    }

    #[test]
    fn the_rules_are_named_and_kept_by_id() {
        assert_eq!(en(&Rule::CarFree.said()), "Car-free");
        assert_eq!(en(&Rule::BusAndBikeOnly.said()), "Bus and bike only");
        assert_eq!(en(&Rule::SpeedLimit { kmh: 30 }.said()), "Speed limit of 30 km/h");
        assert_eq!(en(&Rule::NoParkingOrLoading.said()), "No parking or loading");
        assert_eq!(fr(&Rule::CarFree.said()), "Zone sans voitures");
        assert_eq!(fr(&Rule::SpeedLimit { kmh: 30 }.said()), "Limite de vitesse de 30\u{a0}km/h");
        for id in Rule::IDS {
            assert_eq!(Rule::from_id(id, 40).map(Rule::id), Some(id));
        }
        assert_eq!(Rule::from_id("speed_limit", 40), Some(Rule::SpeedLimit { kmh: 40 }));
        assert_eq!(Rule::from_id("nonsense", 40), None);
        assert_eq!(serde_json::to_string(&Rule::SpeedLimit { kmh: 30 }).unwrap(), r#"{"kind":"speed_limit","kmh":30}"#);
        assert_eq!(serde_json::to_string(&Rule::CarFree).unwrap(), r#"{"kind":"car_free"}"#);
    }

    #[test]
    fn a_conflict_is_a_whole_sentence_with_its_count() {
        assert_eq!(en(&c("travel", 1).said()), "It has a driving lane.");
        assert_eq!(en(&c("travel", 2).said()), "It has 2 driving lanes.");
        assert_eq!(en(&c("loading", 1).said()), "It has a loading zone.");
        assert_eq!(fr(&c("parking", 2).said()), "Elle a 2 voies de stationnement.");
    }

    #[test]
    fn an_area_is_called_by_its_name_or_else_by_its_rule() {
        let r = |name: &str| Restriction { id: "r1".into(), name: name.into(), rule: Rule::CarFree, ring: SQUARE.to_vec() };
        assert_eq!(en(&r("Mile End").title()), "Mile End");
        assert_eq!(en(&r("  ").title()), "Car-free");
        assert_eq!(number_of("r12"), Some(12));
        assert_eq!(number_of("x3"), None);
        assert_eq!(number_of("r"), None);
        assert!(!serde_json::to_string(&r("A")).unwrap().contains("\"id\""), "the id is the document's key, not in the record");
    }

    #[test]
    fn a_street_s_finding_says_whether_it_keeps_to_the_rule_and_what_it_has() {
        let area = Restriction { id: "r1".into(), name: "Test zone".into(), rule: Rule::CarFree, ring: SQUARE.to_vec() };
        let f = Finding::of(&area, &street(&["sidewalk", "travel", "travel", "sidewalk"]));
        assert!(!f.ok());
        assert_eq!(en(&f.label()), "Area rule: Test zone");
        assert_eq!(f.detail().iter().map(en).collect::<Vec<_>>(), ["Car-free: the street does not keep to it.", "It has 2 driving lanes."]);
        assert_eq!(fr(&f.label()), "Règle de zone\u{a0}: Test zone");
        let calm = Finding::of(&area, &street(&["sidewalk", "bike", "sidewalk"]));
        assert!(calm.ok());
        assert_eq!(calm.detail().iter().map(en).collect::<Vec<_>>(), ["Car-free: the street keeps to it."]);
        let limit = Restriction { rule: Rule::SpeedLimit { kmh: 30 }, ..area };
        let info = Finding::of(&limit, &street(&["travel"]));
        assert!(info.ok());
        assert_eq!(en(&info.detail()[0]), "Speed limit of 30 km/h: streets carry no speed, so this is for information only.");
    }
}
