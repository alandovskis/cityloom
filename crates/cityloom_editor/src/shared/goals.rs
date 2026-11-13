//! Guided design goals (#72): a person picks what they want a place to do, and the editors point at the checks and the
//! Atlas measures that bear on it. The guidance is only ever shown: choosing a goal reads the design and changes
//! nothing in it, and the choice is a preference of the page (kept like the panel tab), not part of any city.
//!
//! The catalogue is CityLoom's own placeholder reading of the Atlas, like the measures it points at (`atlas.rs`).
//! Every word is a message (`goal-*`, in `street/i18n`); this file holds ids and message keys only.

use std::rc::Rc;

use leptos::prelude::{ArcRwSignal, Get, Set};

use crate::shared::ports::Storage;

/// Where the chosen goal is kept between pages and visits.
pub const GOAL_KEY: &str = "cityloom-goal";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Goal {
    ThroughTraffic,
    SchoolRoute,
    Buses,
}

/// The editor that shows the guidance: a street's cross-section or a junction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Street,
    Junction,
}

/// A measure to consider, why it fits the goal and what it is likely to cost another goal.
#[derive(Clone, Copy, Debug)]
pub struct Pick {
    /// The Atlas code (`shared::atlas`), named by `atlas::name_key`.
    pub code: &'static str,
    pub place: Place,
    /// The message that says why it fits.
    pub fit: &'static str,
    /// The goal it is likely to work against, and the message that says how.
    pub conflict: Option<(Goal, &'static str)>,
}

const fn pick(code: &'static str, place: Place, fit: &'static str, conflict: Option<(Goal, &'static str)>) -> Pick {
    Pick { code, place, fit, conflict }
}

impl Goal {
    pub const ALL: [Goal; 3] = [Goal::ThroughTraffic, Goal::SchoolRoute, Goal::Buses];

    pub fn id(self) -> &'static str {
        match self {
            Goal::ThroughTraffic => "through-traffic",
            Goal::SchoolRoute => "school-route",
            Goal::Buses => "buses",
        }
    }

    pub fn from_id(id: &str) -> Option<Goal> {
        Goal::ALL.into_iter().find(|g| g.id() == id)
    }

    /// The message that names the goal.
    pub fn name(self) -> &'static str {
        match self {
            Goal::ThroughTraffic => "goal-through-traffic-name",
            Goal::SchoolRoute => "goal-school-route-name",
            Goal::Buses => "goal-buses-name",
        }
    }

    /// The message that says what the goal asks of a design.
    pub fn intro(self) -> &'static str {
        match self {
            Goal::ThroughTraffic => "goal-through-traffic-intro",
            Goal::SchoolRoute => "goal-school-route-intro",
            Goal::Buses => "goal-buses-intro",
        }
    }

    /// The check ids (street and junction) the goal bears on: a check matches an entry equal to it, or an entry
    /// ending in `-` that it starts with.
    pub fn focus(self) -> &'static [&'static str] {
        match self {
            Goal::ThroughTraffic => &["calming-", "crossing", "lanes-cover", "lanes-follow"],
            Goal::SchoolRoute => &["crossing", "edges", "access", "curb-crossing", "curb-near", "curb-sidewalk", "green-sightline", "calming-"],
            Goal::Buses => &["stop-", "transit", "lane-drop", "transition-room", "fits"],
        }
    }

    /// Whether the check `id` bears on the goal.
    pub fn bears_on(self, id: &str) -> bool {
        self.focus().iter().any(|f| if f.ends_with('-') { id.starts_with(f) } else { id == *f })
    }

    /// The measures worth considering for the goal.
    pub fn picks(self) -> &'static [Pick] {
        match self {
            Goal::ThroughTraffic => &THROUGH,
            Goal::SchoolRoute => &SCHOOL,
            Goal::Buses => &BUSES,
        }
    }

    /// The picks of the goal that the editor `place` models.
    pub fn picks_at(self, place: Place) -> impl Iterator<Item = &'static Pick> {
        self.picks().iter().filter(move |p| p.place == place)
    }
}

use Place::{Junction, Street};

const THROUGH: [Pick; 5] = [
    pick("A1", Street, "goal-fit-a1-through", Some((Goal::Buses, "goal-conflict-a1-through-buses"))),
    pick("A2", Street, "goal-fit-a2-through", None),
    pick("N1", Junction, "goal-fit-n1-through", Some((Goal::Buses, "goal-conflict-n1-through-buses"))),
    pick("L3", Junction, "goal-fit-l3-through", Some((Goal::SchoolRoute, "goal-conflict-l3-through-school"))),
    pick("L4", Junction, "goal-fit-l4-through", Some((Goal::SchoolRoute, "goal-conflict-l4-through-school"))),
];

const SCHOOL: [Pick; 3] = [
    pick("A2", Street, "goal-fit-a2-school", None),
    pick("N1", Junction, "goal-fit-n1-school", Some((Goal::Buses, "goal-conflict-n1-school-buses"))),
    pick("M1", Junction, "goal-fit-m1-school", None),
];

