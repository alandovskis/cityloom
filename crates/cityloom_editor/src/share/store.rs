//! The share links made on this device for one project, so their maker can copy them again and delete them:
//! `cityloom-shares` (`{"version":1,"shares":[{token, created_at, expires_at}]}`, oldest first), one of a project's
//! own keys (`shared::scoped::SCOPED_KEYS`, so `cityloom-shares@<project>`, blanked with the project). Takes a
//! project's storage. Never synced: the server keeps no token, so a link made on one device is listed on that one.

use std::rc::Rc;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::shared::ports::Storage;

pub const SHARES_KEY: &str = "cityloom-shares";
pub const VERSION: u32 = 1;
/// How long past its expiry a link is still kept and asked about: this device's clock may be that far wrong. Past it,
/// the server has surely let it expire, and the list forgets it, so it never grows without bound.
pub const CLOCK_SKEW_MS: u64 = 24 * 3600 * 1000;

/// A link made here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Made {
    pub token: String,
    /// When it was made and when the server lets it go (sync stamps, UTC to the millisecond).
    pub created_at: String,
    pub expires_at: String,
}

pub struct ShareStore {
    storage: Rc<dyn Storage>,
}

impl ShareStore {
    pub fn new(storage: Rc<dyn Storage>) -> ShareStore {
        ShareStore { storage }
    }

    /// The links made here for the project, oldest first; none where the document is damaged or a newer build's (the
    /// links still work: only this list of them is lost).
    pub fn list(&self) -> Vec<Made> {
        let Some(raw) = self.storage.recall(SHARES_KEY).filter(|s| !s.trim().is_empty()) else { return Vec::new() };
        let Ok(value) = serde_json::from_str::<Value>(&raw) else { return Vec::new() };
        if value["version"].as_u64() != Some(u64::from(VERSION)) {
            return Vec::new();
        }
        serde_json::from_value(value["shares"].clone()).unwrap_or_default()
    }

    fn write(&self, shares: &[Made]) -> bool {
        self.storage.remember(SHARES_KEY, &json!({ "version": VERSION, "shares": shares }).to_string())
    }

    /// The stamp before which an expired link is no longer worth keeping or asking about (`CLOCK_SKEW_MS`).
    fn stale_before(&self) -> String {
        crate::sync::model::stamp(self.storage.now_ms().saturating_sub(CLOCK_SKEW_MS))
    }

    /// The links worth asking the server about: all but those expired more than `CLOCK_SKEW_MS` ago.
    pub fn worth_asking(&self) -> Vec<Made> {
        let stale = self.stale_before();
        self.list().into_iter().filter(|m| m.expires_at > stale).collect()
    }

    /// Keeps a link just made, forgetting those expired more than `CLOCK_SKEW_MS` ago. False where storage refused (the
    /// link works all the same).
    pub fn add(&self, made: Made) -> bool {
        let mut shares = self.worth_asking();
        shares.retain(|s| s.token != made.token);
        shares.push(made);
        self.write(&shares)
    }

    /// Forgets a link (deleted, or gone from the server).
    pub fn remove(&self, token: &str) -> bool {
        let mut shares = self.list();
        let before = shares.len();
        shares.retain(|s| s.token != token);
        shares.len() == before || self.write(&shares)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::ports::MemoryStorage;

    fn made(token: &str) -> Made {
        Made { token: token.into(), created_at: "2027-01-15T08:00:00.000Z".into(), expires_at: "2027-04-15T08:00:00.000Z".into() }
    }

    #[test]
    fn links_are_kept_oldest_first_and_forgotten() {
        let memory = Rc::new(MemoryStorage::default());
        let store = ShareStore::new(memory.clone());
        assert!(store.list().is_empty());
        assert!(store.add(made("a")) && store.add(made("b")));
        assert_eq!(store.list().iter().map(|m| m.token.as_str()).collect::<Vec<_>>(), ["a", "b"]);
        assert!(store.add(made("a")), "made again, it moves last");
        assert_eq!(store.list().iter().map(|m| m.token.as_str()).collect::<Vec<_>>(), ["b", "a"]);
        assert!(store.remove("b") && store.remove("nothing"));
        assert_eq!(store.list(), vec![made("a")]);
        memory.remember(SHARES_KEY, "{\"version\":2,\"shares\":[]}");
        assert!(store.list().is_empty(), "a newer build's is not read");
    }

    #[test]
    fn a_link_long_past_its_expiry_is_forgotten_when_another_is_made() {
        let memory = Rc::new(MemoryStorage::default());
        memory.set_now(1_800_000_000_000); // 2027-01-15T08:00:00Z
        let store = ShareStore::new(memory.clone());
        let ending = |token: &str, expires_at: &str| Made { token: token.into(), created_at: "2026-10-01T00:00:00.000Z".into(), expires_at: expires_at.into() };
        assert!(store.add(ending("stale", "2027-01-14T07:59:59.000Z")) && store.add(ending("late", "2027-01-14T08:00:01.000Z")));
        assert!(store.add(made("new")));
        assert_eq!(store.list().iter().map(|m| m.token.as_str()).collect::<Vec<_>>(), ["late", "new"], "a day past expiry, it goes");
        assert_eq!(store.worth_asking().iter().map(|m| m.token.as_str()).collect::<Vec<_>>(), ["late", "new"]);
        memory.set_now(1_800_000_000_000 + 2_000);
        assert_eq!(store.worth_asking().iter().map(|m| m.token.as_str()).collect::<Vec<_>>(), ["new"], "nor is one past that asked about");
    }
}
