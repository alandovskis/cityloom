//! What each slice may use of another, and each layer inside a slice: the dependency rule and the layers of the root
//! `CLAUDE.md`, held by tests.
//!
//! `ALLOWED` is the rule. A use of another slice by production code that no entry allows fails, and so does an entry
//! nothing uses, so the table says neither less nor more than the code does. `shared` uses no slice, whatever the table
//! says; `lib.rs`, the composition root, may use any; a folder of `src/` that is neither in `SLICES` nor `shared` fails.
//!
//! The layer rules: in the slices of `layout_guard::LAYERED`, the model uses neither the view-model nor the view of its
//! slice, and the view-model does not use the view; neither takes the slice whole (`use crate::junction`, its glob), which
//! would open them. The library rules, in every slice: the model names neither Leptos nor the browser's crates, whether in
//! a path, an import or an attribute, nor a crate of the same family (`gloo_timers`); the view-model may name Leptos, but
//! neither the browser's crates, even through Leptos, nor Leptos's `document()` and `window()`. The slice's `model.rs` and
//! what is under its `model/` are the model, `vm.rs` and what is under `vm/` the view-model; no other file is held to these
//! rules. `EXCEPTIONS` lets a break pass, with the reason; an entry no break matches fails, and so does a blank reason.
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
//! - After the first braces of an item, a statement or a list's element under `#[cfg(test)]`, the rest of it (an `else`,
//!   a method call) is production.
//! - A generic parameter, or a closure's parameter, under `#[cfg(test)]` may hide what follows it, in other items too.
//! - Under `#[cfg(test)]`, a `<` with no space before it opens generic arguments, so a comparison or a shift rustfmt has
//!   not spaced may hide the elements after it, up to a `>`.
//! - In a macro whose elements end at anything but a `;`, a comma or braces, one under `#[cfg(test)]` may hide the next.
//! - Cycles between slices are accepted.
//! - An item of a layer that the slice's root, or another module, re-exports passes as theirs.
//! - The layer is the file's: an inline `mod model { .. }` elsewhere is in none.
//! - A bare `document()` or `window()` in a view-model counts as Leptos's, even when it is the file's own function.
//! - Leptos's `document` or `window` given as a value without a call, or called through an alias or an imported module of
//!   Leptos (`prelude::window()`), passes.
//! - A name of the code's own that begins as a library's does (`leptos_rules`, a field `web_sys`) counts as the library.

use std::collections::BTreeMap;
use std::path::Path;
use std::str::FromStr;

use proc_macro2::{Delimiter, Group, Ident, Spacing, TokenStream, TokenTree};

use crate::i18n_guard::{is_cfg_test, is_punct};
use crate::layout_guard::LAYERED;

/// The composition root, which may use any slice.
const ROOT: &str = "lib.rs";

/// The kernel every slice may use, and which may use no slice.
const SHARED: &str = "shared";

/// The slices: every folder of `src/` but `shared`.
const SLICES: &[&str] =
    &["account", "archive", "city", "junction", "map", "place", "project", "restriction", "share", "shell", "standards", "street", "sync", "timeline"];

/// Any slice of `SLICES`, as an entry's `by` or as the first component of its `uses`.
const ANY: &str = "*";

/// Leptos, which a model may not name. A crate whose name goes on after a `_` is of its family.
const LEPTOS_CRATE: &str = "leptos";

/// The browser's crates, with their families, which neither a model nor a view-model may name.
const BROWSER_CRATES: &[&str] = &["web_sys", "js_sys", "gloo", "wasm_bindgen"];

/// Leptos's functions that give the browser's objects, which a view-model may not call.
const BROWSER_CALLS: &[&str] = &["document", "window"];

/// The layers, as a break names them.
const MODEL: &str = "model";
const VIEW_MODEL: &str = "view-model";

/// What some code may use of another slice, or a layer against its rules.
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

/// The breaks of a layer's rules that are let pass, if any.
const EXCEPTIONS: &[Entry] = &[];

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

/// A use a layer's rules do not allow its file. Their order is their uses'.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Break {
    used: Use,
    /// `model` or `view-model`.
    layer: &'static str,
}

/// Every `.rs` file under `src`.
fn sources(src: &Path) -> Sources {
    let mut out = Sources::new();
    add_sources(src, "", &mut out);
    out
}

/// Adds the `.rs` files under `dir`, whose own path from `src/` is `prefix`: empty, or ending in `/`.
fn add_sources(dir: &Path, prefix: &str, out: &mut Sources) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = format!("{prefix}{}", entry.file_name().into_string().unwrap());
        if entry.file_type().unwrap().is_dir() {
            add_sources(&entry.path(), &format!("{path}/"), out);
        } else if path.ends_with(".rs") {
            out.insert(path, std::fs::read_to_string(entry.path()).unwrap());
        }
    }
}

/// What a path is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// From the crate's root.
    Crate,
    /// From the crate's root, after `in`, as in a visibility.
    Visibility,
    /// From Leptos or a crate of the browser's.
    Library,
    /// `document` or `window` called by its name alone.
    Call,
}

/// A path production code holds.
struct Reading {
    file: String,
    line: usize,
    /// From the crate's root or the library.
    path: Vec<String>,
    kind: Kind,
    /// Whether it opens what it names to other names: an alias, or `self` in a group.
    opens: bool,
}

impl Reading {
    fn used(&self, path: String) -> Use {
        Use { file: self.file.clone(), line: self.line, path }
    }
}

/// Every use of another slice by production code, sorted, each once.
fn uses(sources: &Sources) -> Vec<Use> {
    sorted(readings(sources).iter().filter_map(use_of_another))
}

/// Every path production code holds.
fn readings(sources: &Sources) -> Vec<Reading> {
    let mut found = Vec::new();
    read(sources, ROOT, &[], &mut found);
    found
}

/// `found` in order, each once.
fn sorted<T: Ord>(found: impl Iterator<Item = T>) -> Vec<T> {
    let mut found: Vec<T> = found.collect();
    found.sort();
    found.dedup();
    found
}

/// The use `reading` is, if it is of another slice, or of the crate's root itself when it opens it to other names. The
/// composition root's are its own business.
fn use_of_another(reading: &Reading) -> Option<Use> {
    if reading.file == ROOT || !matches!(reading.kind, Kind::Crate | Kind::Visibility) {
        return None;
    }
    let path = match reading.path.as_slice() {
        [] if reading.opens => "crate".to_string(),
        [star] if star == "*" => "crate::*".to_string(),
        [slice, ..] if SLICES.contains(&slice.as_str()) && folder(&reading.file) != Some(slice) => reading.path.join("::"),
        _ => return None,
    };
    Some(reading.used(path))
}

/// The break `reading` is, if its file is in a layer whose rules do not allow it.
fn break_by(reading: &Reading) -> Option<Break> {
    let layer = layer(&reading.file)?;
    let breaks = match reading.kind {
        Kind::Crate | Kind::Visibility => breaks_layer(layer, reading),
        Kind::Library => layer == MODEL || reaches_browser(&reading.path),
        Kind::Call => layer == VIEW_MODEL,
    };
    breaks.then(|| Break { used: reading.used(reading.path.join("::")), layer })
}