const BUSES: [Pick; 12] = [
    pick("B1", Street, "goal-fit-b1-buses", None),
    pick("C1", Street, "goal-fit-c1-buses", None),
    pick("E1", Street, "goal-fit-e1-buses", Some((Goal::SchoolRoute, "goal-conflict-e1-buses-school"))),
    pick("A1", Street, "goal-fit-a1-buses", None),
    pick("G1", Junction, "goal-fit-g1-buses", None),
    pick("G2", Junction, "goal-fit-g2-buses", None),
    pick("G3", Junction, "goal-fit-g3-buses", None),
    pick("H1", Junction, "goal-fit-h1-buses", Some((Goal::ThroughTraffic, "goal-conflict-h1-buses-through"))),
    pick("H2", Junction, "goal-fit-h2-buses", None),
    pick("M1", Junction, "goal-fit-m1-buses", Some((Goal::SchoolRoute, "goal-conflict-m1-buses-school"))),
    pick("M2", Junction, "goal-fit-m2-buses", None),
    pick("N1", Junction, "goal-fit-n1-buses", None),
];

/// The chosen goal, kept for the page and between visits. Choosing and clearing only write this preference.
#[derive(Clone)]
pub struct GoalChoice {
    storage: Rc<dyn Storage>,
    chosen: ArcRwSignal<Option<Goal>>,
}

impl GoalChoice {
    /// The goal kept from before, if it is still one.
    pub fn new(storage: Rc<dyn Storage>) -> GoalChoice {
        let chosen = storage.recall(GOAL_KEY).and_then(|id| Goal::from_id(&id));
        GoalChoice { storage, chosen: ArcRwSignal::new(chosen) }
    }

    /// The chosen goal, watched.
    pub fn get(&self) -> Option<Goal> {
        self.chosen.get()
    }

    /// Chooses a goal, or clears the choice with `None`.
    pub fn set(&self, goal: Option<Goal>) {
        self.chosen.set(goal);
        self.storage.remember(GOAL_KEY, goal.map_or("", Goal::id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::atlas::{MEASURES, Where};
    use crate::shared::ports::MemoryStorage;

    #[test]
    fn ids_round_trip() {
        for g in Goal::ALL {
            assert_eq!(Goal::from_id(g.id()), Some(g));
        }
        assert_eq!(Goal::from_id(""), None);
        assert_eq!(Goal::from_id("nonsense"), None);
    }

    #[test]
    fn every_pick_is_an_atlas_measure_modelled_where_it_is_shown() {
        for g in Goal::ALL {
            for p in g.picks() {
                let m = MEASURES.iter().find(|m| m.code == p.code).unwrap_or_else(|| panic!("{} is not in the Atlas table", p.code));
                let expected = if p.place == Place::Street { Where::Street } else { Where::Junction };
                assert_eq!(m.place, expected, "{} is shown in the wrong editor for {}", p.code, g.id());
            }
        }
    }

    #[test]
    fn every_goal_has_something_to_point_at_in_each_editor() {
        for g in Goal::ALL {
            assert!(!g.focus().is_empty());
            assert!(g.picks_at(Place::Street).count() + g.picks_at(Place::Junction).count() > 0, "{}", g.id());
        }
        assert!(Goal::SchoolRoute.picks_at(Place::Street).count() > 0);
    }

    #[test]
    fn a_conflict_names_another_goal_and_a_goal_does_not_repeat_a_measure() {
        for g in Goal::ALL {
            let mut seen = Vec::new();
            for p in g.picks() {
                assert!(!seen.contains(&p.code), "{} twice in {}", p.code, g.id());
                seen.push(p.code);
                if let Some((other, _)) = p.conflict {
                    assert_ne!(other, g, "{} conflicts with its own goal", p.code);
                }
            }
        }
    }

    #[test]
    fn a_goal_bears_on_its_checks_by_id_or_family_and_on_no_other() {
        assert!(Goal::Buses.bears_on("stop-length"));
        assert!(Goal::Buses.bears_on("transit"));
        assert!(!Goal::Buses.bears_on("crossing"));
        assert!(Goal::SchoolRoute.bears_on("crossing"));
        assert!(Goal::SchoolRoute.bears_on("calming-speed"));
        assert!(!Goal::SchoolRoute.bears_on("stop-length"));
        assert!(!Goal::ThroughTraffic.bears_on("crossings"), "an id is matched whole unless the entry ends in a dash");
    }

    #[test]
    fn the_choice_is_kept_and_cleared_and_a_stale_one_is_ignored() {
        let storage = Rc::new(MemoryStorage::default());
        let c = GoalChoice::new(storage.clone());
        assert_eq!(c.get(), None);
        c.set(Some(Goal::Buses));
        assert_eq!(GoalChoice::new(storage.clone()).get(), Some(Goal::Buses));
        c.set(None);
        assert_eq!(GoalChoice::new(storage.clone()).get(), None);
        storage.remember(GOAL_KEY, "retired-goal");
        assert_eq!(GoalChoice::new(storage).get(), None);
    }
}
