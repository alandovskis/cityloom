//! The city map's view-model: how the city reads as places to open, what needs
//! attention and what has changed, where the camera is, and what pressing
//! "start over" means. The view binds to its properties and sends it commands.

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::{Rc, Weak};

use leptos::prelude::*;

use crate::city::model::{City, CityView, EdgeView, NodeView};
use crate::city::projection::Projection;
use crate::city::store::CityStore;
use crate::map::areas::AreasVm;
use crate::map::camera::{Camera, Insets, World};
use crate::map::editable;
use crate::map::overlay;
use crate::map::style;
use crate::restriction::draw::Draw;
use crate::restriction::model::{Conflict, Restriction, Rule, conflicts};
use crate::shared::core::{Core, Presents};
use crate::shared::i18n::{Args, I18n, Locale};
use crate::shared::ports::{MapEvent, Ports};
use crate::shared::said::{self, Arg, Said};
use crate::shared::units::Units;

/// How far a zoom button, or a key, zooms at once.
const ZOOM_STEP: f64 = 1.4;
/// How far an arrow key pans, in pixels.
const PAN_PX: f64 = 80.0;

/// What the map page says where it has no map to show: the keys of its messages.
pub const MISSING: &str = "map-basemap-missing";
pub const OUTSIDE: &str = "map-basemap-outside";
pub const NO_ROADS: &str = "map-no-roads";

/// Whether the basemap is there to draw the places on.
#[derive(Clone, Debug, PartialEq)]
pub enum BasemapState {
    /// Asked for; the tiles have not answered.
    Waiting,
    Ready,
    /// There will be no map, for the reason the message with this key gives.
    Unavailable(&'static str),
}

/// Text as the search compares it: lower case, with the typographic apostrophe and the no-break spaces
/// the French page shows made the plain ones a keyboard types.
fn fold(text: &str) -> String {
    text.to_lowercase().replace('\u{2019}', "'").replace(['\u{a0}', '\u{202f}'], " ")
}

/// The place a `hot` name (`s-7`, `j-3`) opens.
fn href_of(hot: &str) -> Option<String> {
    let (kind, uid) = hot.split_once('-')?;
    let uid: u32 = uid.parse().ok()?;
    match kind {
        "s" => Some(street_href(uid)),
        "j" => Some(junction_href(uid)),
        _ => None,
    }
}

struct CityModel {
    city: City,
    region: usize,
}

impl Presents for CityModel {
    type View = CityView;
    fn present(&self) -> CityView {
        self.city.view(self.region)
    }
}

// ---- words ------------------------------------------------------------------------------
//
// What the page says is worded with the tracked `tr`/`say`: a view that reads a property is drawn again when
// the language is switched. A command words what it announces with `tr_now`.

/// `1 street`, `3 streets`, as data to be worded.
fn streets(n: usize) -> Arg {
    Arg::Said(Box::new(Said::new("map-streets").with("n", Arg::Num(n as i64))))
}

/// A street's sentence with its zone count added when it has several (the `-zones` messages).
fn with_zones(said: Said, n: usize) -> Said {
    if n > 1 { said.with("zones", zones(n)) } else { said }
}

/// `1 zone`, `3 zones`, as data to be worded.
fn zones(n: usize) -> Arg {
    Arg::Said(Box::new(Said::new("map-zones").with("n", Arg::Num(n as i64))))
}

/// What a city's place says, in the language of the page.
fn all_said(i18n: &I18n, units: Units, said: &[Said]) -> Vec<String> {
    said.iter().map(|s| said::say(i18n, units, s)).collect()
}

/// How a place is told to a screen reader: what it is, what it needs, whether it is changed, and what it
/// opens, one sentence after another.
fn label(i18n: &I18n, units: Units, what: String, (ok, failing, edited): (bool, &[Said], bool), opens: &str) -> String {
    let mut sentences = vec![what];
    if !ok {
        let failing = all_said(i18n, units, failing).join(", ").to_lowercase();
        sentences.push(i18n.tr("map-needs-attention", &Args::new().str("failing", failing)));
    }
    if edited {
        sentences.push(i18n.tr("map-changed", &Args::new()));
    }
    sentences.push(i18n.tr(opens, &Args::new()));
    sentences.join(" ")
}

/// The junction's control, named by the junction's message for it.
fn control(n: &NodeView, lower: bool) -> Arg {
    let key = n.control.unwrap_or("jn-control-uncontrolled");
    if lower { Arg::MsgLower(key) } else { Arg::Msg(key) }
}

/// How a junction on the map is told to a screen reader.
pub fn junction_label(n: &NodeView, i18n: &I18n, units: Units) -> String {
    let what = said::say(
        i18n,
        units,
        &Said::new("map-label-junction").with("name", Arg::Said(Box::new(n.name.clone()))).with("control", control(n, true)).with("streets", streets(n.arms)),
    );
    label(i18n, units, what, (n.ok, &n.failing, n.edited), "map-opens-junction")
}

/// How a street on the map is told to a screen reader.
pub fn street_label(e: &EdgeView, i18n: &I18n, units: Units) -> String {
    let what = said::say(i18n, units, &{
        with_zones(
            Said::new(if e.zones > 1 { "map-label-street-zones" } else { "map-label-street" })
                .with("street", Arg::Said(Box::new(e.called.clone())))
                .with("ends", Arg::Said(Box::new(e.ends.clone())))
                .with("width", Arg::Text(units.length_in(e.row_mm, i18n.locale()))),
            e.zones,
        )
    });
    label(i18n, units, what, (e.ok, &e.failing, e.edited), "map-opens-street")
}

pub fn street_href(uid: u32) -> String {
    format!("street.html?street={uid}")
}

pub fn junction_href(uid: u32) -> String {
    format!("intersection.html?junction={uid}")
}

// ---- what the view binds to ----------------------------------------------------------------

/// A place to open: in a list, or as a link on the map. Its name and small print are data, worded by the view
/// in the language of the page.
#[derive(Clone, Debug, PartialEq)]
pub struct PlaceRow {
    pub href: String,
    /// What the map and the list highlight together: `j-3` or `s-7`.
    pub hot: String,
    pub name: Said,
    pub sub: Said,
    pub tag: Option<StateTag>,
}

/// How a place stands, where it needs saying: the key of the words, and whether it is bad news.
#[derive(Clone, Debug, PartialEq)]
pub struct StateTag {
    pub key: &'static str,
    pub bad: bool,
}

/// A place in the notes: a link and one line about it.
#[derive(Clone, Debug, PartialEq)]
pub struct NoteItem {
    pub href: String,
    pub name: String,
    pub detail: String,
}

/// A street inside an area, and what it has that the area's rule does not allow.
#[derive(Clone, Debug, PartialEq)]
pub struct AreaStreet {
    /// The area's id.
    pub area: String,
    pub uid: u32,
    pub href: String,
    /// The street and where it runs, as the notes name it.
    pub name: Said,
    pub rule: Rule,
    pub conflicts: Vec<Conflict>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Status {
    pub text: String,
    pub bad: bool,
}

/// The junction rows and the street rows, each with its folded text (see `MapVm::worded_rows`).
type WordedRows = (Vec<(PlaceRow, String)>, Vec<(PlaceRow, String)>);

/// `WordedRows`, for the city view, language and units they were worded for.
struct Worded {
    view: Rc<CityView>,
    locale: Locale,
    units: Units,
    rows: Rc<WordedRows>,
}

pub struct MapVm {
    me: Weak<MapVm>,
    core: Core<CityModel>,
    store: CityStore,
    ports: Ports,
    i18n: Rc<I18n>,
    camera: ArcRwSignal<Camera>,
    basemap: ArcRwSignal<BasemapState>,
    moved: Cell<bool>,
    /// The part of the map the panels leave open, once the map has said it.
    viewport: Cell<Option<[f64; 4]>>,
    /// The places the open view makes editable, as last sent to the map; `None` until there is a view.
    editable: RefCell<Option<Vec<String>>>,
    hot: ArcRwSignal<Option<String>>,
    search: ArcRwSignal<String>,
    /// The result the arrow keys have moved to, by index into `results`.
    active: ArcRwSignal<Option<usize>>,
    /// The places worded for the search, until the city, the language or the units change.
    worded: RefCell<Option<Worded>>,
    /// The area restrictions: the Area rules panel, and what the map shows of them.
    areas: Rc<AreasVm>,
    /// In the tests: how many streets were read to check them against an area's rule.
    #[cfg(test)]
    editors_read: Cell<usize>,
}

impl MapVm {
    /// The city of the area chosen, saying what it says in the language of `i18n` (the one the shell
    /// switches). When that area's roads could not be had it is a city of no places, named after the area.
    pub fn new(ports: Ports, i18n: Rc<I18n>) -> Rc<MapVm> {
        let store = CityStore::current(ports.storage.clone());
        MapVm::over(store, ports, i18n)
    }

    /// In the tests: the page over the hand-made city they are written against.
    #[cfg(test)]
    pub fn on_sample(ports: Ports, i18n: Rc<I18n>) -> Rc<MapVm> {
        let store = CityStore::sample(ports.storage.clone());
        MapVm::over(store, ports, i18n)
    }

    fn over(store: CityStore, ports: Ports, i18n: Rc<I18n>) -> Rc<MapVm> {
        let has_roads = store.has_network();
        let model = CityModel { city: store.open(), region: store.region() };
        let core = Core::new(model);
        let world = World::round(core.view_now().bounds_mm);
        let basemap = if has_roads { BasemapState::Waiting } else { BasemapState::Unavailable(NO_ROADS) };
        let areas = AreasVm::new(ports.clone(), i18n.clone());
        Rc::new_cyclic(|me| MapVm {
            me: me.clone(),
            core,
            store,
            ports,
            i18n,
            camera: ArcRwSignal::new(Camera::new(world, 800.0, 520.0)),
            basemap: ArcRwSignal::new(basemap),
            moved: Cell::new(false),
            viewport: Cell::new(None),
            editable: RefCell::new(None),
            hot: ArcRwSignal::new(None),
            search: ArcRwSignal::new(String::new()),
            active: ArcRwSignal::new(None),
            worded: RefCell::new(None),
            areas,
            #[cfg(test)]
            editors_read: Cell::new(0),
        })
    }

    // ---- properties ----

    pub fn view(&self) -> Rc<CityView> {
        self.core.view()
    }

    pub fn units(&self) -> Units {
        self.core.units()
    }

    /// The page's words, which the shell switches.
    pub fn i18n(&self) -> &Rc<I18n> {
        &self.i18n
    }

    /// The message `key`, for a view: drawn again when the language is switched.
    pub fn word(&self, key: &str) -> String {
        self.i18n.tr(key, &Args::new())
    }

    /// What the city says, for a view: drawn again when the language or the units change.
    pub fn say(&self, said: &Said) -> String {
        said::say(&self.i18n, self.units(), said)
    }

    pub fn title(&self) -> String {
        self.view().name.to_string()
    }

    fn junctions(v: &CityView) -> Vec<&NodeView> {
        let mut j: Vec<&NodeView> = v.nodes.iter().filter(|n| n.junction).collect();
        j.sort_by_key(|n| n.number);
        j
    }

