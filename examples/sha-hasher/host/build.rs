use std::path::PathBuf;
use zisk_sdk::ZiskStdin;

fn main() {
    let n = 1000u32;
    let stdin_save = ZiskStdin::new();
    stdin_save.write(&n);
    let path = PathBuf::from("tmp/verify_constraints_input.bin");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    stdin_save.save(&path).unwrap();
}
