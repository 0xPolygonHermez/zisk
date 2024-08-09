use zisk_core::{ZiskInst, ZiskOperations};

const AREA_PER_SEC: f64 = 1000000_f64;

const COST_MEM: f64 = 10_f64 / AREA_PER_SEC;
const COST_MEMA_R1: f64 = 20_f64 / AREA_PER_SEC;
const COST_MEMA_R2: f64 = COST_MEMA_R1 * 2_f64 / AREA_PER_SEC;
const COST_MEMA_W1: f64 = COST_MEMA_R1 * 2_f64 / AREA_PER_SEC;
const COST_MEMA_W2: f64 = COST_MEMA_R1 * 4_f64 / AREA_PER_SEC;

const COST_USUAL: f64 = 8_f64 / AREA_PER_SEC;

#[derive(Default, Debug, Clone)]
struct ConstOp {
    b: f64,
    b32: f64,
    a: f64,
    a32: f64,
}

const COST_OP: ConstOp = ConstOp {
    b: 32_f64 / AREA_PER_SEC,
    b32: 16_f64 / AREA_PER_SEC,
    a: 64_f64 / AREA_PER_SEC,
    a32: 32_f64 / AREA_PER_SEC,
};

const COST_STEP: f64 = 50_f64 / AREA_PER_SEC;

const TYPE_STRING: [[&str; 2]; 4] =
    [["b", "Binary"], ["b32", "Binary32"], ["a", "Arith"], ["a32", "Arith32"]];

#[derive(Default, Debug, Clone)]
struct MemoryOperations {
    mread_a: u64, // Aligned
    mwrite_a: u64,
    mread_na1: u64, // Not aligned
    mwrite_na1: u64,
    mread_na2: u64, // Not aligned
    mwrite_na2: u64,
}

#[derive(Debug, Clone)]
pub struct Stats {
    mops: MemoryOperations,
    usual: u64,
    steps: u64,
    ops: [u64; 256],
}

/// Default constructor for Stats structure
impl Default for Stats {
    fn default() -> Self {
        Self { mops: MemoryOperations::default(), usual: 0, steps: 0, ops: [0; 256] }
    }
}

impl Stats {
    pub fn on_memory_read(&mut self, address: u64, width: u64) {
        if (address % 8) != 0 {
            if ((address + width) / 8) < (address / 8) {
                self.mops.mread_na2 += 1;
            } else {
                self.mops.mread_na1 += 1;
            }
        } else {
            self.mops.mread_a += 1;
        }
    }

    pub fn on_memory_write(&mut self, address: u64, width: u64) {
        if (address % 8) != 0 {
            if ((address + width) / 8) < (address / 8) {
                self.mops.mwrite_na2 += 1;
            } else {
                self.mops.mwrite_na1 += 1;
            }
        } else {
            self.mops.mwrite_a += 1;
        }
    }

    pub fn on_steps(&mut self, steps: u64) {
        self.steps = steps;
    }

    pub fn on_op(&mut self, instruction: &ZiskInst, a: u64, b: u64) {
        if self.is_usual(instruction, a, b) {
            self.usual += 1;
        } else {
            self.ops[instruction.op as usize] += 1;
        }
    }

    fn is_usual(&self, instruction: &ZiskInst, a: u64, b: u64) -> bool {
        instruction.is_external_op && (a < 256) && (b < 256)
    }

    pub fn report(&self) -> String {
        let total_mem_ops = self.mops.mread_na1 +
            self.mops.mread_na2 +
            self.mops.mread_a +
            self.mops.mwrite_na1 +
            self.mops.mwrite_na2 +
            self.mops.mwrite_a;
        let total_mem_align_steps = self.mops.mread_na1 +
            self.mops.mread_na2 * 2 +
            self.mops.mwrite_na1 * 2 +
            self.mops.mwrite_na2 * 4;

        let cost_mem = total_mem_ops as f64 * COST_MEM;
        let cost_mem_align = self.mops.mread_na1 as f64 * COST_MEMA_R1 +
            self.mops.mread_na2 as f64 * COST_MEMA_R2 +
            self.mops.mwrite_na1 as f64 * COST_MEMA_W1 +
            self.mops.mwrite_na2 as f64 * COST_MEMA_W2;
        let mut total_op_type = ConstOp::default();

        let operations = ZiskOperations::new();
        for opcode in 0..256 {
            if self.ops[opcode] == 0 {
                continue;
            }
            let op8 = opcode as u8;
            let value = self.ops[opcode] as f64;
            let inst =
                operations.op_from_code.get(&op8).expect("Opcode not found in ZiskOperations");
            match inst.t {
                "i" => continue,
                "b" => total_op_type.b += value,
                "b32" => total_op_type.b32 += value,
                "a" => total_op_type.a += value,
                "a32" => total_op_type.a32 += value,
                _ => panic!(
                    "Stats::report() found invalid operation type opcode={} type={}",
                    opcode, inst.t
                ),
            }
        }

        let cost_bin = total_op_type.b * COST_OP.b;
        let cost_bin32 = total_op_type.b32 * COST_OP.b32;
        let cost_arith = total_op_type.a * COST_OP.a;
        let cost_arith32 = total_op_type.a32 * COST_OP.a32;
        let cost_usual = self.usual as f64 * COST_USUAL;
        let cost_main = self.steps as f64 * COST_STEP;

        let total_cost = cost_main +
            cost_mem +
            cost_mem_align +
            cost_bin +
            cost_bin32 +
            cost_arith +
            cost_arith32 +
            cost_usual;

        let mut output = String::new();
        output += &format!("Total Cost: {:.2}s\n", total_cost);
        output += &format!("    Main Cost: {:.2}s N: {}\n", cost_main, self.steps);
        output += &format!("    Mem Cost: {:.2}s N: {}\n", cost_mem, total_mem_ops);
        output += &format!("    Mem Align: {:.2}s N: {}\n", cost_mem_align, total_mem_align_steps);
        output += &format!("    Bin: {:.2}s N: {}\n", cost_bin, total_op_type.b);
        output += &format!("    Bin32: {:.2}s N: {}\n", cost_bin32, total_op_type.b32);
        output += &format!("    Arith: {:.2}s N: {}\n", cost_arith, total_op_type.a);
        output += &format!("    Arith32: {:.2}s N: {}\n", cost_arith32, total_op_type.a32);
        output += &format!("    Usual: {:.2}s N: {}\n", cost_usual, self.usual);

        for item in &TYPE_STRING {
            //for i in 0..4 {
            output += "\n";
            output += item[1];
            output += "\n";
            for opcode in 0..256 {
                if self.ops[opcode] == 0 {
                    continue;
                }
                let op8 = opcode as u8;
                let value = self.ops[opcode] as f64;
                let inst =
                    operations.op_from_code.get(&op8).expect("Opcode not found in ZiskOperations");
                if inst.t != item[0] {
                    continue;
                }
                output += &format!("    {}: {:.2}s N: {}\n", inst.n, value * COST_OP.b, value);
            }
        }

        output
    }
}