    /// `9 junctions, 32 streets`.
    pub fn counts(&self) -> String {
        let v = self.view();
        let junctions = Said::new("map-junction-count").with("n", Arg::Num(Self::junctions(&v).len() as i64));
        self.say(&Said::new("map-counts").with("junctions", Arg::Said(Box::new(junctions))).with("streets", streets(v.edges.len())))
    }

    pub fn places(&self) -> usize {
        self.view().places
    }

    /// A city with nothing in it: an area whose roads could not be had, or that has none.
    fn has_no_places(&self) -> bool {
        self.places() == 0
    }

    pub fn changes(&self) -> usize {
        self.view().edited
    }

    fn tag(ok: bool, edited: bool) -> Option<StateTag> {
        if !ok { Some(StateTag { key: "map-state-bad", bad: true }) } else { edited.then_some(StateTag { key: "map-state-changed", bad: false }) }
    }

    /// The junctions, in the order of their numbers, as rows to open.
    pub fn junction_rows(&self) -> Vec<PlaceRow> {
        let v = self.view();
        Self::junctions(&v)
            .into_iter()
            .map(|n| PlaceRow {
                href: junction_href(n.uid),
                hot: format!("j-{}", n.uid),
                name: n.name.clone(),
                sub: Said::new("map-junction-sub").with("control", control(n, false)).with("streets", streets(n.arms)),
                tag: Self::tag(n.ok, n.edited),
            })
            .collect()
    }

    /// The streets, as rows to open. The width in the small print is written in the units and with the
    /// decimal mark of the page now; the rows are made again when either changes.
    pub fn street_rows(&self) -> Vec<PlaceRow> {
        let (v, units, locale) = (self.view(), self.units(), self.i18n.locale());
        v.edges
            .iter()
            .map(|e| PlaceRow {
                href: street_href(e.uid),
                hot: format!("s-{}", e.uid),
                name: e.called.clone(),
                sub: {
                    with_zones(
                        Said::new(if e.zones > 1 { "map-street-sub-zones" } else { "map-street-sub" })
                            .with("ends", Arg::Said(Box::new(e.ends.clone())))
                            .with("width", Arg::Text(units.length_in(e.row_mm, locale))),
                        e.zones,
                    )
                },
                tag: Self::tag(e.ok, e.edited),
            })
            .collect()
    }

    // ---- search ----

    /// What was typed in the search box.
    pub fn search_text(&self) -> String {
        self.search.get()
    }

    pub fn set_search(&self, text: &str) {
        self.search.set(text.to_string());
        self.active.set(None);
    }

    /// The places the search finds: the junctions, then the streets.
    pub fn results(&self) -> Vec<PlaceRow> {
        if self.terms().is_empty() {
            return Vec::new();
        }
        let (j, s) = self.narrowed();
        j.into_iter().chain(s).collect()
    }

    /// How many results the dropdown shows before it says how many more there are.
    pub const SHOWN: usize = 8;

    /// The result the arrow keys are on.
    pub fn active_result(&self) -> Option<usize> {
        self.active.get()
    }

    /// Moves down (1) or up (-1) the results shown, and stops at the ends.
    pub fn move_active(&self, by: i32) {
        let n = self.results().len().min(Self::SHOWN) as i32;
        if n == 0 {
            return;
        }
        let at = self.active.get_untracked().map_or(-1, |i| i as i32);
        self.active.set(Some((at + by).clamp(0, n - 1) as usize));
    }

    /// The words searched for, lower case.
    fn terms(&self) -> Vec<String> {
        fold(&self.search.get()).split_whitespace().map(String::from).collect()
    }

    /// The junction rows and the street rows, each with the folded text the page shows for it, worded once
    /// for the city, the language and the units they were worded for. Reading it is tracked, so a closure
    /// that reads it draws again when any of the three changes; typing does not word the places again.
    fn worded_rows(&self) -> Rc<WordedRows> {
        let (view, locale, units) = (self.view(), self.i18n.locale(), self.units());
        if let Some(w) = &*self.worded.borrow()
            && Rc::ptr_eq(&w.view, &view)
            && w.locale == locale
            && w.units == units
        {
            return w.rows.clone();
        }
        let word = |rows: Vec<PlaceRow>| -> Vec<(PlaceRow, String)> {
            rows.into_iter()
                .map(|r| {
                    let text = fold(&format!("{} {}", self.say(&r.name), self.say(&r.sub)));
                    (r, text)
                })
                .collect()
        };
        let rows = Rc::new((word(self.junction_rows()), word(self.street_rows())));
        *self.worded.borrow_mut() = Some(Worded { view, locale, units, rows: rows.clone() });
        rows
    }

    /// The junctions and the streets that the search leaves. The whole phrase is
    /// looked for in a place's name and small print first, so that "junction 4"
    /// finds Junction 4 and the streets that end there and not every junction of
    /// four streets; when that finds nothing, each word is looked for on its own.
    /// What is looked in is the text the page shows, in its language, worded by
    /// `worded_rows`, so the results follow a switch of language or units.
    fn narrowed(&self) -> (Vec<PlaceRow>, Vec<PlaceRow>) {
        let terms = self.terms();
        let worded = self.worded_rows();
        let (junctions, streets) = (&worded.0, &worded.1);
        let phrase = terms.join(" ");
        let keep =
            |rows: &[(PlaceRow, String)], found: &dyn Fn(&str) -> bool| rows.iter().filter(|(_, t)| found(t)).map(|(r, _)| r.clone()).collect::<Vec<_>>();
        let by_phrase = |t: &str| t.contains(&phrase);
        let by_words = |t: &str| terms.iter().all(|w| t.contains(w.as_str()));
        let (j, st) = (keep(junctions, &by_phrase), keep(streets, &by_phrase));
        if j.is_empty() && st.is_empty() { (keep(junctions, &by_words), keep(streets, &by_words)) } else { (j, st) }
    }

    /// Whether something is searched for: the dropdown is open then, with the places or with what to do.
    pub fn searching(&self) -> bool {
        !self.terms().is_empty()
    }

    /// How the search came out, or nothing when nothing is searched for.
    pub fn search_note(&self) -> Option<String> {
        if self.terms().is_empty() {
            return None;
        }
        if self.has_no_places() {
            return Some(self.word("map-search-no-places"));
        }
        let n = self.results().len();
        let i18n = &self.i18n;
        Some(if n == 0 {
            i18n.tr("map-search-none", &Args::new().str("query", self.search.get().trim()).num("places", self.places() as i64))
        } else if n > Self::SHOWN {
            i18n.tr("map-search-found-more", &Args::new().num("n", n as i64).num("shown", Self::SHOWN as i64))
        } else {
            i18n.tr("map-search-found", &Args::new().num("n", n as i64))
        })
    }

    /// Where pressing Enter goes: the result the arrow keys are on, else the first.
    pub fn chosen_href(&self) -> Option<String> {
        let results = self.results();
        let at = self.active.get_untracked().unwrap_or(0).min(Self::SHOWN - 1);
        results.into_iter().nth(at).map(|r| r.href)
    }

    /// How a junction is told on the map.
    pub fn junction_label(&self, uid: u32) -> String {
        let units = self.units();
        self.view().nodes.iter().find(|n| n.uid == uid).map(|n| junction_label(n, &self.i18n, units)).unwrap_or_default()
    }

    pub fn street_label(&self, uid: u32) -> String {
        let units = self.units();
        self.view().edges.iter().find(|e| e.uid == uid).map(|e| street_label(e, &self.i18n, units)).unwrap_or_default()
    }

    /// Every place, as the notes list them: junctions, then streets. Each is its link, its name still
    /// unworded (only the places a note lists need words, see `note_item`), whether it works, whether it
    /// was changed and what fails.
    fn places_for_notes<'a>(&self, v: &'a CityView) -> Vec<(String, Said, bool, bool, &'a [Said])> {
        let mut all = Vec::new();
        for n in Self::junctions(v) {
            all.push((junction_href(n.uid), n.name.clone(), n.ok, n.edited, n.failing.as_slice()));
        }
        for e in &v.edges {
            let name = Said::new("map-note-street").with("street", Arg::Said(Box::new(e.called.clone()))).with("ends", Arg::Said(Box::new(e.ends.clone())));
            all.push((street_href(e.uid), name, e.ok, e.edited, e.failing.as_slice()));
        }
        all
    }

    /// A place of `places_for_notes` as a line of a note, its name worded in the language now.
    fn note_item(&self, href: String, name: &Said) -> NoteItem {
        NoteItem { href, name: self.say(name), detail: String::new() }
    }

    /// The places that need attention, with what is wrong.
    pub fn failing_items(&self) -> Vec<NoteItem> {
        let v = self.view();
        self.places_for_notes(&v)
            .into_iter()
            .filter(|p| !p.2)
            .map(|(href, name, _, _, f)| {
                let mut i = self.note_item(href, &name);
                i.detail = all_said(&self.i18n, self.units(), f).join("; ");
                i
            })
            .collect()
    }

    /// How many streets break the rule of an area they are inside (a speed limit never does).
    fn streets_in_conflict(&self) -> usize {
        let mut uids: Vec<u32> = self.findings().iter().filter(|f| !f.conflicts.is_empty()).map(|f| f.uid).collect();
        uids.sort_unstable();
        uids.dedup();
        uids.len()
    }

    pub fn checks_lead(&self) -> String {
        if self.has_no_places() {
            return self.word("map-checks-no-places");
        }
        let v = self.view();
        let all = self.places_for_notes(&v);
        let failing = all.iter().filter(|p| !p.2).count();
        let streets = self.streets_in_conflict();
        if failing == 0 && streets > 0 {
            self.i18n.tr("map-checks-conflicts", &Args::new().num("streets", streets as i64))
        } else if failing > 0 {
            self.i18n.tr("map-checks-failing", &Args::new().num("n", failing as i64))
        } else {
            self.i18n.tr("map-checks-pass", &Args::new().num("n", all.len() as i64))
        }
    }

    /// The places that have been changed, and whether each still works.
    pub fn changed_items(&self) -> Vec<NoteItem> {
        let v = self.view();
        self.places_for_notes(&v)
            .into_iter()
            .filter(|p| p.3)
            .map(|(href, name, ok, _, f)| {
                let mut i = self.note_item(href, &name);
                i.detail = if ok {
                    self.word("map-still-works")
                } else {
                    let failing = all_said(&self.i18n, self.units(), f).join("; ").to_lowercase();
                    self.i18n.tr("map-detail-needs-attention", &Args::new().str("failing", failing))
                };
                i
            })
            .collect()
    }

    pub fn changes_lead(&self) -> String {
        if self.has_no_places() {
            return self.word("map-changes-no-places");
        }
        let n = self.places_for_notes(&self.view()).iter().filter(|p| p.3).count();
        if n > 0 { self.i18n.tr("map-changes-some", &Args::new().num("n", n as i64)) } else { self.word("map-changes-none") }
    }

