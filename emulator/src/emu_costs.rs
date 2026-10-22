use zisk_core::zisk_ops::ZiskOp;

// IMPORTANT: changing any cost here, or how they are accumulated, requires increasing
// ZISK_COSTS_VERSION (core/src/zisk_ops_costs.rs).

// Mem: 87 cols, 4 lanes => ceil(87/4) = 22 cols per lane, the cost of a dual operation. Dual RAM: a
// lane proves R+R or W+R, so an access costs between 11 (paired) and 22 (alone). Not all dual lanes
// are full, so each access is charged a conservative factor of the dual operation cost: 0.5 for a
// read and 0.8 for a write (rounded up).
pub const MEM_DUAL_OP_COST: u64 = 87_u64.div_ceil(4);
pub const MEM_DUAL_READ_FACTOR_PCT: u64 = 50;
pub const MEM_DUAL_WRITE_FACTOR_PCT: u64 = 80;
pub const MEM_READ_COST: u64 = (MEM_DUAL_OP_COST * MEM_DUAL_READ_FACTOR_PCT).div_ceil(100);
pub const MEM_WRITE_COST: u64 = (MEM_DUAL_OP_COST * MEM_DUAL_WRITE_FACTOR_PCT).div_ceil(100);

pub const MEM_ALIGN_READ_BYTE_COST: u64 = 25;
pub const MEM_ALIGN_WRITE_BYTE_COST: u64 = 32;

pub const MEM_ALIGN_COST: u64 = 50;
pub const MEM_ALIGN_READ_UNALIGNED_1_COST: u64 = MEM_ALIGN_COST * 2;
pub const MEM_ALIGN_READ_UNALIGNED_2_COST: u64 = MEM_ALIGN_COST * 3;
pub const MEM_ALIGN_WRITE_UNALIGNED_1_COST: u64 = MEM_ALIGN_COST * 3;
pub const MEM_ALIGN_WRITE_UNALIGNED_2_COST: u64 = MEM_ALIGN_COST * 5;

pub const ROM_READ_COST: u64 = 22_u64.div_ceil(2); // RomData (2 lanes)
pub const ROM_INIT_COST: u64 = ROM_READ_COST;

pub const INPUT_READ_COST: u64 = 29;

pub const ROM_COST: usize = 16 << 22;
pub const TABLES_COST: usize = (65 + 39) << 21; // VirtualTableZisk1 + VirtualTableZisk0
pub const BASE_COST: usize = ROM_COST + TABLES_COST;

pub const MAIN_COST: u64 = 245_u64.div_ceil(4); // Main (4 lanes)

/// Reduced cost of an ADD that can be proven by BinaryAddHi (hi32(a)=hi32(c)=0, hi32(b) all-0 or
/// all-1). Cheaper than the regular ADD (BINARY_ADD_COST = 21). Used for the add_hi breakdown.
/// BinaryAddHiHuge (8 lanes); BinaryAddHi 15, BinaryAddHiLarge 13.
pub const BINARY_ADD_HI_COST: u64 = 93_u64.div_ceil(8);

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
