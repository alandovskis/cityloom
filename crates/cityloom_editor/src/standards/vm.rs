//! The Profiles section of the street page: the browser's standards profiles, to create, edit, choose, export, import
//! and remove. The view binds to this and decides nothing. It reads and writes `cityloom-standards` (no project's
//! storage, no sync) and tells the page, through `on_change`, when what is in force may have changed.

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::catalogue::{KINDS, StreetClass, class_key, kind_index};
use crate::shared::i18n::{Args, I18n};
use crate::shared::ports::Ports;
use crate::shared::said::{Arg, Said};
use crate::shared::units::{Units, parse_length_mm, parse_number};
use crate::standards::model::{Field, Refusal, SPEED_RANGE_KMH, Standards, WidthRange};
use crate::standards::store::{Profile, StandardsStore};
use crate::standards::text::say_now;

/// A profile as the list shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub id: String,
    pub name: String,
    pub in_force: bool,
}

/// What the person typed in the form of a profile, as typed.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Form {
    pub name: String,
    /// The five length figures, in `Field::LENGTHS` order, in the page's units.
    pub lengths: Vec<String>,
    /// The speed limits offered, separated by commas, in kilometres an hour.
    pub speeds: String,
    /// The target speed of each class, in `StreetClass::ALL` order; blank for none.
    pub targets: Vec<String>,
}

/// A length field of the form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LengthField {
    pub field: Field,
    pub value: String,
    /// The built-in figure, for the hint.
    pub builtin: Said,
}

/// A class's target field of the form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetField {
    pub class: &'static str,
    /// The message naming the class.
    pub class_key: &'static str,
    pub value: String,
}

/// A kind of piece whose widths the profile sets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WidthRow {
    pub kind: &'static str,
    pub said: Said,
}

/// The form of the profile being edited, filled with what it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormView {
    pub id: String,
    pub name: String,
    pub lengths: Vec<LengthField>,
    pub speeds: String,
    pub targets: Vec<TargetField>,
    pub widths: Vec<WidthRow>,
    /// What this profile makes looser than the built-in standards.
    pub warnings: Vec<Said>,
}

pub struct StandardsVm {
    ports: Ports,
    i18n: Rc<I18n>,
    store: StandardsStore,
    /// The units lengths are typed and shown in; read tracked by a view.
    units: Rc<dyn Fn() -> Units>,
    /// Called when what is in force, or what a profile in force holds, may have changed.
    on_change: Rc<dyn Fn()>,
    version: ArcRwSignal<u32>,
    editing: ArcRwSignal<Option<String>>,
    removing: ArcRwSignal<Option<String>>,
    /// The file of the profile just exported: its name and text.
    exported: ArcRwSignal<Option<(String, String)>>,
    error: ArcRwSignal<Option<Said>>,
}

impl StandardsVm {
    /// The section over `ports.storage` (the key is the browser's, whichever project the storage is scoped to).
    pub fn new(ports: Ports, i18n: Rc<I18n>, units: Rc<dyn Fn() -> Units>, on_change: Rc<dyn Fn()>) -> Rc<StandardsVm> {
        let store = StandardsStore::new(ports.storage.clone());
        Rc::new(StandardsVm {
            ports,
            i18n,
            store,
            units,
            on_change,
            version: ArcRwSignal::new(0),
            editing: ArcRwSignal::new(None),
            removing: ArcRwSignal::new(None),
            exported: ArcRwSignal::new(None),
            error: ArcRwSignal::new(None),
        })
    }

    pub fn i18n(&self) -> Rc<I18n> {
        self.i18n.clone()
    }

    /// The units, tracked.
    pub fn units(&self) -> Units {
        (self.units)()
    }

    fn units_now(&self) -> Units {
        untrack(|| (self.units)())
    }

    pub fn word(&self, key: &str) -> String {
        self.i18n.tr(key, &Args::new())
    }

