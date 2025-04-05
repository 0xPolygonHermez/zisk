use super::{bits_to_byte, print_bits, Gate, GateConfig, GateOperation, PinId, PinSource};

const BITRATE: u64 = 1088; // Number of bits absorbed in the sponge

#[derive(Debug)]
pub struct GateState {
    pub gate_config: GateConfig,

    // References
    pub next_ref: u64,
    pub sin_refs: Vec<u64>,
    pub sout_refs: Vec<u64>,

    // Ordered list of operations to implement the circuit
    pub program: Vec<u64>,

    // Ordered list of gates
    pub gates: Vec<Gate>,

    // Counters
    pub xors: u64,
    pub andps: u64,
    pub ors: u64,
    pub ands: u64,
    pub chs: u64,
    pub majs: u64,
    pub adds: u64,
}

impl GateState {
    pub fn new(gate_config: GateConfig) -> Self {
        // Preallocate vectors with appropriate sizes
        let sin_refs = vec![0; gate_config.sin_ref_number as usize];
        let sout_refs = vec![0; gate_config.sout_ref_number as usize];
        let gates = vec![Gate::new(); gate_config.max_refs as usize];

        let mut state = Self {
            gate_config,
            next_ref: 0,
            sin_refs,
            sout_refs,
            program: Vec::new(),
            gates,
            xors: 0,
            andps: 0,
            ors: 0,
            ands: 0,
            chs: 0,
            majs: 0,
            adds: 0,
        };

        state.reset_bits_and_counters();
        state
    }

    pub fn reset_bits_and_counters(&mut self) {
        // Reset all gates
        for gate in &mut self.gates {
            gate.reset();
        }

        // Initialize SinRefs
        for i in 0..self.gate_config.sin_ref_number {
            let rel_dis = i % self.gate_config.sin_ref_group_by;
            self.sin_refs[i as usize] = if rel_dis == 0 {
                self.gate_config.sin_ref0
                    + self.gate_config.sin_ref_distance * i / self.gate_config.sin_ref_group_by
            } else {
                self.sin_refs[(i - 1) as usize] + rel_dis
            };
        }

        // Initialize SoutRefs
        for i in 0..self.gate_config.sout_ref_number {
            let rel_dis = i % self.gate_config.sout_ref_group_by;
            self.sout_refs[i as usize] = if rel_dis == 0 {
                self.gate_config.sout_ref0
                    + self.gate_config.sout_ref_distance * i / self.gate_config.sin_ref_group_by
            } else {
                self.sout_refs[(i - 1) as usize] + rel_dis
            };
        }

        // Calculate the next reference (the first free slot)
        self.next_ref = self.gate_config.first_next_ref;

        // Reset counters
        self.xors = 0;
        self.andps = 0;
        self.ors = 0;
        self.ands = 0;
        self.chs = 0;
        self.majs = 0;
        self.adds = 0;

        // Init ZeroRef gate as XOR(0,1) = 1
        let z = self.gate_config.zero_ref as usize;
        self.gates[z].op = GateOperation::Xor;
        self.gates[z].pins[PinId::A].bit = 0;
        self.gates[z].pins[PinId::B].bit = 1;
        self.gates[z].pins[PinId::C].bit = 1;
    }

    // Set Rin data into bits array at SinRef0 position
    pub fn set_rin(&mut self, p_rin: &[u8]) {
        assert!(self.gate_config.sin_ref_number >= BITRATE);

        let mut ref_idx = 0;
        for i in 0..BITRATE {
            let rel_dis = i % self.gate_config.sin_ref_group_by;
            ref_idx = if rel_dis == 0 {
                self.gate_config.sin_ref0
                    + self.gate_config.sin_ref_distance * i / self.gate_config.sin_ref_group_by
            } else {
                ref_idx + rel_dis
            };
            self.gates[ref_idx as usize].pins[PinId::B].bit = p_rin[i as usize];
            self.gates[ref_idx as usize].pins[PinId::B].source = PinSource::External;
        }
    }

