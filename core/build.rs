use std::path::Path;

fn main() {
    // Ensure lib-float is built before we try to include ziskfloat.elf
    // The build-dependency on lib-float will trigger its build.rs first.

    // Tell cargo to rerun this build script if the float library changes
    let float_lib_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../lib-float/c/lib/ziskfloat.elf");

    println!("cargo:rerun-if-changed={}", float_lib_path.display());

    // Tell the build scripts of the dependents (`links = "zisk_core"`) the memory sizes the
    // executors reserve, so the C, C++ and CUDA code they compile follows core/src/mem.rs
    for feature in ["input_2gb", "ram_2gb"] {
        let enabled = std::env::var_os(format!("CARGO_FEATURE_{}", feature.to_uppercase()));
        println!("cargo:{feature}={}", if enabled.is_some() { 1 } else { 0 });
    }
}
