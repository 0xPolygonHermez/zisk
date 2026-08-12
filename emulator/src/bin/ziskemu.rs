use clap::Parser;
use std::{fmt::Write, fs, process};
use zisk_common::EmuTrace;
use ziskemu::{diff_stats_files, report, resolve_color, EmuOptions, Emulator, ZiskEmulator};

fn main() {
    // Create a emulator options instance based on arguments or default values
    let options: EmuOptions = EmuOptions::parse();

    // Compare two saved stats snapshots without running the emulator.
    if let Some(files) = &options.diff_stats {
        let (old, new) = (&files[0], &files[1]);
        // `--html-report` renders the comparison as a page instead of printing it.
        if let Some(html_path) = &options.html_report {
            let written = fs::read_to_string(old)
                .and_then(|old_csv| fs::read_to_string(new).map(|new_csv| (old_csv, new_csv)))
                .and_then(|(old_csv, new_csv)| {
                    let html = report::render_compare(&old_csv, old, &new_csv, new);
                    fs::write(html_path, html)
                });
            match written {
                Ok(()) => println!("HTML report written to: {html_path}"),
                Err(e) => {
                    eprintln!("Failed to render HTML report for '{old}' and '{new}': {e}");
                    process::exit(1);
                }
            }
            return;
        }
        match diff_stats_files(
            old,
            new,
            options.diff_use_csv(),
            resolve_color(&options.color),
            options.csv_sep(),
        ) {
            Ok(comparison) => print!("{comparison}"),
            Err(e) => {
                eprintln!("Failed to compare stats snapshots '{old}' and '{new}': {e}");
                process::exit(1);
            }
        }
        return;
    }

    //println! {"options={}", options};

    // Log the emulator options if requested
    if options.verbose {
        println!("ziskemu converts an ELF RISCV file into a ZISK rom or loads a ZISK rom file, emulates it with the provided input, and copies the output to console or a file");
    }

    // Call emulate, with these options
    let emulator = ZiskEmulator;
    let result = emulator.emulate(&options, None::<Box<dyn Fn(EmuTrace)>>);

    match result {
        Ok(result) => {
            // println!("Emulation completed successfully");
            result.iter().fold(String::new(), |mut acc, byte| {
                write!(&mut acc, "{byte:02x}").unwrap();
                acc
            });
            // print!("Result: 0x{}", hex_string);
        }
        Err(e) => {
            eprintln!("Error during emulation: {e:?}");
            process::exit(1);
        }
    }
}
