# The Rust toolchain: what `just build`, `just test`, `just format-check` and the server need, with the checkout
# mounted at /src (see dockerfiles/README.md). Nothing of the checkout is built into the image.
# Built from the repository root, which only the pinned wasm-bindgen version is read from
# (rust.Dockerfile.dockerignore keeps everything else out of the context).
FROM node:lts-slim AS node

# The wasm-bindgen pin alone, as a one-line file: the install below is keyed on it, not on the whole Cargo.toml,
# whose `version` every pull request raises.
FROM debian:stable-slim AS pin
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
# justfile's `setup` reads it. A run as the host's user (`--user`) must still write cargo's registry: the mode
# is set in this layer, so the binaries are not copied again into another.
COPY --from=pin /wasm-bindgen-version /tmp/wasm-bindgen-version
RUN cargo install wasm-bindgen-cli --version "$(cat /tmp/wasm-bindgen-version)" --locked \
    && cargo install just --locked \
    && rm -rf /tmp/wasm-bindgen-version "$CARGO_HOME/registry" "$CARGO_HOME/git" \
    && mkdir "$CARGO_HOME/registry" \
    && chmod -R a+rwX "$CARGO_HOME"
# scripts/target-dir.sh and scripts/precompress.mjs run on Node; the node binary alone is enough for them.
COPY --from=node /usr/local/bin/node /usr/local/bin/node
# Cargo's output stays out of the mounted checkout, so it never mixes with the host's own target/ (another
# platform's binaries). The checkout's target/ (where `just e2e*` pins the server), node_modules and .tools hold the
# host's own files too, so the README mounts volumes over them all; a volume takes its owner from the image's
# directory, so these are writable by any user.
ENV CARGO_TARGET_DIR=/target
RUN mkdir -p "$CARGO_TARGET_DIR" /src/target /src/node_modules /src/.tools \
    && chmod 1777 "$CARGO_TARGET_DIR" /src/target /src/node_modules /src/.tools
WORKDIR /src
