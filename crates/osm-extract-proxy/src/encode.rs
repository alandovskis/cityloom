//! `encode` — the project's own OSM PBF encoder (TS-3, BR5.5): deterministic
//! blocks, no timestamp, no `writingprogram`, DenseNodes for the node group
//! (BR5.4: the same clip always produces the same bytes). Pure: builds a
//! `Vec<u8>` in memory, no I/O.

use std::io::Write;

use protobuf::{Message, MessageField};

use crate::osm::Clip;
use crate::proto_gen::fileformat::{Blob, BlobHeader};
use crate::proto_gen::osmformat::{
    DenseNodes, HeaderBlock, PrimitiveBlock, PrimitiveGroup, StringTable, Way,
};

/// The OSM PBF default granularity this encoder always writes: one raw unit
/// is 100 nanodegrees (`osm.rs::RAW_GRANULARITY`).
const GRANULARITY: i32 = crate::osm::RAW_GRANULARITY as i32;

/// Encode a clip as a complete, valid `.osm.pbf` byte stream: an `OSMHeader`
/// fileblock followed by one `OSMData` fileblock. The clip is normalised
/// first (BR5.3), so encoding is deterministic in the elements it carries as
/// well as in the bytes it emits (BR5.4).
pub fn encode(clip: &Clip) -> Vec<u8> {
    let mut clip = clip.clone();
    clip.normalise();
    let mut out = Vec::new();
    write_fileblock(&mut out, "OSMHeader", &header_block_bytes());
    write_fileblock(&mut out, "OSMData", &data_block_bytes(&clip));
    out
}

fn header_block_bytes() -> Vec<u8> {
    let mut header = HeaderBlock::new();
    header.required_features.push("OsmSchema-V0.6".to_string());
    header.optional_features.push("DenseNodes".to_string());
    header
        .write_to_bytes()
        .expect("a HeaderBlock with only string fields always serialises")
}

/// A stable string table: strings keep the order they are first seen in,
/// which is what makes the whole encoding deterministic given a normalised
/// (sorted) clip (BR5.4).
struct StringInterner {
    strings: Vec<String>,
}

impl StringInterner {
    fn new() -> StringInterner {
        // Index 0 is conventionally the empty string; nothing here ever
        // resolves to it, but the convention costs nothing to keep.
        StringInterner {
            strings: vec![String::new()],
        }
    }

    fn intern(&mut self, s: &str) -> u32 {
        if let Some(pos) = self.strings.iter().position(|existing| existing == s) {
            pos as u32
        } else {
            self.strings.push(s.to_string());
            (self.strings.len() - 1) as u32
        }
    }

    fn into_table(self) -> StringTable {
        let mut table = StringTable::new();
        table.s = self.strings.into_iter().map(String::into_bytes).collect();
        table
    }
}

fn data_block_bytes(clip: &Clip) -> Vec<u8> {
    let mut strings = StringInterner::new();

    let mut dense = DenseNodes::new();
    let (mut prev_id, mut prev_lat, mut prev_lon) = (0i64, 0i64, 0i64);
    for node in &clip.nodes {
        dense.id.push(node.id - prev_id);
        prev_id = node.id;
        dense.lat.push(node.lat - prev_lat);
        prev_lat = node.lat;
        dense.lon.push(node.lon - prev_lon);
        prev_lon = node.lon;
        if node.tags.is_empty() {
            dense.keys_vals.push(0);
        } else {
            for (k, v) in &node.tags {
                dense.keys_vals.push(strings.intern(k) as i32);
                dense.keys_vals.push(strings.intern(v) as i32);
            }
            dense.keys_vals.push(0);
        }
    }

    let mut ways = Vec::with_capacity(clip.ways.len());
    for way in &clip.ways {
        let mut w = Way::new();
        w.set_id(way.id);
        for (k, v) in &way.tags {
            w.keys.push(strings.intern(k));
            w.vals.push(strings.intern(v));
        }
        let mut prev_ref = 0i64;
        for r in &way.refs {
            w.refs.push(r - prev_ref);
            prev_ref = *r;
        }
        ways.push(w);
    }

    let mut node_group = PrimitiveGroup::new();
    node_group.dense = MessageField::some(dense);

    let mut block = PrimitiveBlock::new();
    block.primitivegroup.push(node_group);
    if !ways.is_empty() {
        let mut way_group = PrimitiveGroup::new();
        way_group.ways = ways;
        block.primitivegroup.push(way_group);
    }
    block.set_granularity(GRANULARITY);
    block.stringtable = MessageField::some(strings.into_table());

    block
        .write_to_bytes()
        .expect("a PrimitiveBlock built from valid clip data always serialises")
}

