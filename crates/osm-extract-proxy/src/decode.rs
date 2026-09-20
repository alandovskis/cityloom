//! `decode` — reading a clip's PBF bytes back with the `osmpbf` crate
//! (TS-2): used to characterise the project encoder (round-trip, step 6.1)
//! and, at request time, to read a cut's own per-cell store payloads back
//! into memory for reassembly (`cut.rs`).

use std::io::Cursor;

use osmpbf::Element;
use thiserror::Error;

use crate::osm::{Clip, Node, Way};

/// PBF bytes could not be parsed by `osmpbf`.
#[derive(Debug, Error)]
#[error("the extract bytes could not be read as PBF")]
pub struct DecodeError;

/// Decode a complete `.osm.pbf` byte stream (as `encode::encode` produces)
/// back into a [`Clip`]. Relations are not part of this Unit's data model
/// and are ignored if present.
pub fn decode(bytes: &[u8]) -> Result<Clip, DecodeError> {
    let reader = osmpbf::ElementReader::new(Cursor::new(bytes.to_vec()));
    let mut clip = Clip::default();
    reader
        .for_each(|element| match element {
            Element::Node(node) => clip.nodes.push(Node {
                id: node.id(),
                lat: i64::from(node.decimicro_lat()),
                lon: i64::from(node.decimicro_lon()),
                tags: node
                    .tags()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
            }),
            Element::DenseNode(node) => clip.nodes.push(Node {
                id: node.id(),
                lat: i64::from(node.decimicro_lat()),
                lon: i64::from(node.decimicro_lon()),
                tags: node
                    .tags()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
            }),
            Element::Way(way) => clip.ways.push(Way {
                id: way.id(),
                refs: way.refs().collect(),
                tags: way
                    .tags()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
            }),
            Element::Relation(_) => {}
        })
        .map_err(|_| DecodeError)?;
    clip.normalise();
    Ok(clip)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::encode;
    use crate::osm::Way as OsmWay;

    fn sample_clip() -> Clip {
        let mut clip = Clip {
            nodes: vec![
                Node {
                    id: 1,
                    lat: 436_500_000,
                    lon: -796_300_000,
                    tags: Vec::new(),
                },
                Node {
                    id: 2,
                    lat: 436_600_000,
                    lon: -796_200_000,
                    tags: vec![("crossing".to_string(), "traffic_signals".to_string())],
                },
                Node {
                    id: 3,
                    lat: 436_550_000,
                    lon: -796_250_000,
                    tags: Vec::new(),
                },
            ],
            ways: vec![OsmWay {
                id: 100,
                refs: vec![1, 3, 2],
                tags: vec![
                    ("highway".to_string(), "residential".to_string()),
                    ("name".to_string(), "Main Street".to_string()),
                ],
            }],
        };
        clip.normalise();
        clip
    }

    // TS-2/TS-3 — a clip encoded by the project encoder decodes with
    // osmpbf to the same elements.
    #[test]
    fn a_clip_round_trips_through_osmpbf() {
        let clip = sample_clip();
        let bytes = encode(&clip);
        let decoded = decode(&bytes).expect("valid PBF");
        assert_eq!(decoded, clip);
    }

    #[test]
    fn an_empty_clip_round_trips() {
        let clip = Clip::default();
        let bytes = encode(&clip);
        let decoded = decode(&bytes).expect("valid PBF");
        assert_eq!(decoded, clip);
    }

    #[test]
    fn a_way_with_no_tags_round_trips() {
        let mut clip = Clip {
            nodes: vec![
                Node {
                    id: 5,
                    lat: 1_000,
                    lon: 2_000,
                    tags: Vec::new(),
                },
                Node {
                    id: 6,
                    lat: 1_100,
                    lon: 2_100,
                    tags: Vec::new(),
                },
            ],
            ways: vec![OsmWay {
                id: 200,
                refs: vec![5, 6],
                tags: Vec::new(),
            }],
        };
        clip.normalise();
        let decoded = decode(&encode(&clip)).unwrap();
        assert_eq!(decoded, clip);
    }

    #[test]
    fn garbage_bytes_are_a_typed_decode_error() {
        assert!(decode(b"not a pbf file at all, just some bytes").is_err());
    }
}