    /// Whether the city works, in one line.
    pub fn status(&self) -> Status {
        if self.has_no_places() {
            // Nothing works, since there is nothing: not a tick.
            return Status { text: self.word("map-nothing-to-show"), bad: true };
        }
        let v = self.view();
        let bad: Vec<String> = self.places_for_notes(&v).into_iter().filter(|p| !p.2).map(|p| self.say(&p.1)).collect();
        let streets = self.streets_in_conflict();
        if bad.is_empty() && streets > 0 {
            return Status { text: self.i18n.tr("map-status-conflicts", &Args::new().num("n", v.places as i64).num("streets", streets as i64)), bad: true };
        }
        if bad.is_empty() {
            return Status { text: self.i18n.tr("map-status-ok", &Args::new().num("n", v.places as i64)), bad: false };
        }
        let key = if bad.len() > 3 { "map-status-bad-more" } else { "map-status-bad" };
        let names = bad.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
        Status { text: self.i18n.tr(key, &Args::new().num("n", bad.len() as i64).str("names", names)), bad: true }
    }

    pub fn camera(&self) -> Camera {
        self.camera.get()
    }

    pub fn view_box(&self) -> String {
        self.camera.get().view_box()
    }

    /// The place the pointer or the focus is on, which the map and the lists show together.
    pub fn hot(&self) -> Option<String> {
        self.hot.get()
    }

    // ---- commands ----

    pub fn set_hot(&self, hot: Option<String>) {
        if self.hot.get_untracked() != hot {
            self.hot.set(hot);
        }
    }

    pub fn set_region(&self, region: usize) {
        self.core.edit(|m| m.region = region);
    }

    /// The region the city is shown in, by index into the catalogue.
    pub fn region(&self) -> usize {
        self.core.read(|m| m.region)
    }

    /// The city is read again from where it is kept: another tab wrote it, the
    /// page is shown again, or it was started over.
    pub fn reload(&self) {
        let city = self.store.open();
        self.core.edit(|m| m.city = city);
    }

    /// The city is opened again because its roads have arrived since this view-model was made (the home page is
    /// built before the area's roads are read): it is read again and the camera is put on the whole of it.
    pub fn reopen(&self) {
        self.reload();
        let world = World::round(self.core.view_now().bounds_mm);
        self.update_camera(|c| {
            c.world = world;
            c.moved = false;
            c.fit();
        });
    }

    fn update_camera(&self, f: impl FnOnce(&mut Camera)) {
        self.camera.update(f);
    }

    /// The window the map fills changed size.
    pub fn resize(&self, width: f64, height: f64) {
        self.update_camera(|c| c.resize(width, height));
    }

    // ---- the map ----

    pub fn basemap_state(&self) -> BasemapState {
        self.basemap.get()
    }

    /// Where the city lies on the earth: known once its area's roads are.
    pub fn projection(&self) -> Option<Projection> {
        crate::city::projection::for_store(&self.store, self.core.view_now().origin_m)
    }

    /// Listens to what the map says, and tells it the units. Called once, by the page that has a map.
    pub fn attach(&self) {
        let me = self.me.clone();
        self.ports.mapper.listen(Box::new(move |event| {
            if let Some(vm) = me.upgrade() {
                vm.on_map_event(event);
            }
        }));
        self.ports.mapper.set_imperial(matches!(self.core.units(), Units::Feet));
    }

    /// What the map says. While an outline is drawn a press is always a corner of it (or closes it, on its first
    /// corner), whatever lies under it; while one is named a press does nothing. Otherwise a press on a place opens
    /// it, even inside an area, and a press elsewhere selects the area there, or none. A corner dragged is the
    /// areas'; the map tells of no press for the click that ends it.
    fn on_map_event(&self, event: MapEvent) {
        match event {
            MapEvent::Ready { bounds } => self.tiles_ready(bounds),
            MapEvent::Failed => self.give_up(MISSING),
            MapEvent::Press { at, hot, area, vertex } => match self.areas.draw_now() {
                Draw::Drawing(_) => self.areas.on_press(at, vertex),
                Draw::Naming(_) => {}
                Draw::Idle => {
                    let place = hot.as_deref().filter(|h| self.can_edit(h));
                    match place.and_then(href_of) {
                        Some(href) => self.ports.navigator.go(&href),
                        None => {
                            // A refused press on a place (muted, outside every outline) is not a press on nothing: it
                            // lets no selected area go.
                            let refused = hot.is_some() && place.is_none();
                            if refused && area.is_none() {
                                return self.say_not_editable();
                            }
                            let before = self.areas.selected_id_now();
                            self.areas.select(area.as_deref());
                            if refused && self.areas.selected_id_now() == before {
                                self.say_not_editable();
                            }
                        }
                    }
                }
            },
            MapEvent::Vertex { area, index, at, done } => self.areas.drag_vertex(&area, index, at, done),
            MapEvent::Hover { hot } => self.set_hot(hot),
            MapEvent::Moved => self.moved.set(true),
            MapEvent::View { bounds } => {
                self.viewport.set(Some(bounds));
                self.sync_editable();
            }
        }
    }

    fn say_not_editable(&self) {
        self.ports.announcer.say(&self.i18n.tr_now("map-not-editable", &Args::new()));
    }

    /// There will be no map, unless the page never had one to wait for (which has said why already).
    fn give_up(&self, key: &'static str) {
        if self.store.has_network() {
            self.basemap.set(BasemapState::Unavailable(key));
        }
    }

    fn tiles_ready(&self, [west, south, east, north]: [f64; 4]) {
        if !self.store.has_network() {
            return;
        }
        let area = self.store.area();
        if !(west..=east).contains(&area.lon) || !(south..=north).contains(&area.lat) {
            return self.give_up(OUTSIDE);
        }
        self.basemap.set(BasemapState::Ready);
        self.sync_places();
        self.fit();
        self.sync_areas();
    }

    /// Sends the places to the map, once there is a map to show them on; a street that breaks the rule of an area it
    /// is inside is marked.
    pub fn sync_places(&self) {
        let Some(projection) = self.projection() else { return };
        let area = self.store.area();
        if self.basemap.get_untracked() != BasemapState::Ready {
            return;
        }
        let view = self.core.view_now();
        // Only a rule a street can break can tint one.
        let checked: Vec<Restriction> = self.areas.list_now().into_iter().filter(|r| !r.rule.informational()).collect();
        let in_conflict: BTreeSet<u32> = self.findings_in(&view, &checked).into_iter().filter(|s| !s.conflicts.is_empty()).map(|s| s.uid).collect();
        self.ports.mapper.set_places(&style::layers(area.lat), &overlay::places(&view, &projection, &in_conflict));
        self.ports.mapper.set_zone_ticks(&style::zone_tick_layers(), &overlay::zone_ticks(&view, &projection));
        self.sync_editable();
    }

    /// Sends the place the pointer or the focus is on to the map.
    pub fn sync_highlight(&self) {
        self.ports.mapper.highlight(self.hot.get_untracked().as_deref());
    }

    /// Whether the place `hot` can be opened: always, until the map has said which part of it is open.
    fn can_edit(&self, hot: &str) -> bool {
        self.editable.borrow().as_ref().is_none_or(|set| set.iter().any(|h| h == hot))
    }

    /// Tells the map which places the open view makes editable, when that is not what it was last told.
    fn sync_editable(&self) {
        let Some(view) = self.viewport.get() else { return };
        let Some(projection) = self.projection() else { return };
        if self.basemap.get_untracked() != BasemapState::Ready {
            return;
        }
        let now = editable::editable(&overlay::place_boxes(&self.core.view_now(), &projection), view);
        if self.editable.borrow().as_ref() == Some(&now) {
            return;
        }
        self.ports.mapper.set_editable(&now);
        *self.editable.borrow_mut() = Some(now);
    }

    /// Fits the places in what the panels leave of the map, and gives the view back to the page.
    pub fn fit(&self) {
        self.moved.set(false);
        let Some(projection) = self.projection() else { return };
        if self.basemap.get_untracked() != BasemapState::Ready {
            return;
        }
        let Some(bounds) = overlay::bounds(&self.core.view_now(), &projection) else { return };
        let i = self.camera.get_untracked().insets;
        self.ports.mapper.fit(bounds, [i.top, i.right, i.bottom, i.left]);
    }

    /// The colours the places are drawn in, and what each says.
    pub fn status_key(&self) -> Vec<(&'static str, String)> {
        [("ok", "map-state-works"), ("changed", "map-state-changed"), ("bad", "map-state-bad"), ("conflict", "restriction-legend-conflict")]
            .into_iter()
            .map(|(id, key)| (id, self.word(key)))
            .collect()
    }

    // ---- area restrictions ----

    /// The Area rules panel's view-model.
    pub fn areas(&self) -> &Rc<AreasVm> {
        &self.areas
    }

    /// Whether an area can be drawn: there is a map to draw it on, and the document is not a newer build's.
    pub fn can_declare(&self) -> bool {
        self.basemap.get() == BasemapState::Ready && !self.areas.is_future()
    }

    /// What the panel says first: why nothing can be drawn where there is no map, else the areas' own lead.
    pub fn areas_lead(&self) -> String {
        if !self.areas.is_future() && matches!(self.basemap.get(), BasemapState::Unavailable(_)) {
            return self.word("restriction-lead-no-map");
        }
        self.areas.lead()
    }

    /// Sends the areas and the outline being drawn to the map, once there is a map to show them on.
    pub fn sync_areas(&self) {
        if self.basemap.get_untracked() == BasemapState::Ready {
            self.areas.sync();
        }
    }

    /// The streets of `v` inside each of `list`, area by area, with what each has that the area's rule does not
    /// allow. A street inside several areas is found once for each. A rule that is only for information (a speed
    /// limit: streets carry no speed) reads no street to check it.
    fn findings_in(&self, v: &CityView, list: &[Restriction]) -> Vec<AreaStreet> {
        let Some(p) = self.projection() else { return Vec::new() };
        let mut found = Vec::new();
        for r in list {
            for e in v.edges.iter().filter(|e| r.holds(&e.shape_mm, &p)) {
                let conflicts = if r.rule.informational() {
                    Vec::new()
                } else {
                    #[cfg(test)]
                    self.editors_read.set(self.editors_read.get() + 1);
                    let Some(street) = self.core.read(|m| m.city.street_editor(e.uid, m.region).map(|e| e.snapshot())) else { continue };
                    conflicts(r.rule, &street)
                };
                let name = Said::new("map-note-street").with("street", Arg::Said(Box::new(e.called.clone()))).with("ends", Arg::Said(Box::new(e.ends.clone())));
                found.push(AreaStreet { area: r.id.clone(), uid: e.uid, href: street_href(e.uid), name, rule: r.rule, conflicts });
            }
        }
        found
    }

    /// Every street inside an area, for a view: found again when the city or the areas change.
    pub fn findings(&self) -> Vec<AreaStreet> {
        self.findings_in(&self.view(), &self.areas.list())
    }

    /// The streets inside the area `id`.
    pub fn area_streets(&self, id: &str) -> Vec<AreaStreet> {
        let Some(r) = self.areas.find(id) else { return Vec::new() };
        self.findings_in(&self.view(), &[r])
    }

