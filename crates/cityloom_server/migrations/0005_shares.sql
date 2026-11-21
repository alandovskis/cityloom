-- A shared design: a snapshot of a project behind an unguessable link (`share.rs`). The token is never stored: a row
-- is found by its hash (`lookup`), and the body is sealed (AES-256-GCM, AD `share.body.<id>`) under a key derived
-- from the token, so nothing here opens without the link. A share expires 90 days after it is made; its body is then
-- dropped, as when its creator deletes it, and the row stays as a marker (a year past the expiry) so the link says
-- the share expired or was deleted rather than that there never was one. The account's deletion removes its rows.
create table shares (
  id         uuid primary key,
  lookup     bytea not null unique check (octet_length(lookup) = 32),
  user_id    uuid not null references users (id) on delete cascade,
  created_at timestamptz not null,
  expires_at timestamptz not null,
  deleted    boolean not null default false,  -- deleted by its creator before it expired
  body       bytea,                           -- sealed share JSON; null once deleted, or expired and purged
  check (not deleted or body is null)
);
create index shares_user on shares (user_id);
create index shares_expires on shares (expires_at);
