//! A zone's own speed limit (#148). `Zone.speed` is `Undecided` when the zone follows the street's `speed`, the
//! base the way `segments: None` is the base section; a street of one zone has no zone to hold one.

use super::{Speed, Street, Zone};

/// The speed of zone `z`: its own, else the street's. With no zones the street's.
pub(super) fn speed_in(base: Speed, zones: &[Zone], z: usize) -> Speed {
    match zones.get(z) {
        Some(zone) if !zone.speed.is_undecided() => zone.speed,
        _ => base,
    }
}

/// The highest speed any zone has, when one has a speed.
pub(super) fn max_kmh(base: Speed, zones: &[Zone]) -> Option<u16> {
    (0..zones.len().max(1)).filter_map(|z| speed_in(base, zones, z).kmh()).max()
}

/// The speed a transition between zones `b` and `b + 1` is sized by: the higher of the two, since the taper must
/// serve the faster traffic.
pub(super) fn taper_kmh(base: Speed, zones: &[Zone], b: usize) -> Option<u16> {
    [speed_in(base, zones, b).kmh(), speed_in(base, zones, b + 1).kmh()].into_iter().flatten().max()
}

impl Street {
    /// The speed of zone `z`, its own or else the street's.
    pub fn zone_speed(&self, z: usize) -> Speed {
        speed_in(self.speed, &self.zones, z)
    }

    /// The limit of zone `z` in kilometres an hour, when it has one.
    pub fn zone_speed_kmh(&self, z: usize) -> Option<u16> {
        self.zone_speed(z).kmh()
    }

    /// The highest limit any zone has: what an area's speed rule judges the street by.
    pub fn max_speed_kmh(&self) -> Option<u16> {
        max_kmh(self.speed, &self.zones)
    }

