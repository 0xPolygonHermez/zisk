use std::fs;
use std::io;
use std::path::Path;

const INPUT_DIR: &str = "./inputs";

fn main() -> io::Result<()> {
    let num_keccakfs: u64 = 1;

    // Ensure the input directory exists
    let input_dir = Path::new(INPUT_DIR);
    if !input_dir.exists() {
        fs::create_dir_all(input_dir)?;
    }

    // Create the file and write the inputs
    let file_name = format!("{num_keccakfs}_keccakf_inputs.bin");
    let file_path = input_dir.join(file_name);

    let stdin = zisk_sdk::ZiskStdin::new();
    stdin.write(&num_keccakfs);
    stdin.save(&file_path).expect("Failed to write input to file");

    Ok(())
}
