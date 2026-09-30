---
name: CityLoom
description: A softened drafting sheet for rearranging a street's fixed width. Black ink on cool bond paper by day, pale ink on a composed night sheet after dark; tints per segment type, one blue pencil, one redline, quiet rounded frame.
colors:
  sheet: "#e8edf0"
  sheet-deep: "#d9e1e6"
  ink: "#14181c"
  ink-soft: "#3b444c"
  ink-faint: "#59656f"
  rule: "#a6b0b8"
  blue-pencil: "#1d4ed8"
  blue-wash: "rgb(29 78 216 / 0.09)"
  redline: "#b8231a"
  red-wash: "rgb(184 35 26 / 0.09)"
  k-sidewalk: "#f2cfa6"
  k-planting: "#b6dc9f"
  k-bike: "#93d9cc"
  k-travel: "#bcc5ce"
  k-bus: "#cdb8ec"
  k-parking: "#dcd6c5"
  k-median: "#8fc281"
  k-loading: "#f5df7e"
  sky: "#e2eff8"
  s-tree: "#78b565"
  s-car: "#dbe2e9"
  s-bus: "#b79ee3"
  s-van: "#f0cb45"
  s-coat-0: "#ee9b78"
  s-coat-1: "#74aede"
  # dark theme values, one per colour token above (same names, dark- prefix)
  dark-sheet: "#13181d"
  dark-sheet-deep: "#1c232a"
  dark-ink: "#e8edf1"
  dark-ink-soft: "#b9c3cc"
  dark-ink-faint: "#93a0ab"
  dark-rule: "#3b4650"
  dark-blue-pencil: "#7fa6ff"
  dark-blue-wash: "rgb(127 166 255 / 0.16)"
  dark-redline: "#ff8073"
  dark-red-wash: "rgb(255 128 115 / 0.16)"
  dark-k-sidewalk: "#6a5238"
  dark-k-planting: "#41633a"
  dark-k-bike: "#2c6b63"
  dark-k-travel: "#46525e"
  dark-k-bus: "#5a4a7e"
  dark-k-parking: "#50493c"
  dark-k-median: "#3a6b31"
  dark-k-loading: "#7e6d1f"
  dark-sky: "#182838"
  dark-s-tree: "#55924a"
  dark-s-car: "#33404c"
  dark-s-bus: "#8068b8"
  dark-s-van: "#c0a02c"
  dark-s-coat-0: "#d27753"
  dark-s-coat-1: "#5590c4"
typography:
  title:
    fontFamily: "Barlow Semi Condensed, Arial Narrow, Helvetica Neue, Arial, sans-serif"
    fontSize: "32px"
    fontWeight: 600
    lineHeight: 1
    letterSpacing: "-0.005em"
  heading:
    fontFamily: "Barlow Semi Condensed, Arial Narrow, Helvetica Neue, Arial, sans-serif"
    fontSize: "16px"
    fontWeight: 600
    lineHeight: 1.35
    letterSpacing: "normal"
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
    fontSize: "14px"
    fontWeight: 600
    lineHeight: 1.35
    letterSpacing: "normal"
  table-head:
    fontFamily: "Barlow Semi Condensed, Arial Narrow, Helvetica Neue, Arial, sans-serif"
    fontSize: "13px"
    fontWeight: 600
    lineHeight: 1.35
    letterSpacing: "normal"
  field-label:
    fontFamily: "Barlow Semi Condensed, Arial Narrow, Helvetica Neue, Arial, sans-serif"
    fontSize: "13px"
    fontWeight: 500
    lineHeight: 1.35
    letterSpacing: "normal"
  dimension:
    fontFamily: "Barlow Semi Condensed, Arial Narrow, Helvetica Neue, Arial, sans-serif"
    fontSize: "16px"
    fontWeight: 500
    lineHeight: 1.35
    letterSpacing: "normal"
    fontFeature: "tnum"
