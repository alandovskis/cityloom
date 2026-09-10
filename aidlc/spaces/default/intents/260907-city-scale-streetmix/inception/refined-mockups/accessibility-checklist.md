# Accessibility Checklist — Streetmix at City Scale

Target: **WCAG 2.1 Level AA, including the editing surface** (NFR4.1).

`team-practices.md` states the position this checklist is written from, and it
is worth repeating rather than softening: **WCAG 2.1 AA is not fully
CI-verifiable, and a green pipeline is not read as conformance.** This file
separates what a machine can check from what a person must, and says which is
which for every item.

## The one thing that decides how much of this is automatable

Whether the editing surface is built from real page elements or drawn on a
canvas is deferred to Domain Design (`stories.md` OQ2, OQ-US7).

- **DOM**: every item below is verifiable, most of them automatically.
- **Canvas**: axe-core treats `<canvas>` as an opaque node. Everything about the
  lane strip — focus, names, states, activation targets — becomes verifiable
  only if the canvas exposes an accessibility tree **and** its hit-test regions
  are queryable by the test harness. If it does not, those items fall to the
  manual walkthrough, on every release, indefinitely.

That is a cost of the canvas choice, and it belongs in Domain Design's decision
rather than being discovered after B-0. It also determines whether the walking
skeleton's third pass criterion — mapped visibly distinguished from inferred in
what is rendered — can be automated at all.

---

## Perceivable

| # | Requirement | Where | How verified |
|---|---|---|---|
| P1 | Body text ≥ 4.5:1 contrast; large text ≥ 3:1 | Every screen | Automated (axe) |
| P2 | UI components and graphical objects ≥ 3:1 | Buttons, inputs, focus ring, lane blocks, map overlays | Automated for DOM components; **manual** for lane blocks under canvas |
| P3 | The hatch pattern ≥ 3:1 against its own lane fill | Lane strip, estimated lanes | **Manual measurement.** See the fallback below |
| P4 | No state conveyed by colour alone | Provenance, fit status, sharing state, map overlays | **Manual review** against the state inventory; partially automated by asserting the text channel exists |
| P5 | Every value with `inferred` provenance carries "(estimated)" wherever it appears | Lane strip, detail panel, dimensions table, exports | Automated — fixture-driven (AC4.1.4, AC12.2.3) |
| P6 | One sentence per view explains what estimated means | S3 and every view on the committed view list | Automated — presence assertion per view (AC4.1.6) |
| P7 | Text reflows at 320px without loss of content (WCAG 1.4.10) | Every screen | Automated at 360px (AC1.1.3, AC13.2.1); **manual** below that, which is unsupported anyway |
| P8 | Layout usable at 200% zoom | Every screen | **Manual** |

### The hatch fallback, stated rather than assumed

A diagonal hatch on a lane block only a few CSS pixels wide may not achieve 3:1
against its own fill. **If it does not, the hatch becomes decoration and the
textual and accessible-name channels carry the provenance distinction alone.**

This is written down because the alternative is discovering it during
implementation and quietly dropping one of the three channels. Two channels
still satisfy AC4.1.4 and AC4.1.6; what would not is dropping the text and
keeping the pattern.

---

## Operable

| # | Requirement | Where | How verified |
|---|---|---|---|
| O1 | Every action on the committed editing-action list completes by keyboard | Editing surface | Automated, parameterised over the list (AC5.3.1) |
| O2 | Every drag has a selection equivalent | Editing surface | Automated over the same list (AC5.3.2) |
| O3 | Each lane is individually focusable | Lane strip | Automated for DOM; **manual** under canvas without an accessibility tree (AC4.1.5) |
| O4 | Visible focus indicator, ≥ 2px, ≥ 3:1 | Every interactive element | Automated for presence; **manual** for contrast against varied backgrounds |
| O5 | Activation targets ≥ 44 × 44 CSS px where the drawn element is smaller | Lane strip especially | Automated under DOM with emulated touch and a bounding-box assertion; **conditional** under canvas (AC5.3.4) |
| O6 | Drawn lane width is never widened to meet O5 | Lane strip | Automated — assert drawn width equals true scaled width |
| O7 | No keyboard trap | Bottom sheet, dialogs | Automated — tab-cycle assertion |
| O8 | Escape is layered: full → peek → closed | Bottom sheet | Automated |
| O9 | Focus returns to the opener on close | Sheet, dialogs, banner | Automated |
| O10 | Focus is not stolen when the map view changes | Street list | Automated (AC2.3.4) |
| O11 | Skip link is the first focusable element | Every page | Automated |
| O12 | Tab order follows visual order at every breakpoint | Every screen, notably the phone sheet | **Manual** at each breakpoint |
| O13 | A street is selectable without a pointing device | S2, street list | Automated (AC2.3.1, AC2.3.2) |
| O14 | No time limit on any interaction | Whole product | Review — none is designed in |

---

## Understandable

