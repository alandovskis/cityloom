#!/bin/sh
# Runs a command until it succeeds, at most ATTEMPTS times, waiting DELAY seconds after the first failure and twice as
# long after each one that follows; exits with the command's status once the last attempt fails.
#   scripts/retry.sh ATTEMPTS DELAY command [arg...]
set -eu
usage() { echo "usage: $0 ATTEMPTS DELAY command [arg...] (ATTEMPTS >= 1 and DELAY >= 0, whole numbers)" >&2; exit 2; }
[ "$#" -ge 3 ] || usage
attempts=$1
delay=$2
shift 2
case "$attempts" in '' | *[!0-9]*) usage ;; esac
case "$delay" in '' | *[!0-9]*) usage ;; esac
[ "$attempts" -ge 1 ] || usage
attempt=1
while :; do
  status=0
  "$@" || status=$?
  [ "$status" -ne 0 ] || exit 0
  if [ "$attempt" -ge "$attempts" ]; then
    echo "$1 failed (status $status) after $attempts attempts" >&2
    exit "$status"
  fi
  echo "$1: attempt $attempt of $attempts failed (status $status); trying again in ${delay}s" >&2
  sleep "$delay"
  attempt=$((attempt + 1))
  delay=$((delay * 2))
done
