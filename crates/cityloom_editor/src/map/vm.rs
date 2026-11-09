//! The city map's view-model: how the city reads as places to open, what needs
//! attention and what has changed, where the camera is, and what pressing
//! "start over" means. The view binds to its properties and sends it commands.

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::{Rc, Weak};

use leptos::prelude::*;

use crate::city::model::{City, CityView, EdgeView, NodeView, street_note};
use crate::city::projection::Projection;
use crate::city::store::CityStore;
use crate::map::areas::AreasVm;
use crate::map::camera::{Camera, Insets, World};
use crate::map::corridor::{self, Op, Outcome, Plan, Report};
use crate::map::editable;
use crate::map::network::{self, Check, NetworkFinding, NetworkReport, Place, PlaceKind, Severity};
use crate::map::overlay;
use crate::map::report;
use crate::map::report_review;
use crate::map::selection::SelectionVm;
use crate::map::source_quality::{self, Kind as SourceKind, SourceFinding, SourceReport};
use crate::map::style;
use crate::restriction::draw::Draw;
use crate::restriction::model::{Conflict, Restriction, Rule, conflicts, speed_unknown};
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

/// `2 curbside uses`, `1 stop`, as data to be worded.
fn counted(key: &'static str, n: usize) -> Arg {
    Arg::Said(Box::new(Said::new(key).with("n", Arg::Num(n as i64))))
}

/// What a street's curbside and stops hold, as data (`2 curbside uses, 1 stop`), or `None` when it holds
/// neither: the street is then worded as it always was.
fn programme(e: &EdgeView) -> Option<Arg> {
    match (e.curb_count, e.stop_count) {
        (0, 0) => None,
        (n, 0) => Some(counted("map-curb", n)),
        (0, m) => Some(counted("map-stops", m)),
        (n, m) => Some(Arg::Said(Box::new(Said::new("map-programme").with("curb", counted("map-curb", n)).with("stops", counted("map-stops", m))))),
    }
}

