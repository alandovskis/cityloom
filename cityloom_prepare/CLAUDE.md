# `cityloom_prepare`

Loaded when working in this folder; the root `CLAUDE.md` has the project-wide rules.

- Prepares an area's data and defines the `.citymodel` archive. Workspace crate, not built to wasm (the wasm build excludes it and `cityloom_server`).
- Library `archive` + `metro_index`, deps only `tar`, `sha2`, `thiserror`; the server uses it with default features off. Feature `cli` (default) adds `ureq`, `clap`, `serde_json`, `rlimit`, `ctrlc` for the binary.
- **Archive**: plain ustar tar, deterministic (mtime 0, owner 0, mode 0644, fixed member order): `cityloom.idx` first, then `roads/<x>_<y>.osm.pbf`, `basemap/<name>.pmtiles`.
- **Index**: binary; magic `CLOOMIDX`, version 1, flags 0. Header (area name, creation time, source name + replication time, grid `lon0`, `lat0`, `dlon`, `dlat` as in `index.json` plus `nx`, `ny`), entries (kind road/basemap, `x`, `y`, size, SHA-256, path), SHA-256 trailer over all before it. Max 1 MiB; area name lower-case letters, digits, hyphens, max 64.
- `metro_index` is the one writer of `index.json` (`index_json`), for `tiles` and the server's unpack, so bytes match and keep the shape the app reads (`place/tiles.rs`). Checksums: first 16 hex digits of an entry's SHA-256 (`short_checksum`). `nx`/`ny` are in the archive, not `index.json`.
- Binary modules: `progress` (see `just prepare`: pure frame function, ANSI renderer, plain mode, logs and tails, Ctrl-C restores cursor), `download` (resumable), `tiles` (grid, cut with osmium), `basemap` (Planetiler, jar in `.tools`), `prepare` (extract, tiles, basemap, archive), `commands` (`clap`, `pack`).
- osmium and Planetiler stay external. Commands: `prepare`, `tiles`, `basemap`, `pack`.
