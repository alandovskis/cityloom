# The Add a piece menu, by topic (design)

Date: 2026-11-17. Scope: the street page's Add a piece menu (`street::view::page::AddMenu`, `#add-btn`, `#add-menu`).

## Why

People cannot find what they need in the menu. It is sorted by how the app stores things, not by what a person is
looking for: seven groups of pieces (Walking, Greenery, Cycling, Transit, Roadway, Furniture, Utilities) and then
three more groups of things that are not pieces (Green infrastructure, Traffic calming, Transit stops). Two of those
repeat a topic already there ("Greenery" and "Green infrastructure", "Transit" and "Transit stops"), so a person has
to know the app's internal split to know where a bus stop or a rain garden is. The three extra groups also appear and
vanish with the street, with nothing saying why, so an item someone saw yesterday is simply missing today. And the
whole thing is one long scroll of up to ten groups and 29 items.

## Goals

- One group per topic, holding everything about that topic whatever kind of thing it is (piece, stop, green element,
  traffic calming element).
- Nothing vanishes: an item that cannot go on this street stays in its topic, greyed out, with the reason under it.
- One topic shown at a time, chosen from a list of topics on the side; a search box at the top that looks through
  every topic.
- The menu opens on the topic of what is selected.

## Non-goals

- What adding does. Each item adds exactly what it adds today, where it adds it today (see **Items**).
- Synonyms or everyday words in the search. The person chose: names as written only. "bus stop" does not find
  "Curb stop".
- Remembering the last topic between openings or page loads.
- The Transform menu, the junction page, the map. Only the street page has this menu.
- Changing the catalogue (`cityloom_model::catalogue`), the kinds' names, their defaults or their icons.

## Anti-goals (a failure even if it works)

- A topic name that repeats another, or an item reachable under two topics.
- An item that disappears instead of saying why it cannot be added.
- A search that finds nothing for a name that is plainly on the screen (for example because of a capital or an
  accent).
- Losing anything the current menu gives a keyboard or screen-reader user: every item reachable without a pointer,
  Escape back to the button, each item named with its default size.

## The topics and their items

Eight topics, in this order. A piece keeps its catalogue icon and default width; a stop and a traffic calming element
show their default length, as today; a green element shows no size, as today. Items within a topic in the order below.

| Topic (message) | Items |
| --- | --- |
| Walking (`group-walking`) | Sidewalk |
| Greenery (`group-greenery`) | Planting strip, Planted median, then the green elements: Tree, Planting bed, Rain garden, Bioswale |
| Cycling (`group-cycling`) | Bike lane, Bike rack, Bikeshare station |
| Transit (`group-transit`) | Transit lane, Bus shelter, Bus station, then the stops: Curb stop, Curb extension, Boarding island |
| Roadway (`group-roadway`) | Driving lane, Parking, Loading zone, Shoulder, Buffer |
| Traffic calming (`group-calming`) | Raised table, Raised crossing, Chicane, Modal filter |
| Furniture (`group-furniture`) | Bench, Café terrace |
| Utilities (`group-utilities`) | Utility pole, Street lamp |

- The messages `group-green` and `group-stops` are no longer used: remove them from both `.ftl` files (the parity test
  and the i18n guard would otherwise keep a dead entry). `group-calming` ("Traffic calming") stays as the topic's
  name. Traffic calming is its own topic, not under Roadway (my call: nothing else covers it, and it is four items).
- Every item is in exactly one topic; a test holds that the topics cover every catalogue kind, every `StopKind`,
  every `GreenKind` and every calming `Kind` once.
