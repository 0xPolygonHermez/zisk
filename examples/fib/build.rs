extern crate prost_build;

use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::compile_protos(&["src/bin/input.proto"], &["src/"])?;
    prost_build::compile_protos(&["src/bin/output.proto"], &["src/"])?;

    // Get the full path to the script.ld file
    // The CARGO_MANIFEST_DIR environment variable points to the directory containing the crate's Cargo.toml
    let script_path = env::var("CARGO_MANIFEST_DIR").unwrap();
    let script_path = format!("{}/src/script.ld", script_path);

    // Set the linker argument to use the script.ld file
    // This tells the Rust compiler to pass the -T script.ld argument to the linker
    println!("cargo:rustc-link-arg=-T{}", script_path);

    Ok(())
}
