# Build images

Four toolchain images, so that CityLoom can be built and tested without installing its tools on the host. Each one
holds tools only. The checkout is mounted at `/src` and the usual commands run inside the container. `deploy/Dockerfile`
is something else: it is the runtime image that `just deploy` uploads.

| Image           | Dockerfile        | What it holds                                                                                                                                                                                | For                                                          |
| --------------- | ----------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| `cityloom-rust` | `rust.Dockerfile` | Rust stable, the `wasm32-unknown-unknown` target, rustfmt, clippy, `wasm-bindgen-cli` (the version pinned in `crates/cityloom_editor/Cargo.toml`), `just`, `node` (the scripts' helper only) | `just build`, `just test`, `just format-check`'s `cargo fmt` |
| `cityloom-js`   | `js.Dockerfile`   | Node LTS                                                                                                                                                                                     | `npm ci` at the root (MapLibre), `scripts/precompress.mjs`   |
| `cityloom-docs` | `docs.Dockerfile` | Node LTS, Python 3                                                                                                                                                                           | the documentation site in `website/`                         |
| `cityloom-e2e`  | `e2e.Dockerfile`  | everything in `cityloom-rust`, plus Node LTS with npm, and Chromium at the build that the locked Playwright expects                                                                          | `just e2e` and its tiers                                     |

The Rust and Node versions follow CI: `rust:1-slim` is the latest stable and `node:lts-slim` is the current LTS. When
you rebuild an image you get the newest of each. Run `docker buildx bake --pull` to fetch new base images.

## Building

From the repository root:

```bash
docker buildx bake -f dockerfiles/docker-bake.hcl
```

This builds all four images, and builds `cityloom-rust` before `cityloom-e2e`, which starts from it. To build one image,
add its name after the command, for example `docs`. To build the e2e image on its own with `docker build`, point it at a
Rust image that is already built:
`docker build --build-context rust=docker-image://cityloom-rust -f dockerfiles/e2e.Dockerfile -t cityloom-e2e .`

## Running

The host's `node_modules`, `.tools` and `target` can hold binaries built for another platform. On macOS, for example,
`wasm-bindgen` in `.tools/bin` is a macOS binary, and `build.sh` puts `.tools/bin` first on its `PATH`. So the runs below
mount named volumes over these directories. The volumes keep the container's own copies from one run to the next. Cargo
builds into `/target`, never into the checkout's `target/`.

`--user` makes the files the container writes into the checkout belong to you. You need it on Linux. On macOS it does
no harm.

Build the pages. This runs `npm ci` in the JS image, then the wasm build in the Rust image:

```bash
docker run --rm --user "$(id -u):$(id -g)" -v "$PWD":/src -v cityloom-node-modules:/src/node_modules cityloom-js npm ci
```

```bash
docker run --rm --user "$(id -u):$(id -g)" -v "$PWD":/src -v cityloom-node-modules:/src/node_modules -v cityloom-tools:/src/.tools -v cityloom-target:/target cityloom-rust just build
```

Build the documentation site in English and fr-CA:

```bash
docker run --rm --user "$(id -u):$(id -g)" -v "$PWD":/src -v cityloom-website-node-modules:/src/website/node_modules -w /src/website cityloom-docs sh -c 'npm ci && npm run build'
```

Serve the documentation site on http://localhost:3000/:

```bash
docker run --rm -it --user "$(id -u):$(id -g)" -p 3000:3000 -v "$PWD":/src -v cityloom-website-node-modules:/src/website/node_modules -w /src/website cityloom-docs npm run start -- --host 0.0.0.0
```

Run the UI tier of the browser tests. Run the JS image's `npm ci` from above first, because the build copies MapLibre
from the root's `node_modules`:

```bash
docker run --rm --user "$(id -u):$(id -g)" -v "$PWD":/src -v cityloom-node-modules:/src/node_modules -v cityloom-e2e-node-modules:/src/e2e/node_modules -v cityloom-tools:/src/.tools -v cityloom-target:/target cityloom-e2e sh -c 'cd e2e && npm ci && cd .. && just e2e-ui'
```

The integration tier, and `just test` for the server's tests, need a Postgres. Start one on the host with `just db`. It
listens on the host's 127.0.0.1 only, so how the container reaches it depends on the platform:

- macOS and Windows (Docker Desktop): change the URL's host part to `host.docker.internal`, for example
  `-e DATABASE_URL=postgres://postgres:postgres@host.docker.internal:55432/cityloom`.
- Linux: add `--network host` and pass the URL that `just db` prints, unchanged.
