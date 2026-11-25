# The software factory's tests flow, proved by mutation testing

CLM-100. Status: for a person's approval. Nothing is built before it.

## Why

The factory (`.claude/factory/README.md`) cannot build an issue that asks only for tests of code that already works. Its
red gate refuses a new test that passes at once, because such a test proves no change. The 23 gap issues of CLM-99
(CLM-76 to CLM-98) all ask for that, and so do later ones.

A test of working code needs another proof: that it would fail if the code were wrong. The person chose mutation
testing for it: a fault is put into the product code, and a new test must fail because of it.

## Goals

- The factory takes an issue that asks only for tests, from triage to a pull request, without a person in between.
- A script, not an agent's word, says that each new test can fail.
- A person who approves the pull request sees, for each new test, a fault that it caught.

## Non-goals

- Mutation testing in CI, or a mutation score for the product.
- Changing the code or docs flow.
- Finding every fault that no test catches. The gate lists those it meets and does not fail on them.

## Anti-goals

- A test that passes the gate by being sensitive to everything (a snapshot of the whole page). The reviewer still
  judges what a test asserts.
- A gate so slow that it holds the line: its time has a limit (below).
- A fault left in the worktree, committed, or pushed.

## Approaches

1. **`cargo-mutants` runs everything** (`cargo mutants --in-diff`, its own runner). Least code here. It cannot fault
   `view!` markup, `.ftl` or CSS, cannot run a browser test, copies the tree for each run, and says which fault was
   missed but not which test caught one.
2. **`cargo-mutants` only generates, and our gate runs** (chosen by the person). The gate applies each fault's diff,
   runs the new tests, and takes it back. For browser tests it rebuilds the wasm. It makes its own faults where
   `cargo-mutants` has none: markup, `.ftl`, CSS.
3. **The planner or the test-writer writes each fault by hand.** Needs no tool, but the agent that writes the tests
   could choose faults that suit them, and the proof becomes an agent's word again.

## What was measured

