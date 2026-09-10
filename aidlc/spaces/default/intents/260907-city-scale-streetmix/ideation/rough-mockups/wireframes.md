# Rough Wireframes — Streetmix at City Scale

Low-fidelity concept wireframes. Boxes and labels only: no visual design, no real
copy, no colour. Their job is to settle structure and interaction cheaply, before
anything is built.

Upstream inputs: `intent-statement.md` sets the problem as designing corridors and
networks on a real map, for an audience of advocates and the public, with the
differentiator being output a city will accept. `scope-document.md` stages the
work across three deliveries. `intent-backlog.md` orders the proto-Units.

Diagrams use the ASCII standard: `+ - | ^ v < >` and alphanumerics only.

## The Interaction Principle Everything Rests On

Two answers converge on one solution rather than compounding into two costs.
Editing must work on a phone (Q4) and every editing action must have a
keyboard-operable path (Q5). Both rule out drag as the *only* way to change a
lane.

**Select-then-act is the primary interaction.** Pick a lane, then choose what it
becomes and how wide. This works with touch on a small screen, with a keyboard,
and with a screen reader, from one implementation.

**Drag is an accelerator layered on top**, available to mouse and touch users who
want it, never the sole route to any outcome.

This principle shapes every screen below. Where a wireframe shows a drag
affordance, an equivalent select-then-act path exists alongside it.

## Layout Options for Map and Cross-Section

The central layout problem, drawn three ways for comparison (Q2). Streetmix fills
the window with one cross-section; the problem statement puts that cross-section
on a real map, and a map wants space too.

### Option A — Split screen

```
  +-----------------------------------------------------------+
  | Logo    Search a place              [Share]  [Sign in]    |
  +---------------------------+-------------------------------+
  |                           |                               |
  |          MAP              |     CROSS-SECTION             |
  |                           |                               |
  |   street network,         |   +--+---+------+---+--+      |
  |   selected street         |   |sw|bike| lanes|par|sw|     |
  |   highlighted             |   +--+---+------+---+--+      |
  |                           |                               |
  |                           |   Selected: bike lane         |
  |                           |   Type [v]  Width [- 1.8m +]  |
  |                           |                               |
  |                           |   [+ Add lane] [Remove]       |
  +---------------------------+-------------------------------+
  | Corridor: 3 streets selected      [Edit corridor >]       |
  +-----------------------------------------------------------+
```

<!-- Text fallback: a header with logo, place search, share and sign-in. Below,
the window splits vertically: map on the left showing the street network with the
selected street highlighted, cross-section editor on the right showing lanes as
adjacent blocks with a properties panel beneath. A corridor bar spans the
bottom. -->

Both contexts stay visible, so the effect of a change is seen against its
location. Neither gets enough room; on a phone the split has to collapse
entirely, which means designing a second layout anyway.

### Option B — Map primary, cross-section in a drawer

```
  +-----------------------------------------------------------+
  | Logo    Search a place              [Share]  [Sign in]    |
  +-----------------------------------------------------------+
  |                                                           |
  |                    MAP  (full width)                      |
  |                                                           |
  |          selected street highlighted                      |
  |                                                           |
  +-----------------------------------------------------------+
  | ^  Elm Street                                    [x]      |
  |                                                           |
  |   +--+---+------+---+--+                                  |
  |   |sw|bike| lanes|par|sw|                                 |
  |   +--+---+------+---+--+                                  |
  |                                                           |
  |   Selected: bike lane   Type [v]  Width [- 1.8m +]        |
  |   [+ Add lane]  [Remove]        [Extend along corridor]   |
  +-----------------------------------------------------------+
```

<!-- Text fallback: the map fills the whole view. A drawer rises from the bottom
when a street is selected, showing that street's name, its lanes as adjacent
blocks, the properties of the selected lane, and actions including extending
along the corridor. The drawer can be collapsed back down. -->

