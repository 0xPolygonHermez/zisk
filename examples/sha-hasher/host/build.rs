use std::path::PathBuf;
use zisk_common::io::{ZiskIO, ZiskStdin};

fn main() {
    zisk_build::build_program("../guest");
    let n = 1000u32;
    let stdin_save = ZiskStdin::new();
    stdin_save.write(&n);
    stdin_save.save(&PathBuf::from("tmp/verify_constraints_input.bin")).unwrap();
}
