# Shared kernel (`src/shared/`)

Loaded when working in this folder; the root `CLAUDE.md` has the project-wide rules.

- Kernel: catalogue, units, atlas, symbols. `shared` must not depend on any slice.
- Ports (fakes in `ports.rs`): `Announcer`, `Scheduler`, `Storage` (`recall`, `remember`, `keys`), `Mapper` (`MapEvent`, `NoMapper`, `FakeMapper`; `set_restrictions`, `set_draft`, `centre`, `set_insets`, `set_editable`; `MapEvent::View` reports the open view), `Fetcher`, `ArchiveFiles`, `Importer` (OSM reader, separate wasm module), `Account`, `Navigator`.
- `platform.rs`: browser side of the ports (`BrowserMapper` wraps `web/basemap.js`). `core.rs`: `Core<M: Presents>` = model + version signal + cached view; `view()` tracked, `view_now()` untracked (use in commands). Also `keeper.rs`, `shortcut.rs`, `bind.rs`, `testing.rs` (test-only HTML helpers).
- `diagram/`: layered drawing for the street's and junction's 2.5D (the junction is to adopt it later). Plain data in ground mm (`Pt`, `Shape`, `Layer`/`LayerKind`, `Section`, `Diagram`), `Projection` trait, `render` writes SVG.
- `iso/`: shared 2.5D parts: `format` (`esc`, `f1`), `shading` (`shade_filters`; each slice keeps its own ids), `paint` (ground-plane paint in mm, projection-independent: `quads` from polyline + `Style` (`Paint`, `Pattern`, `double`), `class` for CSS; line 120 mm, double gap 120 mm, dash 3 m, gap 6 m).
