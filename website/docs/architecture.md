---
id: architecture
title: Architecture
sidebar_position: 3
---

# Architecture

## Vertical slices

The code is organised by feature, one folder per slice, not by layer. New code goes in the slice that owns the feature.

| Slice          | What it holds                                                                            |
| -------------- | ---------------------------------------------------------------------------------------- |
| `street/`      | The street editor.                                                                       |
| `junction/`    | The junction editor.                                                                     |
| `map/`         | The city map over a MapLibre basemap, and the Area rules panel.                          |
| `shell/`       | The markup the pages share: menu, language switch, shortcuts.                            |
| `city/`        | The city: the layout, its storage, its projection onto the earth and the edits document. |
| `place/`       | Where a city comes from: place search, and fetching and importing an area's roads.       |
| `restriction/` | The rules declared over an area of the map.                                              |
| `shared/`      | The kernel: catalogue, units, ports, translations. It must not depend on any slice.      |

A slice may use another slice's `model`, `city::store`, `city::binding`, `city::projection`, `restriction::store`,
`shell::Target` and `shared`. It must never reach into another slice's view-model, views or text.

## MVVM inside a slice

Each editing slice holds:

- `model.rs`: the editing rules. No DOM, no signals, no I/O. An edit returns a `bool`, and a refusal carries a reason.
- `vm.rs`: the view-model. It exposes presentation-ready reactive getters and takes commands.
- `text.rs`: the words said to a screen reader.
- The Leptos views, which bind to a view-model and forward events. They decide nothing.
- `tests.rs`.

Drawing is done by pure functions that return SVG strings, set with `inner_html`. Pointer and keyboard logic are pure
state machines with tests. The map page is the exception: it draws over a MapLibre basemap and no SVG.

## Ports

A view-model reaches the browser only through ports, defined in `shared/ports.rs` with fakes for tests:
`Announcer`, `Storage`, `Scheduler`, `Mapper`, `Fetcher` (HTTP), `Importer` (the OpenStreetMap reader) and
`Navigator`. `shared/platform.rs` is their browser side. This is how view-models are tested natively, against
`test_ports()`, without a browser.

## Translation

The model never holds wording. It says things as `Said` data (a message key and arguments) that the current language
renders, so a history follows a language switch. Each slice that owns messages has `i18n/en.ftl` and `i18n/fr.ftl`
(Fluent), and a test fails when the two differ. The French terms are in `docs/glossary-fr-ca.md`.

## WebAssembly modules

The workspace has three crates besides the editor itself:

- `cityloom_editor` (the root crate) is the editor, built to WebAssembly and driven by `wasm-bindgen`.
- `osm_network` is the plain network type (serde only).
- `osm_import` runs [osm2streets](https://github.com/a-b-street/osm2streets) and fills the network. It is built as its
  own WebAssembly module, `web/pkg/osm_import*`, which `web/city.js` loads when a place is first opened. This keeps
  osm2streets out of the editor's module.

`city/import.rs` turns a network into the city's layout: roads become streets, and a meeting of three to five arms that
the junction editor can draw becomes a junction.

## JavaScript

Logic belongs in Rust. The page scripts in `web/` (`app.js`, `junction.js`, `map.js`) only load the module, call
`mount_*` and mount the shell. The one exception is `web/basemap.js`, the adapter that owns the MapLibre map.

## Persistence

A page opened from the map edits one place of the city. Its edits are written back through `CityStore` behind a 250 ms
debounce and flushed when the page is hidden. The edits of every area live in one document, `cityloom-edits`, whose
records are keyed by what the place was made from (OpenStreetMap node and way ids), so an edit follows its place into
another area that holds it.
