//! Sessions: a random token in a cookie, its SHA-256 in the database, thirty days sliding.
use std::sync::Arc;

use base64::Engine;
use cookie::{Cookie, SameSite};
use sha2::{Digest, Sha256};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::accounts::{Repo, RepoError};
use crate::clock::Clock;
use crate::crypto::random_bytes;

pub const COOKIE: &str = "__Host-cityloom-session";
pub const LIFETIME: Duration = Duration::days(30);
pub const REFRESH_AFTER: Duration = Duration::days(1);
const TOKEN_LEN: usize = 43;

pub fn new_token() -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(random_bytes::<32>())
}

pub fn hash(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

pub async fn start(repo: &dyn Repo, user: Uuid, now: OffsetDateTime) -> Result<String, RepoError> {
    let token = new_token();
    repo.create_session(hash(&token), user, now, now + LIFETIME).await?;
    Ok(token)
}

#[derive(Debug, PartialEq, Eq)]
pub enum Read {
    Unknown,
    Valid(Uuid),
    /// Valid, and the expiry was moved: the cookie must be set again.
    Refreshed(Uuid),
}

pub async fn read(repo: &dyn Repo, token: &str, now: OffsetDateTime) -> Result<Read, RepoError> {
    if token.len() != TOKEN_LEN {
        return Ok(Read::Unknown);
    }
    let h = hash(token);
    let Some(row) = repo.session(&h).await? else {
        return Ok(Read::Unknown);
    };
    if row.expires_at <= now {
        repo.delete_session(&h).await?;
        return Ok(Read::Unknown);
    }
    if now - row.last_seen_at > REFRESH_AFTER {
        repo.touch_session(&h, now, now + LIFETIME).await?;
        return Ok(Read::Refreshed(row.user_id));
    }
    Ok(Read::Valid(row.user_id))
}

pub fn cookie(token: String) -> Cookie<'static> {
    Cookie::build((COOKIE, token))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(cookie::time::Duration::seconds(LIFETIME.whole_seconds()))
        .build()
}

pub fn removal() -> Cookie<'static> {
    Cookie::build((COOKIE, "")).http_only(true).secure(true).same_site(SameSite::Lax).path("/").max_age(cookie::time::Duration::ZERO).build()
}

pub async fn end(repo: &dyn Repo, token: &str) -> Result<(), RepoError> {
    repo.delete_session(&hash(token)).await
}

pub async fn cleanup(repo: &dyn Repo, now: OffsetDateTime) -> Result<u64, RepoError> {
    repo.delete_expired_sessions(now).await
}

