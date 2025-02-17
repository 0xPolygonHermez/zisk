use clap::{Arg, ArgAction, Command};
use rom_merkle::{gen_elf_hash, get_elf_bin_file_path, get_rom_blowup_factor, DEFAULT_CACHE_PATH};
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("ROM Handler")
        .version("1.0")
        .about("Compute the Merkle Root of a ROM file")
        .arg(
            Arg::new("rom").long("rom").value_name("FILE").help("The ROM file path").required(true),
        )
        .arg(
            Arg::new("default-cache")
                .long("default-cache")
                .value_name("FILE")
                .help("Default cache path")
                .required(false),
        )
        .arg(Arg::new("proving-key").long("proving-key").help("Proving Key path").required(true))
        .arg(
            Arg::new("check")
                .long("check")
                .help("Check the computed hash")
                .required(false)
                .action(ArgAction::SetTrue),
        )
        .get_matches();

    let proving_key_path =
        matches.get_one::<PathBuf>("proving-key").expect("Proving key path is required");
    let rom_path_str = matches.get_one::<String>("rom").expect("ROM path is required");
    let default_cache_path = matches
        .get_one::<String>("default-cache")
        .map(PathBuf::from) // If provided, convert to PathBuf
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CACHE_PATH)); // Otherwise, use default

    let check = *matches.get_one::<bool>("check").expect("Check is required");

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

    let blowup_factor = get_rom_blowup_factor(proving_key_path);

    let rom_bin_path =
        get_elf_bin_file_path(&rom_path.to_path_buf(), &default_cache_path, blowup_factor)?;

    let root = gen_elf_hash(rom_path, rom_bin_path.to_str().unwrap(), blowup_factor, check)?;
    println!("Root hash: {:?}", root);
    println!("ROM hash computed successfully");
    Ok(())
}
