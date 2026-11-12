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
}
