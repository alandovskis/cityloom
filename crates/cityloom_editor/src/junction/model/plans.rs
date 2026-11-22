//! Time-of-day plans (CLM-16): a signalised junction runs its default plan (the timing and the responsive-policy
//! settings its streets have, `Arm.timing`, `calls`, `detection`, `actuation`, `cyclists`, `transit_priority`,
//! `preemption`) and may hold named plans that differ from it only in those settings, with a weekly schedule of which
//! plan runs when. The phases, their order and their patterns are the junction's, shared by every plan.
//!
//! A named plan keeps a full copy of each street's settings (`Settings`): it starts as a copy of the default, and the
//! junction as the plan sees it (`applied`) is the junction with those settings put on its streets, so `signal::plan`,
//! `control::hour`, the checks and the read model run on it unchanged. A street the plan holds nothing for (a street
//! that joined later) runs as in the default.
//!
//! The schedule gives each day type (weekdays, Saturday, Sunday) its periods: each starts at a minute of the day and
//! runs until the next starts, the last until midnight. The first starts at 00:00 and runs the default plan unless a
//! period scheduled at 00:00 says otherwise. Starts are kept distinct and in order (`Schedule::settle`), so every minute
//! of the week is covered by exactly one period: no gap and no overlap can be written. A change of plan takes effect at
//! the end of the cycle running when the period starts (no transition period); the hour's summary runs one plan.
//!
//! Every bound here is a placeholder: the most plans, the longest name, the most periods a day. No DOM, no signals, no
//! I/O, no wording.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::junction::model::{Arm, Junction, Refusal, SIGNAL, State, Timing, actuation, calls, in_force, settle_settings, transit};
use crate::shared::said::{Arg, Said};

/// The id of the default plan: the junction's own settings. Never stored, never removed.
pub const DEFAULT_PLAN: u32 = 0;
/// The most plans a junction holds, the default included (placeholder).
pub const MAX_PLANS: usize = 8;
/// The most characters in a plan's name (placeholder).
pub const MAX_NAME_CHARS: usize = 40;
/// The most periods a day type has, the one from 00:00 included (placeholder).
pub const MAX_PERIODS: usize = 12;
/// The minutes of a day.
pub const MINUTES_A_DAY: u32 = 1_440;

/// A kind of day the schedule tells apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DayType {
    /// Monday to Friday.
    Weekday,
    Saturday,
    Sunday,
}

impl DayType {
    pub const ALL: [DayType; 3] = [DayType::Weekday, DayType::Saturday, DayType::Sunday];

    /// The day type of day `d` of the week, Monday (0) to Sunday (6).
    pub fn of_day(d: u32) -> DayType {
        match d % 7 {
            5 => DayType::Saturday,
            6 => DayType::Sunday,
            _ => DayType::Weekday,
        }
    }

    /// Its name in ids and in the page's markup.
    pub fn id(self) -> &'static str {
        match self {
            DayType::Weekday => "weekday",
            DayType::Saturday => "saturday",
            DayType::Sunday => "sunday",
        }
    }

    pub fn from_id(id: &str) -> Option<DayType> {
        DayType::ALL.into_iter().find(|d| d.id() == id)
    }

    /// The message that names it on its own ("Weekdays").
    pub fn key(self) -> &'static str {
        match self {
            DayType::Weekday => "jn-day-weekday",
            DayType::Saturday => "jn-day-saturday",
            DayType::Sunday => "jn-day-sunday",
        }
    }

    /// The message that says it within a sentence ("on weekdays").
    pub fn on_key(self) -> &'static str {
        match self {
            DayType::Weekday => "jn-day-on-weekday",
            DayType::Saturday => "jn-day-on-saturday",
            DayType::Sunday => "jn-day-on-sunday",
        }
    }
}

/// One street's signal settings as a plan holds them: `Arm`'s, under the same names and saved the same way.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub uid: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timing: Option<Timing>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calls: Option<u32>,
    #[serde(default, skip_serializing_if = "calls::Detection::is_push_button")]
    pub detection: calls::Detection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actuation: Option<actuation::Actuation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cyclists: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transit_priority: Option<transit::Priority>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preemption: Option<u32>,
}

impl Settings {
    /// The settings arm `a` has.
    pub fn of(a: &Arm) -> Settings {
        Settings {
            uid: a.uid,
            timing: a.timing,
            calls: a.calls,
            detection: a.detection,
            actuation: a.actuation,
            cyclists: a.cyclists,
            transit_priority: a.transit_priority,
            preemption: a.preemption,
        }
    }

    /// Puts these settings on arm `a`.
    pub fn put(&self, a: &mut Arm) {
        a.timing = self.timing;
        a.calls = self.calls;
        a.detection = self.detection;
        a.actuation = self.actuation;
        a.cyclists = self.cyclists;
        a.transit_priority = self.transit_priority;
        a.preemption = self.preemption;
    }
}

/// A named plan: its id (stable: a rename or a removal of another plan never changes it), its name, and its streets'
/// settings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimingPlan {
    pub id: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub streets: Vec<Settings>,
}

/// A period of a day type: from `start_min` (minutes after midnight) until the next period starts, plan `plan` runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Period {
    pub start_min: u32,
    pub plan: u32,
}

/// The weekly schedule: each day type's periods after the first, in order of their starts (a period at 00:00 is kept
/// only when it runs a plan other than the default). Saved only once something is scheduled.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Schedule {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub weekday: Vec<Period>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub saturday: Vec<Period>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sunday: Vec<Period>,
}

impl Schedule {
    /// Nothing scheduled: the default runs all week.
    pub fn is_empty(&self) -> bool {
        self.weekday.is_empty() && self.saturday.is_empty() && self.sunday.is_empty()
    }

    fn day(&self, d: DayType) -> &Vec<Period> {
        match d {
            DayType::Weekday => &self.weekday,
            DayType::Saturday => &self.saturday,
            DayType::Sunday => &self.sunday,
        }
    }

    fn day_mut(&mut self, d: DayType) -> &mut Vec<Period> {
        match d {
            DayType::Weekday => &mut self.weekday,
            DayType::Saturday => &mut self.saturday,
            DayType::Sunday => &mut self.sunday,
        }
    }

