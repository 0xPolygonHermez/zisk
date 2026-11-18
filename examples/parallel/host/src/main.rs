//! ZisK host: executes the three guests of the parallel example and checks their committed
//! output against the same functions run natively.
//!
//! # Usage
//!
//! ```text
//! cargo run --release -- [--asm] [--tasks N] [--runs N]
//! ```
//!
//! With `--asm` the assembly emulator runs each fork-join region's tasks at the same time;
//! without it the Rust emulator runs them one after another. Either way the output, and the
//! execution the proof attests, are the same. `--tasks` sets how many tasks each region opens
//! (8 by default), and `--runs` how many times each guest runs (3 by default): the host checks
//! the output of every run and prints the median of each time.

use std::error::Error;

use parallel_common::{BINS, Params, cells_inside, checksum, histogram, values};
use zisk_sdk::{GuestProgram, ProverClient, ZiskStdin, load_program};

static PI: GuestProgram = load_program!("pi-guest");
static HISTOGRAM: GuestProgram = load_program!("histogram-guest");
static SORT: GuestProgram = load_program!("sort-guest");

const USAGE: &str = "\
usage: parallel-host [--asm] [--tasks N] [--runs N]

  --asm      run the assembly emulator, which runs the tasks of each region at the same time
  --tasks N  tasks per region (default 8)
  --runs N   executions of each guest; the host prints the median times (default 3)";

/// What the command line asks for
struct Args {
    /// The assembly emulator instead of the Rust one
    asm: bool,
    /// Tasks per region
    tasks: u64,
    /// Executions of each guest
    runs: usize,
}

impl Args {
    /// Parses the arguments after the program's name: an argument it does not know, or a value
    /// that is not a whole number greater than 0, is an error
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut parsed = Self { asm: false, tasks: 8, runs: 3 };
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--asm" => parsed.asm = true,
                "--tasks" => parsed.tasks = positive(&arg, args.next())?,
                "--runs" => parsed.runs = positive(&arg, args.next())? as usize,
                _ => return Err(format!("unknown argument `{arg}`")),
            }
        }
        Ok(parsed)
    }
}

/// The value that follows `flag`, which must be a whole number greater than 0
fn positive(flag: &str, value: Option<String>) -> Result<u64, String> {
    let value = value.ok_or_else(|| format!("{flag} needs a value"))?;
    match value.parse() {
        Ok(n) if n > 0 => Ok(n),
        _ => Err(format!("{flag} takes a whole number greater than 0, not `{value}`")),
    }
}

/// The median of `values`, which must not be empty: the upper one of the middle two when there
/// is an even number of them
fn median(mut values: Vec<u64>) -> u64 {
    values.sort_unstable();
    values[values.len() / 2]
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        println!("{USAGE}");
        return Ok(());
    }
    let args = Args::parse(args.into_iter()).unwrap_or_else(|err| {
        eprintln!("error: {err}\n\n{USAGE}");
        std::process::exit(2)
    });

    // Execution only: no proving key needed
    let builder = ProverClient::embedded().execute_only();
    let client = if args.asm { builder.assembly() } else { builder.emulator() }.build()?;

    // Executes `program` `args.runs` times, prints its steps and the median times, and returns
    // what each run committed
    let run = |program: &GuestProgram, name: &str, params: Params| -> Result<_, Box<dyn Error>> {
        client.setup(program, false)?;
        let mut steps = 0;
        let mut phases = Vec::with_capacity(args.runs);
        let mut emulations = Vec::with_capacity(args.runs);
        let mut publics = Vec::with_capacity(args.runs);
        for _ in 0..args.runs {
            let stdin = ZiskStdin::new();
            stdin.write(&params);
            let output = client.execute(program, stdin, None, None)?;
            steps = output.get_execution_steps();
            // The execution phase: emulation, counting and memory plan, without the program's
            // start-up; on the assembly emulator, the emulation alone too
            let time = output.get_executor_time();
            phases.push(
                time.execution_duration
                    + time.count_and_plan_duration
                    + time.count_and_plan_mo_duration,
            );
            if let Some(asm) = &time.asm_execution_duration {
                emulations.push((asm.time * 1e3).round() as u64);
            }
            publics.push(output.get_publics().clone());
        }
        let emulation = if emulations.is_empty() {
            String::new()
        } else {
            format!(", emulation {} ms", median(emulations))
        };
        let runs = match args.runs {
            1 => String::new(),
            runs => format!(" (medians of {runs} runs)"),
        };
        println!(
            "{name}: {steps} steps, {} tasks, {}: execution {} ms{emulation}{runs}",
            params.tasks,
            if args.asm { "assembly emulator" } else { "Rust emulator" },
            median(phases),
        );
        Ok(publics)
    };

    // π from the cells of a 2048 × 2048 grid inside a quarter circle
    let params = Params { len: 2048, tasks: args.tasks, seed: 0 };
    let inside = cells_inside(0..params.len, params.len);
    for publics in run(&PI, "pi", params)? {
        assert_eq!(publics.read::<u64>()?, inside, "pi");
    }
    println!("  π ≈ {:.6}", 4.0 * inside as f64 / (params.len * params.len) as f64);

    // A histogram of 2^20 values
    let params = Params { len: 1 << 20, tasks: args.tasks, seed: 0x5eed };
    let counts = histogram(0..params.len, params.seed);
    for publics in run(&HISTOGRAM, "histogram", params)? {
        assert_eq!(publics.read::<[u64; BINS]>()?, counts, "histogram");
    }

    // A sort of 2^18 values, and a checksum and a histogram of the sorted values
    let params = Params { len: 1 << 18, tasks: args.tasks, seed: 0xc0ffee };
    let mut sorted = values(0..params.len, params.seed);
    sorted.sort_unstable();
    let (sum, bins) = (checksum(sorted.iter().copied()), histogram(0..params.len, params.seed));
    for publics in run(&SORT, "sort", params)? {
        assert_eq!(publics.read::<u64>()?, params.len, "sort: length");
        assert_eq!(publics.read::<u64>()?, sum, "sort: checksum");
        assert_eq!(publics.read::<[u64; BINS]>()?, bins, "sort: histogram");
    }

    println!("\u{2713} every guest gave what running its tasks one after another gives");
    Ok(())
}
