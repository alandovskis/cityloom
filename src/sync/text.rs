//! The words syncing says that are not the editors' own.
use crate::shared::i18n::{Args, I18n};
use crate::sync::vm::Status;

/// What the account menu says of the sync; None when nothing is synced (signed out, or storage not on the server).
/// Tracked: ask in a view and it follows a language switch.
pub fn status_text(i18n: &I18n, status: Status) -> Option<String> {
    let key = match status {
        Status::Off => return None,
        Status::Synced => "sync-status-synced",
        Status::NotSynced => "sync-status-pending",
        Status::Paused => "sync-status-paused",
    };
    Some(i18n.tr(key, &Args::new()))
}

/// What is said when the server refuses the work for its size; `limit` is the name the server gave.
pub fn too_large_key(limit: &str) -> &'static str {
    match limit {
        "record" => "sync-too-large-record",
        "records" => "sync-too-large-records",
        "bytes" => "sync-too-large-bytes",
        _ => "sync-too-large-other",
    }
}