The map is the primary object, which matches a network-scale product. The drawer
pattern is native to phones and scales up to desktop unchanged, so one layout
serves every form factor. The cost is that the map is partly covered while
editing.

### Option C — Cross-section primary, map as inset

```
  +-----------------------------------------------------------+
  | Logo   Elm Street                   [Share]  [Sign in]    |
  +-----------------------------------------------------------+
  |  +--------+                                               |
  |  |  map   |          CROSS-SECTION  (full width)          |
  |  | inset  |                                               |
  |  +--------+     +--+---+--------+---+--+                  |
  |                 |sw|bike|  lanes |par|sw|                 |
  |                 +--+---+--------+---+--+                  |
  |                                                           |
  |                 Selected: bike lane                       |
  |                 Type [v]     Width [- 1.8m +]             |
  |                                                           |
  |  < Prev street        Elm St 2/5        Next street >     |
  +-----------------------------------------------------------+
```

<!-- Text fallback: the cross-section editor fills the view with a small map
inset in the top left for context. Lane properties sit below the section. A
corridor stepper along the bottom moves between streets in the selected
corridor. -->

Closest to Streetmix, so the editing experience is the strongest of the three.
But it demotes the map to context, which works against a product whose premise is
network scale. The corridor becomes a stepper rather than something seen.

### Recommendation

**Option B.** Reasons, in order of weight:

1. **One layout for every form factor.** Full-responsive editing is required
   (Q4). A bottom drawer over a full-bleed map is the native phone pattern and
   needs no redesign at desktop width. A and C both require a second layout for
   small screens, which for a solo builder is a second design and a second
   implementation.
2. **The map is the product's premise.** `intent-statement.md` frames the problem
   as corridors and networks on a real map. Option C demotes the thing that makes
   this different from Streetmix.
3. **The drawer is a natural focus boundary.** Opening it moves keyboard focus in,
   Escape closes it, and its contents are a self-contained region for a screen
   reader — which serves the AA commitment (Q5) with a well-understood pattern
   rather than a bespoke one.

Against it: the map is partly obscured while editing. Mitigated by making the
drawer resizable and collapsible, and by keeping the edited street visible above
it.

This is a recommendation, not a decision. Overturn it at the gate if the editing
experience matters more than the map.

## Corridor Interaction Options

The capability that distinguishes this from Streetmix, drawn two ways (Q3).
There is no incumbent to borrow from.

### Option 1 — Edit and extend

```
  Step 1: edit one street          Step 2: extend it
  +---------------------+          +--------------------- +
  |      MAP            |          |      MAP             |
  |    [Elm St]         |          |    [Elm]=[Oak]=[Pine]|
  |     selected        |          |    highlighted path  |
  +---------------------+          +--------------------- +
  | Elm Street          |          | Extend along:        |
  | +--+---+----+--+    |          |  [x] Oak St          |
  | |sw|bik|lane|sw|    |          |  [x] Pine St         |
  | +--+---+----+--+    |          |  [ ] Maple St        |
  |                     |          |                      |
  | [Extend along >]    |          | [Apply to 2 streets] |
  +---------------------+          +--------------------- +
```

<!-- Text fallback: two steps. First the user edits a single street in the
drawer. Then pressing Extend shows the connected streets as a checklist, with the
map highlighting the path, and applies the design to the chosen ones. -->

The user is always editing one concrete street, and the corridor is an extension
of work already done. Nothing new is learned before the first useful result. It
handles the common case — a street changes character along its length — poorly,
because it applies one design to several streets rather than letting them differ.

### Option 2 — Select a route, then walk it

```
  Step 1: pick a route             Step 2: walk it
  +---------------------+          +---------------------+
  |      MAP            |          |      MAP            |
  |  o---o---o---o      |          |  o---O---o---o      |
  |  route drawn        |          |  current step       |
  +---------------------+          +---------------------+
  | Route: 4 streets    |          | Oak St  (2 of 4)    |
  | Elm > Oak > Pine >  |          | +--+---+----+--+    |
  | Maple               |          | |sw|bik|lane|sw|    |
  |                     |          | +--+---+----+--+    |
  | [Edit this route >] |          | [< Prev]   [Next >] |
  +---------------------+          | [Copy from previous]|
  +---------------------+          +---------------------+
```

