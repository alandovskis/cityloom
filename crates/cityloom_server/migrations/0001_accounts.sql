-- One row per person. Every value that says something about them is sealed (AES-256-GCM) under their own data key.
create table users (
  id               uuid primary key,
  wrapped_key      bytea not null,                -- the person's data key, sealed under the master key named by master_key_id
  master_key_id    smallint not null,             -- which of DATA_KEYS wrapped it
  display_name     bytea not null,                -- sealed
  store_on_server  bytea not null,                -- sealed: "1", "0", or "-" while not chosen
  created_at       timestamptz not null default now()
);
-- One row per sign-in method of a person.
create table identities (
  lookup     bytea primary key,                   -- HMAC-SHA256(LOOKUP_KEY, provider || 0x00 || sub): finds the row without storing the sub
  user_id    uuid not null references users (id) on delete cascade,
  provider   bytea not null,                      -- sealed
  subject    bytea not null,                      -- sealed
  label      bytea,                               -- sealed, optional: what the person recognises the method by
  created_at timestamptz not null default now()
);
-- One row per signed-in browser.
create table sessions (
  token_hash   bytea primary key,                 -- SHA-256 of the cookie's token; the token itself is never stored
  user_id      uuid not null references users (id) on delete cascade,
  created_at   timestamptz not null default now(),
  last_seen_at timestamptz not null default now(),
  expires_at   timestamptz not null
);
create index identities_user_id on identities (user_id);
create index sessions_expires_at on sessions (expires_at);
create index sessions_user_id on sessions (user_id);
