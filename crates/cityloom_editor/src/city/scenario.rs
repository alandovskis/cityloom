//! Named alternative designs for one place. A place's record keeps its current design as the baseline (what every
//! record written before scenarios holds); its scenarios are the alternatives kept beside it, each a whole design of
//! the same kind (a `Street` for a street, a `State` for a junction). No wording here: a refusal is a reason the
//! views word.

use serde::{Deserialize, Serialize};

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
    fn it_is_written_as_a_plain_list() {
        let json = serde_json::to_string(&two()).unwrap();
        assert_eq!(json, r#"[{"id":1,"name":"Wider bike lane","design":1},{"id":2,"name":"Parking kept","design":2}]"#);
        assert_eq!(serde_json::from_str::<Scenarios<i32>>(&json).unwrap(), two());
    }
}
