//! A migrated file says nothing in words of its own: every sentence comes from a message.
//!
//! The files are read with Rust's own lexer (`proc_macro2`), so char literals, raw and multi-line
//! strings and comments are what the compiler sees, and `view! { … }` is walked like any group.

use std::str::FromStr;

use proc_macro2::{Delimiter, TokenStream, TokenTree};

const MIGRATED: &[(&str, &str)] = &[
    // Files are added here as their task completes.
    ("src/street/text.rs", include_str!("street/text.rs")),
    ("src/street/notes.rs", include_str!("street/notes.rs")),
    ("src/street/page.rs", include_str!("street/page.rs")),
    ("src/street/view.rs", include_str!("street/view.rs")),
    ("src/street/measures.rs", include_str!("street/measures.rs")),
    ("src/street/inspector.rs", include_str!("street/inspector.rs")),
    ("src/street/svg.rs", include_str!("street/svg.rs")),
    ("src/street/model.rs", include_str!("street/model.rs")),
    ("src/shared/said.rs", include_str!("shared/said.rs")),
    ("src/street/vm.rs", include_str!("street/vm.rs")),
    ("src/street/keys.rs", include_str!("street/keys.rs")),
    ("src/street/watch.rs", include_str!("street/watch.rs")),
    ("src/street/mod.rs", include_str!("street/mod.rs")),
    ("src/shell/vm.rs", include_str!("shell/vm.rs")),
    ("src/shell/view.rs", include_str!("shell/view.rs")),
    ("src/shell/mod.rs", include_str!("shell/mod.rs")),
];

/// Attributes whose value is text a person reads (also as `attr:<name>`).
const TEXT_ATTRS: &[&str] = &["aria-label", "title", "placeholder", "alt"];

/// Literals that are keys, not words.
const ALLOWED: &[&str] = &[
    "Escape",
    "Enter",
    "ArrowDown",
    "ArrowUp",
    "ArrowLeft",
    "ArrowRight",
    "Home",
    "End",
    "Delete",
    "Backspace",
    "Tab",
    "Shift",
    "Control",
    "Meta",
    "PageUp",
    "PageDown",
];

/// Literals that look like words and are not, each with the reason. Empty unless needed.
const ALLOWED_LITERALS: &[(&str, &str)] = &[
    (" fresh", "a CSS class appended to a class list"),
    (" lifted", "a CSS class appended to a class list"),
    (" active", "a CSS class appended to a class list"),
    (" t-blue", "a CSS class appended to a class list"),
    ("(pointer: coarse)", "a media query"),
    ("(max-width: 1100px)", "a media query"),
    ("(prefers-reduced-motion: reduce)", "a media query"),
    ("input, select, textarea", "a CSS selector"),
    ("the page has no #", "a panic message about the page's markup, for the developer"),
    (".notes .tab", "a CSS selector"),
    ("(prefers-color-scheme: dark)", "a media query"),
    ("(min-width: 1360px)", "a media query"),
    ("recipes use catalogue kinds", "a panic message about the code, never shown"),
    ("catalogue kind", "a panic message about the code, never shown"),
];

fn is_path_data(s: &str) -> bool {
    s.chars().all(|c| "MmLlHhVvCcSsQqTtAaZz0123456789 .,-".contains(c))
}

fn looks_like_words(s: &str) -> bool {
    if ALLOWED.contains(&s) || ALLOWED_LITERALS.iter().any(|(l, _)| *l == s) || is_path_data(s) {
        return false;
    }
    let has_word = |t: &str| t.split(|c: char| !c.is_ascii_alphabetic()).any(|w| w.len() >= 2 && w.chars().all(|c| c.is_ascii_alphabetic()));
    let mut chars = s.chars();
    let capitalised = s.len() >= 2
        && chars.next().is_some_and(|c| c.is_ascii_uppercase())
        && chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_alphabetic());
    (s.contains(' ') && has_word(s)) || capitalised
}