    // Mix Rin data with Sin data
    pub fn mix_rin(&mut self) {
        assert!(self.gate_config.sin_ref_number >= BITRATE);

        let mut ref_idx = 0;
        for i in 0..BITRATE {
            let rel_dis = i % self.gate_config.sin_ref_group_by;
            ref_idx = if rel_dis == 0 {
                self.gate_config.sin_ref0
                    + self.gate_config.sin_ref_distance * i / self.gate_config.sin_ref_group_by
            } else {
                ref_idx + rel_dis
            };
            self.xor(ref_idx, PinId::A, ref_idx, PinId::B, ref_idx);
        }
    }

    // Get 32-bytes output from SinRef0
    pub fn get_output(&self, p_output: &mut [u8]) {
        assert!(
            self.gate_config.sin_ref_number >= 32 * 8,
            "get_output called with insufficient sin_ref_number: {} < 256",
            self.gate_config.sin_ref_number
        );

        let mut ref_idx = 0;
        for i in 0..32 {
            let mut bytes = [0u8; 8];
            for j in 0..8 {
                // let ref_idx = self.gate_config.sin_ref0 + (i * 8 + j) * self.gate_config.sin_ref_distance;
                let rel_dis = (i * 8 + j) % self.gate_config.sin_ref_group_by;
                ref_idx = if rel_dis == 0 {
                    self.gate_config.sin_ref0
                        + self.gate_config.sin_ref_distance * (i * 8 + j)
                            / self.gate_config.sin_ref_group_by
                } else {
                    ref_idx + rel_dis
                };
                bytes[j as usize] = self.gates[ref_idx as usize].pins[PinId::A].bit;
            }
            bits_to_byte(&bytes, &mut p_output[i as usize]);
        }
    }

    /// Get a free reference (the next one) and increment counter
    pub fn get_free_ref(&mut self) -> u64 {
        assert!(self.next_ref < self.gate_config.max_refs);

        let result = self.next_ref;
        self.next_ref += 1;
        loop {
            // Skip ZeroRef
            if self.next_ref == self.gate_config.zero_ref {
                self.next_ref += 1;
                continue;
            }

            // Skip Sin gates
            if (self.next_ref >= self.gate_config.sin_ref0)
                && (self.next_ref
                    <= self.gate_config.sin_ref0
                        + (self.gate_config.sin_ref_number - self.gate_config.sin_ref_group_by)
                            * self.gate_config.sin_ref_distance
                            / self.gate_config.sin_ref_group_by
                        + (self.gate_config.sin_ref_group_by - 1))
                && ((self.next_ref - self.gate_config.sin_ref0) % self.gate_config.sin_ref_distance
                    < self.gate_config.sin_ref_group_by)
            {
                self.next_ref += 1;
                continue;
            }

            // Skip Sout gates
            if (self.next_ref >= self.gate_config.sout_ref0)
                && (self.next_ref
                    <= self.gate_config.sout_ref0
                        + (self.gate_config.sout_ref_number - self.gate_config.sout_ref_group_by)
                            * self.gate_config.sout_ref_distance
                            / self.gate_config.sout_ref_group_by
                        + (self.gate_config.sout_ref_group_by - 1))
                && ((self.next_ref - self.gate_config.sout_ref0)
                    % self.gate_config.sout_ref_distance
                    < self.gate_config.sout_ref_group_by)
            {
                self.next_ref += 1;
                continue;
            }

            break;
        }

        assert!(self.next_ref < self.gate_config.max_refs);
        result
    }

    /// Copy Sout references to Sin references
    pub fn copy_sout_refs_to_sin_refs(&mut self) {
        // Check sizes
        assert_eq!(self.gate_config.sin_ref_number, self.gate_config.sout_ref_number);

        // Copy SoutRefs into SinRefs
        self.sin_refs.copy_from_slice(&self.sout_refs);
    }

