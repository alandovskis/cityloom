---
name: CityLoom
description: The category-standard resident map tool. A full-bleed map or one white plate under a few floating white cards, soft 16px corners, system type, one blue for what is chosen, one amber for what needs attention, green only for "passes".
colors:
  land: "#e2e7ec"
  panel: "#ffffff"
  ink: "#111827"
  ink-2: "#4b5563"
  ink-3: "#5f6b7a"
  rule: "#e5e8ec"
  fill: "#f1f3f5"
  field-line: "#cdd3da"
  accent: "#2563eb"
  accent-ink: "#ffffff"
  accent-wash: "#e8effd"
  warn: "#92400e"
  warn-wash: "#fef3c7"
  warn-line: "#f59e0b"
  ok: "#166534"
  ok-wash: "#dcfce7"
  danger: "#b8231a"
  casing: "#7d8996"
  k-sidewalk: "#cfd7de"
  k-planting: "#a5d696"
  k-bike: "#7fd3c3"
  k-travel-map: "#ffffff"
  k-travel-section: "#c3cbd3"
  k-bus: "#f2a99b"
  k-parking-map: "#b9c2cc"
  k-parking-section: "#b4bdc7"
  k-median: "#93c98a"
  k-loading: "#f6d777"
  k-shoulder: "#cbd3db"
  k-bikerack: "#cdd6e2"
  k-bikeshare: "#b5c7e8"
  k-pole: "#9aa4b0"
  k-busshelter: "#f6c9bf"
  k-busstation: "#eeb8ab"
  k-bench: "#d8c8ad"
  k-terrace: "#f1dca0"
  k-streetlamp: "#d9dcc0"
  sky: "#e9f3fb"
  s-tree: "#6fb35e"
  s-car: "#dfe5ec"
  s-van: "#f1cd55"
  s-coat-0: "#ee9b78"
  s-coat-1: "#74aede"
  dark-land: "#0f1317"
  dark-panel: "#1b2127"
  dark-ink: "#e8edf2"
  dark-ink-2: "#b7c1cb"
  dark-ink-3: "#9aa6b2"
  dark-rule: "#2c343c"
  dark-fill: "#252d35"
  dark-field-line: "#3b454f"
  dark-accent: "#8ab4ff"
  dark-accent-ink: "#0b1220"
  dark-accent-wash: "rgb(138 180 255 / 0.16)"
  dark-warn: "#fcd34d"
  dark-warn-wash: "rgb(245 158 11 / 0.18)"
  dark-warn-line: "#f59e0b"
  dark-ok: "#86efac"
  dark-ok-wash: "rgb(34 197 94 / 0.16)"
  dark-danger: "#ff8073"
  dark-casing: "#6a7886"
  dark-road: "#3a4550"
  dark-k-sidewalk: "#27303a"
  dark-k-planting: "#3c6a39"
  dark-k-bike: "#2f7468"
  dark-k-bus: "#8a4b43"
  dark-k-parking-map: "#2f3943"
  dark-k-parking-section: "#3d4750"
  dark-k-travel-section: "#333c45"
  dark-k-median: "#3f7438"
  dark-k-loading: "#8a7a2b"
  dark-k-shoulder: "#232b33"
  dark-k-bikerack: "#46525f"
  dark-k-bikeshare: "#3f5a85"
  dark-k-busshelter: "#7a4039"
  dark-k-busstation: "#6d3832"
  dark-k-bench: "#6e5d48"
  dark-k-terrace: "#7d6a30"
  dark-k-streetlamp: "#5d6148"
  dark-sky: "#17222e"
  dark-s-tree: "#55924a"
  dark-s-car: "#3a4651"
  dark-s-van: "#c0a02c"
  dark-s-coat-0: "#d27753"
  dark-s-coat-1: "#5590c4"
typography:
  display:
    fontFamily: "ui-sans-serif, system-ui, -apple-system, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif"
    fontSize: "clamp(34px, 5vw, 56px)"
    fontWeight: 700
    lineHeight: 1.04
    letterSpacing: "-0.03em"
  headline:
    fontFamily: "ui-sans-serif, system-ui, -apple-system, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif"
    fontSize: "36px"
    fontWeight: 700
    lineHeight: 1.1
    letterSpacing: "-0.02em"
  title:
    fontFamily: "ui-sans-serif, system-ui, -apple-system, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif"
    fontSize: "24px"
    fontWeight: 700
    lineHeight: 1.15
    letterSpacing: "-0.015em"
  body:
    fontFamily: "ui-sans-serif, system-ui, -apple-system, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif"
    fontSize: "15px"
    fontWeight: 400
    lineHeight: 1.45
    fontFeature: "tnum"
  label:
    fontFamily: "ui-sans-serif, system-ui, -apple-system, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif"
    fontSize: "13px"
    fontWeight: 600
    lineHeight: 1.45
