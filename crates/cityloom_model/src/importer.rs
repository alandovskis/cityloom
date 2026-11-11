/// The version of what the importer makes of a tile. Raise it when the importer's output changes for the same data,
/// so that the areas kept from a tile are read again. (An area read from Overpass keeps no checksum and is not
/// read again.)
///
/// 2: a bike track or a sidewalk mapped as a way of its own (`cycleway:both=separate`, `sidewalk:both=separate`) is a
/// lane beside the road.
///
/// 3: a road with no name is left out.
///
/// 4: a buffer between lanes is planting only when its type says planter or grass; any other (a curb, paint, posts,
/// a barrier, a shoulder) is a median; a bike track that runs both ways (a separate cycleway that is not one-way, or
/// `cycleway:<side>:oneway=no`) is one two-way bike lane.
///
/// 5: a curb buffer between a bike lane or a sidewalk and a road lane is that piece's own curb (its width joins the
/// piece's, up to its maximum; what is past it stays a buffer); any other buffer that is not planted (a curb, paint,
/// posts, a barrier, a shoulder) is the neutral `buffer`, not a median.
///
/// 6: the bike tracks mapped as ways of their own are in the tile and the Overpass answer (`highway=cycleway`, and
/// `path` or `footway` designated for bicycles); a track beside a road that is not one-way makes that road's lane
/// two-way, and the tracks themselves are not streets.
///
/// 7: a street carries the speed limit of its road (`maxspeed`).
///
/// 8: a bus window of a lane that is parking the rest of the time keeps the way the lane runs (it was two-way).
///
/// 9: the network is kept in its own shape (junctions with a provenance and a position, lanes with a direction and
/// shape, corridors), which an earlier network cannot be read as.
///
/// 10: the importer runs once over the whole area at pack time (`citymodel_builder`), and the editor cuts a place out
/// of the tile.
///
/// The e2e test `world.spec.ts` names the version in what it expects to be kept: raise it there too.
pub const IMPORTER_VERSION: u32 = 10;

/// A tile's checksum as an area made from it is kept: with the version of the importer that read it.
pub fn with_importer(checksum: &str) -> String {
    format!("{checksum}+i{IMPORTER_VERSION}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_checksum_is_kept_with_the_importer_version() {
        assert_eq!(with_importer("ab12"), "ab12+i10");
        assert_eq!(IMPORTER_VERSION, 10);
    }
}
