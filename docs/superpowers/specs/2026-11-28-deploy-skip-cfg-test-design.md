# Postsubmit: a change to `#[cfg(test)]` code alone does not deploy

## Intent

Since #335, Postsubmit deploys a push only when a file the site is built from (`DEPLOY_INPUTS` of
`scripts/ci/ci_changes.py`) changed since the last deploy. The list goes by path, so a merge that adds tests to the
`#[cfg(test)] mod tests` of an ordinary source file (most of the factory's tests flow: 7 of the 8 tests-flow merges
CLM-77..84 on main) still deploys an identical product. The goal is to skip those deploys.

## Goals, non-goals, anti-goals

- **Goal:** a served `.rs` file counts as changed only when what is left once every `#[cfg(test)]` item is removed
  differs between the base and the head.
- **Non-goal:** other test-only conditions (`cfg(all(test, …))`, `#![cfg(test)]`, the `test-provider` feature): they
  keep deploying.
- **Non-goal:** narrowing the checks: only the `deploy` output changes.
- **Anti-goal** (from #335): a push that changes the site and is not deployed. Anything the scanner cannot read with
  confidence counts as changed.

## Constraints

- Standard-library Python, run by the Changes job without a Rust toolchain; tested in
  `scripts/include/tests/test_ci_changes.py` (class `Deploy`) with temporary repositories, never the checkout's history
  (CI's checkouts are shallow).

## Approaches

- **A. A standard-library Python token scanner.** **Chosen.** No build in the Changes job; fail-safe by construction.
- **B. A small Rust helper on `syn`.** Exact, but the Changes job would build it on every push, and a crate under
  `crates/` is itself a deploy input.
- **C. Regular expressions over the diff's hunks** (each inside `mod tests`). Cannot tell where a block ends, so it
  cannot fail safe. Dropped.

## Design

- For each changed file that is `served()` and ends in `.rs`, read it at the base and at the head (`git show`). A file
  missing on either side is changed.
- Tokenize: whitespace and comments (line, nested block, doc comments) dropped; strings, raw strings (`r#"…"#`), byte and
  C strings, chars and lifetimes, numbers, identifiers and punctuation kept as tokens.
- Remove each `#[cfg(test)]` (exactly those tokens) and the item it marks: any further outer attributes, then up to the
  first `;` at depth 0, or to the `}` that closes the first `{` at depth 0 (and a `;` right after it). Brackets `()`,
  `[]`, `{}` are matched as they go.
- Unreadable means changed: an unterminated string or comment, a bracket that closes another kind or nothing, a marked
  item that reaches `,`, a closer of its parent or the end of the file first (a field, an argument, a match arm).
- Compare the two token lists; equal means the file does not deploy. Any git error means changed.
- `deploys(files)` keeps its signature for the path check; the push path calls a function that also gets the base and
  head, so files whose only change is in `#[cfg(test)]` code are left out before `served()` decides.

The person's calls: the approach. My calls (accepted): comments outside test code do not count (no code is built from
them); a test item added above other code shifts its `line!()` and panic locations, which does not count either (the site
behaves the same).

## Left to the builder

The scanner's internal shape, the names, and the tests' cases. `website/docs/testing/when-tests-run.md` and its fr-CA
copy say what deploys now. The PR's description redoes the measurement on main's recent merges.
