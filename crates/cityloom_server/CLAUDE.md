# `cityloom_server`

Loaded when working in this folder; the root `CLAUDE.md` has the project-wide rules.

Workspace binary and library, not built to wasm (`just test` type-checks wasm with `--exclude cityloom_server --exclude citymodel_builder`).

## Modules

- `static_files`: static site (`403` for a path leaving `web/`, `400` for a bad escape, ranges, `index.html`).
- `config`: env into `Config` (`.env.example`); key fields `Zeroizing`. `AccountSide::Off(OffReason)` when `DATABASE_URL` or a key variable is missing/invalid: `/auth/*` and `/api/*` answer 503, site still served. A database down at start-up is not `Off`: lazy pool, `require_ready` answers 503 until a background task has migrated.
- `crypto`: versioned master keys; per-user data key wrapped under the current master key; AES-256-GCM as `version ‖ nonce ‖ ciphertext ‖ tag`, AD `table.column.<row id>`; identity lookup HMAC.
- `accounts` (rules over a `Repo` trait; `MemoryRepo`), `db` (`PgRepo` seals on write, opens on read; migrations in `cityloom_server/migrations`).
- `session`: `__Host-cityloom-session`, 30 days sliding, refreshed after a day. `flow`: `__Host-cityloom-flow`, encrypted, ten minutes, `safe_return`.
- `providers`: one OpenID Connect client for OpenStreetMap, Google, Microsoft, Apple. `apple.rs` signs/renews Apple's client secret; `microsoft.rs` checks the issuer against the token's own tenant.
- `auth`, `api` (`api/work.rs` is `/api/work`, see `src/sync/CLAUDE.md`).
- `work`: rules over metadata, no I/O (`Put`, `plan_batch`, `Ask`); `db/work.rs` is `WorkRepo`. Migration `0002_work.sql`: `users.work_rev`, `users.work_epoch` (new each time storage is turned on), `work` table (row per `(user, kind, key)`, tombstone flag, purged after 90 days).
- `app`: router, Origin check on changes, `require_ready`, request logs without queries. Rate limits keyed on the rightmost `X-Forwarded-For` (Railway's proxy appends it; `client_address`): `/auth` `AUTH_RATE_PERIOD_SECS`/`AUTH_RATE_BURST` (6 s, 20); `/api/work` own limiter `WORK_RATE_PERIOD_SECS`/`WORK_RATE_BURST` (1 s, 60). `e2e/playwright.config.ts` raises both bursts.
- `fake_provider`: feature `test-provider` only, never in a release (`scripts/check-no-fake.sh`); tests pick who signs in with the `fake-login` cookie.

## Auth rules

- Logs never hold a token, code, cookie, email, `sub` or label.
- Callback outcomes: `#account=<new|signed-in|linked|already-linked|conflict|cancelled|failed>`; `conflict` adds `&provider=<slug>`.
- A callback with a `state` not the flow cookie's leaves that cookie alone (another tab's live flow).

## Key rotation (`db/rotate.rs`; `cityloom_server rewrap|rehash`)

- `rewrap` prints `resealed N data keys`. Order matters: (1) deploy with the new key added and `DATA_KEY_CURRENT` set to it; (2) run it (Railway: `railway ssh`, `/app/cityloom_server rewrap`) until it reseals 0; (3) only then remove the old key. A server still on the old current key keeps wrapping new users under it and `rewrap` does not see them.
- `rehash` (only it reads `OLD_LOOKUP_KEY`) needs the server stopped; prints identities checked, rehashed, already on the new key.

## Area archive

- With `CITYLOOM_AREAS` (folder with exactly one `.citymodel`) and `DATA_ROOT` (writable; both in `.env.example`), start-up unpacks it (`lib.rs::unpack_areas`, `citymodel_builder::archive::unpack_folder`): verifies index trailer and each member's size and SHA-256 (no path taken from the tar), writes to `DATA_ROOT/.unpack-*`, renames to `DATA_ROOT/<area>/` (`metro/` with the `index.json` it writes).
- Marker `.index-sha256` with the same trailer and every file at its size skips the unpack on restart. Any failure (none/several archives, bad index, wrong checksum, unwritable folder) is fatal.
- `/data/metro/` is served from there by `static_files::router_with_data` with the existing rules; without the variable `web/data/` is served.
- Limits: one archive; `scripts/deploy.sh` and `deploy/Dockerfile` still ship the folders; files verified when written, only sizes on restart; bytes after the tar end marker ignored.

## Delivery of the code (`static_files.rs`)

- Scripts, styles, wasm, vendored libs are served under `/v/<build>/…`; `<build>` is a content hash computed by the server (not `build.sh`): metadata pass at most every 2 s on a blocking thread, only for a page or `/v/` address, content hashed only when a file changed. Pages get it wherever the markup writes `{{build}}` (`href="v/{{build}}/app.css"`, `src="v/{{build}}/home.js"`, preloads).
- Relative imports/fetches (`./pkg/…`, wasm beside glue, `./city.js`) resolve under the same prefix, so glue never pairs with another build's wasm. Another build's prefix: 404 `no-store`.
- Versioned URLs: `public, max-age=31536000, immutable`. Pages: filled per request, whole, weak `ETag` (no ranges, no `Last-Modified`; `If-None-Match` weak, lists and `*`; HEAD from the GET); a 304 carries the 200's `Cache-Control`, which keeps the configured `CACHE_CONTROL` (`no-cache` in production).
- New code references must be `v/{{build}}/`; `cityloom_server/tests/static_files.rs` fails on any other and on a reference to a missing file. Exceptions: favicons and `logo-*` svgs by name; `boot.js` is the one script not under `v/`.
- Not versioned: `data/` (tile index `max-age=300, stale-while-revalidate=600`; tiles `CACHE_CONTROL`: a cached old tile would persist under a new checksum), `glyphs/`, `fonts/` (one day).
- `scripts/precompress.mjs` (from `build.sh`) writes `.br`/`.gz` beside `web/pkg` and `web/vendor` files (gitignored); sent as is unless older than the file (then that coding is left out and the file compressed on the fly). Other text, the metro `.json` tiles included, is compressed on the fly (full 200 responses only; Range requests stay raw 206).

### Boot failure

- Pages that run the module carry a hidden `#boot-failed` notice in both languages (`data-boot` keys `ui-boot-*`; checked by `shell::tests`), shown by `window.cityloomBootFailed()` (`web/boot.js`: classic script before the entry script, unversioned so it exists when the build's addresses do not).
- The entry script's `onerror` calls it when the entry script or a static import (`city.js`, `mapcanvas.js`) is a 404 (a deploy after the page opened does this to `/v/<old build>/`); each entry script wraps only `import()` and `init()` of the glue in `try … catch` and calls it.
- Home mounts search first (`mount_home`), the city after roads arrive (`MapPage::mount_city`; `#hero-facts-slot` keeps room for the facts line).

## Tests

`cityloom_server/tests/`: `auth_flow`, `api`, `db`, `no_plaintext`, `rotate`, `providers`, `static_files`, `unavailable`, `work_api`, `work_db`, `areas`, `data_mount`.
