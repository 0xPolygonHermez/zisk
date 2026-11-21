//! Writes the C columns of the `Bf16Table` instances, one binary per instance.
//!
//! The PIL declares A and B as sequences; only C[0..BLOCKS-1] comes from here,
//! through `#pragma extern_fixed_file`. Column names carry the AIR name, which
//! is the alias of each instance (`Bf16Add0`, `Bf16Add1`, ...), so the defaults
//! below must match the instantiation in zisk.pil.
//!
//! Size: each operation needs 2^32 values of 8 bytes, 32 GiB split across the
//! instances, so `--first`/`--count` allow writing only some of them.

use clap::{Parser, ValueEnum};
use rayon::prelude::*;
use std::error::Error;
use std::path::Path;

use proofman_common::{write_fixed_cols_bin, FixedColsInfo};
use proofman_fields::{Goldilocks, PrimeField64};

use zisk_sm_bf16::{Bf16Op, Bf16TableLayout};

type F = Goldilocks;

const AIRGROUP_NAME: &str = "Zisk";

#[derive(Clone, Copy, ValueEnum)]
enum OpArg {
    Add,
    Mul,
}

#[derive(Parser)]
#[command(
    name = "bf16-table-fixed-gen",
    version,
    about = "Generate the C columns of the Bf16Table instances"
)]
struct Args {
    /// Operation of the tables
    #[arg(long, value_enum)]
    op: OpArg,

    /// log2 of the rows of each instance (BF16_TABLE_N)
    #[arg(long, default_value_t = 22)]
    n_bits: u32,

    /// A values covered by each instance (BF16_TABLE_A_COUNT)
    #[arg(long, default_value_t = 4096)]
    a_count: usize,

    /// Output directory (BF16_TABLE_DIR)
    #[arg(long, default_value = "state-machines/bf16/fixed")]
    dir: String,

    /// First instance to write
    #[arg(long, default_value_t = 0)]
    first: usize,

    /// Instances to write, all from --first on by default
    #[arg(long)]
    count: Option<usize>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let op = match args.op {
        OpArg::Add => Bf16Op::Add,
        OpArg::Mul => Bf16Op::Mul,
    };
    let layout = Bf16TableLayout::new(1usize << args.n_bits, args.a_count)?;
    let count = args.count.unwrap_or(layout.instances().saturating_sub(args.first));
    if args.first + count > layout.instances() {
        return Err(format!(
            "instances {}..{} out of 0..{}",
            args.first,
            args.first + count,
            layout.instances()
        )
        .into());
    }
    std::fs::create_dir_all(&args.dir)?;

    println!(
        "bf16 {}: N = 2^{}, A_count = {}, {} blocks, {} instances, writing {}..{} to {}",
        op.name(),
        args.n_bits,
        layout.a_count,
        layout.blocks(),
        layout.instances(),
        args.first,
        args.first + count,
        args.dir
    );

    for instance in args.first..args.first + count {
        let air_name = format!("{}{instance}", op.air_prefix());
        let file = Path::new(&args.dir).join(format!("bf16_{}_{instance}_fixed.bin", op.name()));

        let mut cols: Vec<FixedColsInfo<F>> = (0..layout.blocks())
            .into_par_iter()
            .map(|block| {
                let values = (0..layout.n)
                    .map(|row| {
                        let (a, b) = layout.operands(instance, block, row);
                        F::from_u64(op.apply(a, b) as u64)
                    })
                    .collect();
                FixedColsInfo::new(&format!("{air_name}.C"), Some(vec![block as u64]), values)
            })
            .collect();

        let file = file.to_str().ok_or("output path is not UTF-8")?;
        write_fixed_cols_bin(file, AIRGROUP_NAME, &air_name, layout.n as u64, &mut cols);
        println!("  {air_name}: from_A = {:#06x} -> {file}", layout.from_a(instance));
    }
    Ok(())
}
