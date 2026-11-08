//! The account rules over a [`Repo`], and an in-memory [`Repo`] for tests.
//! `Repo` speaks plaintext domain values; sealing is the Postgres implementation's business.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::crypto::LookupKey;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityInput {
    pub provider: String,
    pub sub: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserView {
    pub id: Uuid,
    pub display_name: String,
    pub store_on_server: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityView {
    /// Hex of the lookup hash.
    pub id: String,
    pub provider: String,
    pub label: Option<String>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRow {
    pub user_id: Uuid,
    pub last_seen_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RepoError {
    #[error("the store is unavailable")]
    Unavailable,
    #[error("a stored value could not be read")]
    Corrupt,
    #[error("that identity already exists")]
    Duplicate,
    #[error("the account no longer exists")]
    Gone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Removal {
    Removed,
    Last,
    NotFound,
}

#[async_trait]
pub trait Repo: Send + Sync {
    async fn find_identity(&self, provider: &str, sub: &str) -> Result<Option<Uuid>, RepoError>;
    /// `Duplicate` if the identity exists.
    async fn create_user(&self, display_name: &str, first: &IdentityInput) -> Result<Uuid, RepoError>;
    /// `Duplicate` if it exists, `Gone` if the user does not.
    async fn add_identity(&self, user: Uuid, identity: &IdentityInput) -> Result<(), RepoError>;
    async fn user(&self, id: Uuid) -> Result<Option<UserView>, RepoError>;
    /// Oldest first.
    async fn identities(&self, user: Uuid) -> Result<Vec<IdentityView>, RepoError>;
    /// Atomic: never removes the last one.
    async fn remove_identity(&self, user: Uuid, identity_id: &str) -> Result<Removal, RepoError>;
    /// `Gone` if the user does not exist.
    async fn set_storage(&self, user: Uuid, store: bool) -> Result<(), RepoError>;
    /// Identities and sessions go with the user.
    async fn delete_user(&self, user: Uuid) -> Result<(), RepoError>;
    async fn create_session(&self, token_hash: [u8; 32], user: Uuid, now: OffsetDateTime, expires_at: OffsetDateTime) -> Result<(), RepoError>;
    async fn session(&self, token_hash: &[u8; 32]) -> Result<Option<SessionRow>, RepoError>;
    async fn touch_session(&self, token_hash: &[u8; 32], now: OffsetDateTime, expires_at: OffsetDateTime) -> Result<(), RepoError>;
    async fn delete_session(&self, token_hash: &[u8; 32]) -> Result<(), RepoError>;
    async fn delete_expired_sessions(&self, now: OffsetDateTime) -> Result<u64, RepoError>;
}

#[derive(Debug, Clone)]
pub struct Claims {
    pub provider: String,
    pub provider_name: String,
    pub sub: String,
    pub name: Option<String>,
    pub preferred_username: Option<String>,
    pub first_time_name: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    New,
    SignedIn,
    Linked,
    AlreadyLinked,
    Conflict,
}

#[derive(Debug, PartialEq, Eq)]
pub enum UnlinkError {
    LastIdentity,
    NotFound,
    Repo(RepoError),
}
impl Outcome {
    pub fn slug(self) -> &'static str {
        match self {
            Outcome::New => "new",
            Outcome::SignedIn => "signed-in",
            Outcome::Linked => "linked",
            Outcome::AlreadyLinked => "already-linked",
            Outcome::Conflict => "conflict",
        }
    }
}

const NAME_MAX: usize = 100;

pub fn display_name(c: &Claims) -> String {
    let pick = |s: &Option<String>| s.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
    let name = pick(&c.name).or_else(|| pick(&c.preferred_username)).or_else(|| pick(&c.first_time_name)).unwrap_or_else(|| c.provider_name.clone());
    name.chars().take(NAME_MAX).collect()
}

fn input(c: &Claims) -> IdentityInput {
    IdentityInput { provider: c.provider.clone(), sub: c.sub.clone(), label: c.label.clone() }
}

pub async fn sign_in(repo: &dyn Repo, c: &Claims) -> Result<(Uuid, Outcome), RepoError> {
    for _ in 0..2 {
        if let Some(user) = repo.find_identity(&c.provider, &c.sub).await? {
            return Ok((user, Outcome::SignedIn));
        }
        match repo.create_user(&display_name(c), &input(c)).await {
            Ok(user) => return Ok((user, Outcome::New)),
            // Another request made the same identity between the look-up and the insert: look again.
            Err(RepoError::Duplicate) => continue,
            Err(e) => return Err(e),
        }
    }
    Err(RepoError::Unavailable)
}

pub async fn link(repo: &dyn Repo, user: Uuid, c: &Claims) -> Result<Outcome, RepoError> {
    if repo.user(user).await?.is_none() {
        return Err(RepoError::Gone);
    }
    match repo.find_identity(&c.provider, &c.sub).await? {
        Some(owner) if owner == user => return Ok(Outcome::AlreadyLinked),
        Some(_) => return Ok(Outcome::Conflict),
        None => {}
    }
    match repo.add_identity(user, &input(c)).await {
        Ok(()) => Ok(Outcome::Linked),
        // Taken by someone in the meantime.
        Err(RepoError::Duplicate) => Ok(match repo.find_identity(&c.provider, &c.sub).await? {
            Some(owner) if owner == user => Outcome::AlreadyLinked,
            _ => Outcome::Conflict,
        }),
        Err(e) => Err(e),
    }
}

pub async fn unlink(repo: &dyn Repo, user: Uuid, identity_id: &str) -> Result<(), UnlinkError> {
    match repo.remove_identity(user, identity_id).await.map_err(UnlinkError::Repo)? {
        Removal::Removed => Ok(()),
        Removal::Last => Err(UnlinkError::LastIdentity),
        Removal::NotFound => Err(UnlinkError::NotFound),
    }
}

pub async fn set_storage(repo: &dyn Repo, user: Uuid, store: bool) -> Result<(), RepoError> {
    repo.set_storage(user, store).await
}

pub async fn delete(repo: &dyn Repo, user: Uuid) -> Result<(), RepoError> {
    repo.delete_user(user).await
}

struct MemUser {
    view: UserView,
    identities: Vec<(String, IdentityView, [u8; 32])>,
}
#[derive(Default)]
struct MemState {
    users: HashMap<Uuid, MemUser>,
    owners: HashMap<[u8; 32], Uuid>,
    sessions: HashMap<[u8; 32], SessionRow>,
}

pub struct MemoryRepo {
    lookup: LookupKey,
    state: Mutex<MemState>,
}

impl MemoryRepo {
    pub fn new(lookup: LookupKey) -> MemoryRepo {
        MemoryRepo { lookup, state: Mutex::new(MemState::default()) }
    }
    fn view(&self, i: &IdentityInput) -> ([u8; 32], IdentityView) {
        let lookup = self.lookup.lookup(&i.provider, &i.sub);
        (lookup, IdentityView { id: hex::encode(lookup), provider: i.provider.clone(), label: i.label.clone(), created_at: OffsetDateTime::now_utc() })
    }
}

#[async_trait]
impl Repo for MemoryRepo {
    async fn find_identity(&self, provider: &str, sub: &str) -> Result<Option<Uuid>, RepoError> {
        Ok(self.state.lock().unwrap().owners.get(&self.lookup.lookup(provider, sub)).copied())
    }
    async fn create_user(&self, display_name: &str, first: &IdentityInput) -> Result<Uuid, RepoError> {
        let mut s = self.state.lock().unwrap();
        let (lookup, view) = self.view(first);
        if s.owners.contains_key(&lookup) {
            return Err(RepoError::Duplicate);
        }
        let id = Uuid::new_v4();
        s.owners.insert(lookup, id);
        s.users.insert(
            id,
            MemUser { view: UserView { id, display_name: display_name.into(), store_on_server: None }, identities: vec![(view.id.clone(), view, lookup)] },
        );
        Ok(id)
    }
    async fn add_identity(&self, user: Uuid, identity: &IdentityInput) -> Result<(), RepoError> {
        let mut s = self.state.lock().unwrap();
        let (lookup, view) = self.view(identity);
        if s.owners.contains_key(&lookup) {
            return Err(RepoError::Duplicate);
        }
        let u = s.users.get_mut(&user).ok_or(RepoError::Gone)?;
        u.identities.push((view.id.clone(), view, lookup));
        s.owners.insert(lookup, user);
        Ok(())
    }
    async fn user(&self, id: Uuid) -> Result<Option<UserView>, RepoError> {
        Ok(self.state.lock().unwrap().users.get(&id).map(|u| u.view.clone()))
    }
    async fn identities(&self, user: Uuid) -> Result<Vec<IdentityView>, RepoError> {
        Ok(self.state.lock().unwrap().users.get(&user).map(|u| u.identities.iter().map(|(_, v, _)| v.clone()).collect()).unwrap_or_default())
    }
    async fn remove_identity(&self, user: Uuid, identity_id: &str) -> Result<Removal, RepoError> {
        let mut s = self.state.lock().unwrap();
        let Some(u) = s.users.get_mut(&user) else { return Ok(Removal::NotFound) };
        let Some(at) = u.identities.iter().position(|(id, _, _)| id == identity_id) else { return Ok(Removal::NotFound) };
        if u.identities.len() == 1 {
            return Ok(Removal::Last);
        }
        let (_, _, lookup) = u.identities.remove(at);
        s.owners.remove(&lookup);
        Ok(Removal::Removed)
    }
    async fn set_storage(&self, user: Uuid, store: bool) -> Result<(), RepoError> {
        self.state.lock().unwrap().users.get_mut(&user).ok_or(RepoError::Gone)?.view.store_on_server = Some(store);
        Ok(())
    }
    async fn delete_user(&self, user: Uuid) -> Result<(), RepoError> {
        let mut s = self.state.lock().unwrap();
        if s.users.remove(&user).is_some() {
            s.owners.retain(|_, o| *o != user);
            s.sessions.retain(|_, row| row.user_id != user);
        }
        Ok(())
    }
    async fn create_session(&self, token_hash: [u8; 32], user: Uuid, now: OffsetDateTime, expires_at: OffsetDateTime) -> Result<(), RepoError> {
        let mut s = self.state.lock().unwrap();
        if !s.users.contains_key(&user) {
            return Err(RepoError::Gone);
        }
        s.sessions.insert(token_hash, SessionRow { user_id: user, last_seen_at: now, expires_at });
        Ok(())
    }
    async fn session(&self, token_hash: &[u8; 32]) -> Result<Option<SessionRow>, RepoError> {
        Ok(self.state.lock().unwrap().sessions.get(token_hash).cloned())
    }
    async fn touch_session(&self, token_hash: &[u8; 32], now: OffsetDateTime, expires_at: OffsetDateTime) -> Result<(), RepoError> {
        if let Some(row) = self.state.lock().unwrap().sessions.get_mut(token_hash) {
            row.last_seen_at = now;
            row.expires_at = expires_at;
        }
        Ok(())
    }
    async fn delete_session(&self, token_hash: &[u8; 32]) -> Result<(), RepoError> {
        self.state.lock().unwrap().sessions.remove(token_hash);
        Ok(())
    }
    async fn delete_expired_sessions(&self, now: OffsetDateTime) -> Result<u64, RepoError> {
        let mut s = self.state.lock().unwrap();
        let before = s.sessions.len();
        s.sessions.retain(|_, row| row.expires_at > now);
        Ok((before - s.sessions.len()) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> MemoryRepo {
        MemoryRepo::new(LookupKey::new(zeroize::Zeroizing::new([3; 32])))
    }

    fn claims(provider: &str, sub: &str) -> Claims {
        Claims {
            provider: provider.into(),
            provider_name: provider.to_uppercase(),
            sub: sub.into(),
            name: Some("Ada".into()),
            preferred_username: None,
            first_time_name: None,
            label: Some(format!("{sub}@example.org")),
        }
    }

    #[tokio::test]
    async fn a_new_identity_makes_a_user_and_a_returning_one_signs_in_the_same_user() {
        let r = repo();
        let (user, outcome) = sign_in(&r, &claims("google", "g1")).await.unwrap();
        assert_eq!(outcome, Outcome::New);
        let (again, outcome) = sign_in(&r, &claims("google", "g1")).await.unwrap();
        assert_eq!((again, outcome), (user, Outcome::SignedIn));
        let u = r.user(user).await.unwrap().unwrap();
        assert_eq!((u.display_name.as_str(), u.store_on_server), ("Ada", None));
        assert_eq!(r.identities(user).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn identities_are_provider_and_sub_never_the_label_or_email() {
        let r = repo();
        let (a, _) = sign_in(&r, &claims("google", "same")).await.unwrap();
        let (b, outcome) = sign_in(&r, &claims("apple", "same")).await.unwrap();
        assert_ne!(a, b);
        assert_eq!(outcome, Outcome::New);
        let mut other = claims("google", "different");
        other.label = claims("google", "same").label;
        let (c, outcome) = sign_in(&r, &other).await.unwrap();
        assert_eq!((outcome, c != a), (Outcome::New, true));
    }

    #[test]
    fn the_display_name_comes_from_name_then_username_then_the_first_time_name_then_the_provider() {
        let mut c = claims("apple", "x");
        c.name = Some("Name".into());
        c.preferred_username = Some("user".into());
        c.first_time_name = Some("First Last".into());
        assert_eq!(display_name(&c), "Name");
        c.name = Some("  ".into());
        assert_eq!(display_name(&c), "user");
        c.preferred_username = None;
        assert_eq!(display_name(&c), "First Last");
        c.first_time_name = None;
        c.provider_name = "Apple".into();
        assert_eq!(display_name(&c), "Apple");
        c.name = Some("x".repeat(500));
        assert_eq!(display_name(&c).chars().count(), 100);
    }

    #[tokio::test]
    async fn linking_adds_an_identity_or_says_already_linked_or_conflict_and_changes_nothing_else() {
        let r = repo();
        let (ada, _) = sign_in(&r, &claims("google", "g1")).await.unwrap();
        let (bob, _) = sign_in(&r, &claims("apple", "a1")).await.unwrap();
        assert_eq!(link(&r, ada, &claims("microsoft", "m1")).await.unwrap(), Outcome::Linked);
        assert_eq!(r.identities(ada).await.unwrap().len(), 2);
        assert_eq!(link(&r, ada, &claims("microsoft", "m1")).await.unwrap(), Outcome::AlreadyLinked);
        assert_eq!(link(&r, ada, &claims("apple", "a1")).await.unwrap(), Outcome::Conflict);
        assert_eq!(r.identities(ada).await.unwrap().len(), 2);
        assert_eq!(r.identities(bob).await.unwrap().len(), 1);
        // Either identity now signs in the one account.
        assert_eq!(sign_in(&r, &claims("microsoft", "m1")).await.unwrap(), (ada, Outcome::SignedIn));
    }

    #[tokio::test]
    async fn linking_to_a_user_deleted_meanwhile_is_gone_and_makes_nothing() {
        let r = repo();
        let (ada, _) = sign_in(&r, &claims("google", "g1")).await.unwrap();
        delete(&r, ada).await.unwrap();
        assert_eq!(link(&r, ada, &claims("apple", "a1")).await.unwrap_err(), RepoError::Gone);
        assert_eq!(r.find_identity("apple", "a1").await.unwrap(), None);
    }

    #[tokio::test]
    async fn unlinking_refuses_the_last_identity_and_an_identity_of_someone_else() {
        let r = repo();
        let (ada, _) = sign_in(&r, &claims("google", "g1")).await.unwrap();
        let (bob, _) = sign_in(&r, &claims("apple", "a1")).await.unwrap();
        let first = r.identities(ada).await.unwrap()[0].id.clone();
        assert_eq!(unlink(&r, ada, &first).await.unwrap_err(), UnlinkError::LastIdentity);
        link(&r, ada, &claims("microsoft", "m1")).await.unwrap();
        let bobs = r.identities(bob).await.unwrap()[0].id.clone();
        assert_eq!(unlink(&r, ada, &bobs).await.unwrap_err(), UnlinkError::NotFound);
        assert_eq!(unlink(&r, ada, "not-hex").await.unwrap_err(), UnlinkError::NotFound);
        unlink(&r, ada, &first).await.unwrap();
        assert_eq!(r.identities(ada).await.unwrap().len(), 1);
        assert_eq!(r.find_identity("google", "g1").await.unwrap(), None);
        let last = r.identities(ada).await.unwrap()[0].id.clone();
        assert_eq!(unlink(&r, ada, &last).await.unwrap_err(), UnlinkError::LastIdentity);
    }

    #[tokio::test]
    async fn the_storage_choice_is_none_until_chosen_and_can_change() {
        let r = repo();
        let (ada, _) = sign_in(&r, &claims("google", "g1")).await.unwrap();
        assert_eq!(r.user(ada).await.unwrap().unwrap().store_on_server, None);
        set_storage(&r, ada, true).await.unwrap();
        assert_eq!(r.user(ada).await.unwrap().unwrap().store_on_server, Some(true));
        set_storage(&r, ada, false).await.unwrap();
        assert_eq!(r.user(ada).await.unwrap().unwrap().store_on_server, Some(false));
        delete(&r, ada).await.unwrap();
        assert_eq!(set_storage(&r, ada, true).await.unwrap_err(), RepoError::Gone);
    }

    #[tokio::test]
    async fn deleting_removes_the_user_their_identities_and_their_sessions() {
        let r = repo();
        let (ada, _) = sign_in(&r, &claims("google", "g1")).await.unwrap();
        link(&r, ada, &claims("apple", "a1")).await.unwrap();
        let now = OffsetDateTime::UNIX_EPOCH;
        r.create_session([1; 32], ada, now, now + time::Duration::days(30)).await.unwrap();
        delete(&r, ada).await.unwrap();
        assert_eq!(r.user(ada).await.unwrap(), None);
        assert_eq!(r.find_identity("google", "g1").await.unwrap(), None);
        assert_eq!(r.find_identity("apple", "a1").await.unwrap(), None);
        assert_eq!(r.session(&[1; 32]).await.unwrap(), None);
        // Signing in again with the same identity is a brand new account.
        assert_eq!(sign_in(&r, &claims("google", "g1")).await.unwrap().1, Outcome::New);
    }

    #[tokio::test]
    async fn sessions_are_kept_touched_and_cleaned_up() {
        let r = repo();
        let (ada, _) = sign_in(&r, &claims("google", "g1")).await.unwrap();
        let t0 = OffsetDateTime::UNIX_EPOCH;
        r.create_session([1; 32], ada, t0, t0 + time::Duration::days(30)).await.unwrap();
        r.create_session([2; 32], ada, t0, t0 + time::Duration::days(1)).await.unwrap();
        let later = t0 + time::Duration::days(2);
        assert_eq!(r.delete_expired_sessions(later).await.unwrap(), 1);
        assert_eq!(r.session(&[2; 32]).await.unwrap(), None);
        r.touch_session(&[1; 32], later, later + time::Duration::days(30)).await.unwrap();
        let s = r.session(&[1; 32]).await.unwrap().unwrap();
        assert_eq!((s.user_id, s.last_seen_at), (ada, later));
        r.delete_session(&[1; 32]).await.unwrap();
        assert_eq!(r.session(&[1; 32]).await.unwrap(), None);
    }
}