pub fn spawn_cleanup(repo: Arc<dyn Repo>, clock: Arc<dyn Clock>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut every_day = tokio::time::interval(std::time::Duration::from_secs(24 * 3600));
        loop {
            every_day.tick().await; // the first tick is at once: that is the start-up pass
            match cleanup(&*repo, clock.now()).await {
                Ok(n) => tracing::info!(deleted = n, "expired sessions removed"),
                Err(e) => tracing::warn!(%e, "expired sessions could not be removed"),
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::{IdentityInput, MemoryRepo};
    use crate::clock::FixedClock; // only the type: `advance` is its own method, and an unused `Clock` import fails `-D warnings`
    use crate::crypto::LookupKey;
    use std::sync::Arc;

    async fn repo_with_user() -> (Arc<MemoryRepo>, Uuid) {
        let r = Arc::new(MemoryRepo::new(LookupKey::new(zeroize::Zeroizing::new([1; 32]))));
        let u = r.create_user("Ada", &IdentityInput { provider: "p".into(), sub: "s".into(), label: None }).await.unwrap();
        (r, u)
    }

    fn t0() -> OffsetDateTime {
        OffsetDateTime::UNIX_EPOCH + time::Duration::days(20_000)
    }

    #[test]
    fn a_token_is_43_url_safe_characters_and_never_repeats() {
        let (a, b) = (new_token(), new_token());
        assert_eq!(a.len(), 43);
        assert!(a.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        assert_ne!(a, b);
    }

    #[test]
    fn only_a_sha256_of_the_token_is_kept() {
        let h = hash("abc");
        assert_eq!(hex::encode(h), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[tokio::test]
    async fn a_started_session_reads_back_its_user() {
        let (r, u) = repo_with_user().await;
        let token = start(&*r, u, t0()).await.unwrap();
        assert_eq!(read(&*r, &token, t0()).await.unwrap(), Read::Valid(u));
        assert_eq!(read(&*r, "nonsense", t0()).await.unwrap(), Read::Unknown);
        assert_eq!(read(&*r, &new_token(), t0()).await.unwrap(), Read::Unknown);
    }

    #[tokio::test]
    async fn use_within_a_day_changes_nothing_and_after_a_day_slides_the_expiry() {
        let (r, u) = repo_with_user().await;
        let token = start(&*r, u, t0()).await.unwrap();
        let row = r.session(&hash(&token)).await.unwrap().unwrap();
        assert_eq!(row.expires_at, t0() + LIFETIME);
        assert_eq!(read(&*r, &token, t0() + time::Duration::hours(23)).await.unwrap(), Read::Valid(u));
        assert_eq!(r.session(&hash(&token)).await.unwrap().unwrap(), row, "not touched within a day");
        let later = t0() + time::Duration::days(1) + time::Duration::minutes(1);
        assert_eq!(read(&*r, &token, later).await.unwrap(), Read::Refreshed(u));
        let row = r.session(&hash(&token)).await.unwrap().unwrap();
        assert_eq!((row.last_seen_at, row.expires_at), (later, later + LIFETIME));
    }

    #[tokio::test]
    async fn a_session_unused_for_thirty_days_is_expired_and_its_row_removed() {
        let (r, u) = repo_with_user().await;
        let token = start(&*r, u, t0()).await.unwrap();
        let after = t0() + LIFETIME + time::Duration::seconds(1);
        assert_eq!(read(&*r, &token, after).await.unwrap(), Read::Unknown);
        assert_eq!(r.session(&hash(&token)).await.unwrap(), None);
    }

    #[tokio::test]
    async fn ending_a_session_deletes_it() {
        let (r, u) = repo_with_user().await;
        let token = start(&*r, u, t0()).await.unwrap();
        end(&*r, &token).await.unwrap();
        assert_eq!(read(&*r, &token, t0()).await.unwrap(), Read::Unknown);
    }

    #[test]
    fn the_cookie_is_host_prefixed_http_only_secure_lax_for_thirty_days_with_no_domain() {
        let c = cookie("tok".into()).to_string();
        assert!(c.starts_with("__Host-cityloom-session=tok;"), "{c}");
        for part in ["HttpOnly", "Secure", "SameSite=Lax", "Path=/", "Max-Age=2592000"] {
            assert!(c.contains(part), "{c} lacks {part}");
        }
        assert!(!c.contains("Domain"), "{c}");
        let gone = removal().to_string();
        assert!(gone.contains("Max-Age=0") && gone.contains("Path=/") && gone.contains("Secure"), "{gone}");
    }

    #[tokio::test(start_paused = true)]
    async fn expired_rows_are_deleted_at_start_up_and_daily() {
        let (r, u) = repo_with_user().await;
        let clock = Arc::new(FixedClock::new(t0()));
        r.create_session([1; 32], u, t0() - time::Duration::days(40), t0() - time::Duration::days(10)).await.unwrap();
        r.create_session([2; 32], u, t0(), t0() + time::Duration::days(1) - time::Duration::hours(1)).await.unwrap();
        let handle = spawn_cleanup(r.clone(), clock.clone());
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;
        assert_eq!(r.session(&[1; 32]).await.unwrap(), None, "at start-up");
        assert!(r.session(&[2; 32]).await.unwrap().is_some());
        clock.advance(time::Duration::days(1));
        tokio::time::advance(std::time::Duration::from_secs(24 * 3600 + 1)).await;
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;
        assert_eq!(r.session(&[2; 32]).await.unwrap(), None, "a day later");
        handle.abort();
    }
}