    /// Copy Sout data to Sin buffer, and reset
    pub fn copy_sout_to_sin_and_reset_refs(&mut self) {
        // Check sizes
        assert_eq!(self.gate_config.sin_ref_number, self.gate_config.sout_ref_number);

        // Collect Sout bits
        let mut local_sout = Vec::with_capacity(self.gate_config.sin_ref_number as usize);
        for i in 0..self.gate_config.sin_ref_number {
            let idx = self.sout_refs[i as usize] as usize;
            local_sout.push(self.gates[idx].pins[PinId::C].bit);
        }

        // Reset state
        self.reset_bits_and_counters();

        // Restore to Sin
        let mut idx = 0;
        for i in 0..self.gate_config.sin_ref_number {
            let rel_dis = i % self.gate_config.sin_ref_group_by;
            idx = if rel_dis == 0 {
                self.gate_config.sin_ref0
                    + self.gate_config.sin_ref_distance * i / self.gate_config.sin_ref_group_by
            } else {
                idx + rel_dis
            };

            self.gates[idx as usize].pins[PinId::A].bit = local_sout[i as usize];
        }
    }

    // Perform the gate operation
    pub fn op(
        &mut self,
        op: GateOperation,
        ref_a: u64,
        pin_a: PinId,
        ref_b: u64,
        pin_b: PinId,
        ref_c: u64,
    ) {
        // Safety checks
        assert!(ref_a < self.gate_config.max_refs);
        assert!(ref_b < self.gate_config.max_refs);
        assert!(ref_c < self.gate_config.max_refs);
        assert!(self.gates[ref_a as usize].pins[pin_a].bit <= 1);
        assert!(self.gates[ref_b as usize].pins[pin_b].bit <= 1);
        assert!(self.gates[ref_c as usize].pins[PinId::C].bit <= 1);

        // Update gate type
        self.gates[ref_c as usize].op = op;

        // Update input A
        self.gates[ref_c as usize].pins[PinId::A].source = PinSource::Wired;
        self.gates[ref_c as usize].pins[PinId::A].wired_ref = ref_a;
        self.gates[ref_c as usize].pins[PinId::A].wired_pin_id = pin_a;
        self.gates[ref_c as usize].pins[PinId::A].bit = self.gates[ref_a as usize].pins[pin_a].bit;

        // Update input B
        self.gates[ref_c as usize].pins[PinId::B].source = PinSource::Wired;
        self.gates[ref_c as usize].pins[PinId::B].wired_ref = ref_b;
        self.gates[ref_c as usize].pins[PinId::B].wired_pin_id = pin_b;
        self.gates[ref_c as usize].pins[PinId::B].bit = self.gates[ref_b as usize].pins[pin_b].bit;

        // Update output R
        self.gates[ref_c as usize].pins[PinId::C].source = PinSource::Gated;
        self.gates[ref_c as usize].pins[PinId::C].wired_ref = ref_c;

        // Calculate output based on operation
        match op {
            GateOperation::Xor => {
                self.gates[ref_c as usize].pins[PinId::C].bit =
                    self.gates[ref_a as usize].pins[pin_a].bit
                        ^ self.gates[ref_b as usize].pins[pin_b].bit;
                self.xors += 1;
            }
            GateOperation::Or => {
                self.gates[ref_c as usize].pins[PinId::C].bit =
                    self.gates[ref_a as usize].pins[pin_a].bit
                        | self.gates[ref_b as usize].pins[pin_b].bit;
                self.ors += 1;
            }
            GateOperation::Andp => {
                self.gates[ref_c as usize].pins[PinId::C].bit =
                    (1 - self.gates[ref_a as usize].pins[pin_a].bit)
                        & self.gates[ref_b as usize].pins[pin_b].bit;
                self.andps += 1;
            }
            GateOperation::And => {
                self.gates[ref_c as usize].pins[PinId::C].bit =
                    self.gates[ref_a as usize].pins[pin_a].bit
                        & self.gates[ref_b as usize].pins[pin_b].bit;
                self.ands += 1;
            }
            GateOperation::Ch => {
                self.gates[ref_c as usize].pins[PinId::C].bit =
                    (self.gates[ref_a as usize].pins[pin_a].bit
                        & self.gates[ref_b as usize].pins[pin_b].bit)
                        ^ ((1 - self.gates[ref_a as usize].pins[pin_a].bit)
                            & self.gates[ref_b as usize].pins[pin_b].bit);
                self.chs += 1;
            }
            GateOperation::Maj => {
                self.gates[ref_c as usize].pins[PinId::C].bit =
                    (self.gates[ref_a as usize].pins[pin_a].bit
                        & self.gates[ref_b as usize].pins[pin_b].bit)
                        ^ (self.gates[ref_a as usize].pins[pin_a].bit
                            & self.gates[ref_b as usize].pins[pin_b].bit)
                        ^ (self.gates[ref_b as usize].pins[pin_b].bit
                            & self.gates[ref_b as usize].pins[pin_b].bit);
                self.majs += 1;
            }
            GateOperation::Add => {
                self.gates[ref_c as usize].pins[PinId::C].bit =
                    self.gates[ref_a as usize].pins[pin_a].bit
                        + self.gates[ref_b as usize].pins[pin_b].bit;
                self.adds += 1;
            }
            _ => {
                panic!("GateState::op() called with unknown operation");
            }
        }

        // Update fan-out counters and connections
        if ref_a != ref_c {
            self.gates[ref_a as usize].pins[pin_a].fan_out += 1;
            self.gates[ref_a as usize].pins[pin_a].connections_to_input_a.push(ref_c);
        }

        if ref_b != ref_c {
            self.gates[ref_b as usize].pins[pin_b].fan_out += 1;
            self.gates[ref_b as usize].pins[pin_b].connections_to_input_b.push(ref_c);
        }

        // Add to program
        self.program.push(ref_c);
    }