/// The text of a string literal as written (escapes left alone), or `None` for any other literal.
fn string_text(lit: &str) -> Option<&str> {
    let raw = lit.strip_prefix('r').map(|r| r.trim_start_matches('#'));
    match raw {
        Some(r) => {
            let hashes = lit.len() - 1 - r.len();
            r.strip_suffix(&"#".repeat(hashes))?.strip_prefix('"')?.strip_suffix('"')
        }
        None => lit.strip_prefix('"')?.strip_suffix('"'),
    }
}

/// What a literal says in words: markup tags, `name="value"` attributes and `{…}` placeholders are not words, what lies between the tags is.
fn pieces(s: &str) -> Vec<String> {
    // A backslash at the end of a line joins it to the next, without that line's indent.
    let s = s.split("\\\n").enumerate().map(|(n, p)| if n == 0 { p } else { p.trim_start() }).collect::<String>().replace("\\\"", "\"");
    let mut no_tags = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match (c, in_tag) {
            ('<', false) if !s.contains("< ") => {
                in_tag = true;
                no_tags.push('\n');
            }
            ('>', true) => {
                in_tag = false;
                no_tags.push('\n');
            }
            (_, false) => no_tags.push(c),
            _ => {}
        }
    }
    let mut no_attrs = String::new();
    let mut rest = no_tags.as_str();
    while let Some(eq) = rest.find("=\"") {
        let head = &rest[..eq];
        let name = head.rfind(|c: char| !(c.is_ascii_alphanumeric() || "-_:".contains(c))).map_or(0, |k| k + 1);
        no_attrs.push_str(&head[..name]);
        let value = &rest[eq + 2..];
        rest = value.find('"').map_or("", |q| &value[q + 1..]);
    }
    no_attrs.push_str(rest);
    let mut bare = String::new();
    let mut depth = 0;
    for c in no_attrs.chars() {
        match c {
            '{' => depth += 1,
            '}' if depth > 0 => depth -= 1,
            _ if depth == 0 => bare.push(c),
            _ => {}
        }
    }
    bare.split('\n').filter(|p| !p.trim().is_empty()).map(str::to_string).collect()
}

fn is_cfg_test(attr: &TokenTree) -> bool {
    let TokenTree::Group(g) = attr else { return false };
    let t: Vec<TokenTree> = g.stream().into_iter().collect();
    matches!(t.as_slice(), [TokenTree::Ident(c), TokenTree::Group(a)]
        if c == "cfg" && a.stream().to_string() == "test")
}

fn is_punct(t: &TokenTree, c: char) -> bool {
    matches!(t, TokenTree::Punct(p) if p.as_char() == c)
}

/// Whether `a` ends where `b` starts, with nothing between.
fn touches(a: &TokenTree, b: &TokenTree) -> bool {
    a.span().end().line == b.span().start().line && a.span().end().column == b.span().start().column
}

/// Where the name of an attribute begins, for the `=` at `eq`: back over identifiers, `-` and `:`. `None` when there is no name.
fn name_start(t: &[TokenTree], eq: usize) -> Option<usize> {
    let mut s = eq;
    while s > 0 && (matches!(t[s - 1], TokenTree::Ident(_)) || is_punct(&t[s - 1], '-') || is_punct(&t[s - 1], ':')) && (s == eq || touches(&t[s - 1], &t[s])) {
        s -= 1;
    }
    (s < eq).then_some(s)
}

fn attribute_name(t: &[TokenTree], start: usize, eq: usize) -> String {
    t[start..eq].iter().map(|x| x.to_string()).collect()
}

/// Whether the value of an attribute is scanned: text a person reads, or code (an event handler, a property, a directive).
fn is_scanned_attr(name: &str) -> bool {
    TEXT_ATTRS.contains(&name.strip_prefix("attr:").unwrap_or(name))
        || ["on:", "prop:", "use:", "bind:", "let:"].iter().any(|p| name.starts_with(p))
        || name == "ref"
        || name == "node_ref"
}

