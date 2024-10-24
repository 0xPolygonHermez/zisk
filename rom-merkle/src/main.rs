use clap::{Arg, Command};
use colored::Colorize;
use p3_field::AbstractField;
use p3_goldilocks::Goldilocks;
use sm_rom::RomSM;
use std::path::Path;
use sysinfo::System;

fn main() {
    let matches = Command::new("ROM Handler")
        .version("1.0")
        .about("Compute the Merkle Root of a ROM file")
        .arg(Arg::new("rom").value_name("FILE").help("The ROM file path").required(true).index(1))
        .get_matches();

    // Get the value of the `rom` argument as a path
    let rom_path_str = matches.get_one::<String>("rom").expect("ROM path is required");
    let rom_path = Path::new(rom_path_str);

    env_logger::builder()
        .format_timestamp(None)
        .format_level(true)
        .format_target(false)
        .filter_level(log::LevelFilter::Info)
        .init();

    println!();
    println!(
        "    {}{}",
        "Proofman by Polygon Labs v".bright_purple().bold(),
        env!("CARGO_PKG_VERSION").bright_purple().bold()
    );

    let system_name = System::name().unwrap_or_else(|| "<unknown>".to_owned());
    let system_kernel = System::kernel_version().unwrap_or_else(|| "<unknown>".to_owned());
    let system_version = System::long_os_version().unwrap_or_else(|| "<unknown>".to_owned());
    println!(
        "{} {} {} ({})",
        format!("{: >12}", "System").bright_green().bold(),
        system_name,
        system_kernel,
        system_version
    );
    let system_hostname = System::host_name().unwrap_or_else(|| "<unknown>".to_owned());
    println!("{} {}", format!("{: >12}", "Hostname").bright_green().bold(), system_hostname);
    println!();

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

    // If all checks pass, continue with the program
    println!("ROM Path is valid: {}", rom_path.display());

    type F = Goldilocks;
    let prover_buffer = &mut [F::zero(); 1];
    let offset = 0;

    if let Err(e) =
        RomSM::<Goldilocks>::compute_trace(rom_path.to_path_buf(), prover_buffer, offset)
    {
        log::error!("Error: {}", e);
        std::process::exit(1);
    }

    log::info!("ROM proof successful");
}