    pub fn named(&self, key: &str, name: &str) -> String {
        self.i18n.tr(key, &Args::new().str("name", name.to_string()))
    }

    fn say(&self, key: &str, name: &str) {
        self.ports.announcer.say(&self.i18n.tr_now(key, &Args::new().str("name", name.to_string())));
    }

    /// Records and announces why something was refused; false.
    fn refuse(&self, refusal: &Refusal) -> bool {
        let said = refusal.said();
        self.ports.announcer.say(&say_now(&self.i18n, self.units_now(), &said));
        self.error.set(Some(said));
        false
    }

    fn changed(&self) {
        self.version.update(|v| *v = v.wrapping_add(1));
        self.error.set(None);
        (self.on_change)();
    }

    // ---- what is shown ----

    /// The profiles, in creation order; tracked.
    pub fn rows(&self) -> Vec<Row> {
        self.version.track();
        let active = self.store.active_id();
        self.store.list().into_iter().map(|p| Row { in_force: active.as_deref() == Some(p.id.as_str()), id: p.id, name: p.name }).collect()
    }

    /// The id of the profile in force, if any; tracked.
    pub fn in_force(&self) -> Option<String> {
        self.version.track();
        self.store.active_id()
    }

    /// The sentence saying which standard the checks use; tracked.
    pub fn in_force_text(&self) -> String {
        self.version.track();
        match self.store.active() {
            Some(p) => self.named("standards-active-profile", &p.name),
            None => self.word("standards-active-builtin"),
        }
    }

    /// The reason the last command was refused, in words; tracked.
    pub fn error(&self) -> Option<String> {
        self.error.get().map(|said| crate::standards::text::say(&self.i18n, self.units(), &said))
    }

    /// The profile being edited; tracked.
    pub fn editing(&self) -> Option<String> {
        self.editing.get()
    }

    /// The profile a question asks about removing; tracked.
    pub fn removing(&self) -> Option<String> {
        self.removing.get()
    }

    /// The file just exported: the profile's name and the text of the file; tracked.
    pub fn exported(&self) -> Option<(String, String)> {
        self.exported.get()
    }

    /// The form of the profile being edited, filled with what it holds; tracked.
    pub fn form(&self) -> Option<FormView> {
        self.version.track();
        let id = self.editing.get()?;
        let profile = self.store.get(&id)?;
        Some(self.form_of(&profile))
    }

    fn form_of(&self, profile: &Profile) -> FormView {
        let (units, locale) = (self.units(), self.i18n.locale());
        let s = &profile.standards;
        let builtin = |field: Field| Said::new("standards-builtin-hint").with("value", Arg::Length(field.builtin_mm().unwrap_or(0)));
        FormView {
            id: profile.id.clone(),
            name: profile.name.clone(),
            lengths: Field::LENGTHS
                .iter()
                .map(|&field| LengthField { field, value: units.fixed_in(s.length_mm(field).unwrap_or(0), 2, locale), builtin: builtin(field) })
                .collect(),
            speeds: s.speed_choices.iter().map(u16::to_string).collect::<Vec<_>>().join(", "),
            targets: StreetClass::ALL
                .iter()
                .map(|c| TargetField { class: c.id(), class_key: class_key(*c), value: s.target_kmh.get(c.id()).map(u16::to_string).unwrap_or_default() })
                .collect(),
            widths: s
                .kind_widths
                .iter()
                .filter_map(|(id, r)| {
                    let k = &KINDS[kind_index(id)?];
                    Some(WidthRow {
                        kind: k.id,
                        said: Said::new("standards-width-row")
                            .with("kind", Arg::Kind(k.id))
                            .with("min", Arg::Length(r.min_mm))
                            .with("max", Arg::Length(r.max_mm))
                            .with("builtin_min", Arg::Length(k.min_mm))
                            .with("builtin_max", Arg::Length(k.max_mm)),
                    })
                })
                .collect(),
            warnings: s.warnings(),
        }
    }

    // ---- commands ----

