use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Ensure we have a clean output directory
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Use the most conservative protobuf compilation settings
    // to minimize file descriptor usage
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir(&out_dir)
        // Disable features that create additional temporary files
        .compile_well_known_types(false)
        .disable_comments(&["."])
        .extern_path(".google.protobuf.Timestamp", "::prost_types::Timestamp")
        // Add support for proto3 optional fields
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile_protos(&["proto/distributed_api.proto"], &["proto/"])?;

    // Tell cargo to rerun this build script if the proto file changes
    println!("cargo:rerun-if-changed=proto/distributed_api.proto");

    Ok(())
}
