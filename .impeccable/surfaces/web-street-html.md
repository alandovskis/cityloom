---
version: 1
slug: "web-street-html"
primary_target: "web/street.html"
related_targets: []
---

# Surface brief: street editor

Scope: one surface, `web/street.html` (the cross-section editor; it was `index.html` before the home page took the root). Mode: Operate. Code-led. It inherits the established world recorded in `web-map-html` and PRODUCT.md (the category standard, benchmarked against Streetmix); this brief records only what is particular to the street.

Audience and job: a resident, on a laptop, a phone or a projector, who opens one street and rearranges its fixed width. Editing and the live "does it fit" answer are one loop. Drag, resize, add, delete and reorder pieces; the width stays fixed and visible; every action works from the keyboard. Behaviour, wording and keys do not change.

Constraints: WCAG 2.2 AA; both themes designed; under 1360px wide the notes start closed (the person can open them) because the drawing needs its width; below 900px the cards stack under the drawing.

## Direction contract

THESIS: The same calm floating-card world as the map, with the cross-section as the one large thing: a white plate in the middle, a title card and one blue "Add a piece" above it, Piece details on the left and Notes on the right. It refuses the old drafting-sheet frame, hatch-heavy ink, and a dashboard of tiles.

OWN-WORLD: The map's tokens (cool land, white cards at 16px, hairline rules, system type, one blue, amber for what does not fit, green only for "passes"). The section is drawn in flat tints with a faint texture beneath them, mid-grey outlines and linework, and system-type labels; pieces are named, so colour is never the only code. Controls are rounded 10-12px with a light border; the one filled blue control is Add a piece.

STORY: The resident sees the street as a clear plate, drags a piece, and the status chip turns amber with a cloud over the overflow; they narrow something and the chip returns to green with the checker's tick.

FIRST VIEWPORT: Bar (logo, Map/Street/Intersection, Piece details, Notes, avatar). Left card: the selected piece's details (or a prompt). Centre: a title card with the street's name and ends beside the toolbar (Add a piece, Undo, Redo, Start over), the welcome note, the section plate, the status chip. Right card: Width, Checks, Changes, Transit tabs and the about-this-street footer.

FORM: The category standard (user's choice), carried from the map; benchmark Streetmix.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
