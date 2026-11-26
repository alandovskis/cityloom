//! A city's config, `cities/<id>.toml`: the one place a city is named.

use std::path::{Path, PathBuf};

use crate::tiles::{CityGrid, Margin};

/// A city as its file gives it.
#[derive(Debug, Clone, PartialEq)]
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
        todo!()
    }

    /// Where the extract is kept.
    pub fn extract(&self, _root: &Path) -> PathBuf {
        todo!()
    }

    /// The folder of osmium's intermediate files.
    pub fn work(&self, _root: &Path) -> PathBuf {
        todo!()
    }

    /// Where the archive is written.
    pub fn archive(&self, _root: &Path) -> PathBuf {
        todo!()
    }
}

/// The ids of the cities that have a file in `root/cities`, sorted.
pub fn known(_root: &Path) -> Vec<String> {
    todo!()
}

/// The city `id` of `root/cities`, checked.
pub fn load(_root: &Path, _id: &str) -> Result<City, String> {
    todo!()
}

/// The city the command line names, or why there is none.
pub fn choose(_root: &Path, _id: Option<&str>) -> Result<City, String> {
    todo!()
}
