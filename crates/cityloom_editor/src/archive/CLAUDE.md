# Archive slice (`src/archive/`)

Loaded when working in this folder; the root `CLAUDE.md` has the project-wide rules.

- Portable design archive (#74; format and choices UNVERIFIED): Export/Import design on the Account page (`#archive-slot`, mounted by `mount_account`; works signed out).
- `model.rs` (no I/O): ONE JSON document: `kind: "cityloom-design"`, integer `format_version` (1; higher refused), `app_version`, `created_at`, `source` (area key, tile checksum, importer version; informational), `area` (informational; import never changes the chosen area), `edits` (raw `cityloom-edits`, written by `Draft::to_json`, read `edits::VERSION - 1`..`NEWEST`), `restrictions` (raw `cityloom-restrictions`, v1).
- Unknown fields ignored; `notes`, `scenarios` reserved, neither read nor written.
- NOT exported: `cityloom-sync`, account/session, `cityloom-lang`, theme, `cityloom-network:<key>`.
- File: `cityloom-<day>.cityloom-design.json`, never a bare `.citymodel` (that is `citymodel_builder`'s).
- `parse_and_validate` checks all before keeping anything. Refuses: over `MAX_BYTES` (10 MB), not JSON, wrong `kind`, damaged header/edits/rules, newer format, edits newer than `NEWEST` or older than 4, rules newer than 1, non-object record, key sync cannot carry. Each is an `ArchiveError` as `Said` (`archive-error-*`, ending "Nothing on this device was changed.").
- `plan_import` per place key: `New`, `Same`, `Conflict` (JSON equality of raw record). `apply(Choice::KeepMine | UseArchive)`: new records always; conflicts only with `UseArchive`, chosen once for the whole import.
- Writes: `edits::Draft::put_raw` (one write; unreadable records untouched) and `RestrictionStore::put_raw`, not `CityStore::write_street` (needs network loaded); then `sync::model::mark` per record. Failed write restores both documents.
- Damaged/newer local edits or rules stop import (`LocalEdits`/`LocalRules`) only if the archive has records for them; no copy aside, no write. Export refuses where local edits/rules are unreadable; no file when no work.
- Port `ArchiveFiles` (`save`, `pick` with callback; `FakeArchiveFiles`; `BrowserFiles` in `platform.rs`: Blob download, never-attached `<input type=file>`).
- `ArchiveVm`: conflict question focused on "Keep mine", `Escape` cancels, result focused and announced. E2E: `e2e/tests/design-archive.spec.ts` (`archive.spec.ts` is the area archive's).
- Known limits:
  - Conflict decided for the whole import, not per place.
  - Rule ids `r<n>` of devices that never synced collide as conflicts (random ids avoid it).
  - Archive unsigned, unencrypted.
  - Area recorded but not applied: work for another area is an orphan until that area is opened.
  - Records the build cannot read pass through unread.
