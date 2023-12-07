# Generate RUST files from proto files

The following command will generate the RUST files from the proto files:

```bash
cargo build
```

The generated files will be located in the `target/debug/build/proto-{hash}/out/pilout.rs` directory.

Copy the file pilout.rs to proto/src/pilout.rs each time you want to update the RUST proto files.

More info:
[Webpage](https://www.swiftdiaries.com/rust/prost/)
