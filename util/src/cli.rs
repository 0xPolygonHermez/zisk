use colored::Colorize;
use sysinfo::System;

pub fn print_banner(extended: bool) {
    println!("");
    println!(
        "    {}{}",
        "PROOFMAN by Polygon Labs v".bright_purple().bold(),
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

    if extended {
        print_extended_banner();
    }
}

pub fn print_extended_banner() {
    let system = System::new_all();

    let system_cores = system.physical_core_count().map(|c| c.to_string()).unwrap_or_else(|| "Unknown".to_owned());
    let system_cores_brand = system.cpus()[0].brand();
    println!("{} {} cores ({})", format!("{: >12}", "CPU").bright_green().bold(), system_cores, system_cores_brand);

    let total_mem = system.total_memory() / 1_000_000_000;
    let available_mem = system.available_memory() / 1_000_000_000;
    println!(
        "{} {}GB total ({}GB available)",
        format!("{: >12}", "Mem").bright_green().bold(),
        total_mem,
        available_mem
    );

    println!(
        "{} {}",
        format!("{: >12}", "Loaded").bright_green().bold(),
        std::env::current_exe().unwrap().display().to_string().as_str()
    );
    println!("{} {}", format!("{: >12}", "Main PID").bright_green().bold(), std::process::id().to_string().as_str());
}
