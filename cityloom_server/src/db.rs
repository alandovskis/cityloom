//! The Postgres `Repo`. It stores only what `crypto` sealed; the plaintext lives in memory for the length of a call.
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::accounts::{IdentityInput, IdentityView, Removal, Repo, RepoError, SessionRow, UserView};
use crate::crypto::{self, Key, LookupKey, MasterKeys};

pub mod rotate;
mod work;

pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new().max_connections(10).connect(database_url).await?;
    migrate(&pool).await?;
    Ok(pool)
}

/// A pool that opens its first connection when it is first used, so a Postgres that is down at start-up is not fatal.
pub fn connect_lazy(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new().max_connections(10).acquire_timeout(Duration::from_secs(5)).connect_lazy(database_url)
}

pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

/// Runs the migrations until one run succeeds (waiting 1 s, then 2 s, and so on up to 30 s between tries), then
/// says the database is ready. A task of its own, so the server is listening, and answering 503, meanwhile.
pub fn spawn_migrations(pool: PgPool, ready: Arc<AtomicBool>) -> tokio::task::JoinHandle<()> {
    spawn_migrations_every(pool, ready, Duration::from_secs(1), Duration::from_secs(30))
}

pub fn spawn_migrations_every(pool: PgPool, ready: Arc<AtomicBool>, first: Duration, longest: Duration) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut wait = first;
        loop {
            match migrate(&pool).await {
                Ok(()) => {
                    ready.store(true, Ordering::Release);
                    tracing::info!("the database is ready");
                    return;
                }
                Err(e) => {
                    // The kind of failure only: the driver's message can carry the connection string.
                    tracing::warn!(kind = migrate_error_kind(&e), retry_in_secs = wait.as_secs_f32(), "the database is not ready");
                    tokio::time::sleep(wait).await;
                    wait = (wait * 2).min(longest);
                }
            }
        }
    })
}

pub struct PgRepo {
    pool: PgPool,
    masters: MasterKeys,
    lookup: LookupKey,
}

const NOT_CHOSEN: &[u8] = b"-";

/// A static name for the kind of a database error, for the log: never its message, which can carry a value.
fn error_kind(e: &sqlx::Error) -> &'static str {
    match e {
        sqlx::Error::Configuration(_) => "Configuration",
        sqlx::Error::Database(_) => "Database",
        sqlx::Error::Io(_) => "Io",
        sqlx::Error::Tls(_) => "Tls",
        sqlx::Error::Protocol(_) => "Protocol",
        sqlx::Error::RowNotFound => "RowNotFound",
        sqlx::Error::ColumnDecode { .. } | sqlx::Error::Decode(_) => "Decode",
        sqlx::Error::Encode(_) => "Encode",
        sqlx::Error::PoolTimedOut => "PoolTimedOut",
        sqlx::Error::PoolClosed => "PoolClosed",
        sqlx::Error::WorkerCrashed => "WorkerCrashed",
        sqlx::Error::Migrate(_) => "Migrate",
        _ => "Other",
    }
}

fn migrate_error_kind(e: &sqlx::migrate::MigrateError) -> &'static str {
    use sqlx::migrate::MigrateError as M;
    match e {
        M::Execute(e) | M::ExecuteMigration(e, _) => error_kind(e),
        M::VersionMissing(_) | M::VersionMismatch(_) | M::VersionNotPresent(_) | M::VersionTooOld(..) | M::VersionTooNew(..) => "MigrationVersion",
        M::Dirty(_) => "MigrationDirty",
        _ => "Migration",
    }
}

/// What the log may say about a database error: never its message, which can carry a value.
fn unavailable(e: sqlx::Error) -> RepoError {
    let kind = match &e {
        sqlx::Error::Database(d) => d.code().map(|c| c.to_string()).unwrap_or_else(|| "database".into()),
        other => error_kind(other).into(),
    };
    tracing::error!(kind, "database error");
    RepoError::Unavailable
}

fn is_code(e: &sqlx::Error, code: &str) -> bool {
    matches!(e, sqlx::Error::Database(d) if d.code().as_deref() == Some(code))
}

fn corrupt<T>(_: T) -> RepoError {
    RepoError::Corrupt
}

