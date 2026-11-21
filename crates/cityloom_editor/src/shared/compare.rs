//! The comparison page of two designs of a place (a baseline and its scenarios): which two are picked (`a=` and
//! `b=` in the address, `base` for the baseline), what differs between them, and the state and host the page's
//! view-model and view share. No wording here: the summary is `Said` data the view words.

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::i18n::I18n;
use crate::shared::said::{Arg, Said};
use crate::shared::scenarios::ScenarioItem;

/// One of the two sides of the comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    A,
    B,
}

/// What the address says of one side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Picked {
    /// The address does not name the side.
    Absent,
    Baseline,
    /// A scenario of the place.
    Scenario(u32),
    /// The address names something that is not the baseline nor a scenario of the place.
    Missing,
}

/// The query's parameters, from `search` with or without its `?` (and without any `#…`).
fn params(search: &str) -> impl Iterator<Item = (&str, &str)> {
    let search = search.split('#').next().unwrap_or("");
    let search = search.rsplit_once('?').map_or(search, |(_, query)| query);
    search.split('&').filter(|p| !p.is_empty()).map(|p| p.split_once('=').unwrap_or((p, "")))
}

/// What `key` (`a` or `b`) of the query `search` picks among the place's scenario `ids`.
pub fn parse_pick(search: &str, key: &str, ids: &[u32]) -> Picked {
    let Some((_, value)) = params(search).find(|(k, _)| *k == key) else {
        return Picked::Absent;
    };
    if value == "base" {
        return Picked::Baseline;
    }
    match value.parse::<u32>() {
        Ok(id) if ids.contains(&id) => Picked::Scenario(id),
        _ => Picked::Missing,
    }
}

/// `path` with `a=` and `b=` set (`base` for None): a present one is replaced where it stands, an absent one is
/// appended; every other parameter keeps its place, and so does the fragment.
pub fn with_picks(path: &str, a: Option<u32>, b: Option<u32>) -> String {
    let (rest, fragment) = path.split_once('#').map_or((path, None), |(rest, fragment)| (rest, Some(fragment)));
    let (base, query) = rest.split_once('?').map_or((rest, ""), |(base, query)| (base, query));
    let word = |id: Option<u32>| id.map_or("base".to_string(), |id| id.to_string());
    let mut parts: Vec<String> = query.split('&').filter(|p| !p.is_empty()).map(str::to_string).collect();
    for (key, id) in [("a", a), ("b", b)] {
        let part = format!("{key}={}", word(id));
        match parts.iter_mut().find(|p| p.split_once('=').map_or(p.as_str(), |(k, _)| k) == key) {
            Some(existing) => *existing = part,
            None => parts.push(part),
        }
    }
    let mut out = format!("{base}?{}", parts.join("&"));
    if let Some(fragment) = fragment {
        out.push('#');
        out.push_str(fragment);
    }
    out
}

/// The two designs the page opens on, and whether a pick of the address named none of the place's: A is the
/// baseline unless named, B the first scenario (else the baseline); a missing pick falls back to the baseline.
pub fn initial_picks(search: &str, ids: &[u32]) -> (Option<u32>, Option<u32>, bool) {
    let a = parse_pick(search, "a", ids);
    let b = parse_pick(search, "b", ids);
    let missing = a == Picked::Missing || b == Picked::Missing;
    let side = |picked: Picked, default: Option<u32>| match picked {
        Picked::Absent => default,
        Picked::Scenario(id) => Some(id),
        Picked::Baseline | Picked::Missing => None,
    };
    (side(a, None), side(b, ids.first().copied()), missing)
}

/// What differs between the two designs, under one heading.
#[derive(Clone, Debug, PartialEq)]
pub struct DiffGroup {
    /// The message key of the heading.
    pub heading: &'static str,
    pub lines: Vec<Said>,
}

/// What differs between the two designs: the groups the page lists, and what else differs that has no group.
#[derive(Clone, Debug, PartialEq)]
pub struct Summary {
    pub groups: Vec<DiffGroup>,
    pub also: Vec<Said>,
}

impl Summary {
    pub fn is_same(&self) -> bool {
        self.also.is_empty() && self.groups.iter().all(|g| g.lines.is_empty())
    }

    /// The sentence over the summary, about the designs `a` and `b` by name.
    pub fn headline(&self, a: &str, b: &str) -> Said {
        let (a, b) = (Arg::Text(a.to_string()), Arg::Text(b.to_string()));
        if self.is_same() {
            return Said::new("cmp-same").with("a", a).with("b", b);
        }
        let n = self.groups.iter().map(|g| g.lines.len()).sum::<usize>() + self.also.len();
        Said::new("cmp-differs").with("a", a).with("b", b).with("n", Arg::Num(n as i64))
    }
}

/// The scenarios of the place, which two are picked, and what the last pick was refused for.
#[derive(Clone)]
pub struct CompareState {
    items: ArcRwSignal<Vec<ScenarioItem>>,
    /// The design on side A; None: the baseline.
    a: ArcRwSignal<Option<u32>>,
    b: ArcRwSignal<Option<u32>>,
    refusal: ArcRwSignal<Option<Said>>,
}

impl Default for CompareState {
    fn default() -> Self {
        CompareState { items: ArcRwSignal::new(Vec::new()), a: ArcRwSignal::new(None), b: ArcRwSignal::new(None), refusal: ArcRwSignal::new(None) }
    }
}

