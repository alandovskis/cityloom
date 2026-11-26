//! A city's config, `cities/<id>.toml`: the one place a city is named. It gives the extract's address, the grid and
//! the margin; the files a city's run reads and writes follow from its id and its extract's name, so cities do not
//! overwrite each other's. Every check is made before anything is downloaded or written.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::archive::is_area;
use crate::tiles::{self, CityGrid, Margin};

/// The folder of the cities' configs, under the repository's root.
const CITIES: &str = "cities";
/// A config's extension.
const EXTENSION: &str = "toml";
/// The end of an extract's file name.
const PBF: &str = ".osm.pbf";

/// A city as its file gives it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct City {
    /// The file's name, and the area name of its archive.
    pub id: String,
    pub extract_url: String,
    pub grid: CityGrid,
    pub margin: Margin,
}

impl City {
    /// The extract's file name: the last segment of its URL.
    pub fn extract_name(&self) -> &str {
        last_segment(&self.extract_url)
    }

    /// Where the extract is kept.
    pub fn extract(&self, root: &Path) -> PathBuf {
        root.join("data").join(self.extract_name())
    }

    /// The folder of osmium's intermediate files.
    pub fn work(&self, root: &Path) -> PathBuf {
        root.join("data").join("metro-work").join(&self.id)
    }

    /// Where the archive is written.
    pub fn archive(&self, root: &Path) -> PathBuf {
        root.join("data").join(format!("{}.citymodel", self.id))
    }
}

fn last_segment(url: &str) -> &str {
    url.rsplit('/').next().unwrap_or_default()
}

fn config(root: &Path, id: &str) -> PathBuf {
    root.join(CITIES).join(format!("{id}.{EXTENSION}"))
}

/// The ids of the cities that have a file in `root/cities`, sorted.
pub fn known(root: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(root.join(CITIES)) else { return Vec::new() };
    let mut ids: Vec<String> = entries
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.is_file() && path.extension().is_some_and(|e| e == EXTENSION))
        .filter_map(|path| path.file_stem().and_then(std::ffi::OsStr::to_str).map(str::to_string))
        .collect();
    ids.sort();
    ids
}

/// The known cities, as an error message ends.
pub(crate) fn listed(root: &Path) -> String {
    let ids = known(root);
    if ids.is_empty() { format!("{CITIES}/ holds no city") } else { format!("known cities: {}", ids.join(", ")) }
}

/// Why `url` cannot be an extract's address: `http://` or `https://`, a host, a path, and a last segment that is a
/// plain file name ending in `.osm.pbf`.
fn url_fault(url: &str) -> Option<&'static str> {
    let Some(rest) = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://")) else {
        return Some("must start with http:// or https://");
    };
    let Some((host, _)) = rest.split_once('/') else { return Some("must have a host and a path") };
    if host.is_empty() {
        return Some("must have a host");
    }
    if rest.contains(['?', '#']) {
        return Some("must have no query or fragment");
    }
    let name = last_segment(url);
    let plain = name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'-');
    let starts = name.bytes().next().is_some_and(|b| b.is_ascii_alphanumeric());
    if !plain || !starts || name.len() <= PBF.len() || !name.ends_with(PBF) {
        return Some(
            "must end in a plain file name ending in .osm.pbf: ASCII letters, digits, '.', '_' and '-', starting with a letter or digit, with no query or fragment",
        );
    }
    None
}

/// The city in `path`, as parsed, or the parser's message.
fn parse(path: &Path) -> Result<City, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("cannot be read: {e}"))?;
    toml::from_str(&text).map_err(|e| e.to_string().trim_end().to_string())
}

/// The city `id` of `root/cities`, checked.
pub fn load(root: &Path, id: &str) -> Result<City, String> {
    let ids = known(root);
    // compared with the files' names, never joined into a path before
    if !ids.iter().any(|known| known == id) {
        return Err(format!("no city \"{id}\" in {CITIES}/ ({})", listed(root)));
    }
    let path = config(root, id);
    let said = |field: &str, reason: &str| format!("{}: {field}: {reason}", path.display());
    let city = parse(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !is_area(&city.id) {
        return Err(said("id", "must be 1 to 64 lower-case letters, digits and hyphens, as an archive's area name"));
    }
    if city.id != id {
        return Err(said("id", &format!("is \"{}\", and must be the file's name, \"{id}\"", city.id)));
    }
    if let Some(fault) = url_fault(&city.extract_url) {
        return Err(said("extract_url", fault));
    }
    if let Err(e) = tiles::check(&city.grid, &city.margin) {
        return Err(said(e.field, &e.reason));
    }
    // cities share an extract only by giving the same address; one name for two would have one overwrite the other
    for other in ids.iter().filter(|other| *other != id) {
        let other_path = config(root, other);
        // a file that cannot be read as a city fails when it is prepared itself
        let Ok(them) = parse(&other_path) else { continue };
        if them.extract_url != city.extract_url && them.extract_name().eq_ignore_ascii_case(city.extract_name()) {
            let reason = format!(
                "gives the extract name {}, as {} does from another address ({}); one would overwrite the other in data/",
                city.extract_name(),
                other_path.display(),
                them.extract_url
            );
            return Err(said("extract_url", &reason));
        }
    }
    Ok(city)
}

/// The city the command line names, or why there is none.
pub fn choose(root: &Path, id: Option<&str>) -> Result<City, String> {
    match id {
        Some(id) => load(root, id),
        None => Err(format!("which city? ({})", listed(root))),
    }
}
