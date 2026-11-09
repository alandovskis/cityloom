# End-to-end tests (`e2e/`)

Loaded when working in this folder; the root `CLAUDE.md` has the project-wide rules.

## General

- Playwright specs `e2e/tests/*.spec.ts`, one per page; `shell.spec.ts` runs the same checks on all three. Run against built `web/pkg`: rebuild after Rust changes (`just e2e` does). Drags/pointer behaviour is covered only here.
- `fixtures.ts`: fails every test on a page error; fresh browser context each (empty storage); starts on the Plateau (automatic fixture serves its metro tile, no network); stores `cityloom-lang=en` (`language.spec.ts` switches).
- Open editors by the address the map gives a place (`openStreet`, `openJunction`, from `placesOnMap`); the map has no list of places.
- Playwright reuses whatever listens on the port, so a stale server (or `just serve`) gives false results: `E2E_PORT=8291 just e2e` when 8137 is held. `E2E_PORT` moves server and Playwright.

## Tiers

- `e2e/tiers.json` is the one place a spec gets its tier, and each spec is in exactly one: `playwright.config.ts` throws and `just test-scripts` fails for a spec in none or in two. A Playwright project per tier: `ui` (one page, network stubbed, static server only: no database, no archive; `just e2e-ui`), `integration` (the real server and a Postgres, `account-server` and `sync`; `just e2e-integration`), `e2e` (the unpacked archive and the real server's delivery: `archive`, `delivery`, `serve`; `just e2e-archive`). `archive_also` (map, street) are UI specs the e2e tier runs again over the archive. `just e2e` runs all three; `just e2e tests/street.spec.ts` runs the named specs whatever their tier.
- CI runs one job per tier after a shared `build` job (the wasm, `web/vendor` and the pinned server as an artifact; `.github/actions/browser-tests`); `scripts/ci/ci_changes.py` picks the tiers and specs a pull request needs.

## Fixtures

- `home.spec.ts` stubs Nominatim, Overpass, metro tiles with `page.route`; `world.spec.ts` serves a one-place tile from `e2e/fixtures/plateau.osm.pbf`.
- `junction.spec.ts` turbo tests serve `e2e/fixtures/turbo.osm.pbf` (`TURBO_PBF`): no Plateau junction has room for the basic layout.
- `restrictions.spec.ts`: area by clicks and by keyboard (map focused, Enter places a corner at the centre, Shift+Enter closes).
- Map page: `mapReady(page)` waits for places; MapLibre is a WebGL canvas (`window.cityloomMap`).

## Server and account specs

- Run against `cityloom_server --features test-provider`: `playwright.config.ts` starts `target/e2e/cityloom_server`, the copy `scripts/pin-server.sh` makes (a shared target dir cannot swap the binary mid-run), with fixed test keys, `FAKE_PROVIDER=1` and the caller's `DATABASE_URL`.
- `fixtures.ts` ignores console errors for `/api/me` and `/api/providers` (401/404/503 normal signed out) and lets `/api/work` ones through. A spec that fails `/api/me/*` on purpose annotates `failing-account-api`.
- `account-server.spec.ts`: fake-provider sign-in, unique `sub` per test, needs `DATABASE_URL`. `account.spec.ts`: stubbed answers, no database.
- `sync.spec.ts`: two contexts as two devices of one account (`account-helpers.ts`; real server, fake provider, Postgres: needs `DATABASE_URL`); `setOffline`, synthetic `visibilitychange`/`online`. Covers keepalive push on leave, conflict, opt-out reaching the other device, merge at sign-in.

## Archive mode

`E2E_ARCHIVE=1` (`just e2e-archive`, the E2E tier: only the `e2e` project, with `archive_also` added): `fixtures.ts` skips routes for the metro index and default tile (a route would answer before the server and prove nothing); `playwright.config.ts` starts a fresh server (never reused) with `CITYLOOM_AREAS` and `DATA_ROOT` in `target/e2e-archive`, which it unpacks and serves. The wasm build excludes `cityloom_server` and `citymodel_builder` (justfile `test`).
