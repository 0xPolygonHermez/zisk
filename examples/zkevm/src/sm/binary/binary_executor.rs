// use crate::config::Config;
// use crate::sm::pols_generated::commit_pols::BinaryCommitPols;
// use crate::utils::USING_PROVER_FORK_NAMESPACE;
use rayon::prelude::*;

use math::FieldElement;
use num_bigint::BigUint;

#[derive(Debug)]
pub struct BinaryAction {
    pub a: BigUint,
    pub b: BigUint,
    pub c: BigUint,
    pub opcode: u64,
    pub type_: u64,
}

pub struct BinaryExecutor<T: FieldElement> {
    // config: &'a Config,
    num_rows: usize,
    factors: Vec<Vec<T>>,
    reset: Vec<T>,
}

impl<T: FieldElement> BinaryExecutor<T> {
    pub const REGISTERS_NUM: usize = 8;
    pub const BYTES_PER_REGISTER: usize = 4;
    pub const STEPS_PER_REGISTER: usize = 2;
    pub const STEPS: usize = Self::STEPS_PER_REGISTER * Self::REGISTERS_NUM;
    pub const LATCH_SIZE: usize = Self::REGISTERS_NUM * Self::STEPS_PER_REGISTER;

    pub fn new(num_rows: usize) -> Self { //(fr: &'a mut Goldilocks, config: &'a Config) -> Self {
        Self {
            // config,
            num_rows,
            factors: Self::build_factors(Self::REGISTERS_NUM, num_rows),
            reset: Self::build_reset(num_rows),
        }
    }

    fn build_factors(registers_num: usize, num_rows: usize) -> Vec<Vec<T>> {
        let mut factors = vec![vec![T::default(); num_rows]; registers_num];

        let shifted = T::from(1u32 << 16);
    
        factors.par_iter_mut().for_each(|factors| {
            for j in 0..registers_num {
                for index in 0..num_rows {
                    let k = (index / Self::STEPS_PER_REGISTER) % Self::REGISTERS_NUM;
                    if j == k {
                        factors[index] = if index % 2 == 0 { T::ONE } else { shifted };
                    } else {
                        factors[index] = T::ZERO;
                    }
                }
            }
        });

        factors
    }

    fn build_reset(num_rows: usize) -> Vec<T> {
        let mut reset = vec![T::default(); num_rows];

        for i in 0..num_rows {
            reset[i] = if (i % Self::STEPS) == 0 { T::ONE } else { T::ZERO };
        }

        reset
    }

