# Evidence: merge connection junctions (HEAD 9d1bd128)

All files are in ./
Captured at 9d1bd128 unless a step says 533ff5a6 (the later commits change the shell view, app.css and shell.spec.ts only). Test-output .txt files keep only the `test ...` lines and the non-empty `test result:` lines. Raw logs are `raw-*`.

1. Importer merge unit tests, `importer::blocks`: two pieces become one road of two zones, chain of three, reversed piece, equal neighbours one zone, ends not touching keep both points, name change / type change / node not passed through (signalled or signed) / three road ends / ring / block back to its own end left alone, priority renamed once or twice, twenty-thousand-roads timing; 14 passed — 01-importer-merge.txt
2. Whole importer integration suite `cargo test -p citymodel_builder --test importer` (real_city, clip_agrees, import, all): 56 passed, 0 failed. Includes the block-split tests (one road, a zone per piece; stop sign, signals and unnamed alley keep two roads) and `streets_of_two_names_meeting_end_to_end_share_a_corridor` (name change) — 02-importer-integration-suite.txt
   Thresholds that moved on this branch (`git diff origin/main -- crates/citymodel_builder/tests`, same lines against the merge base 6ee5bf90): roads in the default area, `> 150` became `> 140` in two places (`tile.network.roads.len()` and `net.roads.len()`), with the comment "150 since the pieces of a block are one road (153 before)" (commit 47ce897e, "Merge the pieces of a block ... into one road with a zone per piece (importer 11)"). Junction uids of the kept-edits test renumbered: Kreuzberg `[12, 14, 28, 37, 54, 97]` became `[11, 12, 23, 31, 47, 75]`; Plateau `[... 74, 76, 96, 97, 99, 100]` became `[... 74, 76, 95, 96, 98, 99]`; reason in the source: "Renumbered when the pieces of a block became one road, importer 11: the nodes between them went, and the junctions are the same ones, at the same places with the same arms." Other test edits stamp records or the kept network with the importer version. No other number moved.
3. `IMPORTER_VERSION` is 11 (grep) and `cut` keeps zones (`a_zoned_road_cut_by_a_frame_keeps_each_zone_where_it_lies`, `clip_to_carries_each_zone_to_the_stretch_of_the_piece_it_covered`); 21 passed — 03-importer-version-and-cut.txt
4. Editor reads zones, `city::import`: a two-zone road opens as one street of two zones, one-zone unchanged, unchanged zoned street not saved, cut-end, short-zone and unsound-zone cases; 32 passed — 04-editor-zones.txt
5. Discard (stale rule, discard at page open, version floor, sync tombstoning, archive stale-only outcome, `Refused::Stale`, i18n parity): relevant lines of seven runs (city::edits 62, city::store 53, sync::tests 54, archive::tests 29, timeline::tests 19, shell::tests 31, i18n 38 passed) — 05-discard.txt (unfiltered: raw-05-discard-full.txt)
6. Stale records are not applied, `city::model` (112 passed), includes `a_stale_street_record_is_not_applied` and `a_stale_junction_record_is_not_applied` — 09-stale-city-model.txt
7. Stale records in the sync, `sync::local` (23 passed), includes `a_stale_record_is_neither_listed_nor_read` and `a_stale_body_is_not_written` — 10-stale-sync-local.txt
8. Restore of stale work, `timeline::work` (12 passed): `a_milestone_holding_stale_work_cannot_be_restored`, so a restore brings back no stale record; the refusal wording in both languages (`timeline::tests::a_stale_milestone_has_its_own_refusal_in_both_languages`) and `archive::tests::a_stale_record_in_an_archive_is_not_imported` are in 12b — 11-stale-timeline-work.txt, 12b-archive-restore-stale.txt
9. Store stamping, `city::store::tests::a_record_written*` (3 passed), with the quoted assertions: `assert_eq!(record["import"], serde_json::json!(10), "stamped with the importer of the network it was made on")` (store.rs:486), `assert_eq!(record["import"], serde_json::json!(11))` (:493), `assert!(record.get("import").is_none(), "{checksum:?}: no version to stamp")` (:501) — 12-write-stamps-import.txt
10. UI tier in full at 9d1bd128 (`npx playwright test --project=ui`, the command `just e2e` runs for that tier, after `just build server e2e-fixtures`): 505 passed, 0 failed, 0 flaky. `restrictions.spec.ts:370` passed, so no comparison run on a fresh checkout of origin/main was needed — 15-e2e-ui-tier.txt (full log raw-15-e2e-ui.log). Earlier run at 533ff5a6: 504 passed, 1 failed (`restrictions.spec.ts:370`, `#area-save` not found; 2 of 3 passed when rerun alone, so intermittent) — 13-e2e-ui-tier.txt, 14-ui-restrictions-first-failure.txt, 14-ui-restrictions-rerun.txt
11. Integration tier, `npx playwright test --project=integration`, captured at 533ff5a6: 25 passed — 13b-e2e-integration-tier.txt
12. E2E tier, `scripts/e2e-archive.sh`, captured at 533ff5a6: 129 passed — 13c-e2e-archive-tier.txt
    Spec expectations changed on this branch (`git diff --stat 6ee5bf90 HEAD -- e2e/tests`; against origin/main itself `read-only.spec.ts` shows as deleted, because main has moved on): calming, curb, edits, fixtures, green, map, shell, stops, timeline, world. Edits-document version expectations (5, 7, 8, 9, 11, 12, 14, 15 become `EDITS_VERSION`, 17); `world.spec.ts` `const IMPORTER = "+i10"` became `"+i11"`; the map spec's tick counts `toBe(1)` become `toBe(MERGED_BOUNDARIES + 1)` and `toHaveLength(MERGED_BOUNDARIES)`, with `const MERGED_BOUNDARIES = 3;`; shell.spec gains the cleared-edits notice tests (now also checking the notice does not cover `.stage-head`). No fixture count changed.
