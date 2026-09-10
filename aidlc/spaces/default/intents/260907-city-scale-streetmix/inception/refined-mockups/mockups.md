# Refined Mockups — Streetmix at City Scale

Upstream inputs: `wireframes.md` and `user-flow.md` (rough-mockups),
`stories.md` and `personas.md` (user-stories), `requirements.md`
(requirements-analysis), `team-practices.md` (practices-discovery),
`refined-mockups-questions.md` (this stage).

Every screen below carries an accessibility annotation naming heading level,
landmark regions and keyboard entry point, and states which stories and
acceptance criteria it satisfies. Screen identifiers S1-S8 are carried forward
from `wireframes.md` unchanged; S9 and S10 are new.

Component-level specifications, keyboard paths and the live-region rules live in
`interaction-spec.md`; tokens and the framework selection criteria in
`design-system-mapping.md`; the verification split between machine and person in
`accessibility-checklist.md`. This file is the screens and what they must do.

Every ASCII diagram here was generated from a fixed-width helper rather than
drawn by hand, and every block was verified to a single line width before
writing — the rough-mockups review found off-by-one padding across several
frames, and measuring is the only way to catch it.

## What this stage changed, and why

Refined Mockups is not a redraw. Four things forced changes to approved screens,
and seven stories added at user-stories had no screen at all.

**Four corrections to approved screens.** Three were recorded in `stories.md`;
the fourth follows from Q5.

| Screen | Was | Now | Why |
|---|---|---|---|
| S4 | "Oak Street is narrower than Elm. Lanes will be scaled to fit." | The design is applied unchanged and the street stays marked as not fitting | A scaled width is neither `mapped`, `inferred` nor `user-set`. Scaling produces a value with no valid provenance state under FR3.1, which is the model the product's credibility rests on. AC6.2.2 |
| S7 | Three mutually exclusive visibility radios | Two independent controls: people you invite, and a link toggle | FR8.2 requires separate explicit choices. Under the radio model, inviting three people and then enabling a link does something to those invitations that no artifact specifies, and narrowing back silently kills a link that may already be in a meeting agenda. AC10.2.1-AC10.2.3 |
| S8 | Preview drawn inside the export dialog | Two variants - with and without the preview | US12.3 (preview) is Should Have. If it does not ship, the approved screen has a hole in the middle |
| Screen States, Success row | "A brief confirmation naming what happened and how to undo it" | The confirmation names what happened and points at the persistent undo control | Q5 moved the undo action out of the toast. The toast no longer carries it |

**Seven stories with no screen**, and where each one now lives:

| Story | Home |
|---|---|
| US5.5 — undo | A persistent control in the drawer header, drawn in S3 |
| US6.3 — choose a corridor | Folded into S4, per Q6 |
| US6.4 — apply across the corridor | Folded into S4, including its outcome state |
| US7.5 — a correction that no longer resolves | S9, new |
| US8.2 — work gone from this device | S10, new |
| US8.3 — keep a design off this device | A control in the map header beside the design name, drawn in S3, with its three states drawn below it. Also an action on each card in S6 |
| US13.1 — status announcements | Not a screen. The live-region rule in `interaction-spec.md`, bound to all 23 committed status messages |

`refined-mockups-questions.md`'s own preamble says "Six" and lists seven. That
file is the confirmed record of what was asked and answered, so it is left as
written rather than edited after its confirmation; the table above supersedes
its count.

**US6.2 (the fit warning) is not in this list.** It had partial coverage in the
pre-correction S4, which drew a single width note. What changed is not the
absence of a screen but the number of states that note must carry — two became
four. That is a correction to S4, not a missing screen.

## The phone is a first-class layout, not an afterthought

`user-flow.md` records that the audience arrives "frequently on a phone", and
Option B was chosen over the alternatives specifically for "one layout for every
form factor". No wireframe showed a phone, and NFR4.4 sets the floor at 360 CSS
pixels.

Q1 settles it: the drawer becomes a **bottom sheet with two heights**. The map
is never replaced by a second view, so Option B's justification survives contact
with the form factor it was chosen for.

- **Peek** - street name, the drawer header including undo, the cross-section,
  and a prompt. Enough to confirm the right street was selected and to undo a
  mistake without expanding anything.
- **Full** - covers the map; the lane detail, controls and actions.

Both heights are reachable by keyboard and by drag. The header is in the peek
deliberately: without it, undo disappears exactly when a corridor apply has just
changed four streets.

## The map carries four overlays and therefore needs a legend

Your answers give the map four simultaneous jobs: the street network; the
current design's streets highlighted (Q8); markers for streets with unresolved
corrections (Q4); and per-street fit status after a corridor apply (Q6).

Four overlays without a legend is a map nobody can read. The legend is itself an
accessibility surface - it cannot be colour-only, and it needs a text
equivalent. `interaction-spec.md` specifies it as a collapsible region with a
text list, not a floating key.

---

## S1 - Hero landing page (stage 1)

Carried forward from `wireframes.md` unchanged in structure. Two additions.

