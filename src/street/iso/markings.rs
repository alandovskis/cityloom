//! The pavement markings a street would carry, inferred from its pieces (MTQ, Tome V and the Manuel technique
//! sur le marquage routier): which lines lie between neighbouring pieces and which symbol each lane carries.
//! A pure function of the `View`, with no SVG: the drawing is `paint.rs`.

use crate::shared::catalogue::KINDS;
use crate::shared::iso::paint::{Paint, Pattern, Style};
use crate::street::model::SegView;
use crate::street::svg::curb_edges;

const SOLID_WHITE: Style = Style { paint: Paint::White, pattern: Pattern::Solid, double: false };
const BROKEN_WHITE: Style = Style { paint: Paint::White, pattern: Pattern::Broken, double: false };
const DOUBLE_WHITE: Style = Style { paint: Paint::White, pattern: Pattern::Solid, double: true };
const DOUBLE_YELLOW: Style = Style { paint: Paint::Yellow, pattern: Pattern::Solid, double: true };
const BROKEN_YELLOW: Style = Style { paint: Paint::Yellow, pattern: Pattern::Broken, double: false };

/// A painted line down the street, centred `x_mm` across it from its left edge (the coordinates of
/// `SegView.x_mm`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line {
    pub x_mm: i32,
    pub style: Style,
}

/// What is painted on a lane, laid `ARROW_AT_MM` along it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Symbol {
    /// A straight-ahead lane arrow on a driving lane, pointing the way the lane runs.
    Ahead { toward: bool },
    /// The reserved-lane diamond, with "BUS" beyond it.
    Bus,
    /// The bike symbol with a small arrow, or two opposed small arrows on a two-way bike lane.
    Bike { two_way: bool, toward: bool },
}

/// The symbol of the piece `uid`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mark {
    pub uid: u32,
    pub symbol: Symbol,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Markings {
    pub lines: Vec<Line>,
    pub marks: Vec<Mark>,
}

/// Whether two pieces each run one way and opposite ways. A piece with no direction (a two-way bus or bike lane) is
/// opposed to neither neighbour.
fn runs_opposite(a: &SegView, b: &SegView) -> bool {
    matches!((a.direction, b.direction), (Some(x), Some(y)) if x != y)
}

/// The line on the boundary between two neighbouring pieces, by the kind each shows at the shown time. Where the
/// roadway meets a curb, sidewalk, planting or median there is none: the curb marks the edge, and so does the curb
/// of a bike lane or island (`curbed`: either neighbour carries one on this shared edge).
fn between(a: &SegView, b: &SegView, curbed: bool) -> Option<Style> {
    if curbed {
        return None;
    }
    let opposed = runs_opposite(a, b);
    match (KINDS[a.kind].id, KINDS[b.kind].id) {
        ("travel", "travel") => Some(if opposed { DOUBLE_YELLOW } else { BROKEN_WHITE }),
        ("travel", "bus") | ("bus", "travel") => Some(if opposed { DOUBLE_YELLOW } else { DOUBLE_WHITE }),
        ("travel", "bike") | ("bike", "travel") => Some(if opposed { DOUBLE_YELLOW } else { SOLID_WHITE }),
        ("travel", "parking" | "loading" | "shoulder") | ("parking" | "loading" | "shoulder", "travel") => Some(SOLID_WHITE),
        ("bus", "bus") => opposed.then_some(DOUBLE_YELLOW),
        _ => None,
    }
}

fn symbol(s: &SegView) -> Option<Symbol> {
    match KINDS[s.kind].id {
        "travel" => s.direction.map(|d| Symbol::Ahead { toward: d == "toward" }),
        "bus" => Some(Symbol::Bus),
        "bike" => Some(Symbol::Bike { two_way: s.direction.is_none(), toward: s.direction == Some("toward") }),
        _ => None,
    }
}