    /// The periods of day type `d` in order, the first starting at 00:00 (the default's unless a plan is scheduled
    /// there); each runs until the next starts, the last until midnight.
    pub fn periods(&self, d: DayType) -> Vec<Period> {
        let stored = self.day(d);
        let first = (stored.first().map(|p| p.start_min) != Some(0)).then_some(Period { start_min: 0, plan: DEFAULT_PLAN });
        first.into_iter().chain(stored.iter().copied()).collect()
    }

    /// The plan scheduled at `minute` (after midnight, bounded to the day) of day type `d`.
    pub fn plan_at(&self, d: DayType, minute: u32) -> u32 {
        let minute = minute.min(MINUTES_A_DAY - 1);
        self.periods(d).iter().rev().find(|p| p.start_min <= minute).map_or(DEFAULT_PLAN, |p| p.plan)
    }

    /// The plan scheduled at minute `m` of the week, from Monday 00:00.
    pub fn plan_in_week(&self, m: u32) -> u32 {
        self.plan_at(DayType::of_day(m / MINUTES_A_DAY), m % MINUTES_A_DAY)
    }

    /// Keeps the schedule one that covers the week: in each day type, starts within the day, distinct and in order,
    /// each period's plan one that `known` knows (else the default: a removed plan's periods run the default), no
    /// period at 00:00 that only says the default, and at most `MAX_PERIODS` periods.
    pub fn settle(&mut self, known: impl Fn(u32) -> bool) {
        for d in DayType::ALL {
            let day = self.day_mut(d);
            for p in day.iter_mut().filter(|p| !known(p.plan)) {
                p.plan = DEFAULT_PLAN;
            }
            day.retain(|p| p.start_min < MINUTES_A_DAY);
            day.sort_by_key(|p| p.start_min);
            day.dedup_by_key(|p| p.start_min);
            if day.first().is_some_and(|p| *p == Period { start_min: 0, plan: DEFAULT_PLAN }) {
                day.remove(0);
            }
            let room = if day.first().is_some_and(|p| p.start_min == 0) { MAX_PERIODS } else { MAX_PERIODS - 1 };
            day.truncate(room);
        }
    }
}

/// The junction as plan `id` sees it: its streets given the plan's settings (a street it holds none for keeps the
/// default's). The default, or a plan it does not have, is the junction itself.
pub fn applied(s: &State, id: u32) -> Cow<'_, State> {
    let Some(plan) = s.plans.iter().find(|p| p.id == id) else { return Cow::Borrowed(s) };
    let mut t = s.clone();
    for a in &mut t.arms {
        if let Some(settings) = plan.streets.iter().find(|x| x.uid == a.uid) {
            settings.put(a);
        }
    }
    Cow::Owned(t)
}

/// Whether the junction has plan `id` (the default always).
pub fn has(s: &State, id: u32) -> bool {
    id == DEFAULT_PLAN || s.plans.iter().any(|p| p.id == id)
}

/// The plans' ids, the default first, then the named ones in the order they were added.
pub fn ids(s: &State) -> Vec<u32> {
    std::iter::once(DEFAULT_PLAN).chain(s.plans.iter().map(|p| p.id)).collect()
}

/// Plan `id`'s name: the person's own, or the words for the default.
pub fn name_of(s: &State, id: u32) -> Said {
    match s.plans.iter().find(|p| p.id == id) {
        Some(p) => crate::street::model::named_said(&p.name),
        None => Said::new("jn-plan-default"),
    }
}

/// A name as kept: without spaces at either end, and no longer than `MAX_NAME_CHARS`.
fn bounded(name: &str) -> String {
    name.trim().chars().take(MAX_NAME_CHARS).collect::<String>().trim_end().to_string()
}

/// Keeps the plans true to the junction, on load as after an edit: none off a signal (they are signal timing and go
/// with it); otherwise distinct ids other than the default's, at most `MAX_PLANS` with the default, names bounded,
/// settings only for streets the junction has, each brought within its bounds as an arm's are (`settle_settings`), and
/// the schedule settled on the plans there are.
pub(super) fn settle(s: &mut State) {
    if s.control != SIGNAL {
        s.plans.clear();
        s.schedule = Schedule::default();
        return;
    }
    // The ids kept, never more than `MAX_PLANS - 1`: a document holding a great many plans is settled in a time that
    // grows with their number alone.
    let mut kept: Vec<u32> = Vec::with_capacity(MAX_PLANS - 1);
    s.plans.retain(|p| {
        let keep = kept.len() < MAX_PLANS - 1 && p.id != DEFAULT_PLAN && !kept.contains(&p.id);
        if keep {
            kept.push(p.id);
        }
        keep
    });
    let arms = &s.arms;
    for p in &mut s.plans {
        p.name = bounded(&p.name);
        let mut kept: Vec<Settings> = Vec::new();
        for x in &p.streets {
            let Some(arm) = arms.iter().find(|a| a.uid == x.uid) else { continue };
            if kept.iter().any(|k| k.uid == x.uid) {
                continue;
            }
            let mut a = arm.clone();
            x.put(&mut a);
            settle_settings(&mut a, true);
            kept.push(Settings::of(&a));
        }
        p.streets = kept;
    }
    let known: Vec<u32> = ids(s);
    s.schedule.settle(|id| known.contains(&id));
}

/// Why a name cannot be plan `except`'s (none: a new plan's) among the plans of `s`: empty, too long, or another
/// plan's already (whatever the case).
fn name_refusal(s: &State, name: &str, except: Option<u32>) -> Option<Refusal> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Some(Refusal::PlanNameEmpty);
    }
    if trimmed.chars().count() > MAX_NAME_CHARS {
        return Some(Refusal::PlanNameTooLong);
    }
    let lower = trimmed.to_lowercase();
    s.plans.iter().any(|p| Some(p.id) != except && p.name.to_lowercase() == lower).then_some(Refusal::PlanNameTaken)
}

fn said(s: Said) -> Arg {
    Arg::Said(Box::new(s))
}

impl Junction {
    /// The plan the page shows and edits: every timing and responsive-policy edit goes to it (`in_plan`).
    pub fn viewing(&self) -> u32 {
        self.viewing
    }

    /// Shows plan `id`, so the timing and policy edits go to it. Not an edit: no history, nothing to keep. False for a
    /// plan the junction does not have.
    pub fn view_plan(&mut self, id: u32) -> bool {
        if !has(self.current(), id) {
            return false;
        }
        self.viewing = id;
        true
    }

