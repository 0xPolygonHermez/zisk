#![allow(dead_code)]
use clap::{Arg, Command};
use std::error::Error;

use fields::{Field, Goldilocks, PrimeField64};
use proofman_common::{write_fixed_cols_bin, FixedColsInfo};
use zisk_core::zisk_ops::ZiskOp;

type F = Goldilocks;

#[derive(Debug, Clone)]
pub struct FrequentOpsHelpers {
    pub table_by_op: [usize; 256],
    pub table_ops: Vec<Vec<[u64; 2]>>,
}

const FREQUENT_OP_EMPTY: usize = 256;

impl Default for FrequentOpsHelpers {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper methods for managing a table of FROPS.
///
/// This struct provides utilities to add, query, and analyze tables of operand pairs
/// associated with opcodes, as well as to generate and test lookup tables for frequent operations.
/// IMPORTANT: These helpers not are optimized for performance only used to create or test tables.
impl FrequentOpsHelpers {
    pub const NO_FROPS: usize = usize::MAX;
    /// Creates a new empty `FrequentOpsTableHelpers` instance.
    ///
    /// Initializes the table with no opcodes and an empty operations vector.
    ///
    /// # Returns
    ///
    /// A new `FrequentOpsTableHelpers` with all opcode slots marked as empty.
    pub fn new() -> Self {
        Self { table_by_op: [FREQUENT_OP_EMPTY; 256], table_ops: Vec::new() }
    }

    /// Adds a set of operand pairs to the table for a given opcode, either by moving or cloning the contents.
    ///
    /// # Arguments
    ///
    /// * `op` - The opcode to associate with the operand pairs.
    /// * `ops` - The vector of operand pairs to add.
    /// * `move_contents` - If true, moves the contents of `ops`; otherwise, clones them.
    pub fn add_ops(&mut self, op: u8, ops: &mut Vec<[u64; 2]>, move_contents: bool) {
        let mut index = self.table_by_op[op as usize];
        if index == FREQUENT_OP_EMPTY {
            index = self.table_ops.len();
            self.table_ops.push(Vec::new());
            self.table_by_op[op as usize] = index;
        }
        if move_contents {
            self.table_ops[index].append(ops);
        } else {
            self.table_ops[index].extend(ops.iter().cloned());
        }
    }

    /// Generates all possible operand pairs for low values.
    ///
    /// Creates a vector containing all combinations of operand pairs where the first operand (a)
    /// ranges from 0 to `max_a_low_value` (exclusive) and the second operand (b) ranges from 0 to
    /// `max_b_low_value` (exclusive).
    ///
    /// # Arguments
    ///
    /// * `max_a_low_value` - The exclusive upper bound for the first operand (a).
    /// * `max_b_low_value` - The exclusive upper bound for the second operand (b).
    ///
    /// # Returns
    ///
    /// A vector of operand pairs [a, b] for all combinations within the specified ranges.
    pub fn get_low_values_operations(
        &mut self,
        max_a_low_value: u64,
        max_b_low_value: u64,
    ) -> Vec<[u64; 2]> {
        let mut ops: Vec<[u64; 2]> = Vec::new();
        for i in 0..max_a_low_value {
            for j in 0..max_b_low_value {
                ops.push([i, j]);
            }
        }
        ops
    }

    /// Returns the total count of operations stored in the table.
    ///
    /// Sums up all operand pairs across all opcodes in the table.
    ///
    /// # Returns
    ///
    /// The total number of operand pairs stored across all opcodes.
    pub fn count(&self) -> usize {
        self.table_ops.iter().map(|ops| ops.len()).sum()
    }

    /// Prints the table offsets to stdout in a format suitable for inclusion in source code.
    ///
    /// Generates and prints the starting opcode and offset array as const declarations
    /// that can be copied into source code.
    pub fn print_table_offsets(&self) {
        let (start, offsets) = self.generate_table_offsets();
        println!("const OP_TABLE_OFFSETS_START: usize = {start};");
        println!("const OP_TABLE_OFFSETS: [usize; {}] = {:?};", offsets.len(), &offsets);
    }

