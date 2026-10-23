//! The browser's side of the ports a view-model is given.

use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;

// `account_error` is imported inside `api` below, where only the wasm32 build uses it: a host build would warn about an unused import, and CI builds with `-D warnings`.
use crate::shared::ports::{
    Account, AccountError, Announcer, Answer, Fetched, Fetcher, Importer, Me, Navigator, Page, Ports, ProviderInfo, Scheduler, Storage, WorkAnswer, WorkAsk,
    WorkError, WorkPage, WorkPushed, WorkPut, WorkSync,
};
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

    fn keys(&self) -> Vec<String> {
        store::keys()
    }

    fn now_ms(&self) -> u64 {
        #[cfg(target_arch = "wasm32")]
        {
            js_sys::Date::now() as u64
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
        }
    }

    fn random_hex(&self) -> Option<String> {
        Some(crate::shared::random::hex128())
    }

    fn identity(&self) -> usize {
        // The page's one localStorage, whatever handle reached it: browser_ports() makes a new handle at every mount.
        0
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
    fn prepare(&self) {
        use wasm_bindgen::JsCast;
        let Some(window) = web_sys::window() else { return };
        if let Some(prepare) = js_sys::Reflect::get(&window, &"cityloomPrepareOsm".into()).ok().and_then(|f| f.dyn_into::<js_sys::Function>().ok()) {
            let _ = prepare.call0(&window);
        }
    }

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

struct BrowserAccount;

#[cfg(target_arch = "wasm32")]
async fn api(method: &str, path: &str, json: Option<&str>) -> Result<String, AccountError> {
    use crate::shared::ports::account_error;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    let init = web_sys::RequestInit::new();
    init.set_method(method);
    if let Some(json) = json {
        init.set_body(&wasm_bindgen::JsValue::from_str(json));
        let headers = web_sys::Headers::new().map_err(|_| AccountError::Unavailable)?;
        headers.set("content-type", "application/json").map_err(|_| AccountError::Unavailable)?;
        init.set_headers(&headers);
    }
    let request = web_sys::Request::new_with_str_and_init(path, &init).map_err(|_| AccountError::Unavailable)?;
    let window = web_sys::window().ok_or(AccountError::Unavailable)?;
    // The browser sets `Origin` itself on a same-origin PUT, POST or DELETE; cookies go along (same-origin default).
    let response: web_sys::Response =
        JsFuture::from(window.fetch_with_request(&request)).await.map_err(|_| AccountError::Unavailable)?.dyn_into().map_err(|_| AccountError::Unavailable)?;
    if !response.ok() {
        return Err(account_error(response.status()));
    }
    let text = JsFuture::from(response.text().map_err(|_| AccountError::Unavailable)?).await.map_err(|_| AccountError::Unavailable)?;
    Ok(text.as_string().unwrap_or_default())
}

/// Runs the request and tells `done`; natively (no network) the answer is at once `Unavailable`.
fn ask<T: 'static>(method: &'static str, path: String, json: Option<String>, read: fn(&str) -> Result<T, AccountError>, done: Answer<T>) {
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(async move { done(api(method, &path, json.as_deref()).await.and_then(|body| read(&body))) });
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (method, path, json, read);
        done(Err(AccountError::Unavailable));
    }
}

fn nothing(_: &str) -> Result<(), AccountError> {
    Ok(())
}

impl Account for BrowserAccount {
    fn me(&self, done: Answer<Me>) {
        ask("GET", "/api/me".into(), None, Me::parse, done);
    }
    fn providers(&self, done: Answer<Vec<ProviderInfo>>) {
        ask("GET", "/api/providers".into(), None, ProviderInfo::parse_list, done);
    }
    fn set_storage(&self, store: bool, done: Answer<()>) {
        ask("PUT", "/api/me/storage".into(), Some(format!("{{\"store_on_server\":{store}}}")), nothing, done);
    }
    fn unlink(&self, identity_id: &str, done: Answer<()>) {
        ask("DELETE", format!("/api/me/identities/{identity_id}"), None, nothing, done);
    }
    fn delete(&self, done: Answer<()>) {
        ask("DELETE", "/api/me".into(), None, nothing, done);
    }
    fn sign_out(&self, done: Answer<()>) {
        ask("POST", "/auth/signout".into(), None, nothing, done);
    }
}

