//! `cut` — clip assembly for one request: read exactly the touched cells
//! (BR5.1), keep ways whose extent intersects the box together with every
//! node they reference (BR5.2), de-duplicate and sort (BR5.3), and stop
//! between cells once the deadline has passed (NFR3.1.7, BR10.3).

use std::collections::HashSet;
use std::time::Instant;

use crate::decode::decode;
use crate::encode::encode;
use crate::geo::CanonicalBox;
use crate::grid::TouchedCells;
use crate::osm::Clip;
use crate::store::Store;

/// The result of assembling and encoding one request's clip.
#[derive(Debug)]
pub enum CutOutcome {
    /// The encoded extract bytes (BR5.5); ready to serve and to cache.
    Extract(Vec<u8>),
    /// The box is covered but its clip contains no way (BR4.2): not cached.
    NothingMapped,
    /// The deadline elapsed before the cut finished (BR10.3, NFR3.1.7).
    DeadlineExceeded,
}

/// Assemble the clip for `canonical_box` from exactly the cells `touched`
/// names, stopping early if `deadline` passes between cells.
pub fn cut(
    store: &Store,
    touched: &TouchedCells,
    canonical_box: &CanonicalBox,
    deadline: Instant,
) -> CutOutcome {
    let mut clip = Clip::default();
    for cell_id in touched.iter() {
        if Instant::now() >= deadline {
            return CutOutcome::DeadlineExceeded;
        }
        let Ok(Some(bytes)) = store.read_cell(cell_id) else {
            continue;
        };
        let Ok(cell_clip) = decode(&bytes) else {
            continue;
        };
        for way in &cell_clip.ways {
            let Some(extent) = cell_clip.way_extent(way) else {
                continue;
            };
            if !extent.intersects(canonical_box) {
                continue;
            }
            for node_id in &way.refs {
                if let Some(node) = cell_clip.nodes.iter().find(|n| n.id == *node_id) {
                    clip.nodes.push(node.clone());
                }
            }
            clip.ways.push(way.clone());
        }
    }
    clip.normalise();
    // Every node still referenced stays; a node that only ever belonged to
    // an excluded way (or was duplicated in) is dropped.
    let referenced: HashSet<i64> = clip
        .ways
        .iter()
        .flat_map(|w| w.refs.iter().copied())
        .collect();
    clip.nodes.retain(|n| referenced.contains(&n.id));

    if !clip.has_ways() {
        return CutOutcome::NothingMapped;
    }
    CutOutcome::Extract(encode(&clip))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::grid::GridSpec;
    use crate::osm::{Node, Way};
    use crate::store::StoreWriter;

    fn node(id: i64, lat: i64, lon: i64) -> Node {
        Node {
            id,
            lat,
            lon,
            tags: Vec::new(),
        }
    }

    fn way(id: i64, refs: &[i64]) -> Way {
        Way {
            id,
            refs: refs.to_vec(),
            tags: vec![("highway".to_string(), "residential".to_string())],
        }
    }

    fn far_ahead() -> Instant {
        Instant::now() + Duration::from_secs(60)
    }

    // BR5.2 — a way with its extent intersecting the box is kept with every
    // node it references, even nodes outside the box.
    #[test]
    fn keeps_a_way_with_every_referenced_node_even_outside_the_box() {
        let dir = tempfile::tempdir().unwrap();
        let grid = GridSpec::default_cells();
        let b = CanonicalBox::parse("-79.631,43.649,-79.629,43.651").unwrap();
        let touched = grid.touched(&b);

        // Node 2 sits well outside the requested box, but way 1 (which
        // intersects the box through node 1) still references it.
        let clip = Clip {
            nodes: vec![
                node(1, 436_500_000, -796_300_000),
                node(2, 500_000_000, -900_000_000),
            ],
            ways: vec![way(1, &[1, 2])],
        };
        let mut writer = StoreWriter::create(dir.path().join("store.bin")).unwrap();
        let mut cells = Vec::new();
        for cell_id in touched.iter() {
            let (offset, length, digest) = writer.write_cell(&encode(&clip)).unwrap();
            cells.push(crate::manifest::CellEntry {
                cell_id,
                offset,
                length,
                digest,
                way_count: 1,
                node_count: 2,
            });
        }
        writer.flush().unwrap();
        let store = Store::open(dir.path().join("store.bin"), &cells).unwrap();

        match cut(&store, &touched, &b, far_ahead()) {
            CutOutcome::Extract(bytes) => {
                let decoded = decode(&bytes).unwrap();
                assert_eq!(decoded.node_count(), 2, "node 2 travels with the way");
                assert_eq!(decoded.way_count(), 1);
            }
            other => panic!("expected an extract, got {other:?}"),
        }
    }

    // BR5.3 — a way present in two touched cells appears once, and the
    // output is sorted.
    #[test]
    fn a_way_in_two_touched_cells_appears_once_and_output_is_sorted() {
        let dir = tempfile::tempdir().unwrap();
        let grid = GridSpec::default_cells();
        let b = CanonicalBox::parse("-79.6305,43.6495,-79.6295,43.6505").unwrap(); // 4 cells
        let touched = grid.touched(&b);
        assert_eq!(touched.count(), 4);

        let clip = Clip {
            nodes: vec![
                node(2, 436_500_000, -796_300_000),
                node(1, 436_500_000, -796_300_000),
            ],
            ways: vec![way(1, &[1, 2])],
        };
        let mut writer = StoreWriter::create(dir.path().join("store.bin")).unwrap();
        let mut cells = Vec::new();
        for cell_id in touched.iter() {
            let (offset, length, digest) = writer.write_cell(&encode(&clip)).unwrap();
            cells.push(crate::manifest::CellEntry {
                cell_id,
                offset,
                length,
                digest,
                way_count: 1,
                node_count: 2,
            });
        }
        writer.flush().unwrap();
        let store = Store::open(dir.path().join("store.bin"), &cells).unwrap();

        match cut(&store, &touched, &b, far_ahead()) {
            CutOutcome::Extract(bytes) => {
                let decoded = decode(&bytes).unwrap();
                assert_eq!(decoded.way_count(), 1, "one way, not four");
                assert_eq!(
                    decoded.nodes.iter().map(|n| n.id).collect::<Vec<_>>(),
                    vec![1, 2],
                    "sorted output"
                );
            }
            other => panic!("expected an extract, got {other:?}"),
        }
    }

    // BR4.2 — a covered box with no ways yields "nothing mapped".
    #[test]
    fn a_covered_box_with_no_ways_yields_nothing_mapped() {
        let dir = tempfile::tempdir().unwrap();
        let grid = GridSpec::default_cells();
        let b = CanonicalBox::parse("-79.631,43.649,-79.629,43.651").unwrap();
        let touched = grid.touched(&b);

        let empty = Clip::default();
        let mut writer = StoreWriter::create(dir.path().join("store.bin")).unwrap();
        let mut cells = Vec::new();
        for cell_id in touched.iter() {
            let (offset, length, digest) = writer.write_cell(&encode(&empty)).unwrap();
            cells.push(crate::manifest::CellEntry {
                cell_id,
                offset,
                length,
                digest,
                way_count: 0,
                node_count: 0,
            });
        }
        writer.flush().unwrap();
        let store = Store::open(dir.path().join("store.bin"), &cells).unwrap();

        assert!(matches!(
            cut(&store, &touched, &b, far_ahead()),
            CutOutcome::NothingMapped
        ));
    }

    // NFR3.1.7 / BR10.3 — the cut checks its deadline between cells and
    // stops early.
    #[test]
    fn stops_early_once_the_deadline_has_passed() {
        let dir = tempfile::tempdir().unwrap();
        let grid = GridSpec::default_cells();
        let b = CanonicalBox::parse("0,0,0.04,0.03").unwrap(); // 12 cells
        let touched = grid.touched(&b);

        let clip = Clip {
            nodes: vec![node(1, 100, 100)],
            ways: vec![way(1, &[1])],
        };
        let mut writer = StoreWriter::create(dir.path().join("store.bin")).unwrap();
        let mut cells = Vec::new();
        for cell_id in touched.iter() {
            let (offset, length, digest) = writer.write_cell(&encode(&clip)).unwrap();
            cells.push(crate::manifest::CellEntry {
                cell_id,
                offset,
                length,
                digest,
                way_count: 1,
                node_count: 1,
            });
        }
        writer.flush().unwrap();
        let store = Store::open(dir.path().join("store.bin"), &cells).unwrap();

        let already_past = Instant::now() - Duration::from_secs(1);
        assert!(matches!(
            cut(&store, &touched, &b, already_past),
            CutOutcome::DeadlineExceeded
        ));
    }
}
