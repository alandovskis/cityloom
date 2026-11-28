# Deploy skips `#[cfg(test)]`-only changes — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `ci_changes.py --event push` reports `deploy=false` when every served file that changed is a `.rs` file whose
only change is inside `#[cfg(test)]` items.

**Architecture:** A token scanner in `scripts/ci/ci_changes.py` (standard library) turns a Rust file into a token list
without whitespace and comments, removes each `#[cfg(test)]` item, and returns `None` on anything it cannot read.
`deploys()` gains the base and head; a served `.rs` file is skipped when both sides scan and are equal.

**Tech Stack:** Python 3 standard library, `unittest`, git.

**Spec:** `docs/superpowers/specs/2026-11-28-deploy-skip-cfg-test-design.md`

## Global Constraints

- Standard library only; tests in `scripts/include/tests/test_ci_changes.py`, class `Deploy`; run by `just test-scripts`.
- Tests never read the checkout's history: git cases build a temporary repository (as `test_a_base_not_behind_head_deploys`).
- Fail-safe: unreadable, missing on a side, or any git error counts as changed.
- Only the exact attribute `#[cfg(test)]` (tokens `# [ cfg ( test ) ]`) marks test code.
- CI configuration only (`scripts/ci/**`, `scripts/include/**`, `docs/**`, `website/**`): no version bump.

## Review Focus

- A `'` lifetime (`&'a str`, `'static`) versus a char (`'{'`, `'\''`): a mislexed `'{'` would unbalance braces → test
  `test_chars_and_lifetimes_do_not_unbalance`.
- Braces inside strings and raw strings (`"{"`, `r#"}"#`) inside the test module → same test.
- `#[cfg(test)]` on a field or argument (ends at `,`/`)`) → unreadable, deploys → `test_a_marked_field_deploys`.
- A changed attribute on the test mod itself, e.g. `#[cfg(test)]` removed (tests become real code) → deploys →
  `test_removing_the_cfg_deploys`.
- `#[cfg(test)] use x;` and `#[cfg(test)] const N: usize = 3;` (end at `;`), and `#[cfg(test)] impl X { }` followed by
  code → handled → `test_each_kind_of_item_is_removed`.

---

### Task 1: The scanner

**Files:** Modify `scripts/ci/ci_changes.py`; Test `scripts/include/tests/test_ci_changes.py` (class `Deploy`).

**Interfaces — Produces:** `rust_tokens(text: str) -> list[str] | None`; `outside_tests(text: str) -> list[str] | None`
(tokens with every `#[cfg(test)]` item removed; `None` when unreadable).

- [ ] **Step 1: Failing tests** (`outside_tests` of base and head compared):
  - `test_a_test_added_to_the_tests_module_does_not_count`: equal.
  - `test_a_new_tests_module_does_not_count`: code, then code + `#[cfg(test)] mod tests { … }`: equal.
  - `test_whitespace_and_comments_do_not_count`: equal.
  - `test_a_change_outside_the_tests_counts`: not equal.
  - `test_removing_the_cfg_deploys`: not equal.
  - `test_each_kind_of_item_is_removed`: `use`, `const`, `fn` with `-> [u8; 3]`, `impl`, `mod`, a second attribute
    (`#[cfg(test)] #[derive(Debug)] struct S { a: u8 }`); the result equals the code without them.
  - `test_chars_and_lifetimes_do_not_unbalance`: test module holding `'{'`, `'\''`, `"}"`, `r#"}"#`, `b'{'`, `&'a str`,
    nested `/* /* } */ */`: removed cleanly, code after it kept.
  - `test_what_cannot_be_read_is_none`: unterminated string, unterminated comment, `(]`, extra `}`, marked field
    `struct S { #[cfg(test)] a: u8, }`, marked item running to end of file: `None`.
- [ ] **Step 2:** `python3 -m unittest scripts/include/tests/test_ci_changes.py -k Deploy` → FAIL (`AttributeError`).
- [ ] **Step 3:** Implement. Lexer: one regex alternation tried at each position (whitespace, `//` line comment, nested
      `/* */` by a counter, raw strings by counting `#`s, `b"`/`c"`/`"` strings with escapes, char `'x'`/`'\…'` tried before
      lifetime `'ident`, identifiers, numbers, single punctuation). Remover: walk tokens; at `# [ cfg ( test ) ]`, skip any
      further `# [ … ]`, then scan with a bracket stack: `;` at depth 0 ends it; the `}` closing the first `{` at depth 0 ends
      it (eat one following `;`); `,`, a closer at depth 0, mismatched closer, or end of tokens → `None`. Check balance of the
      whole remainder too (mismatched → `None`).
- [ ] **Step 4:** Tests pass.
- [ ] **Step 5:** Commit "Read a Rust file without its #[cfg(test)] items".

### Task 2: The deploy decision uses it

**Files:** Modify `scripts/ci/ci_changes.py` (`deploys`, `main`, module docstring); Test as above.

**Interfaces — Consumes:** `outside_tests`. **Produces:** `deploys(files, base: str = "", head: str = "HEAD") -> bool`;
`changed_outside_tests(path: str, base: str, head: str) -> bool` (True unless both sides exist, scan, and are equal; a
git error is True).

- [ ] **Step 1: Failing tests** with a temporary repo (`ROOT` patched), files at `crates/x/src/a.rs`:
  - `test_a_change_to_test_code_alone_does_not_deploy`: commit 2 adds a test to `mod tests` →
    `deploys(["crates/x/src/a.rs"], c1, c2)` is False.
  - `test_a_change_to_code_beside_tests_deploys`: True.
  - `test_a_new_or_deleted_file_deploys`: True for each.
  - `test_a_served_file_that_is_not_rust_deploys`: `web/app.js` → True.
  - `test_without_a_base_the_path_decides`: `deploys(["crates/x/src/a.rs"])` is True (existing tests stay as they are).
- [ ] **Step 2:** Run → FAIL.
- [ ] **Step 3:** Implement: in `deploys`, a served `.rs` file with a base counts only if `changed_outside_tests`.
      `main` passes `a.base, a.head` (empty base when `a.files` is used). Docstring's `deploy` line says so.
- [ ] **Step 4:** `just test-scripts` → all pass (Script tests may show #336's `dockerfiles/` failure until it merges).
- [ ] **Step 5:** Commit "Skip the deploy of a push that changes only #[cfg(test)] code".

### Task 3: Documentation, measurement, PR

**Files:** `website/docs/testing/when-tests-run.md`,
`website/i18n/fr-CA/docusaurus-plugin-content-docs/current/testing/when-tests-run.md`.

- [ ] **Step 1:** Replace "The list goes by file: a test added to a `#[cfg(test)]` module … deploys" with how a served
      `.rs` file is read without its `#[cfg(test)]` items, and that anything unreadable deploys (both languages; OQLF
      spacing in fr-CA).
- [ ] **Step 2:** `just format-check` and `just docs-build` pass.
- [ ] **Step 3:** Measurement: for main's last 40 merges, run old and new `deploys` with base = parent; table of the
      merges whose decision changes.
- [ ] **Step 4:** Commit, push, PR with the measurement; CI green.
