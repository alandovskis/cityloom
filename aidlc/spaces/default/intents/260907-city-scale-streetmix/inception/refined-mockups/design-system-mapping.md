# Design System Mapping — Streetmix at City Scale

Q3: a **CSS-only framework for layout and controls, with project tokens layered
on top**. This file defines the tokens, names the components, and states what
the framework must satisfy — without naming the framework, which depends on a
decision this stage does not own.

## Why the framework is not named here

Q3 chose the approach, not the dependency. Naming one requires two things that
are still open:

- **Whether the editing surface is DOM or canvas** (`stories.md` OQ2). A
  classless, semantic-first framework suits the "native HTML elements first"
  accessibility rule; a class-based one suits a component library.
- **Which Rust framework renders the client.** `team-practices.md` records that
  Leptos, Yew and Dioxus all render real DOM, so a CSS framework is compatible
  with any of them — but their idioms for applying classes differ.

Recorded as an open question for Domain Design rather than guessed here.

## What the framework must satisfy — the selection criteria

Written now so Domain Design has a checkable list rather than a preference.

| # | Criterion | Why |
|---|---|---|
| 1 | CSS only, no JavaScript runtime | The client is Rust compiled to WASM. A framework requiring a JS runtime adds a second language to the build for styling alone |
| 2 | A permissive licence, recorded in the asset and dependency manifest | `team-practices.md` requires origin and licence for every third-party dependency, checked by `scripts/verify.sh` |
| 3 | Not Streetmix, and no Streetmix-derived assets | Affirmed prohibition in `project.md`; verifiable at the manifest and lockfile |
| 4 | Focus styles present and overridable | The framework must not ship `outline: none`; if it does, it is disqualified rather than patched |
| 5 | Native form controls styled, not replaced | A framework that replaces `<select>` with a scripted widget breaks the accessibility floor the whole set rests on |
| 6 | Works at 360 CSS pixels without horizontal body scroll | NFR4.4 |
| 7 | Its own contrast defaults meet WCAG 2.1 AA, or are fully overridable by tokens | The project cannot inherit a palette it cannot fix |
| 8 | Bundle size small enough to keep the WASM artifact plus CSS within the CI size ceiling | `team-practices.md` asserts a bundle-size ceiling as the budget guard |

A framework failing 3, 4, 5 or 7 is rejected outright. The rest are trade-offs.

## Tokens

Project tokens layer over the framework and are the source of truth where the
two disagree.

### Spacing

A 4px base scale. Named rather than numeric, so a change to the scale does not
require touching every call site.

| Token | Value | Used for |
|---|---|---|
| `space-xs` | 4px | Gaps inside a control |
| `space-sm` | 8px | Between adjacent controls; minimum gap between touch targets |
| `space-md` | 16px | Between form rows; drawer padding at 360px |
| `space-lg` | 24px | Between sections within the drawer |
| `space-xl` | 32px | Between major regions |
| `space-2xl` | 48px | Landing-page section rhythm |

### Type

| Token | Size | Weight | Used for |
|---|---|---|---|
| `type-hero` | 32px / 40px at 360px | 700 | S1 headline (h1) |
| `type-heading` | 20px | 600 | Street name (h2), dialog titles |
| `type-subheading` | 16px | 600 | "Connected streets:", group labels |
| `type-body` | 16px | 400 | Prose, explanations, the estimated-lane sentence |
| `type-value` | 16px | 500 | Dimensions and their qualifiers |
| `type-small` | 14px | 400 | Lane numbers, street lengths, timestamps |

**Nothing below 14px.** A 12px caption is where the estimated qualifier would
end up if the scale allowed it, and that qualifier is load-bearing.

### Colour

Stated as roles with contrast ratios, not as hex values — the palette is chosen
against the framework's defaults at implementation, and the ratios are the
contract.

