#!/usr/bin/env bash
# The single gate. Exit 0 means: formatted, lint-clean, builds (native + wasm),
# and every test passes. Nothing else in this repo is authoritative about
# "done".
#
# Output contract:
#   - a passing stage prints its name and, only if the whole run succeeds, a
#     short tail (<= TAIL_OK lines, capped at 40)
#   - a failing stage prints its name and the last TAIL_FAIL lines of its log,
#     immediately, since that is the terminal state of the run
#   - passing-stage log content is never printed when a later stage fails —
#     it is buffered and discarded, not streamed live, because streaming live
#     would leak it onto the terminal before the eventual failure is known
#   - a summary block always names every stage and its state
#
# Stages fail fast: the first failure stops the run, so the feedback loop stays
# short. Remaining stages are reported as "skipped", never as passing.

set -uo pipefail

cd "$(dirname "$0")/.."
REPO_ROOT="$PWD"

TAIL_OK="${CITYLOOM_TAIL_OK:-8}"
TAIL_FAIL="${CITYLOOM_TAIL_FAIL:-60}"

LOG_DIR="$(mktemp -d "${TMPDIR:-/tmp}/cityloom-verify.XXXXXX")"
trap 'rm -rf "$LOG_DIR"' EXIT

STAGE_NAMES=()
STAGE_STATES=()
STAGE_TAILS=()
FAILED_STAGE=""

bold()  { printf '\033[1m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }
red()   { printf '\033[31m%s\033[0m\n' "$*"; }
dim()   { printf '\033[2m%s\033[0m\n' "$*"; }

# run <name> <command...>
#
# Captures all output to a per-stage log. A passing stage's tail is buffered,
# not printed, until the whole run's outcome is known — printing it live would
# leak it even on runs where a later stage goes on to fail.
run() {
    local name="$1"; shift
    local log="$LOG_DIR/$name.log"

    if [[ -n "$FAILED_STAGE" ]]; then
        STAGE_NAMES+=("$name"); STAGE_STATES+=("skipped"); STAGE_TAILS+=("")
        return 0
    fi

    printf '\033[1m==> %s\033[0m\n' "$name"
    if "$@" >"$log" 2>&1; then
        STAGE_NAMES+=("$name"); STAGE_STATES+=("pass")
        STAGE_TAILS+=("$(tail -n "$TAIL_OK" "$log")")
        green "    ok"
    else
        local code=$?
        STAGE_NAMES+=("$name"); STAGE_STATES+=("fail"); STAGE_TAILS+=("")
        FAILED_STAGE="$name"
        red "    FAILED (exit $code) — last $TAIL_FAIL lines:"
        tail -n "$TAIL_FAIL" "$log" | sed 's/^/    /'
    fi
}

# --- Rust -------------------------------------------------------------------

run fmt    cargo fmt --all --check
run clippy cargo clippy --workspace --all-targets --all-features -- -D warnings
run build  cargo build --workspace --all-targets
run wasm   cargo build -p cityloom-client --target wasm32-unknown-unknown
run test   cargo nextest run --workspace --status-level fail

# --- Python harness ---------------------------------------------------------
#
# Integration and e2e tests are deselected unless their infrastructure is up.
# CITYLOOM_FULL=1 (used in CI) removes the deselection so the gate covers them.

PYTEST_ARGS=(-q)
if [[ "${CITYLOOM_FULL:-0}" != "1" ]]; then
    PYTEST_ARGS+=(-m "not integration and not e2e")
fi

run py-lint uv run --frozen ruff check .
run py-fmt  uv run --frozen ruff format --check .
run py-test uv run --frozen pytest "${PYTEST_ARGS[@]}"

# --- Stage output (only ever shown once the whole run has succeeded) -------

if [[ -z "$FAILED_STAGE" ]]; then
    for i in "${!STAGE_NAMES[@]}"; do
        [[ -n "${STAGE_TAILS[$i]}" ]] || continue
        printf '\033[1m==> %s\033[0m\n' "${STAGE_NAMES[$i]}"
        printf '%s\n' "${STAGE_TAILS[$i]}" | sed 's/^/    /'
    done
fi

# --- Summary ----------------------------------------------------------------

echo
bold "verify summary"
for i in "${!STAGE_NAMES[@]}"; do
    case "${STAGE_STATES[$i]}" in
        pass)    printf '  \033[32m%-8s\033[0m %s\n' "pass"    "${STAGE_NAMES[$i]}" ;;
        fail)    printf '  \033[31m%-8s\033[0m %s\n' "FAIL"    "${STAGE_NAMES[$i]}" ;;
        skipped) printf '  \033[2m%-8s %s\033[0m\n'  "skipped" "${STAGE_NAMES[$i]}" ;;
    esac
done

if [[ -n "$FAILED_STAGE" ]]; then
    echo
    red "verify FAILED at stage: $FAILED_STAGE"
    dim "reproduce just that stage, then re-run: just verify"
    exit 1
fi

if [[ "${CITYLOOM_FULL:-0}" != "1" ]]; then
    echo
    dim "note: integration and e2e tests were deselected."
    dim "      run 'just verify-full' with Postgres up to include them."
fi

echo
green "verify PASSED"
exit 0