- The worked example must show the same street before and after with at least
  one visible dimension (AC1.1.2), and at least one value marked as estimated,
  so the product's central honesty is visible before anyone signs up to
  anything.
- At 360 CSS pixels the hero stacks in one column with no horizontal body scroll
  (AC1.1.3).

**Satisfies**: US1.1 (AC1.1.1-AC1.1.4), NFR4.4.

**Accessibility**: h1 is the hero headline; landmarks header, main, footer;
keyboard entry point is "Find your street", the first interactive element in
main after a skip link.

---

## S2 - Map, nothing selected (stage 1)

```
+----------------------------------------------------------+
| Logo  [ Search a place       ]       [Sign in]           |
+----------------------------------------------------------+
|                                                          |
|                      MAP                                 |
|        street network, nothing selected                  |
|                                                          |
|   +-----------------------------------------+            |
|   | Select a street to start designing      |            |
|   | or [use my location]                    |            |
|   |                                         |            |
|   | Continue a recent design:               |            |
|   |  - Elm St corridor   3 streets  2d ago  |            |
|   |  - Oak St            1 street   9d ago  |            |
|   +-----------------------------------------+            |
|                                                          |
+----------------------------------------------------------+
```

<!-- Text fallback: a header with a place search and sign-in. The map fills the
view with no street selected. A centred prompt over it explains to select a
street, offers use-my-location, and lists recent designs held on this device
with their street counts and ages. -->

The prompt is the empty state; the map is never bare. "Continue a recent design"
is how AC8.1.4 is reached on arrival - before any design is current, the map
header's design count has nothing to show, so the entry point has to be here.

**Satisfies**: US1.2 (AC1.2.1, AC1.2.2), US2.1 (AC2.1.1-AC2.1.4), US8.1
(AC8.1.4).

**Accessibility**: h1 is a visually hidden page title; landmarks header, main;
keyboard entry point is the search field. Streets are reachable as a list
alternative to the map (US2.3) - the conformant path for selecting a street
without a pointer.

### S2 at 360 pixels

```
+----------------------------------+
| [ Search a place  ]   [=]        |
+----------------------------------+
|                                  |
|            MAP                   |
|                                  |
|  +------------------------+      |
|  | Select a street to     |      |
|  | start designing        |      |
|  | [use my location]      |      |
|  |                        |      |
|  | Continue a recent      |      |
|  | design:                |      |
|  |  - Elm St corridor     |      |
|  |    3 streets, 2d ago   |      |
|  |  - Oak St              |      |
|  |    1 street, 9d ago    |      |
|  +------------------------+      |
|                                  |
+----------------------------------+
```

Search collapses to a single field with a menu control. The prompt and the
recent-design list stack in one column. Nothing is removed; the map area
shrinks.

---

## S3 - Street selected, drawer open (stage 1)

```
+----------------------------------------------------------+
| Logo  [ Search a place       ]       [Sign in]           |
+----------------------------------------------------------+
|                      MAP                                 |
|   Elm Street selected  *  Elm St corridor: 3 streets     |
+----------------------------------------------------------+
| ^ Elm Street          [undo]  [! 1 unresolved]   [x]     |
|                                                          |
|   +----+//////+--------+------+----+                     |
|   | sw | bike | lanes  | park | sw |                     |
|   +----+//////+--------+------+----+                     |
|     1     2        3        4      5                     |
|                                                          |
|   Hatched lanes are estimated: the map data did not      |
|   give a width, so this one was worked out.              |
|                                                          |
|   Lane 2 of 5: bike lane                                 |
|   Type   [ bike lane           v ]                       |
|   Width  [ - ]  1.8 m (estimated)  [ + ]                 |
|                                                          |
|   [+ Add lane] [Remove lane] [Extend along corridor >]   |
+----------------------------------------------------------+
```

<!-- Text fallback: the map above shows the selected street highlighted and, to
its right, the name and street count of the design it belongs to. Below, a
drawer shows the street name, an undo control, a badge reading one unresolved,
and a close control. The lanes appear as numbered adjacent blocks, one of them
hatched. A sentence explains that hatched lanes are estimated. Below that, the
selected lane's type and width as form controls, with the width value carrying
the word estimated. Three actions: add lane, remove lane, extend along
corridor. -->

Three things in this screen are new relative to `wireframes.md` S3.

**Provenance is visible three ways at once** (Q2). The lane block is hatched;
the value reads "1.8 m (estimated)"; and one sentence per view explains what
estimated means. That redundancy is deliberate and each channel serves a
different reader - someone scanning the drawing, someone reading the number, and
someone whose screen reader announces neither hatching nor parentheses unless
told to. The third channel is specified in `interaction-spec.md` as the lane's
accessible name.

**Undo is a persistent control in the header** (Q5), not a toast action. It is
present in every drawer state including the phone peek.

**The unresolved badge** appears only when this street has a correction that
could not be re-applied. It opens S9.

**Keeping a design off this device is a design-level action, not a street-level
one**, so its control sits in the map header beside the design's name and street
count rather than in the drawer header. The drawer is scoped to one street; a
design spans several, and uploading one street of a corridor is not a thing
anyone wants.