    /// An area's small print in the panel: its rule, and how many streets are inside and in conflict.
    pub fn area_row_sub(&self, r: &Restriction) -> String {
        let streets = self.area_streets(&r.id);
        let inside = streets.len() as i64;
        let in_conflict = streets.iter().filter(|s| !s.conflicts.is_empty()).count() as i64;
        let summary = if inside == 0 {
            Said::new("restriction-summary-none")
        } else if r.rule.informational() {
            Said::new("restriction-summary-info").with("inside", Arg::Num(inside))
        } else {
            Said::new("restriction-summary").with("inside", Arg::Num(inside)).with("conflicts", Arg::Num(in_conflict))
        };
        self.say(&Said::new("restriction-row-sub").with("rule", Arg::Said(Box::new(r.rule.said()))).with("summary", Arg::Said(Box::new(summary))))
    }

    /// How a street inside an area stands, in words.
    pub fn street_detail(&self, s: &AreaStreet) -> String {
        if s.rule.informational() {
            return self.word("restriction-street-inside");
        }
        if s.conflicts.is_empty() {
            return self.word("restriction-street-keeps");
        }
        s.conflicts.iter().map(|c| self.say(&c.said())).collect::<Vec<_>>().join(" ")
    }

    /// What floats over the map, in pixels from each edge: the whole city is fitted in what is left, until
    /// the person moves the map themselves.
    pub fn set_insets(&self, insets: Insets) {
        self.update_camera(|c| c.set_insets(insets));
        let i = self.camera.get_untracked().insets;
        self.ports.mapper.set_insets([i.top, i.right, i.bottom, i.left]);
        if !self.moved.get() {
            self.fit();
        }
    }

    pub fn set_units(&self, units: Units) {
        self.core.set_units(units);
        self.ports.mapper.set_imperial(matches!(units, Units::Feet));
    }

    pub fn zoom_in(&self) {
        self.moved.set(true);
        self.ports.mapper.zoom_by(ZOOM_STEP);
    }

    pub fn zoom_out(&self) {
        self.moved.set(true);
        self.ports.mapper.zoom_by(1.0 / ZOOM_STEP);
    }

    fn pan(&self, dx: f64, dy: f64) {
        self.moved.set(true);
        self.ports.mapper.pan_by(dx, dy);
    }

    /// The whole city fits the window again: the hero's camera, and the map.
    pub fn fit_camera(&self) {
        self.update_camera(|c| c.fit());
        self.fit();
    }

    /// What a key does to the map; says whether the key was one of its own.
    pub fn press_key(&self, key: &str) -> bool {
        match key {
            "ArrowLeft" => self.pan(-PAN_PX, 0.0),
            "ArrowRight" => self.pan(PAN_PX, 0.0),
            "ArrowUp" => self.pan(0.0, -PAN_PX),
            "ArrowDown" => self.pan(0.0, PAN_PX),
            "+" | "=" => self.zoom_in(),
            "-" | "_" => self.zoom_out(),
            "0" => self.fit_camera(),
            _ => return false,
        }
        true
    }
}

impl crate::shell::Target for MapVm {
    fn set_units(&self, units: Units) {
        MapVm::set_units(self, units);
    }

    fn apply_region(&self, region: usize) -> bool {
        self.set_region(region);
        true
    }

    fn region_id(&self) -> String {
        crate::shared::catalogue::REGIONS[self.region()].id.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::camera::Insets;
    use crate::place::area::Area;
    use crate::restriction::draw::Draw;
    use crate::restriction::fixtures::{bent_midpoint, square};
    use crate::restriction::model::Conflict;
    use crate::restriction::store::RestrictionStore;
    use crate::shared::i18n::Locale;
    use crate::shared::ports::{FakeMapper, MapCall, MapEvent, MemoryStorage, Ports, RecordingAnnouncer, RecordingNavigator, Storage, test_ports};
    use crate::shared::said::say_now;
    use osm_network::{Control, Lane, LaneKind, Network, Node, Road, Way};

    fn vm() -> (Rc<MapVm>, Rc<RecordingAnnouncer>, Rc<MemoryStorage>) {
        vm_in(Locale::En)
    }

    fn vm_in(locale: Locale) -> (Rc<MapVm>, Rc<RecordingAnnouncer>, Rc<MemoryStorage>) {
        let (ports, said, storage) = test_ports();
        (MapVm::on_sample(ports, crate::i18n_for(locale)), said, storage)
    }

    /// What the page shows of a place's name or small print, in its language now.
    fn shown(vm: &MapVm, said: &Said) -> String {
        say_now(vm.i18n(), vm.units(), said)
    }

    fn name_of(vm: &MapVm, r: &PlaceRow) -> String {
        shown(vm, &r.name)
    }

    fn sub_of(vm: &MapVm, r: &PlaceRow) -> String {
        shown(vm, &r.sub)
    }

    /// Another page writes a street of the city into the shared storage.
    fn nudge_street(storage: &Rc<MemoryStorage>, street: u32, mm: i32) {
        let store = CityStore::sample(storage.clone());
        let mut e = store.open().street_editor(street, 0).unwrap();
        let u = e.view().segments[0].uid;
        assert!(e.set_width(u, e.view().segments[0].width_mm + mm), "{mm}");
        assert!(store.write_street(street, e.snapshot()));
    }

    fn names(vm: &MapVm) -> Vec<String> {
        vm.results().iter().map(|r| name_of(vm, r)).collect()
    }

    #[test]
    fn nothing_is_found_until_something_is_searched_for_and_the_places_list_is_whole() {
        let (vm, ..) = vm();
        assert_eq!(vm.search_text(), "");
        assert!(vm.results().is_empty());
        assert_eq!(vm.search_note(), None);
        vm.set_search("avenue");
        assert_eq!(vm.junction_rows().len() + vm.street_rows().len(), vm.places(), "the places list is not narrowed");
    }

    #[test]
    fn a_search_finds_places_by_name_and_by_the_small_print() {
        let (vm, ..) = vm();
        vm.set_search("avenue");
        assert!(!vm.results().is_empty() && names(&vm).iter().all(|n| n.to_lowercase().contains("avenue")));
        vm.set_search("signal");
        let found = vm.results();
        assert!(!found.is_empty() && found.iter().all(|r| sub_of(&vm, r).to_lowercase().contains("traffic signal")));
    }

    #[test]
    fn what_a_search_finds_follows_a_switch_of_language() {
        let (vm, ..) = vm();
        vm.set_search("jonction 4");
        let owner = Owner::new();
        owner.set();
        let v = StoredValue::new_local(vm.clone());
        let found = Memo::new(move |_| v.with_value(|vm| vm.results().len()));
        assert_eq!(found.get(), 0, "nothing is called a jonction in English");
        vm.i18n().set(Locale::FrCa);
        assert!(found.get() > 0, "the results are worked out again from the French the page now shows");
    }

    #[test]
    fn the_places_are_worded_once_until_the_city_the_language_or_the_units_change() {
        let (vm, _, storage) = vm();
        let first = vm.worded_rows();
        vm.set_search("avenue");
        assert!(Rc::ptr_eq(&first, &vm.worded_rows()), "typing does not word the places again");
        vm.set_search("avenue 1");
        let _ = (vm.results(), vm.search_note());
        assert!(Rc::ptr_eq(&first, &vm.worded_rows()));
        vm.i18n().set(Locale::FrCa);
        let french = vm.worded_rows();
        assert!(!Rc::ptr_eq(&first, &french), "a switch of language words them again");
        assert!(Rc::ptr_eq(&french, &vm.worded_rows()));
        vm.set_units(Units::Feet);
        let feet = vm.worded_rows();
        assert!(!Rc::ptr_eq(&french, &feet), "so does a change of units");
        nudge_street(&storage, vm.view().edges[0].uid, -100);
        vm.reload();
        assert!(!Rc::ptr_eq(&feet, &vm.worded_rows()), "and an edit of the city");
    }

    #[test]
    fn the_dropdown_is_open_exactly_when_something_is_searched_for() {
        let (vm, ..) = vm();
        assert!(!vm.searching());
        vm.set_search("   ");
        assert!(!vm.searching() && vm.search_note().is_none());
        vm.set_search("zzz");
        assert!(vm.searching() && vm.search_note().is_some());
    }

    #[test]
    fn folding_makes_the_typed_and_the_typographic_characters_one() {
        assert_eq!(fold("Bout d\u{2019}Une RUE"), "bout d'une rue");
        assert_eq!(fold("12,0\u{a0}m \u{202f}x"), "12,0 m  x");
    }

    #[test]
    fn a_search_ignores_case_and_spaces_looks_for_the_phrase_first_and_then_for_each_word() {
        let (vm, ..) = vm();
        vm.set_search("  JUNCTION   4 ");
        let found = vm.results();
        assert!(found.iter().any(|r| name_of(&vm, r) == "Junction 4"));
        assert!(found.iter().all(|r| format!("{} {}", name_of(&vm, r), sub_of(&vm, r)).contains("Junction 4")), "not every junction of four streets");
        vm.set_search("avenue junction 5");
        let found = vm.results();
        assert!(!found.is_empty() && found.iter().all(|r| name_of(&vm, r).to_lowercase().contains("avenue") && sub_of(&vm, r).contains("Junction 5")));
    }

    #[test]
    fn the_note_counts_the_matches_and_says_what_to_do_when_there_are_none() {
        let (vm, ..) = vm();
        vm.set_search("Junction 1 stop");
        assert_eq!(vm.search_note().as_deref(), Some("1 place matches"));
        vm.set_search("zzz");
        assert_eq!(vm.search_note(), Some(format!("No places match \u{201c}zzz\u{201d}. Clear the search to see all {}.", vm.places())));
        vm.set_search("sample");
        assert!(vm.search_note().unwrap().ends_with(&format!("showing the first {}", MapVm::SHOWN)));
    }

    #[test]
    fn the_arrow_keys_move_down_the_results_and_stop_at_the_ends() {
        let (vm, ..) = vm();
        vm.set_search("junction");
        assert_eq!(vm.active_result(), None);
        vm.move_active(-1);
        assert_eq!(vm.active_result(), Some(0));
        vm.move_active(1);
        vm.move_active(1);
        assert_eq!(vm.active_result(), Some(2));
        for _ in 0..20 {
            vm.move_active(1);
        }
        assert_eq!(vm.active_result(), Some(MapVm::SHOWN - 1));
        vm.set_search("junction 4");
        assert_eq!(vm.active_result(), None, "a new search starts again");
        vm.set_search("zzz");
        vm.move_active(1);
        assert_eq!(vm.active_result(), None);
    }

    #[test]
    fn enter_opens_the_result_the_arrows_are_on_or_else_the_first_and_nothing_when_there_is_none() {
        let (vm, ..) = vm();
        vm.set_search("junction");
        assert_eq!(vm.chosen_href(), vm.results().first().map(|r| r.href.clone()));
        vm.move_active(1);
        vm.move_active(1);
        assert_eq!(vm.chosen_href(), vm.results().get(1).map(|r| r.href.clone()));
        vm.set_search("avenue");
        assert!(vm.chosen_href().unwrap().starts_with("street.html?street="));
        vm.set_search("zzz");
        assert_eq!(vm.chosen_href(), None);
    }

    #[test]
    fn clearing_the_search_finds_nothing_and_says_nothing() {
        let (vm, ..) = vm();
        vm.set_search("avenue");
        vm.set_search("");
        assert_eq!(vm.search_note(), None);
        assert_eq!(vm.chosen_href(), None);
    }

    #[test]
    fn what_floats_over_the_map_makes_the_whole_city_fit_in_the_open_part() {
        let (vm, ..) = vm();
        vm.resize(1000.0, 600.0);
        let open = vm.camera().fit_k;
        vm.set_insets(Insets { left: 376.0, top: 88.0, right: 376.0, bottom: 72.0 });
        assert!(vm.camera().fit_k < open);
        assert_eq!(vm.camera().k, vm.camera().fit_k);
        vm.zoom_in();
        vm.fit_camera();
        assert_eq!(vm.camera().k, vm.camera().fit_k, "whole city fits the open part again");
    }

    #[test]
    fn a_search_matches_the_words_the_page_shows_in_its_language() {
        let (vm, ..) = vm_in(Locale::FrCa);
        vm.set_search("jonction 4");
        let found = vm.results();
        assert!(found.iter().any(|r| name_of(&vm, r) == "Jonction 4"), "{:?}", names(&vm));
        assert!(found.iter().all(|r| format!("{} {}", name_of(&vm, r), sub_of(&vm, r)).contains("Jonction 4")), "{:?}", names(&vm));
        vm.set_search("junction 4");
        assert_eq!(names(&vm), Vec::<String>::new(), "the English names are not shown, so not found");
        assert_eq!(vm.search_note().as_deref(), Some("Aucun lieu ne correspond à «\u{a0}junction 4\u{a0}». Effacez la recherche pour revoir les 32 lieux."));
    }

    #[test]
    fn a_switch_of_language_rewords_what_the_search_finds_on_the_same_page() {
        let (vm, ..) = vm();
        vm.set_search("jonction 4");
        assert!(vm.results().is_empty(), "nothing in English says jonction");
        vm.i18n().set(Locale::FrCa);
        assert!(names(&vm).iter().any(|n| n == "Jonction 4"), "{:?}", names(&vm));
        vm.set_search("junction 4");
        assert!(vm.results().is_empty());
        vm.i18n().set(Locale::En);
        assert!(names(&vm).iter().any(|n| n == "Junction 4"), "{:?}", names(&vm));
    }

    #[test]
    fn the_title_and_counts_describe_the_city() {
        let (vm, ..) = vm();
        let owner = Owner::new();
        owner.set();
        let v = vm.view();
        assert_eq!(vm.title(), v.name);
        assert_eq!(vm.counts(), format!("{} junctions, {} streets", v.nodes.iter().filter(|n| n.junction).count(), v.edges.len()));
        assert_eq!((vm.places(), vm.changes()), (v.places, 0));
    }

    #[test]
    fn the_junctions_are_listed_in_the_order_of_their_numbers_with_a_link_to_each() {
        let (vm, ..) = vm();
        let rows = vm.junction_rows();
        let view = vm.view();
        let numbers: Vec<u32> = rows.iter().map(|r| view.nodes.iter().find(|n| n.name == r.name).unwrap().number).collect();
        let mut sorted = numbers.clone();
        sorted.sort_unstable();
        assert_eq!(numbers, sorted);
        assert!(rows[0].href.starts_with("intersection.html?junction=") && rows[0].hot.starts_with("j-"));
        assert!(sub_of(&vm, &rows[0]).contains(" street"));
        assert_eq!(rows[0].tag, None);
    }

    #[test]
    fn the_streets_are_listed_with_where_they_run_and_how_wide_in_the_units_shown() {
        let (vm, ..) = vm();
        let rows = vm.street_rows();
        assert_eq!(rows.len(), vm.view().edges.len());
        let e = &vm.view().edges[0];
        assert_eq!(rows[0].href, format!("street.html?street={}", e.uid));
        assert_eq!(rows[0].hot, format!("s-{}", e.uid));
        assert_eq!(sub_of(&vm, &rows[0]), format!("{} \u{b7} {}", shown(&vm, &e.ends), Units::Metres.length(e.row_mm)));
        vm.set_units(Units::Feet);
        assert!(sub_of(&vm, &vm.street_rows()[0]).ends_with(&Units::Feet.length(e.row_mm)));
    }

    #[test]
    fn a_place_tells_a_screen_reader_what_it_is_and_what_it_opens() {
        let (vm, ..) = vm();
        let (n, e) = (vm.view().nodes.iter().find(|n| n.junction).unwrap().uid, vm.view().edges[0].uid);
        let j = vm.junction_label(n);
        assert!(j.ends_with(" Opens the junction plan.") && j.contains("street"), "{j}");
        let s = vm.street_label(e);
        assert!(s.ends_with(" wide. Opens the street cross-section."), "{s}");
        assert_eq!(vm.junction_label(9_999), "");
    }

    #[test]
    fn a_street_changed_by_another_page_shows_as_changed_after_the_map_reads_again() {
        let (vm, _, storage) = vm();
        let e = vm.view().edges[0].uid;
        assert_eq!(vm.changes(), 0);
        nudge_street(&storage, e, -100);
        assert_eq!(vm.changes(), 0, "not until it is read again");
        vm.reload();
        assert_eq!(vm.changes(), 1);
        assert_eq!(vm.street_rows()[0].tag, Some(StateTag { key: "map-state-changed", bad: false }));
        assert!(vm.street_label(e).contains(" Changed."));
        let items = vm.changed_items();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].detail, "Still works");
        assert_eq!(vm.changes_lead(), "1 place changed from the city as first laid out.");
    }

