//! The folds of a junction's signal plan level (CLM-49): one per signal option, closed at first, each saying on its line
//! what the option has set and how many warnings (its findings) it has, so a closed fold hides no problem.

use crate::junction::model::read_model::JView;
use crate::shared::i18n::{Args, I18n};
use crate::shared::said::say_now;
use crate::shared::units::Units;

/// One signal option of the plan level, in the level's order: the time-of-day plans above the phases, the others under
/// them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignalFold {
    Plans,
    Calls,
    Actuation,
    Bikes,
    Priority,
    Preemption,
}

impl SignalFold {
    pub const ALL: [SignalFold; 6] =
        [SignalFold::Plans, SignalFold::Calls, SignalFold::Actuation, SignalFold::Bikes, SignalFold::Priority, SignalFold::Preemption];

    /// The fold's element id.
    pub fn id(self) -> &'static str {
        match self {
            SignalFold::Plans => "jn-fold-plans",
            SignalFold::Calls => "jn-fold-calls",
            SignalFold::Actuation => "jn-fold-actuation",
            SignalFold::Bikes => "jn-fold-bikes",
            SignalFold::Priority => "jn-fold-priority",
            SignalFold::Preemption => "jn-fold-preemption",
        }
    }

    /// The message that names it: the heading of the section it folds.
    pub fn heading(self) -> &'static str {
        match self {
            SignalFold::Plans => "jn-plans-heading",
            SignalFold::Calls => "jn-calls-heading",
            SignalFold::Actuation => "jn-act-heading",
            SignalFold::Bikes => "jn-bikes-heading",
            SignalFold::Priority => "jn-transit-heading",
            SignalFold::Preemption => "jn-preempt-heading",
        }
    }
}

/// What the option has set, as its fold's line says it: how many crossings, streets or movements it is on for, or, for
/// the plans, how many there are and which is shown. Empty where the junction has no signal.
pub fn set_line(fold: SignalFold, v: &JView, i18n: &I18n) -> String {
    let Some(s) = v.signal.as_ref() else { return String::new() };
    let count = |key: &'static str, n: usize| i18n.tr_now(key, &Args::new().num("n", n as i64));
    match fold {
        SignalFold::Plans => {
            let Some(plans) = v.plans.as_ref() else { return String::new() };
            let name = plans.plans.iter().find(|p| p.id == plans.viewing).map(|p| say_now(i18n, Units::Metres, &p.name)).unwrap_or_default();
            i18n.tr_now("jn-fold-plans-set", &Args::new().num("n", plans.plans.len() as i64).str("name", name))
        }
        SignalFold::Calls => count("jn-fold-calls-set", s.calls.crossings.iter().filter(|c| c.on_call).count()),
        SignalFold::Actuation => count("jn-fold-actuation-set", s.actuation.streets.iter().filter(|a| a.settings.is_some()).count()),
        SignalFold::Bikes => count("jn-fold-bikes-set", s.bicycles.movements.iter().filter(|m| m.on_detection).count()),
        SignalFold::Priority => count("jn-fold-priority-set", s.transit.streets.iter().filter(|t| t.priority.is_some()).count()),
        SignalFold::Preemption => count("jn-fold-preemption-set", s.preemption.streets.iter().filter(|p| p.rate.is_some()).count()),
    }
}

/// Whether the option's fold is drawn: where its section has something to list (a crossing, a street, a movement
/// cyclists may ride), as the section itself is drawn; the plans wherever the junction has a signal.
pub fn shown(fold: SignalFold, v: &JView) -> bool {
    let Some(s) = v.signal.as_ref() else { return false };
    match fold {
        SignalFold::Plans => v.plans.is_some(),
        SignalFold::Calls => !s.calls.crossings.is_empty(),
        SignalFold::Actuation => !s.actuation.streets.is_empty(),
        SignalFold::Bikes => !s.bicycles.movements.is_empty(),
        SignalFold::Priority => !s.transit.streets.is_empty(),
        SignalFold::Preemption => !s.preemption.streets.is_empty(),
    }
}

