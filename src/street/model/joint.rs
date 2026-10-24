//! The zone across the nearest boundary: what the 2.5D draws beyond the stretch it shows.

use serde::Serialize;

use super::zones::{ZoneEnd, zone_label, zone_label_mid};
use super::{Change, Editor, Host, Said, SegView};

/// The stretch the 2.5D shows, in millimetres (a test pins it to `iso::STRETCH_MM`).
pub const SHOWN_MM: i32 = 40_000;

/// How far past the stretch shown a boundary still counts as near, in millimetres.
pub const JOINT_REACH_MM: i32 = 20_000;

/// The neighbouring zone's section, as the pieces are at the shown time.
#[derive(Serialize)]
pub struct Joint {
    pub segments: Vec<SegView>,
    pub total_mm: i32,
    /// The neighbour lies beyond the selected zone in the direction a section is laid out (first end toward last).
    pub far: bool,
    /// The boundary's position from the anchored end.
    pub at_mm: i32,
    /// The neighbour's index.
    pub zone: usize,
    pub label: Said,
    pub label_mid: Said,
    /// The transition at this boundary, when the two sections need one; the 2.5D draws it.
    #[serde(skip)]
    pub taper: Option<Taper>,
}

/// The transition at a joint's boundary, seen from the zone that holds it (the host).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Taper {
    /// The host is the selected zone (otherwise it is the neighbour across the boundary).
    pub host_is_selected: bool,
    pub length_mm: i32,
    /// Every piece of both sides, as `Transition::changes`, but `before` is always the host's side and `after` the
    /// other zone's.
    pub changes: Vec<Change>,
}

/// The neighbour of the `selected` zone across the nearest boundary, and whether it lies `far` (beyond the zone in
/// the drawing's direction). `zones` are each zone's `(from_mm, to_mm)`. The part shown is the zone's first
/// `SHOWN_MM`; the end boundary is near up to `JOINT_REACH_MM` past it, the start boundary always. The end
/// boundary wins when both are near.
pub fn near_boundary(zones: &[(i32, i32)], selected: usize, from_end: ZoneEnd) -> Option<(usize, bool)> {
    let &(from, to) = zones.get(selected)?;
    let next = (selected + 1 < zones.len() && to <= from + SHOWN_MM + JOINT_REACH_MM).then_some(selected + 1);
    let prev = selected.checked_sub(1);
    let (zone, is_next) = match (next, prev) {
        (Some(n), _) => (n, true),
        (None, Some(p)) => (p, false),
        (None, None) => return None,
    };
    Some((zone, is_next == from_end.is_first()))
}