struct BrowserWork;

#[cfg(target_arch = "wasm32")]
async fn work_api(method: &str, path: &str, json: Option<&str>, keepalive: bool) -> Result<String, WorkError> {
    use crate::shared::ports::work_error;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    let init = web_sys::RequestInit::new();
    init.set_method(method);
    if keepalive {
        // Lets the request outlive the page that is being left. Set by name: not every web-sys release types it.
        let _ = js_sys::Reflect::set(init.as_ref(), &"keepalive".into(), &true.into());
    }
    if let Some(json) = json {
        init.set_body(&wasm_bindgen::JsValue::from_str(json));
        let headers = web_sys::Headers::new().map_err(|_| WorkError::Unavailable)?;
        headers.set("content-type", "application/json").map_err(|_| WorkError::Unavailable)?;
        init.set_headers(&headers);
    }
    let request = web_sys::Request::new_with_str_and_init(path, &init).map_err(|_| WorkError::Unavailable)?;
    let window = web_sys::window().ok_or(WorkError::Unavailable)?;
    let response: web_sys::Response =
        JsFuture::from(window.fetch_with_request(&request)).await.map_err(|_| WorkError::Unavailable)?.dyn_into().map_err(|_| WorkError::Unavailable)?;
    let status = response.status();
    let text = match response.text() {
        Ok(promise) => JsFuture::from(promise).await.ok().and_then(|t| t.as_string()).unwrap_or_default(),
        Err(_) => String::new(),
    };
    if response.ok() { Ok(text) } else { Err(work_error(status, &text)) }
}

/// Runs the request and tells `done`; natively (no network) the answer is at once `Unavailable`.
fn ask_work<T: 'static>(
    method: &'static str,
    path: String,
    json: Option<String>,
    keepalive: bool,
    read: fn(&str) -> Result<T, WorkError>,
    done: WorkAnswer<T>,
) {
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(async move { done(work_api(method, &path, json.as_deref(), keepalive).await.and_then(|body| read(&body))) });
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (method, path, json, keepalive, read);
        done(Err(WorkError::Unavailable));
    }
}

fn nothing_work(_: &str) -> Result<(), WorkError> {
    Ok(())
}

impl WorkSync for BrowserWork {
    fn pull(&self, ask: WorkAsk, done: WorkAnswer<WorkPage>) {
        ask_work("GET", format!("/api/work?{}", ask.query()), None, false, WorkPage::parse, done);
    }
    fn push(&self, batch: Vec<WorkPut>, keepalive: bool, done: WorkAnswer<WorkPushed>) {
        let body = serde_json::json!({ "records": batch }).to_string();
        ask_work("PUT", "/api/work".into(), Some(body), keepalive, WorkPushed::parse, done);
    }
    fn clear(&self, done: WorkAnswer<()>) {
        ask_work("DELETE", "/api/work".into(), None, false, nothing_work, done);
    }
}

struct BrowserPage;

impl Page for BrowserPage {
    fn path(&self) -> String {
        #[cfg(target_arch = "wasm32")]
        {
            let location = leptos::prelude::window().location();
            format!("{}{}", location.pathname().unwrap_or_else(|_| "/".into()), location.search().unwrap_or_default())
        }
        #[cfg(not(target_arch = "wasm32"))]
        "/".to_string()
    }

