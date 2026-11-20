//! Named alternative designs for one place. A place's record keeps its current design as the baseline (what every
//! record written before scenarios holds); its scenarios are the alternatives kept beside it, each a whole design of
//! the same kind (a `Street` for a street, a `State` for a junction). No wording here: a refusal is a reason the
//! views word.

use serde::{Deserialize, Serialize};

use crate::shared::said::{Arg, Said};

/// The most scenarios one place keeps: a record stays well inside the server's `MAX_RECORD_BYTES`.
pub const MAX_SCENARIOS: usize = 8;
/// The longest name, in characters.
pub const MAX_NAME_CHARS: usize = 60;

/// One alternative: a name the person chose and the whole design.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scenario<T> {
    /// Unique among its place's scenarios (one more than the highest, so the last one's id is free again once it is deleted).
    pub id: u32,
    pub name: String,
    pub design: T,
}

/// Why a scenario action was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The name is empty once trimmed.
    NameEmpty,
    /// The name is longer than `MAX_NAME_CHARS`.
    NameTooLong,
    /// Another scenario of the place has this name (compared ignoring case).
    NameTaken,
    /// The place already keeps `MAX_SCENARIOS`.
    Full,
    /// No scenario has this id.
    Missing,
}

impl Refusal {
    /// What the refusal says, for a scenario that was to be called `name` (as typed).
    pub fn said(self, name: &str) -> Said {
        let max = |key, max: usize| Said::new(key).with("max", Arg::Num(max as i64));
        match self {
            Refusal::NameEmpty => Said::new("scenario-name-empty"),
            Refusal::NameTooLong => max("scenario-name-too-long", MAX_NAME_CHARS),
            Refusal::NameTaken => Said::new("scenario-name-taken").with("name", Arg::Text(name.trim().to_string())),
            Refusal::Full => max("scenario-full", MAX_SCENARIOS),
            Refusal::Missing => Said::new("scenario-missing"),
        }
    }
}

/// What a scenario action did, or what it asks, to be said to the person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum News {
    /// A scenario made from the baseline, now shown.
    Created(String),
    /// Scenario `from` copied as `name`, now shown.
    Duplicated {
        from: String,
        name: String,
    },
    Renamed {
        from: String,
        name: String,
    },
    /// A scenario deleted; the baseline is shown.
    Deleted(String),
    /// Deleting this scenario waits for the person to confirm it.
    AskDelete(String),
    /// The design now shown: a scenario by name, or the baseline.
    Showing(Option<String>),
    /// The scenario shown was deleted elsewhere; the baseline is shown. `edit_lost`: a change made to it since could
    /// not be kept.
    Gone {
        edit_lost: bool,
    },
}

impl News {
    pub fn said(&self) -> Said {
        let text = |s: &String| Arg::Text(s.clone());
        match self {
            News::Created(name) => Said::new("scenario-created").with("name", text(name)),
            News::Duplicated { from, name } => Said::new("scenario-duplicated").with("from", text(from)).with("name", text(name)),
            News::Renamed { from, name } => Said::new("scenario-renamed").with("from", text(from)).with("name", text(name)),
            News::Deleted(name) => Said::new("scenario-deleted").with("name", text(name)),
            News::AskDelete(name) => Said::new("scenario-delete-ask").with("name", text(name)),
            News::Showing(None) => Said::new("scenario-switched-baseline"),
            News::Showing(Some(name)) => Said::new("scenario-switched").with("name", text(name)),
            News::Gone { edit_lost: false } => Said::new("scenario-gone"),
            News::Gone { edit_lost: true } => Said::new("scenario-gone-edit"),
        }
    }
}

/// The scenarios of one place, in the order they were made.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Scenarios<T>(Vec<Scenario<T>>);

impl<T> Default for Scenarios<T> {
    fn default() -> Self {
        Scenarios(Vec::new())
    }
}

impl<T: Clone> Scenarios<T> {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Scenario<T>> {
        self.0.iter()
    }

    pub fn get(&self, id: u32) -> Option<&Scenario<T>> {
        self.0.iter().find(|s| s.id == id)
    }

    /// The name as it would be kept (trimmed), or why it cannot be, leaving out `except` (a scenario being renamed).
    fn checked(&self, name: &str, except: Option<u32>) -> Result<String, Refusal> {
        let name = name.trim();
        if name.is_empty() {
            return Err(Refusal::NameEmpty);
        }
        if name.chars().count() > MAX_NAME_CHARS {
            return Err(Refusal::NameTooLong);
        }
        let taken = self.0.iter().any(|s| Some(s.id) != except && s.name.to_lowercase() == name.to_lowercase());
        if taken { Err(Refusal::NameTaken) } else { Ok(name.to_string()) }
    }

