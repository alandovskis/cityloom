<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
- 2026-09-09 — Read Q7 ("no overview screen in Stage 1") and AC8.1.4 ("device storage holds multiple designs") as two separable questions rather than a contradiction to resolve one way. Q8 kept both: no dedicated screen, and the map carries the overview role.
- 2026-09-09 — Read the approved Screen States "Success" row as needing a fourth wireframe correction, because Q5 moved undo out of the toast. The three corrections inherited from user-stories became four.

## Deviations
- 2026-09-09 — Built every ASCII diagram programmatically from a box helper rather than by hand, after the rough-mockups review found off-by-one padding across several frames. All 13 blocks verified even by measurement before writing.
- 2026-09-09 — Declined to name the CSS framework Q3 selected. The choice depends on the deferred DOM-versus-canvas decision and on which Rust framework renders the client, so `design-system-mapping.md` states eight selection criteria instead, four of them disqualifying.

## Tradeoffs
- 2026-09-09 — Specified provenance in three redundant channels (hatch, textual qualifier, accessible name) rather than one. Costs three implementations of one decision; each serves a reader the others do not reach, and the fallback if the hatch fails contrast is stated rather than discovered.
- 2026-09-09 — Kept the corridor flow on one screen per Q6, accepting that the checklist row must carry four fit states and wrap to two lines at 360 pixels, rather than adding a result screen.

## Open questions
- 2026-09-09 — Whether four map overlays plus a legend survive at 360 CSS pixels. The treatment is specified; that it works is not asserted, and it needs the first build to answer.
- 2026-09-09 — Whether the hatch pattern reaches 3:1 against its own lane fill at small sizes. If not, the textual and accessible-name channels carry provenance alone.
