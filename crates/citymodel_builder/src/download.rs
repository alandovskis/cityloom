//! A resumable download, the port of `download` and `open_from` in the former Python `prepare` and of the jar download in
//! the former Python `basemap_tiles`.
//!
//! The file is written to `<target>.part` and renamed only once whole, so an interrupted download never leaves a
//! file that looks complete. A `.part` left by an earlier attempt is continued (`Range`) when the server answers 206
//! and started again when it answers 200.

use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use crate::progress::StepError;

const CHUNK: usize = 1 << 20;
/// Redirects followed before giving up (urllib's limit).
const MAX_REDIRECTS: usize = 10;

/// An answer, its body not read yet.
pub struct Fetched {
    pub status: u16,
    pub content_length: Option<u64>,
    pub body: Box<dyn Read + Send>,
    /// The last redirect that named a release (`.../releases/download/...`).
    pub release: Option<String>,
}

fn fail(line: impl Into<String>) -> StepError {
    StepError::new([line.into()])
}

/// `location`, sent in answer to a request for `base`, as an absolute address (RFC 3986 section 5, as urllib's
/// `urljoin` does it for the redirects it follows). A fragment is dropped: it is never sent.
fn resolve(base: &str, location: &str) -> String {
    let location = location.split('#').next().unwrap_or("");
    let base = base.split('#').next().unwrap_or("");
    let scheme_end = base.find("://").map_or(0, |i| i + 3);
    let has_scheme = location.find("://").is_some_and(|i| i > 0 && location[..i].bytes().all(|b| b.is_ascii_alphanumeric() || b"+-.".contains(&b)));
    if has_scheme {
        return location.to_string();
    }
    let origin_end = base[scheme_end..].find('/').map_or(base.len(), |i| scheme_end + i);
    if let Some(rest) = location.strip_prefix("//") {
        return format!("{}//{rest}", &base[..scheme_end - 2]);
    }
    let without_query = base.split('?').next().unwrap_or("");
    if location.is_empty() {
        return base.to_string();
    }
    if location.starts_with('?') {
        return format!("{without_query}{location}");
    }
    let (path, query) = match location.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (location, None),
    };
    let merged = if path.starts_with('/') {
        path.to_string()
    } else {
        let directory = &without_query[origin_end..];
        let directory = &directory[..directory.rfind('/').map_or(0, |i| i + 1)];
        format!("{}{path}", if directory.is_empty() { "/" } else { directory })
    };
    let mut segments: Vec<&str> = Vec::new();
    let parts: Vec<&str> = merged.split('/').collect();
    for (n, part) in parts.iter().enumerate() {
        let last = n + 1 == parts.len();
        match *part {
            "." | ".." => {
                if *part == ".." && segments.len() > 1 {
                    segments.pop();
                }
                if last {
                    segments.push("");
                }
            }
            _ => segments.push(part),
        }
    }
    let path = segments.join("/");
    format!("{}{path}{}", &base[..origin_end], query.map_or(String::new(), |q| format!("?{q}")))
}

/// A GET for `url`, asking from byte `from` on when `from > 0`. Redirects are followed by hand (up to 10; a relative
/// `Location` is resolved against the address that sent it, and `Range` goes with every request), and the last one
/// that names a release is remembered. Any status is an answer, not an error; the body is not read.
pub fn get(url: &str, from: u64) -> Result<Fetched, StepError> {
    // identity: Range and Content-Length must mean bytes on disk, so the server is asked not to compress.
    let config = ureq::Agent::config_builder().max_redirects(0).http_status_as_error(false).accept_encoding("identity").build();
    let agent: ureq::Agent = config.into();
    let (mut at, mut release) = (url.to_string(), None);
    for _ in 0..=MAX_REDIRECTS {
        let mut request = agent.get(&at);
        if from > 0 {
            request = request.header("Range", format!("bytes={from}-"));
        }
        let response = request.call().map_err(|e| fail(format!("{at}: {e}")))?;
        let status = response.status().as_u16();
        if matches!(status, 301 | 302 | 303 | 307 | 308) {
            let location =
                response.headers().get("location").and_then(|v| v.to_str().ok()).ok_or_else(|| fail(format!("{at}: a redirect with no Location")))?;
            at = resolve(&at, location);
            if at.contains("/releases/download/") {
                release = Some(at.clone());
            }
            continue;
        }
        let content_length = response.headers().get("content-length").and_then(|v| v.to_str().ok()).and_then(|v| v.trim().parse().ok());
        return Ok(Fetched { status, content_length, body: Box::new(response.into_body().into_reader()), release });
    }
    Err(fail(format!("{url}: more than {MAX_REDIRECTS} redirects")))
}