    /// The speed the transition at boundary `b` (between zones `b` and `b + 1`) is sized by.
    pub fn taper_speed_kmh(&self, b: usize) -> Option<u16> {
        taper_kmh(self.speed, &self.zones, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::street::model::{Side, zoned};

    fn two(base: Speed, second: Speed) -> Street {
        let mut s = zoned(Street::sample(0, Side::Right), &[0, 20_000], 40_000);
        s.speed = base;
        s.zones[1].speed = second;
        s
    }

    #[test]
    fn a_zone_follows_the_street_unless_it_has_a_speed() {
        let s = two(Speed::Kmh(50), Speed::Kmh(30));
        assert_eq!((s.zone_speed_kmh(0), s.zone_speed_kmh(1)), (Some(50), Some(30)));
        let s = two(Speed::Kmh(50), Speed::Undecided);
        assert_eq!(s.zone_speed_kmh(1), Some(50));
        assert_eq!(two(Speed::NotSet, Speed::Undecided).zone_speed_kmh(1), None);
    }

    #[test]
    fn a_zone_may_have_no_limit_where_the_street_has_one() {
        let s = two(Speed::Kmh(50), Speed::NotSet);
        assert_eq!((s.zone_speed_kmh(1), s.max_speed_kmh()), (None, Some(50)));
    }

    #[test]
    fn the_highest_zone_speed_judges_the_street_and_sizes_a_taper() {
        let s = two(Speed::Kmh(30), Speed::Kmh(50));
        assert_eq!((s.max_speed_kmh(), s.taper_speed_kmh(0)), (Some(50), Some(50)));
        let s = two(Speed::NotSet, Speed::NotSet);
        assert_eq!((s.max_speed_kmh(), s.taper_speed_kmh(0)), (None, None));
    }

    #[test]
    fn a_street_with_no_zones_has_its_own_speed() {
        let mut s = Street::sample(0, Side::Right);
        s.speed = Speed::Kmh(40);
        assert_eq!((s.zone_speed_kmh(0), s.max_speed_kmh()), (Some(40), Some(40)));
    }

    #[test]
    fn the_zone_speed_is_saved_as_speed_kmh_and_left_out_when_it_follows() {
        let s = two(Speed::Kmh(50), Speed::Kmh(30));
        let json = serde_json::to_value(&s.zones).unwrap();
        assert_eq!(json[1]["speed_kmh"], 30);
        assert!(json[0].get("speed_kmh").is_none());
        let none = serde_json::to_value(&two(Speed::Kmh(50), Speed::NotSet).zones).unwrap();
        assert!(none[1]["speed_kmh"].is_null() && none[1].as_object().unwrap().contains_key("speed_kmh"));
        let back: Vec<Zone> = serde_json::from_value(json).unwrap();
        assert_eq!(back, s.zones);
    }

    use crate::street::model::{Editor, Keep, Said, ZoneEnd, ZoneFrame};

    const LEN: i32 = 60_000;

    /// A street of 50 km/h split in two at 30 m, anchored at its first end; the second zone is selected.
    fn split(speed: Speed) -> Editor {
        let mut s = Street::sample(0, Side::Right);
        s.speed = speed;
        let mut e = Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: LEN, from_end: ZoneEnd::First }));
        e.split_zone(Some(30_000)).unwrap();
        e
    }

    #[test]
    fn the_speed_of_a_split_street_is_set_for_the_selected_zone_only() {
        let mut e = split(Speed::Kmh(50));
        assert_eq!(e.zone(), 1);
        assert!(e.set_speed(Some(30)));
        let s = e.snapshot();
        assert_eq!((s.speed, s.zones[0].speed, s.zones[1].speed), (Speed::Kmh(50), Speed::Undecided, Speed::Kmh(30)));
        assert_eq!(e.view().speed_kmh, Some(30));
        assert_eq!(e.view().street_speed_kmh, Some(50));
        assert!(!e.view().speed_follows);
        let zone = Said::new("zone-unnamed-mid").with("n", crate::street::model::Arg::Num(2));
        assert_eq!(
            e.view().revisions.last().unwrap().label,
            Said::new("rev-zone-speed").with("zone", crate::street::model::Arg::Said(Box::new(zone))).with("kmh", crate::street::model::Arg::Num(30))
        );
        e.select_zone(0);
        assert_eq!((e.view().speed_kmh, e.view().speed_follows), (Some(50), true));
    }

    #[test]
    fn a_zone_speed_is_one_undoable_step() {
        let mut e = split(Speed::Kmh(50));
        assert!(e.set_speed(Some(30)));
        assert!(e.undo());
        assert_eq!(e.snapshot().zones[1].speed, Speed::Undecided);
        assert!(e.redo());
        assert_eq!(e.snapshot().zones[1].speed, Speed::Kmh(30));
        assert!(e.reset());
        assert_eq!(e.snapshot().zones.len(), 0, "Start over restores the one-zone street, speed included");
    }

    #[test]
    fn a_zone_may_be_given_no_limit_and_may_follow_the_street_again() {
        let mut e = split(Speed::Kmh(50));
        assert!(e.set_speed(None));
        assert_eq!(e.snapshot().zones[1].speed, Speed::NotSet);
        assert_eq!(e.view().speed_kmh, None);
        assert!(!e.set_speed(None), "already so");
        assert!(e.follow_street_speed());
        assert_eq!(e.snapshot().zones[1].speed, Speed::Undecided);
        assert!(!e.follow_street_speed(), "already follows");
    }

    #[test]
    fn a_speed_equal_to_the_streets_is_the_streets() {
        let mut e = split(Speed::Kmh(50));
        assert!(e.set_speed(Some(30)));
        assert!(e.set_speed(Some(50)));
        assert_eq!(e.snapshot().zones[1].speed, Speed::Undecided);
    }

    #[test]
    fn a_street_of_one_zone_sets_the_streets_speed() {
        let mut s = Street::sample(0, Side::Right);
        s.speed = Speed::Kmh(50);
        let mut e = Editor::from_street(&s, &s, 0);
        assert!(!e.follow_street_speed());
        assert!(e.set_speed(Some(30)));
        assert_eq!(e.snapshot().speed, Speed::Kmh(30));
    }

    #[test]
    fn splitting_keeps_the_speed_in_both_halves_and_merging_asks_when_they_differ() {
        let mut e = split(Speed::Kmh(50));
        assert!(e.set_speed(Some(30)));
        e.split_zone(Some(45_000)).unwrap();
        let s = e.snapshot();
        assert_eq!((s.zones[1].speed, s.zones[2].speed), (Speed::Kmh(30), Speed::Kmh(30)));
        e.select_zone(0);
        assert!(e.merge_needs_choice(true), "same pieces, different speeds");
        e.select_zone(1);
        assert!(!e.merge_needs_choice(true), "the same speed and pieces");
    }

    #[test]
    fn merging_keeps_the_speed_of_the_zone_kept_and_one_zone_left_gives_it_to_the_street() {
        let mut e = split(Speed::Kmh(50));
        assert!(e.set_speed(Some(30)));
        e.merge_zone(false, Keep::Selected).unwrap();
        let s = e.snapshot();
        assert!(s.zones.is_empty(), "one zone left is no zoning");
        assert_eq!(s.speed, Speed::Kmh(30));
    }

    #[test]
    fn the_street_speed_check_of_a_table_is_the_speed_of_the_zone_it_lies_in() {
        let mut e = split(Speed::Kmh(60));
        assert!(e.set_speed(Some(30)));
        e.add_calming(crate::street::model::CalmingKind::Table, 40_000, 7_000).unwrap();
        let speed = |e: &Editor| e.view().checks.into_iter().find(|c| c.id == "calming-speed").unwrap();
        assert!(speed(&e).ok, "the table is in the 30 km/h zone");
        let mut e = split(Speed::Kmh(60));
        assert!(e.set_speed(Some(30)));
        e.add_calming(crate::street::model::CalmingKind::Table, 10_000, 7_000).unwrap();
        let c = speed(&e);
        assert!(!c.ok, "the table is in the 60 km/h zone");
        assert!(c.zone.is_some(), "and the check names it");
    }

    #[test]
    fn an_areas_speed_rule_judges_the_fastest_zone() {
        use crate::restriction::model::{Conflict, Rule, conflicts, speed_unknown};
        let mut e = split(Speed::Kmh(50));
        assert!(e.set_speed(Some(30)));
        let s = e.snapshot();
        assert_eq!(conflicts(Rule::SpeedLimit { kmh: 40 }, &s), [Conflict::Speed { kmh: 50 }]);
        assert!(conflicts(Rule::SpeedLimit { kmh: 50 }, &s).is_empty());
        assert!(!speed_unknown(Rule::SpeedLimit { kmh: 40 }, &s));
    }

    #[test]
    fn the_end_zone_a_junction_reads_has_the_zones_speed() {
        let mut e = split(Speed::Kmh(50));
        assert!(e.set_speed(Some(30)));
        let s = e.snapshot();
        assert_eq!(s.at_end(true).speed, Speed::Kmh(50));
        assert_eq!(s.at_end(false).speed, Speed::Kmh(30));
    }
}