rounded:
  sm: "4px"
  md: "6px"
  card: "10px"
  lg: "14px"
spacing:
  hair: "1px"
  heavy: "2px"
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "18px"
  xl: "20px"
  frame: "12px"
components:
  button-tool:
    backgroundColor: "{colors.sheet}"
    textColor: "{colors.ink}"
    typography: "{typography.label}"
    rounded: "{rounded.md}"
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
    rounded: "{rounded.md}"
    padding: "4px 10px"
    height: "34px"
  field-width:
    backgroundColor: "{colors.sheet}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    padding: "2px 6px"
    height: "30px"
    width: "68px"
  field-width-invalid:
    textColor: "{colors.redline}"
  icon-button:
    backgroundColor: "{colors.sheet}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    size: "30px"
  icon-button-danger-hover:
    backgroundColor: "{colors.redline}"
    textColor: "{colors.sheet}"
  legend-row:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    typography: "{typography.body}"
    padding: "3px 6px 3px 0"
  note-heading:
    textColor: "{colors.ink}"
    typography: "{typography.heading}"
    padding: "0 0 6px"
  about-card:
    backgroundColor: "{colors.sheet}"
    textColor: "{colors.ink}"
    typography: "{typography.field-label}"
    rounded: "{rounded.card}"
    padding: "8px 14px 9px"
---

# Design System: CityLoom

## Overview

**Creative North Star: "The Typical Section Sheet"**

The interface is a drafting sheet that has been softened for residents, not an engineering tool. One quiet 1px border with a 14px corner encloses the viewport; there are no tick rails, no double frame and no title block. The ground is cool bond paper, everything is drawn in black ink at graded line weights and then lightly coloured in: a pale tint per segment type, a pale sky behind the section, tinted symbols. Two saturated pencils sit above that colouring: a blue pencil for what is selected or focused, and a redline for what does not fit. It is recognisable with content removed by its cool paper, thin grey rules, rounded controls and the section drawing itself.

The sheet has two themes, light and dark, and the same drawing lives in both. It follows the system setting until the person chooses Light or Dark in the header, and remembers the choice. Dark is a composed night sheet, not an inversion: a blue-black ground, pale ink, deeper muted tints, and the same two pencils lifted to stay legible.

Density is that of a working sheet but the voice is plain: headings, table heads and drawing labels are sentence case in a condensed grotesque, tabular figures for every dimension, hairline rules between rows. Controls and the about-this-street card have gentle corners; there is no saturated fill and no call-to-action button. The primary action is dragging a piece inside the drawing. A left inspector holds the selected piece's width, surface and, for sidewalks and bike lanes, curb; it collapses like the notes and stacks below the drawing on narrow screens. The material and curb lists are synthetic placeholders and do not change any outcome. In Standard the tint and hatch name the kind of piece, a paving course along the top of each slab carries the surface hatch, and curb blocks carry the curb hatch. An opt-in Engineering drawing (avatar menu) redraws the same section as plain ink line work, hatched by surface material, with curbs, lane markings, extension lines, a sheet border and a title block; it is the one place a title block appears, and Standard stays the default. The drawing keeps its drafting devices (dimension strings, revision cloud, a graphic scale bar) and the panels carry the rest (pieces of the street, where the width goes, people moved, does it work, your changes).

Each segment type has its own light tint and its own hatch texture, and is named on the drawing when the segment is wide enough, with a two-letter code as the narrow fallback. The tint makes the sheet readable at a glance; the hatch and name carry the identity, so the drawing still survives greyscale and colour-blindness.

**Key Characteristics:**
- One sheet, one border (1px Faint Ink, 14px radius), controls at 6px, no rails.
- Black ink on cool paper, coloured in with pale tints; blue pencil for selection, redline for failure, and no other saturated accent.
- Tint plus hatch plus name (or two-letter code) encodes segment type; colour is never the only code.
- Tabular figures and sentence-case condensed type; sizes stay in a 13 to 16px band, with only the street title (32px) larger.
- Two themes, light and dark, sharing one structure: system-following until the person picks Light or Dark; dark is composed, not inverted.
- Flat: depth comes from line weight and quiet ruled divisions, not shadow.

