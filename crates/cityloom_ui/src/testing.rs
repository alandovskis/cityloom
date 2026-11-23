//! Helpers for drawing the controls to HTML on the host and reading it, shared by the controls' tests.

use leptos::prelude::*;
use leptos::tachys::view::RenderHtml;

/// A component drawn to HTML.
pub fn html(view: impl FnOnce() -> AnyView) -> String {
    let owner = Owner::new();
    owner.set();
    view().to_html()
}

/// A word that stays as it is.
pub fn word(text: &'static str) -> Signal<String> {
    Signal::derive(move || text.to_string())
}

/// The opening tags of every element `name` in `h`, in order.
pub fn tags<'a>(h: &'a str, name: &str) -> Vec<&'a str> {
    h.match_indices(&format!("<{name}"))
        .map(|(at, open)| (at, at + open.len()))
        .filter(|(_, end)| h[*end..].starts_with([' ', '>']))
        .map(|(at, _)| &h[at..=at + h[at..].find('>').unwrap()])
        .collect()
}

/// The value of attribute `name` of an opening tag.
pub fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let open = format!(" {name}=\"");
    let from = tag.find(&open)? + open.len();
    Some(&tag[from..from + tag[from..].find('"').unwrap()])
}

/// Whether an opening tag has class `name`.
pub fn has_class(tag: &str, name: &str) -> bool {
    attr(tag, "class").is_some_and(|c| c.split_whitespace().any(|c| c == name))
}

/// What `h` says once its tags are taken out.
pub fn said(h: &str) -> String {
    h.split('<').map(|part| part.split_once('>').map_or(part, |(_, text)| text)).collect::<String>().trim().to_string()
}
