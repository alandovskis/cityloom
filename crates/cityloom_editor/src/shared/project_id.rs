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

/// Whether `key` of `project` comes back as itself from its wire key. Only a first-project key that starts with another
/// project's id and `:` does not (`p-abc:w1` would travel as project `p-abc`'s `w1`): such a record is not the first
/// project's to send, list or remove, and is never kept there.
pub fn round_trips(project: &str, key: &str) -> bool {
    split_key(&wire_key(project, key)) == (project, key)
}

/// The pages that open inside a project: the map, the two editors, the timeline and the page that shares it.
pub const PROJECT_PAGES: [&str; 5] = ["map.html", "street.html", "intersection.html", "timeline.html", "share.html"];

/// What a page's address carries to open its project read-only: the design can be looked at, not changed. Nothing
/// on a page adds it or takes it away (`link_in` keeps it on the links between a project's pages); the share link
/// (#221) is to open a design with it.
pub const VIEW: &str = "view=1";

/// The project the query string `search` (with or without its `?`) names: `Some("")` for a bare or empty `project`
/// (named, but none), None where it names none. The one reading of `project=` for every page.
pub fn named_project(search: &str) -> Option<&str> {
    search.trim_start_matches('?').split('&').find_map(|pair| match pair.split_once('=') {
        Some(("project", id)) => Some(id),
        _ => (pair == "project").then_some(""),
    })
}

/// Whether the page whose query string is `search` (with or without its `?`) opens its project read-only.
pub fn viewing(search: &str) -> bool {
    search.trim_start_matches('?').split('&').any(|pair| pair == VIEW)
}

/// `href` naming `project` (`project=<id>` added to its query) where it leads to one of `PROJECT_PAGES` and names no
/// project yet; any other address, or no project, leaves it as it is.
pub fn link(href: &str, project: Option<&str>) -> String {
    link_in(href, project, false)
}

/// `link`, from a page that shows `project` read-only when `read_only`: a link to another of its pages opens that
/// read-only too (`VIEW` added), so the mode cannot be left by going from page to page.
pub fn link_in(href: &str, project: Option<&str>, read_only: bool) -> String {
    let Some(project) = project else { return href.to_string() };
    let (rest, fragment) = href.split_once('#').map_or((href, None), |(r, f)| (r, Some(f)));
    let (path, query) = rest.split_once('?').map_or((rest, None), |(p, q)| (p, Some(q)));
    let page = path.rsplit('/').next().unwrap_or(path);
    if !PROJECT_PAGES.contains(&page) {
        return href.to_string();
    }
    let mut pairs: Vec<String> = query.unwrap_or_default().split('&').filter(|p| !p.is_empty()).map(str::to_string).collect();
    let named = named_project(query.unwrap_or_default()).is_some();
    let viewed = pairs.iter().any(|pair| pair == VIEW);
    if named && (viewed || !read_only) {
        return href.to_string();
    }
    if !named {
        pairs.push(format!("project={project}"));
    }
    if read_only && !viewed {
        pairs.push(VIEW.to_string());
    }
    let fragment = fragment.map(|f| format!("#{f}")).unwrap_or_default();
    format!("{path}?{}{fragment}", pairs.join("&"))
}

/// `href`, a page of a project, as the same page of `project` opened to edit: its `project=` names `project` and
/// `VIEW` is gone; the rest of its query and its fragment stay. An address of no page that shows a project's work (the
/// list, the page that opens a share link) gives `project`'s map. Relative, the page's name alone: the app's pages are
/// side by side, and a path from the address bar (`//host/…` names another host) never becomes a link.
pub fn editable_in(href: &str, project: &str) -> String {
    let (rest, fragment) = href.split_once('#').map_or((href, None), |(r, f)| (r, Some(f)));
    let (path, query) = rest.split_once('?').map_or((rest, ""), |(p, q)| (p, q));
    let page = path.rsplit('/').next().unwrap_or(path);
    if page == "share.html" || !PROJECT_PAGES.contains(&page) {
        return format!("map.html?project={project}");
    }
    let mut pairs: Vec<String> =
        query.split('&').filter(|pair| !pair.is_empty() && *pair != VIEW && *pair != "project" && !pair.starts_with("project=")).map(str::to_string).collect();
    pairs.push(format!("project={project}"));
    let fragment = fragment.map(|f| format!("#{f}")).unwrap_or_default();
    format!("{page}?{}{fragment}", pairs.join("&"))
}