    #[test]
    fn a_street_that_no_longer_works_is_flagged_everywhere_it_is_listed() {
        let (vm, _, storage) = vm();
        let e = vm.view().edges.iter().find(|e| !e.freeway).unwrap().uid;
        nudge_street(&storage, e, 1_000);
        vm.reload();
        assert!(!vm.status().text.starts_with(&format!("{} places", vm.places())));
        let st = vm.status();
        assert!(st.bad && st.text.starts_with("1 place needs attention: "), "{}", st.text);
        let row = vm.street_rows().into_iter().find(|r| r.hot == format!("s-{e}")).unwrap();
        assert_eq!(row.tag, Some(StateTag { key: "map-state-bad", bad: true }));
        assert!(vm.street_label(e).contains(" Needs attention: "));
        assert_eq!(vm.failing_items().len(), 1);
        assert!(vm.checks_lead().starts_with("1 place needs attention. Open one"));
        assert!(vm.changed_items()[0].detail.starts_with("Needs attention: "));
    }

    #[test]
    fn a_city_that_works_says_so_for_every_place() {
        let (vm, ..) = vm();
        let st = vm.status();
        assert_eq!(st, Status { text: format!("{} places. Every check passes.", vm.places()), bad: false });
        assert_eq!(vm.checks_lead(), format!("Every check passes in all {} places.", vm.places()));
        assert!(vm.failing_items().is_empty());
        assert_eq!(vm.changes_lead(), "Nothing changed yet. Open a street or a junction to change it.");
    }

    #[test]
    fn the_status_names_three_places_and_says_there_are_more() {
        let (vm, _, storage) = vm();
        let streets: Vec<u32> = vm.view().edges.iter().filter(|e| !e.freeway).map(|e| e.uid).take(4).collect();
        for s in streets {
            nudge_street(&storage, s, 1_000);
        }
        vm.reload();
        let st = vm.status();
        assert!(st.text.starts_with("4 places need attention: ") && st.text.ends_with(" and more."), "{}", st.text);
        let named = vm.failing_items();
        for item in &named[..3] {
            assert!(st.text.contains(&item.name), "{} in {}", item.name, st.text);
        }
        assert!(!st.text.contains(&named[3].name) || named[3].name == named[0].name);
    }

    #[test]
    fn the_region_is_read_from_storage_and_can_be_changed() {
        let (ports, ..) = test_ports();
        ports.storage.remember(crate::city::store::REGION_KEY, "united-kingdom");
        let vm = MapVm::on_sample(ports, crate::i18n_for(Locale::En));
        assert_eq!(vm.region(), 3);
        vm.set_region(0);
        assert_eq!(vm.region(), 0);
    }

    #[test]
    fn the_hot_place_is_set_and_cleared_by_the_view() {
        let (vm, ..) = vm();
        assert_eq!(vm.hot(), None);
        vm.set_hot(Some("s-3".into()));
        assert_eq!(vm.hot().as_deref(), Some("s-3"));
        vm.set_hot(None);
        assert_eq!(vm.hot(), None);
    }

    /// A city of one street that bends, kept for an area that the page opens.
    fn area_vm() -> (Rc<MapVm>, Rc<FakeMapper>, Rc<RecordingNavigator>, Rc<MemoryStorage>) {
        let (vm, mapper, navigator, _, storage) = area_vm_all();
        (vm, mapper, navigator, storage)
    }

    /// `area_vm` with the announcer it was built over.
    fn area_vm_said() -> (Rc<MapVm>, Rc<FakeMapper>, Rc<RecordingNavigator>, Rc<RecordingAnnouncer>) {
        let (vm, mapper, navigator, said, _) = area_vm_all();
        (vm, mapper, navigator, said)
    }

    fn area_vm_all() -> (Rc<MapVm>, Rc<FakeMapper>, Rc<RecordingNavigator>, Rc<RecordingAnnouncer>, Rc<MemoryStorage>) {
        area_vm_over(|_| {})
    }

    /// `area_vm_all`, after `before` has changed the stored city.
    fn area_vm_over(before: impl FnOnce(&CityStore)) -> (Rc<MapVm>, Rc<FakeMapper>, Rc<RecordingNavigator>, Rc<RecordingAnnouncer>, Rc<MemoryStorage>) {
        let (ports, said, storage) = test_ports();
        let area = Area::new("Bendville", 45.5, -73.6);
        let node = |id, x_m, y_m| Node { id, osm_nodes: vec![id as i64], osm_versions: Default::default(), x_m, y_m, junction: false, control: Control::None };
        let lane = |way| Lane { kind: LaneKind::Driving, way, width_m: 3.0, buffer_type: None, hours: None };
        let network = Network {
            left_hand: false,
            nodes: vec![node(1, 0.0, 0.0), node(2, 100.0, 50.0)],
            roads: vec![Road {
                id: 1,
                osm_ways: vec![7],
                osm_versions: Default::default(),
                name: Some("Bend Street".into()),
                highway: "residential".into(),
                from: 1,
                to: 2,
                lanes: vec![lane(Way::Forward), lane(Way::Backward)],
                points: vec![(0.0, 0.0), (80.0, 0.0), (100.0, 50.0)],
            }],
        };
        let stored = CityStore::for_area(storage.clone(), area.clone());
        assert!(stored.keep_network(&network));
        before(&stored);
        assert!(CityStore::choose(&*storage, &area));
        let (mapper, navigator) = (Rc::new(FakeMapper::default()), Rc::new(RecordingNavigator::default()));
        let ports = Ports { mapper: mapper.clone(), navigator: navigator.clone(), ..ports };
        let vm = MapVm::new(ports, crate::i18n_for(Locale::En));
        vm.attach();
        mapper.take(); // the scale's units, said when it attached
        (vm, mapper, navigator, said, storage)
    }