## Colors

Cool bond paper, graphite ink, pale tints for what the drawing depicts, and two pencils for what the user does and what fails. Saturated chroma is confined to blue and red, and both are rare; the tints sit under the ink linework and never carry state. The hex values in Primary through Neutral are the light theme; the dark theme has the same tokens with the values under Night sheet below (frontmatter keys `dark-*`).

### Primary
- **Blue Pencil** (#1d4ed8): the only selection and focus color. Focus ring (2px, 2px offset), text caret and text selection, selected segment box and its dimension and name text, the "Change N" label on the drawing and the current row of Your changes, drag ghost, insertion caret, and handle hover fill. Its meaning is "this is what you are acting on".
- **Blue Wash** (rgb(29 78 216 / 0.09)): tint under the current change row and the drag ghost.

### Secondary
- **Redline** (#b8231a): the only failure color. Over-width wash, revision cloud, over-width dimension string and its "too wide" text, failed checks, an invalid width field, and the danger hover on remove. If red is on the sheet, something does not fit.
- **Red Wash** (rgb(184 35 26 / 0.09)): tint over the overflow zone.

### Tertiary (depiction tints)
Fills for what the drawing shows. Each is paired with ink hatch and outline, so none is ever the sole identifier. Used on the slab and Today-strip rectangles and the legend and inspector swatches; the mode chips in the Where the width goes table reuse five of them.
- **Sidewalk Sand** (#f2cfa6), **Planting Green** (#b6dc9f), **Bike Teal** (#93d9cc), **Travel Slate** (#bcc5ce), **Bus Lilac** (#cdb8ec), **Parking Stone** (#dcd6c5), **Median Green** (#8fc281, also the median kerb symbol), **Loading Yellow** (#f5df7e): the eight segment-type tints (k-sidewalk to k-loading in the frontmatter).
- **Sky** (#e2eff8): the pale rectangle behind the section elevation, above the ground line; no pointer events.
- **Symbol fills**: tree (#78b565, trees and shrubs), car (#dbe2e9), bus (#b79ee3), van (#f0cb45), and two coat variants for the person (#ee9b78, #74aede). Symbols keep their 1.2px ink outline; symbol parts without a fill stay paper.
- **Mode chips**: an 11px square with a 1px ink border and 3px radius before each row label in the Where the width goes table (Walking, Biking, Buses, Cars and trucks, Greenery), reusing tints: foot is Sidewalk Sand, bike is Bike Teal, transit is Bus Lilac, vehicle is Travel Slate, green is Planting Green. No new values.

### Neutral
- **Bond Paper** (#e8edf0): the sheet ground, every control fill, and text on inverted (ink) surfaces.
- **Bond Paper Deep** (#d9e1e6): hover fill for tools, icon buttons and legend rows.
- **Black Ink** (#14181c): primary text, control borders, symbol outlines, hatch strokes, dimension lines, and the pressed state.
- **Soft Ink** (#3b444c): secondary text, column heads, captions, hints.
- **Faint Ink** (#59656f): the sheet's outer border, tertiary text, disabled text, street-edge label text, drag-grip glyph, unassigned-space outline.
- **Rule Grey** (#a6b0b8): the quiet structural line: dividers between header, drawing, status bar, lower band and notes column, note-heading underlines, the about card border, hairlines between table rows, disabled control borders. Never used for text.

### Night sheet (dark theme)
Same token names, re-composed for a dark ground; every value is a `dark-*` key in the frontmatter. Applied when the person has chosen Dark, or when they have chosen nothing and the system prefers dark.
- **Night ground** (#13181d, `dark-sheet`): the sheet ground and every control fill. **Night Deep** (#1c232a): hover fills for tools, icon buttons and legend rows.
- **Pale Ink** (#e8edf1): primary text, control borders, symbol outlines, hatch strokes, dimension lines, the pressed state (pale fill, night text). **Soft Pale Ink** (#b9c3cc): secondary text. **Faint Pale Ink** (#93a0ab): the sheet border, tertiary and disabled text. **Night Rule** (#3b4650): dividers, hairlines, card border.
- **Blue Pencil, night** (#7fa6ff) with wash rgb(127 166 255 / 0.16); **Redline, night** (#ff8073) with wash rgb(255 128 115 / 0.16). Same jobs as in light; the washes are stronger (16% against 9%) so they still register on the dark ground.
- **Depiction tints, muted and deep**: sidewalk #6a5238, planting #41633a, bike #2c6b63, travel #46525e, bus #5a4a7e, parking #50493c, median #3a6b31, loading #7e6d1f. **Sky** #182838.
- **Symbol fills**: tree #55924a, car #33404c, bus #8068b8, van #c0a02c, coats #d27753 and #5590c4.
- Hatch strokes and dot stipple follow Pale Ink, so hatch still reads over the darker tints. Label halos follow the night ground.

### Chrome colour
- **Chrome Teal** (light #0f4f4c, dark #0f4744, ink #f3f6f8 / #e8edf1): the top bar behind the logo, wordmark and avatar. It is the mark's own teal taken deep, so the brand sits in the chrome and nowhere in the drawing.
- **Chrome Wash** (light #e3f0ee, dark #14302f) with **Chrome Line** (#b7d3cf / #2f5654): the header band and its bottom rule.
- **Weave Stripe:** a 5px strip under the top bar in the mark's five band colours (#e59a4a, #5fae4a, #2aa898, #8a6fd1, #e0c23a) at 20/14/32/20/14%, the same proportions as the logo. Fixed in both themes.
- The avatar's focus ring is Chrome Ink on the teal bar (blue would not reach contrast there). The account menu is a paper panel and keeps Blue focus.

### Named Rules
**The Two Pencils Rule.** Blue means selected, red means failed. Neither may be used decoratively, and no tint may borrow either hue to mean something else. It holds in both themes: in dark, blue and red are the lighter pencils (#7fa6ff, #ff8073) and stay reserved for selection and failure; no night tint is blue or red enough to be mistaken for them.

**The Composed Night Rule.** Dark is designed, not computed: a new colour token gets a hand-picked dark value alongside its light one, tints go darker and muted while ink goes pale, and nothing is produced by inverting or filtering the light theme.

**The Colour-Plus-Hatch-Plus-Name Rule.** A segment type is identified by its tint, its hatch and its name together (the two-letter code stands in for the name only when the segment is too narrow). Colour is never the only code: every tint ships with a hatch and a name and code, and a new category needs all three. It holds in dark: the tints are darker, the hatch strokes use the pale ink, and the name or code stays.

## Typography

**Display, Body and Label Font:** Barlow Semi Condensed (self-hosted woff2, weights 400, 500, 600; fallback Arial Narrow, Helvetica Neue, Arial, sans-serif)

**Character:** One condensed grotesque, set in sentence case at natural spacing, with tabular figures for every number. Hierarchy comes from weight (500 vs 600) and size, not from case or tracking.

### Hierarchy
- **Title** (600, 32px, line-height 1, -0.005em, sentence case): street name in the sheet header. The 26px size under 640px in an earlier rule is overridden in the build, which stays at 32px.
- **Body** (400, 15px, 1.35): hints, status line, fit statement, the "Street cross-section" sub-title; tabular figures on by default for the whole page.
- **Note** (500, 14px): note tables, key legend, check detail.
- **Heading** (600, 16px): panel headings (Add a piece, Where the width goes, People moved, Does it work?, Your changes), with a 1px Rule Grey underline.
- **Label** (600, 14px): tool buttons (Undo, Redo, Start over), drawing labels "Today", "Your design", "Street edge", "Unused", "Scale" (SVG, 0.01em).
- **Table head** (600, 13px): column heads of the notes tables. The unit tag beside width fields and the legend sub-text are 12px. Key caps (kbd) are 13px 600.
- **Field label** (500, 13px): the about-card captions.
- **Dimension** (500, 16px, tabular): widths above segments, overall strings ("Street width", "Your design"), scale bar numerals. Segment names on the drawing are 600, 15px; when the segment is narrow the two-letter code is shown instead. Text over hatch carries a 4px paper-coloured halo.

### Named Rules
**The Tabular Rule.** Every number is a tabular figure, so a changing dimension does not shift its neighbours.

**The Plain-Words Rule.** Headings, labels, table heads, buttons and drawing text are sentence case at normal letter-spacing and use resident language (Today, Your design, Street width, Add a piece). No tracked capitals. The only capitals are the two-letter segment codes.

## Layout

The sheet fills the viewport inside a 12px frame (6px under 640px), as a single block with a 1px Faint Ink border and 14px radius (10px under 640px), clipping its content to the corner. Inside: a top bar (the CityLoom mark and wordmark left, and the avatar menu right) over a header (street name and "Street cross-section" sub-title left, and Undo/Redo/Start over plus a Hide notes / Show notes button right), then a body of two columns, the work area and a fixed 340px notes column, divided by a 1px Rule Grey line. The work area stacks the drawing (Today strip above Your design section, same scale and origin), a status bar (fit statement, key hints), then a lower band of Pieces of the street (left) and Add a piece (right, min 280px). The notes column holds Where the width goes, People moved, Does it work?, Your changes, and the about-this-street card pinned at the bottom.

The drawing scales so the street width fills the width minus margins; the SVG has a 680px minimum and scrolls horizontally on narrow screens, with vertical geometry scaled 0.9 to 1.1. Rhythm is tight: 4, 8, 12, 18, 20px paddings; table rows are 3px above and below. All region dividers are 1px Rule Grey.

Breakpoints: at 1100px the notes column drops below the work area with a 1px Rule Grey top line; at 640px the frame narrows to 6px.

### Named Rules
**The Quiet-Rule Rule.** Regions are separated by single 1px Rule Grey lines and table rows. The one enclosed box is the about-this-street card; legend entries are ruled rows, never cards, and nothing nests inside another box.

## Elevation & Depth

Flat in both themes. There are no shadows at rest and no layered surfaces; depth is line weight (1px border and dividers, 2px object outlines in the drawing, 0.8px dimension and boundary lines, 3.4px ground line). The one exception is state-only: the chip being dragged floats with a soft shadow (`--shadow-lift`: `0 6px 14px -4px rgb(20 24 28 / 0.28)` in light, `0 8px 18px -4px rgb(0 0 0 / 0.55)` in dark) and a blue border, because it is literally lifted off the sheet. A dragged segment is shown lifted by dropping its opacity to 0.35.

### Named Rules
**The Flat Sheet Rule.** Nothing rests above the paper. Only an object in the user's hand may cast a shadow.

## Shapes

Softly rounded, with a small ladder of radii: sheet 14px (`--radius-lg`), about-this-street card 10px, controls 6px (`--radius`: buttons, width inputs, icon buttons, the drag chip), 4px on swatches and key caps, 3px on mode chips. The m/ft toggle is one joined pair, rounded only at its outer ends. Legend rows are square ruled rows, not rounded cards. The drawing itself stays hard-edged: rectangles with 2px ink outline over a hatch, handles as 18 by 16px square grips with a double-headed arrow, dimension strings as a line with 45-degree tick marks at each end (4px), street-edge lines as long-dash-dot strokes, boundary lines as 5/4 dashes, selection as a 2px blue rectangle, overflow as a scalloped revision cloud in red drawn in over 620ms. Elevation symbols (person, tree, shrub, cyclist, car, bus, van, parking sign, median kerb) are single-weight line art, 1.2px non-scaling stroke, round joins.

Hatch vocabulary, 1px ink strokes over the segment tint, one per segment type: sidewalk (dot stipple), planting (grass ticks), bike lane (45-degree lines), travel lane (staggered dashes), bus lane (tight reverse diagonal), parking (horizontal lines), median (crosshatch grid), loading (chevrons).

## Components

### Buttons (tools and unit toggle)
- **Notes toggle:** a tool button with a panel icon and the label "Hide notes" or "Show notes" (aria-expanded, aria-controls, aria-keyshortcuts `]`). It, or the `]` key outside a text field, collapses the 340px notes column so the drawing takes the full width; the choice is remembered in localStorage and applied before first paint.
- **Tool icons:** Undo, Redo and Start over each carry a 16px line icon (1.6px round-cap stroke in the button's text colour, so it follows hover, pressed and disabled) 6px before the label. The label stays; icons never replace it.
- **Shape:** 6px radius, 1px ink border, 34px tall. Undo, Redo and Start over sit apart with a 6px gap; the m/ft toggle is joined (borders overlap by 1px) with outer corners rounded.
- **Default:** paper fill, ink text, 14px 600 sentence case, no tracking, padding 4px 14px. Unit toggle is 10px side padding.
- **Hover:** Bond Paper Deep fill. **Active (pressed):** ink fill, paper text, the same as the selected unit (aria-pressed).
- **Disabled:** Faint Ink text, Rule Grey border, transparent fill.
- **Focus:** 2px blue outline at 2px offset.
- Transitions: 120ms, cubic-bezier(0.16, 1, 0.3, 1), background and colour only; disabled under reduced motion.

### Theme toggle
- The Light | Dark pair in the avatar menu is the same joined 34px toggle (6px outer corners, pressed = inked fill). Its pressed state shows the theme in effect, so with no saved choice it tracks the system live.
- Behaviour: follows the system (`prefers-color-scheme`) until a choice is made; the choice is stored in localStorage under `cityloom-theme` (storage failures are tolerated, and the choice then lasts the visit). A small inline script in the page head applies the stored choice before first paint, and the page declares `color-scheme: light dark`. The choice is announced in the status line ("Dark theme.").

### Avatar and account menu
- A 34px circle at the far right of the top bar: 1px Ink outline, Bond Paper Deep fill (Bond Paper on hover and while open), a line-art head-and-shoulders in 1.6px round-cap ink. It is a button (aria-label "Settings", aria-expanded) that opens a disclosure panel below it, right-aligned: 14px radius, 1px Faint Ink border, Bond Paper fill, the lift shadow (the one resting exception, because it floats over content), 220px minimum. The panel holds a "Guest" heading (16px 600, Rule Grey underline), then a Units row and a Theme row, each a label and a joined toggle (m | ft, Light | Dark). It closes on Escape (focus returns to the avatar), on an outside press, and when focus leaves it. There are no accounts yet, so "Guest" is a label, not a sign-in.

### Inputs / Fields
- **Width field:** 68px by 30px, 1px ink border, 6px radius, paper fill, right-aligned tabular 500 text, unit tag beside it. **Invalid:** border and text turn redline. **Focus:** blue outline, blue caret.

### Add a piece
- **Pieces of the street:** there is no longer a table of pieces. Width, surface and curb are edited in the left inspector; moving and removing pieces is done on the drawing with the keyboard.
- **Add a piece:** the palette is one ruled row per segment type (44px swatch, name, default width, small grip glyph), draggable, Bond Paper Deep on hover. Not cards.
- **Icon buttons:** 30px, 1px ink border, 6px radius, 14px 1.6px square-cap line icons; remove hovers redline with paper icon and stands 10px apart from the move arrows.

### CityLoom mark
- **Mark:** `web/logo.svg`, a woven street section: five vertical bands of unequal widths in the sidewalk, planting, bike, travel and loading pencils (#e59a4a, #5fae4a, #2aa898, #8a6fd1, #e0c23a), woven together by four straight ink threads that pass over and under alternate bands, each thread offset from its neighbour like a basket weave; the bands stand for the modes of travel. Colours are fixed and identical in both themes. The mark and wordmark appear once, as a lockup at the top left of the top bar, mark 30px (28px under 640px); they are not repeated in the about card.
- **Favicon:** `web/favicon.svg` is the same mark with the threads switching to Night Ink in dark schemes.
- **Wordmark:** "CityLoom" in Barlow Semi Condensed 600 (20px, 18px on phones), live text beside the mark in the top bar, which is closed by a 1px Rule Grey line; never outlined into the SVG. Below 24px the weave blurs; use the favicon there.
- **Do not** recolour the bands, put a container or shadow around the mark, or show it under 16px.

### Notes and the about-this-street card
- **Notes:** sentence-case heading (16px 600) with a 1px Rule Grey underline, right-aligned tabular tables, checks with 16px square-cap line icons; a failing check turns whole redline.
- **Your changes:** compact table of numbered steps, current row in Blue Wash, "Street today" base row (marked with a dash) in Soft Ink, scrolls at 104px height below 1100px.
- **About this street card:** 2-column grid pinned to the bottom of the notes column, 10px radius, 1px Rule Grey border and dividers, 16px below. A full-width Street row, then Width and Changes made. Captions 13px over 17px 600 values, all sentence case.

### Section drawing (signature component)
The Today strip and Your design section share one origin and scale. Each proposed segment: line-art symbol standing on a 3.4px ground line, a 34px slab with 2px ink outline, its segment tint and hatch over it, its name below (two-letter code when narrow), its width above on a dimension line. Overall strings below: "Street width", then "Your design" total if different. Street-edge lines dash-dot at both ends, labelled "Street edge"; a graphic scale bar of five alternating black and paper blocks closes the drawing. Selected segment: 2px blue box, with a blue "Change N" label after a change. Handles: square grips that turn blue-filled on hover or drag, and whose boundary line becomes a solid 2px blue. Over-width: red wash, scalloped red cloud, red second dimension string and "too wide" note.

## Do's and Don'ts

### Do:
- **Do** draw with ink lines at graded weights: 1px structure and borders, 2px object outlines, 0.8px dimensions.
- **Do** encode any new category with a light tint, a new hatch texture, a name and a two-letter code together.
- **Do** keep tints pale enough that 1px ink hatch and 15 to 16px names stay legible over them.
- **Do** use blue only for the thing being acted on and red only for a failed fit.
- **Do** use the radius ladder (6px controls, 10px card, 14px sheet) and outline controls in 1px ink on paper fill.
- **Do** set headings and labels in sentence case, plain resident wording, and numbers in tabular figures.
- **Do** give every new colour token a light value and a hand-set dark value together, and check tints against hatch and ink in both themes.
- **Do** halo any label that sits over a hatch with 4px of paper.
- **Do** give every new control a 2px blue focus outline and honour prefers-reduced-motion.

### Don't:
- **Don't** let a tint stand alone as the identifier of a type, or use a tint as a state signal.
- **Don't** add cards beyond the about-this-street card, nested boxes, filled call-to-action buttons or icon-card sidebars.
- **Don't** add shadows to anything at rest, in either theme.
- **Don't** derive the dark theme by inverting or filtering the light one, and don't let a dark tint drift toward blue or red.
- **Don't** introduce a third saturated accent or use blue and red decoratively.
- **Don't** replace line-art symbols with filled or glyph icons.
- **Don't** set body-scale text in a face other than Barlow Semi Condensed.
