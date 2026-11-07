#!/bin/sh
# Copies the test-provider server just built into target/e2e, for the browser tests of this checkout. Cargo's target
# directory may be shared with other checkouts (see target-dir.sh): their builds replace <target>/debug/cityloom_server,
# and a suite that runs for minutes must not start whichever one came last. Run right after the build.
set -eu
cd "$(dirname "$0")/.."
dir=$(./scripts/target-dir.sh)
mkdir -p target/e2e
cp "$dir/debug/cityloom_server" target/e2e/cityloom_server
