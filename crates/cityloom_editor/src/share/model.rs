//! The share format (#221): one JSON document holding a project as it was when it was shared, kept by the server
//! behind an unguessable link and opened read-only by anyone who has it. No wording here, and no I/O of its own: a
//! project's work is read through the `Storage` port, and every refusal is `ShareError` data, said as `Said`.
//!
//! **The format** (`format_version` 1):
//!
//! ```json
//! { "kind": "cityloom-share", "format_version": 1, "app_version": "0.97.0", "created_at": "2027-01-15T08:00:00.000Z",
//!   "project": { "name": "Plateau", "area": "area=45.5230,-73.5810&name=Plateau" },
//!   "edits": { …the project's `cityloom-edits` document… },
//!   "restrictions": { …the project's `cityloom-restrictions` document (its area rules)… } }
//! ```
//!
//! `edits` and `restrictions` are the stores' own documents taken as raw JSON, as the design archive and a milestone
//! take them, so a record or a field this build does not know travels unchanged; either is left out where the project
//! has none. The project's milestones are not in it (history, not work), nor anything of the account, the sync or the
//! person's settings.
//!
//! **Versioning** is the archive's: `format_version` is an integer; a reader takes exactly the versions it knows (1)
//! and refuses a higher one as a newer CityLoom's; unknown fields are ignored. A change old readers could misread must
//! raise it; one that only adds optional fields does not. The server checks `kind` and the version too.
//!
//! **Reading** checks everything before anything is kept (`parse_and_validate`): the header, the project's name and
//! area, and the two documents by the archive's rules (`archive::model::{read_edits, read_rules}`: versions, objects,
//! keys the sync could carry).

use std::rc::Rc;

use serde_json::{Map, Value};

use crate::archive::model::{ArchiveError, read_edits, read_rules};
use crate::city::edits::EDITS_KEY;
use crate::place::area::Area;
use crate::restriction::store::RESTRICTIONS_KEY;
use crate::shared::ports::{ShareFailure, Storage};
use crate::shared::said::{Arg, Said};

/// The `kind` of every share document.
pub const MAGIC: &str = "cityloom-share";
/// The only format this build writes, and the newest it reads.
pub const FORMAT_VERSION: u32 = 1;
/// The most a share may weigh: what the server keeps of one (`cityloom_server::share::MAX_BODY_BYTES`).
pub const MAX_BYTES: usize = 1024 * 1024;
/// How long the server keeps a share (`cityloom_server::share::RETENTION_DAYS`), for the words that say so.
pub const RETENTION_DAYS: i64 = 90;
/// How many live shares an account may hold (`cityloom_server::share::MAX_LIVE_SHARES`), for the words that say so.
pub const MAX_LIVE: i64 = 50;
/// How many shares an account may make in 24 hours, deleted ones too (`cityloom_server::share::MAX_SHARES_PER_DAY`),
/// for the words that say so.
pub const MAX_PER_DAY: i64 = 100;
/// The page a share link opens.
pub const PAGE: &str = "share.html";

/// Why a share was not made, opened or deleted. Opening one keeps nothing on the device when it fails (`said_opening`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShareError {
    /// The address holds no token, or not a whole one.
    BadLink,
    NotFound,
    Expired,
    Deleted,
    /// A format, or a document in it, from a newer CityLoom than this page.
    Newer,
    /// What came back is not a whole share.
    Damaged,
    /// No answer: the network.
    Network,
    /// The server cannot help now.
    Unavailable,
    /// The browser's storage refused the copy.
    Storage,
    /// Making or deleting a share needs an account.
    SignedOut,
    /// The page shows the project read-only.
    ReadOnly,
    /// The project's work cannot be read (damaged, or a newer build's).
    Unusable,
    TooLarge,
    /// The account holds as many shares as it may.
    Full,
    /// The account made as many shares as it may in 24 hours.
    TooManyToday,
    /// Another account's share.
    NotYours,
    /// Anything else the server refused.
    Refused,
}

