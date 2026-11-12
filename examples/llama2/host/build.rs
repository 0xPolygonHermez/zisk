// Cargo runs this build script before compiling the host.
fn main() {
    // Compile the guest crate to a ZisK-target ELF. The build exports its
    // path as the `ZISK_ELF_llama2-guest` environment variable, which
    // `llama2-zisk` hands to ziskemu.
    zisk_sdk::build_program("../guest");
}
