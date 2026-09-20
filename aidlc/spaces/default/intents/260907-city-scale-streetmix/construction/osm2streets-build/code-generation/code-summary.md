# Code Summary — `osm2streets-build` (U1)

## What was built

This Unit is a pin and its provenance, not application code, exactly as
`code-generation-plan.md` scoped it. Three things were produced:

1. **The verified, documented dependency pin.** `docs/osm2streets-pin.md`
   records the pinned `osm2streets` commit
   (`fc119c47dac567d030c6ce7c24a48896f58ed906`) and the pinned transitive
   `abstutil` commit (`0964f29315820c91b171b585eb51e300164e9197`), both
   re-verified live against the upstream repositories' `main` branches on
   2026-09-16, plus the exact build command
   (`cargo build --workspace --locked`), the toolchain versions
   (`rustc`/`cargo` 1.97.1, read from the repository's own
   `rust-toolchain.toml`, matching what `osm-extract-proxy` already pinned),
   and the WASM build tool recorded as `TBD` (deferred to
   `u6-client-surfaces`'s framework choice, not guessed). `docs/dependencies.md`
   gained a new "Pending git dependencies" section with rows for
   `osm2streets` and `abstutil`, both Apache-2.0, both explicitly marked as
   pinned-but-not-yet-consumed-by-a-crate.

2. **The golden-fixture dataset.** Five real OSM extracts, fetched live from
   the public Overpass API on 2026-09-16, committed under
   `fixtures/osm2streets/`: `well-tagged-street.osm.xml` (Northeast Pacific
   Street, Seattle — complete lane/width/turn tagging), `thinly-tagged-street.osm.xml`
   (Northeast 48th Street, Seattle — bare `lanes=2`, no width, no
   forward/backward split), `one-way-street.osm.xml` (Madison Street,
   Seattle — `oneway=yes`), `street-with-cycleway.osm.xml` (North Northlake
   Way paired with the adjacent Burke-Gilman Trail, Seattle — a
   separately-mapped cycleway way, not a same-way `cycleway:left` tag), and
   `one-intersection.osm.xml` (the 15th Avenue Northeast / Northeast 50th
   Street junction, Seattle — a real four-way intersection, fetched as a
   bounding-box extract so it captures the multiple way segments osm2streets'
   `split_ways` divides the two streets into). `fixtures/osm2streets/README.md`
   documents each fixture's real-world location, what it exercises, and why.
   All five are real, named locations; none required a synthetic
   substitute.

3. **The provenance/traceability record for this Unit itself**:
   `source-manifest.json` (every path this Unit wrote) and `traceability.json`
   (NFR7.2.1/.2/.4 and AC3.1.4/AC3.1.5 mapped `OK`; NFR7.2.3/.5 mapped `GAP`
   with the human follow-up as their target).

## Key decisions

- **Real Overpass extracts over synthetic fragments for all five fixtures.**
  `team.md`'s named preference is real data; every category had a suitable
  small, real, identifiable example in the Seattle area (osm2streets'/A-B
  Street's own home city, which turned out to be well-mapped enough that no
  category needed a synthetic substitute). Seattle was chosen opportunistically
  once the first search (well-tagged street) succeeded there, not for any
  significance to this project beyond "well-mapped, findable via Overpass
  without excessive querying."
- **`street-with-cycleway.osm.xml` pairs two ways, not one.** The plan's
  named case is "a street with a separately mapped cycleway" — meaning a
  road and an adjacent, independently-mapped `highway=cycleway` way, as
  distinct from `well-tagged-street.osm.xml`'s own `cycleway:left=track`
  tag on the road itself. A single-way fixture cannot exercise that
  distinction, so this fixture commits both ways (with their referenced
  nodes) rather than one.
- **`one-intersection.osm.xml` is a bounding-box extract, not a single way
  id.** An intersection is inherently multi-way; a single `way(id)` fetch
  cannot represent one. The bounding box incidentally captured transit-stop
  nodes alongside the road geometry — left in rather than hand-pruned, since
  a fixture drawn from a real extract should look like what the adapter
  will actually see in production.
- **The thinly-tagged fixture still carries a bare `lanes=2` tag.** The
  plan's own wording for this category is "missing width/lanes" — read as
  "missing the detailed tagging (width, per-direction split, turn lanes)
  that lets osm2streets avoid inferring," not "missing every lane-related
  tag." A street with genuinely zero lane information is rare enough in a
  well-mapped city that requiring it would have pushed this fixture toward
  a synthetic fragment; the chosen fixture still forces inference of
  per-lane width and the forward/backward split, which is what the
  provenance test floor (`team.md`) actually needs.
- **Overpass API access required an explicit `Accept` header.** The default
  `curl` request (no `Accept` header) was rejected with `406 Not Acceptable`
  by the server; adding `Accept: application/xml` (or `application/json` for
  exploratory queries) resolved it. Also encountered transient `504`-style
  "server too busy" responses from the public instance during fetching,
  handled with a short backoff and retry — consistent with `team.md`'s own
  characterisation of Overpass as "impolite to hit too hard," which is part
  of why this Unit fetches these five extracts once and commits them rather
  than fetching live at test time.

## Deviations and gaps

- **The GitHub fork (NFR7.2.3, security-design.md SD-1) does not exist and
  was not created by this Unit.** Creating a GitHub repository requires a
  human with write access to the project's own GitHub account; this Unit's
  tooling has no such credential and no standing to create one on the
  project owner's behalf. The pin in `docs/osm2streets-pin.md` is therefore
  an interim state, consumed directly from `a-b-street/osm2streets` and
  `a-b-street/abstreet`. **Exact follow-up scope, recorded in
  `docs/osm2streets-pin.md`'s "Fork status: NOT YET DONE" section:** (1)
  fork `a-b-street/osm2streets` into the project's own GitHub account; (2)
  once the client crate exists (`u4-street-import`), point its `Cargo.toml`
  `git =` URL at the fork instead of upstream — the `rev =` value is
  unchanged, the same pinned commit; (3) update `docs/osm2streets-pin.md`'s
  pin table to name the fork; (4) apply GitHub branch protection
  (force-push disabled) to the fork's default branch at
  `environment-provisioning` (NFR7.2.5, security-design.md SD-4) — that
  stage already owns GitHub-repository configuration project-wide.
- **No Cargo.toml dependency declaration was added.** This Unit adds no
  workspace crate and no consuming crate exists yet — `u4-street-import`
  owns that. The pinned commits, the exact dependency-declaration snippet
  (`rev =`, no `branch`/`tag`), and the transitive `abstutil` re-pin are all
  documented in `docs/osm2streets-pin.md` ready for that Unit to apply
  verbatim. `Cargo.lock` is correspondingly unchanged by this Unit — no
  lockfile edit was expected, since nothing in the workspace depends on
  `osm2streets` yet.
- **The golden-fixture suite's actual assertions do not exist yet.** This
  Unit commits the five fixture *inputs* only; the test code that runs
  osm2streets against them and asserts specific output (lane counts,
  widths, provenance values) is `u4-street-import`'s job per
  `unit-of-work.md`. `fixtures/osm2streets/README.md`'s closing section
  names this explicitly as what that Unit needs to pick up.
- **This Unit's own `unit-test-instructions.md` commands were run during
  authoring, not re-run as a final gate.** All five fixture files were
  validated well-formed with `xmllint --noout` as each was fetched (every
  one passed); the pin-file grep checks
  (`fc119c47dac567d030c6ce7c24a48896f58ed906` and
  `0964f29315820c91b171b585eb51e300164e9197` both present in
  `docs/osm2streets-pin.md`) and the dependency-manifest grep checks
  (`osm2streets` and `abstutil` both present in `docs/dependencies.md`)
  were re-run after the files were finalised and both passed. A final
  combined re-run of the full `unit-test-instructions.md` table was blocked
  mid-session by this workspace's `aidlc-plan-approval-guard` hook, which
  refused further `xmllint` invocations as "mutation-capable" pending a
  recorded plan-approval step; the individual validations above were
  already captured before that point and are not superseded by it.

## Traceability

See `traceability.json` in this directory. `NFR7.2.1`, `NFR7.2.2`, `NFR7.2.4`,
`AC3.1.4`, `AC3.1.5` are `OK`. `NFR7.2.3` and `NFR7.2.5` are `GAP`, targeting
the named human follow-up above — not presented as done.
