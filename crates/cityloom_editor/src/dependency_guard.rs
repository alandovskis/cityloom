//! What each slice may use of another: the dependency rule of the root `CLAUDE.md`, held by a test.
//!
//! `ALLOWED` is the rule. A use of another slice by production code that no entry allows fails, and so does an entry
//! nothing uses, so the table says neither less nor more than the code does. `shared` uses no slice, whatever the table
//! says; `lib.rs`, the composition root, may use any; a folder of `src/` that is neither in `SLICES` nor `shared` fails.
//!
//! The files are read from disk with Rust's own lexer (`proc_macro2`), from `lib.rs` down the `mod` declarations, so a
//! path in a macro's body (`view!`, `vec!`) counts and one in a comment or a string does not. Test code is exempt: an
//! item under `#[cfg(test)]`, and a file whose `mod` declaration is, whatever the file's name.
//!
//! Its limits:
//! - It reads names, not resolved items: an item one slice re-exports counts as that slice's.
//! - A root module that is not a slice, or a function such as `crate::json`, passes as a target.
//! - It does not see a path in a string (`#[serde(with = "..")]`), a path through the crate's own name, or what a macro
//!   builds.
//! - Only exactly `cfg(test)` is test code: what is under `cfg(any(.., test))` is production.
//! - Cycles between slices are accepted.

use std::collections::BTreeMap;
use std::path::Path;

/// The kernel every slice may use, and which may use no slice.
const SHARED: &str = "shared";

/// The slices: every folder of `src/` but `shared`.
const SLICES: &[&str] =
    &["account", "archive", "city", "junction", "map", "place", "project", "restriction", "share", "shell", "standards", "street", "sync", "timeline"];

/// Any slice of `SLICES`, as an entry's `by` or as the first component of its `uses`.
const ANY: &str = "*";

/// What some code may use of another slice.
struct Entry {
    /// Who: a slice's folder, a file (`map/vm/report.rs`), or `ANY` slice.
    by: &'static [&'static str],
    /// What: module paths from the slice, each covering what is under it; `*::model` is any slice's model.
    uses: &'static [&'static str],
    why: &'static str,
}

