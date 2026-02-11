use std::path::PathBuf;
use zisk_sdk::{ZiskIO, ZiskStdin, build_program};

fn main() {
    build_program("../guest");
    let n = 1000u32;
    let stdin_save = ZiskStdin::new();
    stdin_save.write(&n);
    // Check if path exists, if not write
    let path = PathBuf::from("tmp/verify_constraints_input.bin");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    stdin_save.save(&path).unwrap();
}
