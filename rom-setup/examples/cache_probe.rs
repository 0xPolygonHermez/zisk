//! Throwaway probe: prints the asm-binary cache base name for a fixed guest ELF
//! identity, so a wrapper script can mutate an input (lib / float / transpiler)
//! and observe whether the cache filename changes.
//!
//! `get_assembly_file_paths_from_id` is `pub` on both the base branch and the
//! fix branch, so the same probe runs on either — the point is that on base the
//! name is invariant to lib/float/transpiler changes (the bug), and on the fix
//! branch it changes (the fix).

fn main() {
    let [mt, _rh, _mo] = rom_setup::get_assembly_file_paths_from_id(
        "probe",
        "deadbeefcafef00d",
        std::path::Path::new("/tmp"),
        false,
    );
    println!("{}", mt.file_name().unwrap().to_string_lossy());
}