On this machine (macOS, CLM-100's worktree at `8edd3d32`), in one module of the editor (`junction/vm/keys.rs`) and the
junction's two specs:

| What | Time |
| --- | --- |
| First native test build, once for an issue | 138 s |
| A native fault: rebuild, then run the module's tests | 12 s and 4 s |
| First wasm build at optimisation level 1, once for an issue | 92 s |
| A browser fault: rebuild at level 1, then `wasm-bindgen` | 14 s and 1 s |
| The same in release (`just build`'s profile) | 50 s to 57 s, and 1 s |
| `junction` and `junction-a11y` specs, no fault: level 1, release | 191 s and 136 s (111 pass in both) |
| `junction-a11y` alone, with a fault, level 1 | 49 s |
| The same two specs on a development build (level 0) | 359 s, most tests fail: not usable |

`cargo-mutants` 27.1.0 lists 19,639 faults in `cityloom_editor` over 3,809 functions (median 2 a function, nine in ten
have at most 10, the largest 324) and 1,310 in `citymodel_builder`. It lists them in seconds, without building.

## The flow

Triage gives an issue the flow **tests** when it asks only for tests: no change to what the product does. An issue that
also needs a change to the product is **code**.

```
Todo ─ queue ─ triage ─ plan ─ test-writer ─ tests gate ─ review ─ evidence ─ PR ─ approval ─ merge ─ deploy
              (tests)  (tests)  └── factory-build ──┘
```

There is no red commit, no implementer and no `Factory-Red` trailer. Records: `**Factory · tests** <sha>` takes the
place of `green`, as `**Factory · docs**` does in the docs flow. Rounds: `rounds.green`.

**Plan.** Its headings: `## Change`, `## Where`, `## Tests`, `## Faults`, `## Version`, `## Out of scope`. `## Faults`
holds one fenced block, which the gate reads. The planner writes it, not the test-writer, so the one who writes the tests does not choose the faults. The
orchestrator passes it to the gate as a file it saved from the record:

```json
{
  "rust": [{ "file": "crates/cityloom_editor/src/shared/platform.rs", "functions": ["copy_text"] }],
  "markup": [{ "file": "crates/cityloom_editor/src/timeline/view.rs", "functions": ["question"] }],
  "ftl": [{ "file": "crates/cityloom_editor/src/timeline/i18n/en.ftl", "messages": ["timeline-title"] }],
  "css": [{ "file": "web/editor.css", "declarations": [{ "rule": ":root", "property": "--danger" }] }]
}
```

Every file and function named must exist at the base, or the plan is sent back. `rust` and `markup` may both name one
function. For a parity test of a whole file, `messages` may be `"*"`.

**Test-writer.** In this flow it writes tests that pass on today's code, adds no stub, raises the patch version and
`Cargo.lock` (a change under `crates/` or `e2e/` must raise it: `scripts/check-version.sh`), and runs `just format`. A new spec
goes in `tiers.json` and `spec-map.json`, and new test files in `features.json`, as today. It may change a guard's own
tests (`shared/contrast.rs`). It may look at neither the gate's fault list nor the faults' results: on a later round it is
told which of its tests caught nothing, not which faults there were.

**A test that fails on today's code** has found a bug. The test-writer leaves it out and reports it. The orchestrator
files it as an issue in Triage, linked to this one, and names it in the pull request's Unknowns. If nothing else is
left to test, the issue is handed back.

**Review.** As in the code flow. The reviewer is also asked whether a test asserts the behaviour the issue names, or
only something the faults happen to break.

**Evidence.** The gate's report, the new tests' own output at the head, and for browser tests the run's screenshots.
The pull request's Proof has a table: each new test, and one fault it caught.

## The tests gate

```sh
python3 "$F/gate.py" tests --repo <wt> --base "$BASE" --faults <file>
```

It refuses (exit 2) as the others do (dirty worktree, no database, no setup), and when `cargo-mutants` is missing
or not the pinned version. It fails (exit 1) unless all of these hold:

1. **Only tests changed.** Every changed file is a test file (`TEST_FILES`), `features.json`, the version line of
   `crates/cityloom_editor/Cargo.toml` or of `Cargo.lock`, documentation, or a `.rs` file whose only changes are in
   `#[cfg(test)]` items. A whole file is test code when its `mod` declaration is `#[cfg(test)]` (`shared/contrast.rs`).
2. **At least one new test**, found as the red gate finds them (listed at the head and not at the base). No test is gone, ignored or skipped.
3. **Every test passes** at the head, with warnings denied, as the green gate runs them (every Rust test, the wasm
   build, the changed specs), and `just format-check`, `scripts/check-version.sh` and `just test-scripts` pass.
4. **The new tests are steady**: each passes again in a second run by itself.
5. **Each new test catches at least one fault.**

The report lists each new test with the first three faults it caught and how many in all, then the faults no new test
caught, then the faults left out by the limit. Uncaught faults are information for the reviewer and the person: some
faults change nothing a person can see.

### The faults

- **rust**: `cargo mutants --list --json --diff` for the file, kept when the fault's function is named. Constructors or
  code that `cargo-mutants` skips have no faults: the plan then names a function that uses them.
- **markup**: in the `view!` of a named function: remove one attribute; change one string literal; swap two neighbouring elements of the same
  parent. Made by the gate with its own tokenizer (`_code_mask`), not by parsing Rust.
- **ftl**: remove one message; remove one placeable from it; rename one variable.
- **css**: a declaration's value is changed. A colour becomes the panel's colour of the same theme (which fails any
  contrast), any other value becomes `initial`.

A fault that does not compile is counted and dropped. A fault after which a run hangs (no end after four times the
unfaulted time, at least 60 s) is listed as hung and counts for no test.

### Running one fault

1. Apply the fault's diff to the worktree.
2. Native tests: build the test binaries once (`cargo test --no-run` for the crates that hold the new tests), then run
   only the new tests by name.
3. Browser tests: build the wasm at level 1 with the fault, run `wasm-bindgen` into `web/pkg`, run only the new tests by
   file and line. Level 1 is set by environment variables (`CARGO_PROFILE_DEV_OPT_LEVEL=1`, `CARGO_PROFILE_DEV_DEBUG=0`),
   so no manifest changes. The gate first runs the new browser tests at level 1 without a fault, and refuses if they
   fail there but passed in release.
4. Take the diff back, in a `finally`. At the end `git status` must be clean; if not, the gate says which file and exits 2.
   A gate that is killed leaves a dirty worktree, which the next gate refuses, naming the fault's file from a note it
   keeps outside the worktree.

Native faults run before browser ones. A native fault that all native tests catch is still run for the browser tests,
since each new test needs its own proof; a fault is not run for a test that has already caught three.

### The limit

At most 40 faults run for native tests and 12 for browser tests, and the gate stops starting new ones 30 minutes after
it began, counting only fault time. At the measured times that is about 11 minutes for native tests (40 x 16 s)
and about 10 for the browser (12 x 50 s). When there are more faults, they are taken in turn, one from each named
function or file, in the order of the plan, so that no function takes them all. A test that has caught nothing when the
limit is reached fails the gate, and the report says the limit was the reason.

## Where it goes

- `.claude/factory/gate.py`: `tests`, with the faults in a new module `.claude/factory/faults.py`. Standard library only.
- `.claude/factory/README.md`, `factory-triage`, `factory-plan`, `factory-build`, `factory-review`, `factory-evidence`,
  and the agents `triage`, `planner`, `test-writer`, `reviewer`, `evidence-check`.
- `cargo-mutants` 27.1.0 is installed once, by hand, into the main checkout's `.tools`
  (`cargo install cargo-mutants --version 27.1.0 --root .tools`), and the gate runs it from there. The README says so,
  and the gate's refusal gives the command. No `justfile` recipe: the `justfile` is not CI configuration
  (`CI_CONFIGURATION`), so a change to it would raise the product's version for a tool the product does not use.
- `CLAUDE.md` (the factory's bullet), and the Testing page that names the factory's gates, in English and fr-CA
  (`website/docs/testing.md`, or the page CLM-60 moves that part to).
- Tests: `scripts/include/tests/test_factory_gate.py` (each rule, with fake runs) and a new `test_factory_faults.py`
  (each kind of fault on sample text, and a diff taken back exactly).

It is built by hand, in a session with a person, since the factory may not change its own gate. It changes only
`.claude/**`, `scripts/include/**` and documentation, so it raises no version.

## Order of work

1. `faults.py` with its tests. 2. `gate.py tests` with its tests. 3. The skills, agents and README. 4. A trial on two
issues by hand: CLM-92 (markup) and CLM-98 (browser). 5. The pull request; the 23 issues are unblocked when it merges.

## Calls made here, not the person's

The planner names the faults; each new test needs one; uncaught faults do not fail; the limits; level 1 for browser
faults; the version raised by the test-writer; the order of work.