```
+----------------------------------------------------------+
| Logo  [ Search a place       ]       [Sign in]           |
+----------------------------------------------------------+
|                      MAP                                 |
|   Elm Street selected  *  Elm St corridor: 3 streets     |
|                           [ Keep off this device ]       |
+----------------------------------------------------------+
| ^ Elm Street          [undo]  [! 1 unresolved]   [x]     |
+----------------------------------------------------------+
```

<!-- Text fallback: the map header row shows the selected street, the design it
belongs to with its street count, and a control reading "Keep off this device".
Below it the drawer header shows the street name, undo, the unresolved badge and
close. -->

### Keeping a design off this device — the three states

US8.3 is a Stage 1 Must Have. It was raised from Should Have at Q12 because
AC8.2.3's mandatory message names uploading as the mitigation, and an apology
naming a capability that does not exist is worse than no apology.

```
+----------------------------------------------------------+
|  Keeping "Elm St corridor" off this device...            |
|  [=========================                    ]         |
|                                                          |
|  ---------------------------------------------           |
|                                                          |
|  Kept off this device.                                   |
|  This design now exists on the service as well as        |
|  in this browser.        [ Remove from service ]         |
|                                                          |
|  ---------------------------------------------           |
|                                                          |
|  Could not keep this design off the device.              |
|  It is still here in this browser, and nothing was       |
|  lost.                          [ Try again ]            |
+----------------------------------------------------------+
```

<!-- Text fallback: three stacked states. In progress, naming the design and
showing a progress bar. Succeeded, stating the design now exists on the service
as well as in this browser, with an action to remove it from the service.
Failed, stating the design is still in this browser and nothing was lost, with a
try-again action. -->

Three things about the wording, each answering an acceptance criterion:

- **"Kept off this device", not "Uploaded"** (AC8.3.1). The user's concern is
  losing work when a browser clears its storage, not the mechanics of a
  transfer. The message names the outcome they care about.
- **Failure states that nothing was lost** (AC8.3.3). The design remains on the
  device; a failed upload is not a failed save. Saying so is the difference
  between a retry and a panic.
- **"Remove from service" needs no account** (AC8.3.4). The design was uploaded
  without one and must be removable without one, or the product has taken
  something it will not give back.

**Nothing leaves the device until this control is used** (AC8.3.2). That is the
whole point of Q9's browser-first decision, and the control is the only thing
that crosses that line.

**Satisfies**: US2.2, US3.1 (AC3.1.1, AC3.1.3), US4.1 (AC4.1.1-AC4.1.6), US4.2,
US5.1, US5.2, US5.5, US7.3, US7.5, US8.3 (AC8.3.1-AC8.3.4). **Not AC8.3.5** —
see Assumptions & Open Questions.

**Accessibility**: h2 is the street name inside the drawer; landmarks header,
main, and the drawer as a labelled complementary region; keyboard entry point is
the first lane, arrow keys move between lanes, Escape closes the drawer. Each
lane is individually focusable (AC4.1.5).

### S3 at 360 pixels - peek

```
+----------------------------------+
| [ Search a place ]  [=]          |
| Elm St corridor: 3 streets       |
|      [ Keep off this device ]    |
+----------------------------------+
|                                  |
|            MAP                   |
|     Elm Street selected          |
|                                  |
+----------------------------------+
| ^ Elm Street [undo] [!] [x]      |
|                                  |
|  +---+/////+------+----+---+     |
|  | sw| bik |lanes | pk | sw|     |
|  +---+/////+------+----+---+     |
|   1    2      3     4   5        |
|                                  |
| Tap a lane to edit               |
+----------------------------------+
```

### S3 at 360 pixels - full

```
+----------------------------------+
| v Elm Street [undo] [!] [x]      |
| Elm St corridor    [ Design v ]  |
+----------------------------------+
|  +---+/////+------+----+---+     |
|  | sw| bik |lanes | pk | sw|     |
|  +---+/////+------+----+---+     |
|   1    2      3     4   5        |
|                                  |
| Hatched lanes are estimated:     |
| the map data did not give a      |
| width, so this one was           |
| worked out.                      |
|                                  |
| Lane 2 of 5: bike lane           |
|                                  |
| Type                             |
| [ bike lane             v ]      |
|                                  |
| Width                            |
| [ - ] 1.8 m (estimated) [ + ]    |
|                                  |
| [+ Add lane] [Remove lane]       |
| [ Extend along corridor > ]      |
+----------------------------------+
```

### S3 at 360 pixels - full, design disclosure open

```
+----------------------------------+
| v Elm Street [undo] [!] [x]      |
| Elm St corridor    [ Design v ]  |
| +------------------------------+ |
| | Elm St corridor - 3 streets  | |
| | [ Keep off this device ]     | |
| | [ See all streets ]          | |
| +------------------------------+ |
+----------------------------------+
|  +---+/////+------+----+---+     |
|  | sw| bik |lanes | pk | sw|     |
|  +---+/////+------+----+---+     |
|   1    2      3     4   5        |
+----------------------------------+
```