    pub fn execute(&mut self, inputs: &mut Vec<BinaryAction>/*, pols: &mut BinaryCommitPols*/) {
        let t_256 = T::from(256u32);

        // Check that we have enough room in polynomials
        assert!(inputs.len() * Self::LATCH_SIZE <= self.num_rows, "BinaryExecutor::execute() Too many Binary entries={} > N/LATCH_SIZE={}", inputs.len(), self.num_rows / Self::LATCH_SIZE);

        // Local array of N uint32
        let mut c0_temp: Vec<u32> = vec![0; self.num_rows];
        // TODO! Be sure capacity() is the way to check we've reserved enough memory
        assert!(c0_temp.capacity() >= self.num_rows, "BinaryExecutor::execute() failed allocating memory for c0Temp");

        for i in 0..inputs.len() {
            let opcode = inputs[i].opcode;
            let reset4 = if opcode == 8 { 1 } else { 0 };
            let mut previous_are_lt4 = T::ZERO;

            // TODO! Check if it has to be LE or BE
            let a_bytes = inputs[i].a.to_bytes_be();
            let b_bytes = inputs[i].b.to_bytes_be();
            let c_bytes = inputs[i].c.to_bytes_be();

            for j in 0..Self::STEPS {
                let last = j == (Self::STEPS - 1);
                let index = i * Self::STEPS + j;
                pols.opcode[index] = T::from(opcode);

                let c_in = T::ZERO;
                let mut c_out = T::ZERO;
                let reset = j == 0;
                let mut use_carry = false;
                let mut use_previous_are_lt4 = 0;

                for k in 0..2 {
                    c_in = if k == 0 { pols.c_in[index] } else { c_out };

                    let byte_a = a_bytes[j * 2 + k];
                    let byte_b = b_bytes[j * 2 + k];
                    let byte_c = c_bytes[j * 2 + k];
                    let reset_byte = reset && k == 0;
                    let last_byte = last && k == 1;
                    pols.free_in_a[k][index] = T::from(byte_a);
                    pols.free_in_b[k][index] = T::from(byte_b);
                    pols.free_in_c[k][index] = T::from(byte_c);

                    // ONLY for carry, ge4 management
                    match opcode {
                        // ADD (OPCODE = 0)
                        0 => {
                            let sum = byte_a + byte_b + fr.to_u64(c_in);
                            c_out = T::from(sum >> 8);
                        }
                        // SUB (OPCODE = 1)
                        1 => {
                            if byte_a.wrapping_sub(fr.to_u64(c_in)) >= byte_b as i64 {
                                c_out = T::ZERO;
                            } else {
                                c_out = T::ONE;
                            }
                        }
                        // LT (OPCODE = 2)
                        // LT4 (OPCODE = 8)
                        2 | 8 => {
                            if reset_byte {
                                pols.free_in_c[0][index] = T::from(c_bytes[Self::STEPS - 1]);
                            }
                    
                            if byte_a < byte_b {
                                c_out = T::ONE;
                            } else if byte_a == byte_b {
                                c_out = c_in;
                            } else {
                                c_out = T::ZERO;
                            }
                    
                            if last_byte {
                                if opcode == 2 || c_out == T::ZERO {
                                    use_carry = true;
                                    pols.free_in_c[1][index] = T::from(c_bytes[0]);
                                } else {
                                    use_previous_are_lt4 = 1;
                                    pols.free_in_c[1][index] = c_out;
                                }
                            }
                        }
                        // SLT (OPCODE = 3)
                        3 => {
                            use_carry = last;
                            if reset_byte {
                                pols.free_in_c[0][index] = T::from(c_bytes[Self::STEPS - 1]);
                            }
                    
                            if last_byte {
                                let sig_a = byte_a >> 7;
                                let sig_b = byte_b >> 7;
                    
                                if sig_a > sig_b {
                                    c_out = T::ONE;
                                } else if sig_a < sig_b {
                                    c_out = T::ZERO;
                                } else {
                                    if byte_a < byte_b {
                                        c_out = T::ONE;
                                    } else if byte_a == byte_b {
                                        c_out = c_in;
                                    } else {
                                        c_out = T::ZERO;
                                    }
                                    pols.free_in_c[k][index] = T::from(c_bytes[0]);
                                }
                            } else {
                                if byte_a < byte_b {
                                    c_out = T::ONE;
                                } else if byte_a == byte_b {
                                    c_out = c_in;
                                } else {
                                    c_out = T::ZERO;
                                }
                            }
                        }
                        // EQ (OPCODE = 4)
                        4 => {
                            if reset_byte {
                                pols.free_in_c[k][index] = T::from(c_bytes[Self::STEPS - 1]);
                            }
                    
                            if byte_a == byte_b && fr.is_zero(c_in) {
                                c_out = T::ZERO;
                            } else {
                                c_out = T::ONE;
                            }
                    
                            if last_byte {
                                use_carry = true;
                                c_out = if fr.is_zero(c_out) { T::ONE } else { T::ZERO };
                                pols.free_in_c[k][index] = T::from(c_bytes[0]);
                            }
                        }
                        // AND (OPCODE = 5)
                        5 => {
                            if byte_c == 0 && fr.is_zero(c_in) {
                                c_out = T::ZERO;
                            } else {
                                c_out = T::ONE;
                            }
                        }
                        _ => {
                            c_in = T::ZERO;
                            c_out = T::ZERO;
                        }
                    }
                }
            }

            if inputs[i].type_ == 1 {
                pols.result_bin_op[(i + 1) * Self::STEPS % self.num_rows] = T::ONE;
            } else if inputs[i].type_ == 2 {
                pols.result_valid_range[(i + 1) * Self::STEPS % self.num_rows] = T::ONE;
            }

            for index in (inputs.len() * Self::STEPS)..self.num_rows {
                let next_index = (index + 1) % self.num_rows;
                let reset = if (index % Self::STEPS) == 0 { T::ZERO } else { T::ONE };
                
                pols.a[0][next_index] = pols.a[0][index] * reset + pols.free_in_a[0][index] * self.factors[0][index] + t_256 * pols.free_in_a[1][index] * self.factors[0][index];
                pols.b[0][next_index] = pols.b[0][index] * reset + pols.free_in_b[0][index] * self.factors[0][index] + t_256 * pols.free_in_b[1][index] * self.factors[0][index];
            
                let c0_temp = pols.c[0][index] * reset + pols.free_in_c[0][index] * self.factors[0][index] + t_256 * pols.free_in_c[1][index] * self.factors[0][index];
            
                pols.c[0][next_index] = pols.use_carry[index] * (pols.c_out[index] - c0_temp) + c0_temp;
            
                for j in 1..Self::REGISTERS_NUM {
                    pols.a[j][next_index] = pols.a[j][index] * reset + pols.free_in_a[0][index] * self.factors[j][index] + t_256 * pols.free_in_a[1][index] * self.factors[j][index];
                    pols.b[j][next_index] = pols.b[j][index] * reset + pols.free_in_b[0][index] * self.factors[j][index] + t_256 * pols.free_in_b[1][index] * self.factors[j][index];
                    pols.c[j][next_index] = pols.c[j][index] * reset + pols.free_in_c[0][index] * self.factors[j][index] + t_256 * pols.free_in_c[1][index] * self.factors[j][index];
                }
            }            
        }        
    }    
}