impl CompareState {
    fn side(&self, side: Pick) -> &ArcRwSignal<Option<u32>> {
        match side {
            Pick::A => &self.a,
            Pick::B => &self.b,
        }
    }

    pub fn items(&self) -> Vec<ScenarioItem> {
        self.items.get()
    }

    /// The design picked on `side` (None: the baseline), for a view (watched).
    pub fn pick(&self, side: Pick) -> Option<u32> {
        self.side(side).get()
    }

    /// The same, for a command.
    pub fn pick_now(&self, side: Pick) -> Option<u32> {
        self.side(side).get_untracked()
    }

    pub fn refusal(&self) -> Option<Said> {
        self.refusal.get()
    }

    /// The place's scenarios as kept now, by id and name.
    pub fn list(&self, items: Vec<(u32, String)>) {
        let items: Vec<ScenarioItem> = items.into_iter().map(|(id, name)| ScenarioItem { id, name }).collect();
        if self.items.with_untracked(|now| *now != items) {
            self.items.set(items);
        }
    }

    /// Picks `id` (None: the baseline) on `side`.
    pub fn set(&self, side: Pick, id: Option<u32>) {
        self.side(side).set(id);
        self.settle();
    }

    pub fn refuse(&self, why: Said) {
        self.refusal.set(Some(why));
    }

    /// An action was done: nothing is refused any more.
    pub fn settle(&self) {
        if self.refusal.with_untracked(Option::is_some) {
            self.refusal.set(None);
        }
    }

    /// The name of design `id` (None: the baseline, called `baseline`), for a command.
    pub fn name_of(&self, id: Option<u32>, baseline: &str) -> String {
        match id {
            None => baseline.to_string(),
            Some(id) => self.items.with_untracked(|items| items.iter().find(|i| i.id == id).map(|i| i.name.clone())).unwrap_or_else(|| baseline.to_string()),
        }
    }
}

/// A page that compares two designs of a place: the street's or the junction's comparison view-model.
pub trait CompareHost {
    fn i18n(&self) -> Rc<I18n>;
    fn compare_state(&self) -> &CompareState;
    /// Picks design `id` (None: the baseline) on `side`.
    fn choose(&self, side: Pick, id: Option<u32>);
    /// What differs between the two designs picked.
    fn summary(&self) -> Summary;
    /// Where the page's back link goes, and the message key of its label.
    fn back_href(&self) -> String;
    fn back_key(&self) -> &'static str;
    /// The place's name, to say.
    fn place_name(&self) -> Said;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pick_is_base_an_id_of_the_place_or_missing() {
        assert_eq!(parse_pick("street=7&a=base", "a", &[1, 2]), Picked::Baseline);
        assert_eq!(parse_pick("street=7&a=2", "a", &[1, 2]), Picked::Scenario(2));
        assert_eq!(parse_pick("street=7", "a", &[1, 2]), Picked::Absent);
        for bad in ["a=9", "a=abc", "a="] {
            assert_eq!(parse_pick(bad, "a", &[1, 2]), Picked::Missing, "{bad}");
        }
    }

    #[test]
    fn picks_are_written_into_the_address_keeping_the_rest() {
        assert_eq!(with_picks("/compare.html?street=7&project=p1&view=1", None, Some(2)), "/compare.html?street=7&project=p1&view=1&a=base&b=2");
        assert_eq!(with_picks("/compare.html?a=1&street=7&b=2#x", Some(2), None), "/compare.html?a=2&street=7&b=base#x");
    }

    #[test]
    fn the_first_scenario_is_b_by_default_and_a_missing_pick_is_the_baseline() {
        assert_eq!(initial_picks("street=7", &[3, 5]), (None, Some(3), false));
        assert_eq!(initial_picks("street=7", &[]), (None, None, false));
        assert_eq!(initial_picks("street=7&a=5&b=9", &[3, 5]), (Some(5), None, true));
    }

    #[test]
    fn a_summary_with_nothing_in_it_is_the_same_and_counts_its_lines_otherwise() {
        let same = Summary { groups: vec![DiffGroup { heading: "cmp-h-space", lines: vec![] }], also: vec![] };
        assert!(same.is_same());
        assert_eq!(same.headline("Baseline", "Wider"), Said::new("cmp-same").with("a", Arg::Text("Baseline".into())).with("b", Arg::Text("Wider".into())));
        let two = Summary { groups: vec![DiffGroup { heading: "cmp-h-space", lines: vec![Said::new("x")] }], also: vec![Said::new("y")] };
        assert!(!two.is_same());
        let expected = Said::new("cmp-differs").with("a", Arg::Text("Baseline".into())).with("b", Arg::Text("Wider".into())).with("n", Arg::Num(2));
        assert_eq!(two.headline("Baseline", "Wider"), expected);
    }

    #[test]
    fn the_state_keeps_the_picks_and_names_them() {
        let state = CompareState::default();
        state.list(vec![(3, "Wider".into())]);
        state.refuse(Said::new("x"));
        state.set(Pick::B, Some(3));
        assert_eq!((state.pick_now(Pick::A), state.pick_now(Pick::B)), (None, Some(3)));
        assert!(state.refusal().is_none());
        assert_eq!(state.name_of(Some(3), "Baseline"), "Wider");
        assert_eq!(state.name_of(None, "Baseline"), "Baseline");
    }
}
