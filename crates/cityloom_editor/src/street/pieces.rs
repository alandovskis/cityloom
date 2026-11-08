//! What the inspector, the 2.5D and the importer share about drawing a piece: which pieces a curb stands beside, how
//! wide a curb is, and the arrow that says which way traffic goes.

use crate::shared::catalogue::KINDS;
use crate::street::model::SegView;

/// The kinds a curb is drawn beside: road pieces.
pub const ROAD: [&str; 4] = ["travel", "bus", "parking", "loading"];

/// How wide a curb is, in millimetres of street: a planted curb is a strip of planting, a Kassel
/// kerb is wider with a sloped road face, a boarding island is a wide raised platform, a
/// bike-friendly curb is a low ramp a wheel can ride up.
pub fn curb_width_mm(curb: &str) -> f64 {
    match curb {
        "planted" => 600.0,
        "island" => 900.0,
        "kassel" => 250.0,
        "bikefriendly" => 300.0,
        _ => 150.0,
    }
}

/// The edges of the piece at `i` that carry its curb, each `true` for its right edge: those that
/// face a road piece. An island stands on one side only, the one facing the buses.
pub fn curb_edges(segs: &[SegView], i: usize) -> Vec<bool> {
    let Some(s) = segs.get(i) else { return Vec::new() };
    let Some(curb) = s.curb else { return Vec::new() };
    let kind_of = |n: Option<&SegView>| n.map(|n| KINDS[n.kind].id);
    let left = if i > 0 { segs.get(i - 1) } else { None };
    let mut sides = vec![(left, false), (segs.get(i + 1), true)];
    sides.retain(|(nb, _)| kind_of(*nb).is_some_and(|id| ROAD.contains(&id)));
    if curb == "island" {
        let rank = |id: &str| ["bus", "travel"].iter().position(|k| *k == id).map_or(-1, |p| p as i32);
        sides.sort_by_key(|(nb, _)| std::cmp::Reverse(rank(kind_of(*nb).unwrap_or(""))));
        sides.truncate(1);
    }
    sides.into_iter().map(|(_, right)| right).collect()
}

/// Traffic direction as an arrow `h` tall and `0.64 h` across its head: up runs
/// away from you, down comes toward you. The paler outline under the arrow keeps
/// it readable over hatching.
pub fn dir_glyph(direction: &str, cx: f64, cy: f64, h: f64) -> String {
    let d = if direction == "toward" { 1.0 } else { -1.0 };
    let r = |v: f64| (v * 100.0).round() / 100.0;
    let (half, head) = (h / 2.0, r(h * 0.32));
    let path = format!(
        "M{cx},{} V{} M{},{} L{cx},{} L{},{}",
        cy - half * d,
        cy + half * d,
        r(cx - head),
        r(cy + (half - head) * d),
        cy + half * d,
        r(cx + head),
        r(cy + (half - head) * d)
    );
    format!("<path class=\"dir-halo\" d=\"{path}\"/><path class=\"dir\" d=\"{path}\"/>")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::street::model::Editor;

    #[test]
    fn a_curb_is_on_the_edges_that_face_a_road_piece_and_an_island_on_one_only() {
        use crate::shared::catalogue::kind_index;
        let mut e = Editor::new(0);
        let v = e.view();
        // The left sidewalk faces parking on its right, the right one on its left; a driving lane has no curb.
        assert_eq!(curb_edges(&v.segments, 0), vec![true]);
        assert_eq!(curb_edges(&v.segments, 5), vec![false]);
        assert_eq!(curb_edges(&v.segments, 2), Vec::<bool>::new());
        assert_eq!(curb_edges(&v.segments, 99), Vec::<bool>::new());
        // A bike lane between parking and a driving lane has its curb on both edges, a boarding island on one.
        let u = e.add(kind_index("bike").unwrap(), 2);
        assert_eq!(curb_edges(&e.view().segments, 2), vec![false, true]);
        let island = crate::shared::catalogue::CURBS.iter().position(|c| c.id == "island").unwrap();
        assert!(e.set_curb(u, Some(island)));
        // The island stands on the driving lane's side (the right), not the parking's.
        assert_eq!(curb_edges(&e.view().segments, 2), vec![true]);
        assert_eq!((curb_width_mm("granite"), curb_width_mm("planted"), curb_width_mm("island")), (150.0, 600.0, 900.0));
        assert_eq!((curb_width_mm("kassel"), curb_width_mm("bikefriendly")), (250.0, 300.0));
    }

    #[test]
    fn a_direction_arrow_points_up_away_from_you_and_down_toward_you() {
        assert_eq!(
            dir_glyph("away", 10.0, 20.0, 28.0),
            "<path class=\"dir-halo\" d=\"M10,34 V6 M1.04,14.96 L10,6 L18.96,14.96\"/><path class=\"dir\" d=\"M10,34 V6 M1.04,14.96 L10,6 L18.96,14.96\"/>"
        );
        assert!(dir_glyph("toward", 10.0, 20.0, 28.0).contains("d=\"M10,6 V34 M1.04,25.04 L10,34 L18.96,25.04\""));
    }
}