<!-- Text fallback: two steps. First the user draws or picks a route through the
network, shown as connected nodes on the map with the street list beneath. Then
they step through it street by street, editing each, with an option to copy the
previous street's design forward. -->

The corridor is the object of work from the start, which matches the problem
statement more directly. Each street can differ while "copy from previous" keeps
the common case cheap. The cost is a route-drawing interaction before any editing
happens — more to learn, and more to build.

### Recommendation

**Option 1 for stage 1, with Option 2's stepper as the growth path.** Reasons:

1. Option 1 delivers value at the first street, before the corridor concept is
   introduced at all. For a first release meeting real users, a tool that is
   useful after one interaction beats one that requires a route first.
2. Option 1's "extend along" checklist is a select-then-act interaction already,
   so it satisfies the keyboard and phone requirements without extra design.
3. Option 2's per-street stepper can be added later over the same data model, at
   which point "extend" becomes one way to populate a route rather than a
   competing idea.

Against it: if corridors that vary along their length turn out to be the norm
rather than the exception, Option 1 is the wrong primitive and the growth path
becomes a rewrite. This is worth watching with the first real user.

This is a recommendation, not a decision. Overturn it at the gate if the corridor
should be the object of work from the start.

## Screens

Layout follows Option B throughout. Each screen carries a one-line accessibility
note giving heading level, landmark regions, and keyboard entry point.

### S1 — Hero landing page (stage 1)

```
  +-----------------------------------------------------------+
  |  Logo                                    [Sign in]        |
  +-----------------------------------------------------------+
  |                                                           |
  |        Redesign your street. Show your city.              |
  |                                                           |
  |        one-paragraph explanation of what this is          |
  |                                                           |
  |        [ Find your street ]                               |
  |                                                           |
  |    +------------------------------------------------+     |
  |    |   worked example: a real street, before/after  |     |
  |    +------------------------------------------------+     |
  |                                                           |
  +-----------------------------------------------------------+
  |  How it works  |  Examples  |  About                      |
  +-----------------------------------------------------------+
```

<!-- Text fallback: a landing page with logo and sign-in in the header, a hero
headline and one-paragraph explanation, a primary call to action reading "Find
your street", a worked before-and-after example below it, and a footer with
secondary links (no licensing claim is made here; this product's licence is
not decided by any upstream artifact). -->

Accessibility: h1 is the hero headline; landmarks are header, main, footer;
keyboard entry point is the "Find your street" button, reachable as the first
interactive element in main after a skip link.

### S2 — Map, nothing selected (stage 1)

```
  +-----------------------------------------------------------+
  | Logo   [ Search a place            ]     [Sign in]        |
  +-----------------------------------------------------------+
  |                                                           |
  |                        MAP                                |
  |                                                           |
  |            street network, nothing selected               |
  |                                                           |
  |     +--------------------------------------------+        |
  |     |  Select a street to start designing        |        |
  |     |  or [use my location]                      |        |
  |     +--------------------------------------------+        |
  +-----------------------------------------------------------+
```

<!-- Text fallback: header with a place search. The map fills the view with no
street selected. A centred prompt over it explains to select a street, with a use
my location action. -->

Empty state: the prompt is the empty state; the map is never shown bare without
guidance.

Accessibility: h1 is a visually hidden page title; landmarks are header, main;
keyboard entry point is the search field. Streets are reachable as a list
alternative to map clicking — the AA path for selecting a street without a
pointer.

### S3 — Street selected, drawer open (stage 1)

