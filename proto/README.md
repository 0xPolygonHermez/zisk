# Generate RUST files from proto files

The `cargo build` command will generate the 'pilout.rs', which serves as a central management point for all data within the proto files.
The generated file can be found in the `target/debug/build/proto-{hash}/out/pilout.rs` directory and is already included in the project as a module. Alternatively, you can update the Rust proto files by copying the pilout.rs file to proto/src/pilout.rs whenever necessary.