/// Whether `reading`, a path of the crate in a file of `layer`, is of a layer of its own slice that `layer` may not use, or
/// takes the slice whole.
fn breaks_layer(layer: &str, reading: &Reading) -> bool {
    let [slice, rest @ ..] = reading.path.as_slice() else { return false };
    if folder(&reading.file) != Some(slice) || !LAYERED.contains(&slice.as_str()) {
        return false;
    }
    match rest {
        [] => reading.kind != Kind::Visibility,
        [next, ..] => next == "*" || next == "view" || next == "vm" && layer == MODEL,
    }
}

/// Whether `path`, of a library, names a crate of the browser's, through Leptos or not, or is of Leptos's `document` or
/// `window`.
fn reaches_browser(path: &[String]) -> bool {
    path.iter().any(|name| is_browser(name)) || path.last().is_some_and(|name| BROWSER_CALLS.contains(&name.as_str()))
}

/// Adds to `found` the paths in `file`, which is `module`, and in the files of the modules it declares.
fn read(sources: &Sources, file: &str, module: &[String], found: &mut Vec<Reading>) {
    let stream = TokenStream::from_str(&sources[file]).unwrap_or_else(|e| panic!("src/{file} does not lex: {e}"));
    Reader { sources, file, found }.walk(stream, module);
}

/// What reads one file.
struct Reader<'a> {
    sources: &'a Sources,
    file: &'a str,
    found: &'a mut Vec<Reading>,
}

impl Reader<'_> {
    /// Reads the production code of `stream`, which stands in `module`.
    fn walk(&mut self, stream: TokenStream, module: &[String]) {
        let t: Vec<TokenTree> = stream.into_iter().collect();
        let mut i = 0;
        while i < t.len() {
            i = self.step(&t, i, module);
        }
    }

    /// Reads what begins at `t[i]`, and gives where it ends.
    fn step(&mut self, t: &[TokenTree], i: usize, module: &[String]) -> usize {
        if is_punct(&t[i], '#') && t.get(i + 1).is_some_and(is_cfg_test) {
            return after_test_code(t, i + 2);
        }
        if let Some(end) = self.declaration(t, i, module) {
            return end;
        }
        if starts_path(t, i) {
            let kind = if i >= 1 && is_word(&t[i - 1], "in") { Kind::Visibility } else { Kind::Crate };
            return self.path(t, i, module, kind, false);
        }
        if starts_library(t, i) {
            return self.path(t, i, &[], Kind::Library, false);
        }
        if is_bare_call(t, i) {
            self.keep(t[i].span().start().line, &[t[i].to_string()], Kind::Call, false);
        }
        if let TokenTree::Group(group) = &t[i] {
            self.walk(group.stream(), module);
        }
        i + 1
    }

    /// Reads the module that a `mod` at `t[i]` declares in `module`, if there is one, and gives where it ends.
    fn declaration(&mut self, t: &[TokenTree], i: usize, module: &[String]) -> Option<usize> {
        let [keyword, TokenTree::Ident(name), body, ..] = &t[i..] else { return None };
        if !is_word(keyword, "mod") {
            return None;
        }
        let mut inner = module.to_vec();
        inner.push(bare(name));
        if let Some(group) = braced(body) {
            self.walk(group.stream(), &inner);
        } else if is_punct(body, ';') {
            self.declared(&inner);
        } else {
            return None;
        }
        Some(i + 3)
    }

    /// Reads the file of `module`, which this file declares.
    fn declared(&mut self, module: &[String]) {
        let path = module.join("/");
        let files = [format!("{path}.rs"), format!("{path}/mod.rs")];
        let Some(file) = files.iter().find(|file| self.sources.contains_key(*file)) else {
            panic!("src/{} declares the module {}, which has no file", self.file, module.join("::"));
        };
        read(self.sources, file, module, self.found);
    }

    /// Finds what the path of `kind` at `t[i]` names, from `from` (a group's names are `grouped`), and gives where the path
    /// ends.
    fn path(&mut self, t: &[TokenTree], mut i: usize, from: &[String], kind: Kind, grouped: bool) -> usize {
        let mut at = from.to_vec();
        while let Some(part) = t.get(i) {
            let mut opens = false;
            match part {
                TokenTree::Ident(name) if name == "crate" => at.clear(),
                TokenTree::Ident(name) if name == "self" => opens = grouped,
                TokenTree::Ident(name) if name == "super" => {
                    at.pop();
                }
                TokenTree::Ident(name) => at.push(bare(name)),
                star if is_punct(star, '*') => at.push("*".to_string()),
                group => {
                    let Some(group) = braced(group) else { return i };
                    self.names(group, &at, kind);
                    return i + 1;
                }
            }
            i += 1;
            if is_separator(t, i) && t.get(i + 2).is_some_and(is_part) {
                i += 2;
                continue;
            }
            opens |= t.get(i).is_some_and(|next| is_word(next, "as"));
            self.keep(part.span().start().line, &at, kind, opens);
            return i;
        }
        i
    }

    /// Finds what each name of the group of a path of `kind` names, from `from`.
    fn names(&mut self, group: &Group, from: &[String], kind: Kind) {
        let t: Vec<TokenTree> = group.stream().into_iter().collect();
        let mut i = 0;
        while i < t.len() {
            i = self.path(&t, i, from, kind, true);
            // An alias changes nothing in what is used.
            i += t[i..].iter().take_while(|x| !is_punct(x, ',')).count() + 1;
        }
    }

    fn keep(&mut self, line: usize, path: &[String], kind: Kind, opens: bool) {
        self.found.push(Reading { file: self.file.to_string(), line, path: path.to_vec(), kind, opens });
    }
}

/// Where what is under a `#[cfg(test)]` ends, from what follows it: an item or a statement at its `;` or its braces, a
/// field or a struct literal's entry at its comma, anything else (an element of a list, a match arm, an expression
/// statement, a macro) at the first of the three. Without a comma outside `<..>`, the first comma is the one.
fn after_test_code(t: &[TokenTree], start: usize) -> usize {
    let begin = after_visibility(t, start);
    let field = is_field(t, begin);
    let item = is_item(t, begin);
    let mut depth = 0_usize;
    let mut first_comma = None;
    for i in begin..t.len() {
        if is_punct(&t[i], ';') || !field && braced(&t[i]).is_some() {
            return i + 1;
        }
        if opens_angle(t, i) {
            depth += 1;
        } else if is_punct(&t[i], '>') && !is_joint(&t[i - 1], '-') && !is_joint(&t[i - 1], '=') {
            depth = depth.saturating_sub(1);
        } else if is_punct(&t[i], ',') && !item {
            if depth == 0 {
                return i + 1;
            }
            first_comma.get_or_insert(i + 1);
        }
    }
    first_comma.unwrap_or(t.len())
}

/// Where the attributes and the visibility at `t[i]` end.
fn after_visibility(t: &[TokenTree], mut i: usize) -> usize {
    while t.get(i).is_some_and(|x| is_punct(x, '#')) {
        i += 2;
    }
    if t.get(i).is_some_and(|x| is_word(x, "pub")) {
        i += 1;
        if t.get(i).is_some_and(parenthesized) {
            i += 1;
        }
    }
    i
}

/// Whether a field or a struct literal's entry is at `t[i]`: a name and one `:`.
fn is_field(t: &[TokenTree], i: usize) -> bool {
    matches!(t.get(i), Some(TokenTree::Ident(_))) && t.get(i + 1).is_some_and(|x| is_punct(x, ':')) && !is_separator(t, i + 1)
}

