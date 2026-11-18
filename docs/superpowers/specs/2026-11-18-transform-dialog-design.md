# Transform dialog: design

Date: 2026-11-18. Slice: street (`crates/cityloom_editor/src/street/`). Replaces the Transform menu of the street page.

## Why

The street page's **Transform** button opens a WAI-ARIA menu (`street::view::page::Transform`) holding every whole-street
change in one flat list: up to 3 Make pedestrian kinds, up to 14 transit measures (Atlas A1–F2, the ones
`MeasureView.can_apply && !present`) and all 5 street templates, each group headed by a paragraph of hint. Choosing an
item applies it at once.

The person opens it already knowing the one result they want ("put bus lanes in the middle"). Today they cannot find
it: the list has no hierarchy (headings weigh no more than items, the hints bury the items, 20+ entries in one column),
and they cannot see what a choice will do before it is done.

Success: the person reaches the change they have in mind in a few steps, sees the street before and after and what
changes, and only then commits.

## Goals

1. A clear hierarchy: group, then (for transit) family, then the choice.
2. A preview of the choice the person points at or moves to: the street now and after, drawn the same way, and what
   changes, in words.
3. Nothing changes until the person says so (Apply).

## Non-goals

- New transformations. The three groups and their items, and the rules for which are offered, stay as they are.
- Changing what any transformation does (`Editor::make_pedestrian`, `Editor::apply_measure`, `Editor::apply_template`).
- Previewing on the page's own drawing (option A of the brainstorm; rejected in favour of before/after in the dialog).
- Search or filtering inside the dialog.

## Anti-goals (a failure even if it works)

- The preview differs from what Apply does. The preview must come from running the same edit, not from an
  approximation.
- The preview touches the real street: its history, its undo stack, the city it is bound to or the sync.
- Losing what the menu has today: keyboard use, the screen reader's announcements, both languages, one undoable step
  per transformation, the reasons a template does not fit.

## What the person sees

Transform opens a modal dialog titled "Transform this street" (wording to settle in both `.ftl` files) with three
columns and a footer:

1. **Groups** (left): Make pedestrian, Transit priority, Street templates, each with how many choices it has (templates:
   how many of the five fit, e.g. "4 of 5"). A group with nothing in it is left out, as today. The first group with
   something in it is selected when the dialog opens.
2. **Choices** (middle): the selected group's choices.
   - Make pedestrian: the three kinds, with their icons, as today.
   - Transit priority: grouped under one heading per Atlas family, by the code's letter, only families with a choice
     listed: A Transit streets, B Centre-running, C Edge-running, D Offset, E Curb-adjacent, F Contraflow (the 14
     codes the street page offers, A1–F2, are all in these six families; queue-jumps and bus gates are the junction
     page's) (the Atlas's
     own family names are the reference; headings are new messages in `street/i18n/{en,fr}.ftl`). Each choice shows
     its code and name.
   - Street templates: all five; one that does not fit stays listed, struck through, with its reason under it (the
     reason `TemplateRefusal::said` gives today).
3. **Preview** (right), for the choice that is current (pointed at with the mouse, or moved to with the keyboard):
   - its name (and code, for a measure);
   - **Now** and **After**, the street's 2.5D drawn as a pair at one scale, so the same width looks the same in both;
   - **What changes**: the widths each use gained or lost and the design checks that began to pass or fail, the same
     comparison the page already makes after an edit;
   - for a template that does not fit, or any choice the edit refuses: no After, the reason instead;
   - before anything is current: a short line saying to choose a transformation.
4. **Footer**: Cancel and Apply. Apply is disabled while nothing is chosen or the current choice does not fit.