impl ShareError {
    /// What to tell the person.
    pub fn said(&self) -> Said {
        match self {
            ShareError::BadLink => Said::new("share-error-bad-link"),
            ShareError::NotFound => Said::new("share-error-not-found"),
            ShareError::Expired => Said::new("share-error-expired").with("days", Arg::Num(RETENTION_DAYS)),
            ShareError::Deleted => Said::new("share-error-deleted"),
            ShareError::Newer => Said::new("share-error-newer"),
            ShareError::Damaged => Said::new("share-error-damaged"),
            ShareError::Network => Said::new("share-error-network"),
            ShareError::Unavailable => Said::new("share-error-unavailable"),
            ShareError::Storage => Said::new("share-error-storage"),
            ShareError::SignedOut => Said::new("share-error-signed-out"),
            ShareError::ReadOnly => Said::new("share-error-read-only"),
            ShareError::Unusable => Said::new("share-error-unusable"),
            ShareError::TooLarge => Said::new("share-error-too-large").with("megabytes", Arg::Num((MAX_BYTES / (1024 * 1024)) as i64)),
            ShareError::Full => Said::new("share-error-full").with("count", Arg::Num(MAX_LIVE)),
            ShareError::TooManyToday => Said::new("share-error-too-many-today").with("count", Arg::Num(MAX_PER_DAY)),
            ShareError::NotYours => Said::new("share-error-not-yours"),
            ShareError::Refused => Said::new("share-error-refused"),
        }
    }
}

impl ShareError {
    /// What to tell the person whose link did not open: every such failure ends saying that nothing was kept on this
    /// device. The failures a link's maker meets too (no network, no server, a refusal) have words of their own here,
    /// since a link that could not be deleted is still kept.
    pub fn said_opening(&self) -> Said {
        match self {
            ShareError::Network => Said::new("share-open-error-network"),
            ShareError::Unavailable => Said::new("share-open-error-unavailable"),
            ShareError::Refused => Said::new("share-open-error-refused"),
            other => other.said(),
        }
    }
}

impl From<ShareFailure> for ShareError {
    fn from(f: ShareFailure) -> ShareError {
        match f {
            ShareFailure::SignedOut => ShareError::SignedOut,
            ShareFailure::NotFound => ShareError::NotFound,
            ShareFailure::Expired => ShareError::Expired,
            ShareFailure::Deleted => ShareError::Deleted,
            ShareFailure::NotYours => ShareError::NotYours,
            ShareFailure::Full => ShareError::Full,
            ShareFailure::TooManyToday => ShareError::TooManyToday,
            ShareFailure::TooLarge => ShareError::TooLarge,
            ShareFailure::Newer => ShareError::Newer,
            ShareFailure::Network => ShareError::Network,
            // The rate limit, said as the server not answering now (making or deleting one link meets it rarely).
            ShareFailure::Unavailable | ShareFailure::Busy => ShareError::Unavailable,
            ShareFailure::Refused => ShareError::Refused,
        }
    }
}

