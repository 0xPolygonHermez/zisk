use clap::{Arg, Command};
use colored::Colorize;
use p3_goldilocks::Goldilocks;
use proofman_common::{GlobalInfo, ProofType, SetupCtx};
use sm_rom::RomSM;
use stark::StarkBufferAllocator;
use std::{path::Path, sync::Arc};
use sysinfo::System;

fn main() {
    let matches = Command::new("ROM Handler")
        .version("1.0")
        .about("Compute the Merkle Root of a ROM file")
        .arg(Arg::new("rom").value_name("FILE").help("The ROM file path").required(true).index(1))
        .arg(
            Arg::new("proving_key")
                .value_name("FILE")
                .help("The proving key folder path")
                .required(true)
                .index(2),
        )
        .arg(
            Arg::new("global_info")
                .value_name("FILE")
                .help("The global info file path")
                .required(true)
                .index(3),
        )
        .get_matches();

    // Get the value of the `rom` argument as a path
    let rom_path_str = matches.get_one::<String>("rom").expect("ROM path is required");
    let rom_path = Path::new(rom_path_str);
    let proving_key_path_str =
        matches.get_one::<String>("proving_key").expect("Proving key path is required");
    let proving_key_path = Path::new(proving_key_path_str);
    let global_info_path_str =
        matches.get_one::<String>("global_info").expect("Global info path is required");
    let global_info_path = Path::new(global_info_path_str);

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

    let buffer_allocator: Arc<StarkBufferAllocator> =
        Arc::new(StarkBufferAllocator::new(proving_key_path.to_path_buf()));
    let global_info = GlobalInfo::new(global_info_path);
    let sctx = Arc::new(SetupCtx::new(&global_info, &ProofType::Basic));

    if let Err(e) =
        RomSM::<Goldilocks>::compute_trace(rom_path.to_path_buf(), buffer_allocator, &sctx)
    {
        log::error!("Error: {}", e);
        std::process::exit(1);
    }

    // Compute LDE and Merkelize and get the root of the rom
    // TODO: Implement the logic to compute the trace

    log::info!("ROM proof successful");
}
