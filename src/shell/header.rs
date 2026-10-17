//! The sticky header and what it hides. Under 900 px the bar is a sticky card that can be a third of a small
//! window (a phone, or a desktop at 400% zoom), so a control the keyboard focuses would sit under it (WCAG 2.4.11).
//! The page keeps `--bar-stuck`, how far down the bar covers the window while it is stuck, and the stylesheet
//! gives the document that much `scroll-padding-top`, so focus and anchors scroll clear of the bar.

use leptos::prelude::*;
use leptos::web_sys::Element;
use wasm_bindgen::prelude::*;

/// How far from the top of the window a bar covers: its offset from the top and its height plus a gap of 8 px,
/// while it is sticky or fixed; nothing otherwise (a bar that scrolls away hides nothing for long).
pub(crate) fn covered_px(position: &str, top: f64, height: f64) -> f64 {
    if position == "sticky" || position == "fixed" { (top + height + 8.0).ceil() } else { 0.0 }
}

fn publish(bar: &Element) {
    let Some(root) = document().document_element() else { return };
    let Ok(Some(style)) = window().get_computed_style(bar) else { return };
    let top = style.get_property_value("top").ok().and_then(|t| t.trim_end_matches("px").parse::<f64>().ok()).unwrap_or(0.0);
    let position = style.get_property_value("position").unwrap_or_default();
    let covered = covered_px(&position, top, bar.get_bounding_client_rect().height());
    let _ = root.unchecked_ref::<leptos::web_sys::HtmlElement>().style().set_property("--bar-stuck", &format!("{covered}px"));
}

/// Keeps `--bar-stuck` at the bar's real size, now and whenever it changes (a wrapped row, another language, a
/// breakpoint).
pub(crate) fn watch() {
    let Ok(Some(bar)) = document().query_selector(".bar") else { return };
    publish(&bar);
    let again = bar.clone();
    let observer = Closure::<dyn FnMut(leptos::web_sys::js_sys::Array)>::new(move |_| publish(&again));
    if let Ok(o) = leptos::web_sys::ResizeObserver::new(observer.as_ref().unchecked_ref()) {
        o.observe(&bar);
    }
    observer.forget();
}

#[cfg(test)]
mod tests {
    use super::covered_px;

    #[test]
    fn a_sticky_or_fixed_bar_covers_its_offset_and_height_and_a_gap_and_a_bar_in_the_flow_nothing() {
        assert_eq!(covered_px("sticky", 8.0, 120.0), 136.0);
        assert_eq!(covered_px("fixed", 16.0, 64.0), 88.0);
        assert_eq!(covered_px("sticky", 8.0, 120.4), 137.0);
        assert_eq!(covered_px("static", 8.0, 120.0), 0.0);
        assert_eq!(covered_px("relative", 0.0, 50.0), 0.0);
    }
}
