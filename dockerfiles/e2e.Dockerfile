# The browser tests (tests/e2e/, Playwright): the Rust image (dockerfiles/rust.Dockerfile, which `just e2e` builds the wasm
# and the server with, and which has Node and npm) plus Chromium with its system libraries, with the checkout mounted
# at /src (see dockerfiles/README.md). Nothing of the checkout is built into the image.
# `rust` is the Rust image, given as a build context: dockerfiles/docker-bake.hcl builds it first, or
# `docker build --build-context rust=docker-image://cityloom-rust …` takes one already built.
# Built from the repository root, which only tests/e2e/package-lock.json is read from (e2e.Dockerfile.dockerignore).

# The locked Playwright version alone, as a one-line file: the Chromium install below is keyed on it, not on the
# whole lockfile, which changes with every e2e dependency.
FROM rust AS pin
COPY tests/e2e/package-lock.json /package-lock.json
RUN node -e ' \
      const entry = require("/package-lock.json").packages?.["node_modules/@playwright/test"]; \
      if (!entry?.version) { console.error("no @playwright/test version in tests/e2e/package-lock.json: the image cannot pin Chromium"); process.exit(1); } \
      require("fs").writeFileSync("/playwright-version", entry.version);'

FROM rust
# Chromium at the build the locked Playwright expects, outside the checkout so that `npm ci` in tests/e2e/ finds it,
# with the libraries it needs (`--with-deps`). Rebuild the image when tests/e2e/package-lock.json moves Playwright.
ENV PLAYWRIGHT_BROWSERS_PATH=/ms-playwright
COPY --from=pin /playwright-version /tmp/playwright-version
RUN npx -y "playwright@$(cat /tmp/playwright-version)" install --with-deps chromium \
    && rm -rf /tmp/playwright-version /tmp/npm-cache /var/lib/apt/lists/* \
    && chmod -R a+rX "$PLAYWRIGHT_BROWSERS_PATH"
# The README mounts a volume over tests/e2e/node_modules too (the host's packages may be another platform's); writable by any user.
RUN mkdir -p /src/tests/e2e/node_modules && chmod 1777 /src/tests/e2e/node_modules
