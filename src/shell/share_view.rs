//! The Share button of the header and its menu, drawn into the `#share-slot` of the editors' pages. The
//! words of the button and of the heading are set by the shell's popover binding (`view::bind_menu`).

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::i18n::{Args, I18n};

#[component]
pub fn ShareMenu(i18n: Rc<I18n>) -> impl IntoView {
    let i18n = StoredValue::new_local(i18n);
    view! {
        <button type="button" id="share-btn" class="btn quiet" aria-expanded="false" aria-controls="share-menu">
            <svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false">
                <path d="M8 10V2.5M5 5.5l3-3 3 3M3.5 8.5v5h9v-5"></path>
            </svg>
            <span id="share-btn-label"></span>
        </button>
        <div id="share-menu" class="menu" hidden>
            <p class="menu-h" id="share-heading"></p>
            <p class="menu-act">
                <button type="button" id="print" class="btn">
                    <svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false">
                        <path d="M4.5 6V2.5h7V6M4.5 11.5h-2V6h11v5.5h-2M4.5 9.5h7v4h-7z"></path>
                    </svg>
                    <span>{move || i18n.with_value(|i| i.tr("ui-print", &Args::new()))}</span>
                </button>
            </p>
        </div>
    }
}