```
  +-----------------------------------------------------------+
  | Logo   [ Search a place            ]     [Sign in]        |
  +-----------------------------------------------------------+
  |                        MAP                                |
  |            Elm Street highlighted                         |
  +-----------------------------------------------------------+
  | ^  Elm Street                                     [x]     |
  |                                                           |
  |   +----+------+--------+------+----+                      |
  |   | sw | bike |  lanes | park | sw |   <- select a lane   |
  |   +----+------+--------+------+----+                      |
  |     1     2       3       4     5                         |
  |                                                           |
  |   Lane 2 of 5: bike lane                                  |
  |   Type  [ bike lane        v ]                            |
  |   Width [ - ]  1.8 m  [ + ]                               |
  |                                                           |
  |   [+ Add lane]  [Remove lane]  [Extend along corridor >]  |
  +-----------------------------------------------------------+
```

<!-- Text fallback: the map above with the selected street highlighted, and a
drawer below showing the street name, its lanes as numbered adjacent blocks, the
selected lane's type and width as form controls, and actions to add, remove, or
extend along the corridor. -->

Select-then-act in full: lanes are numbered and individually focusable; type is a
select; width is a stepper with typed entry. Drag to reorder or resize is
available on top and duplicates nothing that cannot be done otherwise.

Accessibility: h2 is the street name inside the drawer; landmarks are header,
main, and the drawer as a labelled complementary region; keyboard entry point is
the first lane, with arrow keys moving between lanes and Escape closing the
drawer.

### S4 — Extend along corridor (stage 1)

```
  +----------------------------------------------------------+
  |                        MAP                               |
  |        Elm = Oak = Pine highlighted as a path            |
  +----------------------------------------------------------+
  | ^  Extend Elm Street's design                     [x]    |
  |                                                          |
  |   Connected streets:                                     |
  |    [x] Oak Street       120 m                            |
  |    [x] Pine Street      200 m                            |
  |    [ ] Maple Street      90 m                            |
  |                                                          |
  |   Note: Oak Street is narrower than Elm. Lanes will be   |
  |   scaled to fit. Review after applying.                  |
  |                                                          |
  |   [Cancel]                      [Apply to 2 streets]     |
  +----------------------------------------------------------+
```

<!-- Text fallback: the map highlights the chain of streets. The drawer lists
connected streets as checkboxes with their lengths, warns where a target street
is narrower than the source, and offers cancel or apply. -->

The width warning is the error-prevention case: a design that does not fit is
caught before it is applied rather than reported afterwards.

Accessibility: h2 is the drawer heading; the street list is a labelled checkbox
group; keyboard entry point is the first checkbox.

### S5 — Sign in (stage 2)

```
  +----------------------------------------------------------+
  |                    Sign in to save                       |
  |                                                          |
  |   Your design is safe. Signing in lets you keep it,      |
  |   name it, and choose who can see it.                    |
  |                                                          |
  |   Email  [                              ]                |
  |                     [ Continue ]                         |
  |                                                          |
  |   [Not now - keep designing]                             |
  +----------------------------------------------------------+
```

<!-- Text fallback: a sign-in panel explaining that the current design is safe
and that signing in allows keeping, naming and controlling visibility. An email
field, a continue button, and a dismissal that returns to designing. -->

The dismissal matters: stage 1 works without accounts, so sign-in must never be
a wall in front of the tool.

Accessibility: h1 is the panel heading; it is a modal with focus trapped and
Escape returning to the map; keyboard entry point is the email field.

### S6 — My designs (stage 2)

```
  +----------------------------------------------------------+
  | Logo   My designs                        [Account v]     |
  +----------------------------------------------------------+
  |   +------------------+  +------------------+             |
  |   | Elm St corridor  |  | Oak St           |             |
  |   | 3 streets        |  | 1 street         |             |
  |   | Private          |  | Shared with 2    |             |
  |   | [Open] [Share]   |  | [Open] [Share]   |             |
  |   +------------------+  +------------------+             |
  |                                                          |
  |   Empty state: "No designs yet. [Find a street]"         |
  +----------------------------------------------------------+
```

