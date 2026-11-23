# Junction signal folds (CLM-49) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** On the junction's signal plan level, each of the six signal options is a closed fold whose line says what is set and how many warnings it has, so the phases come first.

**Architecture:** A pure function per option gives the fold's line from the view (`JView`) and `I18n`, in a new `junction/view/signal_folds.rs`; the inspector's existing `fold` helper gains a warnings part and wraps each of the six sections unchanged.

**Tech Stack:** Rust (Leptos 0.8), Fluent `.ftl` (en, fr-CA), CSS, Playwright.

**Spec:** `docs/superpowers/specs/2026-11-23-junction-signal-folds-design.md`

Worktree `/Users/alex/src/cityloom/worktrees/clm-32-step2`, branch `alex/clm-49-fold-the-junctions-signal-options-so-the-phases`. Local e2e: `PATH=/opt/homebrew/opt/node@22/bin:/Users/alex/src/cityloom/main/.tools/bin:$PATH E2E_PORT=8147` (port 8137 is the main checkout's server); `npm ci` at the root and in `e2e/` first.

## Global Constraints

- Test-driven; one commit per task; `just format` before each; `RUSTFLAGS=-D warnings`.
- Every new message in both `junction/i18n/en.ftl` and `fr.ftl`, same ids and variables, plurals by selector, OQLF rules.
- Each section inside a fold keeps its markup, ids and `h3` (every existing selector still finds it).
- The phases stay open; the folds start closed and are not remembered.
- Version `0.112.0` → `0.113.0`, `cargo update -p cityloom_editor`.

## Review Focus

1. **A plan other than the default shown** (time-of-day plans): the plans fold names the plan shown, not the default. Pinned in Task 1 (`plans_line` test with two plans, the second viewed).
2. **An option with a finding but nothing set** (e.g. pre-emption off but a finding): the warning still shows. Pinned in Task 1 (warnings counted from `findings`, independent of the count of what is set).
3. **French** with one and several (plural selectors, no-break space before `:` if any). Pinned in Task 1 (a French test of one and two).
4. **Read-only:** folds open, nothing inside editable. Pinned in Task 2 (read-only e2e opens a fold and finds its controls disabled).
5. **Keyboard:** a fold opens with Enter on its summary. Pinned in Task 2 (e2e focuses `#jn-fold-calls summary`, presses Enter, sees `#signal-calls`).

---

### Task 1: The folds' lines

**Files:**
- Create: `crates/cityloom_editor/src/junction/view/signal_folds.rs` (and `pub mod signal_folds;` in `junction/view/mod.rs`; the file in `i18n_guard.rs`'s `MIGRATED`)
- Modify: `crates/cityloom_editor/src/junction/i18n/en.ftl`, `fr.ftl`
- Test: in `signal_folds.rs` (`#[cfg(test)] mod tests`, native)

**Interfaces:**
- Produces:
  - `pub enum SignalFold { Plans, Calls, Actuation, Bikes, Priority, Preemption }` with `pub const ALL: [SignalFold; 6]` in that order, `pub fn id(self) -> &'static str` (`jn-fold-plans`, `jn-fold-calls`, `jn-fold-actuation`, `jn-fold-bikes`, `jn-fold-priority`, `jn-fold-preemption`) and `pub fn heading(self) -> &'static str` (`jn-plans-heading`, `jn-calls-heading`, `jn-act-heading`, `jn-bikes-heading`, `jn-transit-heading`, `jn-preempt-heading`).
  - `pub fn set_line(fold: SignalFold, v: &JView, i18n: &I18n) -> String` — what is set; empty when the junction has no signal.
  - `pub fn warnings(fold: SignalFold, v: &JView) -> usize` — the option's `findings.len()`; for `Plans` the sum over `plans.plans[*].findings`.
  - `pub fn warnings_line(n: usize, i18n: &I18n) -> String` — "" for 0, else `jn-fold-warnings` with `n`.
- Counts: calls `crossings.iter().filter(|c| c.on_call)`; actuation `streets.filter(settings.is_some())`; bikes `movements.filter(on_detection)`; priority `transit.streets.filter(priority.is_some())`; pre-emption `preemption.streets.filter(rate.is_some())`; plans `plans.plans.len()` and the name (`PlanView.name`, a `Said`, through `say_now`) of the one whose `id == plans.viewing`.

Messages (English; French alongside, OQLF):
```
jn-fold-plans-set = { $n ->
    [one] { $name }
   *[other] { $n } plans, { $name } shown
}
jn-fold-calls-set = { $n -> [0] None on call [one] 1 crossing on call *[other] { $n } crossings on call }
jn-fold-actuation-set = { $n -> [0] None actuated [one] 1 street actuated *[other] { $n } streets actuated }
jn-fold-bikes-set = { $n -> [0] None on detection [one] 1 movement on detection *[other] { $n } movements on detection }
jn-fold-priority-set = { $n -> [0] None with priority [one] 1 street with priority *[other] { $n } streets with priority }
jn-fold-preemption-set = { $n -> [0] None with pre-emption [one] 1 street with pre-emption *[other] { $n } streets with pre-emption }
jn-fold-warnings = { $n -> [one] 1 warning *[other] { $n } warnings }
```
(Multi-line selector form in the files, as the others there. `[0]` is a numeric variant Fluent matches before the plural category; check the engine matches it, else use `jn-fold-…-none` messages.)

- [ ] **Step 1: Write the failing tests** in `signal_folds.rs`:
  - `a_signal_with_nothing_set_says_so_on_each_fold`: `Junction::new(0)`, `set_control(SIGNAL)`; for Calls…Preemption `set_line` is "None on call", "None actuated", "None on detection", "None with priority", "None with pre-emption"; Plans is the default plan's name (read it from `v.plans`); `warnings` is whatever `findings.len()` is (assert equal to the view's, not a constant).
  - `each_fold_counts_what_is_set`: after `set_on_call` on two crossings, `set_actuated` on one street, `set_bike_detection` on one, `set_transit_priority(uid, Some(mode))` on one with transit (pick from the view's `transit.streets` with `has_transit && has_phase`; skip with a comment if the sample has none and use `Junction::new(k)` that has), `set_preemption` on one: "2 crossings on call", "1 street actuated", "1 movement on detection", "1 street with priority", "1 street with pre-emption".
  - `the_plans_fold_names_the_plan_shown`: `add_plan("Evening")`, view it (the vm's or model's view-plan call); `set_line(Plans)` is "2 plans, Evening shown".
  - `warnings_are_counted_from_the_findings_and_said_only_when_there_are_some`: `warnings_line(0, en) == ""`, `warnings_line(1, en) == "1 warning"`, `warnings_line(3, en) == "3 warnings"`; and a junction set to give a finding (pre-emption on a street with no phase, or actuated where refused — whichever the model leaves as a finding) has `warnings(fold, &v) == v.signal…findings.len() > 0`.
  - `the_folds_speak_french`: fr-CA of "None on call", "1 street actuated", "2 warnings" — read the fr values you wrote; no English left.
- [ ] **Step 2: Run** `cargo test -p cityloom_editor --lib junction::view::signal_folds` — Expected: compile error (module missing).
- [ ] **Step 3: Implement** the module and messages as above.
- [ ] **Step 4: Run** the same, then `cargo test -p cityloom_editor --lib -- i18n junction::` — Expected: PASS (parity and guard included).
- [ ] **Step 5: Commit** — `Say what each of the junction's signal options has set, and its warnings (CLM-49)`.

---

### Task 2: The folds on the signal plan level

**Files:**
- Modify: `crates/cityloom_editor/src/junction/view/inspector.rs` (`fold`; the signal level's view: `plans_section`, `calls_section`, `actuation_section`, `bikes_section`, `priority_section`, `preemption_section` call sites, ~lines 1694–1727)
- Modify: `web/editor.css` (`.jn-fold-warn` next to `.jn-fold-set`)
- Test: `crates/cityloom_editor/src/junction/view/tests.rs`; e2e specs that touch these sections (`junction.spec.ts`, `junction-a11y.spec.ts`, `read-only.spec.ts`, `impact.spec.ts` if it does)
- Docs: `crates/cityloom_editor/src/junction/CLAUDE.md` (the signal plan level), `website/docs` + fr-CA where the signal plan level is described (grep "Pedestrian calls"/"Appels piétons"), version bump.

**Interfaces:**
- Consumes: Task 1's `SignalFold`, `set_line`, `warnings`, `warnings_line`.
- Produces: `fold(w, id, heading, set, warn, body)` — `warn: impl Fn() -> String + Send + Sync + 'static`, drawn after the set line as `<span class="jn-fold-warn">` only when non-empty; the street's two folds pass `|| String::new()`.

- [ ] **Step 1: Write the failing component test** `the_signal_plan_folds_each_option_and_says_what_is_set_and_its_warnings`: `shared(0)`, `set_control(SIGNAL)`, `s.show_plan()`, `panel(&s)`:
  - six `<details class="jn-fold" id=…>` with the six ids, none `open`, in order plans, then (after `id="i-h-signal"`) calls, actuation, bikes, priority, pre-emption; `i-h-signal` before `jn-fold-calls` and after `jn-fold-plans`.
  - each summary contains its heading's English and its `set_line`; inside each fold the section's id (`signal-plans`, `signal-calls`, `signal-actuation`, `signal-bikes`, `signal-transit`, `signal-preemption`) as before.
  - after an edit that makes a finding (as in Task 1), that fold's summary contains `class="jn-fold-warn"` and "1 warning"; a fold with none has no `jn-fold-warn`.
  - the street's two folds (existing test `a_street_folds_its_transit_priority_and_position…`) still pass unchanged.
- [ ] **Step 2: Run** `cargo test -p cityloom_editor --lib junction::view::tests::the_signal_plan_folds` — Expected: FAIL (no folds).
- [ ] **Step 3: Implement**: `fold` takes `warn`; on the signal level wrap each section in `fold(w, f.id(), f.heading(), move || set_line(f, &w.view(), &w.i18n()), move || warnings_line(warnings(f, &w.view()), &w.i18n()), section)`. CSS `.jn-fold-warn { color: var(--warn); font-size: 12px; font-weight: 600; }` (and a separator "·" between set and warnings as text in the message or a CSS `::before` — choose the message: keep words in Fluent).
- [ ] **Step 4: Run** junction view tests and the guard — Expected: PASS.
- [ ] **Step 5: Browser tests.** Add to `junction.spec.ts`: `test("the signal plan's options are folds, closed, each saying what is set; Enter on one opens it")` — show the plan (the existing way the specs reach it), expect six `details.jn-fold` closed, `#jn-fold-calls summary` contains "None on call", focus it, press Enter, `#signal-calls` visible. Add to `read-only.spec.ts`'s junction read-only test: open `#jn-fold-calls`, the on-call checkbox is disabled. Then run `just e2e tests/junction.spec.ts tests/junction-a11y.spec.ts tests/read-only.spec.ts tests/impact.spec.ts`; each test that now fails because a control is inside a closed fold opens that fold first (`await page.locator("#jn-fold-<x> summary").click()`), never dropping an assertion. Expected: PASS.
- [ ] **Step 6: Docs and version**, `just format`, `just test`, `just e2e` (DATABASE_URL `postgres://postgres:postgres@127.0.0.1:55432/cityloom`), `just docs-build` if `website/` changed — Expected: all PASS.
- [ ] **Step 7: Commit** — `Fold the junction's signal options so the phases come first (0.113.0) (CLM-49)` (docs and version may be a separate commit).
