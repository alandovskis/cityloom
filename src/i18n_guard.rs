//! A migrated file says nothing in words of its own: every sentence comes from a message.

const MIGRATED: &[(&str, &str)] = &[
    // Files are added here as their task completes.
];

/// Attributes whose value is text a person reads.
const TEXT_ATTRS: &[&str] = &["aria-label", "title", "placeholder", "alt"];

/// Literals that are keys, not words.
const ALLOWED: &[&str] = &["Escape", "Enter", "ArrowDown", "ArrowUp", "ArrowLeft", "ArrowRight", "Home", "End", "Delete", "Backspace", "Tab"];

fn is_path_data(s: &str) -> bool {
    s.chars().all(|c| "MmLlHhVvCcSsQqTtAaZz0123456789 .,-".contains(c))
}

fn looks_like_words(s: &str) -> bool {
    if ALLOWED.contains(&s) || is_path_data(s) {
        return false;
    }
    let has_word = |t: &str| t.split(|c: char| !c.is_ascii_alphabetic()).any(|w| w.len() >= 3 && w.chars().all(|c| c.is_ascii_lowercase()));
    let capitalised = s.chars().next().is_some_and(|c| c.is_ascii_uppercase()) && s.chars().skip(1).take(3).all(|c| c.is_ascii_lowercase());
    (s.contains(' ') && has_word(s)) || (capitalised && s.len() >= 4)
}

/// The string literals of `src` (outside comments and the test module) with their line and the text just before the opening quote.
fn literals(src: &str) -> Vec<(usize, String, String)> {
    let body = src.split("\n#[cfg(test)]").next().unwrap_or(src);
    let mut out = Vec::new();
    for (n, line) in body.lines().enumerate() {
        let t = line.trim_start();
        if t.starts_with("//") {
            continue;
        }
        let b = line.as_bytes();
        let mut i = 0;
        while i < b.len() {
            if b[i] == b'"' {
                // Not trimmed: an attribute's `=` touches its quote, a Rust `let a = "…"` does not.
                let before = line[..i].to_string();
                let mut j = i + 1;
                let mut lit = String::new();
                while j < b.len() && b[j] != b'"' {
                    if b[j] == b'\\' && j + 1 < b.len() {
                        j += 1;
                    }
                    lit.push(b[j] as char);
                    j += 1;
                }
                out.push((n + 1, lit, before));
                i = j;
            }
            i += 1;
        }
    }
    out
}

fn attribute_name(before: &str) -> Option<&str> {
    before.strip_suffix('=').map(|s| s.rsplit(|c: char| c.is_whitespace() || c == '<').next().unwrap_or(""))
}

#[test]
fn migrated_files_have_no_hardcoded_text() {
    let mut offenders = Vec::new();
    for (path, src) in MIGRATED {
        for (line, lit, before) in literals(src) {
            let text_attr = attribute_name(&before).is_some_and(|a| TEXT_ATTRS.contains(&a));
            let other_attr = attribute_name(&before).is_some() && !text_attr;
            if !other_attr && looks_like_words(&lit) {
                offenders.push(format!("{path}:{line}: {lit:?}"));
            }
        }
    }
    assert!(offenders.is_empty(), "text still in the code:\n{}", offenders.join("\n"));
}

#[test]
fn the_guard_finds_words_and_leaves_ids_classes_and_paths_alone() {
    let src = "let a = \"Every metre\";\nlet b = \"btn quiet\";\n<i class=\"btn quiet\" aria-label=\"Close it\">\nlet p = \"M3 3l10 10M13 3 3 13\";\nlet k = \"ArrowDown\";\nlet id = \"t-checks\";\n";
    let found: Vec<_> = literals(src)
        .into_iter()
        .filter(|(_, l, before)| {
            let a = attribute_name(before);
            let text = a.is_some_and(|a| TEXT_ATTRS.contains(&a));
            (text || a.is_none()) && looks_like_words(l)
        })
        .map(|(_, l, _)| l)
        .collect();
    assert_eq!(found, vec!["Every metre", "btn quiet", "Close it"]);
}