    fn next_id(&self) -> u32 {
        self.0.iter().map(|s| s.id).max().map_or(1, |m| m + 1)
    }

    /// Keeps `design` as a new scenario; its id.
    pub fn create(&mut self, name: &str, design: T) -> Result<u32, Refusal> {
        if self.0.len() >= MAX_SCENARIOS {
            return Err(Refusal::Full);
        }
        let name = self.checked(name, None)?;
        let id = self.next_id();
        self.0.push(Scenario { id, name, design });
        Ok(id)
    }

    /// A copy of scenario `id` under `name`; the copy's id.
    pub fn duplicate(&mut self, id: u32, name: &str) -> Result<u32, Refusal> {
        let design = self.get(id).ok_or(Refusal::Missing)?.design.clone();
        self.create(name, design)
    }

    pub fn rename(&mut self, id: u32, name: &str) -> Result<(), Refusal> {
        let name = self.checked(name, Some(id))?;
        self.0.iter_mut().find(|s| s.id == id).ok_or(Refusal::Missing)?.name = name;
        Ok(())
    }

    /// Replaces the design of scenario `id` (an edit made within it).
    pub fn set_design(&mut self, id: u32, design: T) -> Result<(), Refusal> {
        self.0.iter_mut().find(|s| s.id == id).ok_or(Refusal::Missing)?.design = design;
        Ok(())
    }

    pub fn delete(&mut self, id: u32) -> Result<(), Refusal> {
        let at = self.0.iter().position(|s| s.id == id).ok_or(Refusal::Missing)?;
        self.0.remove(at);
        Ok(())
    }
}

/// One action of the management of a place's scenarios.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// A new scenario, named so, starting as the baseline.
    Create(String),
    /// A copy of scenario `id`, named so.
    Duplicate(u32, String),
    Rename(u32, String),
    Delete(u32),
}

