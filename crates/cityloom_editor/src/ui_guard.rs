//! The controls of `cityloom_ui` are the only ones of their kind: no slice draws its own radio group, stepper, headed
//! section of the details or question.

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    const STREET: &str = "street/view/inspector.rs";
    const JUNCTION: &str = "junction/view/inspector.rs";
    const ACCOUNT: &str = "account/view.rs";

    fn src() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
    }

    /// Every Rust file under `dir`.
    fn files(dir: &Path) -> Vec<PathBuf> {
        let mut found = Vec::new();
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                found.extend(files(&path));
            } else if path.extension().is_some_and(|e| e == "rs") {
                found.push(path);
            }
        }
        found
    }

    /// The part of a file before its first test module (`#[cfg(test)]`, then `mod` or `pub mod`).
    fn before_tests(text: &str) -> &str {
        let cut = text.match_indices("#[cfg(test)]").find(|(at, attribute)| {
            let next = text[at + attribute.len()..].trim_start();
            let next = next.strip_prefix("pub ").unwrap_or(next);
            next.starts_with("mod ")
        });
        cut.map_or(text, |(at, _)| &text[..at])
    }

    /// The editor's code: every file but the tests and the guards, by its path under `src`.
    fn code() -> Vec<(String, String)> {
        let mut all: Vec<(String, String)> = files(&src())
            .into_iter()
            .filter(|p| {
                let name = p.file_name().unwrap().to_str().unwrap();
                name != "tests.rs" && !name.ends_with("_guard.rs")
            })
            .map(|p| {
                let path = p.strip_prefix(src()).unwrap().to_str().unwrap().replace('\\', "/");
                (path, before_tests(&std::fs::read_to_string(&p).unwrap()).to_string())
            })
            .collect();
        all.sort();
        assert!(all.len() > 100 && all.iter().any(|(path, _)| path == STREET), "the editor's code is read: {} files", all.len());
        all
    }

    fn of<'a>(code: &'a [(String, String)], path: &str) -> &'a str {
        &code.iter().find(|(p, _)| p == path).unwrap_or_else(|| panic!("{path}")).1
    }

    /// The files that hold `needle`.
    fn holding<'a>(code: &'a [(String, String)], needle: &str) -> Vec<&'a str> {
        code.iter().filter(|(_, text)| text.contains(needle)).map(|(path, _)| path.as_str()).collect()
    }

    /// How many times `text` uses the component `name`: `<Name` and then no letter, digit or underscore.
    fn uses(text: &str, name: &str) -> usize {
        let open = format!("<{name}");
        text.match_indices(&open).filter(|(at, o)| !text[at + o.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '_')).count()
    }

    /// Those of the files `paths` that do not use the component `name`.
    fn without<'a>(code: &[(String, String)], name: &str, paths: &[&'a str]) -> Vec<&'a str> {
        paths.iter().copied().filter(|path| uses(of(code, path), name) == 0).collect()
    }

    #[test]
    fn every_radio_group_is_the_ui_crate_s() {
        let code = code();
        for own in [r#"role="radiogroup""#, r#"role="radio""#, r#"type="radio""#] {
            assert_eq!(holding(&code, own), Vec::<&str>::new(), "{own}: use cityloom_ui's RadioGroup");
        }
        assert_eq!(without(&code, "RadioGroup", &[STREET, JUNCTION, ACCOUNT]), Vec::<&str>::new());
    }

    #[test]
    fn every_stepper_is_the_ui_crate_s() {
        let code = code();
        assert_eq!(holding(&code, r#"class="stepper""#), Vec::<&str>::new(), "use cityloom_ui's Stepper");
        assert_eq!(without(&code, "Stepper", &[STREET, JUNCTION]), Vec::<&str>::new());
    }

    /// Whether `text` holds a section of the details with a heading directly inside it.
    fn headed_section(text: &str) -> bool {
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        text.split(r#"<section class="insp-sec"#).skip(1).any(|rest| {
            let contents = rest.split_once('>').map_or("", |(_, c)| c).trim_start();
            contents.starts_with(r#"<h3 class="note-h""#)
        })
    }

    #[test]
    fn every_headed_inspector_section_is_the_ui_crate_s() {
        let code = code();
        let own: Vec<&str> = code.iter().filter(|(_, text)| headed_section(text)).map(|(path, _)| path.as_str()).collect();
        assert_eq!(own, Vec::<&str>::new(), "use cityloom_ui's Section");
        let of_the_ui =
            [STREET, JUNCTION, "street/view/calming_tools.rs", "street/view/curb_tools.rs", "street/view/green_tools.rs", "street/view/stop_tools.rs"];
        assert_eq!(without(&code, "Section", &of_the_ui), Vec::<&str>::new());
    }

    #[test]
    fn every_confirmation_is_the_ui_crate_s() {
        let code = code();
        assert_eq!(holding(&code, r#"class="confirm""#), Vec::<&str>::new(), "use cityloom_ui's Confirm");
        let asked = [(ACCOUNT, 2), ("archive/view.rs", 1), ("project/view.rs", 1), ("timeline/view.rs", 1), ("share/view.rs", 1)];
        for (path, times) in asked {
            assert_eq!(uses(of(&code, path), "Confirm"), times, "{path}");
        }
    }

    #[test]
    fn the_helpers_the_ui_crate_replaced_are_gone() {
        let code = code();
        let replaced: [(&str, &[&str]); 3] =
            [(STREET, &["radio", "radio_keys", "tick"]), (JUNCTION, &["tick", "focus_radio", "choice_radio", "show_detection"]), (ACCOUNT, &["radio"])];
        let left: Vec<String> = replaced
            .iter()
            .flat_map(|(path, helpers)| helpers.iter().map(move |helper| (path, helper)))
            .filter(|(path, helper)| of(&code, path).contains(&format!("fn {helper}(")))
            .map(|(path, helper)| format!("{path}: {helper}"))
            .collect();
        assert_eq!(left, Vec::<String>::new());
    }
}
