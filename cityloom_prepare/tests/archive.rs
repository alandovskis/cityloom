use std::fs;
use std::path::{Path, PathBuf};

use cityloom_prepare::archive::{self, ArchiveError, Entry, Header, Kind, PackError, PackInputs, read_index};
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

// ---- unpack ----

fn sha(data: &[u8]) -> [u8; 32] {
    Sha256::digest(data).into()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn header(area: &str) -> Header {
    Header {
        area: area.into(),
        created: 0,
        source: "quebec-latest.osm.pbf".into(),
        source_time: 0,
        grid: Grid { lon0: -73.6078, lat0: 45.5161, dlon: 0.0257, dlat: 0.018, nx: 4, ny: 3 },
    }
}

fn road(x: u16, y: u16, data: &[u8]) -> (Entry, Vec<u8>) {
    (Entry { kind: Kind::Road, x, y, size: data.len() as u64, sha256: sha(data), path: format!("roads/{x}_{y}.osm.pbf") }, data.to_vec())
}

fn basemap(data: &[u8]) -> (Entry, Vec<u8>) {
    (Entry { kind: Kind::Basemap, x: 0, y: 0, size: data.len() as u64, sha256: sha(data), path: "basemap/montreal.pmtiles".into() }, data.to_vec())
}

fn file_header(path: &str, size: u64) -> tar::Header {
    let mut h = tar::Header::new_ustar();
    h.set_path(path).unwrap();
    h.set_size(size);
    h.set_mode(0o644);
    h.set_cksum();
    h
}

fn good() -> Vec<(Entry, Vec<u8>)> {
    vec![road(0, 0, b"tile zero zero"), road(1, 2, b"tile one two, longer"), basemap(&[7u8; 5000])]
}

/// An archive of the given index bytes, then the given members, each at its own path.
fn archive_of(dir: &Path, name: &str, index: &[u8], members: Vec<(tar::Header, Vec<u8>)>) -> PathBuf {
    let mut tar = tar::Builder::new(Vec::new());
    tar.append(&file_header("cityloom.idx", index.len() as u64), index).unwrap();
    for (h, data) in members {
        tar.append(&h, &data[..]).unwrap();
    }
    let path = dir.join(name);
    fs::write(&path, tar.into_inner().unwrap()).unwrap();
    path
}

/// An archive of `pieces`: the index first (from the entries), then each piece's bytes under its entry's path.
fn write_archive(dir: &Path, name: &str, header: &Header, pieces: &[(Entry, Vec<u8>)]) -> PathBuf {
    let entries: Vec<Entry> = pieces.iter().map(|(e, _)| e.clone()).collect();
    let index = archive::write_index(header, &entries);
    let members = pieces.iter().map(|(e, d)| (file_header(&e.path, d.len() as u64), d.clone())).collect();
    archive_of(dir, name, &index, members)
}

/// Every file under `dir` with its bytes, to compare a tree before and after.
fn tree(dir: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                out.push((path.strip_prefix(base).unwrap().display().to_string(), fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

fn roots() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let (archives, data) = (tmp.path().join("areas"), tmp.path().join("data"));
    fs::create_dir_all(&archives).unwrap();
    fs::create_dir_all(&data).unwrap();
    (tmp, archives, data)
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    names
}

fn refused<T: std::fmt::Debug>(result: Result<T, ArchiveError>, text: &str) -> ArchiveError {
    let error = result.unwrap_err();
    assert!(error.to_string().contains(text), "{error}");
    error
}

fn only_areas_and_data(tmp: &Path) {
    assert_eq!(names(tmp), ["areas", "data"]);
}

#[test]
fn a_good_archive_unpacks_and_its_index_json_matches() {
    let (_tmp, archives, data) = roots();
    let h = header("montreal");
    let a = write_archive(&archives, "montreal.cityloom", &h, &good());
    let got = archive::unpack(&a, &data).unwrap();
    assert_eq!(got, archive::Unpacked { area: "montreal".into(), dir: data.join("montreal"), reused: false, tiles: 2 });
    let dir = data.join("montreal");
    assert_eq!(fs::read(dir.join("metro/0_0.osm.pbf")).unwrap(), b"tile zero zero");
    assert_eq!(fs::read(dir.join("metro/1_2.osm.pbf")).unwrap(), b"tile one two, longer");
    assert_eq!(fs::read(dir.join("basemap/montreal.pmtiles")).unwrap(), vec![7u8; 5000]);
    let index = read_index(&fs::read(&a).unwrap()[512..512 + archive::write_index(&h, &good().iter().map(|p| p.0.clone()).collect::<Vec<_>>()).len()]).unwrap();
    assert_eq!(fs::read_to_string(dir.join(".index-sha256")).unwrap(), hex(&index.trailer));
    let expected = cityloom_prepare::metro_index::index_json(
        &h.grid,
        &[("0_0".to_string(), short_checksum(&sha(b"tile zero zero"))), ("1_2".to_string(), short_checksum(&sha(b"tile one two, longer")))],
    );
    let json = fs::read_to_string(dir.join("metro/index.json")).unwrap();
    assert_eq!(json, expected);
    assert!(json.starts_with("{\"lon0\":"));
    assert_eq!(names(&data), ["montreal"]);
}

#[test]
fn an_archive_pack_writes_unpacks_to_the_same_files() {
    let (tmp, _archives, data) = roots();
    let src = tmp.path().join("src");
    let (roads, maps) = inputs_in(&src, &[(10, 3), (0, 0)], &["montreal.pmtiles"]);
    let folder = tmp.path().join("folder");
    fs::create_dir_all(&folder).unwrap();
    pack_to(&folder.join("a.cityloom"), &roads, &maps).unwrap();
    let got = archive::unpack_folder(&folder, &data).unwrap();
    assert_eq!((got.area.as_str(), got.tiles), ("montreal", 2));
    for (x, y, path) in &roads {
        assert_eq!(fs::read(data.join(format!("montreal/metro/{x}_{y}.osm.pbf"))).unwrap(), fs::read(path).unwrap());
    }
    assert_eq!(fs::read(data.join("montreal/basemap/montreal.pmtiles")).unwrap(), fs::read(&maps[0]).unwrap());
}

#[test]
fn refuses_a_tampered_tile() {
    let (tmp, archives, data) = roots();
    let mut pieces = good();
    pieces[0].1[0] ^= 1;
    let a = write_archive(&archives, "a.cityloom", &header("montreal"), &pieces);
    let e = refused(archive::unpack(&a, &data), "checksum");
    assert!(matches!(e, ArchiveError::Member { .. }));
    assert!(names(&data).is_empty());
    only_areas_and_data(tmp.path());
}

#[test]
fn refuses_a_wrong_size() {
    let (_tmp, archives, data) = roots();
    // the entry says one byte more than the member holds
    let mut pieces = good();
    pieces[0].0.size += 1;
    let a = write_archive(&archives, "a.cityloom", &header("montreal"), &pieces);
    refused(archive::unpack(&a, &data), "size");
    // the member holds one byte more than the entry says
    let mut pieces = good();
    pieces[0].1.push(b'x');
    let b = write_archive(&archives, "b.cityloom", &header("montreal"), &pieces);
    refused(archive::unpack(&b, &data), "size");
    assert!(names(&data).is_empty());
}

#[test]
fn refuses_a_missing_tile() {
    let (_tmp, archives, data) = roots();
    let pieces = good();
    let entries: Vec<Entry> = pieces.iter().map(|p| p.0.clone()).collect();
    let index = archive::write_index(&header("montreal"), &entries);
    let members = vec![
        (file_header("roads/0_0.osm.pbf", pieces[0].1.len() as u64), pieces[0].1.clone()),
        (file_header("basemap/montreal.pmtiles", pieces[2].1.len() as u64), pieces[2].1.clone()),
    ];
    let a = archive_of(&archives, "a.cityloom", &index, members);
    match archive::unpack(&a, &data).unwrap_err() {
        ArchiveError::Missing { path } => assert_eq!(path, "roads/1_2.osm.pbf"),
        other => panic!("{other}"),
    }
    assert!(names(&data).is_empty());
}

#[test]
fn refuses_an_extra_member() {
    let (_tmp, archives, data) = roots();
    let pieces = good();
    let entries: Vec<Entry> = pieces.iter().map(|p| p.0.clone()).collect();
    let index = archive::write_index(&header("montreal"), &entries);
    let mut members: Vec<_> = pieces.iter().map(|(e, d)| (file_header(&e.path, d.len() as u64), d.clone())).collect();
    members.push((file_header("roads/3_2.osm.pbf", 3), b"abc".to_vec()));
    let a = archive_of(&archives, "a.cityloom", &index, members);
    refused(archive::unpack(&a, &data), "not in the index");
    assert!(names(&data).is_empty());
}

#[test]
fn refuses_a_member_that_appears_twice() {
    let (_tmp, archives, data) = roots();
    let pieces = good();
    let entries: Vec<Entry> = pieces.iter().map(|p| p.0.clone()).collect();
    let index = archive::write_index(&header("montreal"), &entries);
    let mut members: Vec<_> = pieces.iter().map(|(e, d)| (file_header(&e.path, d.len() as u64), d.clone())).collect();
    members.insert(1, members[0].clone());
    let a = archive_of(&archives, "a.cityloom", &index, members);
    refused(archive::unpack(&a, &data), "twice");
    assert!(names(&data).is_empty());
}

#[test]
fn refuses_an_index_that_lists_a_path_twice() {
    let (_tmp, archives, data) = roots();
    let mut pieces = good();
    pieces.push(pieces[0].clone());
    let a = write_archive(&archives, "a.cityloom", &header("montreal"), &pieces);
    refused(archive::unpack(&a, &data), "twice");
    assert!(names(&data).is_empty());
}

#[test]
fn refuses_a_symlink_a_directory_and_a_hard_link() {
    for kind in [tar::EntryType::Symlink, tar::EntryType::Directory, tar::EntryType::Link] {
        let (tmp, archives, data) = roots();
        let pieces = good();
        let entries: Vec<Entry> = pieces.iter().map(|p| p.0.clone()).collect();
        let index = archive::write_index(&header("montreal"), &entries);
        let mut h = tar::Header::new_ustar();
        h.set_path("roads/0_0.osm.pbf").unwrap();
        h.set_size(0);
        h.set_mode(0o644);
        h.set_entry_type(kind);
        if kind != tar::EntryType::Directory {
            h.set_link_name("/etc/passwd").unwrap();
        }
        h.set_cksum();
        let a = archive_of(&archives, "a.cityloom", &index, vec![(h, Vec::new())]);
        refused(archive::unpack(&a, &data), "regular file");
        assert!(names(&data).is_empty());
        only_areas_and_data(tmp.path());
    }
}

#[test]
fn refuses_dot_dot_in_a_path() {
    let (tmp, archives, data) = roots();
    let pieces = good();
    let entries: Vec<Entry> = pieces.iter().map(|p| p.0.clone()).collect();
    let index = archive::write_index(&header("montreal"), &entries);
    let mut h = tar::Header::new_ustar();
    h.set_size(3);
    h.set_mode(0o644);
    let raw = b"roads/../../escape";
    h.as_old_mut().name[..raw.len()].copy_from_slice(raw);
    h.set_cksum();
    let a = archive_of(&archives, "a.cityloom", &index, vec![(h, b"abc".to_vec())]);
    let e = archive::unpack(&a, &data).unwrap_err();
    assert!(matches!(e, ArchiveError::Member { .. }), "{e}");
    assert!(!tmp.path().join("escape").exists() && !data.join("escape").exists());
    assert!(names(&data).is_empty());
}

#[test]
fn refuses_a_path_outside_the_two_patterns() {
    let (_tmp, archives, data) = roots();
    let mut pieces = good();
    pieces[0].0.path = "etc/passwd".into();
    let a = write_archive(&archives, "a.cityloom", &header("montreal"), &pieces);
    let e = archive::unpack(&a, &data).unwrap_err();
    assert!(matches!(e, ArchiveError::Member { .. }), "{e}");
    assert!(names(&data).is_empty());
}

#[test]
fn refuses_a_tile_outside_the_grid() {
    let (_tmp, archives, data) = roots();
    let pieces = vec![road(9, 0, b"far away"), basemap(b"m")];
    let a = write_archive(&archives, "a.cityloom", &header("montreal"), &pieces);
    refused(archive::unpack(&a, &data), "outside the grid");
    assert!(names(&data).is_empty());
}

#[test]
fn refuses_an_index_that_is_not_first() {
    let (_tmp, archives, data) = roots();
    let pieces = good();
    let entries: Vec<Entry> = pieces.iter().map(|p| p.0.clone()).collect();
    let index = archive::write_index(&header("montreal"), &entries);
    let mut tar = tar::Builder::new(Vec::new());
    tar.append(&file_header("roads/0_0.osm.pbf", 3), &b"abc"[..]).unwrap();
    tar.append(&file_header("cityloom.idx", index.len() as u64), &index[..]).unwrap();
    let a = archives.join("a.cityloom");
    fs::write(&a, tar.into_inner().unwrap()).unwrap();
    assert!(matches!(archive::unpack(&a, &data).unwrap_err(), ArchiveError::IndexNotFirst));
    assert!(names(&data).is_empty());
}

#[test]
fn refuses_an_index_above_one_megabyte() {
    let (_tmp, archives, data) = roots();
    let a = archive_of(&archives, "a.cityloom", &vec![0u8; (1 << 20) + 1], vec![]);
    assert!(matches!(archive::unpack(&a, &data).unwrap_err(), ArchiveError::IndexTooBig));
    assert!(names(&data).is_empty());
}

#[test]
fn refuses_a_bad_index() {
    use cityloom_prepare::archive::IndexError;
    let (_tmp, archives, data) = roots();
    let a = write_archive(&archives, "a.cityloom", &header("montreal"), &good());
    let mut bytes = fs::read(&a).unwrap();
    bytes[512 + 20] ^= 1;
    fs::write(&a, bytes).unwrap();
    assert!(matches!(archive::unpack(&a, &data).unwrap_err(), ArchiveError::Index(IndexError::Trailer)));

    let pieces = good();
    let entries: Vec<Entry> = pieces.iter().map(|p| p.0.clone()).collect();
    let mut index = archive::write_index(&header("montreal"), &entries);
    index[8..10].copy_from_slice(&2u16.to_le_bytes());
    let body = index[..index.len() - 32].to_vec();
    index = [body.clone(), Sha256::digest(&body).to_vec()].concat();
    let b = archive_of(&archives, "b.cityloom", &index, vec![]);
    assert!(matches!(archive::unpack(&b, &data).unwrap_err(), ArchiveError::Index(IndexError::Version(2))));
    assert!(names(&data).is_empty());
}

#[test]
fn refuses_hostile_areas() {
    for area in ["../escape", "a/b", "", "Montreal"] {
        let (tmp, archives, data) = roots();
        let a = write_archive(&archives, "a.cityloom", &header(area), &good());
        let e = archive::unpack(&a, &data).unwrap_err();
        assert!(matches!(e, ArchiveError::Area(_)), "{area:?}: {e}");
        only_areas_and_data(tmp.path());
        assert!(names(&data).is_empty());
    }
}

#[test]
fn a_truncated_archive_is_refused_and_leaves_nothing() {
    let (_tmp, archives, data) = roots();
    let a = write_archive(&archives, "a.cityloom", &header("montreal"), &good());
    let bytes = fs::read(&a).unwrap();
    fs::write(&a, &bytes[..bytes.len() * 6 / 10]).unwrap();
    assert!(archive::unpack(&a, &data).is_err());
    assert!(names(&data).is_empty());
}

#[test]
fn a_failure_leaves_the_area_already_there_unchanged() {
    let (_tmp, archives, data) = roots();
    let a = write_archive(&archives, "a.cityloom", &header("montreal"), &good());
    archive::unpack(&a, &data).unwrap();
    let before = tree(&data.join("montreal"));
    // another index (another tile), whose bytes do not match it
    let mut pieces = good();
    pieces[0] = road(0, 0, b"other tile data");
    pieces[0].1 = b"OTHER TILE DATA".to_vec();
    let b = write_archive(&archives, "b.cityloom", &header("montreal"), &pieces);
    refused(archive::unpack(&b, &data), "checksum");
    assert_eq!(tree(&data.join("montreal")), before);
    assert_eq!(names(&data), ["montreal"]);
}

/// A restart trusts the files it unpacked and verified earlier: the same index and every file at its size is reused
/// without hashing again, so a same-index archive with other bytes is not read.
#[test]
fn a_restart_trusts_what_it_unpacked_and_verified_earlier() {
    let (_tmp, archives, data) = roots();
    let a = write_archive(&archives, "a.cityloom", &header("montreal"), &good());
    archive::unpack(&a, &data).unwrap();
    let before = tree(&data.join("montreal"));
    let mut bytes = fs::read(&a).unwrap();
    let at = bytes.windows(14).position(|w| w == b"tile zero zero").unwrap();
    bytes[at] ^= 1;
    fs::write(&a, bytes).unwrap();
    assert!(archive::unpack(&a, &data).unwrap().reused);
    assert_eq!(tree(&data.join("montreal")), before);
}

#[test]
fn a_second_unpack_of_the_same_archive_skips_the_work() {
    let (_tmp, archives, data) = roots();
    let a = write_archive(&archives, "a.cityloom", &header("montreal"), &good());
    assert!(!archive::unpack(&a, &data).unwrap().reused);
    fs::write(data.join("montreal/sentinel"), b"x").unwrap();
    assert!(archive::unpack(&a, &data).unwrap().reused);
    assert!(data.join("montreal/sentinel").exists());
}

#[test]
fn a_damaged_unpacked_area_is_unpacked_again() {
    let (_tmp, archives, data) = roots();
    let a = write_archive(&archives, "a.cityloom", &header("montreal"), &good());
    archive::unpack(&a, &data).unwrap();
    let fresh = tree(&data.join("montreal"));
    type Damage = Box<dyn Fn(&Path)>;
    let damages: Vec<(&str, Damage)> = vec![
        ("a tile deleted", Box::new(|d| fs::remove_file(d.join("metro/0_0.osm.pbf")).unwrap())),
        ("a tile one byte shorter", Box::new(|d| fs::OpenOptions::new().write(true).open(d.join("metro/1_2.osm.pbf")).unwrap().set_len(19).unwrap())),
        ("index.json deleted", Box::new(|d| fs::remove_file(d.join("metro/index.json")).unwrap())),
        (".index-sha256 deleted", Box::new(|d| fs::remove_file(d.join(".index-sha256")).unwrap())),
        ("the marker changed", Box::new(|d| fs::write(d.join(".index-sha256"), "0000").unwrap())),
    ];
    for (what, damage) in damages {
        let dir = data.join("montreal");
        fs::write(dir.join("sentinel"), b"x").unwrap();
        damage(&dir);
        let got = archive::unpack(&a, &data).unwrap();
        assert!(!got.reused, "{what}");
        assert_eq!(tree(&dir), fresh, "{what}");
        assert_eq!(names(&data), ["montreal"], "{what}");
    }
}

#[test]
fn a_changed_archive_replaces_the_area() {
    let (_tmp, archives, data) = roots();
    let a1 = write_archive(&archives, "a1.cityloom", &header("montreal"), &good());
    archive::unpack(&a1, &data).unwrap();
    let a2 = write_archive(&archives, "a2.cityloom", &header("montreal"), &[road(0, 0, b"a different tile"), basemap(&[9u8; 100])]);
    let got = archive::unpack(&a2, &data).unwrap();
    assert!(!got.reused);
    let dir = data.join("montreal");
    assert_eq!(fs::read(dir.join("metro/0_0.osm.pbf")).unwrap(), b"a different tile");
    assert!(!dir.join("metro/1_2.osm.pbf").exists());
    assert_eq!(fs::read(dir.join("basemap/montreal.pmtiles")).unwrap(), vec![9u8; 100]);
    assert_eq!(names(&data), ["montreal"]);
}

#[test]
fn leftovers_of_a_crashed_start_are_removed() {
    let (_tmp, archives, data) = roots();
    for leftover in [".unpack-dead/x", ".old-dead/y", "keep/z"] {
        let p = data.join(leftover);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, b"x").unwrap();
    }
    let a = write_archive(&archives, "a.cityloom", &header("montreal"), &good());
    archive::unpack(&a, &data).unwrap();
    assert_eq!(names(&data), ["keep", "montreal"]);
}

#[test]
fn unpack_folder_takes_exactly_one_archive() {
    let (tmp, archives, data) = roots();
    assert!(matches!(archive::unpack_folder(&archives, &data).unwrap_err(), ArchiveError::NoArchive(_)));
    fs::create_dir(archives.join("x.cityloom")).unwrap();
    assert!(matches!(archive::unpack_folder(&archives, &data).unwrap_err(), ArchiveError::NoArchive(_)));
    write_archive(&archives, "a.cityloom", &header("montreal"), &good());
    write_archive(&archives, "b.cityloom", &header("montreal"), &good());
    assert!(matches!(archive::unpack_folder(&archives, &data).unwrap_err(), ArchiveError::TooMany(_)));
    fs::remove_file(archives.join("b.cityloom")).unwrap();
    fs::write(archives.join("notes.txt"), b"hello").unwrap();
    assert_eq!(archive::unpack_folder(&archives, &data).unwrap().area, "montreal");
    assert!(matches!(archive::unpack_folder(&tmp.path().join("missing"), &data).unwrap_err(), ArchiveError::Io { .. }));
}

#[cfg(unix)]
#[test]
fn a_data_root_that_cannot_be_written_is_a_clear_error() {
    use std::os::unix::fs::PermissionsExt;
    let (_tmp, archives, data) = roots();
    let a = write_archive(&archives, "a.cityloom", &header("montreal"), &good());
    fs::set_permissions(&data, fs::Permissions::from_mode(0o500)).unwrap();
    let result = archive::unpack(&a, &data);
    fs::set_permissions(&data, fs::Permissions::from_mode(0o700)).unwrap();
    let Err(error) = result else { return }; // running as root: nothing to refuse
    assert!(matches!(error, ArchiveError::Io { .. }), "{error}");
    assert!(error.to_string().contains(&data.display().to_string()), "{error}");
}
