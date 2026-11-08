//! `osm2network in.osm out.json`: reads OSM XML or PBF and writes the street network as JSON.
//! This is how the default area that ships with the app is made (`just default-area`).

use std::{env, fs, process::ExitCode};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let [_, input, output] = &args[..] else {
        eprintln!("usage: osm2network <in.osm|in.pbf> <out.json>");
        return ExitCode::from(2);
    };
    let result = fs::read(input).map_err(|e| format!("{input}: {e}")).and_then(|osm| osm_import::import(&osm)).and_then(|network| {
        serde_json::to_string(&network).map_err(|e| e.to_string()).and_then(|json| fs::write(output, json).map_err(|e| format!("{output}: {e}")))
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
