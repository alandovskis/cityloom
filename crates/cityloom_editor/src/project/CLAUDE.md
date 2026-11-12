# Project slice (`src/project/`)

Loaded when working in this folder; the root `CLAUDE.md` has the project-wide rules.

- A project is a named container tied to an area. `model.rs`: `Project { id, name, area (Area::to_query), created_at (sync stamp) }`, `Refused { Empty, TooLong, Unknown, Unusable }` (data only: no wording here, the words come later as `Said`), names trimmed, 1-80 characters.
- `store.rs`: `ProjectStore` over `cityloom-projects` (`{"version":1,"projects":[…]}`, creation order). Future version left alone; damaged copied to `cityloom-projects-damaged` only by the next successful change; each write re-reads. `cityloom-project` holds the last-opened id. Each change marks `Kind::Project` (key = the project id) for the sync.
- **`ProjectStore::new` takes the ROOT storage, never a `ScopedStorage`**: the list and the last-opened id belong to no project, and `mark` would otherwise prefix the key.
- `delete` removes the record, blanks the project's scoped keys (`shared::scoped::SCOPED_KEYS`, each as `<key>@<id>`; `Storage` has no removal, an empty value reads as absent) and marks the project record deleted. It does not mark the project's own records: that is `sync::local`'s job.
- Ids: `p-` + 12 hex digits where storage has randomness, else `p1`, `p2`, …; `first` is the first project's id (`shared::project_id`).
- `last_changed` is the newest `edited_at` among the sync entries of a project (its `project/<id>` entry and records whose wire key belongs to it), else `created_at`.
- May use `shared`, `sync::model`, `place::area`; never another slice's `vm`, views or text.