    fn take_fragment(&self) -> Option<String> {
        #[cfg(target_arch = "wasm32")]
        {
            let window = leptos::prelude::window();
            let location = window.location();
            let hash = location.hash().ok().filter(|h| h.len() > 1 && super::ports::is_account_fragment(&h[1..]))?;
            let clean = format!("{}{}", location.pathname().unwrap_or_default(), location.search().unwrap_or_default());
            if let Ok(history) = window.history() {
                let _ = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&clean));
            }
            Some(hash[1..].to_string())
        }
        #[cfg(not(target_arch = "wasm32"))]
        None
    }

    fn languages(&self) -> Vec<String> {
        #[cfg(target_arch = "wasm32")]
        {
            let navigator: web_sys::Navigator = leptos::prelude::window().navigator();
            let list: Vec<String> = navigator.languages().iter().filter_map(|tag| tag.as_string()).collect();
            if list.is_empty() { navigator.language().into_iter().collect() } else { list }
        }
        #[cfg(not(target_arch = "wasm32"))]
        Vec::new()
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
    #[wasm_bindgen(method, js_name = setRestrictions)]
    fn set_restrictions(this: &Basemap, layers: &str, data: &str);
    #[wasm_bindgen(method, js_name = setDraft)]
    fn set_draft(this: &Basemap, layers: &str, data: &str);
    #[wasm_bindgen(method, js_name = setZoneTicks)]
    fn set_zone_ticks(this: &Basemap, layers: &str, data: &str);
    /// The middle of the view, as a `Float64Array` of longitude and latitude.
    #[wasm_bindgen(method)]
    fn centre(this: &Basemap) -> Vec<f64>;
    #[wasm_bindgen(method)]
    fn fit(this: &Basemap, bounds: &[f64], insets: &[f64]);
    #[wasm_bindgen(method, js_name = setInsets)]
    fn set_insets(this: &Basemap, insets: &[f64]);
    #[wasm_bindgen(method, js_name = setEditable)]
    fn set_editable(this: &Basemap, hot: Vec<String>);
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
    fn set_restrictions(&self, layers: &str, data: &str) {
        self.0.set_restrictions(layers, data);
    }
    fn set_draft(&self, layers: &str, data: &str) {
        self.0.set_draft(layers, data);
    }
    fn set_zone_ticks(&self, layers: &str, data: &str) {
        self.0.set_zone_ticks(layers, data);
    }
    fn centre(&self) -> [f64; 2] {
        let c = self.0.centre();
        [c.first().copied().unwrap_or(0.0), c.get(1).copied().unwrap_or(0.0)]
    }
    fn fit(&self, bounds: [f64; 4], insets: [f64; 4]) {
        self.0.fit(&bounds, &insets);
    }
    fn set_insets(&self, insets: [f64; 4]) {
        self.0.set_insets(&insets);
    }
    fn set_editable(&self, hot: &[String]) {
        self.0.set_editable(hot.to_vec());
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
            // An event the module cannot read is dropped, but said in the console: the adapter and the module disagree.
            match serde_json::from_str(&json) {
                Ok(event) => on(event),
                Err(error) => web_sys::console::warn_2(&format!("The map said something the page cannot read ({error}):").into(), &json.into()),
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
        account: Rc::new(BrowserAccount),
        work: Rc::new(BrowserWork),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn natively_the_browser_account_says_unavailable_and_the_page_has_a_root_path_and_no_fragment() {
        let ports = browser_ports();
        let got = Rc::new(RefCell::new(None));
        let g = got.clone();
        ports.account.me(Box::new(move |r| *g.borrow_mut() = Some(r.map(|m| m.id))));
        assert_eq!(*got.borrow(), Some(Err(AccountError::Unavailable)));
        assert_eq!(ports.page.path(), "/");
        assert_eq!(ports.page.take_fragment(), None);
    }

    #[test]
    fn natively_the_browser_work_says_unavailable_and_the_clock_and_randomness_work() {
        let ports = browser_ports();
        let got = Rc::new(RefCell::new(None));
        let g = got.clone();
        ports.work.pull(crate::shared::ports::WorkAsk::first(0), Box::new(move |r| *g.borrow_mut() = Some(r.map(|p| p.rev))));
        assert_eq!(*got.borrow(), Some(Err(crate::shared::ports::WorkError::Unavailable)));
        assert!(ports.storage.now_ms() > 1_700_000_000_000);
        let (a, b) = (ports.storage.random_hex().unwrap(), ports.storage.random_hex().unwrap());
        assert_eq!((a.len(), a == b), (32, false));
    }

    #[test]
    fn every_browser_storage_is_the_same_storage_whatever_handle_reaches_it() {
        // each mount calls browser_ports() again, and the editor's store and the shell's sync must still meet
        assert_eq!(browser_ports().storage.identity(), browser_ports().storage.identity());
    }
}
