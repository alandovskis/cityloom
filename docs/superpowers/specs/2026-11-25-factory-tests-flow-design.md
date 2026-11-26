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

Triage gives an issue the flow **tests** when it asks only for new tests: no change to what the product does, and none
to what an existing test asserts. It hands the issue back when it asks for either, or when the only code the tests could
be proved against is outside what the gate can fault (below). An issue that also needs a change to the product is
**code**.

```
Todo ─ queue ─ triage ─ plan ─ test-writer ─ tests gate ─ review ─ evidence ─ PR ─ approval ─ merge ─ deploy
              (tests)  (tests)  └── factory-build ──┘
```

There is no red commit, no implementer and no `Factory-Red` trailer. The record `**Factory · tests** <sha>` takes the
place of `green`, as `**Factory · docs**` does in the docs flow. Rounds: `rounds.green`.

**Plan.** Its headings: `## Change`, `## Where`, `## Tests`, `## Faults`, `## Version`, `## Out of scope`. It has no
`## Slice dependencies` (the code flow's since CLM-57): test code is exempt from `dependency_guard.rs`, and this flow
changes no product code. `## Faults` holds one fenced `json` block. The planner writes it, so the one who writes the tests does not choose what is faulted.

```json
{
  "rust": [{ "file": "crates/cityloom_editor/src/shared/platform.rs", "functions": ["<impl Page for BrowserPage>::copy_text"] }],
  "markup": [{ "file": "crates/cityloom_editor/src/timeline/view.rs", "functions": ["question"] }],
  "ftl": [{ "file": "crates/cityloom_editor/src/timeline/i18n/en.ftl", "messages": ["timeline-heading"] }],
  "css": [{ "file": "web/editor.css", "rule": ":root", "property": "--danger", "to": "var(--panel)" }]
}
```

A `rust` function is named as `cargo-mutants` prints it; a `markup` function as it is written, and must be the file's
only one of that name. `messages` may be `"*"`. A CSS rule and property must match one declaration of the file (`"nth"`
picks one otherwise); `to` is the planner's, since no value fails every contrast, and two entries fault the two themes of
a `light-dark(a, b)`.

Before the plan is posted, `factory-plan` runs `gate.py faults --repo <main checkout> --faults <file>`. It builds
nothing. It fails when a file, function, message or declaration does not exist, or when an entry yields no fault. The
plan then goes back to the planner, as for any failed check of it.

**Test-writer.** In this flow it writes tests that pass on today's code, adds no stub, raises the patch version and
`Cargo.lock` (a change under `crates/` or `e2e/` must raise it: `scripts/check-version.sh`), and runs `just format`. A
new spec goes in `tiers.json` and `spec-map.json`, and new test files in `features.json`, as today. It sees the plan, so
it knows which functions are faulted, as it knows what it is asked to test. It does not see the faults made there: the
orchestrator gives it `gate.py`'s output with `--brief`, which names the tests that caught nothing and no fault.

**A test that fails on today's code** has found a bug. The test-writer leaves it out and reports it. The orchestrator
files it as an issue in Triage, linked to this one, and names it in the pull request's Unknowns. If nothing else is left
to test, the issue is handed back.

**Review.** As in the code flow, and the tests are the change: a weak test is a finding here, sent back to the
test-writer, not a `testNotes` line. A test that asserts more than the behaviour the issue names (a whole page's markup,
say) is weak, since any fault breaks it.

**Evidence.** The gate's report, the new tests' own output at the head, and for browser tests screenshots taken by a
run with `--trace on`, kept as files. The pull request's Proof has a table: each new test, and one fault it caught.

**After a rebase.** When `git range-diff` shows the branch's commits the same but for the version lines, and no file that the
plan faults changed between the two bases, the gate runs its rules 1 to 4 only (`--proved <sha>`, the head of the last
record) and the record says which head the proof was made at. Otherwise the faults run again. A named function that is
gone from the new base is a hand-back. The `merge` agent changes no product code in this flow: a Sonar or CI finding
there is a hand-back, and `merge.md` says so.

## The tests gate

```sh
python3 "$F/gate.py" tests --repo <wt> --base "$BASE" --faults <file> [--proved <sha>] [--brief]
```

It refuses (exit 2) as the others do (dirty worktree, no database, no setup), and when `cargo-mutants` is missing or not
the pinned version. It fails (exit 1) unless all of these hold:

1. **Only new tests.** Every changed file is one of: a test file (`TEST_FILES`); `scripts/ci/features.json`; the
   version line of `crates/cityloom_editor/Cargo.toml` and of `Cargo.lock`; `shared/contrast.rs`; or another `.rs` file
   whose only changes are in test code. Nothing else of `GUARDED` may change, whatever `TEST_FILES` matches: not
   `.claude/**`, `.github/**`, `scripts/**` (but `features.json` and `spec-map.json`), nor another guard.
   Documentation and `website/**` may not.
   *Test code* is an item under exactly `#[cfg(test)]` or `#[cfg(all(test, …))]`, or a whole file whose `mod` declaration
   is. `any(test, …)` is product code (`cityloom_server/src/providers.rs`), so `cfg_test_blocks` is not used for this.
   `e2e/tests/` helpers that are not specs may gain exports; a changed line there is refused.
2. **Existing tests are kept.** None is gone, ignored or skipped, and the body of none changes: the `#[test]` functions
   and the Playwright tests of the base are in the head with the same text.
3. **At least one new test**, listed at the head and not at the base, as the red gate lists them.
4. **Every test passes** at the head, with warnings denied, as the green gate runs them (every Rust test, the wasm
   build, the changed specs; every spec when a helper changed, as the red gate does), and `just format-check`,
   `scripts/check-version.sh` and `just test-scripts` pass. Each new test passes again in a second run by itself.
5. **Each new test catches at least one fault.** A fault is caught by a new test that ran to its own failure: an
   assertion, a panic, or Playwright's own timeout of that test.

The report lists each new test with the first three faults it caught and how many in all, the faults no new test
caught, those that did not compile, those that hung, and those the limit left out. Uncaught faults are information
for the reviewer and the person: some change nothing a person can see.

**When rule 5 fails**, the test-writer gets one round. If the same tests still catch nothing, the planner gets the full
report, with no test's source, and may replace `## Faults` once, which runs the gate again. Failing after that, or failing because of the
limit, is a hand-back with the full report. These rounds count in `rounds.green`.

### The faults

- **rust**: from `cargo mutants --list --json --diff`, those whose function is named. Its diffs are not patches (their
  `+++` line is a description, with no path), so the gate replaces the span it gives with its replacement.
- **markup**: in the `view!` of a named function, remove one attribute, change one string literal, or swap two
  neighbouring elements of one parent. Made by the gate with its own tokenizer (`_code_mask`), not by parsing Rust.
- **ftl**: remove one message, remove one placeable from it, or rename one variable.
- **css**: the declaration's value becomes `to`.

A fault that does not compile is counted and dropped.

**What can be faulted for a native test:** any crate. **For a browser test:** the editor's wasm, its `.ftl` and CSS
(`web/*.css` needs no build). Not: `cityloom_server`, `cityloom_report_pdf`, `citymodel_builder`, `web/*.js`,
`web/*.html`, nor the specs that need `E2E_ARCHIVE=1`. Triage hands an issue back that needs one. None of the 23 does:
21 are native tests of the editor or the builder, and CLM-76 and CLM-98 are browser tests of the editor's wasm (CLM-98 in
the integration tier, with the server unfaulted).

