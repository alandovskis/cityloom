//! The browser's side of the ports a view-model is given.

use std::rc::Rc;

use crate::shared::ports::{Announcer, Fetched, Fetcher, Ports, Scheduler, Storage};
use crate::shared::{live, store};

struct BrowserAnnouncer;

impl Announcer for BrowserAnnouncer {
    fn say(&self, text: &str) {
        live::say(text);
    }
}

struct BrowserStorage;

impl Storage for BrowserStorage {
    fn recall(&self, key: &str) -> Option<String> {
        store::recall(key)
    }

    fn remember(&self, key: &str, value: &str) -> bool {
        store::remember(key, value)
    }
}

/// Timers, kept by id so that they can be called off.
#[derive(Default)]
struct BrowserScheduler {
    next: std::cell::Cell<u32>,
    #[cfg(target_arch = "wasm32")]
    handles: std::rc::Rc<std::cell::RefCell<std::collections::HashMap<u32, leptos::leptos_dom::helpers::TimeoutHandle>>>,
}

impl Scheduler for BrowserScheduler {
    #[cfg(target_arch = "wasm32")]
    fn after(&self, ms: u32, f: Box<dyn FnOnce()>) -> u32 {
        let id = self.next.get() + 1;
        self.next.set(id);
        let handles = self.handles.clone();
        let done = leptos::prelude::set_timeout_with_handle(
            move || {
                handles.borrow_mut().remove(&id);
                f();
            },
            std::time::Duration::from_millis(ms as u64),
        );
        if let Ok(h) = done {
            self.handles.borrow_mut().insert(id, h);
        }
        id
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn after(&self, _ms: u32, _f: Box<dyn FnOnce()>) -> u32 {
        let id = self.next.get() + 1;
        self.next.set(id);
        id
    }

    #[cfg(target_arch = "wasm32")]
    fn cancel(&self, id: u32) {
        if let Some(h) = self.handles.borrow_mut().remove(&id) {
            h.clear();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn cancel(&self, _id: u32) {}
}

struct BrowserFetcher;

impl Fetcher for BrowserFetcher {
    #[cfg(target_arch = "wasm32")]
    fn fetch(&self, url: &str, body: Option<&str>, done: Box<dyn FnOnce(Fetched)>) {
        let (url, body) = (url.to_string(), body.map(str::to_string));
        leptos::task::spawn_local(async move { done(request(&url, body.as_deref()).await) });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn fetch(&self, _url: &str, _body: Option<&str>, done: Box<dyn FnOnce(Fetched)>) {
        done(Err("there is no network here".to_string()));
    }
}

#[cfg(target_arch = "wasm32")]
async fn request(url: &str, body: Option<&str>) -> Fetched {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    let words = |e: wasm_bindgen::JsValue| e.as_string().unwrap_or_else(|| "the request failed".to_string());
    let init = web_sys::RequestInit::new();
    if let Some(body) = body {
        init.set_method("POST");
        init.set_body(&wasm_bindgen::JsValue::from_str(body));
    }
    let request = web_sys::Request::new_with_str_and_init(url, &init).map_err(words)?;
    let window = web_sys::window().ok_or("there is no window")?;
    let response: web_sys::Response = JsFuture::from(window.fetch_with_request(&request)).await.map_err(words)?.dyn_into().map_err(words)?;
    if !response.ok() {
        return Err(format!("the server answered {}", response.status()));
    }
    let buffer = JsFuture::from(response.array_buffer().map_err(words)?).await.map_err(words)?;
    Ok(js_sys::Uint8Array::new(&buffer).to_vec())
}

/// What a view-model gets on a page: the live region and local storage.
pub fn browser_ports() -> Ports {
    Ports {
        fetcher: Rc::new(BrowserFetcher),
        announcer: Rc::new(BrowserAnnouncer),
        storage: Rc::new(BrowserStorage),
        scheduler: Rc::new(BrowserScheduler::default()),
    }
}
