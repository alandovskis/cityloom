//! Helpers for drawing components to HTML on the host, shared by the slices' tests.

use leptos::prelude::*;
use leptos::tachys::view::RenderHtml;

/// A component drawn to HTML.
pub fn html(view: impl FnOnce() -> AnyView) -> String {
    let owner = Owner::new();
    owner.set();
    view().to_html()
}

pub fn count(html: &str, needle: &str) -> usize {
    html.matches(needle).count()
}

pub fn button_tag(html: &str, id: &str) -> String {
    let at = html.find(&format!("id=\"{id}\"")).unwrap();
    let start = html[..at].rfind("<button").unwrap();
    let end = html[at..].find('>').unwrap() + at;
    html[start..=end].to_string()
}
