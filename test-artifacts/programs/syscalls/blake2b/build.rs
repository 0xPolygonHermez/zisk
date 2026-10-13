use std::fs;
use std::io;
use std::path::Path;

const INPUT_DIR: &str = "./inputs";

fn main() -> io::Result<()> {
    let num_blake2b_rounds: u64 = 1;

    // Ensure the input directory exists
    let input_dir = Path::new(INPUT_DIR);
    if !input_dir.exists() {
        fs::create_dir_all(input_dir)?;
    }

    // Create the file and write the inputs
    let file_name = format!("{num_blake2b_rounds}_blake2b_inputs.bin");
    let file_path = input_dir.join(file_name);

    fs::write(&file_path, stdin_frame(num_blake2b_rounds))?;

    Ok(())
}

/// Encodes `value` as a single-frame ZisK stdin file, byte-identical to what
/// `ZiskStdin::write(&value)` + `save()` produce: an 8-byte little-endian length
/// prefix, the bincode-standard (varint) encoding of `value`, then zero padding to
/// an 8-byte boundary. Done by hand so this build script needs no build
/// dependencies; depending on `zisk-sdk` here pulled the whole host prover into the
/// programs workspace build.
fn stdin_frame(value: u64) -> Vec<u8> {
    let payload: Vec<u8> = match value {
        0..=250 => vec![value as u8],
        251..=0xffff => [&[251u8][..], &(value as u16).to_le_bytes()].concat(),
        0x1_0000..=0xffff_ffff => [&[252u8][..], &(value as u32).to_le_bytes()].concat(),
        _ => [&[253u8][..], &value.to_le_bytes()].concat(),
    };
    let mut frame = (payload.len() as u64).to_le_bytes().to_vec();
    frame.extend_from_slice(&payload);
    frame.resize(frame.len().div_ceil(8) * 8, 0);
    frame
}