Pointing at a choice previews it; clicking it (or Enter/Space) chooses it, and the preview stays on the chosen one
while the mouse moves elsewhere. Apply applies the chosen one; double-click or Enter on a chosen item applies too
(builder's call, see below).

After Apply: the dialog closes, the change is one undoable step, it is announced as today (the edit's label, the
template report), and the focus goes to the drawing (`#wrap`). Cancel, Escape or the close button: the dialog closes,
nothing changes, the focus returns to the Transform button.

The Transform button stays where it is (`#transform-slot`) and is shown whenever the dialog has something to offer
(always, since the templates are always listed).

## How (decided)

- **Same edits, tried on a clone.** `Editor` gets `#[derive(Clone)]` (every field is already `Clone`-able: `State`
  is). The preview clones the real editor, so the clone keeps everything the edit reads (the selected zone, the
  region, the time shown, the zone frame and the ends; the School template needs the frame to split out its zone and
  place its raised table), runs the very edit Apply would run, and reads the clone's `View`. Building a copy with
  `Editor::snapshot` + `Editor::from_street` was considered and rejected: it drops `frame`, `ends` and `time_min` and
  makes the current street the copy's "today", so School would preview differently from Apply. The real editor is never
  touched until Apply. *(Decided after the builder check; my call.)*
- **View-model, not view, builds the preview.** `StreetVm` gets `preview(choice)` returning data: the Before and
  After `View`s, the `Impact` (`street::vm::impact::{Input::of, diff}`, with the area findings built from the clone's
  snapshot exactly as `StreetVm::findings` does for the page, so area checks are included as in the page's own summary),
  the template report where the choice is a template, or the refusal. The view draws it; the view-model never calls
  `street::view::drawing`. Tested natively against the port fakes: the preview's After view and impact equal those
  after applying the same choice for real on the same street, including School on a street with a frame and a street
  shown at a time other than noon.
- **Drawing: the page's own 2.5D, at one scale, following the theme.** The pair is drawn with the page's renderer
  (`street::view::iso::iso_svg`, CSS classes and `light-dark()` tokens), extended so Before and After can share one
  projection scale (the smaller of the two that fit, as `shared::flat::flatten_pair` does for the PDF), with tests in
  `street/view/iso`. The report's `drawing::figures` / `Figure` path is not used: it has no SVG output in the editor
  and is light-theme only. *(My call: a light-only drawing on a dark page would look broken and need its own contrast
  checks.)*
- **What changes, in words:** the impact (widths gained or lost, class change, checks that began to pass or fail),
  said with the existing impact wording; for a template, also the lines of its report (`text::template_report`: for
  School, the school zone, the speed limit and the raised table, applied or skipped and why), since those are most
  of what School does. *(My call.)*
- **An offered choice that the edit refuses** (a measure whose `can_apply` is true but `apply_measure` returns false,
  or a pedestrian kind likewise): it stays listed; its preview shows no After, says it cannot be applied (with the
  reason where the model gives one), and Apply is disabled, the same as a template that does not fit. *(My call.)*
- **A choice** is one type naming any of the three (pedestrian kind, measure code, template), so preview and apply
  share one path.
- **Accessibility:** a WAI-ARIA modal dialog (focus kept inside, Escape closes, focus returns to the button); the
  group and choice columns are listboxes (or a tablist and a listbox, builder's call) moved with the arrows, Home and
  End; the preview region is labelled and its "What changes" text is what a screen reader reads (the drawing has its
  `alt`, `drawing::alt`). New colour pairs go through `shared/contrast.rs`.
- **Translation:** every new string in both `street/i18n/en.ftl` and `fr.ftl`, whole sentences, fr-CA per the
  glossary; the i18n guard's lists updated for new ids and classes.
- Tests first, as everywhere: view-model tests for which choices and families are offered, the preview, Apply and
  Cancel; component tests in `street/view/tests.rs` for the dialog's markup and roles; the browser specs that drive the
  Transform menu today updated to the dialog: `e2e/tests/{street,templates,impact,map,junction,network}.spec.ts`.
- `street/CLAUDE.md`'s "Pedestrian streets, Transform menu, templates" section rewritten for the dialog.

## Left to the builder

- Whether the dialog is a native `<dialog>` (`showModal`) or a `role="dialog"` element; whether its markup lives in
  `web/street.html` or is all made by the component.
- The dialog's size and how it fits a narrow window (columns stacking is acceptable).
- Whether double-click / Enter on an already chosen item also applies.
- Whether previews are computed on demand or cached per choice while the dialog is open (the street does not change
  while it is open).
- Exact wording of new messages, within the conventions.
- Version bump: minor (a feature).