| # | Requirement | Where | How verified |
|---|---|---|---|
| U1 | Page language declared | Every page | Automated |
| U2 | Every input has a visible label, not a placeholder standing in for one | All forms | Automated |
| U3 | Errors identified in text, with a specific correction | Width stepper, search, import failure, upload failure | Automated per case |
| U4 | Error text is associated via `aria-describedby`, not merely adjacent | Same | Automated |
| U5 | Import failure names the street and its typed failure reason | S3 error | Automated (AC7.1.1) |
| U6 | No stack trace, library text or internal identifier reaches a user | S3 error, every failure | Automated — negative assertion (AC7.1.2) |
| U7 | Navigation and control placement consistent across screens | Whole product | **Manual review** |
| U8 | No unexpected context change on focus or input (WCAG 3.2.1, 3.2.2) | Whole product | **Manual**, plus one automated case below |
| U8a | No tab stop collapses the bottom sheet, opens the design disclosure, or moves focus as a side effect of receiving focus | BottomSheet, DesignDisclosure, map header | Automated — tab through every stop at 360px and assert the sheet height and the disclosure-open state are unchanged. This exists because an earlier draft of the keep-off-this-device control did exactly that |
| U10 | Every visible label is contained in its element's accessible name (WCAG 2.5.3) | Every labelled control, notably any label shortened at a narrow viewport | Automated — compare visible text against the computed accessible name for every control in the committed editing-action list plus the keep-off-this-device control |
| U9 | The destructive option is never the initial focus | Unresolved-correction banner, deletion | **Manual review** |

---

## Robust

| # | Requirement | Where | How verified |
|---|---|---|---|
| R1 | Valid, well-formed markup; unique IDs | Every screen | Automated |
| R2 | Native elements preferred over ARIA re-implementation | Especially `<select>`, checkboxes, `<button>` | **Manual review**, and criterion 5 in the framework selection list |
| R3 | Landmarks present: header, main, complementary, footer | Every screen | Automated |
| R4 | Heading hierarchy in order, no skipped levels | Every screen | Automated |
| R5 | Status messages announced without moving focus | All 23 committed status messages | Automated, parameterised over the list (AC13.1.1, AC13.1.2) |
| R6 | The unresolved banner is a `region`, not an `alert` | S9 | **Manual review** — an alert that never dismisses is a role misuse a scanner will not catch |
| R7 | Live-region text matches the visible text exactly | Status messages | Automated — string equality |

---

## The four map overlays

Called out separately because a map is where accessibility work is most often
abandoned.

| # | Requirement | How verified |
|---|---|---|
| M1 | Every overlay distinguished by shape or pattern as well as colour | **Manual review** |
| M2 | A legend exists, is keyboard-reachable, and lists every active overlay with its count | Automated for presence and reachability |
| M3 | The legend is the map's text equivalent, referenced by the map's accessible description | Automated for the reference; **manual** for adequacy |
| M4 | The legend is collapsed by default at 360px and its toggle states how many overlays are active | Automated |
| M5 | Every map-only affordance has a non-map equivalent — street selection has the list (US2.3); unresolved corrections have the drawer badge (Q4) | **Manual review** against the affordance inventory |

M5 is the one that matters most. A marker on a map is not an affordance for
someone who cannot use the map, so every marker needs a twin somewhere in the
document flow.

---

## What is automated, and where it runs

`team-practices.md` establishes two CI tiers. Accessibility work sits in the
slow tier — pull requests to `main`, and nightly — because it needs a real
browser.

| Layer | Tool | Catches |
|---|---|---|
| Static scan | axe-core via Playwright | P1, P2 (DOM only), U1, U2, R1, R3, R4 — roughly the contrast, names, roles and landmarks floor |
| Keyboard interaction tests | Playwright, parameterised over the committed editing-action list | O1, O2, O3, O7, O8, O9, O10, O13 |
| Status-region tests | Playwright, parameterised over the 23 committed status messages | R5, R7 |
| Focus-side-effect tests | Playwright, tabbing every stop at 360px | U8a — asserts no tab stop changes sheet height or menu state |
| Label-in-name tests | Playwright, comparing visible text to computed accessible name | U10 |
| Target-size tests | Playwright with emulated touch at 360px | O6, and **O5 under DOM only** — see the caveat below |
| Provenance tests | Fixture-driven, against the committed thinly-tagged extract | P5, P6, and the walking skeleton's third pass criterion |

**axe-core catches roughly a third of issues and none inside a canvas.** It is a
floor, not evidence about the editing surface.

**Caveat on O5, restated here because this table is what a CI-tier definition
gets copied from.** O5 (44 × 44 activation targets) is automated **under DOM
only**, where the target is the element's own box and a bounding-box assertion
measures it. Under canvas the hit-test region is internal and unmeasurable
unless it is deliberately exposed to the test harness — so O5 becomes either a
manual item or a condition on choosing canvas. The Operable table above carries
the full statement. Nothing in this file may be read as presupposing the
editing-surface decision, and a summary row that drops the condition would do
exactly that.

## What a person must do, every release

A defined manual walkthrough before each stage's release, per
`team-practices.md`:

1. **Full edit path with a screen reader.** Find a street, select it, import,
   read the cross-section including provenance, change a lane type, change a
   width, add a lane, undo, extend along a corridor, read the outcome.
2. **Keyboard only, no mouse plugged in**, through the same path.
3. **Zoom to 200%** on S2, S3 and S4.
4. **Phone at 360px**, both sheet heights, with the four map overlays active.
5. **Colour-blindness simulation** across the provenance states, the four fit
   states and the map overlays.
6. **The hatch contrast measurement** (P3), and a decision on whether the
   fallback applies.

Items 1 and 4 are the two that would catch a canvas surface silently failing.

## Standing gaps

- **OQ6 in `requirements.md`**: no desktop browser matrix exists. The mobile
  matrix is NFR4.6; desktop is assumed evergreen. This must be settled before
  the accessibility CI tier gets its browser list, or that tier is built against
  an unstated target.
- **OQ-US7 in `stories.md`**: if Domain Design selects canvas without an
  accessibility tree, items O3, O5 and P2 for the lane strip, and the walking
  skeleton's third pass criterion, all move to the manual walkthrough on the
  very first Bolt and stay there.
- **The hatch fallback** (P3) is a real possibility, not a hypothetical. Its
  resolution is recorded above rather than left to implementation.
