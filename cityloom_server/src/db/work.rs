//! The Postgres `WorkRepo`: bodies are sealed on the way in and opened on the way out, every call runs in one
//! transaction that first locks the user's row (so two batches for one account are applied one after the other) and
//! checks, inside that lock, that the account stores its work on the server.
use std::collections::BTreeMap;

use async_trait::async_trait;
use sqlx::Row;
use time::OffsetDateTime;
use uuid::Uuid;

use super::{PgRepo, corrupt, unavailable};
use crate::crypto::{self, Key};
use crate::work::{self, Ask, Id, Kind, Meta, Pulled, Pushed, PushedItem, Put, Record, Totals, Verdict, WorkError, WorkRepo};

fn db(e: sqlx::Error) -> WorkError {
    unavailable(e).into()
}

struct Locked {
    key: Key,
    rev: i64,
    epoch: Uuid,
}

fn id_of(row: &sqlx::postgres::PgRow) -> Result<Id, WorkError> {
    let kind: String = row.try_get("kind").map_err(db)?;
    let key: String = row.try_get("key").map_err(db)?;
    Ok(Id { kind: Kind::parse(&kind).ok_or(WorkError::Corrupt)?, key })
}

impl PgRepo {
    /// Locks the user's row and checks the account stores its work on the server.
    async fn lock_work(&self, conn: &mut sqlx::PgConnection, user: Uuid) -> Result<Locked, WorkError> {
        let row = sqlx::query("select wrapped_key, master_key_id, store_on_server, work_rev, work_epoch from users where id = $1 for update")
            .bind(user)
            .fetch_optional(&mut *conn)
            .await
            .map_err(db)?;
        let Some(row) = row else { return Err(WorkError::Gone) };
        let wrapped: Vec<u8> = row.try_get("wrapped_key").map_err(db)?;
        let master: i16 = row.try_get("master_key_id").map_err(db)?;
        let key = self.masters.unwrap(u8::try_from(master).map_err(corrupt)?, user, &wrapped).map_err(corrupt)?;
        let store: Vec<u8> = row.try_get("store_on_server").map_err(db)?;
        let store = crypto::open(&key, &crypto::aad("users", "store_on_server", &user.to_string()), &store).map_err(corrupt)?;
        if store.as_slice() != b"1" {
            return Err(WorkError::StorageOff);
        }
        Ok(Locked { key, rev: row.try_get("work_rev").map_err(db)?, epoch: row.try_get("work_epoch").map_err(db)? })
    }

    /// Deletes the tombstones older than 90 days (by when the server stored them) and raises the floor to the
    /// highest revision it removed. Returns the floor. When no tombstone is that old it only reads: a pull or a push
    /// does not write the user's row for it.
    async fn purge_tombstones(&self, conn: &mut sqlx::PgConnection, user: Uuid, now: OffsetDateTime) -> Result<i64, WorkError> {
        let cutoff = now - time::Duration::days(work::TOMBSTONE_DAYS);
        let stale: bool = sqlx::query_scalar("select exists (select 1 from work where user_id = $1 and deleted and changed_at < $2)")
            .bind(user)
            .bind(cutoff)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?;
        if !stale {
            return sqlx::query_scalar("select work_floor from users where id = $1").bind(user).fetch_one(&mut *conn).await.map_err(db);
        }
        sqlx::query_scalar(
            "with gone as (delete from work where user_id = $1 and deleted and changed_at < $2 returning rev) \
             update users set work_floor = greatest(work_floor, coalesce((select max(rev) from gone), 0)) where id = $1 returning work_floor",
        )
        .bind(user)
        .bind(cutoff)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)
    }

    /// Opens a row's body; a row that does not open is marked, logged by kind and key only, and served without a body.
    fn open_row(&self, locked: &Locked, user: Uuid, id: Id, row: &sqlx::postgres::PgRow) -> Result<Record, WorkError> {
        let rev: i64 = row.try_get("rev").map_err(db)?;
        let edited_at: OffsetDateTime = row.try_get("edited_at").map_err(db)?;
        let deleted: bool = row.try_get("deleted").map_err(db)?;
        let sealed: Option<Vec<u8>> = row.try_get("body").map_err(db)?;
        let (body, unreadable) = match sealed {
            None => (None, false),
            Some(sealed) => match crypto::open(&locked.key, &work::aad(&id, user), &sealed) {
                Ok(plain) => (Some(plain.to_vec()), false),
                Err(_) => {
                    tracing::error!(kind = id.kind.as_str(), key = %id.key, "a stored work body could not be opened");
                    (None, true)
                }
            },
        };
        Ok(Record { id, rev, edited_at, deleted, body, unreadable })
    }
}