/// Whether the `=` at `i` is an attribute's: it touches the value after it and the name before it.
fn attribute_at(t: &[TokenTree], i: usize) -> Option<String> {
    let value = t.get(i + 1)?;
    if !is_punct(&t[i], '=')
        || !touches(&t[i], value)
        || (i > 0 && matches!(&t[i - 1], TokenTree::Punct(_)) && !is_punct(&t[i - 1], '-') && !is_punct(&t[i - 1], ':'))
    {
        return None;
    }
    let start = name_start(t, i)?;
    touches(&t[i - 1], &t[i]).then(|| attribute_name(t, start, i))
}

/// Where the value of the non-text attribute whose `=` is at `eq` ends: one group or literal, else up to the next attribute or `>`.
fn value_end(t: &[TokenTree], eq: usize) -> usize {
    if matches!(t[eq + 1], TokenTree::Group(_) | TokenTree::Literal(_)) {
        return eq + 2;
    }
    let mut j = eq + 1;
    while j < t.len() {
        if attribute_at(t, j).is_some() {
            return name_start(t, j).unwrap_or(j);
        }
        if is_punct(&t[j], '>') && !is_punct(&t[j - 1], '=') && !is_punct(&t[j - 1], '-') {
            return j;
        }
        j += 1;
    }
    t.len()
}

/// Calls `visit(line, text)` for each string literal in code that is not a test or an attribute that is not text.
fn walk(stream: TokenStream, visit: &mut dyn FnMut(usize, &str)) {
    let t: Vec<TokenTree> = stream.into_iter().collect();
    let mut i = 0;
    while i < t.len() {
        // An attribute (and so a doc comment) is not text; `#[cfg(test)]` also takes the item after it.
        if is_punct(&t[i], '#') {
            let mut j = i + 1;
            if t.get(j).is_some_and(|x| is_punct(x, '!')) {
                j += 1;
            }
            if let Some(attr @ TokenTree::Group(_)) = t.get(j).filter(|x| matches!(x, TokenTree::Group(g) if g.delimiter() == Delimiter::Bracket)) {
                i = j + 1;
                if is_cfg_test(attr) {
                    while i < t.len() && !is_punct(&t[i], ';') && !matches!(&t[i], TokenTree::Group(b) if b.delimiter() == Delimiter::Brace) {
                        i += 1;
                    }
                    i += 1;
                }
                continue;
            }
        }
        if let Some(name) = attribute_at(&t, i) {
            if !is_scanned_attr(&name) {
                i = value_end(&t, i);
                continue;
            }
        }
        match &t[i] {
            TokenTree::Group(g) => walk(g.stream(), visit),
            TokenTree::Literal(l) => {
                let text = l.to_string();
                if let Some(s) = string_text(&text) {
                    visit(l.span().start().line, s);
                }
            }
            _ => {}
        }
        i += 1;
    }
}

/// The text still in `src` as `(line, literal)`.
fn offenders(src: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    match TokenStream::from_str(src) {
        Ok(stream) => walk(stream, &mut |line, s| out.extend(pieces(s).into_iter().filter(|p| looks_like_words(p)).map(|p| (line, p)))),
        Err(e) => out.push((0, format!("does not lex: {e}"))),
    }
    out
}

#[test]
fn migrated_files_have_no_hardcoded_text() {
    let mut found = Vec::new();
    for (path, src) in MIGRATED {
        found.extend(offenders(src).into_iter().map(|(line, lit)| format!("{path}:{line}: {lit:?}")));
    }
    assert!(found.is_empty(), "text still in the code:\n{}", found.join("\n"));
}

// ---- the keys ----
//
// A mistyped message id does not fail: the key itself is shown. So every string literal in a migrated file
// that is shaped like a message id (kebab-case with at least one hyphen: `measure-refused`) must be a
// message in BOTH languages. The net is wide on purpose, any such literal and not only the argument of a
// call, because keys also sit in tables and consts and in `Said::new`/`Arg::Msg` and `plain(..)` calls; the
// price is the few literals of that shape that are not messages, each listed in `NOT_MESSAGE_IDS` with its
// reason. (A key built at run time, `format!("region-{}", id)`, is out of the net's sight; the tests of
// the slice that builds it cover it.)