    /// Makes a profile named `name` with the built-in figures and opens it for editing. False, with the reason said, where refused.
    pub fn create(&self, name: &str) -> bool {
        match self.store.create(name) {
            Ok(id) => {
                let kept = self.store.get(&id).map(|p| p.name).unwrap_or_default();
                self.editing.set(Some(id));
                self.removing.set(None);
                self.changed();
                self.say("standards-said-created", &kept);
                true
            }
            Err(r) => self.refuse(&r),
        }
    }

    /// Chooses the profile in force (`None`: the built-in standards).
    pub fn select(&self, id: Option<&str>) -> bool {
        match self.store.select(id) {
            Ok(()) => {
                self.changed();
                match id.and_then(|id| self.store.get(id)) {
                    Some(p) => self.say("standards-said-in-force", &p.name),
                    None => self.ports.announcer.say(&self.word_now("standards-said-builtin")),
                }
                true
            }
            Err(r) => self.refuse(&r),
        }
    }

    fn word_now(&self, key: &str) -> String {
        self.i18n.tr_now(key, &Args::new())
    }

    /// Opens profile `id` for editing.
    pub fn begin_edit(&self, id: &str) {
        if self.store.get(id).is_some() {
            self.removing.set(None);
            self.error.set(None);
            self.editing.set(Some(id.to_string()));
        }
    }

    pub fn close_edit(&self) {
        self.editing.set(None);
        self.error.set(None);
    }

    /// Reads what was typed in the form against the profile `id` holds (its widths are kept), as the figures it names:
    /// every field is read as the page's units say, and the figures must be valid. None of it is kept.
    pub fn read_form(&self, id: &str, form: &Form) -> Result<(String, Standards), Refusal> {
        let profile = self.store.get(id).ok_or(Refusal::Unknown)?;
        let units = self.units_now();
        let mut s = profile.standards;
        for (field, typed) in Field::LENGTHS.iter().zip(&form.lengths) {
            let mm = parse_length_mm(typed, units).ok_or(Refusal::NotANumber)?;
            s.set_length_mm(*field, mm);
        }
        let mut speeds = Vec::new();
        for word in form.speeds.split([',', ';', ' ']).map(str::trim).filter(|w| !w.is_empty()) {
            let n = parse_number(word).filter(|n| n.fract() == 0.0 && *n >= 0.0 && *n <= f64::from(u16::MAX)).ok_or(Refusal::NotANumber)?;
            speeds.push(n as u16);
        }
        speeds.sort_unstable();
        speeds.dedup();
        s.speed_choices = speeds;
        s.target_kmh.clear();
        for (class, typed) in StreetClass::ALL.iter().zip(&form.targets) {
            let typed = typed.trim();
            if typed.is_empty() {
                continue;
            }
            let n = parse_number(typed).filter(|n| n.fract() == 0.0 && *n >= 0.0 && *n <= f64::from(u16::MAX)).ok_or(Refusal::NotANumber)?;
            s.target_kmh.insert(class.id().to_string(), n as u16);
        }
        s.validate()?;
        let name = crate::standards::model::clean_name(&form.name)?;
        Ok((name, s))
    }

    /// Keeps what the form says, under its name, when all of it is valid. Says what was refused and keeps nothing
    /// otherwise; says what is looser than the built-in standards when it is kept.
    pub fn save(&self, id: &str, form: &Form) -> bool {
        let (name, standards) = match self.read_form(id, form) {
            Ok(read) => read,
            Err(r) => return self.refuse(&r),
        };
        let renamed = self.store.get(id).is_some_and(|p| p.name != name);
        if renamed && let Err(r) = self.store.rename(id, &name) {
            return self.refuse(&r);
        }
        match self.store.update(id, standards) {
            Ok(()) => {
                self.changed();
                self.say("standards-said-saved", &name);
                true
            }
            Err(r) => self.refuse(&r),
        }
    }

