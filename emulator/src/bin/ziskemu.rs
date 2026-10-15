use clap::Parser;
use std::{fmt::Write, fs, process};
use zisk_common::{EmuTrace, RandomSeed, RandomSeedArgs};
use ziskemu::{
    diff_stats_files, report, resolve_color, EmuOptions, Emulator, FailureReason, ZiskEmulator,
    ZiskEmulatorErr, ZISK_VERSION_MESSAGE,
};

/// ziskemu's command line: the emulator options plus the random seed.
#[derive(Parser)]
#[command(version = ZISK_VERSION_MESSAGE, about, long_about = None)]
struct Cli {
    #[command(flatten)]
    options: EmuOptions,

    #[command(flatten)]
    random_seed: RandomSeedArgs,
}

fn main() {
    // Create a emulator options instance based on arguments or default values
    let Cli { mut options, random_seed: seed } = Cli::parse();

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

    // Each execution gets a fresh seed for the host randomness generator, unless one is given.
    // Reported on stderr: stdout may carry the guest output (--log-output, --log-output-riscof).
    let provided = seed.provided();
    let random_seed = match provided.map_or_else(RandomSeed::random, Ok) {
        Ok(random_seed) => random_seed,
        Err(e) => {
            eprintln!("Error drawing a random seed: {e}");
            process::exit(1);
        }
    };
    let note = if provided.is_some() { " (provided)" } else { "" };
    eprintln!("Random seed: {random_seed}{note}");
    options.random_seed = Some(random_seed);

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
        Err(ZiskEmulatorErr::ExecutionFailed { reason: FailureReason::ExitCode(code), .. }) => {
            // Report the guest's exit code to the host (as a shell exit status: its low 8
            // bits, or 1 if those are 0, so a failure never looks like success)
            eprintln!("Error during emulation: guest exited with code {}", code as i64);
            process::exit(match code as u8 {
                0 => 1,
                status => status as i32,
            });
        }
        Err(e) => {
            eprintln!("Error during emulation: {e:?}");
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    /// `EmuOptions` and `RandomSeedArgs` are flattened together: no clashing names or short flags.
    #[test]
    fn cli_is_well_formed() {
        Cli::command().debug_assert();
    }

    #[test]
    fn random_seed_flag_reaches_the_cli() {
        let cli =
            Cli::try_parse_from(["ziskemu", "-e", "guest.elf", "--random-seed", "2a"]).unwrap();
        assert_eq!(cli.random_seed.provided(), Some("2a".parse().unwrap()));
        assert_eq!(cli.options.elf.as_deref(), Some("guest.elf"));
    }
}
