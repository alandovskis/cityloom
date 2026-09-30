---
name: CityLoom
description: A civil drafting sheet for rearranging a street's fixed right-of-way. Black ink on cool bond paper, one blue pencil, one redline.
colors:
  sheet: "#f3f6f8"
  sheet-deep: "#e5eaee"
  ink: "#14181c"
  ink-soft: "#3b444c"
  ink-faint: "#59656f"
  rule: "#b4bcc3"
  blue-pencil: "#1d4ed8"
  blue-wash: "rgb(29 78 216 / 0.09)"
  redline: "#b8231a"
  red-wash: "rgb(184 35 26 / 0.09)"
typography:
  title:
    fontFamily: "Barlow Semi Condensed, Arial Narrow, Helvetica Neue, Arial, sans-serif"
    fontSize: "30px"
    fontWeight: 600
    lineHeight: 1
    letterSpacing: "0.02em"
  body:
    fontFamily: "Barlow Semi Condensed, Arial Narrow, Helvetica Neue, Arial, sans-serif"
    fontSize: "15px"
    fontWeight: 400
    lineHeight: 1.35
    letterSpacing: "normal"
    fontFeature: "tnum"
  note:
    fontFamily: "Barlow Semi Condensed, Arial Narrow, Helvetica Neue, Arial, sans-serif"
    fontSize: "14px"
    fontWeight: 500
    lineHeight: 1.35
    letterSpacing: "normal"
  label:
    fontFamily: "Barlow Semi Condensed, Arial Narrow, Helvetica Neue, Arial, sans-serif"
    fontSize: "13px"
    fontWeight: 600
    lineHeight: 1.35
    letterSpacing: "0.12em"
  field-label:
    fontFamily: "Barlow Semi Condensed, Arial Narrow, Helvetica Neue, Arial, sans-serif"
    fontSize: "11px"
    fontWeight: 500
    lineHeight: 1.35
    letterSpacing: "0.1em"
  dimension:
    fontFamily: "Barlow Semi Condensed, Arial Narrow, Helvetica Neue, Arial, sans-serif"
    fontSize: "16px"
    fontWeight: 500
    lineHeight: 1.35
    letterSpacing: "normal"
    fontFeature: "tnum"
rounded:
  none: "0px"
spacing:
  hair: "1px"
  heavy: "2px"
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "18px"
  xl: "20px"
  rail: "22px"
  frame: "10px"
components:
  button-tool:
    backgroundColor: "{colors.sheet}"
    textColor: "{colors.ink}"
    typography: "{typography.label}"
    rounded: "{rounded.none}"
    padding: "4px 14px"
    height: "34px"
  button-tool-hover:
    backgroundColor: "{colors.sheet-deep}"
  button-tool-active:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.sheet}"
  button-tool-disabled:
    backgroundColor: "transparent"
    textColor: "{colors.ink-faint}"
  unit-toggle-pressed:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.sheet}"
    rounded: "{rounded.none}"
    padding: "4px 10px"
    height: "34px"
  field-width:
    backgroundColor: "{colors.sheet}"
    textColor: "{colors.ink}"
    rounded: "{rounded.none}"
    padding: "2px 6px"
    height: "30px"
    width: "68px"
  field-width-invalid:
    textColor: "{colors.redline}"
  icon-button:
    backgroundColor: "{colors.sheet}"
    textColor: "{colors.ink}"
    rounded: "{rounded.none}"
    size: "30px"
  icon-button-danger-hover:
    backgroundColor: "{colors.redline}"
    textColor: "{colors.sheet}"
  schedule-row-selected:
    backgroundColor: "{colors.blue-wash}"
    textColor: "{colors.blue-pencil}"
  legend-row:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    typography: "{typography.body}"
    padding: "3px 6px 3px 0"
  title-block-cell:
    textColor: "{colors.ink}"
    typography: "{typography.field-label}"
    padding: "5px 18px 6px"
---

# Design System: CityLoom

## Overview

**Creative North Star: "The Typical Section Sheet"**

The interface is a civil drafting sheet, not an app window. One double-ruled border encloses the whole viewport; tick margins numbered 1 to 8 across the top and A to D down the left frame the work; a title block closes the right column. The ground is cool bond paper, everything is drawn in black ink at graded line weights, and only two other colors exist: a blue pencil for what is selected or focused, and a redline for what does not fit. It is recognisable with all content removed by its border, tick margins and title block.

Density is that of a working sheet: small condensed capitals for labels, tabular figures for every dimension, hairline rules between rows. There is no card, no rounded corner, no fill color and no call-to-action button. The primary action is dragging a segment inside the drawing. Drafting devices (dimension strings, revision clouds, a graphic scale bar, a schedule table, a revision table) carry the information that an app would put in chrome.

Segment types are never told apart by color. Each has its own hatch texture and its own two-letter mark, so the drawing survives greyscale and colour-blindness.

**Key Characteristics:**
- One sheet, one border (2px ink plus a 1px outer rule), square corners everywhere.
- Black ink on cool paper; blue pencil for selection, redline for failure, nothing else chromatic.
- Hatch pattern plus mark, never colour, encodes segment type.
- Tabular figures and condensed capitals; sizes stay in a narrow 11 to 16px band, with only the sheet title larger.
- Flat: depth comes from line weight and ruled divisions, not shadow.

