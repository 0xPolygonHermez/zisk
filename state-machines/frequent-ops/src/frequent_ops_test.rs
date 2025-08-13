mod frequent_ops_table;
use clap::{Arg, Command};
pub use frequent_ops_table::FrequentOpsTable;
use std::path::Path;
use zisk_core::zisk_ops::{OpType, ZiskOp};
/// Test binary for the FrequentOpsTable.
///
/// This program builds and analyzes the frequent operations table, printing statistics
/// such as the number of frequent operations and their distribution. Optionally, it can
/// read a binary file containing operation data using the `--store_ops_file` argument.
///
/// Usage:
///     cargo run --bin frequent_ops_test [-- --store_ops_file <STORE_OPS_FILE>]
///
/// Arguments:
///     --store_ops_file, -s   Optional path to a binary file containing operation data.
fn main() {
    let matches = Command::new("frequent_ops_test")
        .version("1.0")
        .about("Tests frequent operations table")
        .arg(
            Arg::new("store_ops_file")
                .short('s')
                .long("store_ops_file")
                .value_name("STORE_OPS_FILE")
                .help("Binary file containing operation data")
                .required(false),
        )
        .get_matches();

    let file_path = matches.get_one::<String>("store_ops_file");

    let mut fops = FrequentOpsTable::new();
    fops.build_table();
    let count = fops.count();
    let mut bits = 20;
    while count > (1 << bits) {
        bits += 1;
    }
    let size = 1 << bits;
    let available = size - count;
    println!(
        "Frequent Ops Count: {} (2^{} => available:{} {}%)",
        count,
        bits,
        available,
        (available * 100) / size
    );
    println!("Table:");
    // const LOW_VALUES: u64 = MAX_A_LOW_VALUE * MAX_B_LOW_VALUE;
    // println!("a b: {}", LOW_VALUES * ALL_256_CODES.len() as u64);
    for (op, count) in fops.get_top(200) {
        println!("{:<12} {:>8}", ZiskOp::try_from_code(op).unwrap().name(), count);
    }

    // Print and test the table offsets
    fops.print_table_offsets();
    fops.test_table_offsets();

    if let Some(file_path) = file_path {
        // Benchmark tests
        // Read binary file and benchmark table lookups
        if let Ok(file_data) = std::fs::read(file_path) {
            let mut operations = Vec::new();

            // Parse binary data: 1 byte op + 8 bytes a + 8 bytes b = 17 bytes per entry
            for chunk in file_data.chunks_exact(17) {
                let op = chunk[0];
                let a = u64::from_le_bytes(chunk[1..9].try_into().unwrap());
                let b = u64::from_le_bytes(chunk[9..17].try_into().unwrap());
                operations.push((op, a, b));
            }

            println!("Loaded {} operations from file", operations.len());

            // Benchmark table lookups
            let start = std::time::Instant::now();
            let mut found_count = 0;

            println!("Starting benchmark .....");
            for (op, a, b) in &operations {
                // println!("Checking operation: op={}, a=0x{:X}, b=0x{:X}", op, a, b);
                if FrequentOpsTable::is_frequent_op(*op, *a, *b) {
                    found_count += 1;
                }
            }

            let duration = start.elapsed();
            println!("Benchmark: {} operations processed in {:?}", operations.len(), duration);
            println!(
                "Found {} operations in table ({:.2}%)",
                found_count,
                (found_count as f64 / operations.len() as f64) * 100.0
            );
            println!("Average time per lookup: {:?}", duration / operations.len() as u32);

            println!("Starting test .....");
            println!("Generate full table .....");
            let full_table = fops.generate_full_table();
            println!("Testing full table .....");
            let mut ariths: u32 = 0;
            let mut bin_adds: u64 = 0;
            let mut bin_basics: u64 = 0;
            let mut bin_extends: u64 = 0;
            let mut tot_ariths: u32 = 0;
            let mut tot_bin_adds: u64 = 0;
            let mut tot_bin_basics: u64 = 0;
            let mut tot_bin_extends: u64 = 0;
            for (i, (op, a, b)) in operations.iter().enumerate() {
                // println!("Checking operation: op={}, a=0x{:X}, b=0x{:X}", op, a, b);
                let res = FrequentOpsTable::get_row(*op, *a, *b);
                let opcode = ZiskOp::try_from_code(*op).unwrap();
                if opcode == ZiskOp::Add {
                    tot_bin_adds += 1;
                } else {
                    match opcode.op_type() {
                        OpType::Arith => tot_ariths += 1,
                        OpType::ArithA32 => tot_ariths += 1,
                        OpType::ArithAm32 => tot_ariths += 1,
                        OpType::Binary => tot_bin_basics += 1,
                        OpType::BinaryE => tot_bin_extends += 1,
                        _ => (),
                    }
                }
                if FrequentOpsTable::is_frequent_op(*op, *a, *b) {
                    if let Some(row) = res {
                        if opcode == ZiskOp::Add {
                            bin_adds += 1;
                        } else {
                            match opcode.op_type() {
                                OpType::Arith => ariths += 1,
                                OpType::ArithA32 => ariths += 1,
                                OpType::ArithAm32 => ariths += 1,
                                OpType::Binary => bin_basics += 1,
                                OpType::BinaryE => bin_extends += 1,
                                _ => (),
                            }
                        }
                        assert!(row < full_table.len());
                        let (_op, _a, _b, _, _) = full_table[row];
                        assert!(
                        _op == *op && _a == *a && _b == *b,
                        "Value {}/{} {}% Row {} mismatch for op={}({}), a=0x{:X}(0x{:X}), b=0x{:X}(0x{:X})",
                        i + 1,
                        operations.len(),
                        (i + 1) * 100 / operations.len(),
                        row,
                        op,
                        _op,
                        a,
                        _a,
                        b,
                        _b,
                    );
                    } else {
                        panic!("Failed to get row for op={op}, a=0x{a:X}, b=0x{b:X}");
                    }
                }
            }
            let filename =
                Path::new(file_path).file_name().and_then(|n| n.to_str()).unwrap_or(file_path);

            println!(
                "BENCHMARK_CSV;{};{};{};{:.2}%;Arith;{};{};BinAdd;{};{};Bin;{};{};BinE;{};{}",
                filename,
                operations.len(),
                found_count,
                (found_count as f64 / operations.len() as f64) * 100.0,
                ariths,
                tot_ariths,
                bin_adds,
                tot_bin_adds,
                bin_basics,
                tot_bin_basics,
                bin_extends,
                tot_bin_extends,
            );
        } else {
            println!("Could not read store_ops file: {file_path}");
        }
    }
    println!("Done!");
}