    pub fn xor(&mut self, ref_a: u64, pin_a: PinId, ref_b: u64, pin_b: PinId, ref_c: u64) {
        self.op(GateOperation::Xor, ref_a, pin_a, ref_b, pin_b, ref_c);
    }

    pub fn xor_res(&mut self, ref_a: u64, ref_b: u64, ref_c: u64) {
        self.xor(ref_a, PinId::C, ref_b, PinId::C, ref_c);
    }

    pub fn andp(&mut self, ref_a: u64, pin_a: PinId, ref_b: u64, pin_b: PinId, ref_c: u64) {
        self.op(GateOperation::Andp, ref_a, pin_a, ref_b, pin_b, ref_c);
    }

    pub fn andp_res(&mut self, ref_a: u64, ref_b: u64, ref_c: u64) {
        self.andp(ref_a, PinId::C, ref_b, PinId::C, ref_c);
    }

    /// Prints operation statistics (development purposes)
    pub fn print_counters(&self) {
        let total_operations =
            self.xors + self.ors + self.andps + self.ands + self.chs + self.majs + self.adds;
        let total_f = total_operations as f64;

        println!("Operation statistics:");
        println!("==========================");
        println!("\txors      = {} = {:.2}%", self.xors, (self.xors as f64 * 100.0) / total_f);
        println!("\tors       = {} = {:.2}%", self.ors, (self.ors as f64 * 100.0) / total_f);
        println!("\tandps     = {} = {:.2}%", self.andps, (self.andps as f64 * 100.0) / total_f);
        println!("\tands      = {} = {:.2}%", self.ands, (self.ands as f64 * 100.0) / total_f);
        println!("\tchs       = {} = {:.2}%", self.chs, (self.chs as f64 * 100.0) / total_f);
        println!("\tmajs      = {} = {:.2}%", self.majs, (self.majs as f64 * 100.0) / total_f);
        println!("\tadds      = {} = {:.2}%", self.adds, (self.adds as f64 * 100.0) / total_f);
        println!("==========================");
        println!("Total       = {} = 100.00%", total_operations);
    }

    /// Prints reference bits (development purposes)
    pub fn print_refs(&self, refs: &[u64], name: &str) {
        // Collect bits safely
        let bits: Vec<u8> =
            refs.iter().map(|&ref_idx| self.gates[ref_idx as usize].pins[PinId::C].bit).collect();

        // Print the bits
        print_bits(&bits, name);
    }

    // // Generate a JSON object containing all data required for the executor script file
    // pub fn save_script_to_json(&self, _j: &mut Json) {
    //     // TODO: implement
    // }

    // // Generate a JSON object containing all a, b, r, and op polynomials values, with length 2^parity
    // pub fn save_pols_to_json(&self, _pols: &mut Json) {
    //     // TODO: implement
    // }

    // // Generate a JSON object containing all wired pin connections, with length 2^parity
    // pub fn save_connections_to_json(&self, _pols: &mut Json) {
    //     // TODO: implement
    // }
}
