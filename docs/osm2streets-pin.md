# osm2streets dependency pin

This is the committed provenance record `team.md` (Deployment) requires for
the `osm2streets` dependency: what commit is pinned, how it is built, and
what toolchain builds it. `scripts/verify.sh`'s dependency-manifest checks
and `docs/dependencies.md`'s `osm2streets`/`abstutil` rows both point back
to this file; this is the one place that carries the build command and
toolchain versions in full, so a maintainer can rebuild the pin six months
from now without reconstructing it from memory (`security-design.md` SD-5).

## Pinned commits

| Dependency | Repository (interim, see Fork status) | Pinned commit | Verified |
|---|---|---|---|
| `osm2streets` | `https://github.com/a-b-street/osm2streets` | `fc119c47dac567d030c6ce7c24a48896f58ed906` | 2026-09-16, live, this Unit |
| `abstutil` | `https://github.com/a-b-street/abstreet` | `0964f29315820c91b171b585eb51e300164e9197` | 2026-09-16, live, this Unit |

Both are `main`-branch HEAD commits at verification time, not chosen for any
other property — the point of the pin is that they stop moving from here,
not that this particular commit is special.

Cargo dependency declaration, as added by `u4-street-import`
(`cityloom-street-import`) — the consuming crate now exists — per
`security-design.md` SD-2 (`rev =`, never `branch`/`tag` alongside it,
since Cargo treats that combination as an error):

```toml
[dependencies]
osm2streets = { git = "https://github.com/a-b-street/osm2streets", rev = "fc119c47dac567d030c6ce7c24a48896f58ed906" }
streets_reader = { git = "https://github.com/a-b-street/osm2streets", rev = "fc119c47dac567d030c6ce7c24a48896f58ed906" }
```

`streets_reader` is `cityloom-street-import`'s own addition, not anticipated
by this file when it was first written: `osm2streets` itself operates only
on an already-built `StreetNetwork` and has no OSM XML/PBF parser of its
own; `streets_reader`, in the same upstream repository at the same pinned
commit, is the crate that offers `osm_to_street_network`, the actual
bytes-in entry point `u4-street-import` needs. Pinned identically to
`osm2streets` — same repository, same `rev` — so this is not a second,
independently-tracked pin.

`osm2streets`'s own `Cargo.toml` on the commit above carries:

```toml
abstutil = { git = "https://github.com/a-b-street/abstreet" }
```

with no `rev`, `tag`, or `branch` — confirmed live on 2026-09-16, the same
unpinned pattern `osm2streets-js` had (`project.md` Mandated, the pin
requirement this Unit exists to close).

**Resolved deviation from the plan above, verified during `u4-street-import`'s
build:** a workspace `[patch]` section pinning `abstutil` to the commit
turned out to be unusable here — Cargo refuses a `[patch]` whose target and
replacement resolve to the *same* commit (`error: patch for abstutil points
to the same source, but patches must point to different sources`), which is
exactly what happens today, since `0964f29315820c91b171b585eb51e300164e9197`
is upstream's live `main` HEAD at verification time. Declaring `abstutil`
directly in `cityloom-street-import`'s own `Cargo.toml` with an explicit
`rev =` was tried next and also rejected — not by Cargo, but by the Rust
compiler: it resolves to a *second*, independent `abstutil` package (a
different Cargo `SourceId` from the one `osm2streets`/`streets_reader`
themselves pull in unpinned), so a `Timer` value built from one does not
type-check where `streets_reader::osm_to_street_network` expects the other
(`E0308`, reproduced against this exact dependency set). The working form:
`cityloom-street-import` declares `abstutil` the same bare, unpinned way
upstream does (`abstutil = { git = "https://github.com/a-b-street/abstreet" }`,
no `rev`), so Cargo unifies it with `osm2streets`/`streets_reader`'s own
reference into exactly one resolved package (confirmed via `cargo tree -p
cityloom-street-import -i abstutil`). The pin itself is enforced one layer
down instead of in this Cargo.toml line: the committed `Cargo.lock` records
the resolved commit (`0964f29315820c91b171b585eb51e300164e9197`, matching
the intended pin above) and `cargo build --locked` (`scripts/verify.sh`,
every CI run) fails hard rather than silently re-resolving to a later
commit the day upstream's default branch moves. `docs/dependencies.md`'s
`abstutil` row carries this same record.

## Build command

```
cargo build --workspace --locked
```

No separate WASM-binding build step: `osm2streets` is consumed as an
ordinary Cargo git dependency of the client crate, not through the
`osm2streets-js` WASM-bindgen wrapper package, so it compiles into the same
Rust binary as the rest of the workspace (`team.md`, Deployment). `Cargo.lock`
is committed at the workspace root once a crate actually depends on
`osm2streets` (`security-design.md` SD-3); `--locked` turns an out-of-date or
manually-edited lockfile into a hard build failure rather than a silent
re-resolution.

## Toolchain versions

| Tool | Version | Source |
|---|---|---|
| `rustc` / `cargo` | 1.97.1 | `rust-toolchain.toml` (repository root), the same pin `osm-extract-proxy` already established |
| WASM build tool (`wasm-pack`, `trunk`, or equivalent) | TBD | Deferred to `u6-client-surfaces`'s DOM-framework selection (`tech-stack-decisions.md`); not guessed here |

## Fork status: NOT YET DONE

`security-design.md` SD-1 requires the dependency be consumed from the
project's own fork (`github.com/<project-org>/osm2streets`), never directly
from `a-b-street/osm2streets`. **That fork does not exist yet.** Creating a
GitHub repository requires a human with write access to the project's GitHub
account; this Unit's own tooling has no such credential and cannot create it.

The pin above is therefore an **interim state**: it satisfies `NFR7.2.1`
(pinned to a commit), `NFR7.2.2` (the transitive `abstutil` pin — now
enforced at the `Cargo.lock` layer rather than a `rev =` in
`cityloom-street-import`'s own `Cargo.toml`; see the resolved deviation
above for why), and `NFR7.2.4` (locked resolution via `Cargo.lock` and
`--locked`) against upstream `a-b-street/osm2streets` directly. It does
**not** satisfy `NFR7.2.3` (fork identity) or `NFR7.2.5` (branch protection
on the fork) — those remain open, not silently treated as done.

**Follow-up, exact scope, once the human creates the fork:**

1. Fork `a-b-street/osm2streets` (and, per the resolved deviation above,
   `a-b-street/abstreet` too — a `[patch]` re-pin was tried and does not
   work while the pinned commit and upstream's live HEAD coincide) into the
   project's own GitHub account.
2. Update the `git =` URL in `crates/cityloom-street-import/Cargo.toml`'s
   `osm2streets` and `streets_reader` entries from
   `https://github.com/a-b-street/osm2streets` to
   `https://github.com/<project-org>/osm2streets` — the `rev =` value stays
   the same commit SHA (`fc119c47dac567d030c6ce7c24a48896f58ed906`); forking
   does not change what code that commit points to. Update the `abstutil`
   entry's bare `git =` URL the same way, still without a `rev =` (see the
   resolved deviation above for why).
3. Update this file's table above to point at the fork instead of upstream.
4. Apply GitHub branch protection (force-push disabled) to the fork's
   default branch at `environment-provisioning`, alongside the project's
   other GitHub hardening steps (push protection, CodeQL, Dependabot —
   `team.md`), per `security-design.md` SD-4.

Until that follow-up lands, `docs/dependencies.md`'s rows for these two
dependencies say so explicitly rather than presenting the fork as complete.
