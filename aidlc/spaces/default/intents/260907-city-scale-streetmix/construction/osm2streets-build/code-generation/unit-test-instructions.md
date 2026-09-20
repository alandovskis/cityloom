# Unit Test Instructions — `osm2streets-build` (U1)

This Unit adds no Rust code and no new workspace crate — only pin
documentation, dependency-manifest rows, and committed fixture files. Its
"test" floor is therefore validation of those artifacts, not `cargo test`
against application logic (there is none here). The golden-fixture suite's
actual assertions against osm2streets output belong to `u4-street-import`
(not yet built) and are explicitly out of this Unit's test floor — this
Unit only commits well-formed fixture data for that later suite to consume.

## How to validate THIS UNIT

| Purpose | Command |
|---|---|
| Every fixture file is well-formed XML | `for f in fixtures/osm2streets/*.osm.xml; do xmllint --noout "$f" || exit 1; done` |
| The provenance file exists and names both pinned commits | `grep -q 'fc119c47dac567d030c6ce7c24a48896f58ed906' docs/osm2streets-pin.md && grep -q '0964f29315820c91b171b585eb51e300164e9197' docs/osm2streets-pin.md` |
| The dependency manifest has both rows | `grep -qi 'osm2streets' docs/dependencies.md && grep -qi 'abstutil' docs/dependencies.md` |

No `cargo test` command is scoped to this Unit because it adds no crate.
Once `u4-street-import` declares the `osm2streets` git dependency and adds
the adapter crate, *that* Unit's `unit-test-instructions.md` will own the
actual `cargo test -p <adapter-crate>` commands exercising these fixtures.

## Expected coverage

Not applicable — no Rust code in this Unit to measure line coverage
against. The `cargo-llvm-cov` floor applies from `u4-street-import` onward,
once the adapter crate exists.

## Test data

The five committed fixtures themselves ARE this Unit's output; see
`fixtures/osm2streets/README.md` for what each represents.

## Definition of green for this Unit

Every fixture file is well-formed, `docs/osm2streets-pin.md` and
`docs/dependencies.md` are both present and complete, and
`code-summary.md` explicitly names the two open follow-ups (the manual
GitHub fork, and `u4-street-import`'s pending consumption of these
fixtures) rather than presenting either as done.
