//! Where a street or a junction came from.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// An OpenStreetMap way (of a street) or node (of a junction) that a place was made from, and the version it had when
/// the data was read.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OsmRef {
    pub id: i64,
    pub version: Option<i32>,
}

/// The page of a way or node (`kind`) on OpenStreetMap.
pub fn osm_url(kind: &str, id: i64) -> String {
    format!("https://www.openstreetmap.org/{kind}/{id}")
}

/// What a link to one reads: `way 4687530 v49`, without the version where it is not known.
pub fn osm_label(kind: &str, r: &OsmRef) -> String {
    match r.version {
        Some(v) => format!("{kind} {} v{v}", r.id),
        None => format!("{kind} {}", r.id),
    }
}

/// Links to the pages on OpenStreetMap of the ways or nodes (`kind`) a place was made from.
#[component]
pub fn Sources(kind: &'static str, refs: Vec<OsmRef>) -> impl IntoView {
    let last = refs.len().saturating_sub(1);
    refs.into_iter()
        .enumerate()
        .map(|(i, r)| view! { <a href=osm_url(kind, r.id) rel="noopener">{osm_label(kind, &r)}</a>{(i < last).then_some(", ")} })
        .collect_view()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_way_or_node_is_linked_to_its_page_on_openstreetmap_and_named_with_its_version() {
        assert_eq!(osm_url("way", 4687530), "https://www.openstreetmap.org/way/4687530");
        assert_eq!(osm_url("node", 29796354), "https://www.openstreetmap.org/node/29796354");
        assert_eq!(osm_label("way", &OsmRef { id: 4687530, version: Some(49) }), "way 4687530 v49");
        assert_eq!(osm_label("node", &OsmRef { id: 7, version: None }), "node 7");
    }
}