### Running one fault

1. Apply it to the worktree.
2. Native tests: build the test binaries (`cargo test --no-run` for the crates that hold the new tests), then run each
   new test by its full name, in a process of its own, so that an abort belongs to one test.
3. Browser tests: build the wasm at level 1, run `wasm-bindgen` into `web/pkg`, run only the new tests by file and line.
   Level 1 is set by the environment (`CARGO_PROFILE_DEV_OPT_LEVEL=1`, `CARGO_PROFILE_DEV_DEBUG=0`), so no manifest
   changes. First the new browser tests run at level 1 without a fault, and the gate refuses if one fails there.
4. Take it back, in a `finally`.

A native test that has not ended after 60 s, or four times its unfaulted time if that is longer, is killed: the fault
is *hung* for it, and caught by no one. A browser test ends by its own timeout, which catches. A spec's timeout is capped at 30 s
in these runs (`--timeout`), and the browser's run as a whole at 5 minutes, after which the tests that did not end are
hung. A fault is not run for a test that has already caught three.

**Put back.** Before it starts, the gate writes the commit it began at to a note outside the worktree, and copies
`web/pkg`. At the end, `git status` must be clean and `web/pkg` is restored from the copy. A gate that finds a note left
by a killed one refuses, names the fault's file, and says to restore it and to run `just build`: a leftover faulted wasm
is not in `git status`, since `web/pkg` is ignored.

