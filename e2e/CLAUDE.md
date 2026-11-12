# End-to-end tests (`e2e/`)

Loaded when working in this folder; the root `CLAUDE.md` has the project-wide rules.

## General

- Playwright specs `e2e/tests/*.spec.ts`, one per page; `shell.spec.ts` runs the same checks on all three. Run against built `web/pkg`: rebuild after Rust changes (`just e2e` does). Drags/pointer behaviour is covered only here.
- `fixtures.ts`: fails every test on a page error; fresh browser context each (empty storage); starts on the Plateau (automatic fixture serves its metro network tile, no network); stores `cityloom-lang=en` (`language.spec.ts` switches).
- Open editors by the address the map gives a place (`openStreet`, `openJunction`, from `placesOnMap`); the map has no list of places.
- Playwright reuses whatever listens on the port, so a stale server (or `just serve`) gives false results: `E2E_PORT=8291 just e2e` when 8137 is held. `E2E_PORT` moves server and Playwright.

## Tiers

- `e2e/tiers.json` is the one place a spec gets its tier, and each spec is in exactly one: `playwright.config.ts` throws and `just test-scripts` fails for a spec in none or in two. A Playwright project per tier: `ui` (one page, network stubbed, static server only: no database, no archive; `just e2e-ui`), `integration` (the real server and a Postgres, `account-server` and `sync`; `just e2e-integration`), `e2e` (the unpacked archive and the real server's delivery: `archive`, `delivery`, `serve`; `just e2e-archive`). `archive_also` (map, street) are UI specs the e2e tier runs again over the archive. `just e2e` runs all three; `just e2e tests/street.spec.ts` runs the named specs whatever their tier.
- CI runs one job per tier after a shared `build` job (the wasm, `web/vendor` and the pinned server as an artifact; `.github/actions/browser-tests`); `scripts/ci/ci_changes.py` picks the tiers and specs a pull request needs.

## Fixtures

- The OSM fixtures (`e2e/fixtures/*.osm.pbf`, `open-ends.osm`) are imported into JSON network tiles in `e2e/fixtures/generated/` (gitignored) by `just e2e-fixtures` (`citymodel_builder fixture`), which every browser recipe runs first and CI's `build` job runs once and hands on in the `browser-build` artifact. `fixtures.ts` names them `PLATEAU_TILE`, `TURBO_TILE`, `OPEN_ENDS_TILE`; `tileFrom(file)` answers a request for a tile with one.
- `about.spec.ts` checks the city behind the About card is read from the metro tile and no other host is asked (no Overpass), and, on every page, the top bar (Projects and About, the current one `aria-current="page"`, no Map link, the logo to `index.html`); `home.spec.ts` that the list asks for no road tile; `world.spec.ts` serves a one-place tile from `PLATEAU_TILE`.
- `junction.spec.ts` turbo tests serve `TURBO_TILE`: no Plateau junction has room for the basic layout. `street.spec.ts` serves `OPEN_ENDS_TILE` for open street ends (its source `open-ends.osm` has a `<bounds>` element: the importer panics inside osm2streets on OSM XML without one).
- `restrictions.spec.ts`: area by clicks and by keyboard (map focused, Enter places a corner at the centre, Shift+Enter closes).
- Map page: `mapReady(page)` waits for places; MapLibre is a WebGL canvas (`window.cityloomMap`).

## Server and account specs

- Run against `cityloom_server --features test-provider`: `playwright.config.ts` starts `target/e2e/cityloom_server`, the copy `scripts/pin-server.sh` makes (a shared target dir cannot swap the binary mid-run), with fixed test keys, `FAKE_PROVIDER=1` and the caller's `DATABASE_URL`.
- `fixtures.ts` ignores console errors for `/api/me` and `/api/providers` (401/404/503 normal signed out) and lets `/api/work` ones through. A spec that fails `/api/me/*` on purpose annotates `failing-account-api`.
- `account-server.spec.ts`: fake-provider sign-in, unique `sub` per test, needs `DATABASE_URL`. `account.spec.ts`: stubbed answers, no database.
- `sync.spec.ts`: two contexts as two devices of one account (`account-helpers.ts`; real server, fake provider, Postgres: needs `DATABASE_URL`); `setOffline`, synthetic `visibilitychange`/`online`. Covers keepalive push on leave, conflict, opt-out reaching the other device, merge at sign-in, and projects: each project's street reaching a clean browser (no seeded project: `cleanBrowser`) in its own project, a rename reaching the other device, a delete sending a device with the project open to `index.html?missing=1`, and rows an older CityLoom wrote (bare keys, no project record, `PUT /api/work` with the site's `Origin`) landing in the first project. `openStreetIn` opens a street by an address naming the project (going to `map.html?project=` and leaving at once can beat the page's remembering it).

## Archive mode

`E2E_ARCHIVE=1` (`just e2e-archive`, the E2E tier: only the `e2e` project, with `archive_also` added): `fixtures.ts` skips routes for the metro index and default tile (a route would answer before the server and prove nothing); `playwright.config.ts` starts a fresh server (never reused) with `CITYLOOM_AREAS` and `DATA_ROOT` in `target/e2e-archive`, which it unpacks and serves. The wasm build excludes `cityloom_server` and `citymodel_builder` (justfile `test`).
