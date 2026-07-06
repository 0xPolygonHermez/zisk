//! `frops-analyzer` — analyze operation traces and propose FROPS (frequent operations).
//!
//! See README.md for usage and FROPS.md for the conceptual model.

mod addstats;
mod codegen;
mod current;
mod ingest;
mod ops;
mod optimize;
mod region;
mod report;

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use ingest::Aggregator;
use optimize::{optimize, Config};

#[derive(Parser)]
#[command(name = "frops-analyzer", version, about = "Analyze op traces and propose FROPS tables")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Args)]
struct CommonArgs {
    /// Directory containing `*.bin` operation traces (from `ziskemu --store-op-output`).
    #[arg(long)]
    input: PathBuf,
    /// Maximum total FROPS table rows (sum of the three tables).
    #[arg(long)]
    max_table: u64,
    /// Number of distributed nodes (the FROPS table is recomputed per node).
    #[arg(long, default_value_t = 1)]
    nodes: u64,
    /// Account for instance padding to NUM_ROWS in the area model.
    #[arg(long, default_value_t = false)]
    padding: bool,
    /// Per-row area cost of the FROPS table.
    #[arg(long, default_value_t = 3)]
    table_cost: u64,
    /// Upper bound (exclusive) for the "low value" region of a and b.
    #[arg(long, default_value_t = 1024)]
    low_cap: u64,
    /// Maximum FROPS regions per opcode (bounds the cost of the membership test).
    #[arg(long, default_value_t = 16)]
    max_regions_per_op: usize,
    /// Table partition bits: each family's table is padded to a multiple of 2^partition_bits rows.
    /// `max-table` bounds the total paid (padded) rows.
    #[arg(long, default_value_t = 21)]
    partition_bits: u32,
}

impl CommonArgs {
    fn config(&self) -> Config {
        Config {
            max_table: self.max_table,
            nodes: self.nodes.max(1),
            padding: self.padding,
            table_cost: self.table_cost,
            low_cap: self.low_cap.clamp(1, 65536),
            max_regions_per_op: self.max_regions_per_op.max(1),
            partition_bits: self.partition_bits.clamp(1, 40),
        }
    }
}

#[derive(Subcommand)]
enum Command {
    /// Approach 1: write compressed `proposal.json` + `report.md` for review (no source changes).
    Analyze {
        #[command(flatten)]
        common: CommonArgs,
        /// Output directory for the report artifacts.
        #[arg(long, default_value = "build/frops-report")]
        report_dir: PathBuf,
    },
    /// Approach 2: regenerate the `*_frops.rs` source files in place (also writes a report).
    Generate {
        #[command(flatten)]
        common: CommonArgs,
        /// Workspace root containing `state-machines/...`.
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        /// Also write the report artifacts here.
        #[arg(long, default_value = "build/frops-report")]
        report_dir: PathBuf,
    },
    /// Per-block stats: non-FROPS 64-bit ADDs whose operands' high 32 bits are zero (no high-half
    /// computation needed), as a share of the non-FROPS ADDs. Uses the current in-tree FROPS.
    AddHi0 {
        /// Directory containing `*.bin` operation traces.
        #[arg(long)]
        input: PathBuf,
    },
    /// Distribution / clustering of a and b over the hi0-no-carry non-FROPS ADD subset.
    AddHi0Dist {
        /// Directory containing `*.bin` operation traces.
        #[arg(long)]
        input: PathBuf,
    },
    /// Per-block distribution of non-FROPS operations, plus EQ specifics (a==b, hi0).
    NonfropDist {
        /// Directory containing `*.bin` operation traces.
        #[arg(long)]
        input: PathBuf,
    },
    /// FROPS table-entry analysis: how many materialised rows have hi32(a)=hi32(b)=hi32(c)=0, and of
    /// those how many also have the flag set. Uses the current in-tree FROPS tables (no input needed).
    TableHi,
    /// FROPS per op: entries per operation and how many have a[1]=b[1]=c[1]=0 and flag=0.
    TableByOp,
    /// Constant-column optimization: split tables into 2^R partitions, order by zero-group then op,
    /// and report the area (column-rows) saved by replacing constant columns. (no input needed)
    TablePartition {
        /// log2 of the partition size in rows.
        #[arg(long, default_value_t = 21)]
        r: u32,
    },
    /// Emit x86-64 macros for the ORIGINAL hand-tuned FROPS (no input needed) to compare cycle costs
    /// against the generated ones. Writes emulator-asm/src/frops/frops_original.s.
    AsmOriginal {
        /// Output path for the generated assembly.
        #[arg(long, default_value = "emulator-asm/src/frops/frops_original.s")]
        output: PathBuf,
    },
    /// Per-file high-half classification of every op: Hi0 (a,b,c hi=0), Hi0+ (a,b hi=0),
    /// HiFFA (hi32(a)=0xFFFFFFFF), HiFFB (hi32(b)=0xFFFFFFFF), HiFF0 (a=FF,b=0), Hi0FF (a=0,b=FF).
    /// By default FROPS operations are NOT counted (analyse what specific machines would handle).
    HiClass {
        /// Directory containing `*.bin` operation traces.
        #[arg(long)]
        input: PathBuf,
        /// Also count operations already covered by FROPS.
        #[arg(long, default_value_t = false)]
        include_frops: bool,
    },
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::Analyze { common, report_dir } => {
            let cfg = common.config();
            let agg = ingest(&common, cfg)?;
            let prop = optimize(&agg, cfg);
            report::write_reports(&agg, &prop, &report_dir)?;
            print_summary(&agg, &prop);
            println!("\nReport written to {}/ (proposal.json, report.md)", report_dir.display());
        }
        Command::Generate { common, workspace, report_dir } => {
            let cfg = common.config();
            let agg = ingest(&common, cfg)?;
            let prop = optimize(&agg, cfg);
            report::write_reports(&agg, &prop, &report_dir)?;
            print_summary(&agg, &prop);
            let written = codegen::generate(&prop, &workspace)?;
            println!("\nGenerated source files (workspace {}):", workspace.display());
            for (path, backed) in &written {
                let note = if *backed { " (previous version saved as *.rs.bak)" } else { "" };
                println!("  {path}{note}");
            }
            println!("\nNext steps:");
            println!("  1. Review the diff of the generated *_frops.rs files.");
            println!("  2. Regenerate the .bin tables, e.g.:");
            println!("       cargo run -p sm-arith  --bin arith_frops_fixed_gen");
            println!("       cargo run -p sm-binary --bin binary_basic_frops_fixed_gen");
            println!("       cargo run -p sm-binary --bin binary_extension_frops_fixed_gen");
            println!("  3. cargo test -p sm-arith -p sm-binary  # offset/accessibility tests");
        }
        Command::AddHi0 { input } => {
            addstats::run(&input)?;
        }
        Command::AddHi0Dist { input } => {
            addstats::run_dist(&input)?;
        }
        Command::NonfropDist { input } => {
            addstats::run_nonfrop(&input)?;
        }
        Command::HiClass { input, include_frops } => {
            addstats::run_hiclass(&input, !include_frops)?;
        }
        Command::AsmOriginal { output } => {
            codegen::generate_original_asm(&output)?;
            println!("Wrote original-FROPS macros to {}", output.display());
        }
        Command::TableHi => {
            addstats::run_table_hi();
        }
        Command::TableByOp => {
            addstats::run_table_by_op();
        }
        Command::TablePartition { r } => {
            addstats::run_table_partition(r);
        }
    }
    Ok(())
}