/// Whether an item or a statement with a keyword of its own is at `t[i]`, so it has a `;` or braces and a comma before
/// them does not end it. The types `fn(..)` and `for<..> fn(..)` are neither.
fn is_item(t: &[TokenTree], mut i: usize) -> bool {
    const WORDS: &[&str] = &["use", "mod", "impl", "struct", "enum", "trait", "type", "const", "static", "let", "if", "match", "for", "while", "loop"];
    while t.get(i).is_some_and(|x| ["async", "unsafe", "extern"].iter().any(|word| is_word(x, word))) {
        i += 1;
        if is_word(&t[i - 1], "extern") && matches!(t.get(i), Some(TokenTree::Literal(_))) {
            i += 1;
        }
    }
    let (Some(word), Some(next)) = (t.get(i), t.get(i + 1)) else { return false };
    if is_word(word, "fn") {
        return !parenthesized(next);
    }
    WORDS.iter().any(|w| is_word(word, w)) && !(is_word(word, "for") && is_punct(next, '<'))
}

/// Whether `t[i]` opens generic arguments: a `<` right after what it belongs to, or after another that opens them. A
/// comparison or a shift has a space before it.
fn opens_angle(t: &[TokenTree], i: usize) -> bool {
    is_punct(&t[i], '<') && if is_joint(&t[i - 1], '<') { opens_angle(t, i - 1) } else { t[i - 1].span().end() == t[i].span().start() }
}

/// Whether a path from the crate, this module or the one above begins at `t[i]`.
fn starts_path(t: &[TokenTree], i: usize) -> bool {
    let above = is_word(&t[i], "crate") || is_word(&t[i], "super");
    if !above && !is_word(&t[i], "self") || i >= 2 && is_separator(t, i - 2) {
        return false;
    }
    is_separator(t, i + 1) || above && t.get(i + 1).is_some_and(|x| is_word(x, "as"))
}

/// Whether Leptos or a crate of the browser's is named at `t[i]`, with a path after it or not. A field or a method is not
/// one; what follows a `::` is, as in `::web_sys`.
fn starts_library(t: &[TokenTree], i: usize) -> bool {
    let TokenTree::Ident(name) = &t[i] else { return false };
    let name = bare(name);
    (is_of(LEPTOS_CRATE, &name) || is_browser(&name)) && !follows_dot(t, i)
}

/// Whether `document()` or `window()` is at `t[i]`, called by its name alone.
fn is_bare_call(t: &[TokenTree], i: usize) -> bool {
    let called = matches!(t.get(i + 1), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis && g.stream().is_empty());
    let declared = i >= 1 && is_word(&t[i - 1], "fn");
    let in_path = i >= 2 && is_separator(t, i - 2);
    called && BROWSER_CALLS.iter().any(|name| is_word(&t[i], name)) && !declared && !in_path && !follows_dot(t, i)
}

/// Whether `t[i]` follows a `.`, and not the `..` of a range.
fn follows_dot(t: &[TokenTree], i: usize) -> bool {
    i >= 1 && is_punct(&t[i - 1], '.') && !(i >= 2 && is_joint(&t[i - 2], '.'))
}

/// Whether `name` is the crate `library` or one of its family: its name, then `_` and more.
fn is_of(library: &str, name: &str) -> bool {
    name.strip_prefix(library).is_some_and(|rest| rest.is_empty() || rest.starts_with('_'))
}

fn is_browser(name: &str) -> bool {
    BROWSER_CRATES.iter().any(|library| is_of(library, name))
}

/// Whether `t` can follow a `::` in a path: a name, a glob or a group. A turbofish cannot, so the path stops before it.
fn is_part(t: &TokenTree) -> bool {
    matches!(t, TokenTree::Ident(_)) || is_punct(t, '*') || braced(t).is_some()
}

/// Whether `t[i]` begins a `::`.
fn is_separator(t: &[TokenTree], i: usize) -> bool {
    matches!(t.get(i..), Some([a, b, ..]) if is_joint(a, ':') && is_punct(b, ':'))
}

/// Whether `t` is `c` with another punctuation mark right after it.
fn is_joint(t: &TokenTree, c: char) -> bool {
    matches!(t, TokenTree::Punct(p) if p.as_char() == c && p.spacing() == Spacing::Joint)
}

fn is_word(t: &TokenTree, word: &str) -> bool {
    matches!(t, TokenTree::Ident(name) if name == word)
}

fn braced(t: &TokenTree) -> Option<&Group> {
    match t {
        TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => Some(g),
        _ => None,
    }
}

fn parenthesized(t: &TokenTree) -> bool {
    matches!(t, TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis)
}

/// The name without the `r#` a raw identifier has.
fn bare(name: &Ident) -> String {
    name.to_string().trim_start_matches("r#").to_string()
}

/// The folder of `src/` that `file` is in: its slice's, or `shared`. A root file is in none.
fn folder(file: &str) -> Option<&str> {
    file.split_once('/').map(|(folder, _)| folder)
}

/// The layer of its slice that `file` is in, if it is in one the rules hold.
fn layer(file: &str) -> Option<&'static str> {
    let (slice, rest) = file.split_once('/')?;
    if !SLICES.contains(&slice) {
        return None;
    }
    match rest.split('/').next()? {
        "model" | "model.rs" => Some(MODEL),
        "vm" | "vm.rs" => Some(VIEW_MODEL),
        _ => None,
    }
}

/// Whether an entry's `by` names `file`.
fn names(by: &str, file: &str) -> bool {
    by == file || folder(file).is_some_and(|folder| by == folder || by == ANY && SLICES.contains(&folder))
}

/// Whether an entry's `pattern` covers `path`, component by component. Nothing covers the crate's root.
fn covers(pattern: &str, path: &str) -> bool {
    let mut path = path.split("::");
    let mut pattern = pattern.split("::");
    let same_slice = path.next().is_some_and(|slice| SLICES.contains(&slice) && pattern.next().is_some_and(|p| p == ANY || p == slice));
    same_slice && pattern.all(|p| path.next() == Some(p))
}

/// What is wrong with each use that `allowed` does not let its file make, in the order of `uses`.
fn refused(uses: &[Use], allowed: &[Entry]) -> Vec<String> {
    let may = |u: &Use| allowed.iter().any(|e| e.by.iter().any(|by| names(by, &u.file)) && e.uses.iter().any(|pattern| covers(pattern, &u.path)));
    uses.iter()
        .filter(|u| folder(&u.file) == Some(SHARED) || !may(u))
        .map(|u| format!("src/{}:{}: uses {}, which ALLOWED does not let it use", u.file, u.line, u.path))
        .collect()
}

/// Each pair of who and what in `allowed` that no use matches, in the table's order.
fn dead(uses: &[Use], allowed: &[Entry]) -> Vec<String> {
    pairs(allowed)
        .filter(|(by, pattern)| !uses.iter().any(|u| names(by, &u.file) && covers(pattern, &u.path)))
        .map(|(by, pattern)| format!("ALLOWED: nothing in {by} uses {pattern}"))
        .collect()
}

