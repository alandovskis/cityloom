//! The view of the shell: binds the markup every page keeps for the account menu,
//! the sidebar buttons and the notes tabs to a `ShellVm`. Each property is an
//! effect that sets an attribute; each event is forwarded to a command.

use std::rc::Rc;

use leptos::prelude::*;
use leptos::web_sys::{Element, Event, EventTarget, HtmlElement, HtmlSelectElement, KeyboardEvent, MediaQueryList, MouseEvent};
use wasm_bindgen::prelude::*;

use crate::shared::i18n::{Args, I18n, Locale};
use crate::shared::units::Units;
use crate::shell::vm::{ShellVm, Target, Theme};

fn listen<E: JsCast + 'static>(target: &EventTarget, event: &str, mut f: impl FnMut(E) + 'static) {
    let cb = Closure::<dyn FnMut(Event)>::new(move |e: Event| f(e.unchecked_into()));
    let _ = target.add_event_listener_with_callback(event, cb.as_ref().unchecked_ref());
    cb.forget();
}

fn by_id(id: &str) -> Option<Element> {
    document().get_element_by_id(id)
}

fn all(selector: &str) -> Vec<Element> {
    let Ok(list) = document().query_selector_all(selector) else { return Vec::new() };
    (0..list.length()).filter_map(|i| list.item(i)).filter_map(|n| n.dyn_into().ok()).collect()
}

/// Typing in a field: the page's keys are not for it.
fn typing(target: Option<EventTarget>) -> bool {
    target.and_then(|t| t.dyn_into::<Element>().ok()).is_some_and(|e| e.closest("input, select, textarea").ok().flatten().is_some())
}

/// Binds the page's shell markup to a view-model made for `target`. `details_word`
/// is the message that names what the left sidebar holds on this page.
pub fn mount(i18n: Rc<I18n>, target: Rc<dyn Target>, details_word: &'static str, notes_give_way: bool) {
    let tabs = all(".notes .tab").iter().map(|t| t.id()).collect();
    let dark = leptos::prelude::window().match_media("(prefers-color-scheme: dark)").ok().flatten();
    // An editor has too little room for its notes beside the drawing below this width.
    let roomy = leptos::prelude::window().match_media("(min-width: 1360px)").ok().flatten().is_none_or(|m| m.matches());
    let vm = ShellVm::new_with(
        crate::shared::platform::browser_ports(),
        i18n,
        target,
        details_word,
        tabs,
        dark.as_ref().is_some_and(|m| m.matches()),
        roomy || !notes_give_way,
    );
    // The effects live as long as the page.
    let owner = Owner::new();
    owner.with(|| {
        bind_language(&vm);
        bind_units(&vm);
        bind_region(&vm);
        bind_theme(&vm, dark);
        bind_menu(&vm);
        bind_sidebars(&vm);
        bind_tabs(&vm);
    });
    if let Some(print) = by_id("print") {
        listen(&print, "click", |_: MouseEvent| {
            let _ = leptos::prelude::window().print();
        });
    }
    std::mem::forget(owner);
}

/// The attributes a page's markup may have translated, as `data-i18n-<name>="key"`.
const TRANSLATED_ATTRIBUTES: [&str; 4] = ["aria-label", "title", "placeholder", "content"];

/// The value of attribute `name` in `tag` (the text between `<` and `>`), if it has one in double quotes.
fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let start = tag.find(&format!(" {name}=\""))? + name.len() + 3;
    let len = tag[start..].find('"')?;
    Some(&tag[start..start + len])
}

/// Every message the markup asks for, with the English text the markup has meanwhile: `(key, text)` for each
/// `data-i18n="key"` (the text up to the next tag) and `(key, value)` for each `data-i18n-<attribute>="key"`
/// (the value of that attribute on the same tag).
pub fn i18n_keys(html: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut rest = html;
    while let Some(open) = rest.find('<') {
        rest = &rest[open + 1..];
        // The tag ends at the first `>` outside quotes.
        let mut quote = false;
        let Some(end) = rest.find(|c: char| {
            quote ^= c == '"';
            c == '>' && !quote
        }) else {
            break;
        };
        let (tag, after) = (&rest[..end], &rest[end + 1..]);
        if tag.starts_with(['/', '!']) {
            continue;
        }
        if let Some(key) = attribute(tag, "data-i18n") {
            let text = &after[..after.find('<').unwrap_or(after.len())];
            found.push((key.to_string(), text.trim().to_string()));
        }
        for name in TRANSLATED_ATTRIBUTES {
            if let Some(key) = attribute(tag, &format!("data-i18n-{name}")) {
                found.push((key.to_string(), attribute(tag, name).unwrap_or_default().to_string()));
            }
        }
        rest = after;
    }
    found
}

