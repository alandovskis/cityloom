//! `repository` — the `sqlx` query functions against `stored_designs`
//! (`logical-components.md` LC-1): one function per operation, each issuing
//! exactly one compile-time-checked query (`performance-design.md` PD-1,
//! `security-design.md` SD-5). The only module in this crate depending on
//! `sqlx`.

use time::OffsetDateTime;

use crate::failure::StorageFailure;

/// This Unit's own record type (`entities.md` `StoredDesign`), deliberately
/// shaped so a Stage 2 account-owned design is the same row type with
/// `owner_account_id` set and `anonymous_design_id`/`expires_at` cleared,
/// not a second table.
#[derive(Clone, Debug, PartialEq)]
pub struct StoredDesign {
    pub stored_design_id: String,
    pub anonymous_design_id: Option<String>,
    pub owner_account_id: Option<String>,
    pub payload: String,
    pub created_at: OffsetDateTime,
    pub expires_at: Option<OffsetDateTime>,
}

/// The result of an insert attempt (`performance-design.md` PD-1): whether
/// the row was created, or a row with that `anonymous_design_id` already
/// existed (`0` rows affected under `ON CONFLICT ... DO NOTHING` —
/// `security-design.md` SD-6's stated non-error outcome, never a caught
/// unique-violation error).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InsertOutcome {
    Inserted,
    Conflict,
}

/// BR1.1/BR7.1 — insert an anonymous upload: exactly the client-supplied
/// `anonymous_design_id` (never re-derived or supplemented with any other
/// signal), the raw payload bytes, `created_at` and `expires_at` computed
/// by the caller (never relying on the column's own `DEFAULT` for a value
/// this Unit needs to return unchanged in the `UploadReceipt`).
pub async fn insert_anonymous(
    pool: &sqlx::PgPool,
    anonymous_design_id: &str,
    payload: &str,
    created_at: OffsetDateTime,
    expires_at: OffsetDateTime,
) -> Result<InsertOutcome, StorageFailure> {
    let result = sqlx::query!(
        r#"
        INSERT INTO stored_designs (anonymous_design_id, payload, created_at, expires_at)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (anonymous_design_id) DO NOTHING
        "#,
        anonymous_design_id,
        payload,
        created_at,
        expires_at,
    )
    .execute(pool)
    .await
    .map_err(|error| crate::failure::from_sqlx_error(&error))?;

    if result.rows_affected() == 1 {
        Ok(InsertOutcome::Inserted)
    } else {
        Ok(InsertOutcome::Conflict)
    }
}

/// BR2.1/BR3.1 — select a row by `anonymous_design_id`, excluding an
/// expired row even though it still physically exists: the row is "never
/// returned" once `expires_at` passes, the sweep is only the eventual
/// physical cleanup (`performance-design.md` PD-1). The same query that
/// authorizes is the query that reads the payload (`security-design.md`
/// SD-4) — there is no separate existence check.
pub async fn select_by_anonymous_id(
    pool: &sqlx::PgPool,
    anonymous_design_id: &str,
) -> Result<Option<StoredDesign>, StorageFailure> {
    let row = sqlx::query!(
        r#"
        SELECT stored_design_id, anonymous_design_id, owner_account_id,
               payload, created_at, expires_at
        FROM stored_designs
        WHERE anonymous_design_id = $1
          AND (expires_at IS NULL OR expires_at > now())
        "#,
        anonymous_design_id,
    )
    .fetch_optional(pool)
    .await
    .map_err(|error| crate::failure::from_sqlx_error(&error))?;

    Ok(row.map(|row| StoredDesign {
        stored_design_id: row.stored_design_id,
        anonymous_design_id: row.anonymous_design_id,
        owner_account_id: row.owner_account_id,
        payload: row.payload,
        created_at: row.created_at,
        expires_at: row.expires_at,
    }))
}

/// BR5.1 — delete by `anonymous_design_id`, idempotent: succeeds whether or
/// not a matching row existed. No account or authentication context is
/// required or checked beyond the identifier itself.
pub async fn delete_by_anonymous_id(
    pool: &sqlx::PgPool,
    anonymous_design_id: &str,
) -> Result<(), StorageFailure> {
    sqlx::query!(
        "DELETE FROM stored_designs WHERE anonymous_design_id = $1",
        anonymous_design_id,
    )
    .execute(pool)
    .await
    .map_err(|error| crate::failure::from_sqlx_error(&error))?;
    Ok(())
}