/// Each pair of who and what in `entries`, in the table's order.
fn pairs(entries: &[Entry]) -> impl Iterator<Item = (&'static str, &'static str)> {
    entries.iter().flat_map(|e| e.by.iter().flat_map(|by| e.uses.iter().map(move |pattern| (*by, *pattern))))
}

/// Every break of a layer rule or a library rule by production code, sorted, each once.
fn broken(sources: &Sources) -> Vec<Break> {
    sorted(readings(sources).iter().filter_map(break_by))
}

/// Whether an exception's `by` and `pattern` let `broken` pass: the file is named, and the path begins with the pattern,
/// component by component.
fn excepts(by: &str, pattern: &str, broken: &Break) -> bool {
    let mut path = broken.used.path.split("::");
    names(by, &broken.used.file) && pattern.split("::").all(|p| path.next() == Some(p))
}

/// What is wrong with each break that no entry of `exceptions` lets pass, in their order.
fn unexcepted(breaks: &[Break], exceptions: &[Entry]) -> Vec<String> {
    breaks
        .iter()
        .filter(|b| !pairs(exceptions).any(|(by, pattern)| excepts(by, pattern, b)))
        .map(|Break { used, layer }| format!("src/{}:{}: uses {}, which a {layer} may not use", used.file, used.line, used.path))
        .collect()
}

/// Each pair of who and what in `exceptions` that no break matches, in the table's order, then each entry without a
/// reason.
fn faulty(breaks: &[Break], exceptions: &[Entry]) -> Vec<String> {
    let idle = pairs(exceptions)
        .filter(|(by, pattern)| !breaks.iter().any(|b| excepts(by, pattern, b)))
        .map(|(by, pattern)| format!("EXCEPTIONS: nothing in {by} breaks a rule with {pattern}"));
    let unexplained = exceptions.iter().filter(|e| e.why.trim().is_empty()).map(|e| format!("EXCEPTIONS: no reason for what {:?} may use: {:?}", e.by, e.uses));
    idle.chain(unexplained).collect()
}

