use colored::Colorize;
use sysinfo::System;
use tracing::info;
use zisk_common::ZiskExecutorTime;
use zisk_sdk::ExecuteOutput;

pub(crate) fn print_banner() {
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

pub(crate) fn print_banner_command(command: impl std::fmt::Display) {
    print_banner_field("Command", command);
}

pub(crate) fn print_banner_field(label: &str, value: impl std::fmt::Display) {
    println!("{} {}", format!("{: >12}", label).bright_green().bold(), value);
}

/// Print the standard command banner shared by the embedded and remote command
/// trees: the ZisK banner, the command name, and the ELF / input / hints fields.
pub(crate) fn print_job_banner(
    command: &str,
    elf: &std::path::Path,
    inputs: Option<&str>,
    hints: Option<&str>,
) {
    print_banner();
    print_banner_command(command);
    print_banner_field("Elf", elf.display());
    let inputs_str = inputs.map_or_else(|| "None".dimmed().to_string(), str::to_string);
    print_banner_field("Input", inputs_str);
    if let Some(hints) = hints {
        print_banner_field("Prec. Hints", hints);
    }
}

pub(crate) fn print_execution_summary(
    executor_time: &ZiskExecutorTime,
    total_duration: u64,
    steps: u64,
    overhead_label: &str,
) {
    info!("Execution completed in {}ms, steps: {}", total_duration, steps);
    info!(
        "Execution summary: {} {}ms + {} {}ms + {} {}ms + {} {}ms",
        overhead_label.dimmed(),
        total_duration - executor_time.total_duration,
        "Execution".dimmed(),
        executor_time.execution_duration,
        "Count&Plan".dimmed(),
        executor_time.count_and_plan_duration,
        "Count&Plan MO".dimmed(),
        executor_time.count_and_plan_mo_duration,
    );

    /*●⎿✔◼✽*/
}

/// Render an [`ExecuteOutput`] to the log — shared by the embedded and remote
/// `execute` commands so both report identically.
pub(crate) fn print_execute_output(output: &ExecuteOutput) {
    // Summary line.
    let steps = output.get_execution_steps();
    let time = output.get_execution_time();
    let cost =
        output.get_execution_cost().map(|c| format!("{} cells", c)).unwrap_or("N/A".to_string());
    info!("Execution completed in {}ms, steps: {}, cost: {}", time, steps, cost);

    // Time breakdown.
    let et = output.get_executor_time();
    info!(
        "Execution time breakdown: {}ms ({} {}ms + {} {}ms + {} {}ms)",
        et.total_duration,
        "Execution".dimmed(),
        et.execution_duration,
        "Count&Plan".dimmed(),
        et.count_and_plan_duration,
        "Count&Plan MO".dimmed(),
        et.count_and_plan_mo_duration,
    );
    if let Some(aei) = &et.asm_execution_duration {
        info!("Assembly execution speed: {:.3}ms ({:.0} MHz)", aei.time * 1000f32, aei.mhz);
    }

    // Plan, when present: one line, machines sorted by name, names dimmed.
    if let Some(plan) = output.get_plan() {
        let mut entries: Vec<_> = plan.iter().collect();
        entries.sort_by_key(|e| e.name);
        let total: usize = entries.iter().map(|e| e.count).sum();
        let body = entries
            .iter()
            .map(|e| format!("{}: {}", e.name.dimmed(), e.count))
            .collect::<Vec<_>>()
            .join(" | ");
        info!("Plan {} | Total instances: {}", body, total);
    }
}