- The topic list is a single source in the street slice (it replaces `ADD_GROUPS`), so the menu, the opening topic and
  the tests read the same table. `shared::catalogue::GROUP_IDS` follows it (Traffic calming added between Roadway and
  Furniture) or is retired if nothing else needs it (builder's call).

## Items: what choosing one does (unchanged)

- A piece is added after the selected piece (`insert_index`, `Editor::add`), as today.
- A stop: `pick_stop_kind` then `add_stop(kind, stop_min_mm())` into the chosen free stretch of a stops row; no free
  stretch chosen is refused and said, as today.
- A traffic calming element: `pick_calming_kind` then `add_calming(kind, kind.default_mm())` into the chosen free
  stretch, refused and said when none is chosen, as today.
- A green element: `add_green_clear(GreenForm::new(kind))` at the first free place from the street's start, as today.
- After a choice the menu closes and the focus goes back where it goes today.

## Greyed-out items and their reasons

Where today a whole group is hidden, its items now stay listed, greyed out, with one short reason under the group of
items it applies to (not repeated under each item). The conditions are exactly what hides the group today:

| Items | Unavailable when | Reason says (English; fr-CA alongside) |
| --- | --- | --- |
| The three stops | `view.stops.sides` is empty | the street has no length to lay stops along, or no lane a bus can stop in (`curbside` or `bus_lane` on either side in any zone); one reason per cause |
| The four calming elements | `view.calming_strip.cells` is empty | the street has no length to lay them along |
| The four green elements | `!view.green.can_add` | no piece off the roadway to stand one on, or the street's ends are not ones it can be measured from (`curb_frame` refusal: no anchor, elsewhere); one reason per cause |

- When two causes apply at once (no anchor and no piece off the roadway; no length and no lane), only the first in
  the table's order is shown: no length before no lane, no anchor/elsewhere before no piece off the roadway. My call.
- The view-model says which cause applies, as `Said` data (a model or view-model value, not wording in the view), so
  each reason is a whole sentence in both `.ftl` files. Where a refusal message already says the same thing (for
  example `curb-refused-*` for no anchor or elsewhere), reuse it rather than writing a second wording.
- A greyed item works like the Transform menu's unsuitable templates: `aria-disabled="true"` (still reachable by the
  keyboard), the reason linked by `aria-describedby`, `--ink-2` text (already measured in `shared/contrast.rs`).
  Choosing it adds nothing, keeps the menu open and announces the reason.
- A piece is never greyed: a piece can always be added.

## Layout

- **Search box** at the top of the menu, under the menu's heading, labelled (visibly or not) in both languages.
- Under it, two columns: the **topics** on the left (the eight names), the **chosen topic's items** on the right.
  The chosen topic is marked by more than colour.
