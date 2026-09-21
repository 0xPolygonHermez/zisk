//! `pilout-constraints` — read the AIR constraints out of a `.pilout`.
//!
//! The `.pilout` the pil2 compiler emits is the only place the constraints
//! exist in resolved form: expressions with their sharing intact, columns under
//! the names the PIL gave them, and a `debugLine` pointing back at the `.pil`
//! line each constraint came from. This reads that and writes it out as a JSON
//! IR, as Lean definitions, or as a listing.
//!
//! See README.md.

mod extract;
mod ir;
mod lean;
mod print;
mod text;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};

use extract::{AirRef, Pilout};
use ir::Ir;

#[derive(Parser)]
#[command(name = "pilout-constraints", version, about = "Extract AIR constraints from a .pilout")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List the AIRs in a pilout, with their shape and constraint count.
    List(ListArgs),
    /// Write the constraints of an AIR as a JSON IR.
    Extract(RenderArgs),
    /// Write the constraints of an AIR as Lean definitions.
    Lean(RenderArgs),
    /// Write the constraints of an AIR as a readable listing.
    Text(RenderArgs),
}

#[derive(Args)]
struct ListArgs {
    /// Path to the `.pilout`.
    #[arg(short, long)]
    pilout: PathBuf,
}

#[derive(Args)]
struct RenderArgs {
    /// Path to the `.pilout`.
    #[arg(short, long)]
    pilout: PathBuf,

    /// AIR name, case-insensitive. Defaults to Main.
    #[arg(short, long, default_value = "Main", conflicts_with = "all")]
    air: String,

    /// Airgroup name, case-insensitive. Defaults to the only one there is.
    #[arg(short = 'g', long)]
    airgroup: Option<String>,

    /// Every AIR in the pilout, one file each.
    #[arg(long)]
    all: bool,

    /// Output directory. Without it, a single AIR goes to stdout.
    #[arg(short, long)]
    out: Option<PathBuf>,

    /// Module the generated Lean imports its prelude from (`lean` only).
    #[arg(long, default_value = "Pil")]
    module: String,
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::List(args) => list(&args),
        Command::Extract(args) => render(&args, Format::Json),
        Command::Lean(args) => render(&args, Format::Lean),
        Command::Text(args) => render(&args, Format::Text),
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Format {
    Json,
    Lean,
    Text,
}

impl Format {
    fn extension(self) -> &'static str {
        match self {
            Format::Json => "json",
            Format::Lean => "lean",
            Format::Text => "txt",
        }
    }
}

fn list(args: &ListArgs) -> Result<()> {
    let pilout = load(&args.pilout)?;
    println!(
        "{} — {} airgroups, {} symbols, {} stages",
        pilout.pilout.name.clone().unwrap_or_default(),
        pilout.pilout.air_groups.len(),
        pilout.pilout.symbols.len(),
        pilout.pilout.num_stages(),
    );
    for air_ref in pilout.airs() {
        let air = &pilout.pilout.air_groups[air_ref.airgroup_id].airs[air_ref.air_id];
        println!(
            "  [{}][{}] {}/{}  rows {}  stages {:?}  fixed {}  expressions {}  constraints {}",
            air_ref.airgroup_id,
            air_ref.air_id,
            air_ref.airgroup,
            air_ref.air,
            air.num_rows.unwrap_or(0),
            air.stage_widths,
            air.fixed_cols.len(),
            air.expressions.len(),
            air.constraints.len(),
        );
    }
    Ok(())
}

fn render(args: &RenderArgs, format: Format) -> Result<()> {
    let pilout = load(&args.pilout)?;
    let air = if args.all { None } else { Some(args.air.as_str()) };
    let selected = pilout.select(args.airgroup.as_deref(), air)?;

    if selected.len() > 1 && args.out.is_none() {
        anyhow::bail!("{} AIRs selected: pass --out <dir> to write them", selected.len());
    }

    // The prelude is shared by every AIR, so it is written once.
    if format == Format::Lean {
        if let Some(dir) = &args.out {
            let path = dir.join(&args.module).join("Prelude.lean");
            write(&path, lean::PRELUDE)?;
            eprintln!("wrote {}", path.display());
        }
    }

    for air_ref in &selected {
        let ir = pilout.extract(air_ref)?;
        let rendered = match format {
            Format::Json => serde_json::to_string_pretty(&ir)? + "\n",
            Format::Lean => lean::render(&ir, &args.module),
            Format::Text => text::render(&ir),
        };
        match &args.out {
            None => print!("{rendered}"),
            Some(dir) => {
                let path = output_path(dir, &args.module, air_ref, format);
                write(&path, &rendered)?;
                eprintln!("wrote {} ({} constraints)", path.display(), ir.constraints.len());
                summarize(&ir);
            }
        }
    }

    // Lake needs a root module listing the library's modules. It is rebuilt
    // from what is on disk rather than from this run's selection, so
    // generating one AIR does not drop the AIRs generated before it.
    if format == Format::Lean {
        if let Some(dir) = &args.out {
            let path = write_lean_root(dir, &args.module)?;
            eprintln!("wrote {}", path.display());
        }
    }
    Ok(())
}

/// `<out>/<module>.lean`, importing every module in `<out>/<module>/`.
fn write_lean_root(dir: &Path, module: &str) -> Result<PathBuf> {
    let module_dir = dir.join(module);
    let mut modules: Vec<String> = std::fs::read_dir(&module_dir)
        .with_context(|| format!("listing {}", module_dir.display()))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|e| e == "lean"))
        .filter_map(|entry| entry.path().file_stem().map(|s| s.to_string_lossy().to_string()))
        .collect();
    modules.sort();

    let mut contents = String::new();
    for name in modules {
        contents.push_str(&format!("import {module}.{name}\n"));
    }
    let path = dir.join(format!("{module}.lean"));
    write(&path, &contents)?;
    Ok(path)
}

/// Lean output is a module tree (`<out>/<module>/<Air>.lean`) so that `lake`
/// can pick it up; the other formats are flat files.
fn output_path(dir: &Path, module: &str, air: &AirRef, format: Format) -> PathBuf {
    let file = format!("{}.{}", air.air, format.extension());
    match format {
        Format::Lean => dir.join(module).join(file),
        _ => dir.join(file),
    }
}

fn write(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(path, contents).with_context(|| format!("writing {}", path.display()))
}

fn summarize(ir: &Ir) {
    eprintln!(
        "  {} expressions reachable of {}, {} named intermediates, {} witness columns",
        ir.reachable().len(),
        ir.expressions.len(),
        ir.columns.intermediates.len(),
        ir.columns.witness.len(),
    );
}

fn load(path: &Path) -> Result<Pilout> {
    Pilout::load(path.to_str().context("pilout path is not valid UTF-8")?)
}