rounded:
  card: "16px"
  search: "14px"
  ctl: "12px"
  field: "10px"
  tab: "9px"
  kbd: "6px"
  pill: "999px"
spacing:
  pad: "16px"
  gap-stage: "12px"
  gap-ctl: "8px"
  bar-h: "64px"
  panel-w: "360px"
  panel-w-editor: "300px"
  notes-w: "340px"
components:
  card:
    backgroundColor: "{colors.panel}"
    textColor: "{colors.ink}"
    rounded: "{rounded.card}"
  button:
    backgroundColor: "{colors.panel}"
    textColor: "{colors.ink}"
    typography: "{typography.label}"
    rounded: "{rounded.ctl}"
    padding: "0 14px"
    height: "40px"
  button-hover:
    backgroundColor: "{colors.fill}"
  button-quiet:
    backgroundColor: "transparent"
    textColor: "{colors.ink-2}"
    rounded: "{rounded.ctl}"
  button-quiet-open:
    backgroundColor: "{colors.accent-wash}"
    textColor: "{colors.accent}"
  button-add:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.accent-ink}"
    rounded: "{rounded.ctl}"
    padding: "0 18px"
  search-bar:
    backgroundColor: "{colors.land}"
    textColor: "{colors.ink}"
    rounded: "{rounded.search}"
    height: "46px"
  search-hero:
    backgroundColor: "{colors.panel}"
    rounded: "{rounded.search}"
    height: "64px"
  status-chip-ok:
    backgroundColor: "{colors.panel}"
    textColor: "{colors.ok}"
    rounded: "{rounded.pill}"
    padding: "10px 18px"
  status-chip-bad:
    backgroundColor: "{colors.warn-wash}"
    textColor: "{colors.warn}"
    rounded: "{rounded.pill}"
    padding: "10px 18px"
  place-row-selected:
    backgroundColor: "{colors.accent-wash}"
    textColor: "{colors.ink}"
    rounded: "{rounded.field}"
  tab-selected:
    backgroundColor: "{colors.panel}"
    textColor: "{colors.ink}"
    rounded: "{rounded.tab}"
    height: "34px"
  chip-tray:
    backgroundColor: "{colors.panel}"
    rounded: "{rounded.ctl}"
    padding: "6px 10px 6px 6px"
  field:
    backgroundColor: "{colors.panel}"
    textColor: "{colors.ink}"
    rounded: "{rounded.field}"
    height: "36px"
---

# Design System: CityLoom

## Overview

**Creative North Star: "The Familiar Map"**

CityLoom looks like the map tool a resident already knows how to use, executed with full craft and sitting alongside Streetmix. The drawing is the page: on the map and home pages the city fills the viewport, and on the editors one large white plate holds the street section or junction plan. Everything else is a small number of quiet white cards floating over a cool grey-blue land. No theme, no costume. The product is for non-experts, so the interface is plainly legible and does not announce itself.

The system is deliberately narrow. System type, one 16px card, one two-layer shadow, one blue, one amber. Colour carries meaning rather than decoration: blue is what the person has chosen or can act on, amber is what does not fit, green appears only to say something passes. Both themes are designed, light by default, dark through `prefers-color-scheme` or an explicit `data-theme`.

The earlier drafting-sheet world (ink on bond paper, condensed type, hard hatch) is retired everywhere. Only a faint texture under flat lane tints survives in the editors, so colour is never the only code.

**Key Characteristics:**
- Floating white cards (16px radius, soft two-layer shadow) over a cool land, never a three-column admin dashboard.
- One blue, one amber, green only for "passes". Selection and links are blue; failures are amber with a cloud and a chip.
- System UI type, 15px body, 600 for names, tabular figures everywhere.
- Flat pale tints for lane and strip kinds, mid-grey linework, named pieces, so colour is not the only code.
- The logo: two linked junction rings, one blue and one amber, as the "oo" of Loom.
- Light and dark themes share the same token names and the same structure.

