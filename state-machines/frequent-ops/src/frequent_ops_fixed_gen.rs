use std::error::Error;

use clap::{Arg, Command};

use sm_frequent_ops::FrequentOpsTable;
use zisk_pil::FrequentOpsFixed;

use fields::{Field, Goldilocks, PrimeField64};
use proofman_common::{write_fixed_cols_bin, FixedColsInfo};

type F = Goldilocks;

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("frequent_ops_fixed_gen")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("output_path")
                .help("Path to the output binary file")
                .default_value("state-machines/frequent-ops/src/frequent_ops_fixed.bin"),
        )
        .get_matches();

    let output_file = matches.get_one::<String>("output").unwrap();
    let num_rows: usize = FrequentOpsFixed::<usize>::NUM_ROWS;

    let mut fops = FrequentOpsTable::new();
    fops.build_table();
    let table = fops.generate_full_table();

    // Generate the columns
    let (op, a0, a1, b0, b1, c0, c1, flag) = cols_gen(table, num_rows);

    let n = 1 << 24;

    // Serialize the columns and write them to a binary file
    let op = FixedColsInfo::new("FrequentOps.OP", None, op);
    // let a = FixedColsInfo::new("FrequentOps.A", Some(vec![2]), a);
    // let b = FixedColsInfo::new("FrequentOps.B", Some(vec![2]), b);
    // let c = FixedColsInfo::new("FrequentOps.C", Some(vec![2]), c);
    // let a0 = FixedColsInfo::new("FrequentOps.A0", None, a0);
    // let b0 = FixedColsInfo::new("FrequentOps.B0", None, b0);
    // let c0 = FixedColsInfo::new("FrequentOps.C0", None, c0);
    // let a1 = FixedColsInfo::new("FrequentOps.A1", None, a1);
    // let b1 = FixedColsInfo::new("FrequentOps.B1", None, b1);
    // let c1 = FixedColsInfo::new("FrequentOps.C1", None, c1);
    // let flag = FixedColsInfo::new("FrequentOps.FLAG", None, flag);
    let a0 = FixedColsInfo::new("FrequentOps.A", Some(vec![0]), a0);
    let a1 = FixedColsInfo::new("FrequentOps.A", Some(vec![1]), a1);
    let b0 = FixedColsInfo::new("FrequentOps.B", Some(vec![0]), b0);
    let b1 = FixedColsInfo::new("FrequentOps.B", Some(vec![1]), b1);
    let c0 = FixedColsInfo::new("FrequentOps.C", Some(vec![0]), c0);
    let c1 = FixedColsInfo::new("FrequentOps.C", Some(vec![1]), c1);
    let flag = FixedColsInfo::new("FrequentOps.FLAG", None, flag);

    write_fixed_cols_bin(
        output_file,
        "Zisk",
        "FrequentOps",
        n as u64,
        &mut [op, a0, a1, b0, b1, c0, c1, flag],
    );
    println!("OP, A, B, C and FLAG columns written to {output_file}");

    Ok(())
}

#[allow(clippy::type_complexity)]
fn cols_gen(
    table: Vec<(u8, u64, u64, u64, bool)>,
    num_rows: usize,
) -> (Vec<F>, Vec<F>, Vec<F>, Vec<F>, Vec<F>, Vec<F>, Vec<F>, Vec<F>) {
    let mut op = vec![F::ZERO; num_rows];
    let mut a0 = vec![F::ZERO; num_rows];
    let mut b0 = vec![F::ZERO; num_rows];
    let mut c0 = vec![F::ZERO; num_rows];
    let mut a1 = vec![F::ZERO; num_rows];
    let mut b1 = vec![F::ZERO; num_rows];
    let mut c1 = vec![F::ZERO; num_rows];
    let mut flag = vec![F::ZERO; num_rows];

    for i in 0..num_rows {
        if i >= table.len() {
            continue;
        }
        op[i] = F::from_u8(table[i].0);
        a0[i] = F::from_u32(table[i].1 as u32);
        b0[i] = F::from_u32(table[i].2 as u32);
        c0[i] = F::from_u32(table[i].3 as u32);
        a1[i] = F::from_u32((table[i].1 >> 32) as u32);
        b1[i] = F::from_u32((table[i].2 >> 32) as u32);
        c1[i] = F::from_u32((table[i].3 >> 32) as u32);
        flag[i] = F::from_bool(table[i].4);
    }

    (op, a0, a1, b0, b1, c0, c1, flag)
}

// fn cols_gen(
//     table: Vec<(u8, [u64; 3], bool)>,
//     num_rows: usize,
// ) -> (Vec<F>, Vec<[F; 2]>, Vec<[F; 2]>, Vec<[F; 2]>, Vec<F>) {
//     let mut op = vec![F::ZERO; num_rows];
//     let mut a = vec![[F::ZERO; 2]; num_rows];
//     let mut b = vec![[F::ZERO; 2]; num_rows];
//     let mut c = vec![[F::ZERO; 2]; num_rows];
//     let mut flag = vec![F::ZERO; num_rows];

//     for i in 0..num_rows {
//         if i >= table.len() {
//             continue;
//         }
//         op[i] = F::from_u8(table[i].0);
//         a[i][0] = F::from_u32(table[i].1[0] as u32);
//         b[i][0] = F::from_u32(table[i].1[1] as u32);
//         c[i][0] = F::from_u32(table[i].1[2] as u32);
//         a[i][1] = F::from_u32((table[i].1[0] >> 32) as u32);
//         b[i][1] = F::from_u32((table[i].1[1] >> 32) as u32);
//         c[i][1] = F::from_u32((table[i].1[2] >> 32) as u32);
//         flag[i] = F::from_bool(table[i].2);
//     }

//     (op, a, b, c, flag)
// }
