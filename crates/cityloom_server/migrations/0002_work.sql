-- The work a person stores on the server: one row per place (street, junction), area rule or the chosen area.
-- The body is sealed (AES-256-GCM) under the person's data key; the kind, key and times are not (the server matches
-- and sorts on them and they say nothing about the person by themselves).
alter table users add column work_rev bigint not null default 0;     -- the account's revision counter
alter table users add column work_floor bigint not null default 0;   -- the highest revision of a tombstone ever purged
alter table users add column work_epoch uuid not null default gen_random_uuid(); -- new each time storage is turned on
create table work (
  user_id    uuid not null references users (id) on delete cascade,
  kind       text not null check (kind in ('street', 'junction', 'rule', 'area')),
  key        text not null check (char_length(key) between 1 and 200),
  rev        bigint not null,                -- the account's revision when this row last changed
  edited_at  timestamptz not null,           -- when the device made the edit (capped at the server's clock)
  changed_at timestamptz not null,           -- when the row last changed here: tombstones expire from this
  deleted    boolean not null,               -- a tombstone, so other devices learn of a removal
  body       bytea,                          -- sealed record JSON; null when deleted
  primary key (user_id, kind, key),
  check ((deleted and body is null) or (not deleted and body is not null))
);
create index work_user_rev on work (user_id, rev);
