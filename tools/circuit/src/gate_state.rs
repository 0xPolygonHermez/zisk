use super::{
    bits_to_byte, bits_to_byte_msb, print_bits, Gate, GateConfig, GateOperation, PinId, PinSource,
};

#[derive(Debug)]
pub struct GateState {
    pub gate_config: GateConfig,

    // References
    pub next_ref: u64,
    pub sin_refs: Vec<u64>,
    pub sout_refs: Vec<u64>,

    // Chronological list of operations to implement the circuit
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
    pub xorandps: u64,
}

impl GateState {
    pub fn new(gate_config: GateConfig) -> Self {
        // Preallocate vectors with appropriate sizes
        let sin_refs = vec![0; gate_config.sin_ref_number as usize];
        let sout_refs = vec![0; gate_config.sout_ref_number as usize];
        let gates = vec![Gate::default(); gate_config.max_refs as usize];

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
            xorandps: 0,
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
            let group = i / self.gate_config.sin_ref_group_by;
            let group_pos = i % self.gate_config.sin_ref_group_by;
            self.sin_refs[i as usize] = self.gate_config.sin_first_ref
                + group * self.gate_config.sin_ref_distance
                + group_pos;
        }

        // Initialize SoutRefs
        for i in 0..self.gate_config.sout_ref_number {
            let group = i / self.gate_config.sout_ref_group_by;
            let group_pos = i % self.gate_config.sout_ref_group_by;
            self.sout_refs[i as usize] = self.gate_config.sout_first_ref
                + group * self.gate_config.sout_ref_distance
                + group_pos;
        }

        // Calculate the next reference (the first free slot)
        self.next_ref = self.gate_config.first_usable_ref;

        // Reset counters
        self.xors = 0;
        self.andps = 0;
        self.ors = 0;
        self.ands = 0;
        self.chs = 0;
        self.majs = 0;
        self.adds = 0;