impl PgRepo {
    pub fn new(pool: PgPool, masters: MasterKeys, lookup: LookupKey) -> PgRepo {
        PgRepo { pool, masters, lookup }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    async fn data_key(&self, user: Uuid) -> Result<Option<Key>, RepoError> {
        let row = sqlx::query("select wrapped_key, master_key_id from users where id = $1").bind(user).fetch_optional(&self.pool).await.map_err(unavailable)?;
        let Some(row) = row else { return Ok(None) };
        let wrapped: Vec<u8> = row.try_get("wrapped_key").map_err(unavailable)?;
        let id: i16 = row.try_get("master_key_id").map_err(unavailable)?;
        let id = u8::try_from(id).map_err(corrupt)?;
        self.masters.unwrap(id, user, &wrapped).map(Some).map_err(corrupt)
    }

    fn seal_text(key: &Key, table: &str, column: &str, id: &str, text: &str) -> Vec<u8> {
        crypto::seal(key, &crypto::aad(table, column, id), text.as_bytes())
    }

    fn open_text(key: &Key, table: &str, column: &str, id: &str, sealed: &[u8]) -> Result<String, RepoError> {
        let bytes = crypto::open(key, &crypto::aad(table, column, id), sealed).map_err(corrupt)?;
        String::from_utf8(bytes.to_vec()).map_err(corrupt)
    }

    async fn insert_identity(&self, tx: &mut sqlx::PgConnection, key: &Key, user: Uuid, i: &IdentityInput) -> Result<(), sqlx::Error> {
        let lookup = self.lookup.lookup(&i.provider, &i.sub);
        let id = hex::encode(lookup);
        sqlx::query("insert into identities (lookup, user_id, provider, subject, label) values ($1, $2, $3, $4, $5)")
            .bind(&lookup[..])
            .bind(user)
            .bind(Self::seal_text(key, "identities", "provider", &id, &i.provider))
            .bind(Self::seal_text(key, "identities", "subject", &id, &i.sub))
            .bind(i.label.as_deref().map(|l| Self::seal_text(key, "identities", "label", &id, l)))
            .execute(tx)
            .await
            .map(|_| ())
    }
}

fn storage_bytes(store: Option<bool>) -> &'static [u8] {
    match store {
        Some(true) => b"1",
        Some(false) => b"0",
        None => NOT_CHOSEN,
    }
}

#[async_trait]
impl Repo for PgRepo {
    async fn find_identity(&self, provider: &str, sub: &str) -> Result<Option<Uuid>, RepoError> {
        let lookup = self.lookup.lookup(provider, sub);
        sqlx::query_scalar("select user_id from identities where lookup = $1").bind(&lookup[..]).fetch_optional(&self.pool).await.map_err(unavailable)
    }

    async fn create_user(&self, display_name: &str, first: &IdentityInput) -> Result<Uuid, RepoError> {
        let id = Uuid::new_v4();
        let key = Key::random();
        let mut tx = self.pool.begin().await.map_err(unavailable)?;
        sqlx::query("insert into users (id, wrapped_key, master_key_id, display_name, store_on_server) values ($1, $2, $3, $4, $5)")
            .bind(id)
            .bind(self.masters.wrap(id, &key))
            .bind(self.masters.current_id() as i16)
            .bind(Self::seal_text(&key, "users", "display_name", &id.to_string(), display_name))
            .bind(crypto::seal(&key, &crypto::aad("users", "store_on_server", &id.to_string()), storage_bytes(None)))
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
        match self.insert_identity(&mut tx, &key, id, first).await {
            Ok(()) => {}
            Err(e) if is_code(&e, "23505") => return Err(RepoError::Duplicate),
            Err(e) => return Err(unavailable(e)),
        }
        tx.commit().await.map_err(unavailable)?;
        Ok(id)
    }

    async fn add_identity(&self, user: Uuid, identity: &IdentityInput) -> Result<(), RepoError> {
        let key = self.data_key(user).await?.ok_or(RepoError::Gone)?;
        let mut conn = self.pool.acquire().await.map_err(unavailable)?;
        match self.insert_identity(&mut conn, &key, user, identity).await {
            Ok(()) => Ok(()),
            Err(e) if is_code(&e, "23505") => Err(RepoError::Duplicate),
            Err(e) if is_code(&e, "23503") => Err(RepoError::Gone),
            Err(e) => Err(unavailable(e)),
        }
    }