    /// Tiles that cover the area `area_vm` opens.
    const COVERING: MapEvent = MapEvent::Ready { bounds: [-74.0, 45.0, -73.0, 46.0] };

    /// The box of the one street `area_vm` holds (`s-1`).
    fn street_box(vm: &MapVm) -> [f64; 4] {
        let boxes = overlay::place_boxes(&vm.core.view_now(), &vm.projection().unwrap());
        assert_eq!(boxes[0].0, "s-1");
        boxes[0].1
    }

    /// A view with the street well inside it, and one that cuts the street in half but is bigger than it.
    fn around(b: [f64; 4]) -> [f64; 4] {
        let (w, h) = (b[2] - b[0], b[3] - b[1]);
        [b[0] - w, b[1] - h, b[2] + w, b[3] + h]
    }
    fn clipping(b: [f64; 4]) -> [f64; 4] {
        let (w, h) = (b[2] - b[0], b[3] - b[1]);
        [b[0] + w / 2.0, b[1] - h, b[2] + 3.0 * w, b[3] + h]
    }
    fn press(hot: &str) -> MapEvent {
        MapEvent::Press { at: [-73.6, 45.5], hot: Some(hot.into()), area: None, vertex: None }
    }

    #[test]
    fn the_open_view_makes_the_street_in_it_editable_and_the_set_is_sent_only_when_it_changes() {
        let (vm, mapper, ..) = area_vm();
        mapper.say(COVERING);
        mapper.take();
        let b = street_box(&vm);
        mapper.say(MapEvent::View { bounds: around(b) });
        assert_eq!(mapper.take(), vec![MapCall::Editable(vec!["s-1".into()])]);
        mapper.say(MapEvent::View { bounds: around(b).map(|v| v + 1e-6) });
        assert!(mapper.take().is_empty(), "the same set is not sent again");
        mapper.say(MapEvent::View { bounds: clipping(b) });
        assert_eq!(mapper.take(), vec![MapCall::Editable(vec![])]);
    }

    #[test]
    fn a_street_longer_than_the_view_is_editable_while_the_view_crosses_it() {
        let (vm, mapper, ..) = area_vm();
        mapper.say(COVERING);
        mapper.take();
        let [w, s, e, n] = street_box(&vm);
        let mid = (w + e) / 2.0;
        let thin = (e - w) / 10.0;
        mapper.say(MapEvent::View { bounds: [mid - thin, s - 1.0, mid + thin, n + 1.0] });
        assert_eq!(mapper.take(), vec![MapCall::Editable(vec!["s-1".into()])], "the view is narrower than the street and crosses it");
        mapper.say(MapEvent::View { bounds: [e + thin, s - 1.0, e + 3.0 * thin, n + 1.0] });
        assert_eq!(mapper.take(), vec![MapCall::Editable(vec![])], "and no longer once it misses it");
    }

    #[test]
    fn a_press_on_a_place_outside_the_open_view_opens_nothing_and_says_why() {
        let (vm, mapper, navigator, said) = area_vm_said();
        mapper.say(COVERING);
        mapper.say(press("s-1"));
        assert_eq!(navigator.take(), vec!["street.html?street=1".to_string()], "no view yet: nothing is held back");
        let b = street_box(&vm);
        mapper.say(MapEvent::View { bounds: clipping(b) });
        mapper.say(press("s-1"));
        assert!(navigator.take().is_empty());
        assert_eq!(said.take(), vec!["Not all of this place is in view. Move the map so all of it shows, then try again.".to_string()]);
        mapper.say(MapEvent::View { bounds: around(b) });
        mapper.say(press("s-1"));
        assert_eq!(navigator.take(), vec!["street.html?street=1".to_string()]);
        assert!(said.take().is_empty());
    }

    #[test]
    fn a_press_on_a_muted_place_inside_an_area_outline_selects_the_area_and_says_nothing_until_it_is_selected() {
        let (vm, mapper, navigator, said, _) = car_free_bendville_said();
        mapper.say(MapEvent::View { bounds: clipping(street_box(&vm)) });
        vm.areas().select(None);
        said.take();
        let inside = MapEvent::Press { at: [-73.6, 45.5], hot: Some("s-1".into()), area: Some("r1".into()), vertex: None };
        mapper.say(inside.clone());
        assert_eq!(vm.areas().selected_id_now().as_deref(), Some("r1"));
        assert!(navigator.take().is_empty());
        let heard = said.take();
        assert!(heard.iter().all(|m| !m.contains("Not all of this place")), "the press chose the area: {heard:?}");
        mapper.say(inside);
        assert!(navigator.take().is_empty());
        assert_eq!(said.take(), vec!["Not all of this place is in view. Move the map so all of it shows, then try again.".to_string()]);
    }

    #[test]
    fn a_press_on_a_muted_place_outside_every_outline_says_why_and_keeps_the_selected_area() {
        let (vm, mapper, _, said, _) = car_free_bendville_said();
        mapper.say(MapEvent::View { bounds: clipping(street_box(&vm)) });
        vm.areas().select(Some("r1"));
        said.take();
        mapper.say(press("s-1"));
        assert_eq!(said.take(), vec!["Not all of this place is in view. Move the map so all of it shows, then try again.".to_string()]);
        assert_eq!(vm.areas().selected_id_now().as_deref(), Some("r1"), "refused, so the selection stays");
        mapper.say(MapEvent::Press { at: [-73.6, 45.5], hot: None, area: None, vertex: None });
        assert_eq!(vm.areas().selected_id_now(), None, "a press on nothing still lets the area go");
    }

    #[test]
    fn sending_the_places_again_does_not_send_an_unchanged_editable_set_again() {
        let (vm, mapper, ..) = area_vm();
        mapper.say(COVERING);
        mapper.say(MapEvent::View { bounds: around(street_box(&vm)) });
        mapper.take();
        vm.sync_places();
        let calls = mapper.take();
        assert_eq!(calls.len(), 1, "{calls:?}");
        assert!(matches!(calls[0], MapCall::Places { .. }), "{calls:?}");
    }

    #[test]
    fn the_insets_are_told_to_the_map_before_the_places_are_fitted_in_what_is_left() {
        let (vm, mapper, ..) = area_vm();
        mapper.say(COVERING);
        mapper.take();
        vm.set_insets(Insets { left: 100.0, top: 50.0, right: 20.0, bottom: 10.0 });
        let calls = mapper.take();
        assert_eq!(calls[0], MapCall::Insets([50.0, 20.0, 10.0, 100.0]));
        assert!(matches!(calls[1], MapCall::Fit { .. }), "{calls:?}");
    }

    /// The map page on the area chosen (here the default one), whose roads were never kept.
    fn roadless_map_vm() -> (Rc<MapVm>, Rc<FakeMapper>, Rc<MemoryStorage>) {
        let (ports, _, storage) = test_ports();
        let mapper = Rc::new(FakeMapper::default());
        let vm = MapVm::new(Ports { mapper: mapper.clone(), ..ports }, crate::i18n_for(Locale::En));
        vm.attach();
        mapper.take();
        (vm, mapper, storage)
    }

    #[test]
    fn the_map_page_of_an_area_whose_roads_were_not_had_shows_no_places_and_names_the_area() {
        let (vm, mapper, _) = roadless_map_vm();
        let owner = Owner::new();
        owner.set();
        assert_eq!(vm.basemap_state(), BasemapState::Unavailable(NO_ROADS));
        assert_eq!(vm.title(), crate::place::area::default_area().name, "the area that could not be loaded");
        assert!(vm.junction_rows().is_empty() && vm.street_rows().is_empty());
        assert_eq!((vm.places(), vm.changes()), (0, 0));
        assert_eq!(vm.counts(), "0 junctions, 0 streets");
        assert_eq!(vm.status(), Status { text: "No roads to show.".to_string(), bad: true });
        assert!(vm.failing_items().is_empty() && vm.changed_items().is_empty());
        assert_eq!(vm.checks_lead(), "There are no places to check.");
        assert_eq!(vm.changes_lead(), "There are no places to change.");
        vm.set_search("avenue");
        assert!(vm.results().is_empty());
        assert_eq!(vm.search_note().as_deref(), Some("There are no places to search."));
        assert!(vm.projection().is_none());
        // the camera the page keeps for the panels has a box round nothing, not an inside-out one
        vm.resize(1000.0, 600.0);
        vm.set_insets(Insets { left: 300.0, ..Insets::default() });
        assert!(vm.camera().fit_k.is_finite() && vm.camera().fit_k > 0.0);
        assert!(matches!(mapper.take()[..], [MapCall::Insets(_)]), "the insets are told, and nothing is fitted");
        mapper.say(COVERING);
        mapper.say(MapEvent::Failed);
        assert_eq!(vm.basemap_state(), BasemapState::Unavailable(NO_ROADS), "the reason stays the roads");
        assert!(mapper.take().is_empty(), "nothing is drawn");
    }

    #[test]
    fn the_map_page_reads_nothing_back_from_another_city_s_keys_when_the_city_is_read_again() {
        let (vm, _, storage) = roadless_map_vm();
        nudge_street(&storage, 1, -100); // under the keys of no area
        vm.reload();
        assert_eq!((vm.places(), vm.changes()), (0, 0));
        assert_eq!(vm.title(), crate::place::area::default_area().name);
    }

    #[test]
    fn the_map_page_of_an_area_whose_roads_were_had_shows_them() {
        let (vm, ..) = area_vm();
        assert_eq!((vm.title().as_str(), vm.street_rows().len()), ("Bendville", 1));
    }

    #[test]
    fn the_map_waits_for_the_tiles_and_then_shows_the_places_and_fits_them() {
        let (vm, mapper, ..) = area_vm();
        assert_eq!(vm.basemap_state(), BasemapState::Waiting);
        assert!(mapper.take().is_empty(), "nothing is shown before the tiles are there");
        mapper.say(COVERING);
        assert_eq!(vm.basemap_state(), BasemapState::Ready);
        let calls = mapper.take();
        assert!(matches!(&calls[0], MapCall::Places { layers, data } if layers.contains("places-street") && data.contains("\"s-1\"")), "{calls:?}");
        assert!(matches!(&calls[1], MapCall::Fit { bounds, insets } if bounds[0] < bounds[2] && bounds[1] < bounds[3] && *insets == [0.0; 4]), "{calls:?}");
        assert!(matches!(&calls[2], MapCall::Restrictions { .. }) && matches!(&calls[3], MapCall::Draft { .. }), "{calls:?}");
    }