        // Init ZeroRef gate as XOR(0,1,0) = 1
        if let Some(z) = self.gate_config.zero_ref {
            self.gates[z as usize].op = GateOperation::Xor;
            self.gates[z as usize].pins[PinId::A].bit = 0;
            self.gates[z as usize].pins[PinId::B].bit = 1;
            self.gates[z as usize].pins[PinId::C].bit = 0;
            self.gates[z as usize].pins[PinId::D].bit = 1;
            self.gates[z as usize].pins[PinId::E].bit = 0;
        }
    }

    // Get 32-bytes output from the state input
    pub fn get_output(&self, output: &mut [u8], to_big_endian: bool) {
        assert!(self.gate_config.sout_ref_number >= 256);

        for i in 0..32 {
            let mut bytes = [0u8; 8];
            for j in 0..8 {
                let group = (i * 8 + j) / self.gate_config.sin_ref_group_by;
                let group_pos = (i * 8 + j) % self.gate_config.sin_ref_group_by;
                let ref_idx = self.gate_config.sin_first_ref
                    + group * self.gate_config.sin_ref_distance
                    + group_pos;
                bytes[j as usize] = self.gates[ref_idx as usize].pins[PinId::A].bit;
            }
            match to_big_endian {
                true => bits_to_byte_msb(&bytes, &mut output[i as usize]),
                false => bits_to_byte(&bytes, &mut output[i as usize]),
            }
        }
    }

    /// Get a free reference (the next one) and increment counter
    pub fn get_free_ref(&mut self) -> u64 {
        assert!(self.next_ref < self.gate_config.max_refs);

        let result = self.next_ref;

        // Update next reference for the next call
        self.next_ref += 1;
        let zero_ref = self.gate_config.zero_ref;
        let sin_ref0 = self.gate_config.sin_first_ref;
        let sin_ref_distance = self.gate_config.sin_ref_distance;
        let sin_ref_group_by = self.gate_config.sin_ref_group_by;
        let sin_last_ref = self.gate_config.sin_last_ref;
        let sout_ref0 = self.gate_config.sout_first_ref;
        let sout_ref_distance = self.gate_config.sout_ref_distance;
        let sout_ref_group_by = self.gate_config.sout_ref_group_by;
        let sout_last_ref = self.gate_config.sout_last_ref;
        while {
            let is_zero = match zero_ref {
                Some(z) => z == self.next_ref,
                None => false,
            };

            // If it coincides with the zero_ref or any sin_ref or sout_ref, skip it
            is_zero
                || (self.next_ref >= sin_ref0
                    && self.next_ref <= sin_last_ref
                    && (self.next_ref - sin_ref0) % sin_ref_distance < sin_ref_group_by)
                || (self.next_ref >= sout_ref0
                    && self.next_ref <= sout_last_ref
                    && (self.next_ref - sout_ref0) % sout_ref_distance < sout_ref_group_by)
        } {
            self.next_ref += 1;
        }

        result
    }

    /// Copy Sout references to Sin references
    pub fn copy_sout_refs_to_sin_refs(&mut self) {
        // Check sizes
        assert!(self.gate_config.sin_ref_number >= self.gate_config.sout_ref_number);

        // Copy SoutRefs into SinRefs
        self.sin_refs.copy_from_slice(&self.sout_refs);
    }

    /// Copy Sout data to Sin buffer, and reset
    pub fn copy_sout_to_sin_and_reset_refs(&mut self) {
        // Check sizes
        assert!(self.gate_config.sin_ref_number >= self.gate_config.sout_ref_number);

        // Collect Sout bits
        let mut local_sout = Vec::with_capacity(self.gate_config.sout_ref_number as usize);
        for i in 0..self.gate_config.sout_ref_number {
            let idx = self.sout_refs[i as usize] as usize;
            local_sout.push(self.gates[idx].pins[PinId::D].bit);
        }

        // Reset state
        self.reset_bits_and_counters();

        // Restore to Sin
        for i in 0..self.gate_config.sout_ref_number {
            let group = i / self.gate_config.sin_ref_group_by;
            let group_pos = i % self.gate_config.sin_ref_group_by;
            let idx = self.gate_config.sin_first_ref
                + group * self.gate_config.sin_ref_distance
                + group_pos;
            self.gates[idx as usize].pins[PinId::A].bit = local_sout[i as usize];
        }
    }

    // Perform the gate operation
    #[allow(clippy::too_many_arguments)]
    pub fn op(
        &mut self,
        op: GateOperation,
        ref_in1: u64,
        pin_in1: PinId,
        ref_in2: u64,
        pin_in2: PinId,
        ref_in3: Option<u64>,
        pin_in3: Option<PinId>,
        ref_out: u64,
    ) {
        // Get the input bits
        let in1 = self.gates[ref_in1 as usize].pins[pin_in1].bit;
        let in2 = self.gates[ref_in2 as usize].pins[pin_in2].bit;
        let in3 = if let (Some(ref_in3), Some(pin_in3)) = (ref_in3, pin_in3) {
            self.gates[ref_in3 as usize].pins[pin_in3].bit
        } else {
            0 // Default value for in3 if not provided
        };

        // Safety checks
        assert!(ref_in1 < self.gate_config.max_refs);
        assert!(ref_in2 < self.gate_config.max_refs);
        assert!(ref_in3.is_none() || ref_in3.unwrap() < self.gate_config.max_refs);
        assert!(ref_out < self.gate_config.max_refs);
        assert!(in1 <= 1);
        assert!(in2 <= 1);
        assert!(ref_in3.is_none() || in3 <= 1);
        assert!(self.gates[ref_out as usize].pins[PinId::D].bit <= 1);

        // Update gate type
        self.gates[ref_out as usize].op = op;

        // Update pin A
        self.gates[ref_out as usize].pins[PinId::A].source = PinSource::Wired;
        self.gates[ref_out as usize].pins[PinId::A].wired_ref = ref_in1;
        self.gates[ref_out as usize].pins[PinId::A].wired_pin_id = pin_in1;
        self.gates[ref_out as usize].pins[PinId::A].bit = in1;

        // Update pin B
        self.gates[ref_out as usize].pins[PinId::B].source = PinSource::Wired;
        self.gates[ref_out as usize].pins[PinId::B].wired_ref = ref_in2;
        self.gates[ref_out as usize].pins[PinId::B].wired_pin_id = pin_in2;
        self.gates[ref_out as usize].pins[PinId::B].bit = in2;

        // Update pin C
        if let (Some(ref_in3), Some(pin_in3)) = (ref_in3, pin_in3) {
            self.gates[ref_out as usize].pins[PinId::C].source = PinSource::Wired;
            self.gates[ref_out as usize].pins[PinId::C].wired_ref = ref_in3;
            self.gates[ref_out as usize].pins[PinId::C].wired_pin_id = pin_in3;
            self.gates[ref_out as usize].pins[PinId::C].bit = in3;
        }

        // Update output D
        self.gates[ref_out as usize].pins[PinId::D].source = PinSource::Gated;
        self.gates[ref_out as usize].pins[PinId::D].wired_ref = ref_out;

        // Calculate output based on operation
        match op {
            GateOperation::Xor => {
                // If there are 2 inputs, in3 = 0 doesn't change the result
                self.gates[ref_out as usize].pins[PinId::D].bit = in1 ^ in2 ^ in3;
                self.xors += 1;
            }
            GateOperation::Or => {
                self.gates[ref_out as usize].pins[PinId::D].bit = in1 | in2;
                self.ors += 1;
            }
            GateOperation::And => {
                self.gates[ref_out as usize].pins[PinId::D].bit = in1 & in2;
                self.ands += 1;
            }
            GateOperation::Andp => {
                self.gates[ref_out as usize].pins[PinId::D].bit = (1 - in1) & in2;
                self.andps += 1;
            }
            GateOperation::Add => {
                // Update output E
                self.gates[ref_out as usize].pins[PinId::E].source = PinSource::Gated;
                self.gates[ref_out as usize].pins[PinId::E].wired_ref = ref_out;

                self.gates[ref_out as usize].pins[PinId::D].bit = in1 ^ in2 ^ in3;
                self.gates[ref_out as usize].pins[PinId::E].bit =
                    (in1 & in2) | (in1 & in3) | (in2 & in3);
                self.adds += 1;
            }

            GateOperation::Ch | GateOperation::Maj | GateOperation::XorAndp => {
                // Ensure there is a third input
                assert!(ref_in3.is_some() && pin_in3.is_some());

                let out = match op {
                    GateOperation::Ch => {
                        self.chs += 1;
                        (in1 & in2) ^ ((1 - in1) & in3)
                    }
                    GateOperation::Maj => {
                        self.majs += 1;
                        (in1 & in2) ^ (in1 & in3) ^ (in2 & in3)
                    }
                    GateOperation::XorAndp => {
                        self.xorandps += 1;
                        in1 ^ ((1 - in2) & in3)
                    }
                    _ => unreachable!(),
                };

                self.gates[ref_out as usize].pins[PinId::D].bit = out;
            }
            _ => {
                panic!("op called with unknown operation");
            }
        }

        // Update fan-out counters and connections
        if ref_in1 != ref_out {
            self.gates[ref_in1 as usize].pins[pin_in1].fan_out += 1;
            self.gates[ref_in1 as usize].pins[pin_in1].connections_to_input_a.push(ref_out);
        }

        if ref_in2 != ref_out {
            self.gates[ref_in2 as usize].pins[pin_in2].fan_out += 1;
            self.gates[ref_in2 as usize].pins[pin_in2].connections_to_input_b.push(ref_out);
        }

        if let (Some(ref_in3), Some(pin_in3)) = (ref_in3, pin_in3) {
            if ref_in3 != ref_out {
                self.gates[ref_in3 as usize].pins[pin_in3].fan_out += 1;
                self.gates[ref_in3 as usize].pins[pin_in3].connections_to_input_c.push(ref_out);
            }
        }

        // Add to program
        self.program.push(ref_out);
    }

    #[rustfmt::skip]
    pub fn xor(&mut self, ref_in1: u64, pin_in1: PinId, ref_in2: u64, pin_in2: PinId, ref_out: u64) {
        self.op(GateOperation::Xor, ref_in1, pin_in1, ref_in2, pin_in2, None, None, ref_out);
    }

    #[rustfmt::skip]
    #[allow(clippy::too_many_arguments)]
    pub fn xor3(&mut self, ref_in1: u64, pin_in1: PinId, ref_in2: u64, pin_in2: PinId, ref_in3: u64, pin_in3: PinId, ref_out: u64) {
        self.op(GateOperation::Xor, ref_in1, pin_in1, ref_in2, pin_in2, Some(ref_in3), Some(pin_in3), ref_out);
    }

    #[rustfmt::skip]
    pub fn andp(&mut self, ref_in1: u64, pin_in1: PinId, ref_in2: u64, pin_in2: PinId, ref_out: u64) {
        self.op(GateOperation::Andp, ref_in1, pin_in1, ref_in2, pin_in2, None, None, ref_out);
    }

    #[rustfmt::skip]
    #[allow(clippy::too_many_arguments)]
    pub fn xor_andp(&mut self, ref_in1: u64, pin_in1: PinId, ref_in2: u64, pin_in2: PinId, ref_in3: u64, pin_in3: PinId, ref_out: u64) {
        self.op(GateOperation::XorAndp, ref_in1, pin_in1, ref_in2, pin_in2, Some(ref_in3), Some(pin_in3), ref_out);
    }

    pub fn or(&mut self, ref_in1: u64, pin_in1: PinId, ref_in2: u64, pin_in2: PinId, ref_out: u64) {
        self.op(GateOperation::Or, ref_in1, pin_in1, ref_in2, pin_in2, None, None, ref_out);
    }

    #[rustfmt::skip]
    pub fn and(&mut self, ref_in1: u64, pin_in1: PinId, ref_in2: u64, pin_in2: PinId, ref_out: u64) {
        self.op(GateOperation::And, ref_in1, pin_in1, ref_in2, pin_in2, None, None, ref_out);
    }

    #[rustfmt::skip]
    #[allow(clippy::too_many_arguments)]
    pub fn ch(&mut self, ref_in1: u64, pin_in1: PinId, ref_in2: u64, pin_in2: PinId, ref_in3: u64, pin_in3: PinId, ref_out: u64) {
        self.op(GateOperation::Ch, ref_in1, pin_in1, ref_in2, pin_in2, Some(ref_in3), Some(pin_in3), ref_out);
    }

    #[rustfmt::skip]
    #[allow(clippy::too_many_arguments)]
    pub fn maj(&mut self, ref_in1: u64, pin_in1: PinId, ref_in2: u64, pin_in2: PinId, ref_in3: u64, pin_in3: PinId, ref_out: u64) {
        self.op(GateOperation::Maj, ref_in1, pin_in1, ref_in2, pin_in2, Some(ref_in3), Some(pin_in3), ref_out);
    }

    #[rustfmt::skip]
    #[allow(clippy::too_many_arguments)]
    pub fn add(&mut self, ref_in1: u64, pin_in1: PinId, ref_in2: u64, pin_in2: PinId, ref_in3: u64, pin_in3: PinId, ref_out: u64) {
        self.op(GateOperation::Add, ref_in1, pin_in1, ref_in2, pin_in2, Some(ref_in3), Some(pin_in3), ref_out);
    }

    /// Prints operation statistics (development purposes)
    pub fn print_circuit_topology(&self) {
        println!("Number of inputs: {}", self.gate_config.sin_ref_number);
        println!("Number of outputs: {}\n", self.gate_config.sout_ref_number);

        let total_operations = self.xors
            + self.ors
            + self.andps
            + self.ands
            + self.chs
            + self.majs
            + self.adds
            + self.xorandps;
        let total_f = total_operations as f64;

        println!("Gates statistics:");
        println!("==========================");
        if self.xors > 0 {
            println!("   xors      = {} = {:.2}%", self.xors, (self.xors as f64 * 100.0) / total_f);
        }
        if self.ors > 0 {
            println!("   ors       = {} = {:.2}%", self.ors, (self.ors as f64 * 100.0) / total_f);
        }
        if self.ands > 0 {
            println!("   ands      = {} = {:.2}%", self.ands, (self.ands as f64 * 100.0) / total_f);
        }
        if self.andps > 0 {
            println!(
                "   andps     = {} = {:.2}%",
                self.andps,
                (self.andps as f64 * 100.0) / total_f
            );
        }
        if self.chs > 0 {
            println!("   chs       = {} = {:.2}%", self.chs, (self.chs as f64 * 100.0) / total_f);
        }
        if self.majs > 0 {
            println!("   majs      = {} = {:.2}%", self.majs, (self.majs as f64 * 100.0) / total_f);
        }
        if self.adds > 0 {
            println!("   adds      = {} = {:.2}%", self.adds, (self.adds as f64 * 100.0) / total_f);
        }
        if self.xorandps > 0 {
            println!(
                "   xorandps  = {} = {:.2}%",
                self.xorandps,
                (self.xorandps as f64 * 100.0) / total_f
            );
        }
        println!("--------------------------");
        println!("   Total     = {total_operations}");
        println!("==========================");
    }

    /// Prints reference bits (development purposes)
    pub fn print_refs(&self, refs: &[u64], name: &str) {
        // Collect bits safely
        let bits: Vec<u8> =
            refs.iter().map(|&ref_idx| self.gates[ref_idx as usize].pins[PinId::A].bit).collect();

        // Print the bits
        print_bits(&bits, name);
    }
}
