# The Rust toolchain: what `just build`, `just test`, `just format-check` and the server need, with the checkout
# mounted at /src (see dockerfiles/README.md). Nothing of the checkout is built into the image.
# Built from the repository root, which only the pinned wasm-bindgen version is read from
# (rust.Dockerfile.dockerignore keeps everything else out of the context).
FROM node:lts-slim AS node

FROM rust:1-slim
# Stable, as CI's `dtolnay/rust-toolchain` takes it, with the wasm target and the components the recipes use.
RUN rustup target add wasm32-unknown-unknown \
    && rustup component add rustfmt clippy
# wasm-bindgen-cli must match the wasm-bindgen crate exactly, so its version comes from the editor's Cargo.toml,
# as the justfile's `setup` reads it.
COPY crates/cityloom_editor/Cargo.toml /tmp/editor-Cargo.toml
RUN version=$(sed -n 's/^wasm-bindgen = "\(.*\)"/\1/p' /tmp/editor-Cargo.toml) \
    && test -n "$version" \
    && cargo install wasm-bindgen-cli --version "$version" --locked \
    && cargo install just --locked \
    && rm -rf /tmp/editor-Cargo.toml "$CARGO_HOME/registry" "$CARGO_HOME/git"
# scripts/target-dir.sh and scripts/precompress.mjs run on Node; the node binary alone is enough for them.
COPY --from=node /usr/local/bin/node /usr/local/bin/node
# The build's output stays out of the mounted checkout, so it never mixes with the host's own target/ (another
# platform's binaries); mount a volume here to keep it between runs.
ENV CARGO_TARGET_DIR=/target
# A run as the host's user (`--user`) must still write cargo's registry and the target directory. The checkout's
# node_modules and .tools hold the host's own packages and binaries (another platform's), so the README mounts volumes
# over them; a volume takes its owner from the image's directory, so these are writable by any user too.
RUN mkdir -p "$CARGO_TARGET_DIR" /src/node_modules /src/.tools \
    && chmod -R a+rwX "$CARGO_HOME" \
    && chmod 1777 "$CARGO_TARGET_DIR" /src/node_modules /src/.tools
WORKDIR /src