/// Whether `s` is shaped like a message id: lower-case kebab-case with at least one hyphen.
fn is_message_id(s: &str) -> bool {
    let word = |w: &str| !w.is_empty() && w.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    let mut parts = s.split('-');
    let first = parts.next().unwrap_or("");
    let rest: Vec<&str> = parts.collect();
    word(first) && first.starts_with(|c: char| c.is_ascii_lowercase()) && !rest.is_empty() && rest.iter().all(|w| word(w))
}

/// Literals shaped like a message id that are not one, each with the reason.
const NOT_MESSAGE_IDS: &[(&str, &str)] = &[
    ("data-notes", "the name of a data- attribute on the document element"),
    ("data-inspector", "the name of a data- attribute on the document element"),
    ("data-welcome", "the name of a data- attribute on the document element"),
    ("data-role", "the name of a data- attribute of an SVG handle"),
    ("data-uid", "the name of a data- attribute of an SVG handle"),
    ("data-i", "the name of a data- attribute of an SVG handle"),
    ("cityloom-welcome", "a storage key"),
    ("notes-toggle", "the id of an element of the page"),
    ("inspector-toggle", "the id of an element of the page"),
    ("t-checks", "the id of a tab of the page"),
    ("i-h-dir", "the id of an inspector heading"),
    ("i-h-curb", "the id of an inspector heading"),
    ("m-planted", "a CSS class of the drawing"),
    ("dim-warn", "a CSS class of the drawing"),
    ("t-warn", "a CSS class of the drawing"),
    ("t-soft", "a CSS class of the drawing"),
    ("bar-w", "a CSS class of the drawing"),
    ("bar-b", "a CSS class of the drawing"),
    ("title-block", "the id of an element of the street page"),
    ("fit-slot", "the id of an element of the street page"),
    ("fit-checks-slot", "the id of an element of the street page"),
    ("checks-n-slot", "the id of an element of the street page"),
    ("clock-slot", "the id of an element of the street page"),
    ("time-note", "the id of an element of the street page"),
    ("welcome-slot", "the id of an element of the street page"),
    ("account-btn", "the id of an element every page shares"),
    ("account-menu", "the id of an element every page shares"),
    ("cityloom-theme", "a storage key"),
    ("cityloom-inspector", "a storage key"),
    ("cityloom-notes", "a storage key"),
    ("cityloom-notes-tab", "a storage key"),
    ("aria-label", "the name of an attribute"),
    ("aria-pressed", "the name of an attribute"),
    ("aria-expanded", "the name of an attribute"),
    ("aria-selected", "the name of an attribute"),
    ("aria-controls", "the name of an attribute"),
    ("data-i18n", "the name of an attribute"),
    ("data-lang", "the name of an attribute"),
    ("data-unit", "the name of an attribute"),
    ("data-theme-set", "the name of an attribute"),
    ("data-theme", "the name of an attribute"),
];

/// The message-id-shaped literals in `src` as `(line, literal)`.
fn message_ids(src: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    match TokenStream::from_str(src) {
        Ok(stream) => walk(stream, &mut |line, s| {
            if is_message_id(s) && !NOT_MESSAGE_IDS.iter().any(|(l, _)| *l == s) {
                out.push((line, s.to_string()));
            }
        }),
        Err(e) => out.push((0, format!("does not lex: {e}"))),
    }
    out
}

/// The ids in `src` that are not a message in `locale`.
fn unknown_ids(src: &str, i18n: &crate::shared::i18n::I18n, locale: crate::shared::i18n::Locale) -> Vec<(usize, String)> {
    message_ids(src).into_iter().filter(|(_, id)| !i18n.has_message(id, locale)).collect()
}

