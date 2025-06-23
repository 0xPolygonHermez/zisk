use colored::Colorize;
use sysinfo::System;

pub fn print_banner() {
    println!();
    println!(
        "{}",
        format!("\x1b[38;2;10;191;131m{: >12} {}\x1b[0m", "ZisK zkVM", env!("CARGO_PKG_VERSION"))
            .bold()
    );

    // System
    let system_name = System::name().unwrap_or_else(|| "<unknown>".to_owned());
    let system_kernel = System::kernel_version().unwrap_or_else(|| "<unknown>".to_owned());
    let system_version = System::long_os_version().unwrap_or_else(|| "<unknown>".to_owned());

    println!(
        "{}",
        format!("{: >12} {} {} ({})", "System", system_name, system_kernel, system_version)
            .bright_green()
            .bold()
    );

    // Hostname
    let system_hostname = System::host_name().unwrap_or_else(|| "<unknown>".to_owned());
    println!("{} {}", format!("{: >12}", "Hostname").bright_green().bold(), system_hostname);

    // CPU
    // let system = System::new_all();

    // let system_cores = system.cpus().len();
    // let system_cores_freq = system.cpus()[0].frequency();
    // let system_cores_vendor_id = system.cpus()[0].vendor_id();
    // let system_cores_brand = system.cpus()[0].brand();

    // let system_cores_desc = format!(
    //     "{} cores @ {}MHz ({}) {}",
    //     system_cores, system_cores_freq, system_cores_vendor_id, system_cores_brand
    // );
    // println!("{} {}", format!("{: >12}", "CPU").bright_green().bold(), system_cores_desc);

    // // Memory
    // let total_mem = system.total_memory() >> 30;
    // let available_mem = system.available_memory() >> 30;
    // println!(
    //     "{} {}GB total ({}GB available)",
    //     format!("{: >12}", "Mem").bright_green().bold(),
    //     total_mem,
    //     available_mem
    // );
}
