---
id: getting-started
title: Getting started
sidebar_position: 2
---

# Getting started

This page is for developers. The repository uses [`just`](https://just.systems/) to run its tasks; run `just` with no
argument to list them.

## What you need

- A Rust toolchain with the `wasm32-unknown-unknown` target.
- Node.js and npm.
- `just`.

`just setup` installs the rest, once:

```sh
just setup
```

It runs `npm ci` at the root (the map's runtime libraries, MapLibre and PMTiles) and in `e2e` (Playwright), installs
`wasm-bindgen-cli` at the version pinned in `Cargo.toml` into `.tools`, and installs Playwright's Chromium.

## Build and serve

```sh
just build   # release wasm of both modules, then wasm-bindgen into web/pkg
just serve   # http://127.0.0.1:8137/ (cargo run -p cityloom_server)
```

`just build` runs `scripts/build.sh`. It builds the editor and the OpenStreetMap reader as two WebAssembly modules,
turns them into JavaScript glue with `wasm-bindgen` in `web/pkg` (not kept in git), and copies the map's libraries into
`web/vendor`.

`just serve` runs `cargo run -p cityloom_server`, the Rust server. It serves `web/` and answers range requests, which
the basemap's PMTiles need. Set `E2E_PORT` to use another port. Without `DATABASE_URL` the account routes answer 503 and
the pages are signed out.

## Map data

```sh
just prepare
```

This runs `cityloom_prepare`, the repository's Rust tool (`cargo run -p cityloom_prepare -- prepare`; its arguments are
the recipe's). It downloads Geofabrik's Quebec extract into `data/` if it is not there (about 1.2 GB; `just prepare --refresh` takes
a newer one). It then cuts the metropolitan area's roads into tiles in `web/data/metro` (needs `osmium-tool`) and builds
the map page's basemap, `web/data/basemap/montreal.pmtiles`, with Planetiler (needs Java 21 or later). Last, it packs both into
one archive, `data/montreal.cityloom`, a tar with a SHA-256 index that the server unpacks at start-up. The tiles, the
basemap and the archive are ignored by git.

Nothing needs the road tiles: without them every place, the default one included, comes from Overpass. Without the
basemap, the map page says so and shows no map.

## Tests and formatting

```sh
just db            # a throwaway Postgres in Docker for the server's tests and the account spec
just test          # cargo test --workspace, and a wasm32 build of the code plain `cargo test` does not compile; needs `DATABASE_URL`
just e2e           # builds, then runs the Playwright tests in e2e/; the account spec needs `DATABASE_URL`
just check         # both
just format        # rustfmt and Prettier
just format-check  # the same, without changing anything
```

CI runs `just format-check`, `just test` and `just e2e` with `RUSTFLAGS=-D warnings`, so a new compiler warning fails
it, and builds this documentation with `just docs-build`. CI's `test` and `e2e` jobs run a Postgres service. Every pull
request raises the version in the root `Cargo.toml`.

`just db` starts the Postgres in a container named `cityloom-db` on 127.0.0.1:55432 and prints the line to run:
`export DATABASE_URL=postgres://postgres:postgres@127.0.0.1:55432/cityloom`. The server's tests and the account spec
(`e2e/tests/account-server.spec.ts`) need it, and say so when it is missing. `just db-stop` removes the container.

## Accounts and deployment

The server reads its settings from the environment; `.env.example` lists every variable with how to make a key. The
account side needs `BASE_URL`, `DATABASE_URL`, `COOKIE_KEY`, `DATA_KEYS`, `DATA_KEY_CURRENT` and `LOOKUP_KEY`; a
provider is offered only when all of its variables are set. Without the account variables the site is served and
`/auth/*` and `/api/*` answer 503. `/api/work` has its own rate limit, `WORK_RATE_PERIOD_SECS` (default 1) and
`WORK_RATE_BURST` (default 60), apart from the sign-in routes' `AUTH_RATE_*`.

Register the redirect URL `https://cityloom.app/auth/{osm|google|microsoft|apple}/callback` with each provider.

Back up `DATA_KEYS` and `LOOKUP_KEY` in a password manager: losing them loses all the data. To rotate a master key, in this
order:

1. Add the new key to `DATA_KEYS` (keep the old one) and set `DATA_KEY_CURRENT` to the new id. Deploy, and wait until
   the server runs with these values.
2. Run `cityloom_server rewrap`. It prints `resealed N data keys`. Run it again until it prints
   `resealed 0 data keys`.
3. Only then remove the old key from `DATA_KEYS`, and deploy.

The order matters: a server still running with the old current key keeps wrapping new users under the old key, and
`rewrap` does not see users created after it started. Removing the old key then leaves those accounts impossible to
open, and their data is lost. On Railway, open a shell in the running service with `railway ssh` and run
`/app/cityloom_server rewrap` (the image's `WORKDIR` is `/app`; the service's variables are already in the
environment). To rotate the lookup key, set `OLD_LOOKUP_KEY` to the old one, `LOOKUP_KEY` to the new one, stop the server and
run `cityloom_server rehash`: a sign-in during the run could create a duplicate account. It prints how many identities
it checked, how many it rehashed and how many were already on the new key.

`just deploy` builds the wasm and the server for Linux (`x86_64-unknown-linux-musl`), refuses a server that holds the
fake provider and uploads `web/` and the binary to Railway. It needs `just setup-deploy` (cargo-zigbuild and the musl
target) and zig (`brew install zig` or `python3 -m pip install ziglang`). `just check-no-fake` runs the same check on a
release build, as CI does.

Deleting an account removes its rows at once. Copies in backups expire with Railway's retention.

### Manual checks, once per provider

Run these after registering a provider (see `.env.example` for its variables), with a browser that has no session.

1. Sign in from the map page. You land on the Account page, the storage choice is shown with nothing selected, and the
   header shows your name.
2. Choose where your work is kept, sign out, and sign in again: the same account, the choice kept.
3. Link a second provider, sign out, sign in with the second one: the same account. Unlink the first; the last sign-in
   has no Unlink button.
4. Cancel at the provider's consent page: the page says "Sign-in cancelled." and nothing was created.
5. Look at the server's logs: they hold paths, statuses, providers and outcomes, and no email, token or cookie.
6. Delete the account from its page: the confirmation is in the page, not a dialog; afterwards signing in with the same
   provider asks for the storage choice again.
7. On Railway, confirm that the rightmost entry of `X-Forwarded-For` is the client's address: the rate limit counts
   that entry. From a shell on your machine run
   `for i in $(seq 1 25); do curl -s -o /dev/null -w '%{http_code} ' -H "X-Forwarded-For: 10.0.0.$i" https://cityloom.app/auth/google/start; done`
   (a different made-up leftmost address each time): the last codes must be 429 (burst 20), because the proxy appends
   your real address and that is the key. A request from another network (a phone on mobile data) at the same moment
   must still be answered 303. If the 429 never comes, or the other network is limited too, Railway's proxy does not
   append the address the way the server assumes; stop and look at `client_address` in `cityloom_server/src/app.rs`.
8. Choose “Store my work on the server” on the Account page, edit a street, and open the site on a second device signed
   in to the same account: the street shows the edit after the tab is shown again.

Apple: sign in, sign out, and sign in again without removing the app from your Apple ID: the name stays (Apple sends it
only the first time). Hide-my-email gives a relay address as the label. The token exchange must succeed with the client
secret in the body (`client_secret_post`), and Apple must accept `openid` in the scope.
Microsoft: sign in once with a personal account and once with a work or school account: two separate identities.
Google: until the consent screen is verified, Google shows an "unverified app" warning; that is expected.
Key rotation: `rewrap` only after the server runs on the new current key, repeated until it prints `resealed 0 data keys`, and the old key removed last; `rehash` needs the server stopped (see above). Check the printed line.

## This documentation

```sh
just setup-docs  # install this documentation's packages (once)
just docs        # serve the site with live reload
just docs-build  # build both languages into website/build
```

The English pages are in `website/docs/` and the French ones in
`website/i18n/fr-CA/docusaurus-plugin-content-docs/current/`. A page exists in both languages.