/// The lines and symbols of the street as it is shown now: a pure function of the pieces shown.
pub fn markings(segs: &[SegView]) -> Markings {
    // Whether the edge between piece `i` and the next carries a curb of either: the right edge of `i`, the left of `i + 1`.
    let curbed = |i: usize| curb_edges(segs, i).contains(&true) || curb_edges(segs, i + 1).contains(&false);
    let mut lines: Vec<Line> = segs
        .windows(2)
        .enumerate()
        .filter_map(|(i, w)| between(&w[0], &w[1], curbed(i)).map(|style| Line { x_mm: w[0].x_mm + w[0].width_mm, style }))
        .collect();
    // A two-way bike lane carries a single broken yellow line down its middle.
    lines.extend(
        segs.iter().filter(|s| KINDS[s.kind].id == "bike" && s.direction.is_none()).map(|s| Line { x_mm: s.x_mm + s.width_mm / 2, style: BROKEN_YELLOW }),
    );
    let marks = segs.iter().filter_map(|s| symbol(s).map(|symbol| Mark { uid: s.uid, symbol })).collect();
    Markings { lines, marks }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::{DIRECTIONS, REGIONS, Side, kind_index};
    use crate::street::model::Editor;

    /// Where piece `i` ends, across the street.
    fn edge(e: &Editor, i: usize) -> i32 {
        let v = e.view();
        v.segments[i].x_mm + v.segments[i].width_mm
    }

    /// The styles of the lines centred at `x`.
    fn at(m: &Markings, x: i32) -> Vec<Style> {
        m.lines.iter().filter(|l| l.x_mm == x).map(|l| l.style).collect()
    }

    fn dir(e: &Editor, i: usize) -> usize {
        let d = e.view().segments[i].direction.expect("a lane with a direction");
        DIRECTIONS.iter().position(|x| x.id == d).unwrap()
    }

    fn opposite(index: usize) -> usize {
        1 - index
    }

    #[test]
    fn two_driving_lanes_running_opposite_ways_have_a_double_solid_yellow_line() {
        let e = Editor::new(0);
        let m = markings(&e.view().segments);
        assert_ne!(dir(&e, 2), dir(&e, 3));
        assert_eq!(at(&m, edge(&e, 2)), vec![DOUBLE_YELLOW]);
    }

    #[test]
    fn two_driving_lanes_running_the_same_way_have_a_single_broken_white_line_and_a_median_has_none() {
        // Sample Avenue 2: sidewalk, planting, parking, travel, travel, median, travel, travel, parking, planting, sidewalk.
        let e = Editor::new(1);
        let m = markings(&e.view().segments);
        assert_eq!(dir(&e, 3), dir(&e, 4));
        assert_eq!(at(&m, edge(&e, 3)), vec![BROKEN_WHITE]);
        assert_eq!(at(&m, edge(&e, 6)), vec![BROKEN_WHITE]);
        assert!(at(&m, edge(&e, 4)).is_empty() && at(&m, edge(&e, 5)).is_empty(), "no line where the roadway meets a median");
        assert_eq!(m.lines.len(), 4, "{:?}", m.lines);
    }

    #[test]
    fn a_buffer_has_no_line_against_it_on_either_side_and_no_symbol() {
        // MTQ Tome V has no line for the edge of a buffer: the buffer is its own marking, so none is drawn against it.
        let mut e = Editor::new(0);
        let buffer = e.add(kind_index("buffer").unwrap(), 3);
        let m = markings(&e.view().segments);
        assert!(at(&m, edge(&e, 2)).is_empty() && at(&m, edge(&e, 3)).is_empty(), "{:?}", m.lines);
        assert!(m.marks.iter().all(|k| k.uid != buffer));
    }

    #[test]
    fn a_driving_lane_beside_parking_loading_or_a_shoulder_has_a_single_solid_white_line() {
        let e = Editor::new(0);
        let m = markings(&e.view().segments);
        assert_eq!(at(&m, edge(&e, 1)), vec![SOLID_WHITE]);
        assert_eq!(at(&m, edge(&e, 3)), vec![SOLID_WHITE]);
        // A loading lane between the parking and the driving lane.
        let mut e = Editor::new(0);
        e.add(kind_index("loading").unwrap(), 2);
        let m = markings(&e.view().segments);
        assert!(at(&m, edge(&e, 1)).is_empty(), "parking beside loading: no line");
        assert_eq!(at(&m, edge(&e, 2)), vec![SOLID_WHITE]);
        // The freeway: shoulder, three lanes, median, three lanes, shoulder.
        let e = Editor::new(3);
        let m = markings(&e.view().segments);
        assert_eq!(at(&m, edge(&e, 0)), vec![SOLID_WHITE]);
        assert_eq!(at(&m, edge(&e, 7)), vec![SOLID_WHITE]);
        assert_eq!(at(&m, edge(&e, 1)), vec![BROKEN_WHITE]);
    }

    #[test]
    fn a_bus_lane_running_with_the_traffic_has_a_double_white_line_and_one_against_it_a_double_yellow() {
        let mut e = Editor::new(0);
        let bus = e.add(kind_index("bus").unwrap(), 3);
        let with = dir(&e, 2);
        assert!(e.set_direction(bus, Some(with)));
        let m = markings(&e.view().segments);
        // travel, then the bus lane with it: double white; the bus lane, then the lane running the other way: double yellow.
        assert_eq!(at(&m, edge(&e, 2)), vec![DOUBLE_WHITE]);
        assert_eq!(at(&m, edge(&e, 3)), vec![DOUBLE_YELLOW]);
    }

    #[test]
    fn a_contraflow_bus_lane_beside_the_lane_it_runs_against_is_double_yellow() {
        let mut e = Editor::new(0);
        let bus = e.add(kind_index("bus").unwrap(), 3);
        let against = opposite(dir(&e, 2));
        assert!(e.set_direction(bus, Some(against)));
        let m = markings(&e.view().segments);
        assert_eq!(at(&m, edge(&e, 2)), vec![DOUBLE_YELLOW]);
        assert_eq!(at(&m, edge(&e, 3)), vec![DOUBLE_WHITE]);
    }

    #[test]
    fn a_two_way_bus_lane_is_opposed_to_neither_neighbour() {
        let mut e = Editor::new(0);
        e.add(kind_index("bus").unwrap(), 3);
        let m = markings(&e.view().segments);
        assert_eq!(at(&m, edge(&e, 2)), vec![DOUBLE_WHITE]);
        assert_eq!(at(&m, edge(&e, 3)), vec![DOUBLE_WHITE]);
    }

    #[test]
    fn two_bus_lanes_running_opposite_ways_have_a_double_yellow_line_and_the_same_way_none() {
        let mut e = Editor::new(0);
        let first = e.add(kind_index("bus").unwrap(), 3);
        let second = e.add(kind_index("bus").unwrap(), 4);
        assert!(e.set_direction(first, Some(0)) && e.set_direction(second, Some(1)));
        assert_eq!(at(&markings(&e.view().segments), edge(&e, 3)), vec![DOUBLE_YELLOW]);
        assert!(e.set_direction(second, Some(0)));
        assert!(at(&markings(&e.view().segments), edge(&e, 3)).is_empty());
    }

    #[test]
    fn a_bike_lane_with_its_own_curb_has_no_line_where_it_meets_the_roadway_and_without_one_has() {
        let mut e = Editor::new(0);
        let bike = e.add(kind_index("bike").unwrap(), 3);
        let curbed = e.view().segments[3].curb.is_some();
        assert!(curbed, "a bike lane has a curb by default");
        let m = markings(&e.view().segments);
        assert!(at(&m, edge(&e, 2)).is_empty() && at(&m, edge(&e, 3)).is_empty(), "the curb marks the edge: {:?}", m.lines);
        assert!(e.set_curb(bike, None));
        let m = markings(&e.view().segments);
        assert_eq!(at(&m, edge(&e, 2)), vec![SOLID_WHITE]);
        assert_eq!(at(&m, edge(&e, 3)), vec![SOLID_WHITE]);
        // Uncurbed and running against the lane before it: the double yellow.
        let against = opposite(dir(&e, 2));
        assert!(e.set_direction(bike, Some(against)));
        assert_eq!(at(&markings(&e.view().segments), edge(&e, 2)), vec![DOUBLE_YELLOW]);
    }

    #[test]
    fn a_two_way_bike_lane_has_solid_white_edges_and_a_broken_yellow_line_down_its_middle() {
        let mut e = Editor::new(0);
        let bike = e.add(kind_index("bike").unwrap(), 3);
        assert!(e.set_curb(bike, None));
        let v = e.view();
        let s = v.segments.iter().find(|s| s.uid == bike).unwrap();
        assert!(s.direction.is_none());
        let m = markings(&v.segments);
        assert_eq!(at(&m, edge(&e, 2)), vec![SOLID_WHITE]);
        assert_eq!(at(&m, edge(&e, 3)), vec![SOLID_WHITE]);
        assert_eq!(at(&m, s.x_mm + s.width_mm / 2), vec![BROKEN_YELLOW]);
    }

    #[test]
    fn a_one_way_bike_lane_with_the_traffic_has_solid_white_on_both_sides_and_no_middle_line() {
        let mut e = Editor::new(0);
        let bike = e.add(kind_index("bike").unwrap(), 3);
        assert!(e.set_curb(bike, None));
        let with = dir(&e, 2);
        assert!(e.set_direction(bike, Some(with)));
        let v = e.view();
        let s = v.segments.iter().find(|s| s.uid == bike).unwrap();
        let m = markings(&v.segments);
        assert_eq!(at(&m, edge(&e, 2)), vec![SOLID_WHITE]);
        // The lane after it runs the other way: the bike lane is opposed to it.
        assert_eq!(at(&m, edge(&e, 3)), vec![DOUBLE_YELLOW]);
        assert!(at(&m, s.x_mm + s.width_mm / 2).is_empty());
    }

    #[test]
    fn a_contraflow_bike_lane_beside_the_lane_it_runs_against_is_double_yellow() {
        let mut e = Editor::new(0);
        let bike = e.add(kind_index("bike").unwrap(), 3);
        assert!(e.set_curb(bike, None));
        let against = opposite(dir(&e, 2));
        assert!(e.set_direction(bike, Some(against)));
        let m = markings(&e.view().segments);
        assert_eq!(at(&m, edge(&e, 2)), vec![DOUBLE_YELLOW]);
        assert_eq!(at(&m, edge(&e, 3)), vec![SOLID_WHITE]);
    }

    #[test]
    fn any_other_pair_has_no_line_and_a_lane_beside_a_curb_has_no_edge_line() {
        // Sample Lane 3: sidewalk, travel, travel, parking, sidewalk. The sidewalk (a curb) marks the edge itself.
        let e = Editor::new(2);
        let m = markings(&e.view().segments);
        assert!(at(&m, edge(&e, 0)).is_empty(), "sidewalk beside a driving lane");
        assert!(at(&m, edge(&e, 3)).is_empty(), "parking beside a sidewalk");
        // A bike lane beside parking is not in the table either.
        let mut e = Editor::new(0);
        e.add(kind_index("bike").unwrap(), 2);
        assert!(at(&markings(&e.view().segments), edge(&e, 1)).is_empty());
        // And the first sample's sidewalk edges carry nothing.
        let e = Editor::new(0);
        let m = markings(&e.view().segments);
        assert!(at(&m, edge(&e, 0)).is_empty() && at(&m, edge(&e, 4)).is_empty());
    }

    #[test]
    fn a_lane_that_is_a_bus_lane_at_the_shown_time_is_marked_as_one() {
        let mut e = Editor::new(0);
        let lane = e.view().segments[2].uid;
        assert!(e.add_variant(lane));
        // At noon it is a driving lane: solid white beside the parking, an arrow.
        let noon = markings(&e.view().segments);
        assert_eq!(at(&noon, edge(&e, 1)), vec![SOLID_WHITE]);
        assert!(matches!(noon.marks[0].symbol, Symbol::Ahead { .. }), "the first symbol is the lane's: the parking has none");
        // At 08:00 the variant (a bus lane) is shown: parking beside a bus lane has no line, and the symbol is the bus's.
        assert!(e.set_time(8 * 60));
        let v = e.view();
        assert_eq!(crate::shared::catalogue::KINDS[v.segments[2].kind].id, "bus");
        let rush = markings(&v.segments);
        assert!(at(&rush, edge(&e, 1)).is_empty());
        assert_eq!(rush.marks[0], Mark { uid: lane, symbol: Symbol::Bus });
    }

    #[test]
    fn a_street_with_no_roadway_has_no_lines_and_no_marks() {
        let mut e = Editor::new(0);
        for uid in e.view().segments.iter().filter(|s| crate::shared::catalogue::KINDS[s.kind].id != "sidewalk").map(|s| s.uid).collect::<Vec<_>>() {
            assert!(e.remove(uid));
        }
        assert_eq!(markings(&e.view().segments), Markings { lines: Vec::new(), marks: Vec::new() });
        // Not a piece at all.
        while let Some(uid) = e.view().segments.first().map(|s| s.uid) {
            assert!(e.remove(uid));
        }
        assert_eq!(markings(&e.view().segments), Markings { lines: Vec::new(), marks: Vec::new() });
        // One piece alone: no pair, so no line.
        let mut e = Editor::new(0);
        while e.view().segments.len() > 1 {
            assert!(e.remove(e.view().segments[0].uid));
        }
        assert!(markings(&e.view().segments).lines.is_empty());
    }

    #[test]
    fn each_kind_of_lane_carries_its_own_symbol_and_the_other_pieces_none() {
        let e = Editor::new(0);
        let v = e.view();
        let m = markings(&v.segments);
        // Two driving lanes, each with an arrow pointing the way it runs; the sidewalks and parking carry none.
        assert_eq!(m.marks.len(), 2);
        let toward = |i: usize| e.view().segments[i].direction == Some("toward");
        assert_eq!(m.marks[0], Mark { uid: v.segments[2].uid, symbol: Symbol::Ahead { toward: toward(2) } });
        assert_eq!(m.marks[1], Mark { uid: v.segments[3].uid, symbol: Symbol::Ahead { toward: toward(3) } });
        assert_ne!(toward(2), toward(3));
        // A bus lane: the diamond, whatever its direction.
        let mut e = Editor::new(0);
        let bus = e.add(kind_index("bus").unwrap(), 3);
        assert!(markings(&e.view().segments).marks.contains(&Mark { uid: bus, symbol: Symbol::Bus }));
        // A bike lane: two-way, then one way each way.
        let bike = e.add(kind_index("bike").unwrap(), 1);
        let find = |e: &Editor| markings(&e.view().segments).marks.into_iter().find(|m| m.uid == bike).unwrap().symbol;
        assert_eq!(find(&e), Symbol::Bike { two_way: true, toward: false });
        assert!(e.set_direction(bike, Some(0)));
        assert_eq!(find(&e), Symbol::Bike { two_way: false, toward: false });
        assert!(e.set_direction(bike, Some(1)));
        assert_eq!(find(&e), Symbol::Bike { two_way: false, toward: true });
    }

    #[test]
    fn a_left_hand_traffic_street_still_has_its_double_yellow_centre_line_and_its_arrows() {
        let mut e = Editor::new(0);
        let uk = REGIONS.iter().position(|r| r.drive_side == Side::Left).unwrap();
        assert!(e.set_region(uk));
        let m = markings(&e.view().segments);
        assert_ne!(dir(&e, 2), dir(&e, 3));
        assert_eq!(at(&m, edge(&e, 2)), vec![DOUBLE_YELLOW]);
        assert_eq!(m.marks.len(), 2);
        let ways: Vec<bool> = m.marks.iter().map(|k| matches!(k.symbol, Symbol::Ahead { toward: true })).collect();
        assert_ne!(ways[0], ways[1], "the two arrows point opposite ways");
    }
}