    /// The default when the plan shown is gone (an undo of its adding, a removal, a sync).
    pub(super) fn fix_viewing(&mut self) {
        if !has(self.current(), self.viewing) {
            self.viewing = DEFAULT_PLAN;
        }
    }

    /// Runs a timing or policy edit (`f`) on the plan shown. The default's is the junction's own; for a named plan the
    /// edit runs, with every check it makes, on the junction as the plan sees it (`applied`), and what it leaves is
    /// kept as the plan's settings in one revision labelled with the plan's name. A setting it has already (true with no
    /// revision) is no edit here either; a refusal is the edit's own.
    pub(super) fn in_plan(&mut self, f: impl FnOnce(&mut Junction) -> bool) -> bool {
        let id = self.viewing;
        if id == DEFAULT_PLAN || !has(self.current(), id) {
            return f(self);
        }
        let mut scratch = Junction {
            linked: self.linked,
            name: self.name.clone(),
            region: self.region,
            states: vec![applied(self.current(), id).into_owned()],
            cursor: 0,
            floor: None,
            selected: self.selected,
            refusal: None,
            refusal_arm: None,
            calls_dropped: 0,
            phases_cut: 0,
            viewing: DEFAULT_PLAN,
        };
        let done = f(&mut scratch);
        if !done {
            self.refusal = scratch.refusal;
            self.refusal_arm = scratch.refusal_arm;
            return false;
        }
        if scratch.cursor == 0 {
            return true;
        }
        let (before, after) = (&scratch.states[0], scratch.current());
        // The plan keeps the streets it holds and takes on those the edit changed; a street it holds nothing for (one
        // that joined later) and the edit left alone runs on as in the default.
        let held: Vec<u32> = self.current().plans.iter().find(|p| p.id == id).map(|p| p.streets.iter().map(|x| x.uid).collect()).unwrap_or_default();
        let streets: Vec<Settings> = after
            .arms
            .iter()
            .map(Settings::of)
            .filter(|now| held.contains(&now.uid) || before.arms.iter().find(|a| a.uid == now.uid).map(Settings::of).as_ref() != Some(now))
            .collect();
        let label = Said::new("jn-rev-in-plan").with("plan", said(name_of(self.current(), id))).with("change", said(after.label.clone()));
        self.edit(label, move |s| s.plans.iter_mut().find(|p| p.id == id).map(|p| p.streets = streets).is_some())
    }

    /// The signal has to be in force for an edit of its plans.
    fn plans_to_edit(&mut self) -> bool {
        if in_force(self.current()).control != SIGNAL {
            self.refuse(Refusal::NotSignal);
            return false;
        }
        true
    }

    /// Adds a plan named `name`, a copy of the default's settings, last. Refused off a signal, past `MAX_PLANS`, and for
    /// a name that is empty, longer than `MAX_NAME_CHARS` or another plan's already.
    pub fn add_plan(&mut self, name: &str) -> bool {
        if !self.plans_to_edit() {
            return false;
        }
        if self.current().plans.len() + 1 >= MAX_PLANS {
            return self.refuse(Refusal::TooManyPlans);
        }
        if let Some(why) = name_refusal(self.current(), name, None) {
            return self.refuse(why);
        }
        let name = bounded(name);
        let id = self.current().plans.iter().map(|p| p.id).max().unwrap_or(DEFAULT_PLAN) + 1;
        let label = Said::new("jn-rev-plan-add").with("name", Arg::Text(name.clone()));
        self.edit(label, move |s| {
            let streets = s.arms.iter().map(Settings::of).collect();
            s.plans.push(TimingPlan { id, name, streets });
            true
        })
    }

    /// Renames plan `id`. The default keeps its name (`DefaultPlanNamed`); the name is refused as `add_plan` refuses
    /// one. The name it has already is no edit.
    pub fn rename_plan(&mut self, id: u32, name: &str) -> bool {
        if id == DEFAULT_PLAN {
            return self.refuse(Refusal::DefaultPlanNamed);
        }
        let Some(now) = self.current().plans.iter().find(|p| p.id == id).map(|p| p.name.clone()) else { return self.refuse(Refusal::DoesNotFit) };
        if !self.plans_to_edit() {
            return false;
        }
        if bounded(name) == now {
            return true;
        }
        if let Some(why) = name_refusal(self.current(), name, Some(id)) {
            return self.refuse(why);
        }
        let name = bounded(name);
        let label = Said::new("jn-rev-plan-rename").with("from", Arg::Text(now)).with("to", Arg::Text(name.clone()));
        self.edit(label, move |s| s.plans.iter_mut().find(|p| p.id == id).map(|p| p.name = name).is_some())
    }

    /// Removes plan `id`; its periods then run the default plan, and the revision says how many. The default cannot be
    /// removed (`DefaultPlanKept`): it runs whenever no other plan is scheduled.
    pub fn remove_plan(&mut self, id: u32) -> bool {
        if id == DEFAULT_PLAN {
            return self.refuse(Refusal::DefaultPlanKept);
        }
        let Some(name) = self.current().plans.iter().find(|p| p.id == id).map(|p| p.name.clone()) else { return self.refuse(Refusal::DoesNotFit) };
        if !self.plans_to_edit() {
            return false;
        }
        let moved = DayType::ALL.iter().map(|d| self.current().schedule.day(*d).iter().filter(|p| p.plan == id).count()).sum::<usize>();
        let label = Said::new("jn-rev-plan-remove").with("name", Arg::Text(name)).with("n", Arg::Num(moved as i64));
        self.edit(label, move |s| {
            s.plans.retain(|p| p.id != id);
            true
        })
    }

    /// The time of a period as a revision says it.
    fn period_label(key: &'static str, day: DayType, start_min: u32) -> Said {
        Said::new(key).with("day", Arg::Msg(day.on_key())).with("time", Arg::Clock(start_min as i32))
    }

