//! `StreetSource` — the import port (BR7.1). Declared, not implemented,
//! here.

use crate::graph::StreetNetworkGraph;

/// The shape of "import a street network for an area" (BR7.1). This Unit
/// declares this port; it depends on no concrete import mechanism and
/// implements nothing of it. The concrete implementor
/// (`StreetImportAdapter`, `u4-street-import`) is supplied by
/// `CompositionRoot`, never named here — enforced by Cargo's own
/// dependency graph, since this crate has no dependency on the
/// osm2streets crate or the adapter crate at all.
pub trait StreetSource {
    /// The typed failure reason a concrete `StreetSource` returns when
    /// an import cannot be completed. Never an osm2streets-native error
    /// type — the implementor maps that at its own boundary.
    type Error;

    /// Imports the street network for the given area, returning a fully
    /// constructed [`StreetNetworkGraph`] whose `Street`s and `Lane`s
    /// already carry their `Provenance` (BR1.1), identity (BR3.1, BR2.1)
    /// and carriageway width (BR4.1).
    fn import_area(&self, area_id: &str) -> Result<StreetNetworkGraph, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// A fake `StreetSource`, proving the trait is usable end to end with
    /// zero osm2streets-related code anywhere in this crate's dependency
    /// tree (BR7.1) — this crate's `Cargo.toml` declares no dependency
    /// beyond `std` (SD-2), and this fake compiles and runs against that
    /// same crate.
    struct FakeStreetSource;

    #[derive(Debug, PartialEq)]
    struct FakeImportError(String);

    impl StreetSource for FakeStreetSource {
        type Error = FakeImportError;

        fn import_area(&self, area_id: &str) -> Result<StreetNetworkGraph, Self::Error> {
            if area_id.is_empty() {
                return Err(FakeImportError("empty area id".to_string()));
            }
            Ok(StreetNetworkGraph::new(
                area_id,
                vec![],
                vec![],
                HashMap::new(),
            ))
        }
    }

    #[test]
    fn a_street_source_implementor_returns_a_graph_for_a_valid_area() {
        let source = FakeStreetSource;

        let graph = source.import_area("area-1").expect("import should succeed");

        assert_eq!(graph.graph_id(), "area-1");
    }

    #[test]
    fn a_street_source_implementor_returns_a_typed_error() {
        let source = FakeStreetSource;

        let result = source.import_area("");

        assert_eq!(result, Err(FakeImportError("empty area id".to_string())));
    }

    #[test]
    fn a_street_source_is_usable_through_a_trait_object() {
        let source: Box<dyn StreetSource<Error = FakeImportError>> = Box::new(FakeStreetSource);

        let graph = source.import_area("area-2").expect("import should succeed");

        assert_eq!(graph.graph_id(), "area-2");
    }
}