<!-- Text fallback: a card grid of saved designs, each card naming the design,
its street count, its sharing status, and open and share actions. An empty state
directs a new user to find a street. -->

Sharing status is shown on every card, because access-controlled-by-default
(scope Q5) is only meaningful if a person can see at a glance what is exposed.

Accessibility: h1 is "My designs"; landmarks are header and main; the grid is a
list; keyboard entry point is the first card.

### S7 — Share (stage 2)

```
  +----------------------------------------------------------+
  |  Share "Elm St corridor"                          [x]    |
  |                                                          |
  |   Who can see this?                                      |
  |    (o) Only me                                           |
  |    ( ) People I invite                                   |
  |    ( ) Anyone with the link                              |
  |                                                          |
  |   Invite by email                                        |
  |   [                              ]  [Invite]             |
  |                                                          |
  |   Link  [ https://.../d/a1b2c3      ] [Copy]             |
  |         Link sharing is off. Turn it on above.           |
  +----------------------------------------------------------+
```

<!-- Text fallback: a share dialog with three mutually exclusive visibility
options defaulting to Only me, an email invitation field, and a link field that
is shown but explicitly inert until link sharing is enabled. -->

"Only me" is preselected, and the link is visibly inert until visibility is
widened. Access-controlled by default is enforced by the interface, not just by
policy.

Accessibility: h2 is the dialog title; radio group with arrow-key selection;
modal with focus trap and Escape to close; keyboard entry point is the first
radio.

### S8 — Export for a meeting (stage 3)

```
  +----------------------------------------------------------+
  |  Export "Elm St corridor"                         [x]    |
  |                                                          |
  |   What are you making?                                   |
  |    ( ) A page to print or hand out                       |
  |    (o) Slides for a presentation                         |
  |    ( ) An image for a document or post                   |
  |                                                          |
  |   Include                                                |
  |    [x] Before and after                                  |
  |    [x] Street location map                               |
  |    [ ] Lane dimensions table                             |
  |                                                          |
  |   +------------------------------------------+           |
  |   |            preview of the output         |           |
  |   +------------------------------------------+           |
  |                                                          |
  |   [Cancel]                            [Export]           |
  +----------------------------------------------------------+
```

<!-- Text fallback: an export dialog asking what the user is making, offering
print page, slides, or image; a set of inclusion checkboxes for before-and-after,
location map and dimensions table; a live preview of the result; cancel and
export actions. -->

The question is "what are you making", not "which file format". The differentiator
is output a city will accept, and the person choosing is an advocate preparing for
a meeting rather than someone picking a file type.

Accessibility: h2 is the dialog title; radio group and checkbox group both
labelled; preview carries a text description of what will be produced; keyboard
entry point is the first radio.

## Screen States

Every screen has more than a happy path. Applied across the set:

| State | Where it shows | Treatment |
|-------|---------------|-----------|
| Empty | S2 map with nothing selected; S6 with no designs | A prompt with a next action, never a bare surface |
| Loading | S3 while a street imports | The drawer opens immediately with the street name and a skeleton cross-section, so the selection is acknowledged before the data arrives |
| Error | S3 when import fails | The drawer states plainly that this street's data could not be read, offers retry, and offers starting from a blank cross-section instead |
| Partial | S3 when OSM tagging is thin | Derived lanes are marked as estimated, with a note that the source data did not specify them. This is the interface consequence of `raid-log.md` R-2 |
| Success | S4 after applying; S8 after exporting | A brief confirmation naming what happened and how to undo it |

The partial state is the one that matters most for credibility. `raid-log.md` R-2
records that where OSM tagging is thin, cross-sections rest on inferred defaults.
A tool whose differentiator is institutional acceptance must not present an
inferred lane width as a measured one.

## Information Architecture

