fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::compile_protos("proto/consensus_api.proto")?;

    // Tell cargo to rerun this build script if the proto file changes
    println!("cargo:rerun-if-changed=proto/consensus_api.proto");

    Ok(())
}