    /// Generates opcode offset information for efficient table lookups.
    ///
    /// Creates an array of offsets indicating where each opcode's operand pairs
    /// begin in the flattened table. This enables O(1) lookup of opcode positions.
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// - `usize`: The starting opcode index (smallest opcode with operations)
    /// - `Vec<usize>`: Array of offsets for each opcode from start to end
    pub fn generate_table_offsets(&self) -> (usize, Vec<usize>) {
        let op_indexes = self.get_op_indexes();
        let mut offsets: [usize; 256] = [0; 256];
        let mut size: usize = 0;
        let mut start: usize = offsets.len();
        let mut end: usize = 0;
        for (op, index) in op_indexes.iter() {
            offsets[*op as usize] = size;
            if (*op as usize) < start {
                start = *op as usize;
            }
            if (*op as usize) > end {
                end = *op as usize;
            }
            size += self.table_ops[*index].len();
        }
        (start, offsets[start..end + 1].to_vec())
    }

    /// Generates a complete table with all operand pairs and their computed results.
    ///
    /// Creates a vector containing tuples with opcode, operand A, operand B,
    /// computed result C, and a boolean flag for each operation in the table.
    /// The result and flag are computed by calling the opcode's function with operands A and B.
    ///
    /// # Returns
    ///
    /// A vector of tuples where each tuple contains:
    /// - `u8`: The opcode
    /// - `u64`: Operand A
    /// - `u64`: Operand B  
    /// - `u64`: Computed result C
    /// - `bool`: Operation flag
    pub fn generate_full_table(&self) -> Vec<(u8, u64, u64, u64, bool)> {
        let op_indexes = self.get_op_indexes();
        let mut table: Vec<(u8, u64, u64, u64, bool)> = Vec::new();
        for (op, index) in op_indexes.iter() {
            table.extend(self.table_ops[*index].iter().map(|ab| {
                let (c, flag) = ZiskOp::try_from_code(*op).unwrap().call_ab(ab[0], ab[1]);
                (*op, ab[0], ab[1], c, flag)
            }));
        }
        table
    }

    /// Generates a simplified table with just opcode and operand pairs.
    ///
    /// Creates a vector containing tuples with opcode, operand A, and operand B
    /// for all operations in the table, without computing results.
    ///
    /// # Returns
    ///
    /// A vector of tuples where each tuple contains:
    /// - `u8`: The opcode
    /// - `u64`: Operand A
    /// - `u64`: Operand B
    pub fn generate_table(&self) -> Vec<(u8, u64, u64)> {
        let op_indexes = self.get_op_indexes();
        let mut table: Vec<(u8, u64, u64)> = Vec::new();
        for (op, index) in op_indexes.iter() {
            table.extend(self.table_ops[*index].iter().map(|ab| (*op, ab[0], ab[1])));
        }
        table
    }

    /// Gets a list of opcode-to-index mappings for all opcodes that have operations.
    ///
    /// Returns pairs of opcode and their corresponding index in the internal table_ops vector.
    /// Only includes opcodes that have been assigned operations (not empty slots).
    ///
    /// # Returns
    ///
    /// A vector of tuples where each tuple contains:
    /// - `u8`: The opcode
    /// - `usize`: The index in the internal table_ops vector
    pub fn get_op_indexes(&self) -> Vec<(u8, usize)> {
        self.table_by_op
            .iter()
            .enumerate()
            .filter(|(_, index)| *index != &FREQUENT_OP_EMPTY)
            .map(|(op, index)| (op as u8, *index))
            .collect()
    }