    async fn user(&self, id: Uuid) -> Result<Option<UserView>, RepoError> {
        let Some(key) = self.data_key(id).await? else { return Ok(None) };
        let row =
            sqlx::query("select display_name, store_on_server from users where id = $1").bind(id).fetch_optional(&self.pool).await.map_err(unavailable)?;
        let Some(row) = row else { return Ok(None) };
        let text_id = id.to_string();
        let name: Vec<u8> = row.try_get("display_name").map_err(unavailable)?;
        let store: Vec<u8> = row.try_get("store_on_server").map_err(unavailable)?;
        let store = crypto::open(&key, &crypto::aad("users", "store_on_server", &text_id), &store).map_err(corrupt)?;
        let store_on_server = match store.as_slice() {
            b"1" => Some(true),
            b"0" => Some(false),
            b"-" => None,
            _ => return Err(RepoError::Corrupt),
        };
        Ok(Some(UserView { id, display_name: Self::open_text(&key, "users", "display_name", &text_id, &name)?, store_on_server }))
    }

    async fn identities(&self, user: Uuid) -> Result<Vec<IdentityView>, RepoError> {
        let Some(key) = self.data_key(user).await? else { return Ok(Vec::new()) };
        let rows = sqlx::query("select lookup, provider, label, created_at from identities where user_id = $1 order by created_at, lookup")
            .bind(user)
            .fetch_all(&self.pool)
            .await
            .map_err(unavailable)?;
        rows.into_iter()
            .map(|row| {
                let lookup: Vec<u8> = row.try_get("lookup").map_err(unavailable)?;
                let id = hex::encode(&lookup);
                let provider: Vec<u8> = row.try_get("provider").map_err(unavailable)?;
                let label: Option<Vec<u8>> = row.try_get("label").map_err(unavailable)?;
                Ok(IdentityView {
                    provider: Self::open_text(&key, "identities", "provider", &id, &provider)?,
                    label: label.map(|l| Self::open_text(&key, "identities", "label", &id, &l)).transpose()?,
                    created_at: row.try_get("created_at").map_err(unavailable)?,
                    id,
                })
            })
            .collect()
    }

    async fn remove_identity(&self, user: Uuid, identity_id: &str) -> Result<Removal, RepoError> {
        let Ok(lookup) = hex::decode(identity_id) else { return Ok(Removal::NotFound) };
        let mut tx = self.pool.begin().await.map_err(unavailable)?;
        // Locks this user's identities, so two removals cannot both leave none.
        let owned: Vec<Vec<u8>> =
            sqlx::query_scalar("select lookup from identities where user_id = $1 for update").bind(user).fetch_all(&mut *tx).await.map_err(unavailable)?;
        if !owned.contains(&lookup) {
            return Ok(Removal::NotFound);
        }
        if owned.len() == 1 {
            return Ok(Removal::Last);
        }
        sqlx::query("delete from identities where lookup = $1 and user_id = $2").bind(&lookup).bind(user).execute(&mut *tx).await.map_err(unavailable)?;
        tx.commit().await.map_err(unavailable)?;
        Ok(Removal::Removed)
    }

