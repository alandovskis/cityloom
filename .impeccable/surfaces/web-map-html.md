---
version: 1
slug: "web-map-html"
primary_target: "web/map.html"
related_targets: []
---

# Surface brief: city map

Scope: one surface, `web/map.html`. Mode: Operate. Code-led (no image generation); the chosen decision mockup, `.impeccable/mocks/decision/canon.png`, rides as the critique reference. Replaces the drafting-sheet look for this page; the street and junction pages follow later.

Audience and job: a resident with no planning training, at home on a laptop, on a phone, projected at a community meeting, or on a workshop touchscreen. They see the whole city, find the places that need attention, and open a junction or street. Success: they can say what is wrong and where within seconds.

Task and content: pan, zoom, choose a place from the map or the list; see whether every place works; read the checks and the changes made; start over. All behaviour, keys, wording and the sample data stay. Avoid: engineering/GIS, game or toy, generic SaaS dashboard.

Constraints: WCAG 2.2 AA; works from the keyboard; light and dark themes both designed; type readable projected and on a phone; Start over stands apart from the tools.

## Direction contract

THESIS: The map is the page. A familiar, calm map-app arrangement: the city fills the viewport and everything else floats over it as a few quiet white cards. It refuses any costume and the three-column admin dashboard.

OWN-WORLD: Cool light land (#e9edf0), white panels with 16px radius and a soft two-layer shadow, hairline #e3e7eb dividers, system UI type at 15/13px with 600 for names, one blue (#2563eb) for selection, links and the primary action, amber (#b45309 on #fef3c7, #f59e0b on the map) for what needs attention, green only for "passes". Roads are white with a pale casing and flat lane tints, junctions are white discs ringed in blue with blue numerals. Recognisable with content removed by its floating cards and rounded blue-ringed discs.

STORY: The resident opens the map and sees one calm city with a chip that says whether it works. If something is wrong the chip turns amber, the street glows amber on the map and its card rises to the top of Places. They press it and land in the editor.

HOME PAGE (added later at the user's request): web/index.html is the first thing seen. A floating card at 16px radius holds the headline, the big search with its filled blue Search button, the way into the map and into a new street, and how the city stands; the real sample city runs off the right and bottom edges behind it, inert. The card uses the system radii (16px card, 14px search, 12px buttons).

FIRST VIEWPORT: Full-bleed map. Top: one floating bar (logo, wordmark, Map / Street / Intersection, avatar). Left: a Places card with the city name, a one-line summary, and a list with attention rows first. Right: a Notes card (Checks, Changes). Bottom-right: grouped zoom and whole-city control. Bottom centre: the status chip. Start over sits alone at the foot of Places.

FORM: The category standard, the user's own choice from the card round: the roll (seed key f82482e8) assigned The Wall Chart and offered The Charrette Plot as the pick, and the user's own choice of the standing exit replaced both. Benchmark Streetmix. Telemetry kind canon, not sent.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