    /// Schedules plan `plan` from `start_min` (minutes after midnight) on day type `day`, until the next period starts.
    /// Refused off a signal, for a start outside the day (`PeriodOutOfDay`; 00:00 is the day's first period, whose plan
    /// is changed rather than added), a start another period has (`PeriodTaken`), past `MAX_PERIODS`
    /// (`TooManyPeriods`) and for a plan the junction does not have.
    pub fn add_period(&mut self, day: DayType, start_min: i32, plan: u32) -> bool {
        if !self.plans_to_edit() {
            return false;
        }
        let Some(start) = u32::try_from(start_min).ok().filter(|m| (1..MINUTES_A_DAY).contains(m)) else { return self.refuse(Refusal::PeriodOutOfDay) };
        if !has(self.current(), plan) {
            return self.refuse(Refusal::DoesNotFit);
        }
        let periods = self.current().schedule.periods(day);
        if periods.iter().any(|p| p.start_min == start) {
            return self.refuse(Refusal::PeriodTaken);
        }
        if periods.len() >= MAX_PERIODS {
            return self.refuse(Refusal::TooManyPeriods);
        }
        let label = Self::period_label("jn-rev-period-add", day, start).with("plan", said(name_of(self.current(), plan)));
        self.edit(label, move |s| {
            s.schedule.day_mut(day).push(Period { start_min: start, plan });
            true
        })
    }

    /// Removes the period starting at `start_min` on day type `day`: the period before it runs on until the next. The
    /// day's first period, from 00:00, cannot be removed (`FirstPeriodKept`): its plan is changed instead.
    pub fn remove_period(&mut self, day: DayType, start_min: u32) -> bool {
        if start_min == 0 {
            return self.refuse(Refusal::FirstPeriodKept);
        }
        if !self.current().schedule.day(day).iter().any(|p| p.start_min == start_min) {
            return self.refuse(Refusal::DoesNotFit);
        }
        if !self.plans_to_edit() {
            return false;
        }
        self.edit(Self::period_label("jn-rev-period-remove", day, start_min), move |s| {
            s.schedule.day_mut(day).retain(|p| p.start_min != start_min);
            true
        })
    }

    /// Runs plan `plan` in the period starting at `start_min` on day type `day` (the first, from 00:00, included). The
    /// plan it runs already is no edit.
    pub fn set_period_plan(&mut self, day: DayType, start_min: u32, plan: u32) -> bool {
        let Some(now) = self.current().schedule.periods(day).into_iter().find(|p| p.start_min == start_min) else { return self.refuse(Refusal::DoesNotFit) };
        if !self.plans_to_edit() {
            return false;
        }
        if !has(self.current(), plan) {
            return self.refuse(Refusal::DoesNotFit);
        }
        if now.plan == plan {
            return true;
        }
        let label = Self::period_label("jn-rev-period-plan", day, start_min).with("plan", said(name_of(self.current(), plan)));
        self.edit(label, move |s| {
            let stored = s.schedule.day_mut(day);
            match stored.iter_mut().find(|p| p.start_min == start_min) {
                Some(p) => p.plan = plan,
                None => stored.insert(0, Period { start_min, plan }),
            }
            true
        })
    }

