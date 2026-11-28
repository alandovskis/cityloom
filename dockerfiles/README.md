# Build images

Four toolchain images, so that CityLoom can be built and tested without installing its tools on the host. Each one
holds tools only. The checkout is mounted at `/src` and the usual commands run inside the container. `deploy/Dockerfile`
is something else: it is the runtime image that `just deploy` uploads.

| Image           | Dockerfile        | What it holds                                                                                                                                                                                                | For                                                          |
| --------------- | ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------ |
| `cityloom-rust` | `rust.Dockerfile` | Rust stable, the `wasm32-unknown-unknown` target, rustfmt, clippy, `wasm-bindgen-cli` (the version pinned in `crates/cityloom_editor/Cargo.toml`), `just`, procps (`kill`, for two tests), Node LTS with npm | `just build`, `just test`, `just format-check`'s `cargo fmt` |
| `cityloom-js`   | `js.Dockerfile`   | Node LTS                                                                                                                                                                                                     | `npm ci` at the root (MapLibre), `scripts/precompress.mjs`   |
| `cityloom-docs` | `docs.Dockerfile` | Node LTS, Python 3                                                                                                                                                                                           | the documentation site in `website/`                         |
| `cityloom-e2e`  | `e2e.Dockerfile`  | everything in `cityloom-rust`, plus Chromium at the build that the locked Playwright expects                                                                                                                 | `just e2e` and its tiers                                     |

The Rust and Node versions follow CI: `rust:1-slim` is the latest stable and `node:lts-slim` is the current LTS. When
you rebuild an image you get the newest of each. Run `docker buildx bake --pull` to fetch new base images.

Two versions are read from the checkout when an image is built, not when it runs: the `wasm-bindgen` line of
`crates/cityloom_editor/Cargo.toml` and `@playwright/test` in `e2e/package-lock.json`. After a pull that changes either
one, build the images again. Otherwise `just build` fails on a wasm-bindgen version mismatch, or Playwright reports that
its browser executable does not exist. The rebuild redoes only the steps that use the changed version.

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

The host's `node_modules`, `.tools` and `target` can hold files built for another platform. On macOS, for example,
`wasm-bindgen` in `.tools/bin` is a macOS binary, and `build.sh` puts `.tools/bin` first on its `PATH`. So the runs below
mount named volumes over these directories, and Cargo builds into a volume at `/target`. The volumes keep the
container's own copies from one run to the next, so the host's files are never touched.

Every checkout is mounted at the same `/src`. If two checkouts shared a target volume, Cargo could take one's build for
the other's. So each checkout's volumes are named after its folder and a checksum of its path, for example
`cityloom-main-4046072687-target`. The name says which checkout a volume belongs to, and the checksum keeps apart two
checkouts whose folders have the same name. Set the prefix once in the shell, at the checkout's root:

```bash
v=cityloom-$(basename "$PWD" | tr -c 'A-Za-z0-9_.\n-' '-')-$(pwd -P | cksum | cut -d ' ' -f 1)
```

The exception is `cityloom-cargo`, which every checkout shares, even when several containers run at once. It holds
Cargo's downloads (the registry and the git dependencies) along with the locks that guard them. A checkout's target
volume is large: about 45 GB after `just test` and `just e2e-ui`. Before you remove a worktree, remove its volumes. Run
this from the worktree's root, with `v` set as above:

```bash
docker volume ls -q --filter "name=^$v-" | xargs docker volume rm
```

`--user` makes the files the container writes into the checkout belong to you. You need it on Linux. On macOS it does
no harm.

Build the pages. This runs `npm ci` in the JS image, then the wasm build in the Rust image:

```bash
docker run --rm --user "$(id -u):$(id -g)" -v "$PWD":/src -v "$v-node-modules":/src/node_modules cityloom-js npm ci
```

```bash
docker run --rm --user "$(id -u):$(id -g)" -v "$PWD":/src -v "$v-node-modules":/src/node_modules -v "$v-tools":/src/.tools -v "$v-target":/target -v cityloom-cargo:/cargo cityloom-rust just build
```

Build the documentation site in English and fr-CA:

```bash
docker run --rm --user "$(id -u):$(id -g)" -v "$PWD":/src -v "$v-website-node-modules":/src/website/node_modules -w /src/website cityloom-docs sh -c 'npm ci && npm run build'
```

Serve the documentation site on http://localhost:3000/. As with `just docs`, the port is open on the host's loopback
only:

```bash
docker run --rm -it --user "$(id -u):$(id -g)" -p 127.0.0.1:3000:3000 -v "$PWD":/src -v "$v-website-node-modules":/src/website/node_modules -w /src/website cityloom-docs npm run start -- --host 0.0.0.0
```

Run the UI tier of the browser tests. Run the JS image's `npm ci` from above first, because the build copies MapLibre
from the root's `node_modules`. `just e2e*` also writes into the checkout's `target/`: the pinned server goes in
`target/e2e` and the archive in `target/e2e-archive`. So one more volume covers the checkout's `target/`:

```bash
docker run --rm --user "$(id -u):$(id -g)" -v "$PWD":/src -v "$v-node-modules":/src/node_modules -v "$v-e2e-node-modules":/src/e2e/node_modules -v "$v-tools":/src/.tools -v "$v-target":/target -v "$v-checkout-target":/src/target -v cityloom-cargo:/cargo cityloom-e2e sh -c 'cd e2e && npm ci && cd .. && just e2e-ui'
```

The integration tier, and `just test` for the server's tests, need a Postgres. Start one on the host with `just db`. It
listens on the host's 127.0.0.1 only, so how the container reaches it depends on the platform:

- macOS and Windows (Docker Desktop): change the URL's host part to `host.docker.internal`, for example
  `-e DATABASE_URL=postgres://postgres:postgres@host.docker.internal:55432/cityloom`.
- Linux: add `--network host` and pass the URL that `just db` prints, unchanged.

`just test` needs more memory than Docker Desktop's default 8 GB when Cargo builds in parallel. Compiling the editor's
tests and linking the test binaries get killed for lack of memory, even with three jobs. Either give the Docker VM more
memory, or run one job at a time with `-e CARGO_BUILD_JOBS=1`.
