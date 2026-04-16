use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);

    // Proto is owned by the gateway-api crate (the public client contract).
    let proto_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?).join("../gateway-api/proto");

    let proto_file = proto_dir.join("zisk_gateway_api.proto");

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir(&out_dir)
        .compile_well_known_types(false)
        .disable_comments(["."])
        .extern_path(".google.protobuf.Timestamp", "::prost_types::Timestamp")
        .extern_path(".google.protobuf.Duration", "::prost_types::Duration")
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile_protos(&[&proto_file], &[&proto_dir])?;

    println!("cargo:rerun-if-changed={}", proto_file.display());

    Ok(())
}
