//! The design archive section of the Account page: it binds to `ArchiveVm` and decides nothing.

use std::rc::Rc;

use leptos::prelude::*;

use crate::archive::model::Choice;
use crate::archive::vm::ArchiveVm;
use crate::shared::i18n::Args;

type Vm = StoredValue<Rc<ArchiveVm>, LocalStorage>;

fn tr(vm: Vm, key: &'static str) -> String {
    vm.with_value(|v| v.i18n().tr(key, &Args::new()))
}

fn confirmation(vm: Vm) -> impl IntoView {
    view! {
        <div
            role="group"
            aria-labelledby="h-archive-confirm"
            aria-describedby="archive-confirm-summary"
            id="archive-confirm"
            class="confirm"
            on:keydown=move |e| {
                if e.key() == "Escape" {
                    e.stop_propagation();
                    vm.with_value(|v| v.cancel());
                }
            }
        >
            <h3 id="h-archive-confirm">{move || tr(vm, "archive-confirm-heading")}</h3>
            <p id="archive-confirm-summary">{move || vm.with_value(|v| v.summary_text())}</p>
            <p class="hint">{move || vm.with_value(|v| v.detail_text())}</p>
            <button type="button" id="archive-keep" class="btn" on:click=move |_| vm.with_value(|v| v.choose(Choice::KeepMine))>
                {move || tr(vm, "archive-confirm-keep")}
            </button>
            <button type="button" id="archive-use" class="btn danger" on:click=move |_| vm.with_value(|v| v.choose(Choice::UseArchive))>
                {move || tr(vm, "archive-confirm-use")}
            </button>
            <button type="button" id="archive-cancel" class="btn" on:click=move |_| vm.with_value(|v| v.cancel())>
                {move || tr(vm, "archive-confirm-cancel")}
            </button>
        </div>
    }
}

#[component]
pub fn ArchiveView(vm: Rc<ArchiveVm>) -> impl IntoView {
    let vm: Vm = StoredValue::new_local(vm);
    // Put focus where the person can read what happened or answer the question. Effects do not run on the host, so the
    // component tests do not depend on this.
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| {
        use crate::archive::vm::Focus;
        use wasm_bindgen::JsCast;
        let (_, to) = vm.with_value(|v| v.focus_request());
        let id = match to {
            Focus::None => return,
            Focus::Result => "archive-result",
            Focus::Safe => "archive-keep",
            Focus::Import => "archive-import",
        };
        if let Some(el) = document().get_element_by_id(id).and_then(|e| e.dyn_into::<leptos::web_sys::HtmlElement>().ok()) {
            let _ = el.focus();
        }
    });
    view! {
        <section class="account-section" aria-labelledby="h-archive">
            <h2 id="h-archive">{move || tr(vm, "archive-heading")}</h2>
            <p class="hint">{move || tr(vm, "archive-lead")}</p>
            <p class="menu-act">
                <button type="button" id="archive-export" class="btn" on:click=move |_| vm.with_value(|v| v.export())>
                    {move || tr(vm, "archive-export")}
                </button>
                <button type="button" id="archive-import" class="btn" aria-describedby="archive-import-help" on:click=move |_| vm.with_value(|v| v.import())>
                    {move || tr(vm, "archive-import")}
                </button>
            </p>
            <p class="hint" id="archive-import-help">
                {move || tr(vm, "archive-import-help")}
            </p>
            {move || vm.with_value(|v| v.confirming()).then(|| confirmation(vm))}
            {move || {
                vm.with_value(|v| v.result_text())
                    .map(|text| {
                        let class = if vm.with_value(|v| v.result_is_error()) { "hint archive-error" } else { "hint" };
                        view! {
                            <p id="archive-result" class=class tabindex="-1">
                                {text}
                            </p>
                        }
                    })
            }}
        </section>
    }
}
