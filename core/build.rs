use std::path::Path;

fn main() {
    // Ensure lib-float is built before we try to include ziskfloat.elf
    // The build-dependency on lib-float will trigger its build.rs first.

    // Tell cargo to rerun this build script if the float library changes
    let float_lib_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../lib-float/c/lib/ziskfloat.elf");

    println!("cargo:rerun-if-changed={}", float_lib_path.display());
}