## Colors

A cool, near-neutral grey-blue ground with white cards, a single saturated blue, and a warm amber for trouble. Hues are used by role, never by section.

### Primary
- **Map Blue** (`accent`, #2563eb light; `dark-accent` #8ab4ff dark): selection, links, focus rings, the filled Search and Add buttons, selected rows, junction discs and their numerals, sliders, the logo's left ring. Its wash (`accent-wash`) is the selected or hover background for rows, tabs-in-use, quiet toggles that are open, and lane arrows. Text on filled blue is `accent-ink`.

### Secondary
- **Attention Amber** (`warn` #92400e text on `warn-wash` #fef3c7; `warn-line` #f59e0b for strokes and fills on the drawing; dark: `dark-warn` #fcd34d on an amber wash at 18%): anything that does not fit or fails. The status chip, failing check rows, the cloud over overflow, over-width dimension lines, amber badges on the map, the invalid-field border. It is also the logo's right ring.

### Tertiary
- **Pass Green** (`ok` #166534 on `ok-wash`; dark `dark-ok` #86efac): only the "it works" state: the status chip text and the checker's tick. Never a decorative or brand colour.
- **Destructive Red** (`danger` #b8231a; dark #ff8073): only the hover fill of destructive controls (delete, start over) in the editors. It is not a second warning colour.

### Neutral
- **Land** (`land` #e2e7ec; dark #0f1317): the page ground, the map ground, the search field at rest.
- **Panel White** (`panel` #ffffff; dark #1b2127): every card, bar, menu and plate.
- **Ink** (`ink` #111827), **Ink 2** (`ink-2` #4b5563), **Ink 3** (`ink-3` #5f6b7a): text, secondary text, and tertiary text and linework. Dark: #e8edf2, #b7c1cb, #9aa6b2.
- **Rule** (`rule` #e5e8ec) hairline dividers; **Fill** (`fill` #f1f3f5) hover and segmented-control wells; **Field Line** (`field-line` #cdd3da) borders of buttons and inputs.

### The drawing tints
Flat, pale, named. Lane and strip kinds: sidewalk #cfd7de, planting #a5d696, bike #7fd3c3, transit lane #f2a99b, parking (map) #b9c2cc, median #93c98a, loading #f6d777, shoulder #cbd3db, bike rack #cdd6e2, bike share #b5c7e8, pole #9aa4b0. The map draws travel lanes and roads white inside a `casing` #7d8996 outline; the street section draws travel and parking a little darker (#c3cbd3, #b4bdc7) so the asphalt reads on a white plate. The editors add sky #e9f3fb, tree #6fb35e, car #dfe5ec, van #f1cd55 and two pedestrian coat colours. In dark, each tint steps to a deep, lower-chroma equivalent (see frontmatter).

### Named Rules
**The One Blue Rule.** Blue means chosen or actionable, and nothing else. A screen has at most one filled blue control (Search on home, Add a piece on the street editor); everything else that is blue is a wash, a ring or a text colour.

**The Amber Is Trouble Rule.** Amber is reserved for what does not fit. If a thing is amber the person should be able to act on it. Do not use amber for emphasis, promotion or decoration.

**The Green Only Passes Rule.** Green appears only on "passes" states. A green element is a statement that the city, street or junction works.

**The Both Themes Rule.** Every colour token has a dark twin of the same name. A new token ships with both. Dark uses a lighter blue (#8ab4ff) and a lighter amber (#fcd34d) text, with washes at 16 to 18% alpha instead of pale fills.

## Typography

**Display, Body and Label Font:** the system UI stack (`ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif`). One family throughout.
**Logotype:** Overpass, outlined to paths in `brand.svg` and the logo files (SIL OFL 1.1, credited on the credits page). It is never loaded as a font and never used for page text.

**Character:** the plain, native type of the operating system. Weight does the work: 700 for titles, 600 for names and controls, 400 for reading. Tabular figures are on globally so widths and counts line up.

### Hierarchy
- **Display** (700, clamp(34px, 5vw, 56px), 1.04, -0.03em): the home headline only.
- **Headline** (700, 36px, 1.1, -0.02em): the headline of a page to read (credits).
- **Title** (700, 24px, 1.15, -0.015em): the place or street name in a title card or panel head. 20px/700 for the piece name in the inspector.
- **Section** (700, 15px; 14px in the inspector with a hairline beneath): note headings.
- **Body** (400, 15px, 1.45): rows, notes, fields. Lead copy 17 to 20px in `ink-2`; reading text capped at 62 to 72ch.
- **Label** (600, 13px; 12px for chips and table cells): sub-lines, hints, keys, status pills. Secondary text is 13 to 14px in `ink-2` or `ink-3`.
- Drawing text uses the same stack: 13px/600 labels, 15px dimensions and marks, with a panel-coloured halo where it crosses linework.

### Named Rules
**The Native Type Rule.** No web fonts and no second family. If a surface needs more voice, change weight or size, not face.

**The Legible Floor Rule.** Nothing smaller than 12px, and 12px only for pills and table-like cells. Body and controls are 14 to 16px so the interface reads on a phone and on a projector.

## Layout

The page is a stack of layers: the drawing at the back, floating cards on top. A fixed top **bar** (64px tall, 16px from the edges) holds the logo, the Map / Street / Intersection switcher, the search (map), the Piece details and Notes toggles and the avatar menu. Cards sit 16px from the viewport edge and 12px below the bar.

**Map page.** The map fills the viewport. Places card on the left (360px, full height), Notes card on the right (360px, as tall as its content), zoom group bottom-right, the status chip bottom-centre of the open area between the cards.

**Home page.** One hero card (up to 780px, 44px/48px padding) holds the headline, the large search with its blue Search button, the way in, and how the city stands. The sample city is drawn full-bleed behind it, inert, and eases in from the card's right edge with a gradient mask. Names are hidden on this backdrop.

**Editors (street and intersection).** A three-part arrangement under the bar: a left details card (300px), a right Notes card (340px, tabs), and a centre column between them. The centre column is a vertical stack with 12px gaps: a **stage head** (title card plus tool strip), the **plate** (the drawing on a white 16px card), the status chip, and for junctions two **tray** cards. Under 1360px wide the Notes card starts closed (the person can open it); under 900px every card stacks below the drawing and the bar wraps to a second and third row.

- *Street:* the title card sits at the left of the stage head; the tool strip (blue Add a piece, Undo, Redo, Start over) is a white 6px-padded card that wraps below the title card when the centre column is narrower than both together. At 1440px it reads as a toolbar under the title.
- *Junction:* the tool strip holds only Undo, Redo and Start over, which are narrow enough to sit to the right of the title card on the same row. Below the plan and chip come two tray cards side by side (Add a street, Start from; 1.4fr and 1fr). Everything fits at 1440x900 without scrolling.
- The strip is the same component in both editors. The two arrangements differ only in what the strip holds, and the wrap is the mechanism; do not hard-code either one.

**The camera-inset pattern.** On the map, any floating card (marked `data-covers`) tells the camera how far it reaches in from each edge. The camera fits and re-fits the city inside the open rectangle between the cards, with a small gap, so a "fit" view never ends up hidden behind a panel. Bar, Places and Notes are covers; closing a panel (toggle, `data-inspector`/`data-notes` = closed) changes its reach to 0 and the map re-fits. The CSS counterpart: `--lw` and `--rw` hold how far the left and right panels reach, and anything that floats in the open area (north arrow, scale bar, zoom group, status chip) is placed from them. A new floating card must be marked as a cover and use these variables, not fixed offsets.

**Spacing rhythm.** 16px page padding, 12px between stacked cards, 8px between controls in a strip, 10 to 12px row padding inside lists, 20px card padding in panels. Under 900px the page padding drops to 8px.

### Named Rules
**The Open Area Rule.** The drawing is always centred and fitted in what the cards leave open. Never place a card over the part of the drawing a person came to see.

## Elevation & Depth

Hybrid: a white card over a tinted land is the main depth cue, supported by two soft shadows. Nothing is ever flat-on-flat; nothing has a hard or offset shadow.

### Shadow Vocabulary
- **Float** (`box-shadow: 0 1px 2px rgb(15 23 42 / 0.08), 0 8px 24px rgb(15 23 42 / 0.12)`; dark `0 1px 2px rgb(0 0 0 / 0.4), 0 10px 28px rgb(0 0 0 / 0.5)`): the bar, side panels, the home hero card, menus, popovers, the zoom group, the status chip, the drag chip.
- **Soft** (`box-shadow: 0 1px 2px rgb(15 23 42 / 0.08), 0 2px 8px rgb(15 23 42 / 0.06)`; dark `0 1px 2px rgb(0 0 0 / 0.4), 0 2px 8px rgb(0 0 0 / 0.3)`): cards that sit on the page rather than over a map (editor title card, tool strip, drawing plate, tray cards, read card) and the raised selected segment of a tab or unit control.
- **Inset amber** (`inset 0 0 0 1px var(--warn-line)`, with Soft): the failing status chip's edge.
- **Focus ring** (`outline: 2px solid var(--accent); outline-offset: 2px`) on `:focus-visible`; fields add `0 0 0 3px var(--accent-wash)` on focus.

### Named Rules
**The Two Shadows Rule.** Float for what hovers over a drawing, Soft for what sits on the page. No third shadow, no hard or offset shadow.

**The Edge Fade Rule.** A scrolling panel body fades into its bottom padding with an 18px mask rather than ending in a hard cut.

## Shapes

Soft and rounded, from the card down. Cards are 16px (`rounded.card`); the search field 14px; buttons and tray chips 12px; fields, rows, pieces of a menu and icon buttons 10px; tabs inside a segmented control 9px; keycaps 6px; status chips and row badges are full pills. The avatar is a circle. A segmented control is an inset well (`fill`) holding a raised white segment.

Drawings are flat. Lane and strip kinds are filled rectangles with mid-grey linework (`ink-2` strokes at 0.8 to 1.6px) and a faint pattern at 30% opacity under the fill. Junctions on the map are white discs ringed in blue with a blue numeral. Selection is a 2.5px blue box; a drag ghost is a blue wash with a dashed blue outline; overflow is a 2px amber cloud that draws itself in once.

The logo's geometry (two overlapping rings, 7px strokes on a 46 by 30 viewbox) is the one recurring custom silhouette. Rounded ring forms may echo it in marks; they are not repeated as decoration.

## Components

### The Logo
Two linked junction rings, the "oo" of Loom: the left ring blue (`--logo-a`, defaulting to #2563eb, #8ab4ff in dark), the right ring amber (#f59e0b), overlapping so each ring crosses over the other at one side and under at the other (a clipped top half and bottom half re-drawn above the pair). The wordmark "CityLoom" is set in Overpass and outlined to paths, with the ring pair standing in for the two o's. Files: `brand.svg` (a sprite with `#wordmark` and `#mark` symbols, used in the bar at 30px tall via `<use>`), `logo.svg` (the mark alone, adapts to dark through a media query), `logo-wordmark.svg` and `logo-wordmark-dark.svg`, `favicon.svg` (white and amber rings on a blue 14px-rounded tile), and the touch and 192/512 icons. Set the left ring's colour through `--logo-a`; never recolour the amber. The logo sits left in the bar on every page and links home.

### Cards / Containers
- **Corner Style:** 16px.
- **Background:** `panel` white (dark #1b2127).
- **Shadow:** Float over a map, Soft on the page (see Elevation).
- **Border:** none; separation is shadow plus hairline `rule` dividers inside.
- **Padding:** 18 to 20px in a panel head and body, 10px/20px in the title card, 12 to 16px in tray cards, 32px/40px in a read card.

### The Bar
A single floating 64px card (grid: logo, centred search up to 640px or the switcher, end controls). The surface switcher is a row of 10px-radius links; the current page has the accent wash with blue text. Quiet toggles (Piece details, Notes) show an accent wash when their panel is open. Under 900px it becomes a sticky card with the search on a second row and the switcher on a third; toggle labels drop to icons.

### Buttons
- **Shape:** 12px radius, 40px minimum height (44px on coarse pointers), 14px/600 text, white fill, 1px `field-line` border, 8px icon gap.
- **Default:** white on `ink`, hover `fill`.
- **Quiet:** transparent, `ink-2`; hover `fill`; open or pressed uses accent wash and blue text.
- **Filled blue:** Add a piece (editor tools), Search (home and map search): `accent` fill, `accent-ink` text, hover darkens with `brightness(0.94)`. Only one per screen.
- **Danger:** on hover only, red fill with panel-coloured text.
- **Icon button:** 32px (38px stepper) square, 10px radius, 14px stroke icons at 1.6 to 1.7 width, round caps. Icons are inline SVG strokes in `currentColor`, never glyphs.
- **Disabled:** `ink-3`, 60% opacity.

### Search
The first thing in the bar and the largest control. 46px tall, 14px radius, land fill at rest; on focus it turns white with a blue border and a 3px blue wash halo. A results popover is a Float card with the selected row in accent wash. On the home page the field is 64px tall with a 2px grey-blue border, a 26px blue magnifier, and a filled blue Search button inside its right edge.

### Place rows and status pills
A two-line row (600 name, 13px `ink-2` detail) with a pill at the right; 10px radius, row hover and current in accent wash, keyboard focus a 2px inset blue ring. A pill is blue on accent wash (normal) or amber on amber wash (needs attention). Attention rows are listed first.

### Status chip
A pill in Float: green text on white when it works, amber text on amber wash with an inset amber edge when it does not. On pass the checker's tick (a 2px stroke check) draws itself in once. In the editors it sits under the plate, and a separate amber "N checks" pill opens the Checks tab.

### Tabs and segmented controls
A `fill` well with 3px padding holding flat segments; the selected segment is white with blue or ink text and Soft shadow. Used for the Notes tabs (Streets/Width, Checks, Changes, Transit/Measures) and the unit switch (blue text when pressed).

### Fields
White, 10px radius, 36px minimum height, 1px `field-line` border. Focus swaps the border to blue with a 3px accent-wash halo. Invalid is an amber border and amber text. Width fields are right-aligned tabular numbers; the stepper pairs two 38px icon buttons around a 20px/600 centred value. Selects match.

### Tray chips (editors)
Draggable chips for streets or pieces to add and junctions to start from: 12px radius, white, 1px `field-line` border, a 44px swatch preview on the left, name 14px/600 and a 12.5px sub-line; a 10px grip glyph on the right while they can be dragged; hover `fill`; pressed (plain choice) uses blue border and wash. The drag chip is the same card in Float with a blue border.

### The editor plate
The drawing on one white 16px Soft card that scrolls horizontally on a phone. A welcome note (accent wash band) and a time-of-day slider with a coloured range track (accent slider) may sit inside it above the drawing. Under a junction plan a two-column swatch key lists the strip kinds.

### The camera-inset floating panel (map)
See Layout. The bar, Places card and Notes card are `data-covers` floating cards; the zoom group, north arrow, scale bar and status chip are placed with `--lw` and `--rw` so they stay in the open area. Floating panels are the product's signature move: a map-app arrangement where the drawing is never covered by what the camera was told to avoid.

### Read card
A page to read (credits) is one white 16px card, 720px wide, on the land, with a 36px headline, an 18px `ink-2` lede, 17px section heads and 62ch text.

## Do's and Don'ts

### Do:
- **Do** keep the drawing the largest thing on every page and let cards float around it.
- **Do** use the token names for both themes and design every new surface in light and dark.
- **Do** mark any new floating panel `data-covers` and place neighbours from `--lw` and `--rw`.
- **Do** use blue for chosen and actionable, amber for what fails, and green only for a pass.
- **Do** keep labels on every drawn piece; tints and the faint pattern are a second code, not the first.
- **Do** keep controls at 40px or more (44px for touch) and text at 13px or more outside pills.
- **Do** draw icons as inline stroke SVGs in `currentColor`.

### Don't:
- **Don't** return to the drafting-sheet look: bond-paper ink, condensed type, heavy hatch, hard offset shadows.
- **Don't** add a second accent hue, a decorative green or a gradient brand colour.
- **Don't** add a web font or any text in a second family.
- **Don't** hard-code a layout for the editors' tool strip; it wraps, and the junction and street arrangements follow from that.
- **Don't** put a card over the part of the city or plan a person is working on.
- **Don't** colour a drawing in blue washes wall-to-wall; lane arrows and turns are blue wash, never solid blue blocks.
- **Don't** reintroduce the old `sheet`, `blue-pencil`, `warning` and `mark-1..5` tokens; they no longer exist in the build.

### Recorded, not canonized
The home hero search border is a hard-coded #7b8794 rather than a token, and the three theme blocks in `app.css` repeat the dark values by hand; both are drift the build carries, not rules for new surfaces. The surface briefs quote an older land value (#e9edf0); the build's `land` (#e2e7ec) governs.
