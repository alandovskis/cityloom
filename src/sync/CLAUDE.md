# Sync slice (`src/sync/`)

Loaded when working in this folder; the root `CLAUDE.md` has the project-wide rules.

Keeps a signed-in person's work on the server when they chose "Store my work on the server"; browser storage stays the working copy. Server side: `cityloom_server`'s `work` table and `/api/work`.

## Modules

- `model.rs` (no I/O): `cityloom-sync` document, `mark`, `decide`, `next_batch`.
- `local.rs`: stores' records as raw JSON by kind and key. A kind whose document is damaged or from a newer build is not `usable`: nothing read, listed, written or replaced; syncing says once it is paused.
- `hub.rs`: thread-local. Editors hook up with `watch`; stores' marks reach the sync through it; `enable`/`disable`/`stop` serve the Account page. Names a storage by `Storage::identity()`.
- `vm.rs` (`SyncVm`, built by the shell), `text.rs`. Port: `WorkSync` (`pull`, `push`, `clear`; `FakeWorkSync`).

## Record

`kind` (`street`, `junction`, `rule`, `area`), `key` (place key, rule id, or `current` for the area; max 200 chars of letters, digits, `~_.:-`), `rev` (account counter), `edited_at` (UTC to the ms, compares as text; server caps a future time at its clock), `deleted` tombstone (purged after 90 days), `body` (JSON sealed with AD `work.<kind>.<key>.<user_id>`). Keys, times, revisions are unsealed: the server matches and sorts on them.

## `/api/work`

- `GET ?since=<rev>&start=<rev>&epoch=<uuid>` -> `{user, epoch, rev, more, records}`, revision order, 500 a page.
  - `since` moves page to page; `start` is where the pull began (the server judges the device's right to its revisions by it); `epoch` is the copy last read (`users.work_epoch`, new each time storage is turned on). A pull from 0 gets live records only, no tombstones.
  - `user` differing from the page's account: stop and ask `/api/me` again (`SyncVm::on_account_changed`). The document switches account only after the first page proves the session's, so two tabs on two accounts do not flip it.
  - `error: "corrupt"` with no body: leave the record alone.
- `410 resync` (`start` older than a purged tombstone, or `since` above the account's rev): pull everything, drop what the server no longer lists. `410 reset` (other epoch): storage was turned off and on meanwhile.
- `PUT {records: [{kind, key, base_rev, edited_at, deleted, body}]}` (max 100, body max 64 KB) -> `{rev, results: [{kind, key, status, rev, record?}]}`; `status` `accepted` | `superseded` (server's newer record comes back).
- `413 {"error": "too-large", "limit": "record"|"records"|"bytes"|"batch"}`: batch refused whole (account cap 5,000 records, 2 MB sealed). `400` bad JSON/key/time. `409 storage-off` any call while storage is not on the server. `401 signed-out`.
- `DELETE`: removes the server copy, new epoch (other devices reset, keep their places). The app never calls it.

## `cityloom-sync` document (version 1)

- Per `<kind>/<key>`: `edited_at`, `pending`, `deleted`, `rev` (last seen), `blocked`. Plus `user`, revision pulled up to, `epoch`.
- Another account starts at rev 0, keeps pending, drops what was synced to the other.
- Unreadable: copied once to `cityloom-sync-damaged`, replaced by an empty one with `unmarked_at` = epoch (`stamp(0)`), so the stores' records count as edited then and the server's copy wins (they go up only where the server has none; `claim_unmarked` clears it).
- Newer version: neither read nor written.

## Marking

- `CityStore::write_street`/`write_junction`/`choose` and `RestrictionStore::{add, update, remove, restore}` call `sync::model::mark` (depends only on `shared`) with `Storage::now_ms`, signed in or not, only when the stored JSON changed (the keeper also writes on a selection); a place moved off is marked deleted.
- The sync's own writes (`RestrictionStore::{put_raw, remove_raw, rename_numbered}`, `edits::Draft::{put_raw, remove_raw}`) never mark.
- Latest `edited_at` wins per place; a pulled record replaces the local one unless the local waits with a later time.

## When it runs

Page open with storage on the server; 250 ms after a write (not within 2 s of the last run); tab shown again; `online` (these two also ask `/api/me` again if the page could not learn who is signed in: `ShellVm::network_back`, `SessionVm::recheck`); `pagehide` as a `keepalive` push cut to 60,000 bytes (`sync::model::KEEPALIVE_BYTES`). The shell's `pagehide` calls `hub::flush()` first so the last edit is marked whatever the handler order.

## Failures

- `410 resync`: full pull. `410 reset`: turn-on merge `SyncDoc::restart` (every record competes again with its own time; nothing removed locally).
- `409 storage-off`: stop. `SyncVm::disable` forgets the server's numbers (`user`, `epoch`, `rev`, each record's `rev`) but keeps every record pending with its `edited_at`, so a stale copy cannot beat a later edit when storage is turned on again.
- `413`: retry one record at a time; the one refused alone is `blocked` (local, out of batches until the place is edited again), its limit said once.
- A skipped record (paused kind, or `corrupt`, which goes through `decide`) does not advance `rev`, so it comes back. A place superseded by a record that cannot be applied stays marked but is not resent in that run.

## Rules, editors, account

- Area rules get ids `r-<32 hex>` where storage has randomness (`Storage::random_hex`; test fake has none unless asked, so tests' `r1` stay); existing `r<n>` are renamed once at first sync.
- Editors reload a place a sync changed (`StreetVm::watch_sync`, `JunctionVm::watch_sync`) and say "Updated from another device." or "Your change to this place was replaced by a later one."
- Opt in (`hub::enable`): merges, says places sent/received. Opt out: confirmed; the server deletes its copy in the transaction that records the choice (`set_storage`); the page forgets the server's numbers, the browser's work stays. Storage off -> on gives a new `work_epoch`; choosing it when already on changes nothing. Deleting the account removes the rows.

## Known limits

- An editor reloaded by a sync keeps no undo history for that place.
- The map page does not redraw a place a sync changed (reads the city on open).
- The area chosen on one device replaces another's when its page next opens.
- Rules list is in id order, so a random-id rule can land anywhere.
- "Received" at opt-in is 0 in normal use (server copy is per account; turning storage off empties it).
