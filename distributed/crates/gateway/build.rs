use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir(&out_dir)
        .compile_well_known_types(false)
        .disable_comments(["."])
        .extern_path(".google.protobuf.Timestamp", "::prost_types::Timestamp")
        .extern_path(".google.protobuf.Duration", "::prost_types::Duration")
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile_protos(&["proto/zisk_gateway_api.proto"], &["proto/"])?;

    println!("cargo:rerun-if-changed=proto/zisk_gateway_api.proto");

    Ok(())
}
