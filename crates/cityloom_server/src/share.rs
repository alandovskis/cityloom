//! A shared design: a snapshot of one project, kept behind an unguessable link for `RETENTION_DAYS`, readable by
//! anyone who has the link and by nobody else. The rules here are over metadata and the token, no I/O; storage is
//! the Postgres `ShareRepo`'s (`db/share.rs`).
//!
//! **The token** is 32 bytes from the operating system's generator, written base64url without padding (43
//! characters); it is the whole link's secret and the server never stores it. A row is found by `lookup(token)`, a
//! SHA-256 of the token under a domain of its own (as a session is found by its token's hash: 256 random bits need no
//! key, and a keyed hash could not be recomputed when the lookup key rotates, since the token is not kept).
//!
//! **The body** is sealed (AES-256-GCM, `crypto::seal`, AD `share.body.<row id>`) under a key derived from the token
//! (`body_key`: HMAC-SHA256 keyed with the token), not under a master key or the creator's data key: a reader with
//! no account opens it, and the server holds nothing that opens it without the link. A database and its keys taken
//! together still give no shared design.
//!
//! **Lifetime.** A share expires `RETENTION_DAYS` after it is made; its creator may delete it before. Either way the
//! body is dropped (an expired one at the next purge, at the latest on its next read) and a marker row stays
//! (`MARKER_DAYS` past the expiry), so the link says the share has expired or was deleted rather than that it never
//! existed. Deleting the account deletes its shares, markers too (the link then says not found).
use async_trait::async_trait;
use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::crypto::{Key, random_bytes};

/// How long a share is kept after it is made.
pub const RETENTION_DAYS: i64 = 90;
/// How long a marker stays after the share's expiry, so its link says "expired" or "deleted" rather than "not found".
pub const MARKER_DAYS: i64 = 365;
/// The most a share's body may weigh: what the server takes of one record of work (`work::MAX_RECORD_BYTES`).
pub const MAX_BODY_BYTES: usize = crate::work::MAX_RECORD_BYTES;
/// The most live shares (not expired, not deleted) one account may hold.
pub const MAX_LIVE_SHARES: usize = 50;
/// The `kind` of every share document.
pub const MAGIC: &str = "cityloom-share";
/// The newest share format this server takes.
pub const FORMAT_VERSION: u64 = 1;
const TOKEN_BYTES: usize = 32;
const TOKEN_LEN: usize = 43;

/// A share's link secret. Never logged: `Debug` shows nothing of it.
#[derive(Clone, PartialEq, Eq)]
pub struct Token([u8; TOKEN_BYTES]);

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Token(..)")
    }
}

impl Token {
    /// A new token from the operating system's generator.
    pub fn random() -> Token {
        Token(random_bytes())
    }

    /// The token a link carries; None for anything that is not 43 characters of base64url that read as 32 bytes.
    pub fn parse(text: &str) -> Option<Token> {
        if text.len() != TOKEN_LEN {
            return None;
        }
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(text).ok()?;
        Some(Token(bytes.try_into().ok()?))
    }

    pub fn text(&self) -> String {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(self.0)
    }

    /// What the row is found by: SHA-256 of the token under the share's own domain.
    pub fn lookup(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(b"cityloom.share.lookup\0");
        hash.update(self.0);
        hash.finalize().into()
    }

    /// The key the body is sealed under: HMAC-SHA256 keyed with the token. Only the link opens it.
    pub fn body_key(&self) -> Key {
        let mut mac = <Hmac<Sha256> as hmac::KeyInit>::new_from_slice(&self.0).expect("HMAC takes any key length");
        mac.update(b"cityloom.share.body.v1");
        Key::from_bytes(mac.finalize().into_bytes().into())
    }
}

/// The associated data a body is sealed with: `share.body.<row id>`.
pub fn aad(id: Uuid) -> String {
    crate::crypto::aad("share", "body", &id.to_string())
}

pub fn expires_at(created_at: OffsetDateTime) -> OffsetDateTime {
    created_at + time::Duration::days(RETENTION_DAYS)
}

/// Why a share's body was not taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Invalid {
    /// Over `MAX_BODY_BYTES`.
    TooLarge,
    /// Not a JSON object of `kind` `cityloom-share` with a whole `format_version` of 1 or more.
    NotAShare,
    /// A `format_version` this server does not know.
    Newer,
}

