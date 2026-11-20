//! The Postgres `ShareRepo`: a body is sealed on the way in under the key its token derives (`share::Token::body_key`)
//! and opened on the way out; the token itself is never stored, only its lookup hash.
use async_trait::async_trait;
use sqlx::Row;
use time::OffsetDateTime;
use uuid::Uuid;

use super::{PgRepo, unavailable};
use crate::crypto;
use crate::share::{self, Created, ShareError, ShareRepo, Standing, Token};

fn db(e: sqlx::Error) -> ShareError {
    unavailable(e).into()
}

/// The foreign key on `user_id`: the account is no more.
fn is_gone(e: &sqlx::Error) -> bool {
    super::is_code(e, "23503")
}

#[async_trait]
impl ShareRepo for PgRepo {
    async fn create(&self, user: Uuid, token: &Token, body: &[u8], now: OffsetDateTime) -> Result<Created, ShareError> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        // Locks the account's row, so two shares made at once cannot both pass the cap.
        let exists: Option<Uuid> = sqlx::query_scalar("select id from users where id = $1 for update").bind(user).fetch_optional(&mut *tx).await.map_err(db)?;
        if exists.is_none() {
            return Err(ShareError::Gone);
        }
        let live: i64 = sqlx::query_scalar("select count(*) from shares where user_id = $1 and body is not null and expires_at > $2")
            .bind(user)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        if live as usize >= share::MAX_LIVE_SHARES {
            return Err(ShareError::Full);
        }
        let id = Uuid::new_v4();
        let expires_at = share::expires_at(now);
        let sealed = crypto::seal(&token.body_key(), &share::aad(id), body);
        sqlx::query("insert into shares (id, lookup, user_id, created_at, expires_at, body) values ($1, $2, $3, $4, $5, $6)")
            .bind(id)
            .bind(&token.lookup()[..])
            .bind(user)
            .bind(now)
            .bind(expires_at)
            .bind(sealed)
            .execute(&mut *tx)
            .await
            .map_err(|e| if is_gone(&e) { ShareError::Gone } else { db(e) })?;
        tx.commit().await.map_err(db)?;
        Ok(Created { token: token.clone(), expires_at })
    }

    async fn open(&self, token: &Token, now: OffsetDateTime) -> Result<Vec<u8>, ShareError> {
        // The body is read only for a live share: an expired or deleted one is answered from its metadata alone.
        let row = sqlx::query(
            "select id, expires_at, deleted, body is not null as kept, case when not deleted and expires_at > $2 then body end as body \
             from shares where lookup = $1",
        )
        .bind(&token.lookup()[..])
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map_err(db)?;
        let Some(row) = row else { return Err(ShareError::NotFound) };
        let id: Uuid = row.try_get("id").map_err(db)?;
        let expires_at: OffsetDateTime = row.try_get("expires_at").map_err(db)?;
        let deleted: bool = row.try_get("deleted").map_err(db)?;
        match share::standing(expires_at, deleted, now) {
            Standing::Deleted => return Err(ShareError::Deleted),
            Standing::Expired => {
                // Not served once expired, whether or not the purge has come by: and dropped now, at the latest.
                let kept: bool = row.try_get("kept").map_err(db)?;
                if kept {
                    sqlx::query("update shares set body = null where id = $1 and body is not null").bind(id).execute(&self.pool).await.map_err(db)?;
                }
                return Err(ShareError::Expired);
            }
            Standing::Live => {}
        }
        let sealed: Option<Vec<u8>> = row.try_get("body").map_err(db)?;
        let sealed = sealed.ok_or(ShareError::Corrupt)?;
        match crypto::open(&token.body_key(), &share::aad(id), &sealed) {
            Ok(plain) => Ok(plain.to_vec()),
            Err(_) => {
                tracing::error!(share = %id, "a stored share body could not be opened");
                Err(ShareError::Corrupt)
            }
        }
    }

    async fn delete(&self, user: Uuid, token: &Token, now: OffsetDateTime) -> Result<(), ShareError> {
        let owner: Option<Uuid> =
            sqlx::query_scalar("select user_id from shares where lookup = $1").bind(&token.lookup()[..]).fetch_optional(&self.pool).await.map_err(db)?;
        match owner {
            None => Err(ShareError::NotFound),
            Some(owner) if owner != user => Err(ShareError::NotYours),
            Some(_) => {
                // Only a live share becomes deleted: one that expired first stays expired (its body goes either way),
                // and one deleted already is left as it is.
                sqlx::query("update shares set deleted = expires_at > $3, body = null where lookup = $1 and user_id = $2 and not deleted")
                    .bind(&token.lookup()[..])
                    .bind(user)
                    .bind(now)
                    .execute(&self.pool)
                    .await
                    .map_err(db)?;
                Ok(())
            }
        }
    }

    async fn purge(&self, now: OffsetDateTime) -> Result<u64, ShareError> {
        let dropped =
            sqlx::query("update shares set body = null where body is not null and expires_at <= $1").bind(now).execute(&self.pool).await.map_err(db)?;
        let forgotten = sqlx::query("delete from shares where expires_at < $1")
            .bind(now - time::Duration::days(share::MARKER_DAYS))
            .execute(&self.pool)
            .await
            .map_err(db)?;
        Ok(dropped.rows_affected() + forgotten.rows_affected())
    }
}