<!-- Text fallback: in peek, the header carries a collapsed search field and a
menu control on one line, the design's name and street count on the second, and
the keep-off-this-device control on the third. Below it the map remains visible
above a short drawer showing the street name, undo, the unresolved badge, close,
the lane strip and a prompt to tap a lane. In full, the drawer covers the map;
its header is two lines - the street name with undo, the unresolved badge and
close, then the design name with a Design disclosure control. Opening it
reveals the design's name and street count, the keep-off-this-device control and
a see-all-streets action. -->

**The map header is three lines at this width, not one.** Search and the menu
share the first; the design name and street count take the second; the
keep-off-this-device control takes the third. **The label is not abbreviated at
any width.** An abbreviated visible label whose accessible name is longer fails
WCAG 2.5.3 Label in Name unless the visible text is contained in the accessible
name, and the simplest way not to fail it is not to abbreviate.

**In the full sheet the map header is not visible**, because the sheet covers
the map. Design-level actions are reached through an explicit **Design**
disclosure button in the sheet's own header, which reveals a labelled group
containing the design's name and street count, the keep-off-this-device control
and a see-all-streets action.

That control exists because the alternative failed. The first attempt made the
map header reachable by tabbing past the sheet's last element, which collapsed
the sheet and moved focus - **a change of context caused by focus arriving, which
WCAG 3.2.1 On Focus prohibits outright.** Revealing a group by pressing a button
is a change of context caused by *activation*, which is permitted, and it is
also more discoverable than a hidden tab stop.

**It is a disclosure, not a menu**, and the distinction is load-bearing rather
than pedantic. Calling it a menu invokes the ARIA menu-button pattern, whose
children must all be menu items - and the popup's first line is the design's
name and street count, which is informational text. Under `role="menu"` that
line is a non-conformant child and assistive technology is not obliged to expose
it, which would fail Name/Role/Value (WCAG 4.1.2) on exactly the content the
popup exists to surface. A disclosure button revealing an ordinary group has no
such obligation, needs no arrow-key handling, and does everything required.
`interaction-spec.md` carries the full component specification.

The lane strip abbreviates labels at this width but keeps every lane visible and
individually targetable - AC5.3.4's 44 by 44 CSS pixel activation target applies
to the lane's target region, never to its drawn width, which is data.

### S3 - import failed

```
+----------------------------------------------------------+
| ^ Elm Street                                     [x]     |
|                                                          |
|   This street's data could not be read.                  |
|   The map does not describe its lanes in a way this      |
|   tool can use.                                          |
|                                                          |
|   [ Try again ]   [ Start from a blank cross-section ]   |
|                                                          |
|   Trying again is worth it if you are on a patchy        |
|   connection. A blank cross-section keeps you moving,    |
|   but everything on it will be marked as your own        |
|   entry rather than measured.                            |
+----------------------------------------------------------+
```

<!-- Text fallback: the drawer states that this street's data could not be read
and that the map does not describe its lanes usably. Two actions: try again, and
start from a blank cross-section. A note explains when each is the better
choice. -->

**Retry is offered first, and it is offered at all** - `wireframes.md` S3 and
`user-flow.md`'s recovery table both promised it and no story delivered it until
AC7.2.1. The likely failures here are transient: a timed-out fetch, a network
blip on a phone, a rate limit. Forcing someone to a blank cross-section after
one throws away the imported street they came for, and drops them into a world
where everything they enter is `user-set`.

The message names the street by the same name shown at selection and states
which typed failure reason applies (AC7.1.1). It carries no stack trace, no
library text and no internal identifier (AC7.1.2).

**Satisfies**: US7.1, US7.2.

---

## S4 - Extend along a corridor (stage 1)

Q6 keeps this as one screen. The checklist carries per-street status; the
confirmation replaces it in place.

```
+----------------------------------------------------------+
| ^ Extend Elm Street's design                     [x]     |
|                                                          |
|   Connected streets:                                     |
|                                                          |
|   [x] Oak Street     120 m   Fits                        |
|   [x] Pine Street    200 m   Does not fit - narrower     |
|                              than the design by 1.2 m    |
|   [ ] Maple Street    90 m   Fit could not be checked    |
|                              - no measured width         |
|   [ ] Birch Street    60 m   Not yet checked             |
|                                                          |
|   Pine Street will receive the design unchanged and      |
|   stay marked as not fitting. Lanes are never scaled.    |
|                                                          |
|   [Cancel]                    [Apply to 2 streets]       |
+----------------------------------------------------------+
```

<!-- Text fallback: the drawer lists connected streets as checkboxes with their
lengths and a fit status each: fits, does not fit with the shortfall stated, fit
could not be checked with the reason, or not yet checked. A note explains that a
non-fitting street receives the design unchanged and stays marked. Cancel and
apply actions, the apply naming how many streets are selected. -->

**Four fit states, not two.** `wireframes.md` had a design either fitting or
being warned about. The stories added two more, and both matter:

- **Fit could not be checked** (AC6.2.3, AC6.2.4) covers a street whose
  carriageway width is wholly derived from default tables, or that has no width
  at all because its import failed. Warning on a default-versus-default
  comparison would present a guess as a finding - the exact failure the
  provenance model exists to prevent. Saying the check could not run is the
  honest outcome.
- **Not yet checked** is the pre-selection state. Without it, an unchecked
  street is indistinguishable from one that passed.

Each state is a word, not an icon or a colour. At 360 pixels the row wraps to
two lines rather than truncating the status.

### S4 - after applying

```
+----------------------------------------------------------+
| ^ Extend Elm Street's design                     [x]     |
|                                                          |
|   Applied to 2 streets.                                  |
|                                                          |
|   Oak Street      design applied, fits                   |
|   Pine Street     design applied, does not fit           |
|   Maple Street    not selected                           |
|   Birch Street    not selected                           |
|                                                          |
|   Undo is in the drawer header and reverses all of       |
|   this in one step.                                      |
|                                                          |
|   [ Back to Elm Street ]                                 |
+----------------------------------------------------------+
```

<!-- Text fallback: the drawer reports how many streets received the design and
lists every connected street with its outcome, including those not selected. It
states that undo is in the drawer header and reverses the whole apply in one
step, and offers a return to the source street. -->

AC6.4.2 requires the confirmation to name how many streets received the design,
which were warned and which could not be checked. AC6.4.3 makes the operation
non-atomic: a street that could not be prepared does not roll back the others,
and is named.

**Satisfies**: US6.1, US6.2, US6.3, US6.4.

**Accessibility**: h2 is the drawer heading; the street list is a labelled
checkbox group where each label includes the fit status, so the status is
announced with the street rather than as separate decoration; keyboard entry
point is the first checkbox. The outcome list is announced through the status
region (US13.1).

---

## S9 - A correction that could not be re-applied (stage 1, new)

Nothing in `wireframes.md` covers this. It is AC7.5.4: you corrected a lane, the
street was imported again, OpenStreetMap changed underneath, and the lane your
correction attached to no longer resolves. The correction is retained and
surfaced - neither applied nor discarded.

```
+----------------------------------------------------------+
| ^ Elm Street                    [undo]           [x]     |
| +------------------------------------------------+       |
| | ! 1 correction could not be re-applied         |       |
| |                                                |       |
| | You set lane 2's width to 2.4 m. The map data  |       |
| | for this street has changed and that lane is   |       |
| | no longer there.                               |       |
| |                                                |       |
| | Your correction has been kept, not applied.    |       |
| |                                                |       |
| | [ Apply to a lane... ]   [ Discard ]           |       |
| +------------------------------------------------+       |
|                                                          |
|   +----+//////+--------+------+----+                     |
|   | sw | bike | lanes  | park | sw |                     |
|   +----+//////+--------+------+----+                     |
|     1     2        3        4      5                     |
+----------------------------------------------------------+
```

<!-- Text fallback: a banner at the top of the drawer states that one correction
could not be re-applied, quotes what the correction said, explains that the map
data for the street has changed and the lane is gone, states that the correction
has been kept rather than applied, and offers to apply it to a chosen lane or to
discard it. The cross-section appears below, unaffected. -->

Q4 puts the entry point in two places: a marker on the street in the map, so a
corridor of ten streets does not require opening each one to discover the
problem, and the badge in the drawer header shown in S3.

**Neither action is destructive by default.** "Apply to a lane..." enters a
selection mode; "Discard" is the only way to lose the correction, and it is
never the default focus.

**Satisfies**: US7.5 (AC7.5.4).

**Accessibility**: the banner is a labelled region, not an alert - it persists
rather than interrupting. It is announced on drawer open via the status region.
Keyboard entry point is the first action in the banner, which precedes the lane
strip in tab order because it is a blocking question about the data below it.

---

## S10 - That design is not on this device (stage 1, new)

```
+----------------------------------------------------------+
|                                                          |
|        That design is not on this device.                |
|                                                          |
|        Designs are kept in this browser unless you       |
|        upload them. This one is not here - it may        |
|        have been on another device, or this              |
|        browser's storage was cleared.                    |
|                                                          |
|        [ Find a street and start again ]                 |
|                                                          |
+----------------------------------------------------------+
```

<!-- Text fallback: a centred message stating that the design is not on this
device, explaining that designs are kept in this browser unless uploaded and
that this one may have been on another device or the browser storage was
cleared, with a single action to find a street and start again. -->

**This screen is shown only when a route names a specific design that device
storage does not hold** (AC8.2.1). A cold start with empty storage and no design
named in the route shows S2's normal first-run prompt and no loss message
(AC8.2.2) - because empty storage is also exactly what a genuine first-time
visitor has, and opening the product by telling a stranger it lost their work is
the worst first impression available.

**Satisfies**: US8.2.

**Accessibility**: h1 is the message heading; landmarks main; keyboard entry
point is the single action.

---

## S5 - Sign in (stage 2)