    async fn set_storage(&self, user: Uuid, store: bool) -> Result<(), RepoError> {
        let key = self.data_key(user).await?.ok_or(RepoError::Gone)?;
        let sealed = crypto::seal(&key, &crypto::aad("users", "store_on_server", &user.to_string()), storage_bytes(Some(store)));
        // One transaction: turning storage off deletes the server copy with the choice, so a device that is offline
        // cannot leave a copy behind (its queued push finds the choice off and is refused).
        let mut tx = self.pool.begin().await.map_err(unavailable)?;
        let Some(before) = sqlx::query_scalar::<_, Vec<u8>>("select store_on_server from users where id = $1 for update")
            .bind(user)
            .fetch_optional(&mut *tx)
            .await
            .map_err(unavailable)?
        else {
            return Err(RepoError::Gone);
        };
        let was_on = crypto::open(&key, &crypto::aad("users", "store_on_server", &user.to_string()), &before).map_err(corrupt)?.as_slice() == b"1";
        sqlx::query("update users set store_on_server = $2 where id = $1").bind(user).bind(sealed).execute(&mut *tx).await.map_err(unavailable)?;
        if store {
            if was_on {
                // Already on: the copy is the same one, and the devices keep their revisions.
                return tx.commit().await.map_err(unavailable);
            }
            // A new epoch each time storage is turned on: a device that missed an off and on again holds revisions that
            // mean nothing to the new copy, and is told to reset (merge) instead of being served an empty page.
            sqlx::query("update users set work_epoch = $2 where id = $1").bind(user).bind(Uuid::new_v4()).execute(&mut *tx).await.map_err(unavailable)?;
        } else {
            sqlx::query("delete from work where user_id = $1").bind(user).execute(&mut *tx).await.map_err(unavailable)?;
            sqlx::query("update users set work_rev = 0, work_floor = 0 where id = $1").bind(user).execute(&mut *tx).await.map_err(unavailable)?;
        }
        tx.commit().await.map_err(unavailable)
    }

    async fn delete_user(&self, user: Uuid) -> Result<(), RepoError> {
        sqlx::query("delete from users where id = $1").bind(user).execute(&self.pool).await.map(|_| ()).map_err(unavailable)
    }

    async fn create_session(&self, token_hash: [u8; 32], user: Uuid, now: OffsetDateTime, expires_at: OffsetDateTime) -> Result<(), RepoError> {
        sqlx::query("insert into sessions (token_hash, user_id, created_at, last_seen_at, expires_at) values ($1, $2, $3, $3, $4)")
            .bind(&token_hash[..])
            .bind(user)
            .bind(now)
            .bind(expires_at)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|e| if is_code(&e, "23503") { RepoError::Gone } else { unavailable(e) })
    }

    async fn session(&self, token_hash: &[u8; 32]) -> Result<Option<SessionRow>, RepoError> {
        let row = sqlx::query("select user_id, last_seen_at, expires_at from sessions where token_hash = $1")
            .bind(&token_hash[..])
            .fetch_optional(&self.pool)
            .await
            .map_err(unavailable)?;
        row.map(|r| {
            Ok(SessionRow {
                user_id: r.try_get("user_id").map_err(unavailable)?,
                last_seen_at: r.try_get("last_seen_at").map_err(unavailable)?,
                expires_at: r.try_get("expires_at").map_err(unavailable)?,
            })
        })
        .transpose()
    }

    async fn touch_session(&self, token_hash: &[u8; 32], now: OffsetDateTime, expires_at: OffsetDateTime) -> Result<(), RepoError> {
        sqlx::query("update sessions set last_seen_at = $2, expires_at = $3 where token_hash = $1")
            .bind(&token_hash[..])
            .bind(now)
            .bind(expires_at)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(unavailable)
    }

    async fn delete_session(&self, token_hash: &[u8; 32]) -> Result<(), RepoError> {
        sqlx::query("delete from sessions where token_hash = $1").bind(&token_hash[..]).execute(&self.pool).await.map(|_| ()).map_err(unavailable)
    }

    async fn delete_expired_sessions(&self, now: OffsetDateTime) -> Result<u64, RepoError> {
        sqlx::query("delete from sessions where expires_at <= $1").bind(now).execute(&self.pool).await.map(|d| d.rows_affected()).map_err(unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_are_logged_by_a_static_name() {
        assert_eq!(error_kind(&sqlx::Error::PoolTimedOut), "PoolTimedOut");
        assert_eq!(error_kind(&sqlx::Error::Io(std::io::Error::other("secret"))), "Io");
        assert_eq!(error_kind(&sqlx::Error::RowNotFound), "RowNotFound");
        assert_eq!(migrate_error_kind(&sqlx::migrate::MigrateError::Execute(sqlx::Error::PoolTimedOut)), "PoolTimedOut");
        assert_eq!(migrate_error_kind(&sqlx::migrate::MigrateError::Dirty(3)), "MigrationDirty");
    }
}