impl Editor {
    pub(super) fn joint(&self) -> Option<Joint> {
        let content = &self.states[self.cursor].content;
        if content.zones.len() < 2 {
            return None;
        }
        let length_mm = content.length_mm.or(self.frame.map(|f| f.length_mm)).unwrap_or(0);
        let spans: Vec<(i32, i32)> = (0..content.zones.len()).map(|z| content.span(z, length_mm)).collect();
        let (zone, far) = near_boundary(&spans, self.zone, content.from_end)?;
        let at_mm = if zone > self.zone { spans[self.zone].1 } else { spans[self.zone].0 };
        let segments = self.seg_views(content.section(zone));
        let boundary = zone.min(self.zone);
        let taper = self.transitions().into_iter().find(|(b, _)| *b == boundary).map(|(_, t)| {
            let host = if t.host == Host::Near { boundary } else { boundary + 1 };
            let changes =
                if host == boundary { t.changes } else { t.changes.into_iter().map(|c| Change { kind: c.kind, before: c.after, after: c.before }).collect() };
            Taper { host_is_selected: host == self.zone, length_mm: t.length_mm, changes }
        });
        Some(Joint {
            total_mm: segments.iter().map(|s| s.width_mm).sum(),
            segments,
            far,
            at_mm,
            zone,
            label: zone_label(&content.zones[zone], zone + 1),
            label_mid: zone_label_mid(&content.zones[zone], zone + 1),
            taper,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixtures::two_zones;
    use super::*;
    use crate::shared::said::Arg;

    #[test]
    fn a_joint_carries_the_transition_from_the_hosts_side() {
        // Zone 2 has one travel lane fewer: the far zone hosts the transition.
        let t = joined_taper(1).expect("the sections differ");
        assert!(t.host_is_selected);
        assert_eq!(t.length_mm, 53_300);
        assert!(t.changes.iter().any(|c| c.before.is_none() && c.after.is_some()), "the lane the host lacks");
        let t = joined_taper(0).expect("the same boundary seen from zone 1");
        assert!(!t.host_is_selected, "zone 2 hosts it, and zone 1 is the neighbour");
        assert!(t.changes.iter().any(|c| c.before.is_none() && c.after.is_some()), "before is the host's side from either zone");
        let mut same = two_zones(false);
        assert!(same.select_zone(1));
        assert!(same.view().zones.joint.expect("a joint").taper.is_none());
    }

    fn joined_taper(selected: usize) -> Option<Taper> {
        let mut e = two_zones(true);
        assert!(e.select_zone(selected));
        e.view().zones.joint.expect("a joint").taper
    }

    #[test]
    fn a_one_zone_street_has_no_joint() {
        assert_eq!(near_boundary(&[(0, 90_000)], 0, ZoneEnd::First), None);
        assert!(Editor::new(0).view().zones.joint.is_none());
    }

    #[test]
    fn the_end_boundary_is_near_up_to_20_m_past_the_stretch_and_not_beyond() {
        assert_eq!(near_boundary(&[(0, 60_000), (60_000, 99_000)], 0, ZoneEnd::First), Some((1, true)));
        assert_eq!(near_boundary(&[(0, 61_000), (61_000, 99_000)], 0, ZoneEnd::First), None, "the first zone has no start boundary");
    }

    #[test]
    fn the_start_boundary_is_taken_when_the_end_is_not_near() {
        let z = [(0, 50_000), (50_000, 150_000), (150_000, 200_000)];
        assert_eq!(near_boundary(&z, 1, ZoneEnd::First), Some((0, false)));
    }

    #[test]
    fn the_end_boundary_wins_when_both_are_near() {
        let z = [(0, 50_000), (50_000, 100_000), (100_000, 200_000)];
        assert_eq!(near_boundary(&z, 1, ZoneEnd::First), Some((2, true)));
    }

    #[test]
    fn the_last_zone_has_its_start_boundary_only() {
        let z = [(0, 50_000), (50_000, 150_000)];
        assert_eq!(near_boundary(&z, 1, ZoneEnd::First), Some((0, false)));
    }

    #[test]
    fn anchored_at_the_last_end_the_sides_swap() {
        let z = [(0, 50_000), (50_000, 100_000), (100_000, 200_000)];
        assert_eq!(near_boundary(&z, 1, ZoneEnd::Last), Some((2, false)), "the next zone is nearer the first end");
        let z = [(0, 50_000), (50_000, 150_000), (150_000, 200_000)];
        assert_eq!(near_boundary(&z, 1, ZoneEnd::Last), Some((0, true)));
    }

    #[test]
    fn the_view_carries_the_neighbours_section_and_labels() {
        let mut e = two_zones(true);
        assert!(e.select_zone(1));
        let j = e.view().zones.joint.expect("zone 2's start is near");
        assert_eq!((j.zone, j.far, j.at_mm), (0, false, 50_000));
        assert_eq!(j.segments.len(), Editor::new(0).view().segments.len());
        assert_eq!(j.total_mm, j.segments.iter().map(|s| s.width_mm).sum::<i32>());
        assert_eq!(j.label, Said::new("zone-unnamed").with("n", Arg::Num(1)));
        assert!(e.select_zone(0));
        assert_eq!(e.view().zones.joint.unwrap().segments.len(), Editor::new(0).view().segments.len() - 1, "from zone 1 the neighbour is zone 2's own section");
    }

    #[test]
    fn the_neighbours_pieces_follow_the_shown_time() {
        use super::super::{KINDS, Variant};
        use crate::shared::catalogue::kind_index;
        let mut e = two_zones(true);
        assert!(e.select_zone(1));
        {
            let content = &mut e.states[e.cursor].content;
            let own = content.zones[0].segments.get_or_insert_with(Vec::new);
            *own = content.segments.clone();
            let lane = own.iter_mut().find(|g| KINDS[g.kind].id == "travel").unwrap();
            let bus = kind_index("bus").unwrap();
            lane.variants.push(Variant {
                kind: bus,
                material: KINDS[bus].materials[0],
                direction: lane.direction,
                from_min: 8 * 60,
                to_min: 9 * 60,
                days: None,
            });
        }
        let at = |e: &mut Editor, min: i32| {
            e.set_time(min);
            let j = e.view().zones.joint.unwrap();
            j.segments.iter().filter(|s| Some(s.kind) == kind_index("bus")).count()
        };
        assert_eq!(at(&mut e, 8 * 60), 1);
        assert_eq!(at(&mut e, 12 * 60), 0);
    }
}
