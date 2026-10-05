---
version: 1
slug: "web-intersection-html"
primary_target: "web/intersection.html"
related_targets: []
---

# Surface brief: intersection editor

Scope: one surface, `web/intersection.html`. Mode: Operate. Code-led. It inherits the established world recorded in `web-map-html` and PRODUCT.md and shares its structure with the street editor; this brief records only what is particular to the junction.

Audience and job: a resident who opens one junction (from the map or as a sandbox on the samples) and changes its streets, corners, crossings, lanes, turns and control, and sees at once what still works. Behaviour, wording and keys do not change.

Constraints: WCAG 2.2 AA; both themes; the plan, its status chip and the tray of streets to add and junctions to start from are all on screen at 1440x900 without scrolling; under 1360px the notes start closed; below 900px the cards stack under the plan.

## Direction contract

THESIS: The same calm floating-card world: the plan of the junction on a white plate in the middle, with the tray of streets to add and samples to start from directly under it, Details on the left and Notes on the right. It refuses the drafting-sheet frame and an engineering-tool density.

OWN-WORLD: The map's tokens and the street editor's components. The plan is drawn with white roads, flat pale tints for sidewalks, planting and parking, mid-grey linework and markings, a blue selection, and amber for what fails; lane arrows and turn buttons are chosen in blue wash, never a wall of solid blue.

STORY: The resident picks a street, changes a lane's turns or the junction's control, watches the status chip and the Checks tab, and undoes if it goes wrong.

FIRST VIEWPORT: Bar as on every editor. Left: the selected street, corner or crossing (or the junction's control). Centre: title card and Undo/Redo/Start over, the plan plate with its key, the status chip, and the two tray cards (Add a street, Start from). Right: Streets, Checks, Changes, Measures and the about-this-junction footer.

FORM: The category standard (user's choice), carried from the map; benchmark Streetmix.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
