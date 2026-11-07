//! The words of what an export or an import did.
use crate::archive::model::{Applied, ArchiveError, Status};
use crate::shared::i18n::{Args, I18n};
use crate::shared::said::{say, say_now};
use crate::shared::units::Units;

/// What the last command did, as data: the words follow a language switch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Exported { name: String, count: usize },
    ExportEmpty,
    ExportFailed,
    Imported(Applied),
    NothingNew,
    Cancelled,
    Failed(ArchiveError),
}

/// The differences an import is waiting on the person to settle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Differences {
    pub differ: usize,
    pub new: usize,
    pub streets: usize,
    pub junctions: usize,
    pub rules: usize,
}

impl Differences {
    pub fn of(plan: &crate::archive::model::ImportPlan) -> Differences {
        use crate::sync::model::Kind;
        let mut d = Differences { differ: plan.count(Status::Conflict), new: plan.count(Status::New), ..Differences::default() };
        for e in plan.conflicts() {
            match e.kind {
                Kind::Street => d.streets += 1,
                Kind::Junction => d.junctions += 1,
                Kind::Rule | Kind::Area => d.rules += 1,
            }
        }
        d
    }
}

/// The message and arguments of an outcome that is not an error (an error is said from its `Said`).
fn parts(outcome: &Outcome) -> Option<(&'static str, Args)> {
    Some(match outcome {
        Outcome::Exported { name, count } => ("archive-exported", Args::new().str("name", name.clone()).num("count", *count as i64)),
        Outcome::ExportEmpty => ("archive-export-empty", Args::new()),
        Outcome::ExportFailed => ("archive-export-failed", Args::new()),
        Outcome::Imported(a) => (
            "archive-imported",
            Args::new().num("added", a.added as i64).num("replaced", a.replaced as i64).num("kept", a.kept as i64).num("same", a.same as i64),
        ),
        Outcome::NothingNew => ("archive-nothing-new", Args::new()),
        Outcome::Cancelled => ("archive-cancelled", Args::new()),
        Outcome::Failed(_) => return None,
    })
}

/// The outcome in words; tracked, for a view.
pub fn outcome_text(i18n: &I18n, outcome: &Outcome) -> String {
    match (outcome, parts(outcome)) {
        (_, Some((key, args))) => i18n.tr(key, &args),
        (Outcome::Failed(e), None) => say(i18n, Units::default(), &e.said()),
        (_, None) => String::new(),
    }
}

/// The same, for a command: nothing is watched.
pub fn outcome_text_now(i18n: &I18n, outcome: &Outcome) -> String {
    match (outcome, parts(outcome)) {
        (_, Some((key, args))) => i18n.tr_now(key, &args),
        (Outcome::Failed(e), None) => say_now(i18n, Units::default(), &e.said()),
        (_, None) => String::new(),
    }
}

pub fn differences_summary(i18n: &I18n, d: &Differences, watched: bool) -> String {
    let args = Args::new().num("differ", d.differ as i64).num("new", d.new as i64);
    if watched { i18n.tr("archive-confirm-summary", &args) } else { i18n.tr_now("archive-confirm-summary", &args) }
}

pub fn differences_detail(i18n: &I18n, d: &Differences) -> String {
    i18n.tr("archive-confirm-detail", &Args::new().num("streets", d.streets as i64).num("junctions", d.junctions as i64).num("rules", d.rules as i64))
}
