use std::fs;
use std::path::{Path, PathBuf};

use cityloom_prepare::archive::{self, Kind, PackError, PackInputs, read_index};
use cityloom_prepare::metro_index::{Grid, short_checksum};
use sha2::{Digest, Sha256};

const GRID: Grid = Grid { lon0: -74.4, lat0: 45.2, dlon: 0.0257, dlat: 0.018, nx: 51, ny: 34 };

fn tile_bytes(name: &str) -> Vec<u8> {
    format!("tile {name} ").repeat(40).into_bytes()
}

/// Files for the tiles named `x_y` and the basemaps, under `dir`; returns the road list and the basemap list.
fn inputs_in(dir: &Path, tiles: &[(u16, u16)], basemaps: &[&str]) -> (Vec<(u16, u16, PathBuf)>, Vec<PathBuf>) {
    fs::create_dir_all(dir).unwrap();
    let roads = tiles
        .iter()
        .map(|&(x, y)| {
            let path = dir.join(format!("{x}_{y}.osm.pbf"));
            fs::write(&path, tile_bytes(&format!("{x}_{y}"))).unwrap();
            (x, y, path)
        })
        .collect();
    let maps = basemaps
        .iter()
        .map(|name| {
            let path = dir.join(name);
            fs::write(&path, b"PMTiles".repeat(1000)).unwrap();
            path
        })
        .collect();
    (roads, maps)
}

fn pack_to(out: &Path, roads: &[(u16, u16, PathBuf)], maps: &[PathBuf]) -> Result<usize, PackError> {
    let inputs =
        PackInputs { area: "montreal", source: "quebec-latest.osm.pbf", source_time: 1_759_900_000, created: 1_760_000_000, grid: GRID, roads, basemaps: maps };
    archive::pack(&inputs, out, &mut |_, _| {})
}

fn members(path: &Path) -> Vec<(tar::Header, String, Vec<u8>)> {
    let mut tar = tar::Archive::new(fs::File::open(path).unwrap());
    tar.entries()
        .unwrap()
        .map(|e| {
            let mut e = e.unwrap();
            let mut data = Vec::new();
            std::io::Read::read_to_end(&mut e, &mut data).unwrap();
            (e.header().clone(), e.path().unwrap().to_str().unwrap().to_string(), data)
        })
        .collect()
}

#[test]
fn members_are_in_the_stated_order_with_string_sorted_tiles() {
    let dir = tempfile::tempdir().unwrap();
    let (roads, maps) = inputs_in(dir.path(), &[(10, 3), (0, 0), (2, 0)], &["montreal.pmtiles"]);
    let out = dir.path().join("a.cityloom");
    assert_eq!(pack_to(&out, &roads, &maps).unwrap(), 4);
    let names: Vec<String> = members(&out).into_iter().map(|m| m.1).collect();
    assert_eq!(names, ["cityloom.idx", "roads/0_0.osm.pbf", "roads/10_3.osm.pbf", "roads/2_0.osm.pbf", "basemap/montreal.pmtiles"]);

    let (roads, maps) = inputs_in(dir.path(), &[(0, 0)], &["montreal.pmtiles", "b.pmtiles"]);
    pack_to(&out, &roads, &maps).unwrap();
    let names: Vec<String> = members(&out).into_iter().map(|m| m.1).collect();
    assert_eq!(names, ["cityloom.idx", "roads/0_0.osm.pbf", "basemap/b.pmtiles", "basemap/montreal.pmtiles"]);
}

#[test]
fn headers_are_the_deterministic_ones() {
    let dir = tempfile::tempdir().unwrap();
    let (roads, maps) = inputs_in(dir.path(), &[(0, 0), (2, 0)], &["montreal.pmtiles"]);
    let out = dir.path().join("a.cityloom");
    pack_to(&out, &roads, &maps).unwrap();
    for (h, name, _) in members(&out) {
        assert_eq!(h.mtime().unwrap(), 0, "{name}");
        assert_eq!((h.uid().unwrap(), h.gid().unwrap(), h.mode().unwrap()), (0, 0, 0o644), "{name}");
        assert_eq!(h.entry_type(), tar::EntryType::Regular, "{name}");
        assert!(h.username_bytes().is_none_or(|b| b.is_empty()), "{name}");
        assert!(h.groupname_bytes().is_none_or(|b| b.is_empty()), "{name}");
    }
    let bytes = fs::read(&out).unwrap();
    assert_eq!(&bytes[257..262], b"ustar");
}

#[test]
fn the_same_inputs_give_the_same_bytes_whatever_the_file_times_or_places() {
    let dir = tempfile::tempdir().unwrap();
    let (r1, m1) = inputs_in(&dir.path().join("one"), &[(0, 0), (2, 0)], &["montreal.pmtiles"]);
    let (r2, m2) = inputs_in(&dir.path().join("two"), &[(2, 0), (0, 0)], &["montreal.pmtiles"]);
    let later = std::time::SystemTime::now() + std::time::Duration::from_secs(86_400);
    for (_, _, p) in &r2 {
        fs::File::options().write(true).open(p).unwrap().set_modified(later).unwrap();
    }
    for p in &m2 {
        fs::File::options().write(true).open(p).unwrap().set_modified(later).unwrap();
    }
    let (a, b) = (dir.path().join("a.cityloom"), dir.path().join("b.cityloom"));
    pack_to(&a, &r1, &m1).unwrap();
    pack_to(&b, &r2, &m2).unwrap();
    assert_eq!(fs::read(&a).unwrap(), fs::read(&b).unwrap());
}

