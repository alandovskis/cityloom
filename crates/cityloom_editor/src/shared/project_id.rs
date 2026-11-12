//! Project ids and the wire keys the sync composes from them. The first project's id is `first` and its records
//! keep their bare keys, so work made before projects existed needs no migration; any other project's records go
//! up as `<project>:<key>`.

/// The id of the first project, the one every person starts with.
pub const FIRST: &str = "first";

/// Whether `id` is a project id: 1 to 16 characters of `a-z`, `0-9` and `-`.
pub fn valid(id: &str) -> bool {
    (1..=16).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// The key a record travels under: bare for the first project, `<project>:<key>` otherwise.
pub fn wire_key(project: &str, key: &str) -> String {
    if project == FIRST { key.to_string() } else { format!("{project}:{key}") }
}

/// Inverts `wire_key`: the project and the key. A key with no project prefix belongs to the first project.
pub fn split_key(wire: &str) -> (&str, &str) {
    match wire.split_once(':') {
        Some((project, key)) if project != FIRST && valid(project) => (project, key),
        _ => (FIRST, wire),
    }
}

/// The pages that open inside a project: the map and the two editors.
pub const PROJECT_PAGES: [&str; 3] = ["map.html", "street.html", "intersection.html"];

/// `href` naming `project` (`project=<id>` added to its query) where it leads to one of `PROJECT_PAGES` and names no
/// project yet; any other address, or no project, leaves it as it is.
pub fn link(href: &str, project: Option<&str>) -> String {
    let Some(project) = project else { return href.to_string() };
    let (rest, fragment) = href.split_once('#').map_or((href, None), |(r, f)| (r, Some(f)));
    let (path, query) = rest.split_once('?').map_or((rest, None), |(p, q)| (p, Some(q)));
    let page = path.rsplit('/').next().unwrap_or(path);
    let named = query.is_some_and(|q| q.split('&').any(|pair| pair == "project" || pair.starts_with("project=")));
    if !PROJECT_PAGES.contains(&page) || named {
        return href.to_string();
    }
    let query = match query {
        Some(q) if !q.is_empty() => format!("{q}&project={project}"),
        _ => format!("project={project}"),
    };
    let fragment = fragment.map(|f| format!("#{f}")).unwrap_or_default();
    format!("{path}?{query}{fragment}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_project_keeps_bare_keys() {
        assert_eq!(wire_key("first", "w1~n2~n3"), "w1~n2~n3");
        assert_eq!(split_key("w1~n2~n3"), ("first", "w1~n2~n3"));
        assert_eq!(split_key("r-abc"), ("first", "r-abc"));
        assert_eq!(split_key("current"), ("first", "current"));
    }

    #[test]
    fn another_project_prefixes_its_keys() {
        assert_eq!(wire_key("p-0123456789ab", "current"), "p-0123456789ab:current");
        assert_eq!(split_key("p-0123456789ab:current"), ("p-0123456789ab", "current"));
        assert_eq!(split_key("p-0123456789ab:w1~n2"), ("p-0123456789ab", "w1~n2"));
    }

    #[test]
    fn ids_have_one_to_sixteen_lowercase_characters() {
        assert!(valid("first"));
        assert!(valid("p-0123456789ab"));
        assert!(valid(&"a".repeat(16)));
        assert!(!valid(""));
        assert!(!valid("Has Upper"));
        assert!(!valid("under_score"));
        assert!(!valid(&"a".repeat(17)));
    }

    #[test]
    fn a_link_to_a_page_of_a_project_names_the_project() {
        assert_eq!(link("map.html", Some("p1")), "map.html?project=p1");
        assert_eq!(link("street.html?street=7", Some("p1")), "street.html?street=7&project=p1");
        assert_eq!(link("intersection.html?junction=3", Some("first")), "intersection.html?junction=3&project=first");
        assert_eq!(link("/map.html#x", Some("p1")), "/map.html?project=p1#x");
        // already named, no project, or a page outside a project: as it is
        assert_eq!(link("map.html?project=p2", Some("p1")), "map.html?project=p2");
        assert_eq!(link("street.html?street=7", None), "street.html?street=7");
        for other in ["index.html", "account.html", "credits.html", "https://www.openstreetmap.org/", "#wrap", "mapping.html"] {
            assert_eq!(link(other, Some("p1")), other);
        }
    }
}