Carried forward from `wireframes.md` unchanged, with one criterion made
explicit: the dismissal ("Not now - keep designing") returns to the design with
work intact and does not ask again in the same session (AC9.1.4). The invitation
appears only when an action requires an account - saving under a name, sharing,
or reaching a design from another device. It never fires on an edit, which is
what keeps AC1.2.3 true.

**Satisfies**: US9.1.

---

## S6 - My designs (stage 2)

Carried forward from `wireframes.md`, with three additions.

- Each card's sharing state is announced with the card, not conveyed by an icon
  alone (AC10.3.3).
- The empty state is drawn, not implied (AC9.3.4).
- Each card carries the same "Keep off this device" control and its removal
  counterpart, in the same three states drawn under S3. A design reached from an
  account is still held in the browser until it is uploaded.

**Satisfies**: US8.3, US9.3, US10.3.

---

## S7 - Share (stage 2, redrawn)

```
+----------------------------------------------------------+
| Share "Elm St corridor"                          [x]     |
|                                                          |
|   This design is private. Two ways to open it up,        |
|   and they work independently.                           |
|                                                          |
|   People you invite                                      |
|   [ name@example.com          ]      [ Invite ]          |
|   Currently: nobody                                      |
|                                                          |
|   Anyone with the link                    [ Off ]        |
|   [ https://.../d/a1b2c3     ]      [ Copy ]             |
|   The link does nothing while this is off.               |
|                                                          |
|   Turning the link on does not change who you have       |
|   invited. Turning it off does not remove them.          |
+----------------------------------------------------------+
```

<!-- Text fallback: a share dialog stating that the design is private and that
two ways to open it exist which work independently. First, people you invite: an
email field with an invite action and a line naming who currently has access.
Second, anyone with the link: an off-on toggle, the link with a copy action, and
a line stating the link does nothing while off. A closing note states that
turning the link on does not change who has been invited, and turning it off
does not remove them. -->

**This is the correction.** The approved screen drew three mutually exclusive
radios, which made "People I invite" and "Anyone with the link" alternatives.
FR8.2 requires them as separate explicit choices, and AC10.2.3 requires them to
be independent in both directions.

The closing sentence exists because "separate explicit choices" does not by
itself tell a user that the two do not interfere. In the one dialog where a
surprise is least acceptable, the interface says so outright.

**Satisfies**: US10.1, US10.2.

**Accessibility**: h2 is the dialog title; two labelled groups rather than one
radio group; modal with focus trap and Escape to close; keyboard entry point is
the invite field. The link toggle's state is in its accessible name, and the
inert link field is `aria-disabled` with the explanation associated via
`aria-describedby` rather than sitting nearby as unassociated text.

---

## S8 - Export for a meeting (stage 3)

```
+----------------------------------------------------------+
| Export "Elm St corridor"                         [x]     |
|                                                          |
|   What are you making?                                   |
|    ( ) A page to print or hand out                       |
|    (o) Slides for a presentation                         |
|    ( ) An image for a document or post                   |
|                                                          |
|   Include                                                |
|    [x] Before and after                                  |
|    [x] Street location map                               |
|    [x] Lane dimensions table                             |
|                                                          |
|   +----------------------------------------------+       |
|   |           preview of the output              |       |
|   +----------------------------------------------+       |
|                                                          |
|   Estimated values are marked in the output with the     |
|   same hatching and the word "estimated".                |
|                                                          |
|   [Cancel]                              [Export]         |
+----------------------------------------------------------+
```

<!-- Text fallback: an export dialog asking what the user is making, offering a
printed page, slides or an image; an inclusion checklist for before-and-after,
street location map and lane dimensions table, all checked; a preview of the
output; a note that estimated values are marked in the output; and cancel and
export actions. -->

The question is "what are you making", not "which file format" - carried forward
from `wireframes.md` and still right: the person choosing is an advocate
preparing for a meeting, not someone picking a file type.

**Two additions.** The lane dimensions table is checked by default rather than
unchecked, because AC12.1.3 requires the output to be legible to someone who was
not present when it was made, and a before-and-after with no dimensions is a
picture. And the dialog states that estimated values carry into the output
marked, which is AC12.2.1 made visible before the export rather than discovered
after it.

### S8 without the preview

US12.3 (preview) is Should Have. If it does not ship, the dialog loses the
preview box and nothing else:

```
+----------------------------------------------------------+
| Export "Elm St corridor"                         [x]     |
|                                                          |
|   What are you making?                                   |
|    ( ) A page to print or hand out                       |
|    (o) Slides for a presentation                         |
|    ( ) An image for a document or post                   |
|                                                          |
|   Include                                                |
|    [x] Before and after                                  |
|    [x] Street location map                               |
|    [x] Lane dimensions table                             |
|                                                          |
|   Estimated values are marked in the output with the     |
|   same hatching and the word "estimated".                |
|                                                          |
|   [Cancel]                              [Export]         |
+----------------------------------------------------------+
```

<!-- Text fallback: the same export dialog with the preview box removed. The
format choices, the inclusion checklist, the estimated-values note and the
cancel and export actions are unchanged. -->

