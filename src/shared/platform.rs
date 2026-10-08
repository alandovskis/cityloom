//! The browser's side of the ports a view-model is given.

use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;

use crate::shared::ports::{Announcer, Fetched, Fetcher, Importer, Navigator, Page, Ports, Scheduler, Storage};
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
        wasm_bindgen_futures::spawn_local(async move { done(request(&url, body.as_deref()).await) });
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

/// The OpenStreetMap reader, which the page script loads on first use and offers as
/// `window.cityloomImportOsm(bytes) -> Promise<string>`.
struct BrowserImporter;

impl Importer for BrowserImporter {
    #[cfg(target_arch = "wasm32")]
    fn import(&self, osm: Vec<u8>, bounds: Option<[f64; 4]>, done: Box<dyn FnOnce(Result<String, String>)>) {
        wasm_bindgen_futures::spawn_local(async move { done(read_osm(&osm, bounds).await) });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn import(&self, _osm: Vec<u8>, _bounds: Option<[f64; 4]>, done: Box<dyn FnOnce(Result<String, String>)>) {
        done(Err("there is no OpenStreetMap reader here".to_string()));
    }
}

#[cfg(target_arch = "wasm32")]
async fn read_osm(osm: &[u8], bounds: Option<[f64; 4]>) -> Result<String, String> {
    use wasm_bindgen::JsCast;
    let words = |e: wasm_bindgen::JsValue| {
        e.as_string()
            .or_else(|| js_sys::Reflect::get(&e, &"message".into()).ok().and_then(|m| m.as_string()))
            .unwrap_or_else(|| "the map data could not be read".to_string())
    };
    let window = web_sys::window().ok_or("there is no window")?;
    let read = js_sys::Reflect::get(&window, &"cityloomImportOsm".into()).map_err(words)?;
    let read: js_sys::Function = read.dyn_into().map_err(|_| "the page has no OpenStreetMap reader".to_string())?;
    let bounds = match bounds {
        Some(b) => js_sys::Float64Array::from(&b[..]),
        None => js_sys::Float64Array::new_with_length(0),
    };
    let promise = read.call2(&wasm_bindgen::JsValue::NULL, &js_sys::Uint8Array::from(osm), &bounds).map_err(words)?;
    let json = wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(promise)).await.map_err(words)?;
    json.as_string().ok_or_else(|| "the map data could not be read".to_string())
}

struct BrowserNavigator;

impl Navigator for BrowserNavigator {
    fn go(&self, href: &str) {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = leptos::prelude::window().location().set_href(href);
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = href;
    }
}

struct BrowserPage;

impl Page for BrowserPage {
    fn language(&self) -> Option<String> {
        #[cfg(target_arch = "wasm32")]
        {
            let navigator: web_sys::Navigator = leptos::prelude::window().navigator();
            navigator.language()
        }
        #[cfg(not(target_arch = "wasm32"))]
        None
    }

    fn set_lang(&self, tag: &str) {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(root) = leptos::prelude::document().document_element() {
                let _ = root.set_attribute("lang", tag);
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = tag;
    }
}

#[wasm_bindgen]
extern "C" {
    /// The map of the page: the adapter `createBasemap` makes in `web/basemap.js`.
    pub type Basemap;
    #[wasm_bindgen(method, js_name = setPlaces)]
    fn set_places(this: &Basemap, layers: &str, data: &str);
    #[wasm_bindgen(method)]
    fn fit(this: &Basemap, west: f64, south: f64, east: f64, north: f64, top: f64, right: f64, bottom: f64, left: f64);
    #[wasm_bindgen(method, js_name = zoomBy)]
    fn zoom_by(this: &Basemap, factor: f64);
    #[wasm_bindgen(method, js_name = panBy)]
    fn pan_by(this: &Basemap, dx: f64, dy: f64);
    #[wasm_bindgen(method)]
    fn highlight(this: &Basemap, hot: Option<String>);
    #[wasm_bindgen(method, js_name = setImperial)]
    fn set_imperial(this: &Basemap, imperial: bool);
    #[wasm_bindgen(method)]
    fn listen(this: &Basemap, on: &js_sys::Function);
}

/// The map on the page, as a `Mapper`.
pub struct BrowserMapper(Basemap);

impl BrowserMapper {
    pub fn new(basemap: Basemap) -> BrowserMapper {
        BrowserMapper(basemap)
    }
}

impl crate::shared::ports::Mapper for BrowserMapper {
    fn set_places(&self, layers: &str, data: &str) {
        self.0.set_places(layers, data);
    }
    fn fit(&self, [w, s, e, n]: [f64; 4], [top, right, bottom, left]: [f64; 4]) {
        self.0.fit(w, s, e, n, top, right, bottom, left);
    }
    fn zoom_by(&self, factor: f64) {
        self.0.zoom_by(factor);
    }
    fn pan_by(&self, dx: f64, dy: f64) {
        self.0.pan_by(dx, dy);
    }
    fn highlight(&self, hot: Option<&str>) {
        self.0.highlight(hot.map(String::from));
    }
    fn set_imperial(&self, imperial: bool) {
        self.0.set_imperial(imperial);
    }
    fn listen(&self, on: Box<dyn Fn(crate::shared::ports::MapEvent)>) {
        let callback = Closure::<dyn Fn(String)>::new(move |json: String| {
            if let Ok(event) = serde_json::from_str(&json) {
                on(event);
            }
        });
        self.0.listen(callback.as_ref().unchecked_ref());
        callback.forget();
    }
}

/// What a view-model gets on a page: the live region and local storage.
pub fn browser_ports() -> Ports {
    Ports {
        navigator: Rc::new(BrowserNavigator),
        importer: Rc::new(BrowserImporter),
        fetcher: Rc::new(BrowserFetcher),
        announcer: Rc::new(BrowserAnnouncer),
        storage: Rc::new(BrowserStorage),
        scheduler: Rc::new(BrowserScheduler::default()),
        mapper: Rc::new(crate::shared::ports::NoMapper),
        page: Rc::new(BrowserPage),
    }
}
