#!/usr/bin/env bash
# The single gate: run this before claiming any work is complete.
# Add formatting, linting, and test steps here as the project grows.
set -euo pipefail

cd "$(dirname "$0")/.."

echo "== verify: no checks configured yet =="
echo "Add formatting, lint, and test commands to scripts/verify.sh as the project grows."
