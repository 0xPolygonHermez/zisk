use std::vec;

use super::{
    bits_to_byte, bits_to_byte_msb, print_bits, Expression, ExpressionManager, Gate, GateConfig,
    GateOperation, PinId, PinSource,
};

#[derive(Debug)]
pub struct GateState {
    pub config: GateConfig,

    // References
    pub next_ref: u64,
    pub sin_refs: Vec<u64>,
    pub sout_refs: Vec<u64>,

    // Chronological list of operations to implement the circuit
    pub program: Vec<u64>,

    // Ordered list of gates
    pub gates: Vec<Gate>,

    // Expression manager
    pub expr_manager: Option<ExpressionManager>,

    // Counters
    pub xor2s: u64,
    pub nands: u64,
    pub xor3s: u64,
    pub xornands: u64,
}

// #[derive(Debug, Clone)]
// pub struct ResetEvent {
//     pub round: Option<usize>,
//     pub step: Option<String>,
//     pub substep: Option<String>,
//     pub operation: GateOperation,
//     pub ref_id: u64,
//     pub first_value: u64,
//     pub second_value: u64,
//     pub op_value: u64,
// }

impl GateState {
    pub fn new(config: GateConfig) -> Self {
        // Preallocate vectors with appropriate sizes
        let sin_refs = vec![0; config.sin_ref_number as usize];
        let sout_refs = vec![0; config.sout_ref_number as usize];
        let gates = vec![Gate::new(); config.max_refs as usize];

        let expr_manager =
            if config.handle_expressions { Some(ExpressionManager::new()) } else { None };
        let mut state = Self {
            config,
            next_ref: 0,
            sin_refs,
            sout_refs,
            program: Vec::new(),
            gates,
            expr_manager,
            xor2s: 0,
            nands: 0,
            xor3s: 0,
            xornands: 0,
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
        for i in 0..self.config.sin_ref_number {
            let group = i / self.config.sin_ref_group_by;
            let group_pos = i % self.config.sin_ref_group_by;
            self.sin_refs[i as usize] =
                self.config.sin_first_ref + group * self.config.sin_ref_distance + group_pos;
        }

        // Initialize SoutRefs
        for i in 0..self.config.sout_ref_number {
            let group = i / self.config.sout_ref_group_by;
            let group_pos = i % self.config.sout_ref_group_by;
            self.sout_refs[i as usize] =
                self.config.sout_first_ref + group * self.config.sout_ref_distance + group_pos;
        }

        // Calculate the next reference (the first free slot)
        self.next_ref = self.config.first_usable_ref;

        // Initialize input expressions
        if let Some(ref mut expr_manager) = self.expr_manager {
            for i in 0..self.config.sin_ref_number {
                let ref_id = self.config.sin_first_ref
                    + (i / self.config.sin_ref_group_by) * self.config.sin_ref_distance
                    + (i % self.config.sin_ref_group_by);
                expr_manager.set_expression(ref_id, Expression::input(ref_id));
            }
        }

        // Reset counters
        self.xor2s = 0;
        self.nands = 0;
        self.xor3s = 0;
        self.xornands = 0;

        // Init ZeroRef gate as XOR(0,1,0) = 1
        if let Some(zero_ref) = self.config.zero_ref {
            self.gates[zero_ref as usize].op = GateOperation::Xor2;
            self.gates[zero_ref as usize].pins[PinId::A].bit = 0;
            self.gates[zero_ref as usize].pins[PinId::B].bit = 1;
            self.gates[zero_ref as usize].pins[PinId::D].bit = 1;
            if let Some(ref mut expr_manager) = self.expr_manager {
                expr_manager.set_expression(zero_ref, Expression::ONE);
            }
        }
    }

    // Get 32-bytes output from the state input
    pub fn get_output(&self, output: &mut [u8], to_big_endian: bool) {
        assert!(self.config.sout_ref_number >= 256);

        for i in 0..32 {
            let mut bytes = [0u8; 8];
            for j in 0..8 {
                let group = (i * 8 + j) / self.config.sin_ref_group_by;
                let group_pos = (i * 8 + j) % self.config.sin_ref_group_by;
                let ref_idx =
                    self.config.sin_first_ref + group * self.config.sin_ref_distance + group_pos;
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
        assert!(self.next_ref < self.config.max_refs);

        let result = self.next_ref;

        // Update next reference for the next call
        self.next_ref += 1;
        let zero_ref = self.config.zero_ref;
        let sin_ref0 = self.config.sin_first_ref;
        let sin_ref_distance = self.config.sin_ref_distance;
        let sin_ref_group_by = self.config.sin_ref_group_by;
        let sin_last_ref = self.config.sin_last_ref;
        let sout_ref0 = self.config.sout_first_ref;
        let sout_ref_distance = self.config.sout_ref_distance;
        let sout_ref_group_by = self.config.sout_ref_group_by;
        let sout_last_ref = self.config.sout_last_ref;
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
        assert!(self.config.sin_ref_number >= self.config.sout_ref_number);

        // Copy SoutRefs into SinRefs
        self.sin_refs.copy_from_slice(&self.sout_refs);
    }

    /// Copy Sout data to Sin buffer, and reset
    pub fn copy_sout_to_sin_and_reset_refs(&mut self) {
        // Check sizes
        assert!(self.config.sin_ref_number >= self.config.sout_ref_number);

        // Collect Sout bits
        let mut local_sout = Vec::with_capacity(self.config.sout_ref_number as usize);
        for i in 0..self.config.sout_ref_number {
            let idx = self.sout_refs[i as usize] as usize;
            local_sout.push(self.gates[idx].pins[PinId::D].bit);
        }

        // Reset state
        self.reset_bits_and_counters();

        // Restore to Sin
        for i in 0..self.config.sout_ref_number {
            let group = i / self.config.sin_ref_group_by;
            let group_pos = i % self.config.sin_ref_group_by;
            let idx = self.config.sin_first_ref + group * self.config.sin_ref_distance + group_pos;
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
        assert!(ref_in1 < self.config.max_refs);
        assert!(ref_in2 < self.config.max_refs);
        assert!(ref_in3.is_none() || ref_in3.unwrap() < self.config.max_refs);
        assert!(ref_out < self.config.max_refs);
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

        // Get involved expressions
        // TODO: Implement for a third operand
        let (expr1, expr2) = if self.expr_manager.is_some() {
            // Threshold check before performing operation
            self.check_val_before_operation(op, ref_in1, ref_in2);

            let expr_manager = self.expr_manager.as_ref().unwrap();
            let expr1 =
                expr_manager.get_expression(ref_in1).cloned().unwrap_or(Expression::input(ref_in1));
            let expr2 =
                expr_manager.get_expression(ref_in2).cloned().unwrap_or(Expression::input(ref_in2));
            (expr1, expr2)
        } else {
            (Expression::ZERO, Expression::ZERO)
        };

        // Calculate output based on operation
        match op {
            GateOperation::Xor2 => {
                // If there are 2 inputs, in3 = 0 doesn't change the result
                self.gates[ref_out as usize].pins[PinId::D].bit = in1 ^ in2 ^ in3;
                self.xor2s += 1;

                if let Some(ref mut expr_manager) = self.expr_manager {
                    expr_manager.set_expression(ref_out, Expression::xor(expr1, expr2));
                }
            }

            GateOperation::Nand => {
                self.gates[ref_out as usize].pins[PinId::D].bit = (1 - in1) & in2;
                self.nands += 1;

                if let Some(ref mut expr_manager) = self.expr_manager {
                    expr_manager.set_expression(ref_out, Expression::nand(expr1, expr2));
                }
            }

            GateOperation::Xor3 | GateOperation::XorNand => {
                // Ensure there is a third input
                assert!(ref_in3.is_some() && pin_in3.is_some());

                let out = match op {
                    GateOperation::Xor3 => {
                        self.xor3s += 1;
                        in1 ^ in2 ^ in3
                    }
                    GateOperation::XorNand => {
                        self.xornands += 1;
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
            self.gates[ref_in1 as usize].pins[pin_in1].add_connection_to(PinId::A, ref_out);
        }

        if ref_in2 != ref_out {
            self.gates[ref_in2 as usize].pins[pin_in2].fan_out += 1;
            self.gates[ref_in2 as usize].pins[pin_in2].add_connection_to(PinId::B, ref_out);
        }

        if let (Some(ref_in3), Some(pin_in3)) = (ref_in3, pin_in3) {
            if ref_in3 != ref_out {
                self.gates[ref_in3 as usize].pins[pin_in3].fan_out += 1;
                self.gates[ref_in3 as usize].pins[pin_in3].add_connection_to(PinId::C, ref_out);
            }
        }

        // Add to program
        self.program.push(ref_out);
    }

    #[rustfmt::skip]
    pub fn xor2(&mut self, ref_in1: u64, pin_in1: PinId, ref_in2: u64, pin_in2: PinId, ref_out: u64) {
        self.op(GateOperation::Xor2, ref_in1, pin_in1, ref_in2, pin_in2, None, None, ref_out);
    }

    #[rustfmt::skip]
    #[allow(clippy::too_many_arguments)]
    pub fn xor3(&mut self, ref_in1: u64, pin_in1: PinId, ref_in2: u64, pin_in2: PinId, ref_in3: u64, pin_in3: PinId, ref_out: u64) {
        self.op(GateOperation::Xor3, ref_in1, pin_in1, ref_in2, pin_in2, Some(ref_in3), Some(pin_in3), ref_out);
    }

    #[rustfmt::skip]
    pub fn nand(&mut self, ref_in1: u64, pin_in1: PinId, ref_in2: u64, pin_in2: PinId, ref_out: u64) {
        self.op(GateOperation::Nand, ref_in1, pin_in1, ref_in2, pin_in2, None, None, ref_out);
    }

    #[rustfmt::skip]
    #[allow(clippy::too_many_arguments)]
    pub fn xor_nand(&mut self, ref_in1: u64, pin_in1: PinId, ref_in2: u64, pin_in2: PinId, ref_in3: u64, pin_in3: PinId, ref_out: u64) {
        self.op(GateOperation::XorNand, ref_in1, pin_in1, ref_in2, pin_in2, Some(ref_in3), Some(pin_in3), ref_out);
    }

    /// Prints operation statistics (development purposes)
    pub fn print_circuit_topology(&self) {
        println!("Number of input bits: {}", self.config.sin_ref_number);
        println!("Number of output bits: {}\n", self.config.sout_ref_number);

        let total_operations = self.xor2s + self.nands + self.xor3s + self.xornands;
        let total_f = total_operations as f64;

        println!("Gates statistics:");
        println!("==========================");
        if self.xor2s > 0 {
            println!(
                "   #XOR2     = {} = {:.2}%",
                self.xor2s,
                (self.xor2s as f64 * 100.0) / total_f
            );
        }
        if self.nands > 0 {
            println!(
                "   #NAND     = {} = {:.2}%",
                self.nands,
                (self.nands as f64 * 100.0) / total_f
            );
        }
        if self.xor3s > 0 {
            println!(
                "   #XOR3     = {} = {:.2}%",
                self.xor3s,
                (self.xor3s as f64 * 100.0) / total_f
            );
        }
        if self.xornands > 0 {
            println!(
                "   #XORNAND  = {} = {:.2}%",
                self.xornands,
                (self.xornands as f64 * 100.0) / total_f
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

// Expression handling methods
impl GateState {
    pub fn set_context(&mut self, round: usize, step: &str) {
        if let Some(ref mut expr_manager) = self.expr_manager {
            expr_manager.set_context(round, step);
        }
    }

    pub fn set_subcontext(&mut self, subcontext: &str) {
        if let Some(ref mut expr_manager) = self.expr_manager {
            expr_manager.set_subcontext(subcontext);
        }
    }

    pub fn get_expression(&self, ref_id: u64) -> Option<&Expression> {
        self.expr_manager.as_ref().and_then(|manager| manager.get_expression(ref_id))
    }

    /// Set expression for a reference
    pub fn set_expression(&mut self, ref_id: u64, expr: Expression) {
        if let Some(ref mut expr_manager) = self.expr_manager {
            expr_manager.set_expression(ref_id, expr);
        }
    }

    pub fn print_expression(&self, ref_id: u64) {
        if let Some(expr) = self.get_expression(ref_id) {
            println!("ref[{}] = {}", ref_id, expr);
        } else {
            println!("ref[{}] = undefined", ref_id);
        }
    }

    pub fn print_max_val_expression(&self, ref_id: u64) {
        if let Some(expr) = self.get_expression(ref_id) {
            println!("ref[{}] max value = {}", ref_id, expr.max_value());
        } else {
            println!("ref[{}] = undefined", ref_id);
        }
    }

    /// Check if performing an operation would exceed the threshold and reset if needed
    fn check_val_before_operation(&mut self, op: GateOperation, ref_in1: u64, ref_in2: u64) {
        let Some(ref mut expr_manager) = self.expr_manager else {
            return;
        };

        let Some(threshold) = self.config.reset_threshold else {
            return;
        };

        let Some(expr1) = expr_manager.get_expression(ref_in1).cloned() else {
            return;
        };
        let Some(expr2) = expr_manager.get_expression(ref_in2).cloned() else {
            return;
        };

        // Predict the result based on operation type
        let predicted_max = Self::eval_op_on_exprs(op, &expr1, &expr2);

        // If predicted result exceeds threshold, reset the largest operand(s)
        if predicted_max > threshold as u64 {
            // Create list of operands with their complexity
            let first_max = expr1.max_value();
            let second_max = expr2.max_value();
            let mut operands = vec![(ref_in1, first_max), (ref_in2, second_max)];

            // Sort by max_value descending to reset largest first
            operands.sort_by(|a, b| b.1.cmp(&a.1));

            // Reset operands until we're under threshold (or all are reset)
            for (ref_id, max_val) in operands {
                if max_val > 1 {
                    expr_manager.create_reset_expression(ref_id, None, Some(predicted_max));

                    // Re-evaluate if we're now under threshold
                    let expr1 = expr_manager.get_expression(ref_in1).cloned().unwrap();
                    let expr2 = expr_manager.get_expression(ref_in2).cloned().unwrap();
                    let new_predicted = Self::eval_op_on_exprs(op, &expr1, &expr2);
                    if new_predicted <= threshold as u64 {
                        break;
                    }
                }
            }
        }
    }

    /// Helper method to predict operation result after potential resets
    fn eval_op_on_exprs(op: GateOperation, expr1: &Expression, expr2: &Expression) -> u64 {
        match op {
            GateOperation::Xor2 => expr1.max_value() + expr2.max_value(),
            GateOperation::Nand => (expr1.max_value() + 1) * expr2.max_value(),
            _ => panic!("Unsupported operation for prediction"),
        }
    }

    pub fn create_proxy_expression(&mut self, ref_id: u64) {
        if let Some(ref mut expr_manager) = self.expr_manager {
            expr_manager.create_proxy_expression(ref_id);
        }
    }

    pub fn manual_reset_expression(&mut self, ref_id: u64, reason: Option<String>) {
        if let Some(ref mut expr_manager) = self.expr_manager {
            expr_manager.create_reset_expression(ref_id, reason, None);
        }
    }

    pub fn manual_im_expression(&mut self, ref_id: u64, reason: Option<String>) {
        if let Some(ref mut expr_manager) = self.expr_manager {
            expr_manager.create_im_expression(ref_id, reason, None);
        }
    }

    pub fn print_round_events(&self, round: usize, limit: Option<usize>) {
        if let Some(ref expr_manager) = self.expr_manager {
            expr_manager.print_round_events(round, limit);
        }
    }

    pub fn print_round_events_summary(&self, round: usize) {
        if let Some(ref expr_manager) = self.expr_manager {
            expr_manager.print_round_events(round, Some(0));
        }
    }

    pub fn print_expression_summary(&self) {
        if let Some(ref expr_manager) = self.expr_manager {
            expr_manager.print_expression_summary();
        }
    }
}