/// Puts the words of the active language into the markup under `root`. It reads the locale, so run in an
/// effect it is done again when the language changes. The `<title>` is deliberately left out: the page that
/// knows its own title (a street named for where it runs) sets it, so it carries no `data-i18n`.
pub fn apply_translations(root: &Element, i18n: &I18n) {
    let _ = i18n.locale();
    let each = |selector: &str| -> Vec<Element> {
        let Ok(list) = root.query_selector_all(selector) else { return Vec::new() };
        (0..list.length()).filter_map(|i| list.item(i)).filter_map(|n| n.dyn_into().ok()).collect()
    };
    for el in each("[data-i18n]") {
        let key = el.get_attribute("data-i18n").unwrap_or_default();
        el.set_text_content(Some(&i18n.tr(&key, &Args::new())));
    }
    for name in TRANSLATED_ATTRIBUTES {
        for el in each(&format!("[data-i18n-{name}]")) {
            let key = el.get_attribute(&format!("data-i18n-{name}")).unwrap_or_default();
            let _ = el.set_attribute(name, &i18n.tr(&key, &Args::new()));
        }
    }
}

fn bind_language(vm: &Rc<ShellVm>) {
    for b in all("[data-lang]") {
        let (v, tag) = (vm.clone(), b.get_attribute("data-lang").unwrap_or_default());
        listen(&b, "click", move |_: MouseEvent| v.set_locale(Locale::parse(&tag)));
        let v = vm.clone();
        Effect::new(move |_| {
            let on = v.locale().tag() == b.get_attribute("data-lang").unwrap_or_default();
            let _ = b.set_attribute("aria-pressed", &on.to_string());
        });
    }
    let v = vm.clone();
    Effect::new(move |_| {
        if let Some(root) = document().document_element() {
            apply_translations(&root, &v.i18n());
        }
    });
}

fn bind_units(vm: &Rc<ShellVm>) {
    for b in all(".unit[data-unit]") {
        let (v, units) = (vm.clone(), b.get_attribute("data-unit").unwrap_or_default());
        listen(&b, "click", move |_: MouseEvent| v.set_units(Units::parse(&units)));
        let v = vm.clone();
        Effect::new(move |_| {
            let on = v.units().word() == b.get_attribute("data-unit").unwrap_or_default();
            let _ = b.set_attribute("aria-pressed", &on.to_string());
        });
    }
}

fn bind_region(vm: &Rc<ShellVm>) {
    let Some(pick) = by_id("region").and_then(|e| e.dyn_into::<HtmlSelectElement>().ok()) else { return };
    let options: String = vm.regions().iter().map(|r| format!("<option value=\"{}\">{}</option>", r.id, r.label)).collect();
    pick.set_inner_html(&options);
    let v = vm.clone();
    let p = pick.clone();
    listen(&pick, "change", move |_: Event| v.choose_region(&p.value()));
    let v = vm.clone();
    Effect::new(move |_| pick.set_value(&v.region()));
}

fn bind_theme(vm: &Rc<ShellVm>, dark: Option<MediaQueryList>) {
    if let Some(dark) = dark {
        let v = vm.clone();
        let d = dark.clone();
        listen(&dark, "change", move |_: Event| v.follow_system(d.matches()));
    }
    for b in all(".theme") {
        let theme = if b.get_attribute("data-theme-set").as_deref() == Some("dark") { Theme::Dark } else { Theme::Light };
        let v = vm.clone();
        listen(&b, "click", move |_: MouseEvent| v.choose_theme(theme));
        let v = vm.clone();
        Effect::new(move |_| {
            let _ = b.set_attribute("aria-pressed", &(v.theme() == theme).to_string());
        });
    }
    let v = vm.clone();
    Effect::new(move |_| {
        if let (Some(t), Some(root)) = (v.chosen_theme(), document().document_element()) {
            let _ = root.set_attribute("data-theme", t.word());
        }
    });
}

