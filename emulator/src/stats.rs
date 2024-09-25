use zisk_core::{ZiskInst, ZiskOperations, REG_FIRST, REG_LAST};

const AREA_PER_SEC: f64 = 1000000_f64;
const COST_MEM: f64 = 10_f64 / AREA_PER_SEC;
const COST_MEMA_R1: f64 = 20_f64 / AREA_PER_SEC;
const COST_MEMA_R2: f64 = 40_f64 / AREA_PER_SEC;
const COST_MEMA_W1: f64 = 40_f64 / AREA_PER_SEC;
const COST_MEMA_W2: f64 = 80_f64 / AREA_PER_SEC;
const COST_USUAL: f64 = 8_f64 / AREA_PER_SEC;
const COST_STEP: f64 = 50_f64 / AREA_PER_SEC;

#[derive(Default, Debug, Clone)]
struct MemoryOperations {
    mread_a: u64, // Aligned
    mwrite_a: u64,
    mread_na1: u64, // Not aligned
    mwrite_na1: u64,
    mread_na2: u64, // Not aligned
    mwrite_na2: u64,
}

#[derive(Default, Debug, Clone)]
struct RegistryOperations {
    reads: u64,
    writes: u64,
}

#[derive(Debug, Clone)]
pub struct Stats {
    mops: MemoryOperations,
    rops: RegistryOperations,
    usual: u64,
    steps: u64,
    ops: [u64; 256],
}

/// Default constructor for Stats structure
impl Default for Stats {
    fn default() -> Self {
        Self {
            mops: MemoryOperations::default(),
            rops: RegistryOperations::default(),
            usual: 0,
            steps: 0,
            ops: [0; 256],
        }
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
        if (REG_FIRST..=REG_LAST).contains(&address) {
            self.rops.reads += 1;
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
        if (REG_FIRST..=REG_LAST).contains(&address) {
            self.rops.writes += 1;
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
        (instruction.op != 0xF1) && instruction.is_external_op && (a < 256) && (b < 256)
    }

    pub fn report(&self) -> String {
        const AREA_PER_SEC: f64 = 1000000_f64;
        // Result of his function
        let mut output = String::new();

        output += "Cost definitions:\n";
        output += &format!("    AREA_PER_SEC: {} steps\n", AREA_PER_SEC);
        output += &format!("    COST_MEMA_R1: {:02} sec\n", COST_MEMA_R1);
        output += &format!("    COST_MEMA_R2: {:02} sec\n", COST_MEMA_R2);
        output += &format!("    COST_MEMA_W1: {:02} sec\n", COST_MEMA_W1);
        output += &format!("    COST_MEMA_W2: {:02} sec\n", COST_MEMA_W2);
        output += &format!("    COST_USUAL: {:02} sec\n", COST_USUAL);
        output += &format!("    COST_STEP: {:02} sec\n", COST_STEP);

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

        let operations = ZiskOperations::new();
        let mut total_opcodes: u64 = 0;
        let mut opcode_steps: [u64; 256] = [0; 256];
        let mut total_opcode_steps: u64 = 0;
        let mut opcode_cost: [f64; 256] = [0_f64; 256];
        let mut total_opcode_cost: f64 = 0_f64;
        for opcode in 0..256 {
            // Skip zero counters
            if self.ops[opcode] == 0 {
                continue;
            }

            // Increase total
            total_opcodes += self.ops[opcode];

            // Get the Zisk instruction corresponding to this opcode
            let op8 = opcode as u8;
            let inst =
                operations.op_from_code.get(&op8).expect("Opcode not found in ZiskOperations");

            // Increase steps
            opcode_steps[opcode] += inst.s;
            total_opcode_steps += inst.s;

            // Increse cost
            let value = self.ops[opcode] as f64;
            opcode_cost[opcode] += value * inst.s as f64 / AREA_PER_SEC;
            total_opcode_cost += value * inst.s as f64 / AREA_PER_SEC;
        }

        let cost_usual = self.usual as f64 * COST_USUAL;
        let cost_main = self.steps as f64 * COST_STEP;

        let total_cost = cost_main + cost_mem + cost_mem_align + total_opcode_cost + cost_usual;

        output += &format!("\nTotal Cost: {:.2} sec\n", total_cost);
        output += &format!("    Main Cost: {:.2} sec {} steps\n", cost_main, self.steps);
        output += &format!("    Mem Cost: {:.2} sec {} steps\n", cost_mem, total_mem_ops);
        output +=
            &format!("    Mem Align: {:.2} sec {} steps\n", cost_mem_align, total_mem_align_steps);
        output += &format!(
            "    Opcodes: {:.2} sec {} steps ({} ops)\n",
            total_opcode_cost, total_opcode_steps, total_opcodes
        );
        output += &format!("    Usual: {:.2} sec {} steps\n", cost_usual, self.usual);
        output += &format!(
            "    Memory: {} a reads + {} na1 reads + {} na2 reads + {} a writes + {} na1 writes + {} na2 writes = {} reads + {} writes = {} r/w\n",
            self.mops.mread_a,
            self.mops.mread_na1,
            self.mops.mread_na2,
            self.mops.mwrite_a,
            self.mops.mwrite_na1,
            self.mops.mwrite_na2,
            self.mops.mread_a + self.mops.mread_na1 + self.mops.mread_na2,
            self.mops.mwrite_a + self.mops.mwrite_na1 + self.mops.mwrite_na2,
            self.mops.mread_a + self.mops.mread_na1 + self.mops.mread_na2 +
            self.mops.mwrite_a + self.mops.mwrite_na1 + self.mops.mwrite_na2,
        );
        output += &format!(
            "    Registy: {} reads + {} writes = {} r/w\n",
            self.rops.reads,
            self.rops.writes,
            self.rops.reads + self.rops.writes
        );

        output += "\nOpcodes:\n";

        for opcode in 0..256 {
            // Skip zero counters
            if self.ops[opcode] == 0 {
                continue;
            }

            // Get the Zisk instruction corresponding to this opcode
            let op8 = opcode as u8;
            let inst =
                operations.op_from_code.get(&op8).expect("Opcode not found in ZiskOperations");

            // Log opcode cost
            output += &format!(
                "    {}: {:.2} sec ({} steps/op) ({} ops)\n",
                inst.n, opcode_cost[opcode], opcode_steps[opcode], self.ops[opcode]
            );
        }

        output
    }
}