/// Checks what the server must know of a body: its size, that it is a share, and of a format this server knows. The
/// rest passes through sealed, as the browser sent it.
pub fn check_body(body: &[u8]) -> Result<(), Invalid> {
    if body.len() > MAX_BODY_BYTES {
        return Err(Invalid::TooLarge);
    }
    let value: serde_json::Value = serde_json::from_slice(body).map_err(|_| Invalid::NotAShare)?;
    let doc = value.as_object().ok_or(Invalid::NotAShare)?;
    if doc.get("kind").and_then(serde_json::Value::as_str) != Some(MAGIC) {
        return Err(Invalid::NotAShare);
    }
    match doc.get("format_version").and_then(serde_json::Value::as_u64) {
        None | Some(0) => Err(Invalid::NotAShare),
        Some(v) if v > FORMAT_VERSION => Err(Invalid::Newer),
        Some(_) => Ok(()),
    }
}

/// How a share stands, from its row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    Live,
    Expired,
    Deleted,
}

/// A row's standing at `now`: deleted by its creator, past its expiry, or live.
pub fn standing(expires_at: OffsetDateTime, deleted: bool, now: OffsetDateTime) -> Standing {
    if deleted {
        Standing::Deleted
    } else if expires_at <= now {
        Standing::Expired
    } else {
        Standing::Live
    }
}

/// A share just made: the link's token and when it expires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Created {
    pub token: Token,
    pub expires_at: OffsetDateTime,
}

/// Why a share call failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ShareError {
    #[error("the store is unavailable")]
    Unavailable,
    #[error("a stored value could not be read")]
    Corrupt,
    #[error("the account no longer exists")]
    Gone,
    #[error("no share has this link")]
    NotFound,
    #[error("the share has expired")]
    Expired,
    #[error("the share was deleted")]
    Deleted,
    #[error("the share is another account's")]
    NotYours,
    #[error("the account holds as many shares as it may")]
    Full,
}

impl From<crate::accounts::RepoError> for ShareError {
    fn from(e: crate::accounts::RepoError) -> ShareError {
        use crate::accounts::RepoError;
        match e {
            RepoError::Unavailable => ShareError::Unavailable,
            RepoError::Gone => ShareError::Gone,
            RepoError::Corrupt | RepoError::Duplicate => ShareError::Corrupt,
        }
    }
}

/// The shares, sealed at rest under keys derived from their tokens.
#[async_trait]
pub trait ShareRepo: Send + Sync {
    /// Keeps `body` (checked already) for `user` behind `token`, until `now` + `RETENTION_DAYS`. `Full` past
    /// `MAX_LIVE_SHARES` live shares, `Gone` for an account that is no more.
    async fn create(&self, user: Uuid, token: &Token, body: &[u8], now: OffsetDateTime) -> Result<Created, ShareError>;
    /// The body behind `token`: `NotFound`, `Expired` (its body dropped then if it was not yet) or `Deleted`.
    async fn open(&self, token: &Token, now: OffsetDateTime) -> Result<Vec<u8>, ShareError>;
    /// Drops the body of `user`'s share behind `token` and keeps its marker, marked deleted where it is live at `now`.
    /// `NotFound` for no such share, `NotYours` for another account's; a share that has expired stays expired (its link
    /// says so), one deleted already stays deleted, and either is `Ok`: there is nothing left to delete.
    async fn delete(&self, user: Uuid, token: &Token, now: OffsetDateTime) -> Result<(), ShareError>;
    /// Drops the bodies of expired shares and the markers `MARKER_DAYS` past their expiry, and says how many of each.
    async fn purge(&self, now: OffsetDateTime) -> Result<Purged, ShareError>;
}

/// What a purge did: the bodies of expired shares it dropped, and the markers it removed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Purged {
    pub bodies: u64,
    pub markers: u64,
}