The dialog is shorter and the export action moves up. This variant exists so the
approved screen does not have a hole in the middle if the Should Have is not
built.

**Satisfies**: US12.1, US12.2, US12.3.

---

## Screen states, applied across the set

`wireframes.md` established five states. Two rows change and three are added.

| State | Where it shows | Treatment |
|---|---|---|
| Empty | S2 with nothing selected; S6 with no designs | A prompt with a next action, never a bare surface |
| Loading | S3 while a street imports | The drawer opens immediately with the street name and a skeleton cross-section, so the selection is acknowledged before the data arrives (AC3.1.3) |
| Error | S3 when import fails | States plainly that the street's data could not be read, names the typed failure reason, and offers retry first and a blank cross-section second (AC7.1.1, AC7.2.1) |
| Partial | S3 when tagging is thin | Derived lanes are hatched, their values carry "(estimated)", and one sentence per view explains what that means (AC4.1.2, AC4.1.6) |
| Success | S4 after applying; S8 after exporting | A confirmation naming what happened and pointing at the persistent undo control. **Changed**: the confirmation no longer carries the undo action itself (Q5) |
| Unresolved | S3 badge and S9; map marker | A correction that survived but could not be re-applied. Persistent, not an alert; never discarded without an explicit action (AC7.5.4) |
| Storage unavailable | S3 on open | Stated before any work is invested, not on first edit (AC8.2.4). The tool stays usable for the session (AC8.2.6) |
| Save failed mid-session | S3, at the moment it happens | Surfaced immediately; the session's work stays available in the open tab until it is closed (AC8.2.5) |

## Information architecture

Stage 1 has no navigation beyond the map, because Q7 declined a design overview
screen. Three surfaces, and the map carries what an overview would have:

- **Landing (S1)** - entry, worked example, route into the tool.
- **Map (S2, S3, S4, S9)** - the working surface. Carries the street network,
  the current design's streets highlighted, unresolved-correction markers, and
  post-apply fit status. The header shows the current design's name and street
  count, and opens a list of its streets.
- **Message (S10)** - the one full-screen state, for a named design that is not
  on this device.

Stage 2 adds S5, S6 and S7. Stage 3 adds S8.

## Assumptions & Open Questions

- **Which CSS framework.** Q3 chose the approach - a CSS-only framework with
  project tokens layered on top - not the dependency. Naming one depends on a
  decision this stage does not own: whether the editing surface is DOM or canvas,
  and which Rust framework renders it. A classless framework suits the "native
  HTML elements first" rule; a class-based one suits a component library. It also
  adds a licence row to the manifest `scripts/verify.sh` checks. For Domain
  Design. [assumption]
- **Whether four map overlays plus a legend survive at 360 pixels.** The
  treatment is specified in `interaction-spec.md`, but whether it holds is a
  question for the first build rather than something this stage can assert.
  [assumption]
- **The hatch pattern's contrast.** A texture must meet 3:1 against its
  background as a graphical object under WCAG 1.4.11, and hatching at small
  sizes can fail on a lane block only a few pixels wide. The fallback if it
  does is stated in `accessibility-checklist.md`: the qualifier and the
  accessible name carry the distinction alone, and the hatch becomes
  decoration. [assumption]
- **AC8.3.5's identifier is not specified here.** The criterion requires an
  uploaded anonymous design to be stored against an identifier carrying no name,
  no contact details and no cross-site linkage. That is a persistence and
  privacy mechanism, not a screen, and nothing in this stage's four artifacts
  treats it. It is named here rather than claimed as satisfied, because a
  Satisfies line that cites a criterion nothing addresses is the same defect the
  reviewer raised against US8.3's original placement, one criterion narrower.
  For Domain Design. [assumption]
- **Whether a person a design is shared with needs an account to view it**
  (`stories.md` OQ-US5) is unresolved and lands on S7. The dialog as drawn does
  not say, because nothing has decided it. For Domain Design.


## Review

