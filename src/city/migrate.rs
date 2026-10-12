//! Moves what older versions kept per area (`cityloom-city:<area key>`, edits by position) into the one edits
//! document, under the keys of the places they were made on. Done once; the old keys are left as they are.

use std::collections::BTreeMap;

use osm_network::Network;

use crate::city::edits::{EDITS_KEY, Edits};
use crate::city::model::{City, Layout};
use crate::city::store::{CITY_KEY, NETWORK_KEY};
use crate::shared::ports::Storage;

/// Builds the edits document from the per-area saves unless it exists, and keeps it (an empty one when there
/// is nothing to move, which marks the migration done). Where two areas hold an edit of the same place, the
/// one whose area's centre is nearest the place wins, and of two equally near the smaller area key; an area
/// whose network or save is missing or unusable is skipped.
pub fn migrate(storage: &dyn Storage) {
    if storage.recall(EDITS_KEY).is_some() {
        return;
    }
    let prefix = format!("{CITY_KEY}:");
    // sorted, so that of two areas the same distance from a place the smaller key is met first and kept
    let mut areas: Vec<String> = storage.keys().into_iter().filter_map(|k| k.strip_prefix(&prefix).map(str::to_string)).collect();
    areas.sort();
    let mut edits = Edits::new();
    let mut nearest_street: BTreeMap<String, f64> = BTreeMap::new();
    let mut nearest_junction: BTreeMap<String, f64> = BTreeMap::new();
    for area in areas {
        let Some(json) = storage.recall(&format!("{prefix}{area}")) else { continue };
        let Some(network) = storage.recall(&format!("{NETWORK_KEY}:{area}")).and_then(|n| serde_json::from_str::<Network>(&n).ok()) else {
            continue;
        };
        let Some(city) = City::load_old_checked(Layout::from_network(&network, ""), &json) else { continue };
        for (distance, key, record) in city.edited_streets() {
            if nearest_street.get(&key).is_none_or(|&best| distance < best) {
                nearest_street.insert(key.clone(), distance);
                edits.streets.insert(key, record);
            }
        }
        for (distance, key, record) in city.edited_junctions() {
            if nearest_junction.get(&key).is_none_or(|&best| distance < best) {
                nearest_junction.insert(key.clone(), distance);
                edits.junctions.insert(key, record);
            }
        }
    }
    // another tab may have made the document while the old saves were read: its edits are newer than any of them
    if storage.recall(EDITS_KEY).is_none() {
        storage.remember(EDITS_KEY, &edits.to_json());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::city::edits::{self, Document};
    use crate::city::model::At;
    use crate::shared::ports::{MemoryStorage, Storage};
    use osm_network::{Control, Lane, LaneKind, Node, Road, Way};

    /// A T of three roads meeting at node 1, around the centre of an area box (400 m, 400 m), `dx_m` metres
    /// east of where `network_with_ids` has it, so that the same OSM ids lie elsewhere in the area box.
    fn network_with_ids_shifted(dx_m: f64) -> Network {
        let node = |id: u32, x_m: f64, y_m: f64, junction| Node {
            id,
            osm_nodes: vec![id as i64 * 10],
            osm_versions: Default::default(),
            x_m: x_m + dx_m,
            y_m,
            junction,
            control: Control::Signs,
        };
        let lane = |kind, way| Lane { kind, way, width_m: 3.0, hours: None };
        let road = |id: u32, to: u32, points: Vec<(f64, f64)>| Road {
            id,
            osm_ways: vec![id as i64 * 100],
            osm_versions: Default::default(),
            name: Some(format!("Street {id}")),
            highway: "residential".into(),
            from: 1,
            to,
            lanes: vec![
                lane(LaneKind::Sidewalk, Way::Forward),
                lane(LaneKind::Driving, Way::Backward),
                lane(LaneKind::Driving, Way::Forward),
                lane(LaneKind::Sidewalk, Way::Forward),
            ],
            points: points.into_iter().map(|(x, y)| (x + dx_m, y)).collect(),
        };
        Network {
            left_hand: false,
            nodes: vec![node(1, 400.0, 400.0, true), node(2, 400.0, 500.0, false), node(3, 500.0, 400.0, false), node(4, 300.0, 400.0, false)],
            roads: vec![
                road(1, 2, vec![(400.0, 400.0), (400.0, 500.0)]),
                road(2, 3, vec![(400.0, 400.0), (500.0, 400.0)]),
                road(3, 4, vec![(400.0, 400.0), (300.0, 400.0)]),
            ],
        }
    }

    fn network_with_ids() -> Network {
        network_with_ids_shifted(0.0)
    }

    /// Stores an area as an old version kept it: its network, and a city save with `edit` applied.
    fn keep_area(storage: &MemoryStorage, area_key: &str, network: &Network, edit: impl FnOnce(&mut City)) {
        storage.remember(&format!("{NETWORK_KEY}:{area_key}"), &serde_json::to_string(network).unwrap());
        let mut city = City::on(Layout::from_network(network, ""));
        edit(&mut city);
        storage.remember(&format!("cityloom-city:{area_key}"), &city.legacy_save());
    }

    fn widen_first_street(c: &mut City, mm: i32) {
        let mut s = c.street_editor(1, 0).unwrap().snapshot();
        s.segments[0].width_mm += mm;
        s.segments[1].width_mm -= mm; // class and total width stay
        assert!(c.keep_street(1, s));
    }

    fn migrated(storage: &MemoryStorage) -> Edits {
        migrate(storage);
        let Document::Current(e) = edits::parse(storage.recall(EDITS_KEY).as_deref()) else { panic!("no document") };
        e
    }

    fn first_street_width(e: &Edits) -> i32 {
        e.streets.values().next().unwrap().street.segments[0].width_mm
    }

    fn today_width() -> i32 {
        City::on(Layout::from_network(&network_with_ids(), "")).street_editor(1, 0).unwrap().snapshot().segments[0].width_mm
    }

    #[test]
    fn an_areas_edits_are_moved_into_the_edits_document() {
        let storage = MemoryStorage::default();
        keep_area(&storage, "45.5000,-73.5000", &network_with_ids(), |c| widen_first_street(c, 100));
        let old = storage.recall("cityloom-city:45.5000,-73.5000");
        let e = migrated(&storage);
        assert_eq!(e.streets.len(), 1);
        assert!(e.streets.keys().next().unwrap().starts_with('w'), "{:?}", e.streets.keys().collect::<Vec<_>>());
        assert_eq!(first_street_width(&e) - today_width(), 100);
        assert_eq!(storage.recall("cityloom-city:45.5000,-73.5000"), old, "the old key is left as it was");
    }

    #[test]
    fn only_what_was_changed_is_moved() {
        let storage = MemoryStorage::default();
        keep_area(&storage, "45.5000,-73.5000", &network_with_ids(), |c| widen_first_street(c, 100));
        let e = migrated(&storage);
        assert_eq!(e.streets.len(), 1, "the old save held every street; only the edited one is moved");
        assert!(e.junctions.is_empty());
    }

    #[test]
    fn an_edited_junction_is_moved_with_the_keys_of_its_streets() {
        let storage = MemoryStorage::default();
        keep_area(&storage, "45.5000,-73.5000", &network_with_ids(), |c| {
            let mut j = c.junction_editor(1, 0).unwrap();
            let arm = j.current().arms[0].uid;
            assert!(j.set_corner(arm, 7_000));
            assert!(c.keep_junction(1, j.snapshot()));
        });
        let e = migrated(&storage);
        assert!(e.streets.is_empty());
        let record = e.junctions.values().next().expect("the junction is moved");
        assert_eq!(e.junctions.len(), 1);
        assert_eq!(record.arm_streets.len(), 3);
        assert!(record.arm_streets.values().all(|s| s.key.as_ref().is_some_and(|k| k.starts_with('w')) && !s.ways.is_empty()));
    }

    #[test]
    fn the_same_place_edited_in_two_areas_takes_the_edit_of_the_area_whose_centre_is_nearest() {
        let storage = MemoryStorage::default();
        // the same OSM ids, sitting at different distances from the centres of their area boxes
        let (near, far) = (network_with_ids_shifted(0.0), network_with_ids_shifted(250.0));
        let d = |n: &Network| Layout::from_network(n, "").centre_distance_m(At::Edge(0));
        assert!(d(&near) < d(&far), "the fixtures must differ: {} vs {}", d(&near), d(&far));
        keep_area(&storage, "45.5100,-73.5000", &far, |c| widen_first_street(c, 300));
        keep_area(&storage, "45.5000,-73.5000", &near, |c| widen_first_street(c, 100));
        assert_eq!(first_street_width(&migrated(&storage)) - today_width(), 100, "the nearer area's edit wins");
    }

    #[test]
    fn a_tie_goes_to_the_smaller_area_key() {
        let storage = MemoryStorage::default();
        keep_area(&storage, "45.6000,-73.5000", &network_with_ids(), |c| widen_first_street(c, 300));
        keep_area(&storage, "45.5000,-73.5000", &network_with_ids(), |c| widen_first_street(c, 100));
        assert_eq!(first_street_width(&migrated(&storage)) - today_width(), 100, "the same distance: the smaller key");
    }

    #[test]
    fn an_area_without_its_network_or_with_a_damaged_save_is_skipped_and_the_others_are_moved() {
        let storage = MemoryStorage::default();
        storage.remember("cityloom-city:1.0000,1.0000", "{not json"); // no network at all
        storage.remember(&format!("{NETWORK_KEY}:2.0000,2.0000"), &serde_json::to_string(&network_with_ids()).unwrap());
        storage.remember("cityloom-city:2.0000,2.0000", r#"{"version":99,"streets":{},"junctions":{}}"#); // a foreign version
        keep_area(&storage, "45.5000,-73.5000", &network_with_ids(), |c| widen_first_street(c, 100));
        assert_eq!(migrated(&storage).streets.len(), 1);
    }

    #[test]
    fn migrating_twice_changes_nothing_and_a_removed_document_is_made_again_from_the_old_keys() {
        let storage = MemoryStorage::default();
        keep_area(&storage, "45.5000,-73.5000", &network_with_ids(), |c| widen_first_street(c, 100));
        let first = migrated(&storage);
        assert_eq!(migrated(&storage), first);
        storage.forget(EDITS_KEY);
        assert_eq!(migrated(&storage), first);
    }

    #[test]
    fn an_existing_document_is_not_touched() {
        let storage = MemoryStorage::default();
        keep_area(&storage, "45.5000,-73.5000", &network_with_ids(), |c| widen_first_street(c, 100));
        storage.remember(EDITS_KEY, "not a document");
        migrate(&storage);
        assert_eq!(storage.recall(EDITS_KEY).as_deref(), Some("not a document"));
    }

    #[test]
    fn a_profile_with_nothing_to_migrate_gets_an_empty_document() {
        let storage = MemoryStorage::default();
        let e = migrated(&storage);
        assert!(e.streets.is_empty() && e.junctions.is_empty());
        assert!(storage.recall(EDITS_KEY).is_some(), "the document marks the migration as done");
    }

    /// Storage in which another tab keeps an edits document while the migration is reading the old saves.
    struct Racing {
        inner: MemoryStorage,
        theirs: String,
    }

    impl Storage for Racing {
        fn recall(&self, key: &str) -> Option<String> {
            self.inner.recall(key)
        }
        fn remember(&self, key: &str, value: &str) -> bool {
            self.inner.remember(key, value)
        }
        fn keys(&self) -> Vec<String> {
            self.inner.remember(EDITS_KEY, &self.theirs);
            self.inner.keys()
        }
        fn first_time(&self, what: &str) -> bool {
            self.inner.first_time(what)
        }
    }

    #[test]
    fn a_document_another_tab_kept_while_the_old_saves_were_read_is_not_written_over() {
        let racing = Racing { inner: MemoryStorage::default(), theirs: "theirs".into() };
        keep_area(&racing.inner, "45.5000,-73.5000", &network_with_ids(), |c| widen_first_street(c, 100));
        migrate(&racing);
        assert_eq!(racing.inner.recall(EDITS_KEY).as_deref(), Some("theirs"));
    }

    fn corner_of_junction(c: &mut City, mm: i32) {
        let mut j = c.junction_editor(1, 0).unwrap();
        let arm = j.current().arms[0].uid;
        assert!(j.set_corner(arm, mm));
        assert!(c.keep_junction(1, j.snapshot()));
    }

    #[test]
    fn the_same_junction_edited_in_two_areas_takes_the_edit_of_the_area_whose_centre_is_nearest() {
        let storage = MemoryStorage::default();
        let (near, far) = (network_with_ids_shifted(0.0), network_with_ids_shifted(250.0));
        let d = |n: &Network| Layout::from_network(n, "").centre_distance_m(At::Node(0));
        assert!(d(&near) < d(&far));
        keep_area(&storage, "45.5100,-73.5000", &far, |c| corner_of_junction(c, 8_000));
        keep_area(&storage, "45.5000,-73.5000", &near, |c| corner_of_junction(c, 6_000));
        let e = migrated(&storage);
        assert_eq!(e.junctions.len(), 1);
        assert_eq!(e.junctions.values().next().unwrap().state.arms[0].corner_mm, 6_000, "the nearer area's edit wins");
    }

    #[test]
    fn the_bare_sample_city_key_is_not_migrated() {
        let storage = MemoryStorage::default();
        storage.remember("cityloom-city", &City::new().legacy_save());
        assert!(migrated(&storage).streets.is_empty());
    }
}