const ALLOWED: &[Entry] = &[
    Entry { by: &[ANY], uses: &["*::model"], why: "A slice may use another slice's model." },
    Entry {
        by: &[ANY],
        uses: &["city::store", "city::binding", "city::projection", "city::edits"],
        why: "Any slice may use the city's documents, their binding to a page, the projection and the edits.",
    },
    Entry { by: &[ANY], uses: &["restriction::store"], why: "Any slice may use the kept restrictions." },
    Entry { by: &[ANY], uses: &["place::area", "place::Bounds"], why: "Any slice may use a place's area and Bounds, the box Area::bounds gives." },
    Entry { by: &[ANY], uses: &["sync::hub"], why: "A slice reaches the sync only through its hub and its model." },
    Entry { by: &["street"], uses: &["standards::store"], why: "The street page reads the standards profile that is kept." },
    Entry { by: &["share"], uses: &["project::copies"], why: "Opening a share link makes the copy of a shared design." },
    Entry {
        by: &["shell"],
        uses: &["project::copies::is_copy", "project::copies::fork", "project::copies::Unforked"],
        why: "The shell makes a copy one of the person's projects; Unforked is what fork refuses with.",
    },
    Entry { by: &["shell"], uses: &["project::context::read_only"], why: "The shell says whether the page shows its project read-only." },
    Entry { by: &["shell"], uses: &["sync::SyncVm"], why: "The shell builds the sync's view-model." },
    Entry {
        by: &["map", "street", "junction", "place", "archive", "city/store.rs"],
        uses: &["project::context"],
        why: "These ask which project a page is opened in, and the city's store the area that project's record names.",
    },
    Entry {
        by: &["account/mod.rs", "shell/view.rs"],
        uses: &["archive::vm::ArchiveVm"],
        why: "The account page mounts the archive panel, and the shell's Share menu exports with it.",
    },
    Entry { by: &["account/mod.rs"], uses: &["archive::view::ArchiveView"], why: "The account page mounts the archive panel." },
    Entry {
        by: &["account/mod.rs"],
        uses: &["project::migrate::move_legacy_work"],
        why: "The account page is in no project and syncs, so it first moves the work from before projects.",
    },
    Entry {
        by: &["street/view/notes_tray.rs", "junction/view/notes_tray.rs"],
        uses: &["shell::tray"],
        why: "The Notes trays of the street and junction pages are the shell's tray.",
    },
    Entry {
        by: &["street/mod.rs", "junction/mod.rs", "map/mod.rs", "account/mod.rs"],
        uses: &["shell::mount", "shell::NotesAtStart"],
        why: "These pages mount the shell around themselves.",
    },
    Entry {
        by: &["street/vm/mod.rs", "junction/vm/mod.rs", "map/vm/mod.rs"],
        uses: &["shell::Target"],
        why: "A page's view-model is the target of the shell's commands.",
    },
    Entry {
        by: &["account/mod.rs", "junction/mod.rs"],
        uses: &["shell::Bare"],
        why: "The account page and a junction that cannot be drawn give the shell no target.",
    },
    Entry {
        by: &["map/vm/mod.rs", "map/vm/areas.rs", "map/view/mod.rs", "map/view/areas_view.rs"],
        uses: &["restriction::draw"],
        why: "The map draws an area's outline with the restriction slice's pure state machine.",
    },
    Entry {
        by: &["map/vm/report.rs"],
        uses: &["street::view::drawing", "junction::view::figure"],
        why: "The report draws the figures of the streets and the junctions.",
    },
    Entry {
        by: &["sync/local.rs"],
        uses: &["project::store", "project::migrate::legacy_waiting", "project::migrate::legacy_kept", "project::migrate::area_moved"],
        why: "The sync carries every project's records and the projects' own; the places wait while work from before projects waits to be moved.",
    },
    Entry {
        by: &["project/store.rs", "sync/local.rs"],
        uses: &["timeline::store"],
        why: "A project's milestones are among its records, which the sync carries and deleting the project blanks.",
    },
];

/// The text of each file, by its path from `src/` with `/` between the folders.
type Sources = BTreeMap<String, String>;

/// A use of another slice. Their order is by file, then line, then path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Use {
    file: String,
    line: usize,
    /// From the slice, without `crate::`: `city::store::CityStore`, `city::store::*`. The crate's root is `crate`.
    path: String,
}

/// Every `.rs` file under `src`.
fn sources(src: &Path) -> Sources {
    todo!()
}

/// Every use of another slice by production code, sorted, each once.
fn uses(sources: &Sources) -> Vec<Use> {
    todo!()
}

/// What is wrong with each use that `allowed` does not let its file make, in the order of `uses`.
fn refused(uses: &[Use], allowed: &[Entry]) -> Vec<String> {
    todo!()
}

/// Each pair of who and what in `allowed` that no use matches, in the table's order.
fn dead(uses: &[Use], allowed: &[Entry]) -> Vec<String> {
    todo!()
}