## Colors

Cool bond paper, graphite ink, and two pencils. Chroma is confined to blue and red, and both are rare.

### Primary
- **Blue Pencil** (#1d4ed8): the only selection and focus color. Focus ring (2px, 2px offset), text caret and text selection, selected segment box and its dimension and mark text, the selected schedule row (text plus 9% wash), the current revision letter and row, drag ghost, insertion caret, and handle hover fill. Its meaning is "this is what you are acting on".
- **Blue Wash** (rgb(29 78 216 / 0.09)): tint under selected schedule row, current revision row, and the drag ghost.

### Secondary
- **Redline** (#b8231a): the only failure color. Over-width wash, revision cloud, over-width dimension string and its text, failed checks, an invalid width field, and the danger hover on delete. If red is on the sheet, something does not fit.
- **Red Wash** (rgb(184 35 26 / 0.09)): tint over the overflow zone.

### Neutral
- **Bond Paper** (#f3f6f8): the sheet ground, every control fill, and text on inverted (ink) surfaces.
- **Bond Paper Deep** (#e5eaee): hover fill for tools, icon buttons and legend rows.
- **Black Ink** (#14181c): primary text, all structural lines (border, header rule, note headings, symbol outlines, hatch strokes, dimension lines), and the pressed state.
- **Soft Ink** (#3b444c): secondary text, column heads, keys, hints.
- **Faint Ink** (#59656f): tertiary text, tick-margin numerals, disabled text, right-of-way boundary text, drag-grip glyph, unassigned-space outline.
- **Rule Grey** (#b4bcc3): hairline separators between table rows and between tick-margin cells; disabled control borders. Never used for text.

### Named Rules
**The Two Pencils Rule.** Blue means selected, red means failed. No other color may be introduced, and neither may be used decoratively.

**The Hatch-Not-Hue Rule.** A segment type is identified by its hatch and its mark. Never assign it a fill color.

## Typography

**Display, Body and Label Font:** Barlow Semi Condensed (self-hosted woff2, weights 400, 500, 600; fallback Arial Narrow, Helvetica Neue, Arial, sans-serif)

**Character:** One condensed grotesque doing the work of drafting lettering: narrow, even-stroked, set in capitals with generous tracking for labels and in tabular figures for every number. Hierarchy comes from weight (500 vs 600), case and tracking, not from size.

### Hierarchy
- **Title** (600, 30px, line-height 1, 0.02em, uppercase; 26px under 640px): street name in the sheet header.
- **Body** (400, 15px, 1.35): hints, status line, fit statement; tabular figures on by default for the whole page.
- **Note** (500, 14px): note tables, sub-title, key legend, check detail.
- **Label** (600, 13px, 0.12em, uppercase): note headings (ruled underneath), tool buttons (0.1em), drawing labels "Existing", "Proposed", "Scale" (14px in the SVG).
- **Field label** (500, 11 to 12px, 0.08 to 0.12em, uppercase): tick-margin numerals, form-field captions, title-block cell captions, table column heads.
- **Dimension** (500, 16px, tabular): widths above segments, overall strings, scale bar numerals. Marks (SW, BL, TL...) are 600, 16px, 0.08em. Text over hatch carries a 4px paper-coloured halo.

### Named Rules
**The Tabular Rule.** Every number is a tabular figure, so a changing dimension does not shift its neighbours.

**The Caps-For-Labels Rule.** Labels are uppercase and tracked; values, names and sentences are not. Figures never uppercase.

## Layout

The sheet fills the viewport inside a 10px frame (6px under 640px). A 22px rail runs along the top and left, divided into 8 by 4 numbered and lettered zones. Inside: a header strip (title left, street picker, undo/redo/reset and units right), then a body of two columns, the work area and a fixed 340px notes column divided by a 1px ink rule. The work area stacks the drawing (Existing strip above Proposed section, same scale and origin), a status bar (fit statement, key hints), then a lower band of segment schedule (left) and legend (right, min 280px).

The drawing scales so the right-of-way fills the width minus margins; the SVG has a 680px minimum and scrolls horizontally on narrow screens, with vertical geometry scaled 0.9 to 1.1. Rhythm is tight: 4, 8, 12, 18, 20px paddings; table rows are 3px above and below. Structural dividers are 1px ink; the frame and the title block top edge are 2px.

Breakpoints: at 1100px the notes column drops below the work area with a 2px top rule; at 860px the schedule and legend stack; at 640px the rail is removed, the frame narrows, and the pattern swatch and type columns of the schedule are hidden.

### Named Rules
**The Ruled-Not-Boxed Rule.** Regions are separated by single ruled lines and table rows, never by cards, gaps of background, or nested boxes.

## Elevation & Depth

Flat. There are no shadows at rest and no layered surfaces; depth is line weight (2px border and title block edge, 1px structure, 0.8px dimension and boundary lines, 3.4px ground line). The one exception is state-only: the chip being dragged floats with a soft shadow (`0 6px 14px -4px rgb(20 24 28 / 0.28)`) and a blue border, because it is literally lifted off the sheet. A dragged segment is shown lifted by dropping its opacity to 0.35.

### Named Rules
**The Flat Sheet Rule.** Nothing rests above the paper. Only an object in the user's hand may cast a shadow.

## Shapes

Square. Every border radius is 0, on buttons, selects, inputs, chips and swatches. Forms are drawn as line art: rectangles with 2px ink outline over a hatch, handles as 18 by 16px square grips with a double-headed arrow, dimension strings as a line with 45-degree tick marks at each end (4px), right-of-way lines as long-dash-dot strokes, boundary lines as 5/4 dashes, selection as a 2px blue rectangle, overflow as a scalloped revision cloud in red drawn in over 620ms. Elevation symbols (person, tree, shrub, cyclist, car, bus, van, parking sign, median kerb) are single-weight line art, 1.2px non-scaling stroke, paper-filled, round joins.

Hatch vocabulary, 1px ink strokes on paper, one per segment type: sidewalk (dot stipple), planting (grass ticks), bike lane (45-degree lines), travel lane (staggered dashes), bus lane (tight reverse diagonal), parking (horizontal lines), median (crosshatch grid), loading (chevrons).

## Components

### Buttons (tools and unit toggle)
- **Shape:** square, 1px ink border, 34px tall, adjoining buttons overlap borders by 1px to read as one segmented strip.
- **Default:** paper fill, ink text, 13px 600 uppercase, 0.1em tracking, padding 4px 14px. Unit toggle is sentence case at 10px side padding.
- **Hover:** Bond Paper Deep fill. **Active (pressed):** ink fill, paper text, the same as the selected unit (aria-pressed).
- **Disabled:** Faint Ink text, Rule Grey border, transparent fill.
- **Focus:** 2px blue outline at 2px offset.
- Transitions: 120ms, cubic-bezier(0.16, 1, 0.3, 1), background and colour only; disabled under reduced motion.

### Inputs / Fields
- **Width field:** 68px by 30px, 1px ink border, no radius, paper fill, right-aligned tabular 500 text, unit tag beside it. **Invalid:** border and text turn redline. **Focus:** blue outline, blue caret.
- **Street select:** same square outline, custom 10px chevron drawn in ink, uppercase caption above at 11px.

### Symbol schedule and legend
- **Schedule:** ruled table, rows separated by 1px Rule Grey, columns Mark, Pattern swatch (44 by 22px hatch), Type, Width field, Actions. Selected row takes Blue Wash and blue mark/name.
- **Legend:** the palette is a symbol schedule as well, one ruled row per segment type (44px swatch, name, default width, small grip glyph), draggable, Bond Paper Deep on hover. Not cards.
- **Icon buttons:** 30px squares, 1px ink border, 14px 1.6px square-cap line icons; delete hovers redline with paper icon and stands 10px apart from the move arrows.

### Sheet notes, revisions and title block
- **Notes:** heading with 1px ink underline, right-aligned tabular tables, checks with 16px square-cap line icons; a failing check turns whole redline.
- **Revisions:** compact table, current row in Blue Wash, base row in Soft Ink, scrolls at 104px height below 1100px.
- **Title block:** 2 by 3 grid pinned to the bottom of the notes column, 2px ink top edge, 1px ink cell dividers, 11.5px captions over 16px uppercase values (Project, Street, Sheet, Date, Revision, Right-of-way).

### Section drawing (signature component)
The Existing strip and Proposed section share one origin and scale. Each proposed segment: line-art symbol standing on a 3.4px ground line, a 34px slab with 2px ink outline and hatch, its mark below, its width above on a dimension line. Overall strings below: right-of-way, then proposed total if different. Right-of-way lines dash-dot at both ends; a graphic scale bar of five alternating black and paper blocks closes the drawing. Selected segment: 2px blue box. Handles: square grips that turn blue-filled on hover or drag, and whose boundary line becomes a solid 2px blue. Over-width: red wash, scalloped red cloud, red second dimension string and "over" note.

## Do's and Don'ts

### Do:
- **Do** draw with ink lines at graded weights: 2px frame and title block edge, 1px structure, 0.8px dimensions.
- **Do** encode any new category with a new hatch texture and a two-letter mark before considering anything else.
- **Do** use blue only for the thing being acted on and red only for a failed fit.
- **Do** keep corners square and controls outlined in 1px ink on paper fill.
- **Do** set labels in tracked uppercase 600 and numbers in tabular figures.
- **Do** halo any label that sits over a hatch with 4px of paper.
- **Do** give every new control a 2px blue focus outline and honour prefers-reduced-motion.

### Don't:
- **Don't** colour segment types; hue is not an identifier here.
- **Don't** use cards, rounded corners, filled call-to-action buttons or icon-card sidebars.
- **Don't** add shadows to anything at rest.
- **Don't** introduce a third accent color or use blue and red decoratively.
- **Don't** replace line-art symbols with filled or glyph icons.
- **Don't** set body-scale text in a face other than Barlow Semi Condensed.