/// Purges once the database is ready (`ready`, set when the migrations have run: before, the table may not exist yet),
/// and then once a day. A link to an expired share is refused on read anyway, so a missed day keeps a sealed body a
/// little longer, never serves it.
pub fn spawn_purge(
    repo: std::sync::Arc<dyn ShareRepo>,
    clock: std::sync::Arc<dyn crate::clock::Clock>,
    ready: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while !ready.load(std::sync::atomic::Ordering::Acquire) {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        let mut every_day = tokio::time::interval(std::time::Duration::from_secs(24 * 3600));
        loop {
            every_day.tick().await; // the first tick is at once: that is the start-up pass
            match repo.purge(clock.now()).await {
                Ok(Purged { bodies, markers }) => tracing::info!(bodies, markers, "expired shares purged"),
                Err(e) => tracing::warn!(%e, "expired shares could not be purged"),
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(days: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap() + time::Duration::days(days)
    }

    #[test]
    fn a_token_is_43_characters_of_base64url_and_reads_back() {
        let token = Token::random();
        let text = token.text();
        assert_eq!(text.len(), 43);
        assert!(text.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'), "{text}");
        assert_eq!(Token::parse(&text), Some(token.clone()));
        assert_ne!(Token::random(), token, "two tokens differ");
    }

    #[test]
    fn anything_else_is_no_token() {
        let good = Token::random().text();
        for bad in ["", "short", &good[..42], &format!("{good}A"), &format!("{}+", &good[..42]), &format!("{}=", &good[..42])] {
            assert_eq!(Token::parse(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn the_lookup_and_the_body_key_depend_on_the_token_and_differ_from_each_other() {
        let (a, b) = (Token([1; 32]), Token([2; 32]));
        assert_eq!(a.lookup(), a.lookup());
        assert_ne!(a.lookup(), b.lookup());
        assert_ne!(a.body_key().0, b.body_key().0);
        assert_ne!(a.lookup(), a.body_key().0, "the stored lookup is not the key");
        assert_ne!(&a.lookup()[..], &Sha256::digest(a.0)[..], "the lookup has a domain of its own");
    }

    #[test]
    fn debug_never_shows_the_token() {
        assert_eq!(format!("{:?}", Token([7; 32])), "Token(..)");
    }

    #[test]
    fn a_share_is_kept_ninety_days() {
        assert_eq!(expires_at(at(0)), at(90));
        assert_eq!(standing(at(90), false, at(89)), Standing::Live);
        assert_eq!(standing(at(90), false, at(90)), Standing::Expired);
        assert_eq!(standing(at(90), true, at(1)), Standing::Deleted);
        assert_eq!(standing(at(90), true, at(100)), Standing::Deleted, "deleted says so even after it would have expired");
    }

    #[test]
    fn the_body_must_be_a_share_of_a_known_format_and_not_too_large() {
        assert_eq!(check_body(br#"{"kind":"cityloom-share","format_version":1,"project":{}}"#), Ok(()));
        assert_eq!(check_body(br#"{"kind":"cityloom-share","format_version":2}"#), Err(Invalid::Newer));
        for bad in [
            &br#"{"kind":"cityloom-design","format_version":1}"#[..],
            br#"{"kind":"cityloom-share"}"#,
            br#"{"kind":"cityloom-share","format_version":0}"#,
            br#"{"kind":"cityloom-share","format_version":"1"}"#,
            b"[1]",
            b"not json",
        ] {
            assert_eq!(check_body(bad), Err(Invalid::NotAShare), "{}", String::from_utf8_lossy(bad));
        }
        let pad = |n: usize| format!(r#"{{"kind":"cityloom-share","format_version":1,"pad":"{}"}}"#, " ".repeat(n));
        let base = pad(0).len();
        assert_eq!(check_body(pad(MAX_BODY_BYTES - base).as_bytes()), Ok(()), "exactly the limit");
        assert_eq!(check_body(pad(MAX_BODY_BYTES - base + 1).as_bytes()), Err(Invalid::TooLarge));
    }

    /// A `ShareRepo` that counts its purges.
    #[derive(Default)]
    struct Counting(std::sync::atomic::AtomicUsize);

    #[async_trait]
    impl ShareRepo for Counting {
        async fn create(&self, _: Uuid, _: &Token, _: &[u8], _: OffsetDateTime) -> Result<Created, ShareError> {
            Err(ShareError::Unavailable)
        }
        async fn open(&self, _: &Token, _: OffsetDateTime) -> Result<Vec<u8>, ShareError> {
            Err(ShareError::Unavailable)
        }
        async fn delete(&self, _: Uuid, _: &Token, _: OffsetDateTime) -> Result<(), ShareError> {
            Err(ShareError::Unavailable)
        }
        async fn purge(&self, _: OffsetDateTime) -> Result<Purged, ShareError> {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(Purged { bodies: 1, markers: 2 })
        }
    }

    #[tokio::test(start_paused = true)]
    async fn the_first_purge_waits_for_the_database_to_be_ready_then_one_runs_each_day() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        let repo = Arc::new(Counting::default());
        let ready = Arc::new(AtomicBool::new(false));
        let clock = Arc::new(crate::clock::FixedClock::new(at(0)));
        let handle = spawn_purge(repo.clone(), clock, ready.clone());
        tokio::time::advance(std::time::Duration::from_secs(30)).await;
        tokio::task::yield_now().await;
        assert_eq!(repo.0.load(Ordering::SeqCst), 0, "not before the migrations have run");
        ready.store(true, Ordering::Release);
        tokio::time::advance(std::time::Duration::from_secs(2)).await;
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;
        assert_eq!(repo.0.load(Ordering::SeqCst), 1, "once ready, at once");
        tokio::time::advance(std::time::Duration::from_secs(24 * 3600 + 1)).await;
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;
        assert_eq!(repo.0.load(Ordering::SeqCst), 2, "then a day later");
        handle.abort();
    }

    #[test]
    fn the_associated_data_names_the_row() {
        let id = Uuid::parse_str("3f2b0000-0000-4000-8000-000000000001").unwrap();
        assert_eq!(aad(id), "share.body.3f2b0000-0000-4000-8000-000000000001");
    }
}
