#!/bin/sh
# Copies the test-provider server just built into target/e2e, for the browser tests of this checkout (`just e2e`,
# `just e2e-archive`; running Playwright by hand needs this after every `cargo build`, or it starts an older copy). Cargo's
# target directory may be shared with other checkouts (see target-dir.sh): their builds replace <target>/debug/cityloom_server,
# and a suite that runs for minutes must not start whichever one came last. Run right after the build.
set -eu
cd "$(dirname "$0")/.."
dir=$(./scripts/target-dir.sh)
mkdir -p target/e2e
# Copy beside the destination and rename over it: a server still running from the old copy keeps its file.
cp "$dir/debug/cityloom_server" target/e2e/cityloom_server.new
grep -aq fake-login target/e2e/cityloom_server.new || {
  rm -f target/e2e/cityloom_server.new
  echo "pin-server.sh: $dir/debug/cityloom_server has no test provider: another checkout rebuilt it without --features test-provider; build again" >&2
  exit 1
}
mv -f target/e2e/cityloom_server.new target/e2e/cityloom_server
