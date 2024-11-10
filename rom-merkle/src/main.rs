use clap::{Arg, Command};
use colored::Colorize;
use p3_goldilocks::Goldilocks;
use proofman_common::{GlobalInfo, ProofType, SetupCtx};
use sm_rom::RomSM;
use stark::StarkBufferAllocator;
use std::{path::Path, sync::Arc};
use sysinfo::System;
use std::ffi::c_void;
use proofman_starks_lib_c::{starks_new_c, fri_proof_new_c, extend_and_merkelize_custom_commit_c};
use zisk_pil::{ ROM_AIR_IDS, ZISK_AIRGROUP_ID };

fn main() {
    let matches = Command::new("ROM Handler")
        .version("1.0")
        .about("Compute the Merkle Root of a ROM file")
        .arg(
            Arg::new("rom")
                .long("rom")
                .value_name("FILE")
                .help("The ROM file path")
                .required(true)
        )
        .arg(
            Arg::new("proving_key")
                .long("proving-key")
                .value_name("FILE")
                .help("The proving key folder path")
                .required(true)
        )
        .arg(
            Arg::new("rom_buffer")
                .long("rom-buffer")
                .value_name("FILE")
                .help("The rom buffer path")
                .required(true)
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

    let buffer_allocator: Arc<StarkBufferAllocator> =
        Arc::new(StarkBufferAllocator::new(proving_key_path.to_path_buf()));
    let global_info = GlobalInfo::new(&proving_key_path);
    let sctx = Arc::new(SetupCtx::new(&global_info, &ProofType::Basic));

    let setup = sctx.get_setup(ZISK_AIRGROUP_ID, ROM_AIR_IDS[0]);

    let p_stark = starks_new_c((&setup.p_setup).into(), std::ptr::null_mut());
    let p_proof = fri_proof_new_c((&setup.p_setup).into());

    match RomSM::<Goldilocks>::compute_trace_rom_buffer(rom_path.to_path_buf(), buffer_allocator, &sctx) {
        Ok((commit_id, buffer_rom)) => {
            extend_and_merkelize_custom_commit_c(
                p_stark,
                commit_id as u64,
                0,
                buffer_rom.as_ptr() as *mut c_void,
                p_proof,
                std::ptr::null_mut(),
                rom_buffer_str.as_str(),
            );
        }
        Err(e) => {
            log::error!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
