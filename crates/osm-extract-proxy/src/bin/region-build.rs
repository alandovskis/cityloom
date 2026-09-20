//! The region-build tool (W1): reads a `regions.toml` (or builds the tiny
//! `--synthetic` fixture), verifies, filters, slices and publishes a
//! manifest and packed store (BR8.x). Exit code reflects the outcome;
//! stdout carries the `buildId` and the pointer's fields for the weekly
//! workflow (`cicd-pipeline.md` CP-4) to read.

use std::path::PathBuf;

use osm_extract_proxy::build::{
    BuildError, RegionConfig, RegionSource, build, now_rfc3339, synthetic_region_bytes,
};
use osm_extract_proxy::grid::GridSpec;

const USER_AGENT: &str = concat!("cityloom-region-build/", env!("CARGO_PKG_VERSION"));

/// Production `RegionSource`: downloads the `.osm.pbf` and its published
/// `.md5` over HTTPS with the project's own `User-Agent` (TS-7).
struct ReqwestSource {
    client: reqwest::blocking::Client,
}

impl ReqwestSource {
    fn new() -> ReqwestSource {
        ReqwestSource {
            client: reqwest::blocking::Client::builder()
                .user_agent(USER_AGENT)
                .build()
                .expect("a default TLS client always builds"),
        }
    }

    fn get(&self, url: &str) -> Result<Vec<u8>, ()> {
        self.client
            .get(url)
            .send()
            .and_then(|r| r.error_for_status())
            .and_then(|r| r.bytes())
            .map(|b| b.to_vec())
            .map_err(|_| ())
    }
}

impl RegionSource for ReqwestSource {
    fn fetch(&self, region: &RegionConfig) -> Result<(Vec<u8>, String), BuildError> {
        let bytes = self
            .get(&region.pbf_url)
            .map_err(|()| BuildError::FetchFailed(region.name.clone()))?;
        let md5_bytes = self
            .get(&region.md5_url)
            .map_err(|()| BuildError::FetchFailed(region.name.clone()))?;
        let md5_text = String::from_utf8_lossy(&md5_bytes).trim().to_string();
        Ok((bytes, md5_text))
    }
}

/// The `--synthetic` fixture: one small in-memory region, for `just
/// synthetic` and local development (`README.md`).
struct SyntheticSource;

impl RegionSource for SyntheticSource {
    fn fetch(&self, region: &RegionConfig) -> Result<(Vec<u8>, String), BuildError> {
        let bytes = synthetic_region_bytes();
        let md5: String = {
            use md5::Digest as _;
            md5::Md5::digest(&bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect()
        };
        let _ = region;
        Ok((bytes, md5))
    }
}

#[derive(serde::Deserialize)]
struct RegionsFile {
    regions: Vec<RegionsFileEntry>,
}

#[derive(serde::Deserialize)]
struct RegionsFileEntry {
    name: String,
    pbf_url: String,
    md5_url: String,
    bounds: String,
}

fn load_regions(path: &std::path::Path) -> Vec<RegionConfig> {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("could not read {}: {err}", path.display()));
    let parsed: RegionsFile = toml::from_str(&text)
        .unwrap_or_else(|err| panic!("could not parse {}: {err}", path.display()));
    parsed
        .regions
        .into_iter()
        .map(|r| RegionConfig {
            name: r.name,
            pbf_url: r.pbf_url,
            md5_url: r.md5_url,
            bounds: r.bounds,
        })
        .collect()
}

struct Args {
    config: Option<PathBuf>,
    out: PathBuf,
    synthetic: bool,
}

fn parse_args() -> Args {
    let mut args = Args {
        config: None,
        out: PathBuf::from("target/data"),
        synthetic: false,
    };
    let raw: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < raw.len() {
        match raw[i].as_str() {
            "--config" => {
                args.config = raw.get(i + 1).map(PathBuf::from);
                i += 2;
            }
            "--out" => {
                if let Some(v) = raw.get(i + 1) {
                    args.out = PathBuf::from(v);
                }
                i += 2;
            }
            "--synthetic" => {
                args.synthetic = true;
                i += 1;
            }
            _ => i += 1,
        }
    }
    args
}

fn main() {
    let args = parse_args();
    let grid = GridSpec::default_cells();
    let built_at = now_rfc3339();

    let result = if args.synthetic {
        let regions = vec![RegionConfig {
            name: "synthetic".to_string(),
            pbf_url: String::new(),
            md5_url: String::new(),
            bounds: "-79.64000,43.64000,-79.62000,43.66000".to_string(),
        }];
        build(&regions, &SyntheticSource, grid, &args.out, &built_at)
    } else {
        let config_path = args
            .config
            .unwrap_or_else(|| panic!("--config <path> is required unless --synthetic is given"));
        let regions = load_regions(&config_path);
        build(&regions, &ReqwestSource::new(), grid, &args.out, &built_at)
    };

    match result {
        Ok(output) => {
            println!("buildId={}", output.build_id.as_str());
            println!("cellCount={}", output.cell_count);
            println!("builtAt={built_at}");
        }
        Err(err) => {
            eprintln!("region-build failed: {err}");
            std::process::exit(1);
        }
    }
}
