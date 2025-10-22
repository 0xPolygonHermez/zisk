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
                precompiled_cost += inst.steps() * (*count);
            } else {
                ops_cost += inst.steps() * (*count);
            }
        }
    }
    (ops_cost, precompiled_cost)
}

pub fn get_ops_ranking(ops: &[u64]) -> Vec<(u8, u64, u64)> {
    let mut ranking: Vec<(u8, u64, u64)> = Vec::new();

    for (opcode, count) in ops.iter().enumerate() {
        if *count > 0 && opcode > 1 {
            if let Ok(inst) = ZiskOp::try_from_code(opcode as u8) {
                let cost = *count * inst.steps();
                ranking.push((opcode as u8, *count, cost));
            }
        }
    }

    // Ordenar por coste descendente (mayor coste primero)
    ranking.sort_by(|a, b| b.2.cmp(&a.2));
    ranking
}

/// Returns a vector of opcodes ranked by cost (1-based ranking)
pub fn get_ops_ranks(ops: &[u64]) -> [usize; 256] {
    let mut ranks = [0usize; 256];
    let ranking = get_ops_ranking(ops);

    for (rank, (opcode, _, _)) in ranking.iter().enumerate() {
        ranks[*opcode as usize] = rank + 1; // 1-based ranking
    }

    ranks
}