    /// Sets the widths a kind of piece may take in profile `id`, typed in the page's units.
    pub fn set_widths(&self, id: &str, kind: &str, min: &str, max: &str) -> bool {
        let units = self.units_now();
        let (Some(min_mm), Some(max_mm)) = (parse_length_mm(min, units), parse_length_mm(max, units)) else {
            return self.refuse(&Refusal::NotANumber);
        };
        let Some(profile) = self.store.get(id) else { return self.refuse(&Refusal::Unknown) };
        let Some(k) = kind_index(kind) else { return self.refuse(&Refusal::UnknownKind(kind.to_string())) };
        let mut s = profile.standards;
        s.kind_widths.insert(KINDS[k].id.to_string(), WidthRange { min_mm, max_mm });
        match self.store.update(id, s) {
            Ok(()) => {
                self.changed();
                self.ports.announcer.say(&say_now(&self.i18n, units, &Said::new("standards-said-widths").with("kind", Arg::KindLower(KINDS[k].id))));
                true
            }
            Err(r) => self.refuse(&r),
        }
    }

    /// Gives a kind of piece its built-in widths again in profile `id`.
    pub fn clear_widths(&self, id: &str, kind: &str) -> bool {
        let Some(profile) = self.store.get(id) else { return self.refuse(&Refusal::Unknown) };
        let Some(k) = kind_index(kind) else { return self.refuse(&Refusal::UnknownKind(kind.to_string())) };
        let mut s = profile.standards;
        s.kind_widths.remove(KINDS[k].id);
        match self.store.update(id, s) {
            Ok(()) => {
                self.changed();
                self.ports.announcer.say(&say_now(
                    &self.i18n,
                    self.units_now(),
                    &Said::new("standards-said-widths-reset").with("kind", Arg::KindLower(KINDS[k].id)),
                ));
                true
            }
            Err(r) => self.refuse(&r),
        }
    }

    /// Asks before removing profile `id`.
    pub fn ask_remove(&self, id: &str) {
        if self.store.get(id).is_some() {
            self.removing.set(Some(id.to_string()));
        }
    }

    pub fn cancel_remove(&self) {
        self.removing.set(None);
    }

    /// Removes profile `id`; the built-in standards are in force again if it was.
    pub fn remove(&self, id: &str) -> bool {
        let name = self.store.get(id).map(|p| p.name).unwrap_or_default();
        match self.store.remove(id) {
            Ok(()) => {
                self.removing.set(None);
                if self.editing.get_untracked().as_deref() == Some(id) {
                    self.editing.set(None);
                }
                if self.exported.get_untracked().as_ref().is_some_and(|(n, _)| *n == name) {
                    self.exported.set(None);
                }
                self.changed();
                self.say("standards-said-removed", &name);
                true
            }
            Err(r) => self.refuse(&r),
        }
    }

    /// Shows the file of profile `id`, to copy or download.
    pub fn export(&self, id: &str) -> bool {
        match (self.store.get(id), self.store.export(id)) {
            (Some(p), Some(text)) => {
                self.exported.set(Some((p.name, text)));
                self.error.set(None);
                true
            }
            _ => self.refuse(&Refusal::Unknown),
        }
    }

    pub fn close_export(&self) {
        self.exported.set(None);
    }

    /// Adds the profile an exported file holds, when it is valid. It is not put in force.
    pub fn import(&self, text: &str) -> bool {
        match self.store.import(text) {
            Ok(id) => {
                let kept = self.store.get(&id).map(|p| p.name).unwrap_or_default();
                self.changed();
                self.say("standards-said-imported", &kept);
                true
            }
            Err(r) => self.refuse(&r),
        }
    }

    /// The kinds of piece a profile can set widths for, by catalogue id.
    pub fn kinds(&self) -> Vec<&'static str> {
        KINDS.iter().map(|k| k.id).collect()
    }

    /// The range of speeds a target or a choice may take, for a hint.
    pub fn speed_range(&self) -> (u16, u16) {
        SPEED_RANGE_KMH
    }
}
