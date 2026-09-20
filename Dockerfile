# osm-extract-proxy (U9) — infrastructure-specification.md ID-2.
#
# Three stages: build the binary from source; fetch and verify the pinned
# data release the committed pointer (data/current-build.toml) names;
# assemble the minimal runtime image. The data stage's only input from the
# source tree is the pointer file, so a code-only change never invalidates
# its layer and a code-only deploy reuses the cached ~500 MB download.

# ---- Stage 1: build the binary ----------------------------------------
FROM rust:1.97.1-bookworm AS builder
WORKDIR /workspace
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates crates
RUN cargo build --release --locked --bin osm-extract-proxy

# ---- Stage 2: fetch and verify the pinned data release -----------------
FROM debian:bookworm-slim AS data
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /data
# Only the pointer file crosses into this stage from the source tree, so
# this layer is cached across every code-only deploy.
COPY data/current-build.toml /tmp/current-build.toml
RUN set -eu; \
    release_tag=$(sed -n 's/^release_tag = "\(.*\)"$/\1/p' /tmp/current-build.toml); \
    store_sha256=$(sed -n 's/^store_sha256 = "\(.*\)"$/\1/p' /tmp/current-build.toml); \
    manifest_sha256=$(sed -n 's/^manifest_sha256 = "\(.*\)"$/\1/p' /tmp/current-build.toml); \
    base="https://github.com/${GITHUB_REPOSITORY:-cityloom/cityloom}/releases/download/${release_tag}"; \
    curl -fsSL -o store.bin "${base}/store.bin"; \
    curl -fsSL -o manifest.json "${base}/manifest.json"; \
    echo "${store_sha256}  store.bin" | sha256sum -c -; \
    echo "${manifest_sha256}  manifest.json" | sha256sum -c -

# ---- Stage 3: the runtime image -----------------------------------------
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --no-create-home --uid 10001 cityloom
COPY --from=builder /workspace/target/release/osm-extract-proxy /usr/local/bin/osm-extract-proxy
COPY --from=data /data/store.bin /data/manifest.json /data/
RUN chown -R cityloom:cityloom /data && chmod 0444 /data/store.bin /data/manifest.json
USER cityloom
ENV CITYLOOM_STORE_PATH=/data/store.bin
ENV CITYLOOM_MANIFEST_PATH=/data/manifest.json
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/osm-extract-proxy"]