/// `performance-design.md` PD-3 — delete up to `batch_size` expired
/// anonymous rows in one short statement (never a table-scoped `DELETE`
/// with no `LIMIT`); never touches a row with `owner_account_id` set
/// (AC11.3.2), since the `WHERE` clause requires `anonymous_design_id IS
/// NOT NULL`. Returns the number of rows this batch deleted — every row
/// this predicate finds it deletes in the same statement, so "examined"
/// and "deleted" are the same count for one batch (`sweep` sums both
/// across the whole cycle for `observability-design.md` OD-3's aggregate
/// row).
pub async fn delete_expired_batch(
    pool: &sqlx::PgPool,
    batch_size: i64,
) -> Result<i64, StorageFailure> {
    let result = sqlx::query!(
        r#"
        DELETE FROM stored_designs
        WHERE stored_design_id IN (
            SELECT stored_design_id
            FROM stored_designs
            WHERE anonymous_design_id IS NOT NULL
              AND expires_at < now()
            LIMIT $1
        )
        "#,
        batch_size,
    )
    .execute(pool)
    .await
    .map_err(|error| crate::failure::from_sqlx_error(&error))?;

    Ok(result.rows_affected() as i64)
}

/// `observability-design.md` OD-2 — the current row counts split
/// anonymous vs. account-owned (`NFR7.4.2`'s `stored_rows_count` gauge),
/// sampled by the expiry sweep on its own cycle (`sweep.rs`), never
/// per-request. No request-derived value is involved, so there is nothing
/// to bind here beyond the two unconditional counts themselves
/// (`security-design.md` SD-5's bound-parameters rule has no free
/// variable to apply to).
pub async fn count_stored_rows_by_ownership(
    pool: &sqlx::PgPool,
) -> Result<(i64, i64), StorageFailure> {
    let anonymous = sqlx::query_scalar!(
        "SELECT count(*) FROM stored_designs WHERE anonymous_design_id IS NOT NULL"
    )
    .fetch_one(pool)
    .await
    .map_err(|error| crate::failure::from_sqlx_error(&error))?
    .unwrap_or(0);

    let account = sqlx::query_scalar!(
        "SELECT count(*) FROM stored_designs WHERE owner_account_id IS NOT NULL"
    )
    .fetch_one(pool)
    .await
    .map_err(|error| crate::failure::from_sqlx_error(&error))?
    .unwrap_or(0);

    Ok((anonymous, account))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_pool;

    fn future(days: i64) -> OffsetDateTime {
        OffsetDateTime::now_utc() + time::Duration::days(days)
    }

    fn past(days: i64) -> OffsetDateTime {
        OffsetDateTime::now_utc() - time::Duration::days(days)
    }

    // NFR6.3.1 / security-design.md SD-7 — the migrated `stored_designs`
    // table has exactly the columns `entities.md` names for `StoredDesign`
    // and no others: no identifying field beyond the opaque identifiers
    // this Unit already owns.
    #[test]
    fn the_migrated_table_has_exactly_the_columns_entities_md_names() {
        crate::test_support::runtime().block_on(async {
            let pool = test_pool().await;
            let mut columns: Vec<String> = sqlx::query_scalar!(
                "SELECT column_name FROM information_schema.columns \
                 WHERE table_name = 'stored_designs'",
            )
            .fetch_all(&pool)
            .await
            .unwrap()
            .into_iter()
            .flatten()
            .collect();
            columns.sort();

            let mut expected = vec![
                "stored_design_id".to_string(),
                "anonymous_design_id".to_string(),
                "owner_account_id".to_string(),
                "payload".to_string(),
                "created_at".to_string(),
                "expires_at".to_string(),
            ];
            expected.sort();

            assert_eq!(
                columns, expected,
                "stored_designs must have exactly entities.md's StoredDesign columns"
            );
        });
    }

    // Insert then select round-trips the payload unchanged.
    #[test]
    fn insert_then_select_round_trips_the_payload_unchanged() {
        crate::test_support::runtime().block_on(async {
            let pool = test_pool().await;
            let id = crate::test_support::unique_id("rt");
            let payload = r#"{"payloadVersion":1,"design":{}}"#;
            let outcome =
                insert_anonymous(&pool, &id, payload, OffsetDateTime::now_utc(), future(30))
                    .await
                    .unwrap();
            assert_eq!(outcome, InsertOutcome::Inserted);

            let found = select_by_anonymous_id(&pool, &id).await.unwrap().unwrap();
            assert_eq!(found.payload, payload);
            assert_eq!(found.anonymous_design_id.as_deref(), Some(id.as_str()));
            assert!(found.owner_account_id.is_none());
        });
    }

    // BR2.1 — a select for an expired row returns not-found even though
    // the row still physically exists.
    #[test]
    fn select_returns_not_found_for_an_expired_row_that_still_exists() {
        crate::test_support::runtime().block_on(async {
            // Excludes concurrent sweep-cycle tests, which delete expired rows
            // across the whole table — see `test_support::exclusive_table_access`.
            let _guard = crate::test_support::exclusive_table_access().await;
            let pool = test_pool().await;
            let id = crate::test_support::unique_id("expired");
            insert_anonymous(&pool, &id, "{}", past(31), past(1))
                .await
                .unwrap();

            let found = select_by_anonymous_id(&pool, &id).await.unwrap();
            assert!(found.is_none(), "an expired row must not be returned");

            // The row still physically exists — confirmed by an unfiltered
            // count, distinct from the filtered select above.
            let count = sqlx::query_scalar!(
                "SELECT count(*) FROM stored_designs WHERE anonymous_design_id = $1",
                id,
            )
            .fetch_one(&pool)
            .await
            .unwrap()
            .unwrap_or(0);
            assert_eq!(count, 1);
        });
    }

    // BR5.1 — delete of a non-existent id succeeds without error.
    #[test]
    fn delete_of_a_nonexistent_id_succeeds_without_error() {
        crate::test_support::runtime().block_on(async {
            let pool = test_pool().await;
            let id = crate::test_support::unique_id("never-existed");
            let result = delete_by_anonymous_id(&pool, &id).await;
            assert!(result.is_ok());
        });
    }

    // security-design.md SD-6 / performance-design.md PD-1 — a second
    // insert with a conflicting anonymous_design_id reports zero rows
    // affected, not an error.
    #[test]
    fn a_conflicting_insert_reports_zero_rows_affected_not_an_error() {
        crate::test_support::runtime().block_on(async {
            let pool = test_pool().await;
            let id = crate::test_support::unique_id("conflict");
            let first = insert_anonymous(
                &pool,
                &id,
                "{\"a\":1}",
                OffsetDateTime::now_utc(),
                future(30),
            )
            .await
            .unwrap();
            assert_eq!(first, InsertOutcome::Inserted);

            let second = insert_anonymous(
                &pool,
                &id,
                "{\"a\":2}",
                OffsetDateTime::now_utc(),
                future(30),
            )
            .await
            .unwrap();
            assert_eq!(second, InsertOutcome::Conflict);

            // The original row is untouched.
            let found = select_by_anonymous_id(&pool, &id).await.unwrap().unwrap();
            assert_eq!(found.payload, "{\"a\":1}");
        });
    }

    // NFR7.4.2/observability-design.md OD-2 — the stored_rows_count split
    // reflects both an anonymous row and an account-owned row, counted
    // independently. `exclusive_table_access` only serialises against
    // *other* whole-table-scanning tests (sweep-cycle tests and this one)
    // — every other test in this crate's suite inserts anonymous rows of
    // its own, by unique id, without taking that lock, and does so
    // concurrently under `cargo test`'s default parallelism. So the
    // anonymous count can only be asserted as a lower bound (this test's
    // own row, plus however many other tests happened to land
    // concurrently); nothing else in the suite ever inserts an
    // account-owned row, so that side of the split is exact.
    #[test]
    fn count_stored_rows_by_ownership_reflects_anonymous_and_account_rows() {
        crate::test_support::runtime().block_on(async {
            let _guard = crate::test_support::exclusive_table_access().await;
            let pool = test_pool().await;
            let (anon_before, acct_before) = count_stored_rows_by_ownership(&pool).await.unwrap();

            let id = crate::test_support::unique_id("count-anon");
            insert_anonymous(&pool, &id, "{}", OffsetDateTime::now_utc(), future(30))
                .await
                .unwrap();

            // No repository function creates an account-owned row yet
            // (that lands in Stage 2); insert one directly to exercise the
            // "account" side of the split this Unit's schema already
            // supports (`entities.md` `StoredDesign`).
            let account_id = crate::test_support::unique_id("count-acct");
            sqlx::query!(
                "INSERT INTO stored_designs (owner_account_id, payload) VALUES ($1, $2)",
                account_id,
                "{}",
            )
            .execute(&pool)
            .await
            .unwrap();

            let (anon_after, acct_after) = count_stored_rows_by_ownership(&pool).await.unwrap();
            assert!(
                anon_after > anon_before,
                "expected at least this test's own anonymous row to be counted (before {anon_before}, after {anon_after})"
            );
            assert_eq!(
                acct_after,
                acct_before + 1,
                "no other test in this suite inserts an account-owned row, so this count must be exact"
            );
        });
    }

    // security-design.md SD-5 — a request-derived value containing SQL
    // metacharacters is bound, never interpolated; no injection occurs.
    #[test]
    fn a_request_derived_value_with_sql_metacharacters_is_bound_not_interpolated() {
        crate::test_support::runtime().block_on(async {
            let pool = test_pool().await;
            let id = crate::test_support::unique_id("id'; DROP TABLE stored_designs; --");
            let outcome = insert_anonymous(&pool, &id, "{}", OffsetDateTime::now_utc(), future(30))
                .await
                .unwrap();
            assert_eq!(outcome, InsertOutcome::Inserted);
            // The table still exists and the row is retrievable by the exact
            // (bound) identifier string, metacharacters and all.
            let found = select_by_anonymous_id(&pool, &id).await.unwrap();
            assert!(found.is_some());
        });
    }
}
