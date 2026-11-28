# Node for the pages' JavaScript: `npm ci` at the root (MapLibre, which scripts/build.sh copies into web/vendor) and
# `node scripts/precompress.mjs`, with the checkout mounted at /src (see dockerfiles/README.md).
# Node LTS, as CI's `node-version: lts/*`. Nothing of the checkout is built into the image.
FROM node:lts-slim
# A run as the host's user (`--user`) has no writable home for npm's cache.
ENV npm_config_cache=/tmp/npm-cache
# The README mounts a volume over the checkout's node_modules (the host's packages may be another platform's); a volume
# takes its owner from the image's directory, so this one is writable by any user.
RUN mkdir -p /src/node_modules && chmod 1777 /src/node_modules
WORKDIR /src