#[test]
fn the_index_describes_the_members() {
    let dir = tempfile::tempdir().unwrap();
    let (roads, maps) = inputs_in(dir.path(), &[(0, 0), (2, 0), (10, 3)], &["montreal.pmtiles"]);
    let out = dir.path().join("a.cityloom");
    pack_to(&out, &roads, &maps).unwrap();
    let all = members(&out);
    let index = read_index(&all[0].2).unwrap();
    let h = &index.header;
    assert_eq!((h.area.as_str(), h.created, h.source.as_str(), h.source_time), ("montreal", 1_760_000_000, "quebec-latest.osm.pbf", 1_759_900_000));
    assert_eq!((h.grid.nx, h.grid.ny), (51, 34));
    let paths: Vec<&str> = index.entries.iter().map(|e| e.path.as_str()).collect();
    let names: Vec<&str> = all[1..].iter().map(|m| m.1.as_str()).collect();
    assert_eq!(paths, names);
    for (entry, member) in index.entries.iter().zip(&all[1..]) {
        assert_eq!(entry.size, member.2.len() as u64);
        assert_eq!(entry.sha256[..], Sha256::digest(&member.2)[..]);
    }
    let kinds: Vec<_> = index.entries.iter().map(|e| (e.kind, e.x, e.y)).collect();
    assert_eq!(kinds, [(Kind::Road, 0, 0), (Kind::Road, 10, 3), (Kind::Road, 2, 0), (Kind::Basemap, 0, 0)]);
    let full: String = Sha256::digest(tile_bytes("0_0")).iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(short_checksum(&index.entries[0].sha256), full[..16]);
}

#[test]
fn the_part_file_is_renamed_when_whole_and_removed_on_failure() {
    let dir = tempfile::tempdir().unwrap();
    let (roads, maps) = inputs_in(dir.path(), &[(0, 0)], &["montreal.pmtiles"]);
    let out = dir.path().join("a.cityloom");
    let part = dir.path().join("a.cityloom.part");
    pack_to(&out, &roads, &maps).unwrap();
    assert!(out.exists() && !part.exists());

    let out = dir.path().join("b.cityloom");
    let part = dir.path().join("b.cityloom.part");
    let mut missing = roads.clone();
    missing.push((1, 0, dir.path().join("1_0.osm.pbf")));
    assert!(matches!(pack_to(&out, &missing, &maps), Err(PackError::Io { .. })));
    assert!(!out.exists() && !part.exists());
}

#[test]
fn a_file_that_changes_size_while_it_is_packed_is_an_error_and_leaves_nothing() {
    for grow in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let (roads, maps) = inputs_in(dir.path(), &[(0, 0)], &["montreal.pmtiles"]);
        let out = dir.path().join("a.cityloom");
        let inputs = PackInputs { area: "montreal", source: "s", source_time: 1, created: 2, grid: GRID, roads: &roads, basemaps: &maps };
        let target = maps[0].clone();
        let mut changed = false;
        // the basemap changes as soon as the copy begins, after the sizes were measured and the files hashed
        let result = archive::pack(&inputs, &out, &mut |done, total| {
            if done > total / 2 && !changed {
                changed = true;
                if grow {
                    fs::write(&target, b"PMTiles".repeat(1001)).unwrap();
                } else {
                    fs::write(&target, b"PMTiles").unwrap();
                }
            }
        });
        assert!(matches!(result, Err(PackError::Io { .. })), "grow {grow}: {result:?}");
        assert!(!out.exists() && !dir.path().join("a.cityloom.part").exists());
    }
}

#[test]
fn refuses_what_the_server_would_refuse() {
    let dir = tempfile::tempdir().unwrap();
    let (roads, maps) = inputs_in(dir.path(), &[(0, 0)], &["montreal.pmtiles"]);
    let out = dir.path().join("a.cityloom");
    let with_area = |area: &str| {
        let inputs = PackInputs { area, source: "s", source_time: 1, created: 2, grid: GRID, roads: &roads, basemaps: &maps };
        archive::pack(&inputs, &out, &mut |_, _| {})
    };
    assert!(matches!(with_area("../x"), Err(PackError::Area(_))));

    let (_, bad_map) = inputs_in(&dir.path().join("m"), &[], &["Montreal.pmtiles"]);
    assert!(matches!(pack_to(&out, &roads, &bad_map), Err(PackError::Name(_))));

    let outside = vec![(51u16, 0u16, roads[0].2.clone())];
    assert!(matches!(pack_to(&out, &outside, &maps), Err(PackError::OutsideGrid(_))));

    assert!(matches!(pack_to(&out, &[], &maps), Err(PackError::Empty("no road tiles"))));
    assert!(matches!(pack_to(&out, &roads, &[]), Err(PackError::Empty("no basemap"))));
    assert!(!out.exists());
}

#[test]
fn reports_progress_up_to_the_total() {
    let dir = tempfile::tempdir().unwrap();
    let (roads, maps) = inputs_in(dir.path(), &[(0, 0), (2, 0)], &["montreal.pmtiles"]);
    let sizes: u64 = roads.iter().map(|r| fs::metadata(&r.2).unwrap().len()).sum::<u64>() + fs::metadata(&maps[0]).unwrap().len();
    let inputs = PackInputs { area: "montreal", source: "s", source_time: 1, created: 2, grid: GRID, roads: &roads, basemaps: &maps };
    let mut last = (0, 0);
    archive::pack(&inputs, &dir.path().join("a.cityloom"), &mut |done, total| last = (done, total)).unwrap();
    assert_eq!(last, (sizes * 2, sizes * 2));
}
