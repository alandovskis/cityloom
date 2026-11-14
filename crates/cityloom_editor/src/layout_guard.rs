//! The large slices keep their files in `model/`, `vm/` and `view/`: at a slice's root only `mod.rs`, `i18n/` and
//! `CLAUDE.md`.

use std::path::{Path, PathBuf};

const LAYERED: &[&str] = &["street", "junction", "map"];
const ROOT_DIRS: &[&str] = &["model", "vm", "view", "i18n"];
const ROOT_FILES: &[&str] = &["mod.rs", "CLAUDE.md"];

/// What lies at a slice's root that belongs in one of its layers, sorted.
fn strays(dir: &Path) -> Vec<String> {
    let mut stray = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().into_string().unwrap();
        // A hidden file (`.DS_Store`, an editor's swap file) is the system's or an editor's, not the slice's.
        if name.starts_with('.') {
            continue;
        }
        let allowed = if entry.file_type().unwrap().is_dir() { ROOT_DIRS } else { ROOT_FILES };
        if !allowed.contains(&name.as_str()) {
            stray.push(name);
        }
    }
    stray.sort();
    stray
}

#[test]
fn layered_slices_keep_their_files_in_model_vm_and_view() {
    for slice in LAYERED {
        let stray = strays(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(slice));
        assert!(stray.is_empty(), "{slice}/ holds {stray:?} at its root; each belongs in model/, vm/ or view/");
    }
}

/// A slice root made of `names` (a name ending in `/` is a folder), in a folder of its own under the temp dir.
fn slice_with(test: &str, names: &[&str]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cityloom-layout-guard-{}-{test}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for name in names {
        match name.strip_suffix('/') {
            Some(folder) => std::fs::create_dir(dir.join(folder)).unwrap(),
            None => std::fs::write(dir.join(name), "").unwrap(),
        }
    }
    dir
}

#[test]
fn a_file_at_a_slice_root_is_a_stray() {
    let dir = slice_with("file", &["mod.rs", "CLAUDE.md", "model/", "vm/", "view/", "i18n/", "page.rs", "iso/"]);
    assert_eq!(strays(&dir), ["iso", "page.rs"]);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_hidden_file_at_a_slice_root_is_not_a_stray() {
    let dir = slice_with("hidden", &["mod.rs", "model/", ".DS_Store", ".mod.rs.swp"]);
    assert_eq!(strays(&dir), Vec::<String>::new(), "files the system or an editor leaves are not the slice's");
    std::fs::remove_dir_all(dir).unwrap();
}
