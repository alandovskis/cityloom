---
title: Test data
---

# Test data

This page describes the test data. The levels that use it are in [Levels of tests](levels.md).

## OpenStreetMap fixtures and tiles

The browser tests' road data are `e2e/fixtures/plateau.osm.pbf`, `turbo.osm.pbf` and `open-ends.osm.pbf`.
`open-ends.osm` is the OSM XML source of `open-ends.osm.pbf`, with its `<bounds>` element; it is not read by the tests.
`just e2e-fixtures` imports each `.osm.pbf` into one network tile, `e2e/fixtures/generated/<name>.json`
(`citymodel_builder fixture`; ignored by git). It is a private recipe, so `just` does not list it; every browser recipe
runs it first. In CI, the `build` job runs it once and hands the tiles on in the `browser-build` artifact, with the
wasm, `web/vendor` and the pinned server.

## `fixtures.ts`

`e2e/tests/fixtures.ts` is what every spec imports.

- `PLATEAU_TILE`, `TURBO_TILE` and `OPEN_ENDS_TILE` are the generated tiles, and `tileFrom(file)` answers a request
  for a tile with one.
- Every test fails on a page error or a console error. Errors from `/api/me`, `/api/providers` and `/api/work` are let
  through, and from `/api/me/*` when the test carries the `failing-account-api` annotation.
- Each test gets a fresh browser context, so storage starts empty.
- An automatic fixture serves the Plateau tile as the default metro tile, except with `E2E_ARCHIVE`, where the server
  answers from the archive.
- Storage starts with `cityloom-lang=en` and the first project.

## The fake sign-in provider

The server's `test-provider` feature (`crates/cityloom_server/Cargo.toml`) adds `src/fake_provider.rs`, an OpenID
provider served by the server itself. Tests choose who signs in with the `fake-login` cookie. The browser tests start
the server with `FAKE_PROVIDER=1`, and `account-server.spec.ts` uses a unique `sub` per test.

## The throwaway Postgres

`just db` starts a container named `cityloom-db` (`postgres:18`) on 127.0.0.1:55432 and prints the
`export DATABASE_URL=…` line to run; `just db-stop` removes it. Each `#[sqlx::test]` makes its own database on it. In
CI, the unit-test and integration jobs run a `postgres:18` service.

## The e2e archive

`scripts/e2e-archive.sh` copies `e2e/fixtures/generated/plateau.json` as the one metro tile, `0_0`, packs it with
`citymodel_builder pack` into `target/e2e-archive/areas/plateau.citymodel`, and runs
`E2E_ARCHIVE=1 npx playwright test --project=e2e`. The server unpacks the archive through `CITYLOOM_AREAS` and
`DATA_ROOT`, both in `target/e2e-archive`.
