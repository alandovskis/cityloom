//! The home page's views: the city drawn quietly behind the hero card, and a line
//! saying how the city stands. The search box is the map's own.

use std::rc::Rc;

use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use leptos::wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use leptos::web_sys::Element;

use crate::map::svg::map_svg;
use crate::map::vm::MapVm;
use crate::shared::bind::Bound;
use crate::shared::tick::Tick;

/// The whole city, drawn as on the map but only to look at: nothing in it can be
/// pressed or reached by the keyboard.
#[component]
pub fn HeroMap(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    let frame = NodeRef::<leptos::html::Div>::new();
    let k = Memo::new(move |_| vm.with(|v| v.camera().k));
    let places = move || vm.with(|v| map_svg(&v.view(), k.get(), v.units(), v.i18n()));

    #[cfg(target_arch = "wasm32")]
    {
        use leptos::wasm_bindgen::closure::Closure;
        Effect::new(move |_| {
            let Some(el) = frame.get() else { return };
            let el: Element = el.into();
            let fit = {
                let el = el.clone();
                move || {
                    let r = el.get_bounding_client_rect();
                    if r.width() > 0.0 && r.height() > 0.0 {
                        vm.with(|v| {
                            v.set_insets(crate::map::view::insets_of(&el));
                            v.resize(r.width(), r.height());
                        });
                    }
                }
            };
            fit();
            let observer = Closure::<dyn FnMut(leptos::web_sys::js_sys::Array)>::new(move |_| fit());
            if let Ok(o) = leptos::web_sys::ResizeObserver::new(observer.as_ref().unchecked_ref()) {
                o.observe(&el);
                crate::map::view::observe_covers(&o);
            }
            observer.forget();
        });
        // What the other pages wrote is read again when the page is shown, and when another tab writes.
        window_event_listener(leptos::ev::storage, move |e: leptos::web_sys::StorageEvent| {
            if e.key().is_none_or(|k| k == crate::city::store::CITY_KEY) {
                vm.with(|v| v.reload());
            }
        });
        window_event_listener(leptos::ev::pageshow, move |e: leptos::web_sys::PageTransitionEvent| {
            if e.persisted() {
                vm.with(|v| v.reload());
            }
        });
    }

    view! {
        <div class="hero-map" node_ref=frame aria-hidden="true" inert="">
            <svg id="map" xmlns="http://www.w3.org/2000/svg" viewBox=move || vm.with(|v| v.view_box())>
                <g inner_html=places></g>
            </svg>
        </div>
    }
}

/// How the city stands: its name and size, and whether it works.
#[component]
pub fn HeroFacts(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    let works = Signal::derive(move || !vm.with(|v| v.status().bad));
    view! {
        <p class=move || if works.get() { "hero-facts" } else { "hero-facts bad" } role="status">
            <Tick works=works/>
            <span>{move || vm.with(|v| format!("{}: {}.", v.title(), v.counts()))}</span>
            " "
            <span>{move || vm.with(|v| v.status().text)}</span>
        </p>
    }
}