    /// Moves the period starting at `from_min` on day type `day` to start at `to_min`, its plan kept. The first period
    /// starts at 00:00 whatever (`FirstPeriodKept`); a start outside the day or another period's is refused as
    /// `add_period` refuses it. The start it has already is no edit.
    pub fn set_period_start(&mut self, day: DayType, from_min: u32, to_min: i32) -> bool {
        if from_min == 0 {
            return self.refuse(Refusal::FirstPeriodKept);
        }
        if !self.current().schedule.day(day).iter().any(|p| p.start_min == from_min) {
            return self.refuse(Refusal::DoesNotFit);
        }
        if !self.plans_to_edit() {
            return false;
        }
        let Some(to) = u32::try_from(to_min).ok().filter(|m| (1..MINUTES_A_DAY).contains(m)) else { return self.refuse(Refusal::PeriodOutOfDay) };
        if to == from_min {
            return true;
        }
        if self.current().schedule.periods(day).iter().any(|p| p.start_min == to) {
            return self.refuse(Refusal::PeriodTaken);
        }
        let label = Self::period_label("jn-rev-period-start", day, from_min).with("to", Arg::Clock(to as i32));
        self.edit(label, move |s| s.schedule.day_mut(day).iter_mut().find(|p| p.start_min == from_min).map(|p| p.start_min = to).is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::junction::model::{PRIORITY, TimingField};
    use crate::shared::testing::{en, fr};

    /// The sample signal ("Avenue and street"), its first street's uid.
    fn signal() -> (Junction, u32) {
        let j = Junction::new(0);
        assert_eq!(j.current().control, SIGNAL);
        let uid = j.current().arms[0].uid;
        (j, uid)
    }

    fn names(j: &Junction) -> Vec<String> {
        j.view().plans.expect("a signal has plans").plans.iter().map(|p| en(&p.name)).collect()
    }

    #[test]
    fn the_placeholders_are_the_ones_chosen() {
        assert_eq!((MAX_PLANS, MAX_NAME_CHARS, MAX_PERIODS, MINUTES_A_DAY), (8, 40, 12, 1_440));
    }

    #[test]
    fn a_junction_saved_before_plans_loads_as_its_default_alone_and_is_written_as_before() {
        let (j, _) = signal();
        let text = serde_json::to_string(j.current()).unwrap();
        assert!(!text.contains("plans") && !text.contains("schedule"), "{text}");
        let back: State = serde_json::from_str(&text).unwrap();
        assert!(back.plans.is_empty() && back.schedule.is_empty());
        let v = j.view().plans.unwrap();
        assert_eq!(v.viewing, DEFAULT_PLAN);
        assert_eq!(names(&j), ["Default plan"]);
        assert!(v.plans[0].is_default);
        for day in &v.days {
            assert_eq!(day.periods, vec![crate::junction::model::read_model::PeriodView { start_min: 0, end_min: MINUTES_A_DAY, plan: DEFAULT_PLAN }]);
        }
        // Off a signal there are none to show.
        assert!(Junction::new(1).view().plans.is_none());
    }

    #[test]
    fn plans_are_added_renamed_and_removed_and_the_default_is_neither_removed_nor_renamed() {
        let (mut j, _) = signal();
        assert!(j.add_plan("  Morning peak "));
        assert_eq!(j.revisions().last().map(en).as_deref(), Some("Add plan “Morning peak”"));
        assert!(j.add_plan("Night"));
        assert_eq!(names(&j), ["Default plan", "Morning peak", "Night"]);
        let ids: Vec<u32> = j.current().plans.iter().map(|p| p.id).collect();
        assert_eq!(ids, [1, 2], "stable ids, never the default's");
        // A new plan is a copy of the default's settings.
        assert_eq!(j.current().plans[0].streets, j.current().arms.iter().map(Settings::of).collect::<Vec<_>>());
        // Names: not empty, not too long, not another plan's (whatever the case).
        for (name, why) in [("   ", Refusal::PlanNameEmpty), (&"x".repeat(MAX_NAME_CHARS + 1)[..], Refusal::PlanNameTooLong), ("night", Refusal::PlanNameTaken)]
        {
            assert!(!j.add_plan(name), "{name}");
            assert_eq!(j.refusal(), Some(why));
        }
        assert!(j.rename_plan(2, "Overnight"));
        assert_eq!(j.revisions().last().map(en).as_deref(), Some("Rename plan “Night” to “Overnight”"));
        assert!(j.rename_plan(2, "Overnight"), "the name it has is no edit");
        assert_eq!(j.revisions().count(), 3);
        assert!(!j.rename_plan(2, "MORNING PEAK"));
        assert_eq!(j.refusal(), Some(Refusal::PlanNameTaken));
        // The default stays.
        assert!(!j.remove_plan(DEFAULT_PLAN));
        assert_eq!(j.refusal(), Some(Refusal::DefaultPlanKept));
        assert!(!j.rename_plan(DEFAULT_PLAN, "Base"));
        assert_eq!(j.refusal(), Some(Refusal::DefaultPlanNamed));
        assert!(j.remove_plan(1));
        assert_eq!(names(&j), ["Default plan", "Overnight"]);
        assert!(j.add_plan("Evening"));
        assert_eq!(j.current().plans.iter().map(|p| p.id).collect::<Vec<_>>(), [2, 3], "a removed plan's id is not given again while a higher one stands");
        // At most `MAX_PLANS`, the default included.
        for n in 0..MAX_PLANS {
            j.add_plan(&format!("Plan {n}"));
        }
        assert_eq!(j.current().plans.len(), MAX_PLANS - 1);
        assert_eq!(j.refusal(), Some(Refusal::TooManyPlans));
        // Every refusal is said in both languages.
        for why in [Refusal::DefaultPlanKept, Refusal::PlanNameTaken] {
            let s = Said::new(why.key());
            assert!(!en(&s).is_empty() && !fr(&s).is_empty() && en(&s) != fr(&s));
        }
    }

    #[test]
    fn plans_and_their_schedule_belong_to_a_signal() {
        let mut j = Junction::new(1);
        assert!(!j.add_plan("Night"));
        assert_eq!(j.refusal(), Some(Refusal::NotSignal));
        let (mut j, _) = signal();
        assert!(j.add_plan("Night"));
        assert!(j.add_period(DayType::Weekday, 22 * 60, 1));
        assert!(j.set_control(PRIORITY));
        assert!(j.current().plans.is_empty() && j.current().schedule.is_empty(), "they go with the signal's timing");
        assert_eq!(j.revisions().last().map(|r| r.key), Some("jn-rev-control-timing"), "and the change says the timing went");
        assert_eq!(j.viewing(), DEFAULT_PLAN);
    }

    #[test]
    fn the_schedule_runs_a_plan_from_each_period_s_start_to_the_next_and_the_default_fills_the_rest() {
        let (mut j, _) = signal();
        assert!(j.add_plan("Morning peak") && j.add_plan("Night"));
        assert!(j.add_period(DayType::Weekday, 7 * 60, 1));
        assert_eq!(j.revisions().last().map(en).as_deref(), Some("“Morning peak” from 07:00 on weekdays"));
        assert!(j.add_period(DayType::Weekday, 9 * 60, DEFAULT_PLAN));
        assert!(j.add_period(DayType::Weekday, 22 * 60, 2));
        assert!(j.set_period_plan(DayType::Sunday, 0, 2));
        assert!(j.add_period(DayType::Sunday, 8 * 60, DEFAULT_PLAN));
        let sched = &j.current().schedule;
        assert_eq!(sched.plan_at(DayType::Weekday, 0), DEFAULT_PLAN);
        assert_eq!(sched.plan_at(DayType::Weekday, 7 * 60 - 1), DEFAULT_PLAN);
        assert_eq!(sched.plan_at(DayType::Weekday, 7 * 60), 1);
        assert_eq!(sched.plan_at(DayType::Weekday, 8 * 60 + 59), 1);
        assert_eq!(sched.plan_at(DayType::Weekday, 9 * 60), DEFAULT_PLAN);
        assert_eq!(sched.plan_at(DayType::Weekday, 23 * 60 + 59), 2);
        assert_eq!(sched.plan_at(DayType::Saturday, 8 * 60), DEFAULT_PLAN, "Saturday has nothing scheduled");
        assert_eq!(sched.plan_at(DayType::Sunday, 3 * 60), 2);
        assert_eq!(sched.plan_at(DayType::Sunday, 8 * 60), DEFAULT_PLAN);
        // Monday to Friday are weekdays; the week starts on Monday at 00:00.
        assert_eq!(sched.plan_in_week(4 * MINUTES_A_DAY + 7 * 60), 1, "Friday 07:00");
        assert_eq!(sched.plan_in_week(5 * MINUTES_A_DAY + 7 * 60), DEFAULT_PLAN, "Saturday 07:00");
        assert_eq!(sched.plan_in_week(6 * MINUTES_A_DAY + 60), 2, "Sunday 01:00");
        // Refusals: the first period stays; starts within the day and not taken; a plan the junction has.
        assert!(!j.remove_period(DayType::Weekday, 0));
        assert_eq!(j.refusal(), Some(Refusal::FirstPeriodKept));
        assert!(!j.set_period_start(DayType::Weekday, 0, 60));
        assert_eq!(j.refusal(), Some(Refusal::FirstPeriodKept));
        for (start, why) in [(0, Refusal::PeriodOutOfDay), (1_440, Refusal::PeriodOutOfDay), (-5, Refusal::PeriodOutOfDay), (420, Refusal::PeriodTaken)] {
            assert!(!j.add_period(DayType::Weekday, start, 2), "{start}");
            assert_eq!(j.refusal(), Some(why));
        }
        assert!(!j.set_period_start(DayType::Weekday, 420, 540));
        assert_eq!(j.refusal(), Some(Refusal::PeriodTaken));
        assert!(!j.add_period(DayType::Weekday, 600, 9));
        assert!(j.set_period_start(DayType::Weekday, 420, 390));
        assert_eq!(j.revisions().last().map(en).as_deref(), Some("Period from 07:00 on weekdays now starts at 06:30"));
        assert_eq!(j.current().schedule.plan_at(DayType::Weekday, 390), 1);
        assert!(j.remove_period(DayType::Weekday, 540));
        assert_eq!(j.current().schedule.plan_at(DayType::Weekday, 600), 1, "the period before runs on");
        // At most `MAX_PERIODS` a day, the one from 00:00 included.
        for k in 1..=MAX_PERIODS as i32 {
            j.add_period(DayType::Saturday, k * 60, 1);
        }
        assert_eq!(j.current().schedule.periods(DayType::Saturday).len(), MAX_PERIODS);
        assert_eq!(j.refusal(), Some(Refusal::TooManyPeriods));
    }

    #[test]
    fn removing_a_plan_runs_its_periods_on_the_default_and_says_how_many() {
        let (mut j, _) = signal();
        assert!(j.add_plan("Night"));
        assert!(j.add_period(DayType::Weekday, 22 * 60, 1) && j.set_period_plan(DayType::Sunday, 0, 1));
        assert!(j.remove_plan(1));
        assert_eq!(j.revisions().last().map(en).as_deref(), Some("Remove plan “Night”: its 2 periods run the default plan"));
        assert_eq!(fr(j.revisions().last().unwrap()), "Retrait du plan «\u{a0}Night\u{a0}»\u{a0}: ses 2 plages horaires suivent le plan par défaut");
        let sched = &j.current().schedule;
        assert_eq!(sched.plan_at(DayType::Weekday, 23 * 60), DEFAULT_PLAN);
        assert_eq!(sched.plan_at(DayType::Sunday, 0), DEFAULT_PLAN);
        assert_eq!(sched.periods(DayType::Sunday).len(), 1, "a period at 00:00 that says the default is the day's first");
        assert!(j.undo());
        assert_eq!(j.current().schedule.plan_at(DayType::Weekday, 23 * 60), 1, "undone, the plan and its periods are back");
    }

    /// A small seeded generator for the property test (SplitMix64).
    struct Rng(u64);

    impl Rng {
        fn below(&mut self, n: u64) -> u64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            (z ^ (z >> 31)) % n
        }
    }

    /// Every minute of the week is covered by exactly one period of its day type, whose plan the junction has.
    fn covers_the_week(s: &State) {
        for d in DayType::ALL {
            let periods = s.schedule.periods(d);
            assert_eq!(periods[0].start_min, 0, "{d:?} starts at 00:00: {periods:?}");
            assert!(periods.windows(2).all(|w| w[0].start_min < w[1].start_min), "{d:?} in order, no two at once: {periods:?}");
            assert!(periods.iter().all(|p| p.start_min < MINUTES_A_DAY && has(s, p.plan)), "{d:?} within the day, known plans: {periods:?}");
            assert!(periods.len() <= MAX_PERIODS);
        }
        for m in 0..7 * MINUTES_A_DAY {
            let (d, at) = (DayType::of_day(m / MINUTES_A_DAY), m % MINUTES_A_DAY);
            let covering: Vec<Period> = {
                let periods = s.schedule.periods(d);
                let ends: Vec<u32> = periods.iter().skip(1).map(|p| p.start_min).chain([MINUTES_A_DAY]).collect();
                periods.iter().zip(ends).filter(|(p, end)| p.start_min <= at && at < *end).map(|(p, _)| *p).collect()
            };
            assert_eq!(covering.len(), 1, "minute {m} of the week: {covering:?}");
            assert_eq!(s.schedule.plan_in_week(m), covering[0].plan);
        }
    }

    #[test]
    fn no_sequence_of_edits_leaves_a_gap_or_an_overlap_in_the_week() {
        let (mut j, _) = signal();
        let mut rng = Rng(16);
        for step in 0..400 {
            let ids = ids(j.current());
            let plan = ids[rng.below(ids.len() as u64) as usize];
            let day = DayType::ALL[rng.below(3) as usize];
            let periods = j.current().schedule.periods(day);
            let start = periods[rng.below(periods.len() as u64) as usize].start_min;
            let minute = rng.below(1_500) as i32 - 30;
            match rng.below(9) {
                0 => j.add_plan(&format!("P{step}")),
                1 => j.remove_plan(plan),
                2 | 3 => j.add_period(day, minute, plan),
                4 => j.remove_period(day, start),
                5 => j.set_period_plan(day, start, plan),
                6 => j.set_period_start(day, start, minute),
                7 => j.undo(),
                _ => j.redo(),
            };
            covers_the_week(j.current());
        }
        assert!(!j.current().schedule.is_empty(), "the walk scheduled something");
        // A document written by hand (a sync, an older build) is settled to one that covers the week too.
        let mut s = j.current().clone();
        s.schedule.weekday = vec![
            Period { start_min: 900, plan: 77 },
            Period { start_min: 60, plan: DEFAULT_PLAN },
            Period { start_min: 60, plan: 1 },
            Period { start_min: 5_000, plan: 1 },
        ];
        s.schedule.saturday = (0..40).map(|k| Period { start_min: k * 30, plan: DEFAULT_PLAN }).collect();
        s.plans.push(TimingPlan { id: DEFAULT_PLAN, name: "Imposter".into(), streets: Vec::new() });
        settle(&mut s);
        covers_the_week(&s);
        assert_eq!(s.schedule.periods(DayType::Weekday).iter().map(|p| p.start_min).collect::<Vec<_>>(), [0, 60, 900]);
        assert_eq!(s.schedule.plan_at(DayType::Weekday, 900), DEFAULT_PLAN, "a plan it does not have runs as the default");
        assert!(s.plans.iter().all(|p| p.id != DEFAULT_PLAN));
    }

    #[test]
    fn a_timing_or_policy_edit_goes_to_the_plan_shown_and_leaves_the_default_as_it_was() {
        let (mut j, uid) = signal();
        assert!(j.add_plan("Night"));
        assert!(!j.view_plan(9), "no such plan");
        assert!(j.view_plan(1));
        let default_before: Vec<Settings> = j.current().arms.iter().map(Settings::of).collect();
        assert!(j.set_timing(uid, TimingField::Green, 40));
        assert_eq!(j.revisions().last().map(en).as_deref().map(|t| t.ends_with(", in plan “Night”")), Some(true), "{:?}", j.revisions().last().map(en));
        assert!(j.set_on_call(uid, true));
        assert!(j.set_timing(uid, TimingField::Green, 40), "the value it has is no edit");
        let edits = j.revisions().count();
        assert_eq!(edits, 3);
        assert_eq!(j.current().arms.iter().map(Settings::of).collect::<Vec<_>>(), default_before, "the default is untouched");
        let night = &j.current().plans[0].streets.iter().find(|x| x.uid == uid).cloned().unwrap();
        assert_eq!(night.timing.map(|t| t.green_s), Some(40));
        assert_eq!(night.calls, Some(calls::DEFAULT_CALLS_PER_HOUR));
        // What is shown is the plan's: its phase's green, its crossing on call.
        let v = j.view();
        let phase = v.signal.as_ref().unwrap().phases.iter().find(|p| p.uid == uid).unwrap();
        assert_eq!(phase.timing.green_s, 40);
        // A refusal is the edit's own, and changes nothing.
        assert!(!j.set_call_rate(uid, 999));
        assert_eq!(j.refusal(), Some(Refusal::CallRateOutOfRange));
        assert_eq!(j.revisions().count(), edits);
        // The default shown again, its own settings are edited.
        assert!(j.view_plan(DEFAULT_PLAN));
        assert_eq!(j.view().signal.as_ref().unwrap().phases.iter().find(|p| p.uid == uid).unwrap().timing.green_s, crate::junction::model::GREEN_DEFAULT_S);
        assert!(j.set_timing(uid, TimingField::Green, 30));
        assert_eq!(j.arm(uid).and_then(|a| a.timing).map(|t| t.green_s), Some(30));
        assert_eq!(j.current().plans[0].streets.iter().find(|x| x.uid == uid).and_then(|x| x.timing).map(|t| t.green_s), Some(40));
        // Undoing the plan's adding shows the default again.
        let mut k = signal().0;
        assert!(k.add_plan("Night") && k.view_plan(1));
        assert!(k.undo());
        assert_eq!(k.viewing(), DEFAULT_PLAN);
    }

    #[test]
    fn a_document_holding_a_great_many_plans_is_settled_to_the_first_distinct_ones_in_little_time() {
        let (j, _) = signal();
        let mut s = j.current().clone();
        // As many as a share of the largest size could hold: the same few ids at first, then each its own.
        s.plans = (0..50_000_u32).map(|k| TimingPlan { id: if k < 100 { k % 5 + 1 } else { k }, name: format!("P{k}"), streets: Vec::new() }).collect();
        let began = std::time::Instant::now();
        settle(&mut s);
        assert!(began.elapsed() < std::time::Duration::from_secs(1), "{:?}", began.elapsed());
        assert_eq!(s.plans.iter().map(|p| p.id).collect::<Vec<_>>(), [1, 2, 3, 4, 5, 100, 101], "the first of each id, `MAX_PLANS` with the default");
        assert_eq!(s.plans[0].name, "P0");
    }

    #[test]
    fn a_plan_holds_every_signal_setting_of_a_street_and_nothing_else() {
        let (j, _) = signal();
        let mut a = j.current().arms[0].clone();
        a.timing = Some(Timing { green_s: 41, ..Timing::default() });
        a.calls = Some(17);
        a.detection = calls::Detection::Passive;
        a.actuation = Some(actuation::Actuation { vehicles: 333, ..actuation::Actuation::default() });
        a.cyclists = Some(19);
        a.transit_priority = Some(transit::Priority { mode: transit::Mode::Both, buses: 23 });
        a.preemption = Some(3);
        // Every field of `Arm` named: one added fails to compile here until it is said to be a signal setting a plan
        // holds (and put in `Settings`, `of` and `put`) or the street's own.
        let Arm {
            uid,
            timing,
            calls,
            detection,
            actuation,
            cyclists,
            transit_priority,
            preemption,
            edge: _,
            section: _,
            beyond: _,
            stops: _,
            curb_near: _,
            bearing: _,
            offset_mm: _,
            corner_mm: _,
            lanes: _,
            crossing: _,
            bulb: _,
            bike_box: _,
            protected: _,
            banned: _,
            bus_lane: _,
            approach: _,
            approach_mm: _,
            stop: _,
            rule: _,
            filter: _,
            walked: _,
        } = a.clone();
        let held = Settings { uid, timing, calls, detection, actuation, cyclists, transit_priority, preemption };
        assert_eq!(Settings::of(&a), held);
        let mut b = j.current().arms[1].clone();
        let street_before = Arm { uid: b.uid, ..a.clone() };
        held.put(&mut b);
        assert_eq!(Settings::of(&b), Settings { uid: b.uid, ..held }, "every setting put");
        assert_ne!(b, street_before, "and nothing of the street's own");
        assert_eq!((b.bearing, b.lanes.clone()), (j.current().arms[1].bearing, j.current().arms[1].lanes.clone()));
    }

    #[test]
    fn every_timing_and_policy_edit_goes_to_the_plan_shown() {
        let mut j = crate::junction::model::fixtures::junction_biked();
        let n = j.current().arms.iter().find(|a| a.bearing == 0).unwrap().uid;
        assert!(j.set_bus_lane(n, true), "the street's own, for transit priority");
        assert!(j.add_plan("Night") && j.view_plan(1));
        let default_of = |j: &Junction| j.current().arms.iter().map(Settings::of).collect::<Vec<_>>();
        let plan_of = |j: &Junction| j.current().plans[0].streets.iter().find(|x| x.uid == n).cloned();
        let edits: [(&str, fn(&mut Junction, u32) -> bool); 13] = [
            ("set_timing", |j, n| j.set_timing(n, TimingField::Green, 37)),
            ("step_timing", |j, n| j.step_timing(n, TimingField::Green, 1)),
            ("set_on_call", |j, n| j.set_on_call(n, true)),
            ("set_detection", |j, n| j.set_detection(n, calls::Detection::Passive)),
            ("set_call_rate", |j, n| j.set_call_rate(n, 31)),
            ("set_actuated", |j, n| j.set_actuated(n, true)),
            ("set_actuation", |j, n| j.set_actuation(n, actuation::Field::Vehicles, 401)),
            ("set_bike_detection", |j, n| j.set_bike_detection(n, true)),
            ("set_cyclists", |j, n| j.set_cyclists(n, 21)),
            ("set_transit_priority", |j, n| j.set_transit_priority(n, Some(transit::Mode::Both))),
            ("set_buses", |j, n| j.set_buses(n, 29)),
            ("set_preemption", |j, n| j.set_preemption(n, true)),
            ("set_emergency_calls", |j, n| j.set_emergency_calls(n, 5)),
        ];
        for (name, edit) in edits {
            let (default, plan) = (default_of(&j), plan_of(&j));
            assert!(edit(&mut j, n), "{name}: {:?}", j.refusal());
            assert_eq!(default_of(&j), default, "{name} leaves the default as it was");
            assert_ne!(plan_of(&j), plan, "{name} changes the plan shown");
        }
    }

    #[test]
    fn a_street_that_joined_later_runs_as_in_the_default_even_after_the_plan_is_edited() {
        let (mut j, first) = signal();
        let later = j.current().arms[1].uid;
        assert!(j.add_plan("Night"));
        // The plan as a junction keeps it once a street has joined since it was made: nothing held for that street.
        assert!(j.edit(Said::new("jn-rev-plan-add"), move |s| {
            s.plans[0].streets.retain(|x| x.uid != later);
            true
        }));
        let green_in = |j: &Junction, id: u32| applied(j.current(), id).arms.iter().find(|a| a.uid == later).and_then(|a| a.timing).map(|t| t.green_s);
        assert!(j.view_plan(1) && j.set_timing(first, TimingField::Green, 40), "an edit of the plan on another street");
        assert!(j.current().plans[0].streets.iter().all(|x| x.uid != later), "the street the plan held nothing for is still not held");
        assert!(j.view_plan(DEFAULT_PLAN) && j.set_timing(later, TimingField::Green, 33));
        assert_eq!(green_in(&j, 1), Some(33), "the plan runs it as the default does, as the default changes");
        // Once edited in the plan itself, it is the plan's.
        assert!(j.view_plan(1) && j.set_timing(later, TimingField::Green, 25));
        assert!(j.view_plan(DEFAULT_PLAN) && j.set_timing(later, TimingField::Green, 35));
        assert_eq!(green_in(&j, 1), Some(25));
    }

    #[test]
    fn each_plan_is_checked_on_its_own_and_a_broken_plan_is_a_finding() {
        let (mut j, _) = signal();
        assert!(j.add_plan("Long"));
        assert!(j.view_plan(1));
        for a in j.current().arms.iter().map(|a| a.uid).collect::<Vec<_>>() {
            assert!(j.set_timing(a, TimingField::Green, 60));
        }
        assert!(j.view_plan(DEFAULT_PLAN));
        let v = j.view();
        let plans = &v.plans.as_ref().unwrap().plans;
        assert!(plans[0].findings.is_empty(), "the default is sound: {:?}", plans[0].findings.iter().map(en).collect::<Vec<_>>());
        assert!(plans[1].cycle_s > 120);
        assert_eq!(plans[1].findings.iter().map(|f| f.key).collect::<Vec<_>>(), ["jn-check-signal-cycle-bad"], "the long plan's cycle is a finding");
        // The checks shown are the plan shown's.
        let cycle_ok = |j: &Junction| j.view().checks.iter().find(|c| c.id == "signal-cycle").map(|c| c.ok);
        assert_eq!(cycle_ok(&j), Some(true));
        assert!(j.view_plan(1));
        assert_eq!(cycle_ok(&j), Some(false));
        assert_eq!(j.view().plans.unwrap().plans[1].findings.len(), 1, "the same finding, shown or not");
        // A policy's finding is its plan's alone: a crossing on call in one plan that no phase walks any more.
        let (mut j, uid) = signal();
        assert!(j.add_plan("Calls") && j.view_plan(1) && j.set_on_call(uid, true));
        let walking: Vec<usize> = crate::junction::model::signal::plan(j.current(), j.region)
            .unwrap()
            .phases
            .iter()
            .enumerate()
            .filter(|(_, p)| p.walks.contains(&uid))
            .map(|(i, _)| i)
            .collect();
        for i in walking {
            assert!(j.set_phase_walk(i, uid, false), "{:?}", j.refusal());
        }
        let v = j.view();
        let plans = &v.plans.as_ref().unwrap().plans;
        let keys = |p: &crate::junction::model::read_model::PlanView| p.findings.iter().map(|f| f.key).collect::<Vec<_>>();
        assert!(!keys(&plans[0]).contains(&"jn-calls-finding-unwalked"), "on recall in the default: no call to find");
        assert_eq!(keys(&plans[1]), [keys(&plans[0]), vec!["jn-calls-finding-unwalked"]].concat(), "the plan's own finding, after the timing's both share");
    }

    #[test]
    fn fixed_inputs_give_a_fixed_summary_for_each_plan() {
        let (mut j, uid) = signal();
        assert!(j.add_plan("Busy"));
        assert!(j.view_plan(1));
        assert!(j.set_on_call(uid, true) && j.set_call_rate(uid, 30));
        let totals = |j: &mut Junction, id: u32| {
            assert!(j.view_plan(id));
            j.view().signal.unwrap().calls.crossings.iter().find(|c| c.uid == uid).and_then(|c| c.totals)
        };
        let busy = totals(&mut j, 1).expect("on call in the busy plan");
        assert_eq!(totals(&mut j, DEFAULT_PLAN), None, "on recall in the default");
        assert_eq!(totals(&mut j, 1), Some(busy), "the same every time");
        assert_eq!((busy.calls, busy.served, busy.delayed, busy.not_served), (30, 14, 16, 0), "pinned");
        let mut again = signal().0;
        assert!(again.add_plan("Busy") && again.view_plan(1) && again.set_on_call(uid, true) && again.set_call_rate(uid, 30));
        assert_eq!(totals(&mut again, 1), Some(busy), "the same junction and plan give the same hour");
    }
}
