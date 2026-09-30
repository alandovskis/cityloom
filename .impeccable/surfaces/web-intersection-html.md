---
version: 1
slug: "web-intersection-html"
primary_target: "web/intersection.html"
related_targets: []
---

# Surface brief: intersection editor

Scope: one surface, `web/intersection.html`. Mode: Operate. Fidelity: working prototype, code-led (no image generation, no comp). Inherits the established Typical Section Sheet world in DESIGN.md; no new identity and no structure tournament (the user chose the prototype path over three structure options).

Audience and job: a resident, no planning training, at a laptop or a projector at a community meeting. They open one junction of three to five streets and change how it works: corners, crossings, the lanes each street brings, which turns are allowed, how it is controlled. Editing and live consequences are one loop.

Task and content: select an arm, corner or crossing and change it; rotate arms, add and remove them, switch to a roundabout; edit a turn schedule; read checks and per-arm numbers. Arm sections come from the sample streets and are read-only here. Junction samples, thresholds and rates are synthetic and labelled so. Out of scope: persistence, save and share, editing an arm's section, real signal timing, slip lanes, a map or network, real traffic data.

Constraints: Rust compiled to WebAssembly owns every rule and every piece of geometry; JavaScript draws and relays input. Every action works from the keyboard. WCAG 2.2 AA assumed. Shares its shell with the street page.

## Direction contract

THESIS: The intersection is the plan sheet that sits beside the typical section sheet. Where the section shows one street's width across, the plan shows the same pieces running out from a junction. It refuses the game-map look of a stylised road with glowing lanes and an app sidebar of icon cards.

OWN-WORLD: The existing sheet, turned to plan: cool bond ground, black ink at graded line weights, each piece of a street in its own tint and hatch and named, carriageway left untinted asphalt hatch, curbs as 2px ink lines with true fillets, crossings as ink-striped bars, a north mark and a graphic scale bar, dimension strings with ticks. One blue pencil for the selected arm, corner or crossing; one redline for a failed check. A drafting schedule (rows entering, columns leaving) for the turn movements. Recognisable with content removed by the plan's curb fillets and the schedule grid.

STORY: The resident sees a real-looking junction at scale, understands each street's section from its strips, changes a corner radius and watches the fillet and the turning speed move, adds a crossing and sees the distance across, bans a turn and sees a lane that now points nowhere turn redline, then fixes the lane. Undo and redo throughout.

FIRST VIEWPORT: Full-viewport sheet as the street page: top bar with the two surface tabs (Street, Intersection), header with junction name and arm count at left and the same tools at right; left inspector for the selected target; centre the plan at true scale with north mark, per-arm crossing dimensions, and a scale bar, over a status line and the Add a street palette; right notes column with Space (per-arm numbers and the turn schedule), Checks and Changes. No call to action; the primary action is dragging an arm, a corner or a crossing in the drawing.

FORM: Plan sheet from the established Typical Section Sheet, extension of an existing surface (no seed key: the concept roll was skipped by the user's choice).

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
