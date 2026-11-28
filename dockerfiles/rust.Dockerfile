# The Rust toolchain: what `scripts/build.sh`, `just test`, `cargo fmt` and the server need, with the checkout
# mounted at /src (see dockerfiles/README.md). Nothing of the checkout is built into the image, and it has no Node:
# the compressed copies that `just build` makes after build.sh (scripts/precompress.mjs) are made in the JS image.
# Built from the repository root, which only the pinned wasm-bindgen version is read from
# (rust.Dockerfile.dockerignore keeps everything else out of the context).

# The wasm-bindgen pin alone, as a one-line file: the install below is keyed on it, not on the whole Cargo.toml,
# whose `version` every pull request raises.
FROM rust:1-slim AS pin
COPY crates/cityloom_editor/Cargo.toml /Cargo.toml
RUN sed -n 's/^wasm-bindgen = "\(.*\)"/\1/p' /Cargo.toml > /wasm-bindgen-version \
    && [ -s /wasm-bindgen-version ] \
    || { echo "no 'wasm-bindgen = \"<version>\"' line in crates/cityloom_editor/Cargo.toml: the image cannot pin wasm-bindgen-cli" >&2; exit 1; }

FROM rust:1-slim
# procps: two tests send a signal through an external `kill` (the server's SIGTERM, citymodel_builder's Ctrl-C),
# which the slim image does not have.
RUN apt-get update \
    && apt-get install -y --no-install-recommends procps \
    && rm -rf /var/lib/apt/lists/*
# Stable, as CI's `dtolnay/rust-toolchain` takes it, with the wasm target and the components the recipes use.
RUN rustup target add wasm32-unknown-unknown \
    && rustup component add rustfmt clippy
# wasm-bindgen-cli must match the wasm-bindgen crate exactly, so its version is the editor's pin, as the
# justfile's `setup` reads it. Both tools go in /usr/local/bin, outside CARGO_HOME (below).
COPY --from=pin /wasm-bindgen-version /tmp/wasm-bindgen-version
RUN cargo install wasm-bindgen-cli --version "$(cat /tmp/wasm-bindgen-version)" --locked --root /usr/local \
    && cargo install just --locked --root /usr/local \
    && rm -rf /tmp/wasm-bindgen-version /usr/local/.crates.toml /usr/local/.crates2.json \
        "$CARGO_HOME/registry" "$CARGO_HOME/git" "$CARGO_HOME"/.package-cache* "$CARGO_HOME"/.global-cache
# Cargo's downloads (the registry, the git dependencies) and the locks that guard them, in one directory, so that
# one volume shares them all between containers that run at once; the toolchain's proxies stay in
# /usr/local/cargo/bin, on the PATH. Cargo's output goes to /target, out of the mounted checkout, so it never mixes
# with the host's own target/ (another platform's binaries). The checkout's target/ (where `just e2e*` pins the
# server), node_modules (which build.sh copies MapLibre from) and .tools hold the host's own files too, so the README
# mounts volumes over them all; a volume takes its owner from the image's directory, so these are writable by any
# user (`--user`).
ENV CARGO_HOME=/cargo CARGO_TARGET_DIR=/target
RUN mkdir -p "$CARGO_HOME" "$CARGO_TARGET_DIR" /src/target /src/node_modules /src/.tools \
    && chmod 1777 "$CARGO_HOME" "$CARGO_TARGET_DIR" /src/target /src/node_modules /src/.tools
WORKDIR /src
