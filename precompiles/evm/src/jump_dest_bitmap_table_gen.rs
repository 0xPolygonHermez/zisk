//! Writes the fixed columns of `JumpDestBitmapTable` to a binary file.
//!
//! The PIL declares the table but does not build it: its 138953 rows out of an
//! interpreted loop cost minutes of compile time, so the columns come from here
//! through `#pragma extern_fixed_file`, the same route the FrequentOps tables
//! take.

use clap::{Arg, Command};
use std::error::Error;

use proofman_fields::{Field, Goldilocks, PrimeField64};
use proofman_common::{write_fixed_cols_bin, FixedColsInfo};

use zisk_precomp_helpers::{build_jump_dest_bitmap_table, JUMP_DEST_BITMAP_TABLE_ROWS};

type F = Goldilocks;

const AIRGROUP_NAME: &str = "Zisk";
const AIR_NAME: &str = "JumpDestBitmapTable";
const DEFAULT_FILE: &str = "precompiles/evm/src/jump_dest_bitmap_table_fixed.bin";

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("jump-dest-bitmap-table-gen")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("output_path")
                .help("Path to the output binary file")
                .default_value(DEFAULT_FILE),
        )
        .get_matches();

    let output_file = matches.get_one::<String>("output").unwrap().as_str();

    let rows = build_jump_dest_bitmap_table();
    assert_eq!(
        rows.len(),
        JUMP_DEST_BITMAP_TABLE_ROWS,
        "row count must match the one declared in jump_dest_bitmap_table.pil"
    );
    println!("Generating {} rows for {AIR_NAME}", rows.len());

    let mut packed = vec![F::ZERO; rows.len()];
    let mut bytes_used = vec![F::ZERO; rows.len()];
    let mut bitmap_byte = vec![F::ZERO; rows.len()];
    let mut state_out = vec![F::ZERO; rows.len()];

    for (index, row) in rows.iter().enumerate() {
        packed[index] = F::from_u64(row.state_cdata4_mem_load);
        bytes_used[index] = F::from_u64(row.bytes_used);
        bitmap_byte[index] = F::from_u64(row.bitmap_byte);
        state_out[index] = F::from_u64(row.state_out);
    }

    let packed = FixedColsInfo::new(&format!("{AIR_NAME}.STATE_CDATA4_MEM_LOAD"), None, packed);
    let bytes_used = FixedColsInfo::new(&format!("{AIR_NAME}.BYTES_USED"), None, bytes_used);
    let bitmap_byte = FixedColsInfo::new(&format!("{AIR_NAME}.BITMAP_BYTE"), None, bitmap_byte);
    let state_out = FixedColsInfo::new(&format!("{AIR_NAME}.STATE_OUT"), None, state_out);

    write_fixed_cols_bin(
        output_file,
        AIRGROUP_NAME,
        AIR_NAME,
        JUMP_DEST_BITMAP_TABLE_ROWS as u64,
        &mut [packed, bytes_used, bitmap_byte, state_out],
    );
    println!(
        "STATE_CDATA4_MEM_LOAD, BYTES_USED, BITMAP_BYTE and STATE_OUT written to {output_file}"
    );

    Ok(())
}
