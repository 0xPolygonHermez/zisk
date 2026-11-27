// Cargo runs this build script before compiling the host.
fn main() {
    // Compile the guest crate to a ZisK-target ELF.
    zisk_sdk::build_program("../guest");
}