/// Append one length-prefixed `BlobHeader` + `Blob` fileblock (fileformat.proto):
/// a big-endian `int32` header length, the `BlobHeader` bytes, then the
/// `Blob` bytes (zlib-compressed, per BR5.5's PBF choice).
fn write_fileblock(out: &mut Vec<u8>, block_type: &str, raw: &[u8]) {
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder
        .write_all(raw)
        .expect("writing to an in-memory zlib encoder never fails");
    let compressed = encoder
        .finish()
        .expect("finishing an in-memory zlib encoder never fails");

    let mut blob = Blob::new();
    blob.set_raw_size(raw.len() as i32);
    blob.set_zlib_data(compressed);
    let blob_bytes = blob
        .write_to_bytes()
        .expect("a Blob with only scalar and bytes fields always serialises");

    let mut header = BlobHeader::new();
    header.set_type(block_type.to_string());
    header.set_datasize(blob_bytes.len() as i32);
    let header_bytes = header
        .write_to_bytes()
        .expect("a BlobHeader with only scalar and string fields always serialises");

    out.extend_from_slice(&(header_bytes.len() as i32).to_be_bytes());
    out.extend_from_slice(&header_bytes);
    out.extend_from_slice(&blob_bytes);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::osm::{Node, Way as OsmWay};

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
            ],
            ways: vec![OsmWay {
                id: 100,
                refs: vec![1, 2],
                tags: vec![("highway".to_string(), "residential".to_string())],
            }],
        };
        clip.normalise();
        clip
    }

    // BR5.4 — the same input yields byte-identical output on two runs.
    #[test]
    fn the_same_clip_encodes_identically_on_two_runs() {
        let clip = sample_clip();
        assert_eq!(encode(&clip), encode(&clip));
    }

    // BR5.5 — the header declares OsmSchema-V0.6 and DenseNodes.
    #[test]
    fn the_header_declares_the_schema_and_dense_nodes() {
        let bytes = header_block_bytes();
        let header = HeaderBlock::parse_from_bytes(&bytes).unwrap();
        assert!(
            header
                .required_features
                .contains(&"OsmSchema-V0.6".to_string())
        );
        assert!(header.optional_features.contains(&"DenseNodes".to_string()));
    }

    // No timestamp or program-name field is written anywhere.
    #[test]
    fn no_timestamp_or_program_name_field_is_written() {
        let header = HeaderBlock::parse_from_bytes(&header_block_bytes()).unwrap();
        assert!(!header.has_writingprogram());
        assert!(!header.has_osmosis_replication_timestamp());

        let block = PrimitiveBlock::parse_from_bytes(&data_block_bytes(&sample_clip())).unwrap();
        for group in &block.primitivegroup {
            if group.dense.is_some() {
                assert!(
                    group.dense.denseinfo.is_none(),
                    "no per-node metadata (which would carry a timestamp)"
                );
            }
            for way in &group.ways {
                assert!(way.info.is_none(), "no per-way metadata");
            }
        }
    }

    #[test]
    fn the_fileblock_framing_is_two_length_prefixed_blocks() {
        let bytes = encode(&sample_clip());
        let mut cursor = 0usize;
        for expected_type in ["OSMHeader", "OSMData"] {
            let header_len =
                i32::from_be_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
            cursor += 4;
            let header = BlobHeader::parse_from_bytes(&bytes[cursor..cursor + header_len]).unwrap();
            assert_eq!(header.type_(), expected_type);
            cursor += header_len;
            cursor += header.datasize() as usize;
        }
        assert_eq!(cursor, bytes.len());
    }
}
