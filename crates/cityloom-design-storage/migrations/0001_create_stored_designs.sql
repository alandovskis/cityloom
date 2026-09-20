-- `entities.md` `StoredDesign`: an anonymous upload keys on
-- `anonymous_design_id`; a Stage 2 account-owned design keys on
-- `owner_account_id` instead, never both. `expires_at` applies only to an
-- anonymous row (BR2.1). `stored_design_id` is server-minted and distinct
-- from the client-minted `anonymous_design_id` (ADR-006).
--
-- Exactly the columns `entities.md` names for `StoredDesign` and no others
-- (NFR6.3.1 — no identifying field beyond the opaque identifier itself,
-- `security-design.md` SD-7).
CREATE TABLE stored_designs (
    stored_design_id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::text,
    anonymous_design_id TEXT,
    owner_account_id TEXT,
    payload TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ
);

-- `infrastructure-specification.md` ID-15 — a unique index on
-- `anonymous_design_id`, additive-only. Postgres treats NULLs as distinct
-- under a unique index, so multiple account-owned rows (which leave this
-- column NULL) are unaffected.
CREATE UNIQUE INDEX stored_designs_anonymous_design_id_key
    ON stored_designs (anonymous_design_id);

-- `performance-design.md` PD-3 — the expiry sweep scans exactly this
-- predicate (`anonymous_design_id IS NOT NULL AND expires_at < now()`) in
-- bounded batches; an index on `expires_at` keeps each batch's scan cheap
-- as the table grows.
CREATE INDEX stored_designs_expires_at_idx
    ON stored_designs (expires_at)
    WHERE anonymous_design_id IS NOT NULL;
