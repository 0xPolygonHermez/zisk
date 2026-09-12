use std::{env, fs, io::BufWriter};

use zisk_koala_poseidon2_foundation::Parameters;

/// Writes the pinned parameters as JSON for `definitions/tools/koala_parameters.mjs`.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let destination = args.next().ok_or("usage: export_parameters OUTPUT.json")?;
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let file = BufWriter::new(fs::File::create(destination)?);
    serde_json::to_writer_pretty(file, &Parameters::pinned())?;
    Ok(())
}