fn ingest(common: &CommonArgs, cfg: Config) -> Result<Aggregator, Box<dyn std::error::Error>> {
    let mut agg = Aggregator::new(cfg.low_cap);
    agg.ingest_dir(&common.input)?;
    if agg.trailing_bytes > 0 {
        eprintln!(
            "warning: {} trailing byte(s) ignored (not a multiple of {} per record)",
            agg.trailing_bytes,
            ingest::RECORD_SIZE
        );
    }
    Ok(agg)
}

fn print_summary(agg: &Aggregator, prop: &optimize::Proposal) {
    let cfg = &prop.config;
    println!(
        "Files: {}  Records: {}  FROPS-candidate: {}  Skipped: {}",
        agg.files,
        agg.records,
        agg.records - agg.skipped_non_frops,
        agg.skipped_non_frops
    );
    println!("Table rows: {} used", prop.total_table_rows);
    println!(
        "Paid (padded to 2^{}): {} / {} (max) = {} partition(s); waste {} ({:.1}%)",
        cfg.partition_bits,
        prop.total_paid_rows,
        cfg.max_table,
        prop.total_paid_rows >> cfg.partition_bits,
        prop.total_waste,
        if prop.total_paid_rows == 0 {
            0.0
        } else {
            100.0 * prop.total_waste as f64 / prop.total_paid_rows as f64
        }
    );
    let (b, p) = if cfg.padding {
        (prop.baseline_pad.total, prop.proposed_pad.total)
    } else {
        (prop.baseline_nopad.total, prop.proposed_nopad.total)
    };
    let saved = b.saturating_sub(p);
    let pct = if b == 0 { 0.0 } else { 100.0 * saved as f64 / b as f64 };
    println!(
        "Area ({}): baseline {} -> proposed {}  (saved {:.2}%)",
        if cfg.padding { "padding" } else { "no padding" },
        b,
        p,
        pct
    );

    // Comparison vs the current in-tree FROPS.
    let cur = if cfg.padding { prop.current_pad.total } else { prop.current_nopad.total };
    let total_occ: u64 = prop.op_total.values().sum();
    let cur_cov: u64 = prop.current_covered.values().sum();
    let prop_cov: u64 = prop.op_covered.values().sum();
    let cov = |c: u64| if total_occ == 0 { 0.0 } else { 100.0 * c as f64 / total_occ as f64 };
    println!(
        "vs current FROPS: rows {} -> {} | coverage {:.2}% -> {:.2}% | area {} -> {} ({})",
        prop.current_total_table_rows,
        prop.total_table_rows,
        cov(cur_cov),
        cov(prop_cov),
        cur,
        p,
        if p <= cur { "proposed better" } else { "current better" }
    );

    // Membership-test cost: regions per opcode.
    let mut per_op: HashMap<u8, usize> = HashMap::new();
    for s in &prop.selected {
        *per_op.entry(s.info.code).or_default() += 1;
    }
    let total: usize = per_op.values().sum();
    let max = per_op.values().copied().max().unwrap_or(0);
    println!("Regions: {total} total, max {max}/op (cap {})", cfg.max_regions_per_op);
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
