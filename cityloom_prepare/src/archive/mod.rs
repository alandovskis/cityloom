//! The `.cityloom` area archive: a plain tar whose first member is the binary index.

mod index;

pub use index::*;

#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    #[error("{what}: {source}")]
    Io { what: String, source: std::io::Error },
    #[error("the index is refused: {0}")]
    Index(#[from] IndexError),
    #[error("the first member of the archive is not cityloom.idx")]
    IndexNotFirst,
    #[error("the index is larger than 1048576 bytes")]
    IndexTooBig,
    #[error("the area name {0:?} is not allowed (lower-case letters, digits and hyphens, at most 64)")]
    Area(String),
    #[error("member {path:?}: {problem}")]
    Member { path: String, problem: &'static str },
    #[error("member {path:?} is listed in the index and missing from the archive")]
    Missing { path: String },
    #[error("no .cityloom file in {0}")]
    NoArchive(std::path::PathBuf),
    #[error("more than one .cityloom file in {0}: this build takes one")]
    TooMany(std::path::PathBuf),
}
