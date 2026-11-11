# Projects: design

## Intent

Today CityLoom has one implicit workspace per browser: one "current area", one
edits document, one restrictions document. A **project** makes the workspace
explicit. A project is a named container tied to one metropolitan area; everything
the user edits (streets, junctions, rules) is saved inside it. The first page a
user sees is the list of their projects.

## Goals

- A project has a name and exactly one metropolitan area, fixed at creation.
- The home page (`web/index.html`) is the list of the user's projects: create,
  open, rename, delete. An empty list invites creating the first one.
- Opening a project opens the map for its area. Street and intersection pages
  edit inside the project they were opened from. All edits, restrictions and the
  area choice are saved per project.
- Storage follows the existing choice. Anonymous users keep projects in the
  browser only. Users who chose "Store my work on the server" have their
  projects synced like their other work.
- Existing work moves into an automatic first project, on both the device and the
  server. Nothing is left behind.
- English and fr-CA throughout, WCAG 2.2 AA, per the project's conventions.

## Non-goals

- Sharing a project or working on it with others. A separate issue, later. The
  design must not make it harder (a project has a stable id that is not derived
  from the user), but nothing for it is built.
- More than one metropolitan area. The catalogue is Montreal only. The area is a
  field on the project so a second area can be added later.
- Moving a project to another area after creation.

## Anti-goals

- Losing or duplicating a user's existing edits during migration.
- A project list that makes a returning user's first click slower than today's
  "land on the map".
- Cross-project leakage: an edit made in one project appearing in another.

## Constraints from the codebase (found by reading it)

- One global area lives in `cityloom-area`; `CityStore::current` reads it, and
  every page uses it (`city/store.rs`).
- All edits are one document, `cityloom-edits`, keyed per place and not per area
  (`city/edits.rs`; versioned, with a `-damaged` copy for unreadable documents).
  Restrictions are `cityloom-restrictions`; sync state is `cityloom-sync`.
- Sync identifies a record as `Id{kind, key}` with kinds Street, Junction, Rule,
  Area; keys are up to 200 characters. The server `work` table has primary key
  `(user_id, kind, key)`, and the row's sealed body is bound to its
  `kind.key.user_id` as associated data.
- Pages are told only a place (`street.html?street=7`), not an area or document.
- There is no generic client migration framework; migrations so far were done by
  hand. Server migrations are sqlx (`crates/cityloom_server/migrations`).
- Slices may not use each other's view-models, views or text; the sync is reached
  through `sync::hub` and `sync::model`. A project slice must respect this.

## Decisions made with the user

- A project is tied to one metropolitan area; Montreal only for now.
- The first page is the list of projects.
- Everything edited is saved in the project.
- Sharing is deferred.
- Existing work moves into an automatic first project.
- Storage follows the existing local/server choice.

## My calls (not yet confirmed, marked for review)

- The create form asks for a name only while Montreal is the sole area; the area
  is shown read-only. A picker appears when a second area exists.
- Where the current hero and introduction go: they move off the first screen to a
  short "about" section below or beside the list. The builder may choose the exact
  placement.
- The project list shows name, area and last-changed time, sorted by last change.
- Deleting a project asks for confirmation and removes its local data and, when
  synced, its server rows.
- Opening a project remembers it as the last one, so a link to a street or
  junction page without a project id falls back to the last-opened project, and to
  the list if none exists.
- Scoping approach: key all per-project data by a project id, rather than keeping
  separate databases. How to carry the id through sync keys and the server schema
  (a key prefix, or a new column with a primary key change) is left to the builder
  to decide with attention to the associated-data binding of existing rows.

## More calls, from the builder check (mine, not yet confirmed)

- **One first project across devices.** The automatic first project gets a
  well-known fixed id (not random, not derived from the user), so every device that
  migrates independently produces the same project and sync merges them. Projects
  the user creates afterwards get random ids.
- **Old clients after the server migration.** The server accepts a write under an
  old-format key and files it under the first project. No compatibility path
  beyond that, and no refusal of old clients.
- **The project record is synced.** It is a new sync `Kind` carrying name, area,
  created and last-changed times, so rename, delete (tombstone) and empty projects
  work. A device with a project open that is deleted elsewhere returns to the list
  with a short notice.
- **Opting in or out of server storage.** Opting in uploads all local projects,
  as it uploads work today; opting out behaves as it does today for work.
- **What triggers migration.** Only legacy edits or restrictions do. A stored area
  alone creates nothing, and the user sees the empty list. A stored area that is
  not Montreal is ignored and the first project uses Montreal.
- **Archive.** Export covers the open project; import goes into the open project
  using the existing New/Same/Conflict plan. The `-damaged` copy handling applies
  per project.
- **Area chooser in the map.** Removed, since the area is fixed per project.

## Migration

On first load after the change, if legacy data exists (`cityloom-edits`,
`cityloom-restrictions`, `cityloom-area`) and no projects exist, create one project
from it, named by a default (for example "My first project"), area from
`cityloom-area` when it is Montreal, and move the legacy data into it. On the
server, existing `work` rows for the account are assigned to that same project in a
new migration, and any new-format client must read the migrated rows. A device that
has legacy data only on the browser and an account that also has server rows must
end with one first project, not two. The migration is tested with the golden-file
approach already used for edits, and in the integration tier for the server.

## Left to the builder

- Id format and project record shape; the module layout (a new `project/` slice
  following the vertical-slice and dependency rules).
- Route/URL shape for project-scoped pages.
- Server schema and the version bumps (edits document version, sync document,
  archive `format_version`, whether an exported archive carries the project name).
- Test placement, and the CI spec-map rules for new folders and specs.
- A product version bump, per the repository rules.
