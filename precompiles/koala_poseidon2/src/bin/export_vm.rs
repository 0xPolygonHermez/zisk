use std::{env, fs, path::PathBuf};

use zisk_koala_poseidon2_foundation::rounds;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let destination = PathBuf::from(args.next().ok_or("usage: export_vm OUTPUT.pil [--check]")?);
    let check = match args.next().as_deref() {
        None => false,
        Some("--check") => true,
        _ => return Err("unexpected argument".into()),
    };
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let expected = rounds::vm_pil();
    if check {
        if fs::read_to_string(&destination)? != expected {
            return Err("VM PIL differs from the pinned generator".into());
        }
    } else {
        fs::write(destination, expected)?;
    }
    Ok(())
}
