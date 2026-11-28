# The browser tests (e2e/, Playwright): the Rust image (dockerfiles/rust.Dockerfile, which `just e2e` builds the wasm
# and the server with) plus Node with npm and Chromium with its system libraries, with the checkout mounted at /src
# (see dockerfiles/README.md). Nothing of the checkout is built into the image.
# `rust` is the Rust image, given as a build context: dockerfiles/docker-bake.hcl builds it first, or
# `docker build --build-context rust=docker-image://cityloom-rust …` takes one already built.
# Built from the repository root, which only e2e/package-lock.json is read from (e2e.Dockerfile.dockerignore).
FROM node:lts-slim AS node

FROM rust
# The whole of Node, npm included; the node image keeps npm and npx as links into its node_modules.
COPY --from=node /usr/local/bin/node /usr/local/bin/node
COPY --from=node /usr/local/lib/node_modules /usr/local/lib/node_modules
RUN ln -s ../lib/node_modules/npm/bin/npm-cli.js /usr/local/bin/npm \
    && ln -s ../lib/node_modules/npm/bin/npx-cli.js /usr/local/bin/npx
# A run as the host's user (`--user`) has no writable home for npm's cache.
ENV npm_config_cache=/tmp/npm-cache
# Chromium at the build the locked Playwright expects, outside the checkout so that `npm ci` in e2e/ finds it,
# with the libraries it needs (`--with-deps`). The version comes from e2e/package-lock.json, so the two cannot drift.
ENV PLAYWRIGHT_BROWSERS_PATH=/ms-playwright
COPY e2e/package-lock.json /tmp/e2e-package-lock.json
RUN version=$(node -p 'require("/tmp/e2e-package-lock.json").packages["node_modules/@playwright/test"].version') \
    && npx -y "playwright@$version" install --with-deps chromium \
    && rm -rf /tmp/e2e-package-lock.json /tmp/npm-cache /var/lib/apt/lists/* \
    && chmod -R a+rX "$PLAYWRIGHT_BROWSERS_PATH"
# The README mounts a volume over e2e/node_modules too (the host's packages may be another platform's); writable by any user.
RUN mkdir -p /src/e2e/node_modules && chmod 1777 /src/e2e/node_modules