```
  Hero landing (S1)
                                                       |
    +-- Map (S2                                        )
    |                                                  |
    |     +-- Street selected (S3                      )
    |                                                  |
    |           +-- Extend along corridor (S4          )
    |           +-- Export (S8)                [stage 3]
                                                       |
    +-- Sign in (S5)                           [stage 2]
                                                       |
          +-- My designs (S6)                  [stage 2]
                                                       |
                +-- Open -> Map (S2                    )
                +-- Share (S7)                 [stage 2]
```

<!-- Text fallback: the hero landing leads to the map or to sign-in. The map
leads to a selected street, which leads to extending along a corridor or
exporting. Sign-in leads to my designs, which leads back into the map or to the
share dialog. -->

The map is the hub. Everything either leads into it or hangs off a street
selected within it.

## Assumptions & Open Questions

- The layout and corridor recommendations are the designer's judgement from the
  confirmed constraints, not a user-tested result. No one has used any of these.
  [assumption]
- The hero landing page (S1) is not in `intent-backlog.md`, which begins at the
  map view. It needs a proto-Unit if it is in the first release, or stage 1 needs
  a simpler entry point until it exists. [assumption]
- "Distinctive from the start" (Q6) implies visual identity work that is not in
  the backlog and competes with stage 1 capability work for a solo builder's
  time. [assumption]
- Wireframes for stages 2 and 3 (S5-S8) sketch capabilities whose details are
  unspecified — what satisfies the public-release gate, and what output formats a
  city actually accepts. They will need revisiting once those are settled.
  [assumption]
- Phone layouts are stated as following from Option B's drawer pattern rather
  than drawn separately at this fidelity. [assumption]
- Whether a street's lanes are better numbered or named for keyboard and screen
  reader navigation is unresolved; numbering is used here as the simpler start.
  [assumption]
- No wireframe covers account deletion or data export, which the public-release
  gate requires. They belong with stage 2's detailed design rather than at this
  fidelity. [assumption]

## Review

**Verdict:** READY
**Reviewer:** aidlc-product-lead-agent
**Date:** 2026-09-08T01:10:36Z
**Iteration:** 2
**Request Challenge:** review:abe0ca9a7366e110d17b8deedf94e54f

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | wireframes.md > Corridor Interaction Options > Recommendation | The Option B layout recommendation is explicitly labelled "a recommendation, not a decision. Overturn it at the gate if..." The corridor Option 1 recommendation gives reasons and a stated risk (if corridors that vary along their length turn out to be the norm, the growth path becomes a rewrite) but never states the same overturnable framing. A reader could take Option 1 as more settled than Option B even though both are equally unvalidated designer judgement. | Add the same explicit "this is a recommendation, not a decision" sentence to the corridor Option 1 recommendation, matching the layout recommendation's framing. | New |
| R-02 | Minor | wireframes.md > S1 Hero landing page > footer nav | The hero footer includes an "Open source" nav item. No upstream artifact states or implies a decision that this product itself will be open source -- constraint-register.md LC-3 addresses Streetmix's own AGPL licence as a non-dependency, not this product's licensing model. Every other uncertain claim in this document is tagged [assumption]; this one implies a licensing decision without that tag or a citation. | Either remove the "Open source" footer item as unfounded placeholder copy, or tag it [assumption] and note it has no upstream source. | New |
| R-03 | Minor | wireframes.md > boxed ASCII screens | Several boxed diagrams are off by one character between the border row and adjacent content rows. The right-hand border character does not sit in the same column as the plus sign above it, so the box is not internally consistent, even though the ASCII character set is compliant and each text fallback is accurate. | Re-pad the affected rows so every line inside a given box shares the same total width. | New |

### Summary

Re-verified after a receipt-repair edit that escaped a stray pipe character in R-03's finding text inside the findings table; no design content in wireframes.md, user-flow.md, or rough-mockups-questions.md changed. All three prior Minor findings remain open and unaddressed, matching the human's prior Approve-with-known-findings disposition at the gate; none rise to Critical or Major, so the verdict is unchanged from the prior pass.