    /// Gets a list of opcodes and their operation counts.
    ///
    /// Returns pairs of opcode and the number of operand pairs stored for that opcode.
    /// Only includes opcodes that have operations assigned to them.
    ///
    /// # Returns
    ///
    /// A vector of tuples where each tuple contains:
    /// - `u8`: The opcode
    /// - `usize`: The number of operand pairs for this opcode
    pub fn get_list(&self) -> Vec<(u8, usize)> {
        self.table_by_op
            .iter()
            .enumerate()
            .filter(|(_, index)| *index != &FREQUENT_OP_EMPTY)
            .map(|(op, index)| (op as u8, self.table_ops[*index].len()))
            .collect()
    }

    /// Gets the top N opcodes sorted by operation count in descending order.
    ///
    /// Returns a list of opcodes and their operation counts, sorted from highest
    /// to lowest count and truncated to the specified number of entries.
    ///
    /// # Arguments
    ///
    /// * `num` - The maximum number of opcodes to return
    ///
    /// # Returns
    ///
    /// A vector of tuples where each tuple contains:
    /// - `u8`: The opcode
    /// - `usize`: The number of operand pairs for this opcode
    pub fn get_top(&self, num: usize) -> Vec<(u8, usize)> {
        let mut list = self.get_list();
        list.sort_by(|a, b| b.1.cmp(&a.1));
        list.truncate(num);
        list
    }

    /// Gets the top 10 opcodes sorted by operation count.
    ///
    /// Convenience method that returns the 10 opcodes with the highest
    /// number of operand pairs, sorted in descending order.
    ///
    /// # Returns
    ///
    /// A vector of tuples where each tuple contains:
    /// - `u8`: The opcode
    /// - `usize`: The number of operand pairs for this opcode
    pub fn get_top10(&self) -> Vec<(u8, usize)> {
        self.get_top(10)
    }