/// How many warnings the option has: its findings; for the plans, every plan's.
pub fn warnings(fold: SignalFold, v: &JView) -> usize {
    let Some(s) = v.signal.as_ref() else { return 0 };
    match fold {
        SignalFold::Plans => v.plans.as_ref().map_or(0, |p| p.plans.iter().map(|plan| plan.findings.len()).sum()),
        SignalFold::Calls => s.calls.findings.len(),
        SignalFold::Actuation => s.actuation.findings.len(),
        SignalFold::Bikes => s.bicycles.findings.len(),
        SignalFold::Priority => s.transit.findings.len(),
        SignalFold::Preemption => s.preemption.findings.len(),
    }
}

/// The warnings as the fold's line says them, nothing when there are none.
pub fn warnings_line(n: usize, i18n: &I18n) -> String {
    if n == 0 { String::new() } else { i18n.tr_now("jn-fold-warnings", &Args::new().num("n", n as i64)) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::junction::model::{Junction, signal, transit};
    use crate::shared::i18n::Locale;

    fn en() -> std::rc::Rc<crate::shared::i18n::I18n> {
        crate::i18n_for(Locale::En)
    }

    fn at(j: &Junction, bearing: i32) -> u32 {
        j.current().arms.iter().find(|a| a.bearing == bearing).unwrap().uid
    }

    #[test]
    fn a_signal_with_nothing_set_says_so_on_each_fold() {
        let j = Junction::new(0);
        let v = j.view();
        let en = en();
        assert_eq!(set_line(SignalFold::Calls, &v, &en), "None on call");
        assert_eq!(set_line(SignalFold::Actuation, &v, &en), "None actuated");
        assert_eq!(set_line(SignalFold::Bikes, &v, &en), "None on detection");
        assert_eq!(set_line(SignalFold::Priority, &v, &en), "None with priority");
        assert_eq!(set_line(SignalFold::Preemption, &v, &en), "None with pre-emption");
        let plans = v.plans.as_ref().unwrap();
        let default = plans.plans.iter().find(|p| p.id == plans.viewing).unwrap();
        assert_eq!(set_line(SignalFold::Plans, &v, &en), crate::shared::said::say_now(&en, crate::shared::units::Units::Metres, &default.name));
        let s = v.signal.as_ref().unwrap();
        assert_eq!(warnings(SignalFold::Calls, &v), s.calls.findings.len());
        assert_eq!(warnings(SignalFold::Preemption, &v), s.preemption.findings.len());
    }

    #[test]
    fn each_fold_counts_what_is_set() {
        let mut j = Junction::new(0);
        let (e, w) = (at(&j, 90), at(&j, 270));
        assert!(j.set_on_call(e, true) && j.set_on_call(w, true));
        // a bus lane gives the street transit, which priority needs
        assert!(j.set_bus_lane(e, true));
        let v = j.view();
        let s = v.signal.as_ref().unwrap();
        let with_phase = s.actuation.streets.iter().find(|a| a.has_phase).unwrap().uid;
        let preempt = s.preemption.streets.iter().find(|p| p.has_phase).unwrap().uid;
        let bus = s.transit.streets.iter().find(|t| t.has_transit && t.has_phase).expect("a street with transit and a phase").uid;
        assert!(j.set_actuated(with_phase, true));
        assert!(j.set_preemption(preempt, true));
        assert!(j.set_transit_priority(bus, Some(transit::Mode::EarlyGreen)));
        let v = j.view();
        let en = en();
        assert_eq!(set_line(SignalFold::Calls, &v, &en), "2 crossings on call");
        assert_eq!(set_line(SignalFold::Actuation, &v, &en), "1 street actuated");
        assert_eq!(set_line(SignalFold::Preemption, &v, &en), "1 street with pre-emption");
        // the sample's streets have no bike lane: a junction with bike lanes for the movements cyclists ride
        let mut biked = crate::junction::model::junction_biked();
        let ridden = biked.view().signal.as_ref().unwrap().bicycles.movements.iter().find(|m| m.ridden).expect("a movement cyclists ride").uid;
        assert!(biked.set_bike_detection(ridden, true));
        assert_eq!(set_line(SignalFold::Bikes, &biked.view(), &en), "1 movement on detection");
        assert_eq!(set_line(SignalFold::Priority, &v, &en), "1 street with priority");
    }

    #[test]
    fn the_plans_fold_names_the_plan_shown() {
        let mut j = Junction::new(0);
        assert!(j.add_plan("Evening"));
        let evening = j.view().plans.as_ref().unwrap().plans.iter().find(|p| p.own_name == "Evening").unwrap().id;
        assert!(j.view_plan(evening));
        assert_eq!(set_line(SignalFold::Plans, &j.view(), &en()), "2 plans, Evening shown");
    }

    #[test]
    fn warnings_are_counted_from_the_findings_and_said_only_when_there_are_some() {
        let en = en();
        assert_eq!(warnings_line(0, &en), "");
        // after what is set, apart from it: "None on call · 1 warning", so a screen reader does not run the two together
        assert_eq!(warnings_line(1, &en), "· 1 warning");
        assert_eq!(warnings_line(3, &en), "· 3 warnings");
        // a crossing left on call that no phase walks any more is a finding of the calls
        let mut j = Junction::new(0);
        let e = at(&j, 90);
        assert!(j.set_on_call(e, true));
        let phases = signal::plan(j.current(), 0).unwrap().phases;
        for (i, p) in phases.iter().enumerate() {
            if p.walks.contains(&e) {
                assert!(j.set_phase_walk(i, e, false));
            }
        }
        let v = j.view();
        assert!(warnings(SignalFold::Calls, &v) > 0);
        assert_eq!(warnings(SignalFold::Calls, &v), v.signal.as_ref().unwrap().calls.findings.len());
        let plans = v.plans.as_ref().unwrap();
        assert_eq!(warnings(SignalFold::Plans, &v), plans.plans.iter().map(|p| p.findings.len()).sum::<usize>());
    }

    #[test]
    fn the_folds_ids_and_headings_are_in_the_level_s_order() {
        let ids: Vec<_> = SignalFold::ALL.iter().map(|f| f.id()).collect();
        assert_eq!(ids, ["jn-fold-plans", "jn-fold-calls", "jn-fold-actuation", "jn-fold-bikes", "jn-fold-priority", "jn-fold-preemption"]);
        let en = en();
        for f in SignalFold::ALL {
            assert!(!en.tr_now(f.heading(), &crate::shared::i18n::Args::new()).is_empty());
        }
        assert!(en.missing().is_empty(), "{:?}", en.missing());
    }

    #[test]
    fn a_fold_is_shown_only_where_its_option_has_something_to_list() {
        let v = Junction::new(0).view();
        // the sample's streets have no bike lane: no movement cyclists ride, so no bicycles section and no fold
        assert!(!shown(SignalFold::Bikes, &v));
        for f in [SignalFold::Plans, SignalFold::Calls, SignalFold::Actuation, SignalFold::Preemption] {
            assert!(shown(f, &v), "{f:?}");
        }
        assert!(shown(SignalFold::Bikes, &crate::junction::model::junction_biked().view()));
        let mut stop = Junction::new(0);
        assert!(stop.set_control(crate::junction::model::PRIORITY));
        assert!(SignalFold::ALL.iter().all(|f| !shown(*f, &stop.view())), "no signal, no folds");
    }

    #[test]
    fn the_folds_speak_french() {
        let fr = crate::i18n_for(Locale::FrCa);
        let mut j = Junction::new(0);
        let v = j.view();
        let none = set_line(SignalFold::Calls, &v, &fr);
        assert!(!none.is_empty() && none != "None on call", "{none}");
        let with_phase = v.signal.as_ref().unwrap().actuation.streets.iter().find(|a| a.has_phase).unwrap().uid;
        assert!(j.set_actuated(with_phase, true));
        // the glossary's term for actuated, as the section's own radio says it (« À la demande »)
        assert_eq!(set_line(SignalFold::Actuation, &j.view(), &fr), "1 rue à la demande");
        assert_eq!(warnings_line(2, &fr), "· 2 avertissements");
        assert!(fr.missing().is_empty(), "{:?}", fr.missing());
    }
}