impl<T: Clone> Scenarios<T> {
    /// Does `action`, a new scenario starting as `baseline`. The id of the scenario it made, if it made one.
    pub fn act(&mut self, action: Action, baseline: &T) -> Result<Option<u32>, Refusal> {
        match action {
            Action::Create(name) => self.create(&name, baseline.clone()).map(Some),
            Action::Duplicate(id, name) => self.duplicate(id, &name).map(Some),
            Action::Rename(id, name) => self.rename(id, &name).map(|()| None),
            Action::Delete(id) => self.delete(id).map(|()| None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two() -> Scenarios<i32> {
        let mut s = Scenarios::default();
        s.create("Wider bike lane", 1).unwrap();
        s.create("Parking kept", 2).unwrap();
        s
    }

    #[test]
    fn two_named_alternatives_are_kept_apart() {
        let s = two();
        assert_eq!(s.len(), 2);
        assert_eq!(s.get(1).unwrap().design, 1);
        assert_eq!(s.get(2).unwrap().name, "Parking kept");
    }

    #[test]
    fn a_name_is_trimmed_and_must_be_new_and_sensible() {
        let mut s = two();
        assert_eq!(s.create("  ", 3), Err(Refusal::NameEmpty));
        assert_eq!(s.create(&"x".repeat(MAX_NAME_CHARS + 1), 3), Err(Refusal::NameTooLong));
        assert_eq!(s.create("wider BIKE lane", 3), Err(Refusal::NameTaken));
        let id = s.create("  Third  ", 3).unwrap();
        assert_eq!(s.get(id).unwrap().name, "Third");
    }

    #[test]
    fn a_duplicate_copies_the_design_and_is_then_independent() {
        let mut s = two();
        let copy = s.duplicate(1, "Copy").unwrap();
        assert_eq!(s.get(copy).unwrap().design, 1);
        s.set_design(copy, 9).unwrap();
        assert_eq!(s.get(1).unwrap().design, 1);
        assert_eq!(s.duplicate(99, "Nope"), Err(Refusal::Missing));
    }

    #[test]
    fn a_rename_may_keep_its_own_name_but_not_take_another() {
        let mut s = two();
        assert_eq!(s.rename(1, "Wider bike lane"), Ok(()));
        assert_eq!(s.rename(1, "parking kept"), Err(Refusal::NameTaken));
        assert_eq!(s.rename(99, "Other"), Err(Refusal::Missing));
    }

    #[test]
    fn a_deleted_scenario_is_gone() {
        let mut s = two();
        s.delete(2).unwrap();
        assert_eq!(s.delete(2), Err(Refusal::Missing));
        assert_eq!(s.create("Fresh", 5).unwrap(), 2, "ids are one more than the highest kept");
    }

    #[test]
    fn a_place_keeps_a_limited_number() {
        let mut s = Scenarios::default();
        for i in 0..MAX_SCENARIOS {
            s.create(&format!("S{i}"), 0).unwrap();
        }
        assert_eq!(s.create("One more", 0), Err(Refusal::Full));
    }

    #[test]
    fn each_action_does_what_its_method_does() {
        let mut s = two();
        assert_eq!(s.act(Action::Create("New".into()), &7), Ok(Some(3)));
        assert_eq!(s.get(3).unwrap().design, 7, "a new scenario starts as the baseline");
        assert_eq!(s.act(Action::Duplicate(1, "Copy".into()), &7), Ok(Some(4)));
        assert_eq!(s.get(4).unwrap().design, 1);
        assert_eq!(s.act(Action::Rename(4, "new".into()), &7), Err(Refusal::NameTaken));
        assert_eq!(s.act(Action::Rename(4, "Copied".into()), &7), Ok(None));
        assert_eq!(s.act(Action::Delete(4), &7), Ok(None));
        assert_eq!(s.act(Action::Delete(4), &7), Err(Refusal::Missing));
    }

    fn said(said: &Said, locale: crate::shared::i18n::Locale) -> String {
        crate::shared::said::say_now(&crate::i18n_for(locale), crate::shared::units::Units::Metres, said)
    }

    #[test]
    fn each_refusal_is_worded_in_both_languages() {
        use crate::shared::i18n::Locale::{En, FrCa};
        let both = |r: Refusal| (said(&r.said("  Parking kept "), En), said(&r.said("  Parking kept "), FrCa));
        assert_eq!(both(Refusal::NameEmpty), ("Give the scenario a name.".into(), "Donnez un nom au scénario.".into()));
        assert_eq!(
            both(Refusal::NameTooLong),
            ("A scenario name can be at most 60 characters long.".into(), "Un nom de scénario compte au plus 60 caractères.".into())
        );
        assert_eq!(
            both(Refusal::NameTaken),
            ("Another scenario is already called “Parking kept”.".into(), "Un autre scénario s’appelle déjà «\u{a0}Parking kept\u{a0}».".into())
        );
        assert_eq!(
            both(Refusal::Full),
            (
                "A place keeps at most 8 scenarios. Delete one to make another.".into(),
                "Un lieu garde au plus 8 scénarios. Supprimez-en un pour en créer un autre.".into()
            )
        );
        assert_eq!(both(Refusal::Missing), ("That scenario is no longer here.".into(), "Ce scénario n’existe plus.".into()));
    }

    #[test]
    fn the_news_of_each_action_is_worded_in_both_languages() {
        use crate::shared::i18n::Locale::{En, FrCa};
        let name = || "Wider bike lane".to_string();
        assert_eq!(said(&News::Created(name()).said(), En), "Scenario “Wider bike lane” made from the baseline. Now editing it.");
        assert_eq!(said(&News::Showing(None).said(), FrCa), "La référence est maintenant affichée.");
        let ask = said(&News::AskDelete(name()).said(), FrCa);
        assert_eq!(ask, "Supprimer le scénario «\u{a0}Wider bike lane\u{a0}»? Appuyez sur Confirmer la suppression pour le supprimer.");
        for news in [
            News::Duplicated { from: name(), name: "Copy".into() },
            News::Renamed { from: name(), name: "Copy".into() },
            News::Deleted(name()),
            News::Showing(Some(name())),
        ] {
            for locale in [En, FrCa] {
                assert!(said(&news.said(), locale).contains("Wider bike lane"), "{news:?} {locale:?}");
            }
        }
    }

    #[test]
    fn it_is_written_as_a_plain_list() {
        let json = serde_json::to_string(&two()).unwrap();
        assert_eq!(json, r#"[{"id":1,"name":"Wider bike lane","design":1},{"id":2,"name":"Parking kept","design":2}]"#);
        assert_eq!(serde_json::from_str::<Scenarios<i32>>(&json).unwrap(), two());
    }
}
