//! The large slices keep their files in `model/`, `vm/` and `view/`: at a slice's root only `mod.rs`, `i18n/` and
//! `CLAUDE.md`.

const LAYERED: &[&str] = &["street", "junction"];
const ROOT_DIRS: &[&str] = &["model", "vm", "view", "i18n"];
const ROOT_FILES: &[&str] = &["mod.rs", "CLAUDE.md"];

#[test]
fn layered_slices_keep_their_files_in_model_vm_and_view() {
    for slice in LAYERED {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(slice);
        let mut stray = Vec::new();
        for entry in std::fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().into_string().unwrap();
            let allowed = if entry.file_type().unwrap().is_dir() { ROOT_DIRS } else { ROOT_FILES };
            if !allowed.contains(&name.as_str()) {
                stray.push(name);
            }
        }
        stray.sort();
        assert!(stray.is_empty(), "{slice}/ holds {stray:?} at its root; each belongs in model/, vm/ or view/");
    }
}