**Verdict:** READY
**Reviewer:** aidlc-product-lead-agent
**Date:** 2026-09-10T03:33:14Z
**Iteration:** 1
**Request Challenge:** review:f7204d7c77e0f0ab71381b612f0b5be2

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | `inception/refined-mockups/mockups.md` | (prior finding, resolved in an earlier pass) | — | Resolved |
| R-02 | Major | `inception/refined-mockups/mockups.md` | (prior finding, resolved in an earlier pass) | — | Resolved |
| R-03 | Major | `inception/refined-mockups/mockups.md` | (prior finding, resolved in an earlier pass) | — | Resolved |
| R-04 | Major | `inception/refined-mockups/mockups.md` | (prior finding, resolved in an earlier pass) | — | Resolved |
| R-05 | Major | `inception/refined-mockups/mockups.md` \> "S3 at 360 pixels - peek" and "S3 at 360 pixels - full"; `inception/refined-mockups/interaction-spec.md` \> "Keep-off-this-device control" \> "Responsive behaviour" | Narrow-viewport diagrams did not draw the map-header row containing the keep-off-device control. | — | Resolved |
| R-06 | Major | `inception/refined-mockups/mockups.md` | (prior finding, resolved in an earlier pass) | — | Resolved |
| R-07 | Major | `inception/refined-mockups/interaction-spec.md` \> "Keep-off-this-device control" \> "Accessibility" (ARIA role row) | The R-05 fix introduced a WCAG 2.5.3 (Label in Name) failure — the abbreviated visible label "Keep off device" was not a contiguous substring of the accessible name. | — | Resolved |
| R-08 | Major | `interaction-spec.md` \> "Keep-off-this-device control" \> "Accessibility" (Keyboard interaction row) | The R-05 fix's tab-past-to-collapse keyboard escape hatch was a WCAG 3.2.1 (On Focus) risk — a focus event triggered an automatic context change. | — | Resolved |
| R-09 | Major | `inception/refined-mockups/interaction-spec.md` \> "Keep-off-this-device control" \> "Responsive behaviour" and "Accessibility"; `inception/refined-mockups/mockups.md` \> "S3 at 360 pixels - full, design menu open" | The R-08 fix introduced the `Design` disclosure button and its popup with none of the ARIA-role specification rigor every other component in `interaction-spec.md` carries, and prose called it "a menu" with arrow-key navigation while the diagram drew the popup's informational first line as non-interactive text — a Name/Role/Value risk under `role="menu"`. | — | Resolved |
| R-10 | Major | `inception/refined-mockups/interaction-spec.md` \> "Keep-off-this-device control" \> "Responsive behaviour"; `inception/refined-mockups/accessibility-checklist.md` \> row U8a | The R-09 fix left two sibling places still calling the `Design` control a menu. | Rename both to disclosure terminology consistent with the `## Design disclosure` section. | Resolved |
| R-11 | Major | `inception/refined-mockups/interaction-spec.md` \> "Design disclosure" \> "Accessibility"; \> "Focus and keyboard rules that span components" (rule 2) | The popup's Escape behaviour was never reconciled with the cross-component "Escape is layered" rule, which enumerated three layers and omitted the popup, leaving two unreconciled specifications for the same state. | Add the popup as an explicit layer stating it consumes the press. | Resolved |

### Summary

Verified, not just accepted. `interaction-spec.md`'s `KeepOffDeviceControl` > Responsive behaviour row now reads "reached through the `Design` disclosure button" and `accessibility-checklist.md`'s U8a row now names `BottomSheet, DesignDisclosure, map header` and asserts against "the disclosure-open state" — both R-10 instances are corrected, and a grep for every remaining "menu" hit in the four artifacts turns up only the unrelated map-header search-and-menu control and historical review-log prose, never the Design control. R-11's rule 2 is now an exhaustive four-row table (modal, disclosure open, sheet full, sheet peek) stating the disclosure's Escape "consumes the press," and the component-level Keyboard/Focus rows for `DesignDisclosure` ("Escape closes the popup and returns focus to the trigger") do not contradict it — the cross-component table is the layer that carries precedence, the component row states the local behaviour, and the two now genuinely agree rather than agreeing by luck. "Consumes the press" is precise enough to implement: the table's own "Next press" column resolves the ordering explicitly (press while popup open closes only the popup; a subsequent press, with the popup already closed, collapses the sheet), so there is no ambiguity about which handler fires first.

I ran the sweep myself rather than take the lead's word for it: grepped all five artifacts for the menu/disclosure rename, the "Keep off device" abbreviated label, AC8.3.5, "scaled to fit," the S8 preview variants, the toast/undo wording, and the six/seven-stories count. Every one of those is now consistent except the one the lead flagged and chose not to silently fix: `refined-mockups-questions.md`'s preamble still says "Six" while listing seven stories. Leaving a confirmed Q&A record un-edited and instead adding a superseding note in `mockups.md` is the right call — that file is provenance for what was asked and answered, and silently correcting it after confirmation would be worse than a visible discrepancy. The mitigation is adequately placed: mockups.md line 50-52, right next to the table that gives the correct count of seven, states plainly that the questions file's "Six" is superseded. A reader who opens only the questions file in isolation would still see the wrong count with no pointer forward, but that file is a stage-internal working artifact rather than one a developer or QA engineer would implement from — the two artifacts a builder actually works from (`mockups.md` and `interaction-spec.md`) carry the correct count and, in mockups.md's case, the explicit correction. I checked the sweep did not introduce anything new: the S8 preview/without-preview pair reads consistently with the corrections table, AC8.3.5's "not specified here" language matches in both `mockups.md` and `interaction-spec.md`, and the R-09/R-10/R-11 finding-table entries the lead added are accurate summaries of what changed, not the source of any new discrepancy.

This pass is clean because the work is done, not because the risk moved out of view. Unlike the last two passes, this one shows the actual signature of a completed sweep: every named rename this stage made is now consistent across all five files, not just the two files a point-fix would touch, and the one item deliberately left inconsistent (the confirmed questions-file count) is inconsistent by policy, disclosed, and pointed at from the artifact a reader would actually use. Approve.
