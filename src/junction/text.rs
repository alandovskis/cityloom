//! What the page says in words after the junction changes: the status line, and
//! what a screen reader is told about an edit or a selection.

use crate::junction::model::CONTROLS;
use crate::junction::read_model::JView;
use crate::shared::units::Units;

pub const UNDONE: &str = "Undone.";
pub const STARTED_OVER: &str = "Started over from the junction as it is today.";

/// "5 streets".
pub fn arms_text(v: &JView) -> String {
    format!("{} streets", v.arms.len())
}

/// The status line: what needs attention, or that every check passes.
pub fn fit_text(v: &JView) -> String {
    let bad: Vec<String> = v.checks.iter().filter(|c| !c.ok).map(|c| c.label.to_lowercase()).collect();
    match bad.len() {
        0 => format!("{}, {}. Every check passes.", arms_text(v), CONTROLS[v.control_index].name.to_lowercase()),
        1 => format!("One check needs attention: {}.", bad[0]),
        n => format!("{n} checks need attention: {}.", bad.join(", ")),
    }
}

/// What the last edit was, and where the junction stands.
pub fn edit_text(v: &JView) -> Option<String> {
    v.revisions.last().map(|r| format!("{}. {}", r.label, fit_text(v)))
}

/// A sample junction was loaded.
pub fn sample_text(v: &JView) -> String {
    format!("{}. {}", v.name, fit_text(v))
}

/// What is selected, for a screen reader; nothing when nothing is.
pub fn selection_text(v: &JView, units: Units) -> Option<String> {
    let s = &v.selected;
    let kind = s.kind?;
    let arm = v.arms.iter().find(|a| a.uid == s.uid);
    let label = arm.map_or("", |a| a.label.as_str());
    Some(match kind {
        "bus" => "Bus lane across the middle".to_string(),
        "cycle" => format!("Cycle track, {} wide", units.length(v.ring.as_ref().and_then(|r| r.cycle_mm).unwrap_or(0))),
        "lane" => format!("Lane {} of {}, {label}", s.lane + 1, arm.map_or(0, |a| a.lanes.len())),
        "arm" => format!("{label}, {} degrees", arm.map_or(0, |a| a.bearing)),
        "corner" => format!("Corner after {label}, {} radius", units.length(v.corners.iter().find(|c| c.uid == s.uid).map_or(0, |c| c.radius_mm))),
        _ => format!("Crossing on {label}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::junction::model::*;

    fn arm_at(j: &Junction, bearing: i32) -> u32 {
        j.current().arms.iter().find(|a| a.bearing == bearing).unwrap().uid
    }

    #[test]
    fn a_junction_that_works_says_how_many_streets_and_how_it_is_run() {
        let j = Junction::new(0);
        assert_eq!(fit_text(&j.view()), "4 streets, traffic signal. Every check passes.");
        assert_eq!(arms_text(&j.view()), "4 streets");
    }

    #[test]
    fn a_junction_that_fails_one_check_names_it() {
        let mut j = Junction::new(0);
        let n = arm_at(&j, 0);
        j.set_island(n, false); // the avenue is then too far to cross in one go
        assert_eq!(fit_text(&j.view()), "One check needs attention: crossing distance.");
    }

    #[test]
    fn a_junction_that_fails_several_counts_and_names_them() {
        let mut j = Junction::new(0);
        let n = arm_at(&j, 0);
        j.set_corner(n, 11_000);
        assert_eq!(fit_text(&j.view()), "2 checks need attention: slow turns at crossings, sidewalk survives the corner.");
    }

    #[test]
    fn an_edit_is_announced_by_its_label_and_where_the_junction_stands() {
        let mut j = Junction::new(0);
        assert_eq!(edit_text(&j.view()), None);
        let n = arm_at(&j, 0);
        j.set_corner(n, 7_000);
        assert_eq!(edit_text(&j.view()).unwrap(), "Corner after Sample Avenue 2 (north): 7000 mm radius. 4 streets, traffic signal. Every check passes.");
    }

    #[test]
    fn a_sample_is_announced_by_its_name() {
        assert_eq!(sample_text(&Junction::new(1).view()), "Street and lane. 3 streets, side streets stop. Every check passes.");
    }

    #[test]
    fn nothing_selected_says_nothing() {
        assert_eq!(selection_text(&Junction::new(0).view(), Units::Metres), None);
    }

    #[test]
    fn each_kind_of_selection_says_what_it_is() {
        let mut j = Junction::new(0);
        let n = arm_at(&j, 0);
        let say = |j: &Junction, u: Units| selection_text(&j.view(), u).unwrap();
        j.select(Target::Arm(n));
        assert_eq!(say(&j, Units::Metres), "Sample Avenue 2 (north), 0 degrees");
        j.select(Target::Corner(n));
        assert_eq!(say(&j, Units::Metres), "Corner after Sample Avenue 2 (north), 6.0 m radius");
        assert_eq!(say(&j, Units::Feet), "Corner after Sample Avenue 2 (north), 19.7 ft radius");
        j.select(Target::Crossing(n));
        assert_eq!(say(&j, Units::Metres), "Crossing on Sample Avenue 2 (north)");
        j.select(Target::Lane(n, 1));
        assert_eq!(say(&j, Units::Metres), "Lane 2 of 2, Sample Avenue 2 (north)");
    }

    #[test]
    fn the_bus_lane_and_cycle_track_say_what_they_are() {
        let mut j = Junction::new(0);
        j.set_control(ROUNDABOUT);
        let (n, s) = (arm_at(&j, 0), arm_at(&j, 180));
        j.set_bus(Some((n, s)));
        j.set_cycle(Some(2_000));
        j.select(Target::Bus);
        assert_eq!(selection_text(&j.view(), Units::Metres).unwrap(), "Bus lane across the middle");
        j.select(Target::Cycle);
        assert_eq!(selection_text(&j.view(), Units::Metres).unwrap(), "Cycle track, 2.0 m wide");
    }
}
