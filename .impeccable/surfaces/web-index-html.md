---
version: 1
slug: "web-index-html"
primary_target: "web/index.html"
related_targets: []
---

# Surface brief: cross-section editor

Scope: one surface, `web/index.html`. Mode: Operate. Fidelity: working prototype, code-led (no image generation, no comp).

Audience and job: a resident, with no planning training, at a laptop or a projector at a community meeting. They open one street and rearrange its fixed right-of-way. Editing and live outcomes are one loop.

Task and content: drag, resize, add and delete segments within a fixed width; the width constraint, the existing street, and placeholder outcomes stay visible. Segment catalogue, rates and sample streets are synthetic and labelled so. Out of scope: persistence, save and share, the city map, a real outcomes model.

Constraints: Rust compiled to WebAssembly owns the editing model; JavaScript only draws and relays input. Every action works from the keyboard. WCAG 2.2 AA assumed. Open decisions: units, real segment catalogue, outcome measures, map handoff.

## Direction contract

THESIS: The editor is a civil drafting sheet, not an app window. The whole viewport is one sheet with a border, a title block and sheet notes. It refuses the flat cartoon street with a sidebar of icon cards.

OWN-WORLD: Cool bond-paper ground, black ink at graded line weights, hatch patterns (never colour alone) encoding each segment type, one blue pencil for selection and focus, one redline red for over-width and failed checks. Single-stroke condensed drafting lettering in caps for labels and tabular figures for dimensions. Dimension strings with ticks, a symbol schedule table, a revision table, line-art elevation symbols. Recognisable with all content removed by its border, tick margins and title block.

STORY: The resident sees a real-looking sheet with the existing street drawn beside their proposal at identical scale. They understand that the width is fixed, drag a bike lane in, watch the dimension string and sheet notes change, see the redline when it does not fit, and recover by resizing or removing.

FIRST VIEWPORT: Full-viewport sheet. Top strip: street name and right-of-way width at left; undo, redo, reset and units at right. Centre: a thin EXISTING strip above the PROPOSED section at true scale with line-art symbols, each segment with its width above and the overall dimension string below. Lower left: the symbol schedule (segment rows with width fields) and the palette. Right column: sheet notes with placeholder outcomes and checks, the revision table, and the title block at the bottom. No primary call-to-action; the primary action is dragging a segment.

FORM: Typical Section Sheet, the model's top-ranked grounded candidate (candidate 1 of 7, offered as the pick). Seed key 8f1cfbdb.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