    #[test]
    fn tiles_that_do_not_cover_the_area_say_so_and_show_nothing() {
        let (vm, mapper, ..) = area_vm();
        mapper.say(MapEvent::Ready { bounds: [10.0, 50.0, 11.0, 51.0] });
        assert_eq!(vm.basemap_state(), BasemapState::Unavailable(OUTSIDE));
        assert!(mapper.take().is_empty());
        vm.sync_places();
        assert!(mapper.take().is_empty(), "still nothing, though asked");
    }

    #[test]
    fn tiles_that_could_not_be_had_say_so() {
        let (vm, mapper, ..) = area_vm();
        mapper.say(MapEvent::Failed);
        assert_eq!(vm.basemap_state(), BasemapState::Unavailable(MISSING));
    }

    #[test]
    fn pressing_a_place_opens_it_and_the_pointer_over_one_makes_it_hot() {
        let (vm, mapper, navigator, _) = area_vm();
        let press = |hot: Option<&str>| MapEvent::Press { at: [-73.6, 45.5], hot: hot.map(String::from), area: None, vertex: None };
        mapper.say(press(Some("s-1")));
        mapper.say(press(Some("j-3")));
        mapper.say(press(Some("nonsense")));
        mapper.say(press(None));
        assert_eq!(navigator.take(), vec!["street.html?street=1".to_string(), "intersection.html?junction=3".to_string()]);
        mapper.say(MapEvent::Hover { hot: Some("s-1".into()) });
        assert_eq!(vm.hot().as_deref(), Some("s-1"));
        mapper.say(MapEvent::Hover { hot: None });
        assert_eq!(vm.hot(), None);
    }

    #[test]
    fn the_hot_place_is_sent_to_the_map() {
        let (vm, mapper, ..) = area_vm();
        vm.set_hot(Some("s-1".into()));
        vm.sync_highlight();
        vm.set_hot(None);
        vm.sync_highlight();
        assert_eq!(mapper.take(), vec![MapCall::Highlight(Some("s-1".into())), MapCall::Highlight(None)]);
    }

    #[test]
    fn the_buttons_and_keys_move_the_map_through_the_port() {
        let (vm, mapper, ..) = area_vm();
        mapper.say(COVERING);
        mapper.take();
        vm.zoom_in();
        vm.zoom_out();
        assert!(vm.press_key("+") && vm.press_key("=") && vm.press_key("-") && vm.press_key("_"));
        assert!(vm.press_key("ArrowLeft") && vm.press_key("ArrowRight") && vm.press_key("ArrowUp") && vm.press_key("ArrowDown"));
        assert!(!vm.press_key("a"));
        assert!(vm.press_key("0"));
        let calls = mapper.take();
        let (zin, zout) = (1.4, 1.0 / 1.4);
        assert_eq!(
            calls[..10],
            [
                MapCall::Zoom(zin),
                MapCall::Zoom(zout),
                MapCall::Zoom(zin),
                MapCall::Zoom(zin),
                MapCall::Zoom(zout),
                MapCall::Zoom(zout),
                MapCall::Pan(-80.0, 0.0),
                MapCall::Pan(80.0, 0.0),
                MapCall::Pan(0.0, -80.0),
                MapCall::Pan(0.0, 80.0),
            ]
        );
        assert!(matches!(calls[10], MapCall::Fit { .. }), "{calls:?}");
    }

    #[test]
    fn the_places_are_fitted_in_what_floats_over_the_map_until_the_person_moves_it() {
        let (vm, mapper, ..) = area_vm();
        mapper.say(COVERING);
        mapper.take();
        vm.set_insets(Insets { left: 100.0, top: 50.0, right: 20.0, bottom: 10.0 });
        assert!(
            matches!(mapper.take()[..], [MapCall::Insets(_), MapCall::Fit { insets, .. }] if insets == [50.0, 20.0, 10.0, 100.0]),
            "top, right, bottom, left"
        );
        mapper.say(MapEvent::Moved);
        vm.set_insets(Insets { left: 200.0, ..Insets::default() });
        assert_eq!(mapper.take(), vec![MapCall::Insets([0.0, 0.0, 0.0, 200.0])], "the person's view is kept: only the insets are told");
        vm.fit_camera();
        assert!(matches!(mapper.take()[..], [MapCall::Fit { insets, .. }] if insets == [0.0, 0.0, 0.0, 200.0]));
        vm.set_insets(Insets::default());
        assert_eq!(mapper.take().len(), 2, "the insets, and fitting gave the view back to the page");
    }

    #[test]
    fn the_units_are_told_to_the_scale() {
        let (vm, mapper, ..) = area_vm();
        vm.set_units(Units::Feet);
        assert_eq!(mapper.take(), vec![MapCall::Imperial(true)]);
        vm.set_units(Units::Metres);
        assert_eq!(mapper.take(), vec![MapCall::Imperial(false)]);
    }

    #[test]
    fn the_key_says_what_the_colours_of_the_places_mean() {
        let (vm, ..) = vm();
        assert_eq!(
            vm.status_key(),
            vec![
                ("ok", "Works".to_string()),
                ("changed", "Changed".to_string()),
                ("bad", "Needs attention".to_string()),
                ("conflict", "Conflicts with an area rule".to_string())
            ]
        );
    }

    #[test]
    fn the_hero_camera_follows_the_window_and_the_view_box_follows_it() {
        let (vm, ..) = vm();
        let o = Owner::new();
        o.set();
        vm.resize(1000.0, 600.0);
        let fit = vm.camera();
        assert_eq!((fit.width, fit.height), (1000.0, 600.0));
        let before = vm.view_box();
        vm.resize(500.0, 600.0);
        assert_ne!(vm.view_box(), before);
    }

    /// Bendville's map, ready, with a car-free area declared round its street's midpoint.
    fn car_free_bendville() -> (Rc<MapVm>, Rc<FakeMapper>, Rc<RecordingNavigator>, Rc<MemoryStorage>) {
        let (vm, mapper, navigator, _, storage) = car_free_bendville_said();
        (vm, mapper, navigator, storage)
    }

    /// `car_free_bendville` with the announcer it was built over.
    fn car_free_bendville_said() -> (Rc<MapVm>, Rc<FakeMapper>, Rc<RecordingNavigator>, Rc<RecordingAnnouncer>, Rc<MemoryStorage>) {
        let (vm, mapper, navigator, said, storage) = area_vm_all();
        mapper.say(COVERING);
        mapper.take();
        let (_, mid) = bent_midpoint(&CityStore::current(storage.clone()));
        let a = vm.areas();
        assert!(a.start());
        for p in square(mid, 0.0005) {
            assert!(a.add_at(p));
        }
        assert!(a.close());
        a.set_form_name("Car zone");
        assert!(a.save());
        said.take();
        (vm, mapper, navigator, said, storage)
    }