    pub fn generate_cmd(
        &self,
        airgroup_name: &'static str,
        air_name: &'static str,
        cmd_name: &'static str,
        default_file: &'static str,
        table: Vec<(u8, u64, u64, u64, bool)>,
        num_rows: usize,
    ) -> Result<(), Box<dyn Error>> {
        let matches = Command::new(cmd_name)
            .version(env!("CARGO_PKG_VERSION"))
            .arg(
                Arg::new("output")
                    .short('o')
                    .long("output")
                    .value_name("output_path")
                    .help("Path to the output binary file")
                    .default_value(default_file),
            )
            .get_matches();

        let output_file = matches.get_one::<String>("output").unwrap().as_str();

        // Generate the columns
        let (op, a0, a1, b0, b1, c0, c1, flag) = Self::cols_gen(table, num_rows);

        // Serialize the columns and write them to a binary file
        let op = FixedColsInfo::new(&format!("{air_name}.OP"), None, op);
        let a0 = FixedColsInfo::new(&format!("{air_name}.A"), Some(vec![0]), a0);
        let a1 = FixedColsInfo::new(&format!("{air_name}.A"), Some(vec![1]), a1);
        let b0 = FixedColsInfo::new(&format!("{air_name}.B"), Some(vec![0]), b0);
        let b1 = FixedColsInfo::new(&format!("{air_name}.B"), Some(vec![1]), b1);
        let c0 = FixedColsInfo::new(&format!("{air_name}.C"), Some(vec![0]), c0);
        let c1 = FixedColsInfo::new(&format!("{air_name}.C"), Some(vec![1]), c1);
        let flag = FixedColsInfo::new(&format!("{air_name}.FLAG"), None, flag);

        write_fixed_cols_bin(
            output_file,
            airgroup_name,
            air_name,
            num_rows as u64,
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
        println!("Generating columns for {num_rows} rows");
        let mut op = vec![F::ZERO; num_rows];
        let mut a0 = vec![F::ZERO; num_rows];
        let mut b0 = vec![F::ZERO; num_rows];
        let mut c0 = vec![F::ZERO; num_rows];
        let mut a1 = vec![F::ZERO; num_rows];
        let mut b1 = vec![F::ZERO; num_rows];
        let mut c1 = vec![F::ZERO; num_rows];
        let mut flag = vec![F::ZERO; num_rows];

        for (i, row) in table.iter().enumerate() {
            op[i] = F::from_u8(row.0);
            a0[i] = F::from_u32(row.1 as u32);
            b0[i] = F::from_u32(row.2 as u32);
            c0[i] = F::from_u32(row.3 as u32);
            a1[i] = F::from_u32((row.1 >> 32) as u32);
            b1[i] = F::from_u32((row.2 >> 32) as u32);
            c1[i] = F::from_u32((row.3 >> 32) as u32);
            flag[i] = F::from_bool(row.4);
        }
        println!(
            "Columns generated successfully op:{} a0:{} a1:{} b0:{} b1:{} c0:{} c1:{} flag:{}",
            op.len(),
            a0.len(),
            a1.len(),
            b0.len(),
            b1.len(),
            c0.len(),
            c1.len(),
            flag.len()
        );
        (op, a0, a1, b0, b1, c0, c1, flag)
    }

    /// Tests that all values in the given table are accessible through the lookup functions.
    ///
    /// Verifies that every entry in the provided table can be found using the `get_row` function
    /// and that the `is_frops` function correctly identifies them as frequent operations.
    ///
    /// # Arguments
    ///
    /// * `table` - The complete table of operations to verify
    /// * `is_frops` - Function to check if an operation is a frequent operation
    /// * `get_row` - Function to get the row index for a frequent operation or NO_FROPS
    ///
    /// # Panics
    ///
    /// Panics if any table entry cannot be found or is incorrectly identified.
    pub fn test_all_accessible_values(
        table: &[(u8, u64, u64, u64, bool)],
        is_frops: fn(u8, u64, u64) -> bool,
        get_row: fn(u8, u64, u64) -> usize,
    ) {
        let tests = table.iter().map(|(op, a, b, _c, _f)| (*op, *a, *b, true)).collect::<Vec<_>>();
        Self::check_tests(table, &tests, true, is_frops, get_row);
    }

    /// Tests the lookup functions with boundary values for low-value operand ranges.
    ///
    /// Generates test cases at the boundaries of the low-value ranges for all opcodes
    /// and verifies that the lookup functions correctly identify which operations
    /// should be found based on the provided list of low-value opcodes.
    ///
    /// # Arguments
    ///
    /// * `max_a_low_value` - The exclusive upper bound for operand A in low-value range
    /// * `max_b_low_value` - The exclusive upper bound for operand B in low-value range
    /// * `table` - The complete table of operations to verify against
    /// * `low_values_opcodes` - List of opcodes that should have low-value operations
    /// * `is_frops` - Function to check if an operation is a frequent operation
    /// * `get_row` - Function to get the row index for a frequent operation or NO_FROPS
    ///
    /// # Panics
    ///
    /// Panics if any boundary value lookup produces incorrect results.
    #[cfg(test)]
    pub fn test_low_values(
        max_a_low_value: u64,
        max_b_low_value: u64,
        table: &[(u8, u64, u64, u64, bool)],
        low_values_opcodes: &[u8],
        is_frops: fn(u8, u64, u64) -> bool,
        get_row: fn(u8, u64, u64) -> usize,
    ) {
        let mut tests: Vec<(u8, u64, u64, bool)> = Vec::new();

        for op in 0..=255 {
            let found = low_values_opcodes.contains(&op);
            tests.push((op, 0, max_b_low_value - 1, found));
            tests.push((op, 0, max_b_low_value - 2, found));
            tests.push((op, 1, max_b_low_value - 1, found));
            tests.push((op, 1, max_b_low_value - 2, found));

            tests.push((op, max_a_low_value - 1, 0, found));
            tests.push((op, max_a_low_value - 2, 0, found));
            tests.push((op, max_a_low_value - 1, 1, found));
            tests.push((op, max_a_low_value - 2, 1, found));

            tests.push((op, max_a_low_value - 1, max_b_low_value - 1, found));
            tests.push((op, max_a_low_value - 2, max_b_low_value - 2, found));
            tests.push((op, max_a_low_value - 1, max_b_low_value - 2, found));
            tests.push((op, max_a_low_value - 2, max_b_low_value - 1, found));

            tests.push((op, 0, max_b_low_value, false));
            tests.push((op, 0, max_b_low_value + 1, false));
            tests.push((op, 1, max_b_low_value, false));
            tests.push((op, 1, max_b_low_value + 1, false));

            tests.push((op, max_a_low_value, 0, false));
            tests.push((op, max_a_low_value + 1, 0, false));
            tests.push((op, max_a_low_value, 1, false));
            tests.push((op, max_a_low_value + 1, 1, false));

            tests.push((op, max_a_low_value - 1, max_b_low_value, false));
            tests.push((op, max_a_low_value, max_b_low_value - 1, false));

            tests.push((op, max_a_low_value, max_b_low_value, false));
            tests.push((op, max_a_low_value + 1, max_b_low_value + 1, false));
            tests.push((op, max_a_low_value, max_b_low_value + 1, false));
            tests.push((op, max_a_low_value + 1, max_b_low_value, false));
        }
        Self::check_tests(table, &tests, false, is_frops, get_row);
    }

    /// Executes a series of test cases against the lookup functions.
    ///
    /// Runs through a list of test cases, each specifying an opcode and operand pair,
    /// and verifies that the lookup functions (is_frops and get_row) behave correctly.
    /// Can operate in strict mode where all expected entries must be found exactly. Strict
    /// mode is used only when verifying all values, because if you test an outside limit value
    /// of group of conditions, this value could be found because it's produced by other group
    /// of conditions.
    ///
    /// # Arguments
    ///
    /// * `table` - The complete table of operations to verify against
    /// * `tests` - List of test cases, each containing (opcode, operand_a, operand_b, should_be_found)
    /// * `strict` - If true, enforces exact matches for all expected entries
    /// * `is_frops` - Function to check if an operation is a frequent operation
    /// * `get_row` - Function to get the row index for a frequent operation or NO_FROPS
    ///
    /// # Panics
    ///
    /// Panics if any test case fails validation or produces unexpected results.
    pub fn check_tests(
        table: &[(u8, u64, u64, u64, bool)],
        tests: &[(u8, u64, u64, bool)],
        strict: bool,
        is_frops: fn(u8, u64, u64) -> bool,
        get_row: fn(u8, u64, u64) -> usize,
    ) {
        for (itest, test) in tests.iter().enumerate() {
            let op_name =
                if let Ok(_op) = ZiskOp::try_from_code(test.0) { _op.name() } else { "?" };
            let index = get_row(test.0, test.1, test.2);
            if index != Self::NO_FROPS {
                if (!test.3 && strict)
                    || table[index].0 != test.0
                    || table[index].1 != test.1
                    || table[index].2 != test.2
                {
                    panic!(
                    "> #{} {1} 0x{2:X}({2}) 0x{3:X}({3}) {4} = {5} 0x{6:X}({6}) 0x{7:X}({7}) = 0x{8:X}({8}) F:{9} [\x1B[31mFAIL\x1B[0m]",
                    itest, op_name, test.1, test.2, test.3, ZiskOp::try_from_code(table[index].0).unwrap().name(), table[index].1, table[index].2, table[index].3, table[index].4 as u8
                );
                }
            } else if test.3 {
                panic!(
                    "> #{} {1} 0x{2:X}({2}) 0x{3:X}({3}) {4} = get_row NOT FOUND [\x1B[31mFAIL\x1B[0m]",
                    itest, op_name, test.1, test.2, test.3
                );
            } else if !is_frops(test.0, test.1, test.2) {
                panic!(
                    "> #{} {1} 0x{2:X}({2}) 0x{3:X}({3}) {4} = is_froops NOT FOUND [\x1B[31mFAIL\x1B[0m]",
                    itest, op_name, test.1, test.2, test.3
                );
            }
        }
        println!("Table Size: {}", table.len());
    }
}
