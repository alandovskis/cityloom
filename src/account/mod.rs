//! The Account page: the person's name, the sign-ins linked to their account, where their work is kept, and deleting
//! the account. It reads and changes the account through the `Account` port.

use std::rc::Rc;

use leptos::prelude::*;
use wasm_bindgen::prelude::*;

use crate::shared::i18n::{I18n, Resources};
use crate::shared::platform::browser_ports;

#[cfg(test)]
mod tests;
pub mod text;
pub mod view;
pub mod vm;

/// What the Account page says, in both languages.
pub const RESOURCES: Resources = Resources { en: include_str!("i18n/en.ftl"), fr: include_str!("i18n/fr.ftl") };

/// The Account page as the script sees it.
#[wasm_bindgen]
pub struct AccountPage(Rc<I18n>);

#[wasm_bindgen]
impl AccountPage {
    /// Binds the shell every page shares (settings menu, sign-in menu, language) to this page.
    pub fn mount_shell(&self) {
        crate::shell::mount(self.0.clone(), Rc::new(crate::shell::Bare::default()), "details-details", false);
    }
}

/// Draws the Account page into `#account-slot` and loads the account.
#[wasm_bindgen]
pub fn mount_account() -> AccountPage {
    // Components create effects as they are built, before any is mounted.
    let _ = any_spawner::Executor::init_wasm_bindgen();
    let ports = browser_ports();
    let i18n = crate::i18n_browser(&ports);
    let vm = vm::AccountVm::new(ports.clone(), i18n.clone());
    let slot_id = "account-slot";
    let slot = leptos::prelude::document().get_element_by_id(slot_id).unwrap_or_else(|| panic!("the page has no #{slot_id}"));
    vm.load();
    leptos::mount::mount_to(slot.unchecked_into(), move || view! { <view::AccountView vm=vm/> }).forget();
    // The design archive works on this device's storage, signed in or not, so it has a slot of its own.
    if let Some(slot) = leptos::prelude::document().get_element_by_id("archive-slot") {
        let archive = crate::archive::vm::ArchiveVm::new(ports, i18n.clone());
        leptos::mount::mount_to(slot.unchecked_into(), move || view! { <crate::archive::view::ArchiveView vm=archive/> }).forget();
    }
    AccountPage(i18n)
}