#[async_trait]
impl WorkRepo for PgRepo {
    async fn pull(&self, user: Uuid, ask: Ask, now: OffsetDateTime) -> Result<Pulled, WorkError> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        let locked = self.lock_work(&mut tx, user).await?;
        // A device that read another copy of the account's work (storage was turned off and on again since) holds
        // revisions that mean nothing here: this comes before every other check.
        if ask.epoch.is_some_and(|epoch| epoch != locked.epoch) {
            return Err(WorkError::Reset);
        }
        let floor = self.purge_tombstones(&mut tx, user, now).await?;
        // A pull that began before a purged tombstone, or goes on above a counter that was reset, cannot trust what it
        // has. The pages after the first are judged by where the pull began (`start`), not by the revision each goes
        // on from: a tombstone purged between two pages must not make a long pull start over for ever.
        if (ask.start > 0 && ask.start < floor) || ask.since > locked.rev {
            return Err(WorkError::Resync);
        }
        let rows = sqlx::query(
            "select kind, key, rev, edited_at, deleted, body from work where user_id = $1 and rev > $2 and (not deleted or $4 > 0) order by rev limit $3",
        )
        .bind(user)
        .bind(ask.since)
        .bind((work::PAGE + 1) as i64)
        .bind(ask.start)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let more = rows.len() > work::PAGE;
        let mut records = Vec::with_capacity(rows.len().min(work::PAGE));
        for row in rows.iter().take(work::PAGE) {
            records.push(self.open_row(&locked, user, id_of(row)?, row)?);
        }
        let rev = if more { records.last().map_or(locked.rev, |r| r.rev) } else { locked.rev };
        tx.commit().await.map_err(db)?;
        Ok(Pulled { epoch: locked.epoch, rev, more, records })
    }

    async fn push(&self, user: Uuid, batch: Vec<Put>, now: OffsetDateTime) -> Result<Pushed, WorkError> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        let locked = self.lock_work(&mut tx, user).await?;
        self.purge_tombstones(&mut tx, user, now).await?;
        let (rows, bytes): (i64, i64) = sqlx::query_as("select count(*), coalesce(sum(octet_length(body)), 0)::bigint from work where user_id = $1")
            .bind(user)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let mut current = BTreeMap::new();
        for put in &batch {
            let row = sqlx::query(
                "select rev, edited_at, deleted, coalesce(octet_length(body), 0)::bigint as size from work where user_id = $1 and kind = $2 and key = $3",
            )
            .bind(user)
            .bind(put.id.kind.as_str())
            .bind(&put.id.key)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if let Some(row) = row {
                let size: i64 = row.try_get("size").map_err(db)?;
                current.insert(
                    put.id.clone(),
                    Meta {
                        rev: row.try_get("rev").map_err(db)?,
                        edited_at: row.try_get("edited_at").map_err(db)?,
                        deleted: row.try_get("deleted").map_err(db)?,
                        size: size as usize,
                    },
                );
            }
        }
        let totals = Totals { rows: rows as usize, bytes: bytes as usize };
        let plan = work::plan_batch(locked.rev, totals, &current, &batch, now)?;
        for (put, item) in batch.iter().zip(&plan.items) {
            if item.verdict != Verdict::Accepted {
                continue;
            }
            let sealed = put.body.as_ref().map(|plain| crypto::seal(&locked.key, &work::aad(&put.id, user), plain));
            sqlx::query(
                "insert into work (user_id, kind, key, rev, edited_at, changed_at, deleted, body) values ($1, $2, $3, $4, $5, $6, $7, $8) \
                 on conflict (user_id, kind, key) do update set rev = excluded.rev, edited_at = excluded.edited_at, changed_at = excluded.changed_at, \
                 deleted = excluded.deleted, body = excluded.body",
            )
            .bind(user)
            .bind(put.id.kind.as_str())
            .bind(&put.id.key)
            .bind(item.rev)
            .bind(item.at)
            .bind(now)
            .bind(put.deleted)
            .bind(sealed)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        if plan.rev != locked.rev {
            sqlx::query("update users set work_rev = $2 where id = $1").bind(user).bind(plan.rev).execute(&mut *tx).await.map_err(db)?;
        }
        let mut items = Vec::with_capacity(batch.len());
        for (put, item) in batch.iter().zip(&plan.items) {
            let record = if item.verdict == Verdict::Superseded {
                let row = sqlx::query("select kind, key, rev, edited_at, deleted, body from work where user_id = $1 and kind = $2 and key = $3")
                    .bind(user)
                    .bind(put.id.kind.as_str())
                    .bind(&put.id.key)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?;
                Some(self.open_row(&locked, user, put.id.clone(), &row)?)
            } else {
                None
            };
            items.push(PushedItem { id: put.id.clone(), verdict: item.verdict, rev: item.rev, record });
        }
        tx.commit().await.map_err(db)?;
        Ok(Pushed { rev: plan.rev, items })
    }

    async fn clear(&self, user: Uuid) -> Result<(), WorkError> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        self.lock_work(&mut tx, user).await?;
        sqlx::query("delete from work where user_id = $1").bind(user).execute(&mut *tx).await.map_err(db)?;
        // A new epoch with the counter back at 0: the other devices are told to reset (merge, removing nothing), not to
        // resync (which removes what the server no longer lists).
        sqlx::query("update users set work_rev = 0, work_floor = 0, work_epoch = $2 where id = $1")
            .bind(user)
            .bind(Uuid::new_v4())
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)
    }
}