- Placement hints: a topic's items are laid in runs by what they are (pieces, then green elements, stops or calming
  elements), and each run has one short hint above it saying where its items go: pieces "Goes after the selected
  piece." (`add-hint`, kept), stops and calming elements "into the chosen free stretch" of their row, green elements
  "at the first free place along the street" (builder's wording, both languages). A topic with one run shows one hint.
  In the search results no hint is shown; each item's placement is part of its description (`aria-describedby`)
  there instead. My call: a single hint that is wrong for half a topic's items would be worse than none.
- **Narrow screens** (where the two columns do not fit; the phone width the menu already supports, `menu_box`): the
  topics become one row of tabs across the top, scrolling sideways inside itself if needed; the items under it. The
  person chose this.
- The menu keeps its place and size rules (`menu_box`: fixed to the window, fits a short window and a phone).
- Every target at least 24 px (44 px on a coarse pointer), as `.add-item` today.

## Search

- Matches the item's name as shown, in the page's language, anywhere in the name, ignoring case and accents
  (« arret » finds « Arrêt… », "cafe" finds "Café terrace"). Topic names are not searched. My call on accents and
  "anywhere in the name".
- While the box holds text, the topic columns are replaced by one flat list of every matching item across all topics,
  in topic order, each with its topic's name beside it. Greyed items match and stay greyed with their reason.
- No match: a line saying nothing matches (both languages).
- The number of matches is announced politely as the person types (both languages, plural selectors).
- Clearing the box returns to the topic view, on the topic that was showing.
- The search text is cleared each time the menu opens.

## Opening topic

The menu opens on the topic of what the details panel shows (`StreetVm::level()`):

- Piece: the topic holding the selected piece's kind.
- Stop: Transit. Calming: Traffic calming. Green: Greenery.
- Curb (a curbside use): the topic holding the use's kind (`Use::kind_id`), so parking opens Roadway, a street tree
  Greenery. My call. A chosen free stretch of a curbside row has no use: Walking.
- Street or Zone (nothing chosen below them): Walking, the first topic. My call (the person did not ask for the last
  topic to be remembered).

## Keyboard and assistive technology

- The button stays a disclosure (`aria-expanded`, `aria-controls`, no `aria-haspopup`), as today; ArrowDown on the
  closed button opens it.
- On opening, the focus goes to the search box. Typing searches.
- The topics are a vertical `tablist` (horizontal on a narrow screen) and the items a `tabpanel` of buttons in a list:
  arrows move through the topics and show each as they go; Tab moves on into the topic's items; arrows move within
  the items, Home/End to the first and last. My call on the exact roles; the builder may choose another WAI-ARIA
  pattern if it serves the same moves better, as long as the a11y spec passes.
- From the search box, ArrowDown goes to the first item shown (a match, or the topic's first item).
- `MenuParts` (shared with the Transform menu) today focuses the first item on opening and arrows through one flat
  list; the Add menu's new focus and keys either make it configurable or give `AddMenu` its own handling, without any
  change to how the Transform menu behaves.
- Escape closes the menu and returns the focus to the button, from anywhere inside it; leaving the menu by focus
  closes it, as today.
- Each item's accessible name is its name and default size; a greyed item is described by its reason.

## Language

- Every new string (search label, placeholder if any, no-match line, match count, the reasons, any placement hint) in
  `street/i18n/en.ftl` and `fr.ftl`, whole sentences, OQLF rules for fr-CA, terms from `docs/glossary-fr-ca.md`.
- The menu follows a language switch while open (the tracked `tr`/`say`), including the search, which matches the
  names in the new language.

## Tests

- View-model (native, against the port fakes): the opening topic for each level; the reason (or none) for each of
  stops, calming and green on streets that have and lack each cause; the search's matches (case, accents, French,
  greyed items included, topic order).
- Component (`street/view/tests.rs`): eight topics, every item once, greyed items with `aria-disabled` and a
  described-by reason, the opening topic, no `group-green`/`group-stops`. The existing add-menu tests change to the new
  shape.
- e2e: every spec that opens the menu (`editors`, `a11y`, `language`, `street`, `green`, `stops`, `calming`, and the
  `map` and `junction` specs that reach the street page's `#add-btn`) is updated to choose a topic or search before
  choosing an item. New cases: search finds an item and adds it; a greyed item says its reason and adds nothing; the
  menu opens on the selected piece's topic; the narrow layout at phone width; the a11y scan of the open menu.
- `src/street/CLAUDE.md` is updated where it describes the menu (the groups "Transit stops", "Green infrastructure",
  "Traffic calming", the classes `.stop-add-item`, `.green-add-item`, `.calming-add-item` if they change).

## Constraints

- Leptos 0.8 view in the street slice; the view decides nothing: which topic is open, what matches, and what is
  greyed with which reason come from the view-model (`StreetVm` or a sub-view-model in `street/vm/`), tested natively.
- WCAG 2.2 AA; any new colour pair goes in `shared/contrast.rs`.
- A minor version bump (a feature) in `crates/cityloom_editor/Cargo.toml`, with `cargo update -p cityloom_editor`.

## Left to the builder

- Exact visual styling of the topic column and the chosen topic, within `web/editor.css`'s tokens.
- Whether the search's filtering runs on every keystroke or after a short pause (the list is 29 items).
- The exact wording of the new messages, within the rules above.
- Whether `GROUP_IDS` stays in `shared::catalogue`.