/// The folders of `src` that are neither a slice nor `shared`, sorted. A hidden one is the system's or an editor's.
fn unknown_folders(src: &Path) -> Vec<String> {
    let mut unknown = Vec::new();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().into_string().unwrap();
        if entry.file_type().unwrap().is_dir() && !name.starts_with('.') && name != SHARED && !SLICES.contains(&name.as_str()) {
            unknown.push(name);
        }
    }
    unknown.sort();
    unknown
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
    fn what_is_under_cfg_test_in_a_list_ends_at_its_own_comma() {
        let shorthand = "fn new() -> S {\n    S {\n        #[cfg(test)]\n        seen,\n        city: crate::city::store::open(),\n    }\n}\n";
        let arm = "fn f(k: Kind) -> u32 {\n    match k {\n        #[cfg(test)]\n        Kind::Seen => crate::street::vm::open(),\n        Kind::Kept => crate::city::store::open(),\n    }\n}\n";
        let variant = "enum Kind {\n    #[cfg(test)]\n    Seen(crate::street::vm::Key),\n    Kept(crate::city::store::Key),\n}\n";
        let tuple = "struct S(\n    #[cfg(test)]\n    crate::street::vm::Key,\n    crate::city::store::Key,\n);\n";
        let call = "fn f() {\n    g(\n        #[cfg(test)]\n        crate::street::vm::open(),\n        crate::city::store::open(),\n    );\n}\n";
        let in_macro = "fn f() {\n    g![\n        #[cfg(test)]\n        crate::street::vm::open(),\n        crate::city::store::open(),\n    ];\n}\n";
        let comparison = "fn new() -> S {\n    S {\n        #[cfg(test)]\n        n: a < b,\n        city: crate::street::vm::open(),\n    }\n}\n";
        let shift = "fn new() -> S {\n    S {\n        #[cfg(test)]\n        n: 1 << 2,\n        city: crate::street::vm::open(),\n    }\n}\n";
        let samples = [
            ("a shorthand field", shorthand, (5, "city::store::open")),
            ("a match arm", arm, (5, "city::store::open")),
            ("an enum variant", variant, (4, "city::store::Key")),
            ("a field of a tuple struct", tuple, (4, "city::store::Key")),
            ("a call's argument", call, (5, "city::store::open")),
            ("a macro's argument", in_macro, (5, "city::store::open")),
            ("a field whose value holds a comparison", comparison, (5, "street::vm::open")),
            ("a field whose value holds a shift", shift, (5, "street::vm::open")),
        ];
        let mut wrong = Vec::new();
        for (name, code, expected) in samples {
            let (found, expected) = (found(code), at(&[expected]));
            if found != expected {
                wrong.push(format!("{name}: found {found:?}, not {expected:?}"));
            }
        }
        assert!(wrong.is_empty(), "{} of {} samples:\n{}", wrong.len(), samples.len(), wrong.join("\n"));
    }

    #[test]
    fn a_qualified_path_in_generic_arguments_does_not_end_what_is_under_cfg_test() {
        let field = "struct S {\n    #[cfg(test)]\n    seen: Map<<X as Tr>::Out, crate::street::vm::Key>,\n    city: crate::city::store::Key,\n}\n";
        let tuple = "struct S(\n    #[cfg(test)]\n    Map<<X as Tr>::Out, crate::street::vm::Key>,\n    crate::city::store::Key,\n);\n";
        let first = "struct S {\n    #[cfg(test)]\n    seen: Map<Vec<<X as Tr>::Out>, crate::street::vm::Key>,\n    city: crate::city::store::Key,\n}\n";
        let samples = [
            ("a field", field, (4, "city::store::Key")),
            ("a field of a tuple struct", tuple, (4, "city::store::Key")),
            ("a field whose first argument holds it", first, (4, "city::store::Key")),
        ];
        let mut wrong = Vec::new();
        for (name, code, expected) in samples {
            let (found, expected) = (found(code), at(&[expected]));
            if found != expected {
                wrong.push(format!("{name}: found {found:?}, not {expected:?}"));
            }
        }
        assert!(wrong.is_empty(), "{} of {} samples:\n{}", wrong.len(), samples.len(), wrong.join("\n"));
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

    /// A sample's lines, each with the paths that break a rule on it. `SLICE` and `LAYER` stand for a slice and a layer.
    type Lines<'a> = &'a [(&'a str, &'a [&'a str])];

    /// The slices whose layers are held to the layer rules.
    const LAYERED_SLICES: &[&str] = &["street", "junction", "map"];

    /// The ways a file at the top of a layer takes the `LAYER` of its slice.
    const FROM_THE_TOP: Lines = &[
        ("use crate::SLICE::LAYER::Panel;", &["SLICE::LAYER::Panel"]),
        ("use crate::{SLICE::{RESOURCES, LAYER::{Panel, frame::Frame}}, shared::units::Units};", &["SLICE::LAYER::Panel", "SLICE::LAYER::frame::Frame"]),
        ("use crate::SLICE::LAYER::{self, frame};", &["SLICE::LAYER", "SLICE::LAYER::frame"]),
        ("use crate::SLICE::{", &[]),
        ("    LAYER::Panel,", &["SLICE::LAYER::Panel"]),
        ("    LAYER::{", &[]),
        ("        Frame,", &["SLICE::LAYER::Frame"]),
        ("    },", &[]),
        ("};", &[]),
        ("use crate::SLICE::LAYER::Panel as P;", &["SLICE::LAYER::Panel"]),
        ("use crate::SLICE::LAYER as other;", &["SLICE::LAYER"]),
        ("use crate::SLICE::{LAYER as again, RESOURCES};", &["SLICE::LAYER"]),
        ("use crate::SLICE::LAYER::*;", &["SLICE::LAYER::*"]),
        ("use crate::SLICE::{LAYER::*, RESOURCES};", &["SLICE::LAYER::*"]),
        ("fn f(p: &crate::SLICE::LAYER::Panel) { <crate::SLICE::LAYER::Frame as Default>::default(); }", &["SLICE::LAYER::Frame", "SLICE::LAYER::Panel"]),
        ("fn g() { for p in crate::SLICE::LAYER::PANELS {} vec![crate::SLICE::LAYER::open::<u32>(1)]; }", &["SLICE::LAYER::PANELS", "SLICE::LAYER::open"]),
        ("fn h() { view! { <crate::SLICE::LAYER::Panel on:click=move |_| crate::SLICE::LAYER::go()/> } }", &["SLICE::LAYER::Panel", "SLICE::LAYER::go"]),
        ("use super::LAYER::Panel;", &["SLICE::LAYER::Panel"]),
        ("use super::{LAYER::frame, RESOURCES};", &["SLICE::LAYER::frame"]),
        ("use self::super::LAYER;", &["SLICE::LAYER"]),
        ("use super::super::SLICE::LAYER::draw;", &["SLICE::LAYER::draw"]),
        ("mod inner { use super::super::LAYER::draw; }", &["SLICE::LAYER::draw"]),
        ("fn k() { super::LAYER::draw(); }", &["SLICE::LAYER::draw"]),
        ("pub use crate::SLICE::LAYER::Panel;", &["SLICE::LAYER::Panel"]),
        ("pub(crate) use super::LAYER::{Frame, draw};", &["SLICE::LAYER::Frame", "SLICE::LAYER::draw"]),
    ];

    /// The ways a deeper file of a layer takes the `LAYER` of its slice.
    const FROM_BELOW: Lines = &[
        ("use crate::SLICE::LAYER::Panel;", &["SLICE::LAYER::Panel"]),
        ("use super::super::LAYER::Panel;", &["SLICE::LAYER::Panel"]),
        ("use super::super::{LAYER::{Frame, draw as d}, RESOURCES};", &["SLICE::LAYER::Frame", "SLICE::LAYER::draw"]),
        ("use super::{super::LAYER, Rule};", &["SLICE::LAYER"]),
        ("use super::super::LAYER::*;", &["SLICE::LAYER::*"]),
        ("use super::super::super::SLICE::LAYER as other;", &["SLICE::LAYER"]),
        ("mod inner { use super::super::super::LAYER::draw; }", &["SLICE::LAYER::draw"]),
        ("fn f() { super::super::LAYER::draw(); }", &["SLICE::LAYER::draw"]),
        ("fn g() { view! { <super::super::LAYER::Panel/> } }", &["SLICE::LAYER::Panel"]),
        ("pub use super::super::LAYER::Panel;", &["SLICE::LAYER::Panel"]),
    ];

    /// The ways a file names the browser's crates, without Leptos.
    const BROWSER: Lines = &[
        ("use web_sys::Window;", &["web_sys::Window"]),
        ("use js_sys::Date;", &["js_sys::Date"]),
        ("use gloo::events;", &["gloo::events"]),
        ("use gloo_timers::callback::Timeout;", &["gloo_timers::callback::Timeout"]),
        ("use wasm_bindgen::prelude::*;", &["wasm_bindgen::prelude::*"]),
        ("use wasm_bindgen_futures::spawn_local;", &["wasm_bindgen_futures::spawn_local"]),
        ("use web_sys;", &["web_sys"]),
        ("use web_sys::{self, Document, console::log_1};", &["web_sys", "web_sys::Document", "web_sys::console::log_1"]),
        ("use {js_sys::Math, web_sys::Event};", &["js_sys::Math", "web_sys::Event"]),
        ("use web_sys::{", &[]),
        ("    Document,", &["web_sys::Document"]),
        ("};", &[]),
        ("use js_sys as js;", &["js_sys"]),
        ("use js_sys::Array as List;", &["js_sys::Array"]),
        ("use ::gloo::net;", &["gloo::net"]),
        ("use l::web_sys::Element;", &["web_sys::Element"]),
        ("pub(crate) use js_sys::Map;", &["js_sys::Map"]),
        ("fn f(e: &web_sys::Event) -> f64 { js_sys::Date::now() }", &["js_sys::Date::now", "web_sys::Event"]),
        ("fn g() { ::web_sys::window(); vec![gloo::net::get::<u32>(1)]; }", &["gloo::net::get", "web_sys::window"]),
        ("fn h() { for i in 0..js_sys::Math::floor(2.5) {} }", &["js_sys::Math::floor"]),
        ("#[wasm_bindgen] pub fn k() {}", &["wasm_bindgen"]),
        ("#[wasm_bindgen(js_name = m)] pub fn l() {}", &["wasm_bindgen"]),
        ("#[wasm_bindgen::prelude::wasm_bindgen] pub fn n() {}", &["wasm_bindgen::prelude::wasm_bindgen"]),
        ("#[cfg_attr(target_arch = \"wasm32\", wasm_bindgen)] pub fn o() {}", &["wasm_bindgen"]),
        ("use gloomy::Sky;", &[]),
        ("use crate::shared::web_sys::Ports;", &[]),
        ("fn p(ports: &Ports) { ports.web_sys().js_sys; }", &[]),
    ];

    /// `file` holding `code`, in a slice whose `mod.rs` declares the three layers, the other files holding nothing but
    /// the declaration of `file` where it is deeper than its layer's own file.
    fn sample(file: &str, code: &str) -> Sources {
        let (slice, rest) = file.split_once('/').unwrap();
        let mut files = tree(&[("lib.rs", &format!("mod {slice};")), (&format!("{slice}/mod.rs"), "mod model;\nmod vm;\nmod view;\n")]);
        for layer in ["model", "vm", "view"] {
            let deeper = rest.strip_prefix(&format!("{layer}/")).and_then(|name| name.strip_suffix(".rs")).filter(|name| *name != "mod");
            let own = if rest == format!("{layer}.rs") { file.to_string() } else { format!("{slice}/{layer}/mod.rs") };
            files.insert(own, deeper.map(|name| format!("mod {name};")).unwrap_or_default());
        }
        files.insert(file.to_string(), code.to_string());
        files
    }

    /// Holds that the rules break in `file` holding `lines`, with the paths beside each line and no other.
    #[track_caller]
    fn breaks(file: &str, lines: Lines) {
        breaks_as(file, lines, "SLICE", "LAYER");
    }

    /// `breaks`, with `slice` for `SLICE` and `layer` for `LAYER`.
    #[track_caller]
    fn breaks_as(file: &str, lines: Lines, slice: &str, layer: &str) {
        let fill = |text: &str| text.replace("SLICE", slice).replace("LAYER", layer);
        let file = fill(file);
        let code: String = lines.iter().map(|(text, _)| fill(text) + "\n").collect();
        let side = if file.split('/').nth(1).unwrap().starts_with("model") { "model" } else { "view-model" };
        let mut expected = Vec::new();
        for (i, (_, paths)) in lines.iter().enumerate() {
            expected.extend(paths.iter().map(|path| Break { used: used(&file, i + 1, &fill(path)), layer: side }));
        }
        expected.sort();
        assert_eq!(broken(&sample(&file, &code)), expected, "in {file}:\n{code}");
    }

    fn model_breaks(file: &str, path: &str) -> Break {
        Break { used: one(file, path), layer: "model" }
    }

    fn vm_breaks(file: &str, path: &str) -> Break {
        Break { used: one(file, path), layer: "view-model" }
    }

    #[test]
    fn a_model_that_uses_its_view_model_or_view_breaks_a_rule() {
        for slice in LAYERED_SLICES {
            for layer in ["vm", "view"] {
                breaks_as("SLICE/model/mod.rs", FROM_THE_TOP, slice, layer);
                breaks_as("SLICE/model.rs", FROM_THE_TOP, slice, layer);
                breaks_as("SLICE/model/rules.rs", FROM_BELOW, slice, layer);
            }
        }
    }

    #[test]
    fn a_view_model_that_uses_its_view_breaks_a_rule() {
        for slice in LAYERED_SLICES {
            breaks_as("SLICE/vm/mod.rs", FROM_THE_TOP, slice, "view");
            breaks_as("SLICE/vm.rs", FROM_THE_TOP, slice, "view");
            breaks_as("SLICE/vm/keys.rs", FROM_BELOW, slice, "view");
        }
    }

    #[test]
    fn a_layer_may_use_what_is_below_it() {
        breaks(
            "junction/model/rules.rs",
            &[
                ("use crate::junction::model::Junction;", &[]),
                ("use super::{Junction, geometry::*, view::sight, vm};", &[]),
                ("use super::super::model as own;", &[]),
                ("use crate::junction::viewer::Viewer;", &[]),
                ("use super::super::view::draw;", &["junction::view::draw"]),
            ],
        );
        breaks(
            "junction/vm/keys.rs",
            &[
                ("use crate::junction::model::{self, Junction};", &[]),
                ("use super::super::model::*;", &[]),
                ("use crate::junction::vm::{JunctionVm, text::*};", &[]),
                ("use super::{super::vm as own, watch::Watch};", &[]),
                ("use crate::junction::vm::view::Shown;", &[]),
                ("use super::view::Shown as S;", &[]),
                ("use crate::junction::view::Shown as T;", &["junction::view::Shown"]),
            ],
        );

        let above = tree(&[
            ("lib.rs", "mod junction;"),
            (
                "junction/mod.rs",
                "mod model;\nmod vm;\nmod view;\nuse view::Page;\nuse self::vm::JunctionVm;\npub use crate::junction::{model::Junction, view::*};\n",
            ),
            ("junction/model/mod.rs", "use crate::junction::vm::JunctionVm;"),
            ("junction/vm/mod.rs", ""),
            ("junction/view/mod.rs", "use crate::junction::{model::Junction, view::Panel, vm::JunctionVm};\nuse super::*;\nuse crate::junction;\n"),
        ]);
        assert_eq!(broken(&above), [model_breaks("junction/model/mod.rs", "junction::vm::JunctionVm")]);

        breaks(
            "share/model.rs",
            &[
                ("use crate::share::vm::ShareVm;", &[]),
                ("use super::view::Panel;", &[]),
                ("use super::*;", &[]),
                ("use crate::share;", &[]),
                ("use web_sys::Window;", &["web_sys::Window"]),
            ],
        );
        breaks("share/vm.rs", &[("use crate::share::view::Panel;", &[]), ("use super::*;", &[]), ("use web_sys::Window;", &["web_sys::Window"])]);
    }

    #[test]
    fn taking_the_slice_whole_from_a_layer_breaks_a_rule() {
        const TOP: Lines = &[
            ("use crate::SLICE;", &["SLICE"]),
            ("use crate::SLICE as j;", &["SLICE"]),
            ("use crate::SLICE::{self};", &["SLICE"]),
            ("use crate::SLICE::{self as k, RESOURCES};", &["SLICE"]),
            ("use crate::{SLICE, shared::units::Units};", &["SLICE"]),
            ("use crate::SLICE::*;", &["SLICE::*"]),
            ("pub(in crate::SLICE) fn f() {}", &[]),
            ("pub(in super::super::SLICE) fn g() {}", &[]),
            ("use crate::SLICE::RESOURCES;", &[]),
            ("fn h() { crate::SLICE::open(); super::open(); }", &[]),
            ("use super::super::SLICE as l;", &["SLICE"]),
            ("use super as m;", &["SLICE"]),
            ("use super::{self as n};", &["SLICE"]),
            ("use super::*;", &["SLICE::*"]),
            ("pub(in crate::SLICE) use super::{*, RESOURCES};", &["SLICE::*"]),
        ];
        const BELOW: Lines = &[
            ("use crate::SLICE as j;", &["SLICE"]),
            ("use super::super as k;", &["SLICE"]),
            ("use super::super::{self as l};", &["SLICE"]),
            ("use super::super::*;", &["SLICE::*"]),
            ("use super::{super::*, Rule};", &["SLICE::*"]),
            ("pub(in super::super) fn f() {}", &[]),
            ("pub(in crate::SLICE) struct S;", &[]),
            ("pub(super) fn g() {}", &[]),
            ("use super::*;", &[]),
            ("use super::super::RESOURCES;", &[]),
        ];
        for slice in LAYERED_SLICES {
            for layer in ["model", "vm"] {
                breaks_as("SLICE/LAYER/mod.rs", TOP, slice, layer);
                breaks_as("SLICE/LAYER/rules.rs", BELOW, slice, layer);
            }
        }
    }

    #[test]
    fn the_report_may_draw_other_slices_figures() {
        breaks(
            "map/vm/report.rs",
            &[
                ("use crate::street::view::drawing;", &[]),
                ("use crate::junction::view::figure;", &[]),
                ("fn f() { crate::street::view::drawing::draw(); }", &[]),
                ("use crate::{junction, street::*};", &[]),
                ("use crate::map::view::details;", &["map::view::details"]),
            ],
        );
    }

    #[test]
    fn a_model_that_names_leptos_or_the_browser_breaks_a_rule() {
        const LEPTOS: Lines = &[
            ("use leptos::prelude::RwSignal;", &["leptos::prelude::RwSignal"]),
            ("use leptos::{prelude::*, web_sys::Window};", &["leptos::prelude::*", "leptos::web_sys::Window"]),
            ("use {leptos::prelude::*, web_sys::Window};", &["leptos::prelude::*", "web_sys::Window"]),
            ("use leptos as l;", &["leptos"]),
            ("use leptos::prelude::Memo as M;", &["leptos::prelude::Memo"]),
            ("use leptos_dom::helpers;", &["leptos_dom::helpers"]),
            ("fn f(n: ::leptos::prelude::RwSignal<u32>) { leptos::prelude::signal(0); }", &["leptos::prelude::RwSignal", "leptos::prelude::signal"]),
            ("fn g() { leptos::prelude::document(); leptos::view! { <p/> } }", &["leptos::prelude::document", "leptos::view"]),
            ("use leptosphere::Crust;", &[]),
            ("use crate::shared::leptos::Glue;", &[]),
            ("fn h() { document(); window(); }", &[]),
        ];
        for file in ["junction/model/mod.rs", "junction/model/rules.rs", "share/model.rs"] {
            breaks(file, LEPTOS);
            breaks(file, BROWSER);
        }
    }

    #[test]
    fn a_view_model_that_names_the_browser_breaks_a_rule() {
        const THROUGH_LEPTOS: Lines = &[
            ("use leptos::web_sys::HtmlElement;", &["leptos::web_sys::HtmlElement"]),
            ("use leptos::wasm_bindgen::JsCast;", &["leptos::wasm_bindgen::JsCast"]),
            ("use leptos::{prelude::*, web_sys::Window};", &["leptos::web_sys::Window"]),
            ("use {leptos::prelude::*, web_sys::Window};", &["web_sys::Window"]),
            ("use leptos::web_sys as w;", &["leptos::web_sys"]),
            ("fn f() { leptos::leptos_dom::helpers::set_timeout(); leptos::reexport::js_sys::eval(); }", &["leptos::reexport::js_sys::eval"]),
            ("use leptos as l;", &[]),
            ("use leptos_dom::helpers;", &[]),
        ];
        for file in ["junction/vm/mod.rs", "junction/vm/keys.rs", "share/vm.rs"] {
            breaks(file, THROUGH_LEPTOS);
            breaks(file, BROWSER);
        }
    }

    #[test]
    fn leptos_document_and_window_in_a_view_model_break_a_rule() {
        const CALLS: Lines = &[
            ("fn f() { leptos::prelude::document(); }", &["leptos::prelude::document"]),
            ("fn g() { leptos::leptos_dom::helpers::window(); }", &["leptos::leptos_dom::helpers::window"]),
            ("fn h() { document().body(); }", &["document"]),
            ("fn k() -> String { let at = window().location(); format!(\"{:?}\", window().name()) }", &["window"]),
            ("fn l() { if let Some(body) = document().body() { show(&body, || window()); } }", &["document", "window"]),
        ];
        const IMPORTS: Lines = &[
            ("use leptos::prelude::{document, RwSignal};", &["leptos::prelude::document"]),
            ("use leptos::prelude::window as w;", &["leptos::prelude::window"]),
            ("use leptos::{prelude::*, leptos_dom::helpers::{window, set_timeout}};", &["leptos::leptos_dom::helpers::window"]),
            ("use leptos_dom::helpers::document as d;", &["leptos_dom::helpers::document"]),
            ("use leptos::prelude::*;", &[]),
            ("fn f() { document().body(); }", &["document"]),
            ("fn g() { window().location(); }", &["window"]),
        ];
        for file in ["junction/vm/mod.rs", "junction/vm/keys.rs", "share/vm.rs"] {
            breaks(file, CALLS);
            breaks(file, IMPORTS);
        }
    }

    #[test]
    fn a_document_or_window_of_the_code_itself_breaks_no_rule() {
        breaks(
            "junction/vm/mod.rs",
            &[
                ("use leptos::prelude::*;", &[]),
                ("fn f(page: &Page) { page.document(); }", &[]),
                ("fn g(&self) { self.window().open(); }", &[]),
                ("impl Page { fn document(&self) {} fn window() {} }", &[]),
                ("fn h() { let window = 3; f(window); }", &[]),
                ("fn k() { f(|window| (window + 1)); }", &[]),
                ("fn l() { window(1); }", &[]),
                ("fn m() { Self::document(); }", &[]),
                ("fn n() { crate::shared::page::document(); }", &[]),
                ("fn o() { storage::window(); }", &[]),
                ("fn p() { super::document(); }", &[]),
                ("use crate::shared::page::{document, window as w};", &[]),
                ("fn q() { document(); }", &["document"]),
            ],
        );
    }

    #[test]
    fn a_view_model_may_use_leptos_signals() {
        breaks(
            "junction/vm/mod.rs",
            &[
                ("use leptos::prelude::*;", &[]),
                ("use leptos::prelude::{RwSignal, Memo};", &[]),
                ("fn f() { leptos::prelude::signal(0); }", &[]),
                ("fn g() { leptos::task::spawn_local(); }", &[]),
            ],
        );
        breaks(
            "junction/model/mod.rs",
            &[
                ("use leptos::prelude::*;", &["leptos::prelude::*"]),
                ("use leptos::prelude::{RwSignal, Memo};", &["leptos::prelude::Memo", "leptos::prelude::RwSignal"]),
                ("fn f() { leptos::prelude::signal(0); }", &["leptos::prelude::signal"]),
                ("fn g() { leptos::task::spawn_local(); }", &["leptos::task::spawn_local"]),
            ],
        );
    }

    #[test]
    fn the_layer_rules_read_production_code_only() {
        breaks(
            "junction/vm/mod.rs",
            &[
                ("//! crate::junction::view::Panel, web_sys::Window", &[]),
                ("/// See [`crate::junction::view`], `web_sys::Window` and `document()`.", &[]),
                ("fn f() { /* web_sys::window() */ let a = \"crate::junction::view document()\"; } // document()", &[]),
                ("fn g() { r#\"js_sys::Date\"#; document(); }", &["document"]),
                ("#[cfg(test)]", &[]),
                ("use crate::junction::view::{Panel, draw};", &[]),
                ("#[cfg(test)]", &[]),
                ("fn h() { web_sys::window(); document(); }", &[]),
                ("#[cfg(test)]", &[]),
                ("mod fixtures { use super::super::view::Panel; use js_sys::Date; }", &[]),
                ("use super::view::Panel;", &["junction::view::Panel"]),
                ("fn k() {", &[]),
                ("    #[cfg(test)]", &[]),
                ("    document().body();", &[]),
                ("    js_sys::Date::now();", &["js_sys::Date::now"]),
                ("}", &[]),
                ("struct Shown {", &[]),
                ("    #[cfg(test)]", &[]),
                ("    seen: Vec<web_sys::Window>,", &[]),
                ("    kept: crate::junction::view::Kept,", &["junction::view::Kept"]),
                ("}", &[]),
                ("#[cfg(target_arch = \"wasm32\")]", &[]),
                ("use web_sys::Event;", &["web_sys::Event"]),
                ("#[cfg(not(test))]", &[]),
                ("fn l() { crate::junction::view::draw(); window(); }", &["junction::view::draw", "window"]),
            ],
        );

        let code = "use crate::junction::view::Panel;\nuse leptos::prelude::*;\n";
        let by_declaration = tree(&[
            ("lib.rs", "mod junction;"),
            ("junction/mod.rs", "mod model;"),
            ("junction/model/mod.rs", "#[cfg(test)]\nmod fixtures;\nmod tests;\n"),
            ("junction/model/fixtures.rs", code),
            ("junction/model/tests.rs", code),
        ]);
        let expected = [(1, "junction::view::Panel"), (2, "leptos::prelude::*")]
            .map(|(line, path)| Break { used: used("junction/model/tests.rs", line, path), layer: "model" });
        assert_eq!(broken(&by_declaration), expected);
    }

    #[test]
    fn only_a_slices_model_and_view_model_are_held_to_the_library_rules() {
        let code = "use leptos::prelude::*; use web_sys::Window; fn f() { document(); }\n";
        let declaring = |modules: &str| format!("{modules}\n{code}");
        let sources = tree(&[
            ("lib.rs", &declaring("mod junction; mod share; mod shared;")),
            ("junction/mod.rs", &declaring("mod view;")),
            ("junction/view/mod.rs", &declaring("mod model; mod vm;")),
            ("junction/view/model.rs", code),
            ("junction/view/vm.rs", code),
            ("share/mod.rs", &declaring("mod models; mod store; mod view; mod vm;")),
            ("share/models.rs", code),
            ("share/store.rs", code),
            ("share/view.rs", code),
            ("share/vm.rs", code),
            ("shared/mod.rs", &declaring("mod model; mod vm;")),
            ("shared/model.rs", code),
            ("shared/vm.rs", code),
        ]);
        assert_eq!(broken(&sources), [vm_breaks("share/vm.rs", "document"), vm_breaks("share/vm.rs", "web_sys::Window")]);
    }

    #[test]
    fn a_break_is_refused_with_its_file_line_path_and_layer() {
        let sources = tree(&[
            ("lib.rs", "mod junction;"),
            ("junction/mod.rs", "mod model;\nmod vm;\n"),
            (
                "junction/vm/mod.rs",
                "mod keys;\nuse web_sys::Window;\nuse super::view::Panel;\nuse leptos::{prelude::*, web_sys::Event};\nfn f() { document(); document(); leptos::prelude::window(); }\n",
            ),
            ("junction/vm/keys.rs", "use super::super::*;\n"),
            (
                "junction/model/mod.rs",
                "use crate::junction::{vm::JunctionVm, view::{draw, draw as d}};\nuse super::super::junction;\nuse leptos::prelude::*;\n\nuse js_sys::Date;\n",
            ),
        ]);
        let expected = [
            "src/junction/model/mod.rs:1: uses junction::view::draw, which a model may not use",
            "src/junction/model/mod.rs:1: uses junction::vm::JunctionVm, which a model may not use",
            "src/junction/model/mod.rs:2: uses junction, which a model may not use",
            "src/junction/model/mod.rs:3: uses leptos::prelude::*, which a model may not use",
            "src/junction/model/mod.rs:5: uses js_sys::Date, which a model may not use",
            "src/junction/vm/keys.rs:1: uses junction::*, which a view-model may not use",
            "src/junction/vm/mod.rs:2: uses web_sys::Window, which a view-model may not use",
            "src/junction/vm/mod.rs:3: uses junction::view::Panel, which a view-model may not use",
            "src/junction/vm/mod.rs:4: uses leptos::web_sys::Event, which a view-model may not use",
            "src/junction/vm/mod.rs:5: uses document, which a view-model may not use",
            "src/junction/vm/mod.rs:5: uses leptos::prelude::window, which a view-model may not use",
        ];
        assert_eq!(unexcepted(&broken(&sources), &[]), expected);
    }

    #[test]
    fn an_exception_lets_its_break_pass_and_no_other() {
        let exceptions = [
            Entry { by: &["junction/vm/mod.rs"], uses: &["web_sys", "junction::view"], why: "A sample." },
            Entry { by: &["map"], uses: &["leptos::prelude", "js_sys::Date"], why: "A sample." },
        ];
        let covered = [
            vm_breaks("junction/vm/mod.rs", "web_sys"),
            vm_breaks("junction/vm/mod.rs", "web_sys::Window"),
            vm_breaks("junction/vm/mod.rs", "junction::view"),
            vm_breaks("junction/vm/mod.rs", "junction::view::Panel"),
            model_breaks("map/model/mod.rs", "leptos::prelude::*"),
            vm_breaks("map/vm/camera.rs", "js_sys::Date::now"),
        ];
        assert_eq!(unexcepted(&covered, &exceptions), Vec::<String>::new());

        let others = [
            model_breaks("junction/model/mod.rs", "web_sys::Window"),
            model_breaks("junction/model/mod.rs", "junction::view::Panel"),
            model_breaks("junction/model/mod.rs", "leptos::prelude::*"),
            vm_breaks("junction/vm/keys.rs", "web_sys::Window"),
            vm_breaks("junction/vm/mod.rs", "junction"),
            vm_breaks("junction/vm/mod.rs", "junction::viewer::Viewer"),
            vm_breaks("junction/vm/mod.rs", "web_sys_more::Window"),
            vm_breaks("junction/vm/mod.rs", "leptos::web_sys::Window"),
            model_breaks("map/model/mod.rs", "leptos"),
            vm_breaks("map/vm/camera.rs", "js_sys"),
            vm_breaks("map/vm/camera.rs", "web_sys::Window"),
        ];
        for other in others {
            let message = format!("src/{}:1: uses {}, which a {} may not use", other.used.file, other.used.path, other.layer);
            assert_eq!(unexcepted(&[covered[0].clone(), other], &exceptions), [message]);
        }
    }

    #[test]
    fn an_exception_nothing_uses_or_without_a_reason_is_faulty() {
        let breaks = broken(&tree(&[
            ("lib.rs", "mod junction;\nmod share;\n"),
            ("junction/mod.rs", "mod vm;"),
            ("junction/vm/mod.rs", "mod keys;\nuse leptos::prelude::*;\nuse web_sys::Window;\n"),
            ("junction/vm/keys.rs", "use super::super::view::Panel;"),
            ("share/mod.rs", "mod model;"),
            ("share/model.rs", "use leptos::prelude::*;"),
        ]));
        let sound = [
            Entry { by: &["junction/vm/mod.rs"], uses: &["web_sys"], why: "A sample." },
            Entry { by: &["junction"], uses: &["junction::view", "web_sys::Window"], why: "A sample." },
            Entry { by: &["share/model.rs"], uses: &["leptos"], why: "A sample." },
        ];
        assert_eq!(faulty(&breaks, &sound), Vec::<String>::new());

        let unsound = [
            Entry { by: &["share"], uses: &["leptos::prelude"], why: " \n" },
            Entry { by: &["junction/vm/mod.rs", "junction/vm/keys.rs"], uses: &["web_sys", "junction::view"], why: "A sample." },
            Entry { by: &["junction/vm/mod.rs"], uses: &["leptos::prelude"], why: "A sample." },
            Entry { by: &["map", "street"], uses: &["js_sys"], why: "" },
        ];
        let expected = [
            "EXCEPTIONS: nothing in junction/vm/mod.rs breaks a rule with junction::view",
            "EXCEPTIONS: nothing in junction/vm/keys.rs breaks a rule with web_sys",
            "EXCEPTIONS: nothing in junction/vm/mod.rs breaks a rule with leptos::prelude",
            "EXCEPTIONS: nothing in map breaks a rule with js_sys",
            "EXCEPTIONS: nothing in street breaks a rule with js_sys",
            "EXCEPTIONS: no reason for what [\"share\"] may use: [\"leptos::prelude\"]",
            "EXCEPTIONS: no reason for what [\"map\", \"street\"] may use: [\"js_sys\"]",
        ];
        assert_eq!(faulty(&breaks, &unsound), expected);
    }

    #[test]
    fn production_code_keeps_to_its_layer() {
        let breaks = broken(&sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src")));
        let refused = unexcepted(&breaks, EXCEPTIONS);
        assert!(refused.is_empty(), "{}", refused.join("\n"));
        let faulty = faulty(&breaks, EXCEPTIONS);
        assert!(faulty.is_empty(), "{}", faulty.join("\n"));
    }
}
