//! Key rotation: reseal the data keys under a new master key; recompute the lookups under a new lookup key.
//! Both are safe to interrupt: every row is readable after each committed step, and the old keys keep working
//! until the operator removes them.
use std::collections::HashMap;

use sqlx::Row;
use uuid::Uuid;

use super::{PgRepo, corrupt, unavailable};
use crate::accounts::RepoError;
use crate::crypto::{Key, LookupKey};

impl PgRepo {
    /// Reseals every data key not wrapped under the current master key, each user in its own transaction, and
    /// returns how many.
    pub async fn rewrap(&self) -> Result<u64, RepoError> {
        let current = self.masters.current_id() as i16;
        let ids: Vec<Uuid> =
            sqlx::query_scalar("select id from users where master_key_id <> $1").bind(current).fetch_all(&self.pool).await.map_err(unavailable)?;
        let mut done = 0;
        for id in ids {
            let mut tx = self.pool.begin().await.map_err(unavailable)?;
            let row = sqlx::query("select wrapped_key, master_key_id from users where id = $1 and master_key_id <> $2 for update")
                .bind(id)
                .bind(current)
                .fetch_optional(&mut *tx)
                .await
                .map_err(unavailable)?;
            let Some(row) = row else { continue };
            let wrapped: Vec<u8> = row.try_get("wrapped_key").map_err(unavailable)?;
            let old_id: i16 = row.try_get("master_key_id").map_err(unavailable)?;
            let key = self.masters.unwrap(u8::try_from(old_id).map_err(corrupt)?, id, &wrapped).map_err(corrupt)?;
            sqlx::query("update users set wrapped_key = $2, master_key_id = $3 where id = $1")
                .bind(id)
                .bind(self.masters.wrap(id, &key))
                .bind(current)
                .execute(&mut *tx)
                .await
                .map_err(unavailable)?;
            tx.commit().await.map_err(unavailable)?;
            done += 1;
        }
        Ok(done)
    }

    /// Recomputes every identity's lookup with this repository's lookup key (the new `LOOKUP_KEY`), from the sealed
    /// provider and subject, and reseals them under the new lookup's AAD, all in one transaction. A row already on
    /// the new hash is left alone; a row that matches neither the old nor the new key refuses the whole run
    /// (`Corrupt`) and changes nothing, which is how a wrong `old` key is found. Returns how many rows changed.
    ///
    /// Operator's order: stop the server, or at least keep it on the old `LOOKUP_KEY`; run `rehash` (with
    /// `OLD_LOOKUP_KEY` and the new `LOOKUP_KEY`); then start the server with the new key. Otherwise a sign-in
    /// during the run, made with a key that does not match the rows, does not find the account and creates a
    /// duplicate one.
    pub async fn rehash(&self, old: &LookupKey) -> Result<u64, RepoError> {
        let mut tx = self.pool.begin().await.map_err(unavailable)?;
        let rows = sqlx::query("select lookup, user_id, provider, subject, label from identities for update").fetch_all(&mut *tx).await.map_err(unavailable)?;
        let mut keys: HashMap<Uuid, Key> = HashMap::new();
        let mut done = 0;
        for row in rows {
            let lookup: Vec<u8> = row.try_get("lookup").map_err(unavailable)?;
            let user: Uuid = row.try_get("user_id").map_err(unavailable)?;
            let key = match keys.get(&user) {
                Some(k) => k.clone(),
                None => {
                    let k = self.data_key(user).await?.ok_or(RepoError::Corrupt)?;
                    keys.insert(user, k.clone());
                    k
                }
            };
            let old_id = hex::encode(&lookup);
            let provider = Self::open_text(&key, "identities", "provider", &old_id, &row.try_get::<Vec<u8>, _>("provider").map_err(unavailable)?)?;
            let sub = Self::open_text(&key, "identities", "subject", &old_id, &row.try_get::<Vec<u8>, _>("subject").map_err(unavailable)?)?;
            let new = self.lookup.lookup(&provider, &sub);
            if new[..] == lookup[..] {
                continue; // already on the new key (a mixed table, or a run repeated)
            }
            if old.lookup(&provider, &sub)[..] != lookup[..] {
                return Err(RepoError::Corrupt); // it matches neither key: the old key given is not the one these were made with
            }
            let new_id = hex::encode(new);
            let label = row
                .try_get::<Option<Vec<u8>>, _>("label")
                .map_err(unavailable)?
                .map(|l| Self::open_text(&key, "identities", "label", &old_id, &l))
                .transpose()?;
            sqlx::query("update identities set lookup = $1, provider = $2, subject = $3, label = $4 where lookup = $5")
                .bind(&new[..])
                .bind(Self::seal_text(&key, "identities", "provider", &new_id, &provider))
                .bind(Self::seal_text(&key, "identities", "subject", &new_id, &sub))
                .bind(label.map(|l| Self::seal_text(&key, "identities", "label", &new_id, &l)))
                .bind(&lookup)
                .execute(&mut *tx)
                .await
                .map_err(unavailable)?;
            done += 1;
        }
        tx.commit().await.map_err(unavailable)?;
        Ok(done)
    }
}