fn bind_menu(vm: &Rc<ShellVm>) {
    let (Some(btn), Some(menu)) = (by_id("account-btn"), by_id("account-menu")) else { return };
    let v = vm.clone();
    listen(&btn, "click", move |_: MouseEvent| v.toggle_menu());
    let (v, m, b) = (vm.clone(), menu.clone(), btn.clone());
    listen(&document(), "pointerdown", move |e: Event| {
        let inside = |el: &Element| e.target().and_then(|t| t.dyn_into::<leptos::web_sys::Node>().ok()).is_some_and(|n| el.contains(Some(&n)));
        if v.menu_open() && !inside(&m) && !inside(&b) {
            v.close_menu();
        }
    });
    let (v, b) = (vm.clone(), btn.clone());
    listen(&document(), "keydown", move |e: KeyboardEvent| {
        if e.key() == "Escape" && v.escape() {
            e.stop_propagation();
            if let Ok(b) = b.clone().dyn_into::<HtmlElement>() {
                let _ = b.focus();
            }
        }
    });
    let (v, m, b) = (vm.clone(), menu.clone(), btn.clone());
    listen(&menu, "focusout", move |e: leptos::web_sys::FocusEvent| {
        let Some(to) = e.related_target().and_then(|t| t.dyn_into::<leptos::web_sys::Node>().ok()) else { return };
        if !m.contains(Some(&to)) && !b.contains(Some(&to)) {
            v.close_menu();
        }
    });
    let v = vm.clone();
    Effect::new(move |_| {
        let open = v.menu_open();
        menu.unchecked_ref::<HtmlElement>().set_hidden(!open);
        let _ = btn.set_attribute("aria-expanded", &open.to_string());
    });
}

fn bind_sidebars(vm: &Rc<ShellVm>) {
    let root = document().document_element();
    for (button, key, data) in [("inspector-toggle", "[", "inspector"), ("notes-toggle", "]", "notes")] {
        let Some(btn) = by_id(button) else { continue };
        let toggle = {
            let v = vm.clone();
            move || if data == "inspector" { v.toggle_inspector() } else { v.toggle_notes() }
        };
        let t = toggle.clone();
        listen(&btn, "click", move |_: MouseEvent| t());
        listen(&document(), "keydown", move |e: KeyboardEvent| {
            if e.key() != key || e.meta_key() || e.ctrl_key() || e.alt_key() || typing(e.target()) {
                return;
            }
            e.prevent_default();
            toggle();
        });
        let (v, root) = (vm.clone(), root.clone());
        Effect::new(move |_| {
            let open = if data == "inspector" { v.inspector_open() } else { v.notes_open() };
            if let Some(root) = &root {
                let _ = root.set_attribute(&format!("data-{data}"), if open { "open" } else { "closed" });
            }
            let _ = btn.set_attribute("aria-expanded", &open.to_string());
        });
    }
}

fn bind_tabs(vm: &Rc<ShellVm>) {
    for t in all(".notes .tab") {
        let (v, id) = (vm.clone(), t.id());
        listen(&t, "click", move |_: MouseEvent| v.show_tab(&id));
        let (v, id) = (vm.clone(), t.id());
        listen(&t, "keydown", move |e: KeyboardEvent| {
            if let Some(to) = v.tab_key(&id, &e.key()) {
                e.prevent_default();
                if let Some(el) = by_id(&to).and_then(|el| el.dyn_into::<HtmlElement>().ok()) {
                    let _ = el.focus();
                }
            }
        });
        let v = vm.clone();
        Effect::new(move |_| {
            let on = v.tab() == t.id();
            let _ = t.set_attribute("aria-selected", &on.to_string());
            t.unchecked_ref::<HtmlElement>().set_tab_index(if on { 0 } else { -1 });
            if let Some(panel) = t.get_attribute("aria-controls").and_then(|id| by_id(&id)) {
                panel.unchecked_into::<HtmlElement>().set_hidden(!on);
            }
        });
    }
}