/// The folders of `src` that are neither a slice nor `shared`, sorted. A hidden one is the system's or an editor's.
fn unknown_folders(src: &Path) -> Vec<String> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(files: &[(&str, &str)]) -> Sources {
        files.iter().map(|(path, text)| (path.to_string(), text.to_string())).collect()
    }

    /// The line and path of each use in `junction/mod.rs` holding `code`.
    fn found(code: &str) -> Vec<(usize, String)> {
        uses(&tree(&[("lib.rs", "mod junction;"), ("junction/mod.rs", code)]))
            .into_iter()
            .map(|u| {
                assert_eq!(u.file, "junction/mod.rs");
                (u.line, u.path)
            })
            .collect()
    }

    fn at(expected: &[(usize, &str)]) -> Vec<(usize, String)> {
        expected.iter().map(|(line, path)| (*line, path.to_string())).collect()
    }

    fn used(file: &str, line: usize, path: &str) -> Use {
        Use { file: file.to_string(), line, path: path.to_string() }
    }

    fn one(file: &str, path: &str) -> Use {
        used(file, 1, path)
    }

    /// Holds that `allowed` refuses each of `no` with its own message, and lets each of `yes` pass.
    fn holds(allowed: &[Entry], no: &[(&str, &str)], yes: &[(&str, &str)]) {
        for (file, path) in no {
            let message = format!("src/{file}:1: uses {path}, which ALLOWED does not let it use");
            assert_eq!(refused(&[one(file, path)], allowed), [message]);
        }
        for (file, path) in yes {
            assert_eq!(refused(&[one(file, path)], allowed), Vec::<String>::new(), "{file} may use {path}");
        }
    }

    #[test]
    fn a_plain_use_of_another_slice_is_found() {
        let code = "use crate::street::vm::StreetVm;\nuse crate::junction::vm::JunctionVm;\nuse crate::shared::units::Units;\nuse crate::json;\n";
        assert_eq!(found(code), at(&[(1, "street::vm::StreetVm")]));
    }

    #[test]
    fn every_name_of_a_grouped_and_nested_use_is_found_on_its_line() {
        let code = "use crate::{\n    city::edits,\n    street::{model::Street, vm::{StreetVm, keys}},\n    shared::units::Units,\n};\n";
        assert_eq!(found(code), at(&[(2, "city::edits"), (3, "street::model::Street"), (3, "street::vm::StreetVm"), (3, "street::vm::keys")]));
    }

    #[test]
    fn self_in_a_group_is_the_module_itself() {
        let code = "use crate::city::edits::{self, Draft};\nuse crate::street::{self};\nuse crate::junction::vm::{self};\n";
        assert_eq!(found(code), at(&[(1, "city::edits"), (1, "city::edits::Draft"), (2, "street")]));
    }

    #[test]
    fn an_alias_does_not_hide_a_use() {
        let code = "use crate::street::vm::StreetVm as Vm;\nuse crate::city as town;\nuse crate::sync::model::{self as sync, Kind as K};\nuse crate as root;\nuse crate::{self as top};\n";
        let expected = [(1, "street::vm::StreetVm"), (2, "city"), (3, "sync::model"), (3, "sync::model::Kind"), (4, "crate"), (5, "crate")];
        assert_eq!(found(code), at(&expected));
    }

    #[test]
    fn a_glob_is_a_use_of_what_it_opens() {
        let code = "use crate::street::vm::*;\nuse crate::city::{store::*, edits};\nuse crate::*;\nuse crate::junction::vm::*;\n";
        assert_eq!(found(code), at(&[(1, "street::vm::*"), (2, "city::edits"), (2, "city::store::*"), (3, "crate::*")]));
    }

    #[test]
    fn a_path_written_in_place_is_found() {
        let code = "fn f(vm: &crate::street::vm::StreetVm) -> u32 {\n    let n = crate::city::store::open::<u32>(1);\n    vec![crate::sync::hub::flush()];\n    <crate::street::model::Street as Default>::default().n + n\n}\n";
        let expected = [(1, "street::vm::StreetVm"), (2, "city::store::open"), (3, "sync::hub::flush"), (4, "street::model::Street")];
        assert_eq!(found(code), at(&expected));
    }

    #[test]
    fn a_path_inside_view_is_found() {
        let code = "fn f() -> impl IntoView {\n    view! {\n        <section class=\"x\">\n            <crate::street::view::Panel vm=vm on:click=move |_| crate::sync::hub::flush()/>\n            {move || crate::city::store::name()}\n        </section>\n    }\n}\n";
        assert_eq!(found(code), at(&[(4, "street::view::Panel"), (4, "sync::hub::flush"), (5, "city::store::name")]));
    }

    #[test]
    fn a_super_path_is_found_only_when_it_leaves_the_slice() {
        let leaving = tree(&[
            ("lib.rs", "mod junction;"),
            ("junction/mod.rs", "mod vm;\nuse super::street::vm::StreetVm;\n"),
            (
                "junction/vm/mod.rs",
                "mod keys;\nuse super::super::city::store::CityStore;\nuse self::super::super::city::keys;\nmod inner {\n    use super::super::super::street::view;\n}\n",
            ),
            ("junction/vm/keys.rs", "use super::{super::super::street::model::Street, JunctionVm};\nuse super::super::super::*;\n"),
        ]);
        let expected = [
            used("junction/mod.rs", 2, "street::vm::StreetVm"),
            used("junction/vm/keys.rs", 1, "street::model::Street"),
            used("junction/vm/keys.rs", 2, "crate::*"),
            used("junction/vm/mod.rs", 2, "city::store::CityStore"),
            used("junction/vm/mod.rs", 3, "city::keys"),
            used("junction/vm/mod.rs", 5, "street::view"),
        ];
        assert_eq!(uses(&leaving), expected);

        let staying = tree(&[
            ("lib.rs", "mod junction;"),
            ("junction/mod.rs", "mod model;\nmod vm;\n"),
            ("junction/model.rs", ""),
            (
                "junction/vm/mod.rs",
                "mod keys;\nuse super::model::Junction;\nuse self::keys::Key;\npub(super) fn f() {}\npub(in super::super) fn g() {}\npub(in crate::junction) fn h(&self) { self.go(); super::model::rules(); crate::json(&1); crate::i18n_browser(); }\nmod inner {\n    use super::*;\n}\n",
            ),
            ("junction/vm/keys.rs", "use super::super::model::{self, Junction};\nuse super::super::*;\n"),
        ]);
        assert_eq!(uses(&staying), []);
    }

    #[test]
    fn a_pub_use_of_another_slice_is_found() {
        let code = "pub use crate::street::vm::StreetVm;\npub(crate) use crate::city::{store, edits::Draft};\n";
        assert_eq!(found(code), at(&[(1, "street::vm::StreetVm"), (2, "city::edits::Draft"), (2, "city::store")]));
    }

    #[test]
    fn comments_and_strings_do_not_count() {
        let code = "//! crate::street::vm::StreetVm\n/// See [`crate::street::vm`].\nfn f() {\n    // crate::street::vm::open();\n    /* crate::street::vm::open() */\n    let a = \"crate::street::vm\";\n    let b = r#\"crate::street::view\"#;\n    crate::city::store::open();\n}\n";
        assert_eq!(found(code), at(&[(8, "city::store::open")]));
    }

    #[test]
    fn what_is_under_cfg_test_does_not_count_and_only_that() {
        let items = "#[cfg(test)]\nuse crate::street::vm::{StreetVm, keys};\n#[cfg(test)]\nfn sample() { crate::street::vm::open(); }\n#[cfg(test)]\nthread_local! { static N: u32 = crate::street::vm::n(); }\nfn f() {\n    #[cfg(test)]\n    crate::street::vm::open();\n    crate::city::store::open();\n}\n#[cfg(test)]\npub(crate) mod fixtures {\n    mod deep {\n        use crate::street::vm::StreetVm;\n    }\n}\nuse crate::sync::hub;\n";
        assert_eq!(found(items), at(&[(10, "city::store::open"), (18, "sync::hub")]));

        let fields = "struct Store {\n    #[cfg(test)]\n    seen: Vec<crate::street::vm::StreetVm>,\n    #[cfg(test)]\n    pub(crate) pairs: Map<u32, crate::street::vm::Key>,\n    city: crate::city::store::CityStore,\n}\nfn new() -> Store {\n    Store {\n        #[cfg(test)]\n        seen: crate::street::vm::none(),\n        city: crate::city::store::open(),\n    }\n}\n";
        assert_eq!(found(fields), at(&[(6, "city::store::CityStore"), (12, "city::store::open")]));

        let production = "#[cfg(any(target_arch = \"wasm32\", test))]\nfn f() { crate::street::vm::open(); }\n#[cfg(not(test))]\nfn g() { crate::street::view::draw(); }\n#[cfg(target_arch = \"wasm32\")]\nuse crate::street::vm::keys;\n";
        assert_eq!(found(production), at(&[(2, "street::vm::open"), (4, "street::view::draw"), (6, "street::vm::keys")]));
    }

    #[test]
    fn a_file_is_test_code_by_its_declaration_not_its_name() {
        let declared_for_tests = tree(&[
            ("lib.rs", "mod junction;"),
            ("junction/mod.rs", "pub mod model;\n#[cfg(test)]\npub(in crate::junction) mod tests;\n"),
            ("junction/tests.rs", "use crate::street::vm::StreetVm;"),
            ("junction/model/mod.rs", "#[cfg(test)]\npub(crate) mod fixtures;\nuse crate::city::store::CityStore;\n"),
            ("junction/model/fixtures.rs", "mod deep;\nuse crate::street::vm::StreetVm;"),
            ("junction/model/fixtures/deep.rs", "use crate::street::vm::StreetVm;"),
        ]);
        assert_eq!(uses(&declared_for_tests), [used("junction/model/mod.rs", 3, "city::store::CityStore")]);

        let named_for_tests = tree(&[
            ("lib.rs", "mod junction;"),
            ("junction/mod.rs", "mod fixtures;\nmod tests;\n"),
            ("junction/fixtures.rs", "use crate::street::vm::StreetVm;"),
            ("junction/tests.rs", "use crate::street::vm::StreetVm;"),
        ]);
        let expected = [used("junction/fixtures.rs", 1, "street::vm::StreetVm"), used("junction/tests.rs", 1, "street::vm::StreetVm")];
        assert_eq!(uses(&named_for_tests), expected);
    }

    #[test]
    fn the_composition_root_may_use_any_slice_and_shared_none() {
        let root = tree(&[
            ("lib.rs", "mod junction;\nmod street;\nfn f() { street::vm::open(); crate::city::store::open(); shell::mount(); }\nuse shell::Bare;\n"),
            ("junction/mod.rs", "use crate::street::vm::StreetVm;"),
            ("street/mod.rs", ""),
        ]);
        assert_eq!(uses(&root), [used("junction/mod.rs", 1, "street::vm::StreetVm")]);

        let shared = tree(&[("lib.rs", "mod shared;"), ("shared/mod.rs", "use crate::city::store::CityStore;")]);
        let all = [Entry { by: &["*", "shared", "shared/mod.rs"], uses: &["city::store"], why: "A sample." }];
        assert_eq!(refused(&uses(&shared), &all), ["src/shared/mod.rs:1: uses city::store::CityStore, which ALLOWED does not let it use"]);
    }

    #[test]
    fn a_use_not_allowed_is_refused_with_its_file_line_and_path() {
        let allowed = [
            Entry { by: &["*"], uses: &["*::model", "city::store"], why: "A sample." },
            Entry { by: &["street/view/notes_tray.rs"], uses: &["shell::tray"], why: "A sample." },
            Entry { by: &["share"], uses: &["project::copies"], why: "A sample." },
        ];
        let junction = tree(&[
            ("lib.rs", "mod junction;"),
            ("junction/mod.rs", "mod vm;"),
            ("junction/vm.rs", "\nuse crate::street::vm::StreetVm;\nuse crate::street::model::Street;\n"),
        ]);
        assert_eq!(refused(&uses(&junction), &allowed), ["src/junction/vm.rs:2: uses street::vm::StreetVm, which ALLOWED does not let it use"]);

        holds(
            &allowed,
            &[
                ("junction/vm.rs", "city::stores::x"),
                ("junction/vm.rs", "street"),
                ("junction/vm.rs", "street::*"),
                ("street/view/notes.rs", "shell::tray::Tray"),
                ("shell/vm.rs", "project::copies::keep"),
                ("glue.rs", "city::store::CityStore"),
            ],
            &[
                ("junction/vm.rs", "city::store"),
                ("junction/vm.rs", "city::store::*"),
                ("street/view/notes_tray.rs", "shell::tray::Tray"),
                ("share/vm.rs", "project::copies::keep"),
            ],
        );
    }

    #[test]
    fn an_entry_nothing_uses_is_dead() {
        let allowed = [
            Entry { by: &["street", "map/vm/mod.rs"], uses: &["city::store", "city::keys"], why: "A sample." },
            Entry { by: &["*"], uses: &["*::model", "place::area"], why: "A sample." },
        ];
        let uses = [
            one("street/vm/mod.rs", "city::store::CityStore"),
            one("map/vm/mod.rs", "city::keys"),
            one("map/view/mod.rs", "city::store::CityStore"),
            one("junction/mod.rs", "street::model"),
        ];
        let expected =
            ["ALLOWED: nothing in street uses city::keys", "ALLOWED: nothing in map/vm/mod.rs uses city::store", "ALLOWED: nothing in * uses place::area"];
        assert_eq!(dead(&uses, &allowed), expected);
    }

    #[test]
    fn a_folder_that_is_no_known_slice_is_unknown() {
        let src = std::env::temp_dir().join(format!("cityloom-dependency-guard-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&src);
        for folder in ["junction", "street", "shared", "bikeshare", ".cache"] {
            std::fs::create_dir_all(src.join(folder)).unwrap();
        }
        std::fs::write(src.join("lib.rs"), "").unwrap();
        assert_eq!(unknown_folders(&src), ["bikeshare"]);
        std::fs::remove_dir_all(src).unwrap();
    }

    #[test]
    fn production_code_uses_only_what_the_table_allows() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let unknown = unknown_folders(&src);
        assert!(unknown.is_empty(), "folders of src/ that are neither in SLICES nor {SHARED}:\n{}", unknown.join("\n"));
        let uses = uses(&sources(&src));
        let refused = refused(&uses, ALLOWED);
        assert!(refused.is_empty(), "{}", refused.join("\n"));
        let dead = dead(&uses, ALLOWED);
        assert!(dead.is_empty(), "{}", dead.join("\n"));
        for entry in ALLOWED {
            assert!(!entry.why.trim().is_empty(), "no reason for what {:?} may use: {:?}", entry.by, entry.uses);
        }
    }

    #[test]
    fn the_table_holds_the_dependency_rule() {
        holds(
            ALLOWED,
            &[
                ("junction/vm/mod.rs", "street::vm::StreetVm"),
                ("junction/mod.rs", "street"),
                ("street/mod.rs", "crate::*"),
                ("street/view/notes.rs", "shell::tray::Tray"),
                ("share/vm.rs", "restriction::draw::Draw"),
                ("shared/units.rs", "city::edits::Draft"),
                ("junction/view/page.rs", "shell::mount"),
                ("map/vm/mod.rs", "street::view::drawing"),
                ("share/vm.rs", "project::context::open_page"),
                ("shell/vm.rs", "project::context::open_page"),
                ("city/binding.rs", "project::context::area_of"),
                ("junction/vm/mod.rs", "standards::store::activate"),
                ("shell/vm.rs", "project::copies::keep"),
                ("account/vm.rs", "sync::SyncVm"),
                ("street/model/mod.rs", "city::scenario::Action"),
            ],
            &[
                ("timeline/work.rs", "city::edits::Draft"),
                ("share/model.rs", "archive::model::read_edits"),
                ("junction/model/mod.rs", "street::model::Street"),
                ("street/view/notes_tray.rs", "shell::tray::Tray"),
                ("map/vm/report.rs", "street::view::drawing"),
                ("city/store.rs", "project::context::area_of"),
                ("shell/vm.rs", "project::context::read_only"),
                ("sync/local.rs", "project::store::ProjectStore"),
                ("account/mod.rs", "project::migrate::move_legacy_work"),
            ],
        );
    }
}