/// `link_in` from a page whose project's storage is `storage`: its project, and read-only where it is.
pub fn link_for(href: &str, storage: &dyn crate::shared::ports::Storage) -> String {
    link_in(href, storage.project().as_deref(), storage.read_only())
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
    fn a_key_with_a_colon_round_trips_unless_it_would_name_another_project_from_the_first() {
        // `first:` is no other project's prefix: the key stays the first project's
        assert_eq!(split_key("first:x"), ("first", "first:x"));
        assert!(round_trips("first", "first:x"));
        // under another project any key comes back whole, colons and all
        assert_eq!(split_key(&wire_key("p1", "p-abc:w1")), ("p1", "p-abc:w1"));
        assert!(round_trips("p1", "p-abc:w1"));
        assert!(round_trips("p1", "first:x"));
        // a first-project key that starts with another project's id would come back as that project's
        assert_eq!(split_key(&wire_key("first", "p-abc:w1")), ("p-abc", "w1"));
        assert!(!round_trips("first", "p-abc:w1"));
        // a prefix that is no project id is part of the key
        assert!(round_trips("first", "Not An Id:w1"));
        assert!(round_trips("first", "w1~n2~n3"));
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
    fn the_project_a_query_names_is_read_one_way_for_every_page() {
        assert_eq!(named_project("project=p1&view=1"), Some("p1"));
        assert_eq!(named_project("?street=7&project=p-abc"), Some("p-abc"));
        assert_eq!(named_project("project"), Some(""), "named, but none: a page then opens none in its place");
        assert_eq!(named_project("project="), Some(""));
        assert_eq!(named_project("projects=p1&view=1"), None);
        assert_eq!(named_project(""), None);
    }

    #[test]
    fn a_page_is_read_only_when_its_address_says_view() {
        assert!(viewing("project=p1&view=1"));
        assert!(viewing("?view=1&project=p1"));
        assert!(viewing("street=7&project=p1&view=1"));
        for no in ["", "project=p1", "view=0", "view", "review=1", "view=yes"] {
            assert!(!viewing(no), "{no}");
        }
    }

    #[test]
    fn a_link_from_a_read_only_page_keeps_it_read_only() {
        assert_eq!(link_in("map.html", Some("p1"), true), "map.html?project=p1&view=1");
        assert_eq!(link_in("street.html?street=7#x", Some("p1"), true), "street.html?street=7&project=p1&view=1#x");
        assert_eq!(link_in("map.html?project=p1&view=1", Some("p1"), true), "map.html?project=p1&view=1");
        // a link naming its project already is still kept read-only
        assert_eq!(link_in("map.html?project=p2", Some("p1"), true), "map.html?project=p2&view=1");
        assert_eq!(link_in("map.html", Some("p1"), false), "map.html?project=p1");
        // a page outside a project is not one of its pages
        for other in ["index.html", "about.html", "account.html", "https://www.openstreetmap.org/", "#wrap"] {
            assert_eq!(link_in(other, Some("p1"), true), other);
        }
        assert_eq!(link_in("map.html", None, true), "map.html");
    }

    #[test]
    fn the_same_page_in_another_project_opened_to_edit() {
        assert_eq!(editable_in("/street.html?street=7&project=s1&view=1", "p-abc"), "street.html?street=7&project=p-abc");
        assert_eq!(editable_in("/map.html?view=1&project=s1#x", "p2"), "map.html?project=p2#x");
        assert_eq!(editable_in("map.html", "p2"), "map.html?project=p2");
        assert_eq!(editable_in("/intersection.html?junction=3&project=&view=1", "p2"), "intersection.html?junction=3&project=p2");
        // only the page's name is kept: a path that names another host (`//host/…`, which a browser reads as that host)
        // never becomes the link
        assert_eq!(editable_in("//evil.example/street.html?street=7&project=s1&view=1", "p2"), "street.html?street=7&project=p2");
        assert_eq!(editable_in("/app/v2/timeline.html?project=s1", "p2"), "timeline.html?project=p2");
        // a page outside a project opens the project's map
        assert_eq!(editable_in("/share.html#token", "p2"), "map.html?project=p2");
        assert_eq!(editable_in("/index.html", "p2"), "map.html?project=p2");
    }

    #[test]
    fn a_link_to_a_page_of_a_project_names_the_project() {
        assert_eq!(link("map.html", Some("p1")), "map.html?project=p1");
        assert_eq!(link("street.html?street=7", Some("p1")), "street.html?street=7&project=p1");
        assert_eq!(link("intersection.html?junction=3", Some("first")), "intersection.html?junction=3&project=first");
        assert_eq!(link("/map.html#x", Some("p1")), "/map.html?project=p1#x");
        assert_eq!(link("timeline.html", Some("p1")), "timeline.html?project=p1");
        // already named, no project, or a page outside a project: as it is
        assert_eq!(link("map.html?project=p2", Some("p1")), "map.html?project=p2");
        assert_eq!(link("street.html?street=7", None), "street.html?street=7");
        for other in ["index.html", "account.html", "credits.html", "https://www.openstreetmap.org/", "#wrap", "mapping.html"] {
            assert_eq!(link(other, Some("p1")), other);
        }
    }
}
