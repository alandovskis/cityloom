# The documentation site (website/, Docusaurus, English and fr-CA): its dev server, its build and what the
# Documentation workflow (.github/workflows/docs.yml) runs, with the checkout mounted at /src (see dockerfiles/README.md).
# Node LTS, as that workflow's `node-version: lts/*`. Nothing of the checkout is built into the image.
FROM node:lts-slim
# The workflow's Architecture pages test (scripts/include/tests/test_architecture_docs.py) runs on Python's standard library.
RUN apt-get update \
    && apt-get install -y --no-install-recommends python3 \
    && rm -rf /var/lib/apt/lists/*
# A run as the host's user (`--user`) has no writable home for npm's cache, nor for Docusaurus's update check,
# which would otherwise end every build with a box asking for sudo.
ENV npm_config_cache=/tmp/npm-cache NO_UPDATE_NOTIFIER=1
# The dev server: `npm run start -- --host 0.0.0.0` in website/.
EXPOSE 3000
# The README mounts a volume over the checkout's node_modules (the host's packages may be another platform's); a volume
# takes its owner from the image's directory, so this one is writable by any user.
RUN mkdir -p /src/website/node_modules && chmod 1777 /src/website/node_modules
WORKDIR /src
