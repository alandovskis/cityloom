#!/bin/sh
# Fails if the server binary given was built with the fake OpenID provider (--features test-provider): the
# provider signs in anyone, so it must never be in a build that is deployed.
set -eu
[ "$#" -eq 1 ] && [ -f "$1" ] || { echo "usage: $0 <cityloom_server binary>" >&2; exit 2; }
for marker in fake-login fake-apple; do
  if grep -aq -- "$marker" "$1"; then
    echo "$1 holds the fake provider ($marker): it was built with --features test-provider" >&2
    exit 1
  fi
done
echo "$1 has no fake provider"