fn part_of(target: &Path) -> PathBuf {
    let mut name = target.file_name().unwrap_or_default().to_os_string();
    name.push(".part");
    target.with_file_name(name)
}

/// Writes `fetched`'s body to `out`, telling `say(done, total)` after every chunk; returns the bytes `out` then has.
/// A body that ends before its `Content-Length` says is not an error here: the caller compares the count.
fn copy_with_progress(
    url: &str,
    fetched: &mut Fetched,
    out: &mut impl Write,
    mut done: u64,
    total: Option<u64>,
    say: &mut dyn FnMut(u64, Option<u64>),
) -> Result<u64, StepError> {
    let mut chunk = vec![0u8; CHUNK];
    loop {
        let read = match fetched.body.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof && total.is_some() => break,
            Err(e) => return Err(fail(format!("{url}: {e}"))),
        };
        out.write_all(&chunk[..read])?;
        done += read as u64;
        say(done, total);
    }
    out.flush()?;
    Ok(done)
}

fn fetch_into(url: &str, target: &Path, part: &Path, resume: bool, say: &mut dyn FnMut(u64, Option<u64>)) -> Result<Option<String>, StepError> {
    let have = if resume { fs::metadata(part).map_or(0, |m| m.len()) } else { 0 };
    let mut fetched = get(url, have)?;
    if fetched.status == 416 && have > 0 {
        // asked from beyond the end: the earlier attempt had the file whole
        fs::rename(part, target)?;
        return Ok(fetched.release);
    }
    if !(200..300).contains(&fetched.status) {
        return Err(fail(format!("{url}: HTTP {}", fetched.status)));
    }
    let resumed = fetched.status == 206;
    let have = if resumed { have } else { 0 };
    let total = fetched.content_length.map(|length| length + have);
    let mut file = if resumed { OpenOptions::new().append(true).open(part) } else { OpenOptions::new().write(true).create(true).truncate(true).open(part) }
        .map_err(|e| fail(format!("{}: {e}", part.display())))?;
    let done = copy_with_progress(url, &mut fetched, &mut file, have, total, say)?;
    drop(file);
    if let Some(total) = total
        && done != total
    {
        return Err(fail(format!("{url}: got {done} bytes of {total}; run again to continue")));
    }
    fs::rename(part, target)?;
    Ok(fetched.release)
}

/// Downloads `url` to `target`, renamed from `<target>.part` only once whole. With `resume`, a `.part` left by an
/// earlier attempt is continued when the server answers 206 and started again on 200, a 416 with bytes in hand means
/// that attempt had the file whole, and a failure keeps the part for the next run. Without `resume` the download
/// starts from nothing and the part is removed on any failure. Returns the last release address the redirects named.
pub fn download(url: &str, target: &Path, resume: bool, say: &mut dyn FnMut(u64, Option<u64>)) -> Result<Option<String>, StepError> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let part = part_of(target);
    let result = fetch_into(url, target, &part, resume, say);
    if !resume {
        let _ = fs::remove_file(&part);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::resolve;

    #[test]
    fn relative_addresses_are_resolved_as_urljoin_does() {
        let base = "http://h:1/a/b/c?q=1";
        for (location, expected) in [
            ("http://other/x", "http://other/x"),
            ("//other/x", "http://other/x"),
            ("/x?y=2", "http://h:1/x?y=2"),
            ("d", "http://h:1/a/b/d"),
            ("./d/e", "http://h:1/a/b/d/e"),
            ("../d", "http://h:1/a/d"),
            ("../../../../d", "http://h:1/d"),
            ("?z=3", "http://h:1/a/b/c?z=3"),
            ("d#frag", "http://h:1/a/b/d"),
            ("", "http://h:1/a/b/c?q=1"),
            ("..", "http://h:1/a/"),
        ] {
            assert_eq!(resolve(base, location), expected, "{location}");
        }
        assert_eq!(resolve("http://h", "x"), "http://h/x");
        assert_eq!(resolve("https://h/a", "/b"), "https://h/b");
    }
}