### The limit

At most 40 faults for native tests and 12 for browser tests, and none starts later than 30 minutes after the first. At the
measured times that is about 11 minutes native (40 x 16 s) and about 13 in the browser (12 x 64 s, with `junction-a11y`
whole as the measure, so fewer tests take less, and tests that all time out more: the 30 minutes are the bound). When
there are more faults, they are taken in turn, one from each named entry, in the order of the plan.

`factory-plan` refuses a plan whose entries are more than the limit of their kind.

## Where it goes

- `.claude/factory/gate.py`: `tests` and `faults`; the faults in a new module `.claude/factory/faults.py`. Standard
  library only.
- `.claude/factory/README.md`, the skills `factory-triage`, `factory-plan`, `factory-build`, `factory-review`,
  `factory-evidence`, and the agents `triage`, `planner`, `test-writer`, `reviewer`, `evidence-check`, `merge`.
- `cargo-mutants` 27.1.0 is installed once, by hand, into the main checkout's `.tools`
  (`cargo install cargo-mutants --version 27.1.0 --root .tools`), and the gate runs it from there. The README says so,
  and the gate's refusal gives the command. No `justfile` recipe: the `justfile` is not CI configuration
  (`CI_CONFIGURATION`), so a change to it would raise the product's version for a tool the product does not use.
- `CLAUDE.md` (the factory's bullet), and the Testing pages that name the factory's gates and their tests, in English
  and fr-CA: `website/docs/testing/when-tests-run.md` (the factory's gates) and `testing/levels.md` (the script tests,
  for `test_factory_faults.py`).
- Tests: `scripts/include/tests/test_factory_gate.py` (each rule, with fake runs) and a new `test_factory_faults.py`
  (each kind of fault on sample text, and the text put back exactly).

It is built by hand, in a session with a person, since the factory may not change its own gate. It changes only
`.claude/**`, `scripts/include/**` and documentation, so it raises no version.

## Order of work

1. `faults.py` with its tests. 2. `gate.py tests` and `faults` with their tests. 3. The skills, agents and README. 4. A
trial by hand on CLM-92 (markup), CLM-97 (CSS) and CLM-98 (browser). 5. The pull request; the 23 issues are unblocked
when it merges.

## Calls made here, not the person's

The person chose the proof (mutation testing) and approach 2. Mine: the planner names what is faulted and writes a CSS
fault's value; the test-writer sees the named functions but no fault; each new test needs one fault; uncaught faults do
not fail; existing tests may not change; a weak test is a finding in this flow; the planner may revise the faults once;
the proof is kept over a rebase that changes nothing it rests on; a browser test's own timeout catches; what a browser
test can be proved against; the limits; level 1; the version raised by the test-writer.
