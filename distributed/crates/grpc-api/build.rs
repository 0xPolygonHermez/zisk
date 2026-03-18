use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Pass 1 — compile the shared types (zisk.common.v1).
    // Must be a separate pass so prost generates the types file before the
    // other protos reference it via extern_path in pass 2.
    tonic_prost_build::configure()
        .build_server(false)
        .build_client(false)
        .out_dir(&out_dir)
        .compile_well_known_types(false)
        .disable_comments(["."])
        .extern_path(".google.protobuf.Timestamp", "::prost_types::Timestamp")
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile_protos(&["proto/zisk_common.proto"], &["proto/"])?;

    // Pass 2 — compile the service protos, treating zisk.common.v1 as external
    // so generated code references crate::common_proto instead of navigating
    // via relative super:: paths (which break when wrapped in a Rust module).
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir(&out_dir)
        .compile_well_known_types(false)
        .disable_comments(["."])
        .extern_path(".google.protobuf.Timestamp", "::prost_types::Timestamp")
        .extern_path(".zisk.common.v1", "crate::common_proto")
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile_protos(
            &["proto/zisk_coordinator_api.proto", "proto/zisk_cluster_api.proto"],
            &["proto/"],
        )?;

    println!("cargo:rerun-if-changed=proto/zisk_common.proto");
    println!("cargo:rerun-if-changed=proto/zisk_coordinator_api.proto");
    println!("cargo:rerun-if-changed=proto/zisk_cluster_api.proto");

    Ok(())
}
