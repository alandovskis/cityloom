# User Flow — Streetmix at City Scale

The paths a person takes through the product, from arriving to producing
something a city has to respond to. Screen references (S1-S8) are to
`wireframes.md`.

Upstream inputs: `intent-statement.md` for the audience and their pain,
`scope-document.md` for what exists in which stage, `intent-backlog.md` for
ordering.

## The Person

`intent-statement.md` records the primary pain this flow relieves: a community
advocate has no credible way to propose a street change that a city will take
seriously. They arrive without training, often without a specific street in mind,
and frequently on a phone.

They are not a planner. Every step below assumes no prior knowledge of street
design vocabulary.

## Happy Path — Stage 1

The path that exists at the end of stage 1. It ends with a design, which is the
point at which `scope-document.md` says value has been delivered.

```
  [Arrive at hero page]  S1
          |
          v
  [Find your street]
          |
          v
  [Search a place, or use my location]  S2
          |
          v
  [Map centres on the area]
          |
          v
  [Select a street]  ------> keyboard: choose from street list
          |
          v
  [Street imports; drawer opens]  S3
          |
          +--> data thin?  ---> lanes shown as estimated
          |                     (partial state, R-2)
          |
          +--> import fails? --> retry, or start from blank
          |
          v
  [Select a lane]
          |
          v
  [Change its type or width]
          |
          v
  [Repeat for other lanes]
          |
          v
  [A design for one street exists]     <-- value delivered
          |
          v
  [Extend along corridor]  S4
          |
          v
  [Choose connected streets; see width warnings]
          |
          v
  [Apply]
          |
          v
  [A corridor design exists]
```

<!-- Text fallback: the user arrives at the hero page, chooses find your street,
searches or uses their location, and the map centres. They select a street either
on the map or from a keyboard-accessible list. The street imports and the drawer
opens, with branches for thin data (lanes marked estimated) and failed import
(retry or start blank). They select a lane, change its type or width, and repeat.
A single-street design now exists and value has been delivered. They may then
extend along a corridor, choosing connected streets with width warnings shown,
and apply, producing a corridor design. -->

**Steps to first value: five.** Arrive, find, select a street, select a lane,
change it. Everything after that is more of the same or extension.

## Happy Path — Stages 2 and 3

What the flow becomes once accounts, sharing and output exist.

```
  [A corridor design exists]    (end of stage 1 flow)
          |
          v
  [Save]  ---> not signed in? ---> [Sign in]  S5
          |                             |
          |    <------------------------+
          |    (design preserved across sign-in)
          v
  [Design saved, private by default]  S6
          |
          +-------------------+
          |                   |
          v                   v
  [Share]  S7          [Export]  S8      <-- stage 3
          |                   |
          v                   v
  [Choose who sees it]  [Choose what you are making]
          |                   |
          v                   v
  [Invite, or enable    [Slides, print page,
   a link]               or image]
          |                   |
          +--------+----------+
                   |
                   v
       [Take it to a meeting; a city has to respond]
```

<!-- Text fallback: from a corridor design the user saves, which prompts sign-in
if needed while preserving the design. The saved design is private by default and
appears in my designs. From there two branches: share, which asks who can see it
and offers invitation or link; and export, which asks what the user is making and
produces slides, a print page or an image. Both converge on taking the result to
a meeting, where a city has to respond. -->

Two properties of this flow matter more than its shape:

- **Sign-in never precedes value.** The design already exists when sign-in is
  offered, and S5 preserves it. A person who declines keeps designing.
- **Sharing defaults closed.** S7 opens on "Only me", per scope Q5.

## Decision Points

| Where | Choice | Why it is a real fork |
|-------|--------|----------------------|
| S1 | Start designing, or read first | The hero page carries a worked example so the second path is not a dead end |
| S2 | Search a place, or use my location | Public users often have no address in mind; location covers "the street I am standing on" |
| S2 | Click the map, or choose from a list | The list is the keyboard and screen-reader path (Q5), not a lesser alternative |
| S3 | Accept imported lanes, or correct them | Where data is thin the import is a starting point, not an answer (R-2) |
| S3 | One street, or extend | Stage 1's value exists at one street; the corridor is optional |
| S4 | Which connected streets | Width warnings surface fit problems before applying, not after |
| S5 | Sign in, or keep designing | Declining must not cost the work in progress |
| S7 | Only me, invited people, or anyone with the link | Default closed; widening is deliberate |
| S8 | What are you making | Framed by purpose, not by file format |

## Error and Recovery Paths

| Failure | Where | What the person sees | Recovery |
|---------|-------|---------------------|----------|
| Street data cannot be read | S3 | The drawer says this street's data could not be read | Retry, or start from a blank cross-section |
| Street data is thin | S3 | Lanes marked as estimated, with a note that the source did not specify them | Correct any lane directly; nothing is blocked |
| Design does not fit a target street | S4 | Width warning naming the narrower street, before applying | Deselect that street, or apply and review |
| Place search finds nothing | S2 | No results, with a suggestion to try a broader place name | Retry, or use my location |
| Location denied | S2 | Falls back to search without an error dialog | Search instead |
| Sign-in fails | S5 | Inline message on the field, design still intact behind the modal | Retry, or dismiss and keep designing |
| Export produces nothing usable | S8 | Preview shows the result before committing | Change what is included, or cancel |

Two of these are error *prevention* rather than error handling: the S4 width
warning and the S8 preview both show the outcome before it is committed.

## Onboarding

There is no tutorial, no tour, and no first-run wizard. The hero page (S1) carries
a worked before-and-after example, and the empty map (S2) carries a one-line
prompt. A person who understands "this is my street and I want a bike lane on it"
has enough.

This follows from the audience: advocates and the public, arriving without
training. A tutorial they must complete before designing is a barrier in front of
a five-step path.

## Accessibility Through the Flow

The keyboard path is the same journey, not a reduced one:

1. Skip link, then the hero call to action (S1).
2. Search field, or the street list as an alternative to clicking the map (S2).
3. Drawer opens with focus moved into it; arrow keys move between lanes; Escape
   closes (S3).
4. Checkbox group for connected streets (S4).
5. Modals trap focus and return it on Escape (S5, S7, S8).

Every editing action is reachable this way. Drag is an accelerator on top and is
never the only route to an outcome (Q5).

## Assumptions & Open Questions

- Five steps to first value is counted from the wireframes, not measured with a
  person. [assumption]
- The flow assumes street selection on a map is obvious to someone who has not
  used the tool. That is untested and is the most likely place for a first-time
  user to stall. [assumption]
- The stage 2 and 3 portions describe capabilities whose details are unsettled —
  what satisfies the public-release gate, and what output a city actually accepts.
  [assumption]
- No flow covers account deletion or data export, which the public-release gate
  requires. They belong with stage 2's detailed design. [assumption]
- "Copy from previous" appears in the corridor option not recommended for stage 1,
  so it is absent from this flow. If corridors that vary along their length prove
  common, this flow changes. [assumption]
- Onboarding by worked example rather than tutorial is a judgement from the
  audience description, not a tested choice. [assumption]