#[test]
fn every_message_id_used_in_a_migrated_file_exists_in_both_languages() {
    use crate::shared::i18n::Locale;
    let mut found = Vec::new();
    // The scan must be seeing the keys (a scan that finds none passes for nothing).
    let seen: usize = MIGRATED.iter().map(|(_, src)| message_ids(src).len()).sum();
    assert!(seen > 150, "only {seen} message ids seen in the migrated files");
    for locale in [Locale::En, Locale::FrCa] {
        let i18n = crate::i18n_for(locale);
        for (path, src) in MIGRATED {
            found.extend(unknown_ids(src, &i18n, locale).into_iter().map(|(line, id)| format!("{path}:{line}: {id:?} is not a message in {}", locale.tag())));
        }
    }
    assert!(found.is_empty(), "keys that are not messages:\n{}", found.join("\n"));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(src: &str) -> Vec<String> {
        offenders(src).into_iter().map(|(_, l)| l).collect()
    }

    #[test]
    fn it_finds_words_in_text_nodes_format_strings_and_calls() {
        assert_eq!(words("view! { <th scope=\"col\">\"Use\"</th> }"), vec!["Use"]);
        assert_eq!(words("let a = \"Every metre\";"), vec!["Every metre"]);
        assert_eq!(words("let a = format!(\"{} left to use\", n);"), vec![" left to use"]);
        assert_eq!(words("let a = format!(\"{n} checks fail\");"), vec![" checks fail"]);
        assert_eq!(words("let a = \"Arrange\"; let b = \"Add\"; let c = \"Bus\"; let d = \"Max\"; let e = \"Set\";").len(), 5);
        assert_eq!(words("x.set_attribute(\"title\", \"Some text\");"), vec!["Some text"]);
        assert_eq!(words("let b = \"btn quiet\";"), vec!["btn quiet"]);
        assert_eq!(words("<i aria-label=\"Close it\">"), vec!["Close it"]);
        assert_eq!(words("<i attr:title=\"Close it\">"), vec!["Close it"]);
    }

    #[test]
    fn it_scans_event_handlers_which_are_code() {
        assert_eq!(words("view! { <button on:click=move |_| announce(\"Saved\")>\"Go\"</button> }"), vec!["Saved", "Go"]);
        assert_eq!(words("view! { <i on:keydown=move |e| { if e.key() == \"Escape\" { say(\"Closed now\") } }></i> }"), vec!["Closed now"]);
        assert_eq!(words("view! { <i prop:value=move || \"Some text\" use:thing=\"Other text\" bind:x=\"More text\"></i> }").len(), 3);
        let quiet = [
            "view! { <i class=move || if on() { \"btn on\" } else { \"btn\" }></i> }",
            "view! { <i id=\"t-checks\"></i> }",
            "view! { <i on:click=move |_| set_open.set(true)></i> }",
        ];
        for src in quiet {
            assert_eq!(words(src), Vec::<String>::new(), "{src}");
        }
    }

    #[test]
    fn it_reads_char_raw_and_multi_line_literals_as_the_compiler_does() {
        assert_eq!(words("s.replace('\"', \"Close\");"), vec!["Close"]);
        assert_eq!(words("let a = r#\"Say \"hi\" now\"#;"), vec!["Say \"hi\" now"]);
        assert_eq!(words("let a = \"first line \\\n        second line\";").len(), 1);
    }

    #[test]
    fn it_skips_test_items_at_any_depth_and_only_those() {
        assert_eq!(words("#[cfg(test)]\nfn helper() {}\nfn f() { let a = \"After it\"; }"), vec!["After it"]);
        assert_eq!(words("#[cfg(test)]\nmod fixtures;\nfn f() { let a = \"After it\"; }"), vec!["After it"]);
        assert_eq!(words("impl A {\n    #[cfg(test)]\n    fn t() { let a = \"In a test\"; }\n    fn f() { let a = \"In code\"; }\n}"), vec!["In code"]);
        assert_eq!(words("#[cfg(test)]\nmod tests { fn t() { let a = \"In a test\"; } }"), Vec::<String>::new());
        assert_eq!(words("#[cfg(not(test))]\nfn f() { let a = \"Kept here\"; }"), vec!["Kept here"]);
    }

    #[test]
    fn it_leaves_ids_classes_paths_keys_and_comments_alone() {
        let quiet = [
            "<i class=\"btn quiet\">",
            "<i id=\"t-checks\">",
            "let p = \"M3 3l10 10M13 3 3 13\";",
            "x.add_event_listener(\"click\");",
            "let k = \"cityloom-welcome\";",
            "let s = format!(\"left:{}px;top:{}px\", a, b);",
            "let k = \"ArrowDown\";",
            "let c = '\"';",
            "// \"A comment about things\"\n/// \"Docs about things\"\nlet a = 1; // \"Trailing comment\"",
            "view! { <i class=(\"btn on\", move || on)>\"\"</i> }",
            "view! { <i class=move || if on { \"btn on\" } else { \"btn off\" } id=\"x\"></i> }",
            "#[wasm_bindgen(js_name = \"Some name\")]\nfn f() {}",
        ];
        for src in quiet {
            assert_eq!(words(src), Vec::<String>::new(), "{src}");
        }
    }

    #[test]
    fn it_flags_the_text_nodes_of_the_notes_table_and_reports_their_lines() {
        let src = "view! {\n    <caption class=\"sr-only\">{format!(\"Width by use, in {}\", u)}</caption>\n    <tr><th scope=\"col\">\"Use\"</th><th scope=\"col\">\"Today\"</th><th scope=\"col\">\"Your design\"</th><th scope=\"col\">\"Change\"</th></tr>\n}";
        let found = offenders(src);
        for w in ["Use", "Today", "Your design", "Change", "Width by use, in "] {
            assert!(found.iter().any(|(_, l)| l == w), "{w}: {found:?}");
        }
        assert_eq!(found.iter().find(|(_, l)| l == "Use").map(|(n, _)| *n), Some(3));
    }

    #[test]
    fn it_reports_a_mistyped_key_in_a_call_a_table_and_a_const() {
        use crate::shared::i18n::{Locale, Resources};
        const R: Resources = Resources { en: "measure-refused = No.\nbadge-fail = Fail\n", fr: "measure-refused = Non.\nbadge-fail = Echec\n" };
        let i18n = crate::shared::i18n::I18n::new(Locale::En, &[R]);
        let src = "const KEYS: [&str; 2] = [\"badge-fail\", \"badge-fial\"];\nfn f(i: &I18n) { i.tr(\"measure-refused\", &a); i.tr(\"measure-refuse\", &a); say(w, \"measure-refsed\"); }";
        let bad: Vec<String> = unknown_ids(src, &i18n, Locale::En).into_iter().map(|(_, id)| id).collect();
        assert_eq!(bad, vec!["badge-fial", "measure-refuse", "measure-refsed"]);
        // A key missing from one language only is reported for that language.
        const ONLY_EN: Resources = Resources { en: "a-b = A\n", fr: "" };
        let i18n = crate::shared::i18n::I18n::new(Locale::FrCa, &[ONLY_EN]);
        assert_eq!(unknown_ids("let k = \"a-b\";", &i18n, Locale::FrCa).len(), 1);
        assert_eq!(unknown_ids("let k = \"a-b\";", &i18n, Locale::En).len(), 0);
    }

    #[test]
    fn a_message_id_is_kebab_case_with_a_hyphen() {
        for yes in ["a-b", "measure-refused", "ui-key-ctrl", "mode-3d-view", "a-1"] {
            assert!(is_message_id(yes), "{yes}");
        }
        for no in ["", "word", "Word-x", "a-", "-a", "a--b", "a_b", "a-B", "t-checks ", "left:{}px", "1-a", "M3-3"] {
            assert!(!is_message_id(no), "{no}");
        }
    }

    #[test]
    fn it_leaves_test_items_alone_when_collecting_keys() {
        assert_eq!(message_ids("#[cfg(test)]\nmod t { fn f() { let a = \"made-up\"; } }").len(), 0);
    }
}