    #[test]
    fn a_street_inside_a_car_free_area_with_driving_lanes_is_found_tinted_and_summed_up() {
        let (vm, mapper, ..) = car_free_bendville();
        let found = vm.findings();
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].area.as_str(), found[0].uid, found[0].href.as_str()), ("r1", 1, "street.html?street=1"));
        assert_eq!(found[0].conflicts, vec![Conflict { kind: "travel", pieces: 2 }]);
        assert_eq!(vm.say(&found[0].name), "Bend Street, through the city");
        assert_eq!(vm.street_detail(&found[0]), "It has 2 driving lanes.");
        let r = vm.areas().find_now("r1").unwrap();
        assert_eq!(vm.area_row_sub(&r), "Car-free · One street inside; one conflicts with the rule.");
        vm.sync_places();
        assert!(matches!(&mapper.take()[0], MapCall::Places { data, .. } if data.contains("\"conflict\":true")));
        assert!(vm.areas().set_rule("r1", Rule::SpeedLimit { kmh: 30 }));
        assert_eq!(vm.area_row_sub(&vm.areas().find_now("r1").unwrap()), "Speed limit of 30 km/h · One street inside.");
        assert_eq!(vm.street_detail(&vm.findings()[0]), "Inside the area.");
        vm.sync_places();
        assert!(matches!(&mapper.take()[0], MapCall::Places { data, .. } if data.contains("\"conflict\":false")), "a speed limit flags nothing");
    }

    #[test]
    fn a_street_that_keeps_to_the_rule_of_its_area_is_said_to_and_is_not_tinted() {
        let (vm, mapper, ..) = car_free_bendville();
        assert!(vm.areas().set_rule("r1", Rule::NoParkingOrLoading));
        let found = vm.findings();
        assert_eq!((found.len(), found[0].conflicts.is_empty()), (1, true));
        assert_eq!(vm.street_detail(&found[0]), "Keeps to the rule.");
        assert_eq!(vm.area_row_sub(&vm.areas().find_now("r1").unwrap()), "No parking or loading · One street inside; none conflicts with the rule.");
        vm.sync_places();
        assert!(matches!(&mapper.take()[0], MapCall::Places { data, .. } if data.contains("\"conflict\":false")));
    }

    #[test]
    fn an_area_s_small_print_is_said_in_french() {
        let (vm, ..) = car_free_bendville();
        vm.i18n().set(Locale::FrCa);
        let r = vm.areas().find_now("r1").unwrap();
        assert_eq!(vm.area_row_sub(&r), "Zone sans voitures · 1 rue à l’intérieur; une est en conflit avec la règle.");
        assert_eq!(vm.status_key()[3], ("conflict", "En conflit avec une règle de zone".to_string()));
    }

    #[test]
    fn a_press_while_drawing_places_a_corner_instead_of_opening_the_place_and_naming_ignores_presses() {
        let (vm, mapper, navigator, _) = area_vm();
        mapper.say(COVERING);
        assert!(vm.areas().start());
        let press = |at: [f64; 2], hot: Option<&str>, vertex: Option<usize>| MapEvent::Press { at, hot: hot.map(String::from), area: None, vertex };
        mapper.say(press([-73.600, 45.500], Some("s-1"), None));
        mapper.say(press([-73.599, 45.500], None, None));
        mapper.say(press([-73.599, 45.501], None, None));
        assert_eq!(vm.areas().draw_now().corners().len(), 3);
        mapper.say(press([-73.600, 45.500], Some("s-1"), Some(0)));
        assert!(matches!(vm.areas().draw_now(), Draw::Naming(_)));
        mapper.say(press([-73.6, 45.5], Some("s-1"), None));
        assert!(matches!(vm.areas().draw_now(), Draw::Naming(c) if c.len() == 3), "naming: a press adds nothing");
        assert!(navigator.take().is_empty(), "no place is opened while an area is drawn or named");
    }

    #[test]
    fn a_press_on_an_area_selects_it_a_press_on_nothing_lets_it_go_and_a_drag_moves_its_corner() {
        let (vm, mapper, navigator, _) = car_free_bendville();
        vm.areas().select(None);
        mapper.say(MapEvent::Press { at: [-73.6, 45.5], hot: None, area: Some("r1".into()), vertex: None });
        assert_eq!(vm.areas().selected_id_now().as_deref(), Some("r1"));
        let corner = vm.areas().find_now("r1").unwrap().ring[0];
        let to = [corner[0] - 0.0001, corner[1]];
        mapper.say(MapEvent::Vertex { area: "r1".into(), index: 0, at: to, done: false });
        mapper.say(MapEvent::Vertex { area: "r1".into(), index: 0, at: to, done: true });
        assert_eq!(vm.areas().find_now("r1").unwrap().ring[0], to);
        mapper.say(MapEvent::Press { at: [-73.6, 45.5], hot: None, area: None, vertex: None });
        assert_eq!(vm.areas().selected_id_now(), None);
        mapper.say(MapEvent::Press { at: [-73.6, 45.5], hot: Some("s-1".into()), area: Some("r1".into()), vertex: None });
        assert_eq!(navigator.take(), ["street.html?street=1"], "a place in an area opens the place");
    }

    #[test]
    fn a_corner_pressed_and_let_go_without_a_move_moves_nothing_and_the_next_press_opens_the_street() {
        let (vm, mapper, navigator, storage) = car_free_bendville();
        let corner = vm.areas().find_now("r1").unwrap().ring[0];
        let kept = storage.recall(crate::restriction::store::RESTRICTIONS_KEY);
        // What the map tells of a corner clicked: the drop alone (it swallows the click that follows).
        mapper.say(MapEvent::Vertex { area: "r1".into(), index: 0, at: corner, done: true });
        assert!(navigator.take().is_empty(), "a drop opens nothing");
        assert_eq!(vm.areas().selected_id_now().as_deref(), Some("r1"), "and the area stays selected");
        assert_eq!(storage.recall(crate::restriction::store::RESTRICTIONS_KEY), kept, "nothing moved");
        mapper.say(MapEvent::Press { at: corner, hot: Some("s-1".into()), area: Some("r1".into()), vertex: None });
        assert_eq!(navigator.take(), ["street.html?street=1"]);
    }

    #[test]
    fn a_press_while_drawing_right_after_a_corner_drop_is_a_corner_and_opens_nothing() {
        let (vm, mapper, navigator, _) = car_free_bendville();
        let corner = vm.areas().find_now("r1").unwrap().ring[0];
        let to = [corner[0] - 0.0001, corner[1]];
        mapper.say(MapEvent::Vertex { area: "r1".into(), index: 0, at: to, done: false });
        mapper.say(MapEvent::Vertex { area: "r1".into(), index: 0, at: to, done: true });
        assert!(vm.areas().start());
        mapper.say(MapEvent::Press { at: corner, hot: Some("s-1".into()), area: Some("r1".into()), vertex: None });
        assert_eq!(vm.areas().draw_now().corners(), &[corner], "a press while drawing is always a corner");
        assert!(navigator.take().is_empty());
    }

    #[test]
    fn the_areas_are_sent_only_to_a_ready_map_and_declaring_needs_one() {
        let (vm, mapper, ..) = area_vm();
        vm.sync_areas();
        assert!(mapper.take().is_empty() && !vm.can_declare(), "the tiles have not answered");
        mapper.say(COVERING);
        mapper.take();
        assert!(vm.can_declare());
        vm.sync_areas();
        assert!(matches!(mapper.take()[..], [MapCall::Restrictions { .. }, MapCall::Draft { .. }]));
    }

    #[test]
    fn declaring_needs_a_document_this_build_can_write() {
        let (vm, mapper, _, storage) = area_vm();
        storage.remember(crate::restriction::store::RESTRICTIONS_KEY, r#"{"version":2,"restrictions":{}}"#);
        vm.areas().reload();
        mapper.say(COVERING);
        assert!(!vm.can_declare());
        assert!(vm.areas_lead().starts_with("These area rules were saved by a newer version"));
    }

    #[test]
    fn an_area_away_from_the_loaded_streets_holds_none_and_a_roadless_city_finds_nothing() {
        let (vm, mapper, _, storage) = area_vm();
        mapper.say(COVERING);
        let (_, mid) = bent_midpoint(&CityStore::current(storage.clone()));
        let far = RestrictionStore::new(storage.clone()).add("Far", Rule::CarFree, square([mid[0] + 0.05, mid[1]], 0.001)).unwrap();
        vm.areas().reload();
        assert!(vm.area_streets(&far.id).is_empty());
        assert_eq!(vm.area_row_sub(&far), "Car-free · No loaded street is inside.");
        assert_eq!(vm.areas_lead(), "One area rule. Select one to see the streets inside it.");
        let (roadless, ..) = roadless_map_vm();
        assert!(roadless.findings().is_empty());
        assert!(!roadless.can_declare());
        assert_eq!(roadless.areas_lead(), "An area is drawn on the map, and there is no map here.");
    }

    #[test]
    fn a_speed_limit_area_reads_no_street_and_tints_none() {
        let (vm, mapper, ..) = car_free_bendville();
        assert!(vm.areas().set_rule("r1", Rule::SpeedLimit { kmh: 30 }));
        vm.editors_read.set(0);
        vm.sync_places();
        assert!(matches!(&mapper.take()[0], MapCall::Places { data, .. } if data.contains("\"conflict\":false")));
        assert_eq!(vm.area_row_sub(&vm.areas().find_now("r1").unwrap()), "Speed limit of 30 km/h · One street inside.");
        assert_eq!(vm.editors_read.get(), 0, "streets carry no speed, so none is read to check one");
    }

    #[test]
    fn the_streets_of_one_area_are_found_without_checking_the_others() {
        let (vm, _, _, storage) = car_free_bendville();
        let (_, mid) = bent_midpoint(&CityStore::current(storage.clone()));
        let far = RestrictionStore::new(storage.clone()).add("Far", Rule::CarFree, square([mid[0] + 0.05, mid[1]], 0.001)).unwrap();
        vm.areas().reload();
        vm.editors_read.set(0);
        assert!(vm.area_streets(&far.id).is_empty());
        assert_eq!(vm.editors_read.get(), 0, "the street inside the other area is not read");
        assert_eq!(vm.area_streets("r1").len(), 1);
        assert_eq!(vm.editors_read.get(), 1);
        assert!(vm.area_streets("r9").is_empty(), "no such area");
    }

    #[test]
    fn a_street_in_conflict_with_an_area_rule_is_not_a_city_where_every_check_passes() {
        let (vm, ..) = car_free_bendville();
        let owner = Owner::new();
        owner.set();
        let before = vm.status();
        assert!(!vm.findings().is_empty() && vm.findings().iter().any(|f| !f.conflicts.is_empty()));
        assert_eq!(before, Status { text: "1 place. One street conflicts with an area rule.".to_string(), bad: true });
        assert_eq!(vm.checks_lead(), "One street conflicts with an area rule. The Area rules panel lists it.");
        assert!(vm.failing_items().is_empty(), "the design checks are what that list is of");
        vm.i18n().set(Locale::FrCa);
        assert_eq!(vm.status().text, "1 lieu. Une rue est en conflit avec une règle de zone.");
        assert_eq!(vm.checks_lead(), "Une rue est en conflit avec une règle de zone. Le panneau Règles de zone la liste.");
    }

    #[test]
    fn a_speed_limit_area_leaves_the_status_and_the_lead_as_they_were() {
        let (vm, ..) = car_free_bendville();
        let owner = Owner::new();
        owner.set();
        assert!(vm.areas().set_rule("r1", Rule::SpeedLimit { kmh: 30 }));
        assert_eq!(vm.status(), Status { text: "1 place. Every check passes.".to_string(), bad: false });
        assert_eq!(vm.checks_lead(), "Every check passes in the one place.");
    }

    #[test]
    fn the_map_sends_the_ticks_with_the_places() {
        let (vm, mapper, ..) = area_vm();
        mapper.say(COVERING);
        let (layers, data) = mapper.zone_ticks();
        assert_eq!(layers, style::zone_tick_layers());
        assert!(serde_json::from_str::<serde_json::Value>(&data).unwrap()["features"].as_array().unwrap().is_empty(), "no zones, no ticks");
        drop(vm);

        let (vm, mapper, ..) = area_vm_over(|store| {
            let city = store.open();
            let shape = city.street_shape(1).unwrap();
            let length = shape.windows(2).map(|w| ((w[1][0] - w[0][0]) as f64).hypot((w[1][1] - w[0][1]) as f64)).sum::<f64>().round() as i32;
            let street = crate::street::model::zoned(city.street_editor(1, 0).unwrap().snapshot(), &[0, length / 2], length);
            assert!(store.write_street(1, street));
        });
        mapper.say(COVERING);
        let (layers, data) = mapper.zone_ticks();
        assert_eq!(layers, style::zone_tick_layers());
        let fc: serde_json::Value = serde_json::from_str(&data).unwrap();
        assert_eq!(fc["features"].as_array().unwrap().len(), 1, "{data}");
        assert_eq!(fc["features"][0]["properties"]["street"], "s-1");
        drop(vm);
    }

    fn zoned_street_vm() -> Rc<MapVm> {
        let (vm, ..) = area_vm_over(|store| {
            let city = store.open();
            let shape = city.street_shape(1).unwrap();
            let length = shape.windows(2).map(|w| ((w[1][0] - w[0][0]) as f64).hypot((w[1][1] - w[0][1]) as f64)).sum::<f64>().round() as i32;
            let street = crate::street::model::zoned(city.street_editor(1, 0).unwrap().snapshot(), &[0, length / 2], length);
            assert!(store.write_street(1, street));
        });
        vm
    }

    #[test]
    fn a_street_with_several_zones_says_how_many_in_its_label_and_row() {
        let vm = zoned_street_vm();
        let uid = vm.view().edges[0].uid;
        assert_eq!(vm.view().edges[0].zones, 2);
        let label = vm.street_label(uid);
        assert!(label.contains(", 2 zones.") && label.ends_with(" Opens the street cross-section."), "{label}");
        assert!(sub_of(&vm, &vm.street_rows()[0]).ends_with(" \u{b7} 2 zones"));
        vm.i18n().set(Locale::FrCa);
        let label = vm.street_label(uid);
        assert!(label.contains(" de large, en 2 zones."), "{label}");
        assert!(sub_of(&vm, &vm.street_rows()[0]).ends_with(" \u{b7} en 2 zones"));
    }

    #[test]
    fn a_street_with_one_zone_is_worded_as_before() {
        let (vm, ..) = area_vm_all();
        // The 0.26.0 wording, word for word.
        assert_eq!(vm.street_label(vm.view().edges[0].uid), "Bend Street, through the city, 6.0 m wide. Opens the street cross-section.");
        assert_eq!(sub_of(&vm, &vm.street_rows()[0]), "through the city \u{b7} 6.0 m");
        for (i, e) in vm.view().edges.iter().enumerate() {
            assert_eq!(e.zones, 1);
            let width = Units::Metres.length(e.row_mm);
            assert_eq!(vm.street_label(e.uid), format!("{}, {}, {width} wide. Opens the street cross-section.", shown(&vm, &e.called), shown(&vm, &e.ends)));
            assert_eq!(sub_of(&vm, &vm.street_rows()[i]), format!("{} \u{b7} {width}", shown(&vm, &e.ends)));
        }
    }
}
