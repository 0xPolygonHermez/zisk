use clap::{Arg, ArgAction, Command};
use rom_merkle::gen_elf_hash;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("ROM Handler")
        .version("1.0")
        .about("Compute the Merkle Root of a ROM file")
        .arg(
            Arg::new("rom").long("rom").value_name("FILE").help("The ROM file path").required(true),
        )
        .arg(
            Arg::new("buffer").long("buffer").value_name("FILE").help("Buffer path").required(true),
        )
        .arg(
            Arg::new("check")
                .long("check")
                .help("Check the computed hash")
                .required(false)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("blowup_factor")
                .long("blowup_factor")
                .value_name("FACTOR")
                .help("Blowup factor")
                .required(false)
                .default_value("2"),
        )
        .get_matches();

    let rom_path_str = matches.get_one::<String>("rom").expect("ROM path is required");
    let rom_buffer_str = matches.get_one::<String>("buffer").expect("Buffer file path is required");
    let check = *matches.get_one::<bool>("check").expect("Check is required");
    let blowup_factor = matches
        .get_one::<String>("blowup_factor")
        .expect("Blowup factor is required")
        .parse::<u64>()?;

    let rom_path = Path::new(&rom_path_str);

    // Check if the path exists
    if !rom_path.exists() {
        log::error!("Error: The specified ROM file does not exist: {}", rom_path_str);
        std::process::exit(1);
    }

    // Check if the path is a file and not a directory
    if !rom_path.is_file() {
        log::error!("Error: The specified ROM path is not a file: {}", rom_path_str);
        std::process::exit(1);
    }

    let root = gen_elf_hash(rom_path, rom_buffer_str, blowup_factor, check)?;
    println!("Root hash: {:?}", root);
    println!("ROM hash computed successfully");
    Ok(())
}
