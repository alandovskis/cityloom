//! Generates the OSM PBF protobuf types from the vendored `.proto` files
//! (`proto/UPSTREAM`) with protobuf-codegen's pure-Rust parser — no `protoc`
//! on the build machine (tech-stack decision TS-3). The output lands in
//! `$OUT_DIR/proto_gen/` and is excluded from the coverage denominator by
//! the committed ignore pattern.

fn main() {
    println!("cargo:rerun-if-changed=proto/fileformat.proto");
    println!("cargo:rerun-if-changed=proto/osmformat.proto");
    protobuf_codegen::Codegen::new()
        .pure()
        .cargo_out_dir("proto_gen")
        .include("proto")
        .input("proto/fileformat.proto")
        .input("proto/osmformat.proto")
        .run_from_script();
}
