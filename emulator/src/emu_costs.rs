use zisk_core::zisk_ops::ZiskOp;

pub const MEM_READ_COST: u64 = 16; // Dual RAM 28 cols => R+R, W+R
pub const MEM_WRITE_COST: u64 = 18; // Dual RAM 28 cols => R+R, W+R
pub const MEM_READ_BYTE_COST: u64 = 25;
pub const MEM_WRITE_BYTE_COST: u64 = 32;
pub const MEM_READ_UNALIGNED_1_COST: u64 = 53 * 2;
pub const MEM_READ_UNALIGNED_2_COST: u64 = 53 * 3;
pub const MEM_WRITE_UNALIGNED_1_COST: u64 = 53 * 3;
pub const MEM_WRITE_UNALIGNED_2_COST: u64 = 53 * 5;
pub const MEM_PRECOMPILE_READ_COST: u64 = MEM_READ_COST;
pub const MEM_PRECOMPILE_WRITE_COST: u64 = MEM_WRITE_COST;

pub const ROM_COST: usize = 21 << 21;
pub const TABLES_COST: usize = (55 + 35 + 29) << 21;
pub const BASE_COST: usize = ROM_COST + TABLES_COST;

pub const MAIN_COST: u64 = 68;

pub fn get_ops_costs(ops: &[u64]) -> (u64, u64) {
    let mut ops_cost = 0;
    let mut precompiled_cost = 0;
    for (op, count) in ops.iter().enumerate() {
        if let Ok(inst) = ZiskOp::try_from_code(op as u8) {
            if inst.input_size() > 0 {
                precompiled_cost += inst.cost() * (*count);
            } else {
                ops_cost += inst.cost() * (*count);
            }
        }
    }
    (ops_cost, precompiled_cost)
}
