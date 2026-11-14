//! A design milestone: a named snapshot of one project's work (its streets, junctions and area rules), kept so the
//! design can be looked at, compared with the present and put back. No wording and no I/O here.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The longest name, in characters.
pub const MAX_NAME_CHARS: usize = 80;
/// The most one milestone may weigh as JSON: what the server takes of one record (`cityloom_server::work::MAX_RECORD_BYTES`).
pub const MAX_BYTES: usize = 1024 * 1024;
/// The most a project's milestones may weigh together on a device. The browser's storage (about 5 MiB for the whole
/// site) also holds every project's work, so history must not crowd the work out.
pub const MAX_TOTAL_BYTES: usize = 3 * 1024 * 1024;

fn is_false(b: &bool) -> bool {
    !*b
}

/// How much work a snapshot holds, for the timeline's compact line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
    pub streets: usize,
    pub junctions: usize,
    pub rules: usize,
}

impl Summary {
    pub fn is_empty(&self) -> bool {
        self.streets + self.junctions + self.rules == 0
    }
}

/// What the timeline lists: a milestone without its snapshot. The key names it, whatever `id` says.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Milestone {
    #[serde(default)]
    pub id: String,
    pub name: String,
    /// When it was made (`sync::model::stamp`, UTC to the millisecond, so two compare as text).
    pub created_at: String,
    /// Made by the app (before a restore), not named by the person.
    #[serde(default, skip_serializing_if = "is_false")]
    pub auto: bool,
    /// The build that made it.
    pub app_version: String,
    /// Who was signed in when it was made, if anyone was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    pub summary: Summary,
}

/// The work a milestone holds, as the stores keep it: the `cityloom-edits` and `cityloom-restrictions` documents.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Snapshot {
    pub edits: Option<Value>,
    pub restrictions: Option<Value>,
}

/// What a new milestone is made of; the store gives it an id and a time.
#[derive(Clone, Debug, PartialEq)]
pub struct Draft {
    pub name: String,
    pub auto: bool,
    pub app_version: String,
    pub author: Option<String>,
    pub summary: Summary,
    pub snapshot: Snapshot,
}

/// Why a change to a milestone was not taken. Data only: the words come later, as `Said`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    Empty,
    TooLong,
    /// The snapshot is more than one record can hold.
    TooLarge,
    /// The project's milestones already weigh as much as they may (`MAX_TOTAL_BYTES`): one has to go first.
    Full,
    Unknown,
    /// The document is damaged or a newer build's, or storage refused.
    Unusable,
}

/// The name as kept: trimmed, 1 to 80 characters.
pub fn clean_name(name: &str) -> Result<String, Refused> {
    let name = name.trim();
    match name.chars().count() {
        0 => Err(Refused::Empty),
        n if n > MAX_NAME_CHARS => Err(Refused::TooLong),
        _ => Ok(name.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_trimmed_and_between_one_and_eighty_characters() {
        assert_eq!(clean_name("  Before the bike lanes "), Ok("Before the bike lanes".to_string()));
        assert_eq!(clean_name("   "), Err(Refused::Empty));
        assert_eq!(clean_name(&"é".repeat(80)).map(|n| n.chars().count()), Ok(80));
        assert_eq!(clean_name(&"é".repeat(81)), Err(Refused::TooLong));
    }

    #[test]
    fn a_milestone_is_written_without_what_is_absent_and_read_with_defaults() {
        let m = Milestone {
            id: "m1".into(),
            name: "A".into(),
            created_at: "2027-01-15T08:00:00.000Z".into(),
            auto: false,
            app_version: "0.68.0".into(),
            author: None,
            summary: Summary { streets: 2, junctions: 1, rules: 0 },
        };
        let json = serde_json::to_value(&m).unwrap();
        assert!(json.get("auto").is_none() && json.get("author").is_none(), "{json}");
        assert_eq!(json["summary"], serde_json::json!({ "streets": 2, "junctions": 1, "rules": 0 }));
        let back: Milestone = serde_json::from_value(json).unwrap();
        assert_eq!(back, m);
        let newer = serde_json::json!({ "name": "B", "created_at": "x", "app_version": "9", "summary": {"streets":0,"junctions":0,"rules":0}, "future": [1] });
        assert_eq!(serde_json::from_value::<Milestone>(newer).unwrap().id, "", "a field a newer build adds is ignored");
    }
}
