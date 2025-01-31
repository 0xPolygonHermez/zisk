use clap::{Arg, Command};
use colored::Colorize;
use p3_goldilocks::Goldilocks;
use proofman_common::{get_custom_commit_trace, GlobalInfo, ProofType, SetupCtx};
use proofman_util::create_buffer_fast;
use sm_rom::RomSM;
use std::{path::Path, sync::Arc};
use sysinfo::System;
use zisk_pil::RomRomTrace;

fn main() {
    let matches = Command::new("ROM Handler")
        .version("1.0")
        .about("Compute the Merkle Root of a ROM file")
        .arg(
            Arg::new("rom").long("rom").value_name("FILE").help("The ROM file path").required(true),
        )
        .arg(
            Arg::new("proving_key")
                .long("proving-key")
                .value_name("FILE")
                .help("The proving key folder path")
                .required(true),
        )
        .arg(
            Arg::new("rom_buffer")
                .long("rom-buffer")
                .value_name("FILE")
                .help("The rom buffer path")
                .required(true),
        )
        .get_matches();

    // Get the value of the `rom` argument as a path
    let rom_path_str = matches.get_one::<String>("rom").expect("ROM path is required");
    let rom_path = Path::new(rom_path_str);
    let proving_key_path_str =
        matches.get_one::<String>("proving_key").expect("Proving key path is required");
    let proving_key_path = Path::new(proving_key_path_str);
    let rom_buffer_str =
        matches.get_one::<String>("rom_buffer").expect("Buffer file path is required");

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

    let global_info = GlobalInfo::new(proving_key_path);
    let sctx = Arc::new(SetupCtx::<Goldilocks>::new(&global_info, &ProofType::Basic));

    let mut custom_rom_trace: RomRomTrace<Goldilocks> = RomRomTrace::new();
    let setup = sctx.get_setup(custom_rom_trace.airgroup_id(), custom_rom_trace.air_id);

    RomSM::compute_custom_trace_rom(rom_path.to_path_buf(), &mut custom_rom_trace);

    let n_ext = (1 << setup.stark_info.stark_struct.n_bits_ext) as usize;
    let n_cols = custom_rom_trace.num_rows();

    let buffer_ext = create_buffer_fast(n_ext * n_cols);

    get_custom_commit_trace(
        custom_rom_trace.commit_id.unwrap() as u64,
        0,
        setup,
        custom_rom_trace.get_buffer(),
        buffer_ext,
        rom_buffer_str.as_str(),
    );
}