13. Screenshot, merged street Rue Saint-Dominique with Zone 1 selected (0.0 to 14.9 m, "The street's section"): cross-section strip reads Sidewalk, Driving lane, Sidewalk — 07-merged-street-zone-1-selected.png (strip only: 07-merged-street-zone-strip-1-selected.png)
14. Screenshot, same street with Zone 2 selected (14.9 to 98.6 m, "Its own section"): cross-section strip reads Sidewalk, Driving lane, Driving lane, Sidewalk, so the section differs from zone 1 — 07-merged-street-zone-2-selected.png (strip only: 07-merged-street-zone-strip-2-selected.png)
15. Screenshots, cleared-edits notice on the map page, English, 1440x900 and 1280x800 (re-shot at 9d1bd128; unchanged place, floating below the bar) — 08-notice-map-desktop-en.png, 08-notice-map-laptop-en.png
16. Screenshots, notice on the street page, English, 1440x900 and 1280x800, re-shot at 9d1bd128: the notice is now the first thing in the editor column, above the title card, which is fully visible (box x 34, y 102, 992 and 832 px wide) — 08-notice-street-desktop-en.png, 08-notice-street-laptop-en.png
17. Screenshots, notice on the intersection page, English, 1440x900 and 1280x800, re-shot at 9d1bd128: above the junction title in the middle column, between the side panels — 08-notice-intersection-desktop-en.png, 08-notice-intersection-laptop-en.png
18. Screenshot, timeline at 1440x900 (notice in the flow above the card, not over it) — 08-notice-timeline-desktop-en.png
19. Screenshots, timeline notice at phone width (390x844), English and fr-CA — 08-notice-timeline-phone-en.png, 08-notice-timeline-phone-fr-ca.png
20. Screenshot, timeline notice in fr-CA at 1440x900 (La carte a été reconstruite avec une rue par îlot ..., button Fermer) — 08-notice-timeline-desktop-fr-ca.png

Notice shots: each page opened, `cityloom-edits@first` seeded with an unstamped street record as shell.spec.ts does, page reloaded.
The title-card overlap seen at 533ff5a6 on the map, street and intersection pages is gone on the street and intersection pages at 9d1bd128 (steps 16 and 17); on the map page the notice still floats at the top left of the map, as designed.

## Commands
```
cd <worktree>
export CARGO_INCREMENTAL=0
cargo test -p citymodel_builder importer::blocks
cargo test -p citymodel_builder --test importer
cargo test -p cityloom_model cut ; grep -rn "IMPORTER_VERSION: u32" crates/cityloom_model/src
cargo test -p cityloom_editor city::import
for f in city::edits city::store sync::tests archive::tests timeline::tests shell::tests i18n; do cargo test -p cityloom_editor $f; done
cargo test -p cityloom_editor city::model ; ... sync::local ; ... timeline::work ; ... city::store::tests::a_record_written
cargo test -p cityloom_editor archive::tests::a_stale_record_in_an_archive_is_not_imported
cargo test -p cityloom_editor timeline::tests::a_stale_milestone_has_its_own_refusal
export E2E_PORT=8291 DATABASE_URL=postgres://postgres:postgres@127.0.0.1:55432/cityloom
just build server e2e-fixtures
(cd e2e && npx playwright test --project=ui)   # at 9d1bd128; at 533ff5a6 `just e2e` ran this first, 1 failed, and stopped
(cd e2e && npx playwright test --project=ui tests/restrictions.spec.ts --repeat-each=3 -g "closing on the first corner with the details collapsed")
(cd e2e && npx playwright test --project=integration)
scripts/e2e-archive.sh
just serve &   # then, from the evidence directory: node shots2.mjs (throwaway Playwright script)
kill <pid listening on 8291>
```