/// Whether `text` is a share link's token as the server makes them: 43 characters of base64url.
pub fn valid_token(text: &str) -> bool {
    text.len() == 43 && text.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Whether the share page whose address has the fragment `hash` (without `#`) and the query string `search` opens a
/// link (`share.html#<token>`) rather than shares a project (`share.html?project=<id>`): for a whole token, or for any
/// other fragment where no project is named (it is then said not to be a whole link). A project's page keeps its own
/// anchors (a skip link's, after a reload) and the account's sign-in fragment.
pub fn opens_a_link(hash: &str, search: &str) -> bool {
    if valid_token(hash) {
        return true;
    }
    !hash.is_empty() && !crate::shared::ports::is_account_fragment(hash) && crate::shared::project_id::named_project(search).is_none()
}

/// The link that opens the share behind `token`, on the site at `origin`. The token rides in the fragment, which the
/// browser never sends as part of an address nor puts in a `Referer`; the page sends it to this site's server alone, in
/// a request header (`ShareApi`), to fetch the share.
pub fn link(origin: &str, token: &str) -> String {
    format!("{origin}/{PAGE}#{token}")
}

/// A share that was read and found whole.
#[derive(Clone, Debug, PartialEq)]
pub struct Shared {
    pub format_version: u32,
    pub app_version: String,
    pub created_at: String,
    /// The project's name, as it was.
    pub name: String,
    /// The project's area, as its record names it (`Area::to_query`).
    pub area: String,
    edits: Option<Value>,
    restrictions: Option<Value>,
    pub streets: usize,
    pub junctions: usize,
    pub rules: usize,
}

impl Shared {
    /// The documents a copy of it keeps: each a key of a project's storage and its text.
    pub fn documents(&self) -> Vec<(&'static str, String)> {
        let mut out = Vec::new();
        if let Some(edits) = &self.edits {
            out.push((EDITS_KEY, edits.to_string()));
        }
        if let Some(restrictions) = &self.restrictions {
            out.push((RESTRICTIONS_KEY, restrictions.to_string()));
        }
        out
    }
}

/// What an archive validator's refusal is to a share: newer, or damaged.
fn part_error(e: ArchiveError) -> ShareError {
    match e {
        ArchiveError::EditsNewer(_) | ArchiveError::RulesNewer(_) | ArchiveError::FutureFormat(_) => ShareError::Newer,
        _ => ShareError::Damaged,
    }
}

/// A document of the project kept under `key`, as JSON; None where none is kept.
fn kept(storage: &Rc<dyn Storage>, key: &str) -> Result<Option<Value>, ShareError> {
    match storage.recall(key).filter(|text| !text.trim().is_empty()) {
        None => Ok(None),
        Some(text) => serde_json::from_str(&text).map(Some).map_err(|_| ShareError::Unusable),
    }
}

/// The share document of the project whose storage is `storage` (its streets, junctions and area rules as they are
/// now), named `name` in `area`. Refused where the project's work cannot be read (`Unusable`: what it holds is not
/// known) or the document would be more than the server keeps (`TooLarge`, never cut). `created_at` is a sync stamp.
pub fn compose(storage: &Rc<dyn Storage>, name: &str, area: &str, app_version: &str, created_at: &str) -> Result<String, ShareError> {
    let edits = kept(storage, EDITS_KEY)?;
    let restrictions = kept(storage, RESTRICTIONS_KEY)?;
    // What this build could not read back is not shared: the same checks as reading a share.
    read_edits(edits.as_ref()).map_err(|_| ShareError::Unusable)?;
    read_rules(restrictions.as_ref()).map_err(|_| ShareError::Unusable)?;
    let mut project = Map::new();
    project.insert("name".into(), name.into());
    project.insert("area".into(), area.into());
    let mut doc = Map::new();
    doc.insert("kind".into(), MAGIC.into());
    doc.insert("format_version".into(), FORMAT_VERSION.into());
    doc.insert("app_version".into(), app_version.into());
    doc.insert("created_at".into(), created_at.into());
    doc.insert("project".into(), Value::Object(project));
    if let Some(edits) = edits {
        doc.insert("edits".into(), edits);
    }
    if let Some(restrictions) = restrictions {
        doc.insert("restrictions".into(), restrictions);
    }
    let text = Value::Object(doc).to_string();
    if text.len() > MAX_BYTES { Err(ShareError::TooLarge) } else { Ok(text) }
}

/// Reads `text` as a share and checks it whole: nothing is kept until every part has been found sound.
pub fn parse_and_validate(text: &str) -> Result<Shared, ShareError> {
    if text.len() > MAX_BYTES {
        return Err(ShareError::Damaged);
    }
    let value: Value = serde_json::from_str(text).map_err(|_| ShareError::Damaged)?;
    let doc = value.as_object().ok_or(ShareError::Damaged)?;
    if doc.get("kind").and_then(Value::as_str) != Some(MAGIC) {
        return Err(ShareError::Damaged);
    }
    let format_version = match doc.get("format_version").and_then(Value::as_u64) {
        None | Some(0) => return Err(ShareError::Damaged),
        Some(v) if v > u64::from(FORMAT_VERSION) => return Err(ShareError::Newer),
        Some(v) => v as u32,
    };
    let text_of = |name: &str| match doc.get(name) {
        None | Some(Value::Null) => Ok(String::new()),
        Some(Value::String(s)) => Ok(s.clone()),
        Some(_) => Err(ShareError::Damaged),
    };
    let (app_version, created_at) = (text_of("app_version")?, text_of("created_at")?);
    let project = doc.get("project").and_then(Value::as_object).ok_or(ShareError::Damaged)?;
    let name = project.get("name").and_then(Value::as_str).and_then(|n| crate::project::model::clean_name(n).ok()).ok_or(ShareError::Damaged)?;
    let area = project.get("area").and_then(Value::as_str).filter(|a| Area::from_query(a).is_some()).ok_or(ShareError::Damaged)?.to_string();
    let present = |name: &str| doc.get(name).filter(|v| !v.is_null()).cloned();
    let (edits, restrictions) = (present("edits"), present("restrictions"));
    // Records made before importer 11 travel as the document holds them, and are never applied where it is opened.
    let (streets, junctions, _stale) = read_edits(edits.as_ref()).map_err(part_error)?;
    let rules = read_rules(restrictions.as_ref()).map_err(part_error)?;
    Ok(Shared {
        format_version,
        app_version,
        created_at,
        name,
        area,
        edits,
        restrictions,
        streets: streets.len(),
        junctions: junctions.len(),
        rules: rules.len(),
    })
}
