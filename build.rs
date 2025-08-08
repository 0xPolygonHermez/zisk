use std::env;
use std::fs;

fn main() {
    // Read the contents of the Cargo.toml file
    let toml_content = fs::read_to_string("Cargo.toml").expect("Failed to read Cargo.toml");

    // Parse the TOML content into a Value structure
    let value: toml::Value = toml::de::from_str(&toml_content).expect("Failed to parse Cargo.toml");

    // Extract the value of the custom field "gha_zisk_setup" from Cargo.toml
    if let Some(setup_file) = value["package"]["metadata"]["gha_zisk_setup"].as_str() {
        // Ensure the build script reruns if Cargo.toml changes
        println!("cargo:rerun-if-changed=Cargo.toml");
        // Set the environment variable GHA_ZISK_SETUP with the extracted value
        println!("cargo:env=GHA_ZISK_SETUP={}", setup_file);
    }
}