/// A street's sentence: the first of `keys` or the second (several zones), and the third or fourth (the
/// `-programme` variants) when the street has curbside uses or stops.
fn street_said(keys: [&'static str; 4], e: &EdgeView) -> Said {
    let programme = programme(e);
    let key = keys[usize::from(e.zones > 1) + 2 * usize::from(programme.is_some())];
    let said = with_zones(Said::new(key), e.zones);
    match programme {
        Some(p) => said.with("programme", p),
        None => said,
    }
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
        street_said(["map-label-street", "map-label-street-zones", "map-label-street-programme", "map-label-street-zones-programme"], e)
            .with("street", Arg::Said(Box::new(e.called.clone())))
            .with("ends", Arg::Said(Box::new(e.ends.clone())))
            .with("width", Arg::Text(units.length_in(e.row_mm, i18n.locale())))
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

/// How far a change to the selected streets has got: nothing asked, a review of it shown, or it made.
#[derive(Clone, Debug, PartialEq)]
pub enum Stage {
    Idle,
    Reviewing(Plan),
    Done(Report),
}

/// One street of a review or a report, as the panel shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct CorridorRow {
    pub street: u32,
    pub name: String,
    pub ready: bool,
    /// What the panel says of it: that it can be changed, or why not.
    pub text: String,
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
    /// A speed limit the street has no speed to check by.
    pub speed_unknown: bool,
}

/// How many findings of a check the panel lists, and how many places of a finding it links: the rest are counted.
pub const NETWORK_SHOWN: usize = 50;
pub const NETWORK_PLACES: usize = 8;

/// A place a network finding concerns: a link and its name.
#[derive(Clone, Debug, PartialEq)]
pub struct NetworkPlace {
    pub href: String,
    /// `s-7` or `j-3`, as the map highlights them.
    pub hot: String,
    pub name: String,
}

/// A network finding as the panel lists it, worded.
#[derive(Clone, Debug, PartialEq)]
pub struct NetworkItem {
    pub rule: &'static str,
    pub severity: &'static str,
    /// The word for the severity, so it is not told by a mark alone.
    pub severity_word: String,
    pub text: String,
    /// The places the finding concerns, the first `NETWORK_PLACES` of them.
    pub places: Vec<NetworkPlace>,
    /// `and 5 more streets`, when places are left out.
    pub more_places: Option<String>,
}

/// One of the network checks and what it found.
#[derive(Clone, Debug, PartialEq)]
pub struct NetworkCheck {
    pub id: &'static str,
    /// The check's id in the sources registry.
    pub source: &'static str,
    pub label: String,
    pub count: usize,
    /// `no findings`, `1 finding`, `3 findings`.
    pub count_text: String,
    pub items: Vec<NetworkItem>,
    /// `12 more findings are not shown.`, when findings are left out (`count` is still all of them).
    pub more: Option<String>,
}

/// A source-data finding as the Provenance tab lists it, worded. It has no severity: it is not a design failure.
#[derive(Clone, Debug, PartialEq)]
pub struct SourceItem {
    pub kind: &'static str,
    /// What looks missing or doubtful.
    pub text: String,
    /// The words `Source` and `Design impact`, so the two lines are told apart by more than their place.
    pub source_label: String,
    pub impact_label: String,
    /// Where the doubt comes from.
    pub source: String,
    /// Which design reading it could mislead.
    pub impact: String,
    pub places: Vec<NetworkPlace>,
    pub more_places: Option<String>,
}

/// One kind of source-data finding and what it found.
#[derive(Clone, Debug, PartialEq)]
pub struct SourceGroup {
    pub id: &'static str,
    pub label: String,
    pub count: usize,
    /// `nothing found`, `1 possible gap`, `3 possible gaps`.
    pub count_text: String,
    pub items: Vec<SourceItem>,
    pub more: Option<String>,
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
    /// The curbside uses and the stops as last sent to the map, as (layers, data): they are sent only when they change.
    sent_curbside: RefCell<(String, String)>,
    sent_stops: RefCell<(String, String)>,
    hot: ArcRwSignal<Option<String>>,
    search: ArcRwSignal<String>,
    /// The result the arrow keys have moved to, by index into `results`.
    active: ArcRwSignal<Option<usize>>,
    /// The places worded for the search, until the city, the language or the units change.
    worded: RefCell<Option<Worded>>,
    /// The area restrictions: the Area rules panel, and what the map shows of them.
    areas: Rc<AreasVm>,
    /// The streets selected on the map, and in the Selected streets panel.
    selection: Rc<SelectionVm>,
    /// The change picked for the selected streets, and how far it has got.
    corridor_op: ArcRwSignal<Op>,
    corridor: ArcRwSignal<Stage>,
    /// The last change made to the selected streets, which can be taken back as one step while this page is open.
    undoable: ArcRwSignal<Option<(Op, corridor::Undo)>>,
    /// The network findings for the city view they were made from: the rules run again only when the city changes.
    network: RefCell<Option<(Rc<CityView>, Rc<NetworkReport>)>>,
    /// The source-data findings, likewise; they are read from the city as the importer laid it out.
    source: RefCell<Option<(Rc<CityView>, Rc<SourceReport>)>>,
    /// Whether a report is being written: a second request meanwhile is ignored.
    report_running: Cell<bool>,
    /// Whether the last report attempt failed: the page shows `map-report-failed` until the next attempt or a dismissal.
    report_failed: ArcRwSignal<bool>,
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
        let selection = SelectionVm::new(ports.clone(), i18n.clone());
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
            sent_curbside: RefCell::new((String::new(), String::new())),
            sent_stops: RefCell::new((String::new(), String::new())),
            hot: ArcRwSignal::new(None),
            search: ArcRwSignal::new(String::new()),
            active: ArcRwSignal::new(None),
            worded: RefCell::new(None),
            areas,
            selection,
            corridor_op: ArcRwSignal::new(Op::ALL[0]),
            corridor: ArcRwSignal::new(Stage::Idle),
            undoable: ArcRwSignal::new(None),
            network: RefCell::new(None),
            report_running: Cell::new(false),
            report_failed: ArcRwSignal::new(false),
            source: RefCell::new(None),
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
                sub: street_said(["map-street-sub", "map-street-sub-zones", "map-street-sub-programme", "map-street-sub-zones-programme"], e)
                    .with("ends", Arg::Said(Box::new(e.ends.clone())))
                    .with("width", Arg::Text(units.length_in(e.row_mm, locale))),
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

    /// The result Enter acts on: the one the arrow keys are on, else the first.
    fn chosen(&self) -> Option<PlaceRow> {
        let at = self.active.get_untracked().unwrap_or(0).min(Self::SHOWN - 1);
        self.results().into_iter().nth(at)
    }

    /// Where pressing Enter goes.
    pub fn chosen_href(&self) -> Option<String> {
        self.chosen().map(|r| r.href)
    }

    /// Shift+Enter in the search: the chosen result joins the selection instead of opening. Nothing found, nothing done.
    pub fn toggle_chosen(&self) {
        if let Some(row) = self.chosen() {
            self.toggle_selected(Some(&row.hot));
        }
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
            let name = street_note(e.called.clone(), e.ends.clone());
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

    // ---- the design review report ----

    /// The design review report as data, worded in the language the page is in now: from the city view and the
    /// network findings already worked out, with no rule run again.
    pub fn report_data(&self) -> cityloom_report::ReportData {
        let v = self.core.view_now();
        let network = self.network_report_of(v.clone());
        let by = |severity| network.findings.iter().filter(|f| f.severity == severity).count();
        let facts = report::Facts {
            area: v.name.to_string(),
            streets: v.edges.len(),
            junctions: v.nodes.iter().filter(|n| n.junction).count(),
            places: v.places,
            edited: v.edited,
            failing: v.failing,
            problems: by(Severity::Problem),
            notices: by(Severity::Notice),
            ..Default::default()
        };
        let (place_pages, left_out) = self.core.read(|m| report::place_pages(&m.city, &v, m.region, &self.i18n, self.units()));
        let units = self.units();
        let areas: Vec<_> = self.areas.list_now().into_iter().map(|r| (r.clone(), self.findings_in(&v, &[r]))).collect();
        // The area rules follow the network checks on one page: a city with none declared would leave a page nearly empty.
        let network_and_areas = [report_review::network_page(&network, &self.i18n, units), report_review::area_page(&areas, &self.i18n, units)].concat();
        let review_pages = vec![network_and_areas, report_review::limits_page(&self.source_report_of(v.clone()), &self.i18n, units)];
        let facts = report::Facts { place_pages, left_out, review_pages, ..facts };
        report::build(&facts, env!("CARGO_PKG_VERSION"), self.ports.storage.now_ms(), &self.i18n)
    }

    /// Writes the report as a PDF and gives it to the person to keep; says so when it cannot. A request made while
    /// one is under way is ignored.
    pub fn download_report(&self) {
        if self.report_running.replace(true) {
            return;
        }
        self.report_failed.set(false);
        let data = self.report_data();
        let name = format!("cityloom-{}.pdf", report::slug(&self.core.view_now().name));
        let Ok(json) = serde_json::to_string(&data) else {
            self.report_running.set(false);
            self.report_failure();
            return;
        };
        let me = self.me.clone();
        self.ports.pdf.write(
            json,
            Box::new(move |result| {
                let Some(vm) = me.upgrade() else { return };
                vm.report_running.set(false);
                let saved = match result {
                    Ok(bytes) => vm.ports.downloads.save(&name, "application/pdf", bytes),
                    Err(e) => Err(e),
                };
                if saved.is_err() {
                    vm.report_failure();
                }
            }),
        );
    }

    /// Says a report failed: to a screen reader and, until the next attempt or a dismissal, on the page.
    fn report_failure(&self) {
        self.report_failed.set(true);
        self.ports.announcer.say(&self.i18n.tr_now("map-report-failed", &Args::new()));
    }

    /// Whether the notice of a failed report is shown (tracked, for a view).
    pub fn report_failed(&self) -> bool {
        self.report_failed.get()
    }

    /// Closes the notice of a failed report.
    pub fn dismiss_report_notice(&self) {
        self.report_failed.set(false);
    }

    // ---- the network checks ----
    //
    // Findings that span the loaded streets, counted apart from the places' own checks (the status line and the
    // checks lead do not count them). They are worked out from the city as the map read it, so an edit made on a
    // street or junction page shows when the map reads the city again (it does when the page is shown, or when
    // another tab writes).

    /// Every finding of the network checks, found again when the city changes.
    fn network_report(&self) -> Rc<NetworkReport> {
        self.network_report_of(self.view())
    }

    /// The same for a view already in hand (a command reads it untracked).
    fn network_report_of(&self, view: Rc<CityView>) -> Rc<NetworkReport> {
        if let Some((seen, found)) = &*self.network.borrow()
            && Rc::ptr_eq(seen, &view)
        {
            return found.clone();
        }
        let found = Rc::new(network::check(&self.core.read(|m| m.city.network_read(m.region))));
        *self.network.borrow_mut() = Some((view, found.clone()));
        found
    }

    /// How many findings the network checks have in all.
    pub fn network_count(&self) -> usize {
        self.network_report().findings.len()
    }

    fn count_text(&self, n: usize) -> String {
        let key = match n {
            0 => "net-count-none",
            1 => "net-count-one",
            _ => "net-count",
        };
        self.i18n.tr(key, &Args::new().num("n", n as i64))
    }

    /// The one line above the network checks: how many findings there are, and how many junctions could not be read.
    pub fn network_lead(&self) -> String {
        if self.has_no_places() {
            return self.word("net-no-places");
        }
        let report = self.network_report();
        let n = report.findings.len();
        let mut lead = if n == 0 { self.word("net-lead-none") } else { self.i18n.tr("net-lead", &Args::new().num("n", n as i64)) };
        if report.unreadable > 0 {
            lead.push(' ');
            lead.push_str(&self.i18n.tr("net-unreadable", &Args::new().num("n", report.unreadable as i64)));
        }
        lead
    }

    /// The six checks, each with its count and its findings (the first `NETWORK_SHOWN` of them), in a fixed order.
    /// None when the city has no places.
    pub fn network_rules(&self) -> Vec<NetworkCheck> {
        if self.has_no_places() {
            return Vec::new();
        }
        let report = self.network_report();
        let mut by_check: Vec<Vec<&NetworkFinding>> = vec![Vec::new(); Check::ALL.len()];
        for f in &report.findings {
            if let Some(at) = Check::ALL.iter().position(|c| *c == f.rule) {
                by_check[at].push(f);
            }
        }
        Check::ALL.into_iter().zip(by_check).map(|(rule, found)| self.network_check(rule, found)).collect()
    }

    /// One check with its count (all the findings) and the first of them.
    fn network_check(&self, rule: Check, found: Vec<&NetworkFinding>) -> NetworkCheck {
        let hidden = found.len().saturating_sub(NETWORK_SHOWN);
        NetworkCheck {
            id: rule.id(),
            source: rule.rule(),
            label: self.word(&format!("net-rule-{}", rule.id())),
            count: found.len(),
            count_text: self.count_text(found.len()),
            items: found.into_iter().take(NETWORK_SHOWN).map(|f| self.network_item(f)).collect(),
            more: (hidden > 0).then(|| self.i18n.tr("net-more-findings", &Args::new().num("n", hidden as i64))),
        }
    }

    fn network_item(&self, f: &NetworkFinding) -> NetworkItem {
        let (severity, key) = match f.severity {
            Severity::Problem => ("problem", "net-severity-problem"),
            Severity::Notice => ("notice", "net-severity-notice"),
        };
        let (places, more_places) = self.place_links(&f.places);
        NetworkItem { rule: f.rule.id(), severity, severity_word: self.word(key), text: self.say(&f.said), places, more_places }
    }

    /// The first `NETWORK_PLACES` of a finding's places as links, and `and 5 more places` for the rest.
    fn place_links(&self, places: &[Place]) -> (Vec<NetworkPlace>, Option<String>) {
        let hidden = places.len().saturating_sub(NETWORK_PLACES);
        let links = places
            .iter()
            .take(NETWORK_PLACES)
            .map(|p| match p.kind {
                PlaceKind::Street => NetworkPlace { href: street_href(p.uid), hot: format!("s-{}", p.uid), name: self.say(&p.name) },
                PlaceKind::Junction => NetworkPlace { href: junction_href(p.uid), hot: format!("j-{}", p.uid), name: self.say(&p.name) },
            })
            .collect();
        (links, (hidden > 0).then(|| self.i18n.tr("net-more-places", &Args::new().num("n", hidden as i64))))
    }

    // ---- the source-data inspector (the Provenance tab) ----
    //
    // Where the imported OpenStreetMap data may be missing or wrong. Counted apart from the places' own checks and
    // from the network checks: nothing here reaches the status line, the checks lead or the area rules' counts.

    fn source_report(&self) -> Rc<SourceReport> {
        self.source_report_of(self.view())
    }

    /// The same for a view already in hand (a command reads it untracked).
    fn source_report_of(&self, view: Rc<CityView>) -> Rc<SourceReport> {
        if let Some((seen, found)) = &*self.source.borrow()
            && Rc::ptr_eq(seen, &view)
        {
            return found.clone();
        }
        let found = Rc::new(source_quality::inspect(&self.core.read(|m| m.city.source_read())));
        *self.source.borrow_mut() = Some((view, found.clone()));
        found
    }

    /// How many possible gaps the source-data inspector found in all.
    pub fn source_count(&self) -> usize {
        self.source_report().findings.len()
    }

    fn source_count_text(&self, n: usize) -> String {
        let key = match n {
            0 => "prov-count-none",
            1 => "prov-count-one",
            _ => "prov-count",
        };
        self.i18n.tr(key, &Args::new().num("n", n as i64))
    }

    /// The line above the source findings.
    pub fn source_lead(&self) -> String {
        if self.has_no_places() {
            return self.word("prov-no-places");
        }
        match self.source_count() {
            0 => self.word("prov-lead-none"),
            n => self.i18n.tr("prov-lead", &Args::new().num("n", n as i64)),
        }
    }

    /// The kinds of finding, in a fixed order, each with its count and its findings (the first `NETWORK_SHOWN`).
    /// None when the city has no places.
    pub fn source_groups(&self) -> Vec<SourceGroup> {
        if self.has_no_places() {
            return Vec::new();
        }
        let report = self.source_report();
        SourceKind::ALL.into_iter().map(|kind| self.source_group(kind, report.findings.iter().filter(|f| f.kind == kind).collect())).collect()
    }

    fn source_group(&self, kind: SourceKind, found: Vec<&SourceFinding>) -> SourceGroup {
        let hidden = found.len().saturating_sub(NETWORK_SHOWN);
        SourceGroup {
            id: kind.id(),
            label: self.word(&format!("prov-kind-{}", kind.id())),
            count: found.len(),
            count_text: self.source_count_text(found.len()),
            items: found.iter().take(NETWORK_SHOWN).map(|f| self.source_item(f)).collect(),
            more: (hidden > 0).then(|| self.i18n.tr("prov-more-findings", &Args::new().num("n", hidden as i64))),
        }
    }

    fn source_item(&self, f: &SourceFinding) -> SourceItem {
        let (places, more_places) = self.place_links(&f.places);
        SourceItem {
            kind: f.kind.id(),
            text: self.say(&f.said),
            source_label: self.word("prov-label-source"),
            impact_label: self.word("prov-label-impact"),
            source: self.say(&f.source),
            impact: self.say(&f.impact),
            places,
            more_places,
        }
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
            // A press with Shift held is a press all the same while an outline is drawn.
            MapEvent::Toggle { at, hot, vertex } => match self.areas.draw_now() {
                Draw::Drawing(_) => self.areas.on_press(at, vertex),
                Draw::Naming(_) => {}
                Draw::Idle => self.toggle_selected(hot.as_deref()),
            },
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
        let in_conflict: BTreeSet<u32> =
            self.findings_in(&view, &self.areas.list_now()).into_iter().filter(|s| !s.conflicts.is_empty()).map(|s| s.uid).collect();
        self.ports.mapper.set_places(&style::layers(area.lat), &overlay::places(&view, &projection, &in_conflict));
        let curbside = (style::curbside_layers(), overlay::curbside(&view, &projection));
        if *self.sent_curbside.borrow() != curbside {
            self.ports.mapper.set_curbside(&curbside.0, &curbside.1);
            *self.sent_curbside.borrow_mut() = curbside;
        }
        let stops = (style::stop_layers(), overlay::stops(&view, &projection));
        if *self.sent_stops.borrow() != stops {
            self.ports.mapper.set_stops(&stops.0, &stops.1);
            *self.sent_stops.borrow_mut() = stops;
        }
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

    // ---- the selected streets ----

    /// The selected streets' view-model.
    pub fn selection(&self) -> &Rc<SelectionVm> {
        &self.selection
    }

    /// Adds the street `hot` names to the selection, or drops it when it is selected. A junction is
    /// refused, and so is a street not wholly in view (as it is not opened), each saying why; a street already
    /// selected can always be dropped.
    pub fn toggle_selected(&self, hot: Option<&str>) {
        let Some(hot) = hot else { return };
        let Some(uid) = hot.strip_prefix("s-").and_then(|u| u.parse::<u32>().ok()) else {
            return self.ports.announcer.say(&self.i18n.tr_now("sel-not-a-street", &Args::new()));
        };
        if !self.selection.contains(uid) && !self.can_edit(&format!("s-{uid}")) {
            return self.say_not_editable();
        }
        self.selection.toggle(uid, &self.street_label(uid));
        self.show_selection();
        self.corridor.set(Stage::Idle);
    }

    /// Drops one street from the selection (the panel's button).
    pub fn deselect(&self, uid: u32) {
        self.selection.remove(uid, &self.street_label(uid));
        self.show_selection();
        self.corridor.set(Stage::Idle);
    }

    pub fn clear_selection(&self) {
        self.selection.clear();
        self.show_selection();
        self.corridor.set(Stage::Idle);
    }

    // ---- changing the selected streets ----

    pub fn corridor_op(&self) -> Op {
        self.corridor_op.get()
    }

    /// Picks the change by its id; a review made for another change goes.
    pub fn set_corridor_op(&self, id: &str) {
        if let Some(op) = Op::from_id(id) {
            self.corridor_op.set(op);
            self.corridor.set(Stage::Idle);
        }
    }

    pub fn corridor_stage(&self) -> Stage {
        self.corridor.get()
    }

    /// Tries the picked change on every selected street, on copies, and shows which can take it and which cannot,
    /// with the reason. Nothing is changed. Nothing is selected, nothing to review.
    pub fn review_corridor(&self) {
        let streets = self.selection.streets_now();
        if streets.is_empty() {
            return;
        }
        let plan = corridor::plan(&self.store.open(), self.store.region(), self.corridor_op.get_untracked(), &streets);
        let said = self.i18n.tr_now(
            "corridor-review",
            &Args::new().str("name", self.say_now(&plan.op.name())).num("ready", plan.ready().len() as i64).num("refused", plan.refused().len() as i64),
        );
        self.corridor.set(Stage::Reviewing(plan));
        self.ports.announcer.say(&said);
    }

    /// Makes the reviewed change on the streets that can take it; the others are left as they are. The map shows
    /// the result. Only from a review.
    pub fn apply_corridor(&self) {
        let Stage::Reviewing(plan) = self.corridor.get_untracked() else { return };
        // Only what was reviewed as ready is made; what the review refused stays in the report as it was said.
        let mut report = corridor::apply(&self.store, plan.op, &plan.ready());
        report.refused.splice(0..0, plan.refused());
        self.reload();
        self.sync_places();
        self.ports.announcer.say(&self.say_now(&report.said()));
        if !report.changed.is_empty() {
            self.undoable.set(Some((report.op, report.undo.clone())));
        }
        self.corridor.set(Stage::Done(report));
    }

    /// What the button that takes the last change back says, while there is one to take back.
    pub fn corridor_undo_label(&self) -> Option<String> {
        self.undoable
            .get()
            .map(|(op, undo)| self.i18n.tr("corridor-undo-button", &Args::new().str("name", self.say_op(op)).num("count", undo.before.len() as i64)))
    }

    /// Takes the last change back, all of it or none: refused, and said so, when a street it changed was edited
    /// since (here or on its own page, by sync or another tab).
    pub fn undo_corridor(&self) {
        let Some((op, undo)) = self.undoable.get_untracked() else { return };
        let name = self.say_now(&op.name());
        let said = match corridor::undo(&self.store, &undo) {
            corridor::Undone::Restored(streets) => {
                self.undoable.set(None);
                self.corridor.set(Stage::Idle);
                self.i18n.tr_now("corridor-undone", &Args::new().str("name", name).num("count", streets.len() as i64))
            }
            corridor::Undone::Refused(streets) => self.i18n.tr_now("corridor-undo-refused", &Args::new().num("count", streets.len() as i64)),
            corridor::Undone::NotKept(streets) => self.i18n.tr_now("corridor-undo-failed", &Args::new().num("count", streets.len() as i64)),
        };
        self.reload();
        self.sync_places();
        self.ports.announcer.say(&said);
    }

    /// Leaves the review or the report.
    pub fn cancel_corridor(&self) {
        self.corridor.set(Stage::Idle);
    }

    /// The streets of a review or report, each with what to say of it, in the language of the page.
    pub fn corridor_rows(&self) -> Vec<CorridorRow> {
        let units = self.units();
        let say = |s: &Said| said::say(&self.i18n, units, s);
        let row = |street: u32, ready: bool, text: String| CorridorRow { street, name: self.street_label(street), ready, text };
        match self.corridor.get() {
            Stage::Idle => Vec::new(),
            Stage::Reviewing(plan) => plan
                .rows
                .iter()
                .map(|r| match &r.outcome {
                    Outcome::Ready => row(r.street, true, self.word("corridor-row-ready")),
                    Outcome::Refused(why) => row(r.street, false, say(why)),
                })
                .collect(),
            Stage::Done(report) => report
                .changed
                .iter()
                .map(|&s| row(s, true, self.word("corridor-row-changed")))
                .chain(report.refused.iter().map(|(s, why)| row(*s, false, say(why))))
                .chain(report.not_kept.iter().map(|&s| row(s, false, self.word("corridor-row-not-kept"))))
                .collect(),
        }
    }

    /// How many streets of the review can be changed.
    pub fn corridor_ready(&self) -> usize {
        match self.corridor.get() {
            Stage::Reviewing(plan) => plan.ready().len(),
            _ => 0,
        }
    }

    /// An operation's name, in the language of the page (tracked).
    pub fn say_op(&self, op: Op) -> String {
        said::say(&self.i18n, self.units(), &op.name())
    }

    /// The button that makes the reviewed change: how many streets it will change.
    pub fn corridor_apply_label(&self, count: usize) -> String {
        self.i18n.tr("corridor-apply", &Args::new().num("count", count as i64))
    }

    fn say_now(&self, said: &Said) -> String {
        said::say_now(&self.i18n, self.units(), said)
    }

    /// The Selected streets panel's lead for `count` streets.
    pub fn selection_lead(&self, count: usize) -> String {
        self.i18n.tr("sel-lead", &Args::new().num("count", count as i64))
    }

    /// What a street's button in the panel is called, for a screen reader.
    pub fn selection_remove_label(&self, name: &str) -> String {
        self.i18n.tr("sel-remove", &Args::new().str("name", name))
    }

    /// Tells the map which streets are selected.
    fn show_selection(&self) {
        self.ports.mapper.set_selected(&self.selection.hot());
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
    /// allow. A street inside several areas is found once for each.
    fn findings_in(&self, v: &CityView, list: &[Restriction]) -> Vec<AreaStreet> {
        let Some(p) = self.projection() else { return Vec::new() };
        let mut found = Vec::new();
        for r in list {
            for e in v.edges.iter().filter(|e| r.holds(&e.shape_mm, &p)) {
                #[cfg(test)]
                self.editors_read.set(self.editors_read.get() + 1);
                let Some(street) = self.core.read(|m| m.city.street_editor(e.uid, m.region).map(|e| e.snapshot())) else { continue };
                let conflicts = conflicts(r.rule, &street);
                let speed_unknown = speed_unknown(r.rule, &street);
                let name = street_note(e.called.clone(), e.ends.clone());
                found.push(AreaStreet { area: r.id.clone(), uid: e.uid, href: street_href(e.uid), name, rule: r.rule, conflicts, speed_unknown });
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
        } else {
            Said::new("restriction-summary").with("inside", Arg::Num(inside)).with("conflicts", Arg::Num(in_conflict))
        };
        self.say(&Said::new("restriction-row-sub").with("rule", Arg::Said(Box::new(r.rule.said()))).with("summary", Arg::Said(Box::new(summary))))
    }

    /// How a street inside an area stands, in words.
    pub fn street_detail(&self, s: &AreaStreet) -> String {
        if s.speed_unknown {
            return self.word("restriction-check-speed-unknown");
        }
        if s.conflicts.is_empty() {
            return self.word("restriction-street-keeps");
        }
        s.conflicts.iter().map(|c| self.say(&c.said(s.rule))).collect::<Vec<_>>().join(" ")
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
    use cityloom_model::{
        Control, Direction, Edge, Junction, Lane, LaneKind, Network, NodeId, NodeRef, Point, Provenance, Road, RoadId, RoadType, WayRef, Zone,
    };

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

    /// Another page writes a junction of the city with `edit` applied to its editor.
    fn edit_junction(storage: &Rc<MemoryStorage>, node: u32, edit: impl FnOnce(&mut crate::junction::model::Junction)) {
        let store = CityStore::sample(storage.clone());
        let mut j = store.open().junction_editor(node, 0).unwrap();
        edit(&mut j);
        assert!(store.write_junction(node, j.snapshot()));
    }

    fn found(vm: &MapVm, id: &str) -> NetworkCheck {
        vm.network_rules().into_iter().find(|r| r.id == id).unwrap()
    }

    #[test]
    fn the_network_checks_are_six_rules_with_a_count_each_and_the_hand_made_city_passes_them() {
        let (vm, ..) = vm();
        let rules = vm.network_rules();
        assert_eq!(rules.iter().map(|r| r.id).collect::<Vec<_>>(), ["cycling", "access", "crossings", "lanes", "transit", "walking"]);
        assert!(rules.iter().all(|r| r.count == 0 && r.items.is_empty() && r.count_text == "no findings"));
        assert_eq!(vm.network_count(), 0);
        assert_eq!(vm.network_lead(), "The network checks found nothing across the streets and junctions.");
        assert_eq!(rules[2].label, "Crossings");
    }

    /// Takes the sidewalks off a street of the hand-made city, as a street edit kept for the map to read; and puts one back.
    fn set_sidewalks(storage: &Rc<MemoryStorage>, street: u32, on: bool) {
        let store = CityStore::sample(storage.clone());
        let mut e = store.open().street_editor(street, 0).unwrap();
        let sidewalk = crate::shared::catalogue::kind_index("sidewalk").unwrap();
        if on {
            e.add(sidewalk, 0);
        } else {
            let uids: Vec<u32> = e.view().segments.iter().filter(|s| s.kind == sidewalk).map(|s| s.uid).collect();
            for u in uids {
                assert!(e.remove(u));
            }
        }
        assert!(store.write_street(street, e.snapshot()));
    }

    #[test]
    fn streets_without_sidewalks_cut_a_walking_route_and_a_fix_on_one_street_drops_the_findings() {
        let (vm, _, storage) = vm();
        assert_eq!(found(&vm, "walking").count, 0, "the hand-made city has walkable routes: {:?}", found(&vm, "walking"));
        // street 7 runs from the edge of the map to junction 1, where streets 5 and 8 meet it; with neither of those
        // walkable, its sidewalk stops at the junction and it is cut off from the rest of the network
        set_sidewalks(&storage, 5, false);
        set_sidewalks(&storage, 8, false);
        vm.reload();
        let walking = found(&vm, "walking");
        assert_eq!(walking.items.iter().filter(|i| i.text.contains("cannot walk")).count(), 1, "{walking:?}");
        assert!(walking.items.iter().all(|i| i.places.iter().any(|p| p.hot == "s-7")), "{walking:?}");
        assert!(walking.items.iter().any(|i| i.text.contains("the sidewalk of")), "{walking:?}");
        assert!(walking.items.iter().all(|i| i.places.iter().any(|p| p.hot == "j-3")), "the junction is named: {walking:?}");
        assert_eq!(vm.network_count(), walking.count, "only the walking check found anything");
        // a sidewalk put back on street 5 is the fix: the route goes on and the findings go
        set_sidewalks(&storage, 5, true);
        vm.reload();
        assert_eq!(found(&vm, "walking").count, 0, "{:?}", found(&vm, "walking"));
        assert_eq!(vm.network_count(), 0);
    }

    #[test]
    fn a_crossing_taken_away_is_found_naming_the_street_and_the_junction_and_the_map_counts_stay_apart() {
        let (vm, _, storage) = vm();
        let before = (vm.status(), vm.checks_lead(), vm.failing_items());
        let (node, arm) = {
            let city = CityStore::sample(storage.clone()).open();
            let j = city.junction_editor(2, 0).unwrap();
            (2, j.snapshot().arms.iter().find(|a| a.crossing.is_some()).map(|a| a.uid).unwrap())
        };
        edit_junction(&storage, node, |j| assert!(j.set_crossing(arm, false)));
        vm.reload();
        let crossings = found(&vm, "crossings");
        assert_eq!(crossings.count, 1, "{crossings:?}");
        assert_eq!(crossings.count_text, "1 finding");
        let item = &crossings.items[0];
        assert_eq!((item.severity, item.severity_word.as_str()), ("problem", "Problem"));
        assert_eq!(item.text, "This street takes traffic into the junction and has no pedestrian crossing.");
        assert_eq!(item.places.len(), 2);
        assert!(item.places[0].href.starts_with("street.html?street=") && item.places[1].href == "intersection.html?junction=2");
        assert_eq!(item.places[1].hot, "j-2");
        assert!(vm.network_lead().starts_with("The network checks found 1 finding"));
        // the places' own checks do not count it
        let after = (vm.status(), vm.checks_lead(), vm.failing_items());
        assert_eq!(before.0.bad, after.0.bad);
        assert_eq!(before.2, after.2);
        // put back, the finding clears
        edit_junction(&storage, node, |j| assert!(j.set_crossing(arm, true)));
        vm.reload();
        assert_eq!(found(&vm, "crossings").count, 0);
    }

    #[test]
    fn a_check_lists_the_first_findings_and_counts_the_rest_and_a_finding_links_the_first_places() {
        let (vm, ..) = vm();
        let place = |uid: u32| network::Place { kind: PlaceKind::Street, uid, name: Said::new("map-note-street") };
        let finding = |n: u32| NetworkFinding {
            rule: Check::Cycling,
            severity: Severity::Problem,
            said: Said::new("net-cycling-island").with("n", Arg::Num(n as i64)),
            places: (1..=n).map(place).collect(),
        };
        let all: Vec<NetworkFinding> = (0..NETWORK_SHOWN as u32 + 12).map(|_| finding(11)).collect();
        let check = vm.network_check(Check::Cycling, all.iter().collect());
        assert_eq!(check.count, NETWORK_SHOWN + 12, "the count is all of them");
        assert_eq!(check.count_text, format!("{} findings", NETWORK_SHOWN + 12));
        assert_eq!(check.items.len(), NETWORK_SHOWN);
        assert_eq!(check.more.as_deref(), Some("12 more findings are not listed."));
        assert_eq!(check.items[0].places.len(), NETWORK_PLACES);
        assert_eq!(check.items[0].more_places.as_deref(), Some("and 3 more places"));
        assert_eq!(check.items[0].rule, "cycling");
        let few = vm.network_check(Check::Cycling, vec![&finding(2)]);
        assert!(few.more.is_none() && few.items[0].more_places.is_none());
    }

    #[test]
    fn junctions_that_could_not_be_read_are_said_and_not_taken_for_nothing_found() {
        let (vm, ..) = vm();
        *vm.network.borrow_mut() = Some((vm.view(), Rc::new(NetworkReport { findings: Vec::new(), unreadable: 1 })));
        assert_eq!(
            vm.network_lead(),
            "The network checks found nothing across the streets and junctions. 1 junction could not be read, so its crossings are not checked."
        );
        *vm.network.borrow_mut() = Some((vm.view(), Rc::new(NetworkReport { findings: Vec::new(), unreadable: 3 })));
        assert!(vm.network_lead().ends_with("3 junctions could not be read, so their crossings are not checked."));
        vm.i18n().set(Locale::FrCa);
        assert!(vm.network_lead().contains("3 jonctions n’ont pas pu être lues"), "{}", vm.network_lead());
    }

    #[test]
    fn a_city_with_no_places_has_no_network_checks_only_its_lead() {
        let (ports, ..) = test_ports();
        let vm = MapVm::new(ports, crate::i18n_for(Locale::En)); // the default area, whose roads are not kept: no places
        assert!(vm.network_rules().is_empty());
        assert_eq!(vm.network_lead(), "There is no network to check.");
    }

    #[test]
    fn the_findings_follow_the_language_and_are_worked_out_once_per_city() {
        let (vm, _, storage) = vm();
        let (node, arm) = {
            let j = CityStore::sample(storage.clone()).open().junction_editor(2, 0).unwrap();
            (2, j.snapshot().arms.iter().find(|a| a.crossing.is_some()).map(|a| a.uid).unwrap())
        };
        edit_junction(&storage, node, |j| assert!(j.set_crossing(arm, false)));
        vm.reload();
        let first = vm.network_report();
        assert!(Rc::ptr_eq(&first, &vm.network_report()));
        vm.i18n().set(Locale::FrCa);
        assert_eq!(found(&vm, "crossings").label, "Passages pour piétons");
        assert!(found(&vm, "crossings").items[0].text.contains("passage pour piétons"));
        vm.reload();
        assert!(!Rc::ptr_eq(&first, &vm.network_report()));
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
        let node = |id: u32, x_m, y_m| Junction {
            id: NodeId(id),
            provenance: Provenance::of_nodes([NodeRef::new(id as i64)]),
            position: Point::new(x_m, y_m),
            junction: false,
            control: Control::None,
        };
        let lane = |way| Lane { kind: LaneKind::Driving, direction: way, width_m: 3.0, buffer_type: None, windows: vec![], shape: vec![] };
        let network = Network {
            left_hand: false,
            corridors: vec![],
            junctions: vec![node(1, 0.0, 0.0), node(2, 100.0, 50.0)],
            roads: vec![Road {
                edge: Edge { id: RoadId(1), from: NodeId(1), to: NodeId(2) },
                provenance: Provenance::of_ways([WayRef::new(7)]),
                name: "Bend Street".into(),
                road_type: RoadType::Local,
                zones: vec![Zone::whole(vec![lane(Direction::Forward), lane(Direction::Backward)])],
                points: vec![Point::new(0.0, 0.0), Point::new(80.0, 0.0), Point::new(100.0, 50.0)],
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
    fn a_shift_press_adds_the_street_to_the_selection_and_another_drops_it_and_the_map_is_told() {
        let (vm, mapper, navigator, said) = area_vm_said();
        mapper.say(MapEvent::Toggle { at: [-73.6, 45.5], hot: Some("s-1".into()), vertex: None });
        assert_eq!(vm.selection().streets(), vec![1]);
        assert_eq!(mapper.take().into_iter().filter(|c| matches!(c, MapCall::Selected(_))).collect::<Vec<_>>(), vec![MapCall::Selected(vec!["s-1".into()])]);
        assert!(said.take().iter().any(|s| s.contains("added to the selection. One street selected.")));
        mapper.say(MapEvent::Toggle { at: [-73.6, 45.5], hot: Some("s-1".into()), vertex: None });
        assert!(vm.selection().streets().is_empty());
        assert_eq!(mapper.take(), vec![MapCall::Selected(vec![])]);
        assert!(navigator.take().is_empty(), "a shift press opens nothing");
    }

    #[test]
    fn only_a_street_that_can_be_edited_is_selected_and_anything_else_says_why() {
        let (vm, mapper, _, said) = area_vm_said();
        mapper.say(COVERING);
        mapper.take();
        said.take();
        mapper.say(MapEvent::Toggle { at: [-73.6, 45.5], hot: Some("j-3".into()), vertex: None });
        mapper.say(MapEvent::Toggle { at: [-73.6, 45.5], hot: None, vertex: None });
        assert_eq!(said.take(), vec!["Only streets can be selected.".to_string()], "a junction is refused, nothing at all is not worth saying");
        // a street the open view does not hold is muted: it is no more selected than opened
        mapper.say(MapEvent::View { bounds: clipping(street_box(&vm)) });
        mapper.take();
        said.take();
        mapper.say(MapEvent::Toggle { at: [-73.6, 45.5], hot: Some("s-1".into()), vertex: None });
        assert!(vm.selection().streets().is_empty());
        assert_eq!(said.take(), vec!["Not all of this place is in view. Move the map so all of it shows, then try again.".to_string()]);
        assert!(mapper.take().is_empty());
    }

    #[test]
    fn a_shift_press_while_an_outline_is_drawn_is_still_a_corner_and_selects_nothing() {
        let (vm, mapper, ..) = area_vm();
        mapper.say(COVERING);
        mapper.take();
        assert!(vm.areas().start());
        mapper.say(MapEvent::Toggle { at: [-73.6, 45.5], hot: Some("s-1".into()), vertex: None });
        assert!(vm.selection().streets().is_empty());
        assert!(matches!(vm.areas().draw_now(), Draw::Drawing(corners) if corners.len() == 1), "the press went to the outline as its first corner");
    }

    // ---- changing the selected streets ----

    fn sample_vm() -> (Rc<MapVm>, Rc<RecordingAnnouncer>) {
        let (ports, said, _) = test_ports();
        (MapVm::on_sample(ports, crate::i18n_for(Locale::En)), said)
    }

    /// A street of the sample city that takes the Greenway; the freeway (23) takes no template.
    fn greenway_street(vm: &MapVm) -> u32 {
        let city = vm.store.open();
        (1..=22u32).find(|&s| corridor::plan(&city, 0, Op::Template(crate::street::model::Template::Greenway), &[s]).ready() == vec![s]).unwrap()
    }

    #[test]
    fn nothing_selected_has_nothing_to_review() {
        let (vm, said) = sample_vm();
        vm.review_corridor();
        assert_eq!(vm.corridor_stage(), Stage::Idle);
        assert!(said.take().is_empty());
    }

    #[test]
    fn a_review_says_which_selected_streets_can_take_the_change_and_changes_none() {
        let (vm, said) = sample_vm();
        let ok = greenway_street(&vm);
        let before = vm.store.open().street_editor(ok, 0).unwrap().snapshot();
        vm.toggle_selected(Some(&format!("s-{ok}")));
        vm.toggle_selected(Some("s-23"));
        said.take();
        vm.set_corridor_op("greenway");
        vm.review_corridor();
        assert_eq!(said.take(), vec!["Neighbourhood greenway: 1 street can be changed. 1 street cannot, and will be left as it is.".to_string()]);
        let rows = vm.corridor_rows();
        assert_eq!((rows[0].ready, rows[0].text.as_str()), (true, "Can be changed."));
        assert!(!rows[1].ready && rows[1].text.contains("freeway"), "{:?}", rows[1]);
        assert_eq!(vm.corridor_ready(), 1);
        assert_eq!(vm.store.open().street_editor(ok, 0).unwrap().snapshot(), before, "a review writes nothing");
    }

    #[test]
    fn applying_changes_the_reviewed_streets_says_what_was_done_and_leaves_the_refused_alone() {
        let (vm, said) = sample_vm();
        let ok = greenway_street(&vm);
        let before = vm.store.open().street_editor(ok, 0).unwrap().snapshot();
        let free = vm.store.open().street_editor(23, 0).unwrap().snapshot();
        vm.toggle_selected(Some(&format!("s-{ok}")));
        vm.toggle_selected(Some("s-23"));
        vm.set_corridor_op("greenway");
        vm.apply_corridor();
        assert_eq!(vm.corridor_stage(), Stage::Idle, "nothing is made that was not reviewed");
        vm.review_corridor();
        said.take();
        vm.apply_corridor();
        assert_eq!(said.take(), vec!["Neighbourhood greenway: 1 street changed. 1 was left as it was.".to_string()]);
        assert!(matches!(vm.corridor_stage(), Stage::Done(r) if r.changed == vec![ok]));
        assert_ne!(vm.store.open().street_editor(ok, 0).unwrap().snapshot(), before);
        assert_eq!(vm.store.open().street_editor(23, 0).unwrap().snapshot(), free);
        assert_eq!(vm.selection().streets(), vec![ok, 23], "the selection stays");
    }

    #[test]
    fn a_review_goes_when_the_selection_or_the_change_does() {
        let (vm, _) = sample_vm();
        let ok = greenway_street(&vm);
        vm.toggle_selected(Some(&format!("s-{ok}")));
        vm.review_corridor();
        assert!(matches!(vm.corridor_stage(), Stage::Reviewing(_)));
        vm.toggle_selected(Some("s-23"));
        assert_eq!(vm.corridor_stage(), Stage::Idle);
        vm.review_corridor();
        vm.set_corridor_op("pedestrian");
        assert_eq!(vm.corridor_stage(), Stage::Idle);
        vm.review_corridor();
        vm.clear_selection();
        assert_eq!(vm.corridor_stage(), Stage::Idle);
    }

    fn change_two_streets(vm: &MapVm) -> [u32; 2] {
        let city = vm.store.open();
        let ready: Vec<u32> = (1..=22u32)
            .filter(|&s| corridor::plan(&city, 0, Op::Template(crate::street::model::Template::Greenway), &[s]).ready() == vec![s])
            .take(2)
            .collect();
        for s in &ready {
            vm.toggle_selected(Some(&format!("s-{s}")));
        }
        vm.set_corridor_op("greenway");
        vm.review_corridor();
        vm.apply_corridor();
        [ready[0], ready[1]]
    }

    #[test]
    fn a_change_can_be_undone_from_the_map_even_after_the_selection_is_cleared() {
        let (vm, said) = sample_vm();
        assert_eq!(vm.corridor_undo_label(), None, "nothing to undo before a change");
        let [a, b] = change_two_streets(&vm);
        let (changed_a, changed_b) = (vm.store.open().street_editor(a, 0).unwrap().snapshot(), vm.store.open().street_editor(b, 0).unwrap().snapshot());
        assert_eq!(vm.corridor_undo_label().as_deref(), Some("Undo: Neighbourhood greenway on 2 streets"));
        vm.clear_selection();
        said.take();
        vm.undo_corridor();
        assert_eq!(said.take(), vec!["Undone: Neighbourhood greenway. 2 streets are as they were.".to_string()]);
        assert_eq!(vm.corridor_undo_label(), None, "it is undone once");
        assert_ne!(vm.store.open().street_editor(a, 0).unwrap().snapshot(), changed_a);
        assert_ne!(vm.store.open().street_editor(b, 0).unwrap().snapshot(), changed_b);
    }

    #[test]
    fn an_undo_is_refused_when_a_street_was_edited_since_and_nothing_is_restored() {
        let (vm, said) = sample_vm();
        let [a, b] = change_two_streets(&vm);
        let mut e = vm.store.open().street_editor(b, 0).unwrap();
        let u = e.view().segments[0].uid;
        assert!(e.set_width(u, e.view().segments[0].width_mm + 100));
        assert!(vm.store.write_street(b, e.snapshot()));
        let (now_a, now_b) = (vm.store.open().street_editor(a, 0).unwrap().snapshot(), vm.store.open().street_editor(b, 0).unwrap().snapshot());
        said.take();
        vm.undo_corridor();
        assert_eq!(said.take(), vec!["Nothing was undone: 1 street has been edited since the change.".to_string()]);
        assert_eq!(vm.store.open().street_editor(a, 0).unwrap().snapshot(), now_a);
        assert_eq!(vm.store.open().street_editor(b, 0).unwrap().snapshot(), now_b);
        assert!(vm.corridor_undo_label().is_some(), "the undo is kept: nothing was lost");
    }

    #[test]
    fn a_new_change_replaces_the_one_that_can_be_undone() {
        let (vm, _) = sample_vm();
        change_two_streets(&vm);
        vm.cancel_corridor();
        vm.set_corridor_op("pedestrian");
        vm.review_corridor();
        vm.apply_corridor();
        let label = vm.corridor_undo_label().unwrap();
        assert!(label.starts_with("Undo: Make pedestrian"), "{label}");
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
        assert_eq!(found[0].conflicts, vec![Conflict::Pieces { kind: "travel", pieces: 2 }]);
        assert_eq!(vm.say(&found[0].name), "Bend Street, through the city");
        assert_eq!(vm.street_detail(&found[0]), "It has 2 driving lanes.");
        let r = vm.areas().find_now("r1").unwrap();
        assert_eq!(vm.area_row_sub(&r), "Car-free · One street inside; one conflicts with the rule.");
        vm.sync_places();
        assert!(matches!(&mapper.take()[0], MapCall::Places { data, .. } if data.contains("\"conflict\":true")));
        assert!(vm.areas().set_rule("r1", Rule::SpeedLimit { kmh: 30 }));
        assert_eq!(vm.area_row_sub(&vm.areas().find_now("r1").unwrap()), "Speed limit of 30 km/h · One street inside; none conflicts with the rule.");
        assert_eq!(vm.street_detail(&vm.findings()[0]), "Speed not known: OpenStreetMap gives none and none is set.");
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

    /// Bendville with its street's speed set to `kmh` and a 30 km/h area over it.
    fn slow_bendville(kmh: Option<u16>) -> (Rc<MapVm>, Rc<FakeMapper>) {
        let (vm, mapper, _, _, storage) = area_vm_over(|store| {
            let mut street = store.open().street_editor(1, 0).unwrap().snapshot();
            street.speed = crate::street::model::Speed::from_kmh(kmh);
            assert!(store.write_street(1, street));
        });
        mapper.say(COVERING);
        mapper.take();
        let (_, mid) = bent_midpoint(&CityStore::current(storage));
        assert!(vm.areas().start());
        for p in square(mid, 0.0005) {
            assert!(vm.areas().add_at(p));
        }
        assert!(vm.areas().close());
        assert!(vm.areas().save());
        assert!(vm.areas().set_rule("r1", Rule::SpeedLimit { kmh: 30 }));
        mapper.take();
        (vm, mapper)
    }

    #[test]
    fn a_speed_limit_area_tints_the_streets_over_its_limit() {
        let (vm, mapper) = slow_bendville(Some(50));
        vm.sync_places();
        assert!(matches!(&mapper.take()[0], MapCall::Places { data, .. } if data.contains("\"conflict\":true")));
        assert_eq!(vm.findings()[0].conflicts, vec![Conflict::Speed { kmh: 50 }]);
        assert_eq!(vm.street_detail(&vm.findings()[0]), "Speed limit 50 km/h, over the area\u{2019}s 30 km/h.");

        let (vm, mapper) = slow_bendville(None);
        vm.sync_places();
        assert!(matches!(&mapper.take()[0], MapCall::Places { data, .. } if data.contains("\"conflict\":false")));
        assert_eq!(vm.street_detail(&vm.findings()[0]), "Speed not known: OpenStreetMap gives none and none is set.");
    }

    #[test]
    fn a_speed_limit_area_counts_its_breaking_streets_in_the_status_and_the_lead() {
        let (vm, _) = slow_bendville(Some(50));
        let owner = Owner::new();
        owner.set();
        assert_eq!(vm.status(), Status { text: "1 place. One street conflicts with an area rule.".to_string(), bad: true });
        assert_eq!(vm.area_row_sub(&vm.areas().find_now("r1").unwrap()), "Speed limit of 30 km/h · One street inside; one conflicts with the rule.");

        let (vm, _) = slow_bendville(None);
        assert_eq!(vm.status(), Status { text: "1 place. Every check passes.".to_string(), bad: false });
        assert_eq!(vm.checks_lead(), "Every check passes in the one place.");
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

    /// `area_vm` whose one street has a parking lane and a use on it, and a bulb stop.
    fn programme_vm() -> (Rc<MapVm>, Rc<FakeMapper>, Rc<RecordingNavigator>, Rc<RecordingAnnouncer>) {
        let (vm, mapper, navigator, said, _) = area_vm_over(|store| {
            let city = store.open();
            let mut street = city.street_editor(1, 0).unwrap().snapshot();
            let parking = crate::shared::catalogue::kind_index("parking").unwrap();
            street.segments.last_mut().unwrap().kind = parking;
            let shape = city.street_shape(1).unwrap();
            street.length_mm = Some(shape.windows(2).map(|w| ((w[1][0] - w[0][0]) as f64).hypot((w[1][1] - w[0][1]) as f64)).sum::<f64>().round() as i32);
            street.curb = vec![crate::street::model::Instance {
                side: crate::shared::catalogue::Side::Right,
                from_mm: 10_000,
                length_mm: 6_000,
                use_: crate::street::model::Use::Parking,
            }];
            street.stops = vec![crate::street::model::Stop {
                side: crate::shared::catalogue::Side::Right,
                from_mm: 30_000,
                length_mm: 12_000,
                kind: crate::street::model::StopKind::Bulb,
                shelter: false,
                name: Some("Rue Bend".into()),
            }];
            assert!(store.write_street(1, street));
        });
        (vm, mapper, navigator, said)
    }

    #[test]
    fn places_sync_sends_the_curbside_and_the_stops() {
        let (vm, mapper, ..) = programme_vm();
        mapper.say(COVERING);
        let view = vm.core.view_now();
        let projection = vm.projection().unwrap();
        let (layers, data) = mapper.curbside();
        assert_eq!(layers, style::curbside_layers());
        assert_eq!(data, overlay::curbside(&view, &projection));
        let fc: serde_json::Value = serde_json::from_str(&data).unwrap();
        assert_eq!(fc["features"].as_array().unwrap().len(), 1, "{data}");
        assert_eq!(fc["features"][0]["properties"]["use"], "parking");
        let (layers, data) = mapper.stops();
        assert_eq!((layers, data.as_str()), (style::stop_layers(), overlay::stops(&view, &projection).as_str()));
        let fc: serde_json::Value = serde_json::from_str(&data).unwrap();
        let parts: Vec<&str> = fc["features"].as_array().unwrap().iter().map(|f| f["properties"]["part"].as_str().unwrap()).collect();
        assert_eq!(parts, ["platform", "dot"], "{data}");
        assert_eq!(fc["features"][1]["properties"]["name"], "Rue Bend");
    }

    #[test]
    fn the_same_data_is_not_sent_twice() {
        let (vm, mapper, ..) = programme_vm();
        mapper.say(COVERING);
        assert_eq!((mapper.curbside_sent(), mapper.stops_sent()), (1, 1));
        vm.sync_places();
        vm.sync_places();
        assert_eq!((mapper.curbside_sent(), mapper.stops_sent()), (1, 1), "nothing changed");
    }

    #[test]
    fn a_changed_curbside_is_sent_again() {
        let (vm, mapper, ..) = programme_vm();
        mapper.say(COVERING);
        assert_eq!((mapper.curbside_sent(), mapper.stops_sent()), (1, 1));
        let before = mapper.curbside().1;
        let mut street = vm.store.open().street_editor(1, 0).unwrap().snapshot();
        street.curb.clear();
        assert!(vm.store.write_street(1, street));
        vm.reload();
        vm.sync_places();
        assert_eq!((mapper.curbside_sent(), mapper.stops_sent()), (2, 1), "only the curbside changed");
        let after = mapper.curbside().1;
        assert_ne!(after, before);
        assert!(serde_json::from_str::<serde_json::Value>(&after).unwrap()["features"].as_array().unwrap().is_empty());
    }

    #[test]
    fn a_press_on_a_muted_streets_use_says_not_editable() {
        let (vm, mapper, navigator, said) = programme_vm();
        mapper.say(COVERING);
        mapper.say(MapEvent::View { bounds: clipping(street_box(&vm)) });
        said.take();
        mapper.say(press("s-1"));
        assert!(navigator.take().is_empty());
        assert_eq!(said.take(), vec!["Not all of this place is in view. Move the map so all of it shows, then try again.".to_string()]);
    }

    #[test]
    fn a_press_on_a_use_opens_its_street() {
        let (_vm, mapper, navigator, _) = programme_vm();
        mapper.say(COVERING);
        mapper.say(press("s-1"));
        assert_eq!(navigator.take(), vec!["street.html?street=1".to_string()]);
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

    /// `area_vm` whose one street has a parking lane, `uses` uses on it and `stops` bulb stops, in `zones`
    /// zones.
    fn counted_vm(uses: usize, stops: usize, zones: usize) -> Rc<MapVm> {
        let (vm, ..) = area_vm_over(|store| {
            let city = store.open();
            let mut street = city.street_editor(1, 0).unwrap().snapshot();
            let parking = crate::shared::catalogue::kind_index("parking").unwrap();
            street.segments.last_mut().unwrap().kind = parking;
            let shape = city.street_shape(1).unwrap();
            let length = shape.windows(2).map(|w| ((w[1][0] - w[0][0]) as f64).hypot((w[1][1] - w[0][1]) as f64)).sum::<f64>().round() as i32;
            street.length_mm = Some(length);
            street.curb = (0..uses as i32)
                .map(|i| crate::street::model::Instance {
                    side: crate::shared::catalogue::Side::Right,
                    from_mm: 10_000 + i * 8_000,
                    length_mm: 6_000,
                    use_: crate::street::model::Use::Parking,
                })
                .collect();
            street.stops = (0..stops as i32)
                .map(|i| crate::street::model::Stop {
                    side: crate::shared::catalogue::Side::Right,
                    from_mm: 30_000 + i * 15_000,
                    length_mm: 12_000,
                    kind: crate::street::model::StopKind::Bulb,
                    shelter: false,
                    name: None,
                })
                .collect();
            if zones > 1 {
                street = crate::street::model::zoned(street, &[0, length / 2], length);
            }
            assert!(store.write_street(1, street));
        });
        vm
    }

    #[test]
    fn a_street_with_uses_and_a_stop_says_so_in_its_row_and_label() {
        let vm = counted_vm(2, 1, 1);
        let e = &vm.view().edges[0];
        assert_eq!((e.curb_count, e.stop_count), (2, 1));
        let label = vm.street_label(e.uid);
        assert!(label.contains(" wide, 2 curbside uses, 1 stop.") && label.ends_with(" Opens the street cross-section."), "{label}");
        assert!(sub_of(&vm, &vm.street_rows()[0]).ends_with(" \u{b7} 2 curbside uses, 1 stop"));
        let (vm_uses, vm_stops) = (counted_vm(1, 0, 1), counted_vm(0, 3, 1));
        assert!(sub_of(&vm_uses, &vm_uses.street_rows()[0]).ends_with(" \u{b7} 1 curbside use"));
        assert!(sub_of(&vm_stops, &vm_stops.street_rows()[0]).ends_with(" \u{b7} 3 stops"));
    }

    #[test]
    fn the_same_in_french() {
        let vm = counted_vm(2, 1, 1);
        vm.i18n().set(Locale::FrCa);
        let uid = vm.view().edges[0].uid;
        let label = vm.street_label(uid);
        assert!(label.contains(" de large, 2 usages du bord de rue, 1 arrêt d\u{2019}autobus."), "{label}");
        assert!(sub_of(&vm, &vm.street_rows()[0]).ends_with(" \u{b7} 2 usages du bord de rue, 1 arrêt d\u{2019}autobus"));
    }

    #[test]
    fn a_street_without_any_says_what_it_said() {
        let (vm, ..) = area_vm_all();
        for (i, e) in vm.view().edges.iter().enumerate() {
            assert_eq!((e.curb_count, e.stop_count), (0, 0));
            let width = Units::Metres.length(e.row_mm);
            assert_eq!(vm.street_label(e.uid), format!("{}, {}, {width} wide. Opens the street cross-section.", shown(&vm, &e.called), shown(&vm, &e.ends)));
            assert_eq!(sub_of(&vm, &vm.street_rows()[i]), format!("{} \u{b7} {width}", shown(&vm, &e.ends)));
        }
    }

    #[test]
    fn zones_and_programme_compose() {
        let vm = counted_vm(1, 0, 2);
        let e = &vm.view().edges[0];
        assert_eq!((e.zones, e.curb_count), (2, 1));
        let label = vm.street_label(e.uid);
        assert!(label.contains(" wide, 2 zones, 1 curbside use."), "{label}");
        assert!(sub_of(&vm, &vm.street_rows()[0]).ends_with(" \u{b7} 2 zones \u{b7} 1 curbside use"));
        vm.i18n().set(Locale::FrCa);
        let label = vm.street_label(e.uid);
        assert!(label.contains(" de large, en 2 zones, 1 usage du bord de rue."), "{label}");
        assert!(sub_of(&vm, &vm.street_rows()[0]).ends_with(" \u{b7} en 2 zones \u{b7} 1 usage du bord de rue"));
    }
}
