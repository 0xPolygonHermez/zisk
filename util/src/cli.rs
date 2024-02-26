use sysinfo::System;

pub const PURPLE: &str = "\x1b[35m";
pub const GREEN: &str = "\x1b[32;1m";
pub const RESET: &str = "\x1b[37;0m";
pub const BOLD: &str = "\x1b[1m";

pub fn print_banner(extended: bool) {
    println!("");
    println!("    {}{}PROOFMAN by Polygon Labs v{}{}", BOLD, PURPLE, env!("CARGO_PKG_VERSION"), RESET);
    let system_name = System::name().unwrap_or_else(|| "<unknown>".to_owned());
    let system_kernel = System::kernel_version().unwrap_or_else(|| "<unknown>".to_owned());
    let system_version = System::long_os_version().unwrap_or_else(|| "<unknown>".to_owned());
    println!(
        "{}{}{} {} {} ({})",
        GREEN,
        format!("{: >12}", "System"),
        RESET,
        system_name,
        system_kernel,
        system_version
    );
    let system_hostname = System::host_name().unwrap_or_else(|| "<unknown>".to_owned());
    println!("{}{}{} {}", GREEN, format!("{: >12}", "Hostname"), RESET, system_hostname);

    if extended {
        print_extended_banner();
    }
}

pub fn print_extended_banner() {
    let system = System::new_all();

    let system_cores = system.physical_core_count().map(|c| c.to_string()).unwrap_or_else(|| "Unknown".to_owned());
    let system_cores_brand = system.cpus()[0].brand();
    println!("{}{}{} {} cores ({})", GREEN, format!("{: >12}", "CPU"), RESET, system_cores, system_cores_brand);

    let total_mem = system.total_memory() / 1_000_000_000;
    let available_mem = system.available_memory() / 1_000_000_000;
    println!("{}{}{} {}GB total ({}GB available)", GREEN, format!("{: >12}", "Mem"), RESET, total_mem, available_mem);

    println!(
        "{}{}{} {}",
        GREEN,
        format!("{: >12}", "Loaded"),
        RESET,
        std::env::current_exe().unwrap().display().to_string().as_str()
    );
    println!("{}{}{} {}", GREEN, format!("{: >12}", "Main PID"), RESET, std::process::id().to_string().as_str());
}