| Token | Role | Contrast requirement |
|---|---|---|
| `fg-default` | Body text on the default surface | ≥ 4.5:1 |
| `fg-muted` | Secondary text — street lengths, timestamps | ≥ 4.5:1 (not 3:1; it carries real content) |
| `bg-surface` | Drawer, dialogs, cards | — |
| `bg-page` | Page behind the surface | — |
| `accent` | Primary action, current selection | ≥ 3:1 against adjacent surfaces as a UI component |
| `focus-ring` | Focus indicator | ≥ 3:1 against both the component and the surface behind it |
| `warn` | "Does not fit" | ≥ 4.5:1 as text; never the sole carrier of the state |
| `unknown` | "Fit could not be checked" | ≥ 4.5:1 as text; visually distinct from `warn` in shape as well as hue |
| `lane-mapped` | A lane whose width is measured | ≥ 3:1 against the drawer surface |
| `lane-inferred` | A lane whose width is estimated | ≥ 3:1, and its hatch ≥ 3:1 against its own fill |
| `lane-userset` | A lane the user corrected or entered | ≥ 3:1, distinct in pattern from `lane-inferred` |

**Colour is never the sole carrier of any state in this product.** Every state
in the table above has a shape, pattern or word alongside it. That is a WCAG
1.4.1 requirement and, independently, the reason the provenance model is
legible at all.

### Elevation and radius

| Token | Value | Used for |
|---|---|---|
| `radius-sm` | 4px | Inputs, buttons |
| `radius-md` | 8px | Cards, drawer top corners |
| `shadow-sheet` | Subtle upward shadow | Bottom sheet against the map |
| `shadow-modal` | Larger, with a scrim | S5, S7, S8 |

## Components

Ten, mapped to framework primitives where one exists and built where it does
not.

| Component | Framework primitive | Project-owned | Specified in |
|---|---|---|---|
| Button (primary, secondary, destructive) | Yes — native `<button>`, framework-styled | Token overrides only | — |
| Text input | Yes — native `<input>` | Token overrides only | — |
| Select | Yes — native `<select>` | Token overrides only. **Never replaced by a scripted widget** | — |
| Checkbox group | Yes — native, in a `<fieldset>` with `<legend>` | Fit-status label composition | `interaction-spec.md` — Corridor checklist |
| Toggle | Partially — a styled checkbox | The on/off label in the accessible name | `interaction-spec.md` — S7 link toggle |
| Card | Yes | Sharing-state line | — |
| Dialog | Native `<dialog>` where supported | Focus trap, Escape, scrim | `interaction-spec.md` — focus rules |
| **Bottom sheet** | No | Entirely project-owned | `interaction-spec.md` — BottomSheet |
| **Lane strip** | No | Entirely project-owned, including provenance rendering and activation targets | `interaction-spec.md` — LaneStrip |
| **Map overlay legend** | No | Entirely project-owned | `interaction-spec.md` — Map overlays |

The three project-owned components are the three the framework cannot supply
because they are this product rather than generic UI. That is the expected
shape: a framework carries the commodity, and the thing that makes the product
different is built.

## Responsive breakpoints

| Name | Range | Layout |
|---|---|---|
| Phone | 360–767px | Two-height bottom sheet; single column; corridor rows wrap to two lines; legend collapsed |
| Tablet | 768–1023px | Fixed drawer, lower half; map above |
| Desktop | 1024px+ | Fixed drawer, lower third; map above; content max-width ~1200px |

**360px is the floor, not a target** (NFR4.4). Below it is unsupported and need
not be tested. The mobile pass/fail matrix is NFR4.6: current and previous major
version of Safari on iOS and Chrome on Android. Desktop support is assumed via
current evergreen browsers, with no matrix established — `requirements.md` OQ6
carries that gap, and it must be settled before the accessibility CI tier gets
its browser list.

## What this file does not decide

- The framework itself — open, for Domain Design.
- Concrete hex values — chosen at implementation against the framework's
  defaults; the contrast ratios above are the contract.
- Iconography — no icon is load-bearing in this design. Every state carries a
  word. If icons are added later they are supplementary, and each needs an
  accessible name and a manifest entry for its licence.
- Whether the hatch pattern holds contrast at small sizes — recorded in
  `accessibility-checklist.md` with its fallback.
