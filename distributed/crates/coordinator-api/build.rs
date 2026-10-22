const EXT_DEPRECATED: &str =
    "#[deprecated(since = \"1.4.0-alpha\", note = \"job metadata travels on \
    `JobRequestMessage::metadata`; removed in the first non-alpha release\")]";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir(&out_dir)
        .compile_well_known_types(false)
        .disable_comments(["."])
        .extern_path(".google.protobuf.Timestamp", "::prost_types::Timestamp")
        .extern_path(".google.protobuf.Duration", "::prost_types::Duration")
        // prost derives neither Eq nor Hash for a message with a map, and JobRequestMessage
        // derived both before it had `metadata`: generate that map as a BTreeMap, which
        // implements them, and derive them by hand.
        .btree_map(".zisk.coordinator.v1.JobRequestMessage.metadata")
        .message_attribute(".zisk.coordinator.v1.JobRequestMessage", "#[derive(Eq, Hash)]")
        // prost turns no `option deprecated` on a message into `#[deprecated]`; the extended
        // API's messages need it for their users to see the deprecation.
        .message_attribute(".zisk.coordinator.v1.JobRequestExtMessage", EXT_DEPRECATED)
        .message_attribute(".zisk.coordinator.v1.JobKindExt", EXT_DEPRECATED)
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile_protos(
            &["proto/zisk_coordinator_api.proto", "proto/zisk_coordinator_api_ext.proto"],
            &["proto/"],
        )?;

    println!("cargo:rerun-if-changed=proto/zisk_coordinator_api.proto");
    println!("cargo:rerun-if-changed=proto/zisk_coordinator_api_ext.proto");
    Ok(())
}
