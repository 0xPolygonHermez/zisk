use core::panic;
use std::cell::RefCell;

use crate::{bits_to_u32, u32_to_bits};

use super::{GateState, PinId};

#[derive(Debug, Clone, Copy)]
pub struct GateU32<'a> {
    pub state: &'a RefCell<GateState>,
    pub bits: [GateBit; 32],
}

#[derive(Debug, Clone, Copy)]
pub struct GateBit {
    pub ref_: u64,
    pub pin_id: PinId,
}

impl GateBit {
    pub fn new(ref_: u64) -> Self {
        GateBit { ref_, pin_id: PinId::A }
    }
}

impl<'a> GateU32<'a> {
    pub fn new(state: &'a RefCell<GateState>) -> Self {
        let default_ref = state.borrow().gate_config.zero_ref.unwrap();
        let mut gate = Self { state, bits: [GateBit::new(default_ref); 32] };
        gate.from_u32(0);
        gate
    }

    pub fn from_u32(&mut self, value: u32) {
        let bits = u32_to_bits(value);

        for (i, bit) in bits.iter().enumerate() {
            self.bits[i].pin_id = match bit {
                0 => PinId::A,
                1 => PinId::B,
                _ => panic!("Invalid bit value in from_u32"),
            };
        }
    }

    pub fn to_u32(&self) -> u32 {
        let mut bits = [0u8; 32];
        let state = self.state.borrow();
        for (i, bit) in self.bits.iter().enumerate() {
            let ref_ = bit.ref_ as usize;
            let pin_id = bit.pin_id;
            bits[i] = state.gates[ref_].pins[pin_id].bit;
        }

        bits_to_u32(&bits)
    }

    pub fn rotate_right(&mut self, pos: usize) {
        let mut rotated = [GateBit::new(self.state.borrow().gate_config.zero_ref.unwrap()); 32];
        for (i, rotated_bit) in rotated.iter_mut().enumerate() {
            *rotated_bit = self.bits[(i + pos) % 32];
        }
        self.bits = rotated;
    }

    pub fn shift_right(&mut self, pos: usize) {
        let mut shifted = [GateBit::new(self.state.borrow().gate_config.zero_ref.unwrap()); 32];

        // Shift the bits
        shifted[..32 - pos].copy_from_slice(&self.bits[pos..]);

        // Zero out the remaining bits
        for s in shifted.iter_mut().skip(32 - pos) {
            *s = GateBit::new(self.state.borrow().gate_config.zero_ref.unwrap());
        }

        self.bits = shifted;
    }
}

// TODO: Do an XOR of 3 numbers!

/// XOR 2 numbers of 32 bits
pub fn gate_u32_xor(gate_state: &mut GateState, a: &GateU32, b: &GateU32, r: &mut GateU32) {
    for i in 0..32 {
        let out_ref = gate_state.get_free_ref();
        gate_state.xor(a.bits[i].ref_, a.bits[i].pin_id, b.bits[i].ref_, b.bits[i].pin_id, out_ref);
        r.bits[i].ref_ = out_ref;
        r.bits[i].pin_id = PinId::D;
    }
}

/// And 2 numbers of 32 bits
pub fn gate_u32_and(gate_state: &mut GateState, a: &GateU32, b: &GateU32, r: &mut GateU32) {
    for i in 0..32 {
        let out_ref = gate_state.get_free_ref();
        gate_state.and(a.bits[i].ref_, a.bits[i].pin_id, b.bits[i].ref_, b.bits[i].pin_id, out_ref);
        r.bits[i].ref_ = out_ref;
        r.bits[i].pin_id = PinId::D;
    }
}

/// Not 1 number of 32 bits
pub fn gate_u32_not(gate_state: &mut GateState, a: &GateU32, r: &mut GateU32) {
    // NOT(a) is the same operation as XOR(a,1)
    for i in 0..32 {
        let out_ref = gate_state.get_free_ref();
        gate_state.xor(
            a.bits[i].ref_,
            a.bits[i].pin_id,
            gate_state.gate_config.zero_ref.unwrap(),
            PinId::B,
            out_ref,
        );
        r.bits[i].ref_ = out_ref;
        r.bits[i].pin_id = PinId::D;
    }
}

/*
    Add 2 numbers of 32 bits, taking into account the carry bit

    Example with a = b = 0b1...1:
    =============================
    bit  0:  r = 1 (a) + 1 (b)             = 0 = xor(a,b)              carry = 1 = and(a,b)
    bit  1:  r = 1 (a) + 1 (b) + 1 (carry) = 1 = xor(xor(a,b),carry))  carry = 1 = or(and(a,b),and(b,carry),and(a,carry))
    bit  2:  r = 1 (a) + 1 (b) + 1 (carry) = 1 = xor(xor(a,b),carry))  carry = 1 = or(and(a,b),and(b,carry),and(a,carry))
    ...
    bit 30:  r = 1 (a) + 1 (b) + 1 (carry) = 1 = xor(xor(a,b),carry))  carry = 1 = or(and(a,b),and(b,carry),and(a,carry))
    bit 31:  r = 1 (a) + 1 (b) + 1 (carry) = 1 = xor(xor(a,b),carry))  carry is not needed any more
*/
pub fn gate_u32_add(gate_state: &mut GateState, a: &GateU32, b: &GateU32, r: &mut GateU32) {
    let mut carry = GateBit { ref_: gate_state.gate_config.zero_ref.unwrap(), pin_id: PinId::A };

    for i in 0..32 {
        // Calculate result bit
        if i == 0 {
            r.bits[i].ref_ = gate_state.get_free_ref();
            gate_state.xor(
                a.bits[i].ref_,
                a.bits[i].pin_id,
                b.bits[i].ref_,
                b.bits[i].pin_id,
                r.bits[i].ref_,
            );
            r.bits[i].pin_id = PinId::D;
        } else {
            let xor_ref = gate_state.get_free_ref();
            gate_state.xor(
                a.bits[i].ref_,
                a.bits[i].pin_id,
                b.bits[i].ref_,
                b.bits[i].pin_id,
                xor_ref,
            );
            r.bits[i].ref_ = gate_state.get_free_ref();
            gate_state.xor(xor_ref, PinId::D, carry.ref_, carry.pin_id, r.bits[i].ref_);
            r.bits[i].pin_id = PinId::D;
        }

        // Calculate carry bit
        if i == 0 {
            carry.ref_ = gate_state.get_free_ref();
            gate_state.and(
                a.bits[i].ref_,
                a.bits[i].pin_id,
                b.bits[i].ref_,
                b.bits[i].pin_id,
                carry.ref_,
            );
            carry.pin_id = PinId::D;
        } else if i < 31 {
            let and_ref1 = gate_state.get_free_ref();
            gate_state.and(
                a.bits[i].ref_,
                a.bits[i].pin_id,
                b.bits[i].ref_,
                b.bits[i].pin_id,
                and_ref1,
            );

            let and_ref2 = gate_state.get_free_ref();
            gate_state.and(carry.ref_, carry.pin_id, b.bits[i].ref_, b.bits[i].pin_id, and_ref2);

            let and_ref3 = gate_state.get_free_ref();
            gate_state.and(a.bits[i].ref_, a.bits[i].pin_id, carry.ref_, carry.pin_id, and_ref3);

            let or_ref = gate_state.get_free_ref();
            gate_state.or(and_ref1, PinId::D, and_ref2, PinId::D, or_ref);

            carry.ref_ = gate_state.get_free_ref();
            gate_state.or(or_ref, PinId::D, and_ref3, PinId::D, carry.ref_);
            carry.pin_id = PinId::D;
        }
    }
}

// TODO: For the future!
// pub fn gate_u32_add(gate_state: &mut GateState, a: &GateU32, b: &GateU32, r: &mut GateU32) {
//     let mut carry = GateBit::new(gate_state.gate_config.zero_ref.unwrap());
//     for i in 0..32 {
//         if i == 0 {
//             let out_ref = gate_state.get_free_ref();
//             let carry_val = gate_state.add(
//                 a.bits[i].ref_,
//                 a.bits[i].pin_id,
//                 b.bits[i].ref_,
//                 b.bits[i].pin_id,
//                 gate_state.gate_config.zero_ref.unwrap(),
//                 PinId::A,
//                 out_ref,
//             );
//             r.bits[i].ref_ = out_ref;
//             r.bits[i].pin_id = PinId::D;
//         } else if i < 31 {
//             let out_ref = gate_state.get_free_ref();
//             let carry_val = gate_state.add(
//                 a.bits[i].ref_,
//                 a.bits[i].pin_id,
//                 b.bits[i].ref_,
//                 b.bits[i].pin_id,
//                 carry.ref_,
//                 carry.pin_id,
//                 out_ref,
//             );
//             r.bits[i].ref_ = out_ref;
//             r.bits[i].pin_id = PinId::D;
//         } else {
//             let out_ref = gate_state.get_free_ref();
//             gate_state.xor3(
//                 a.bits[i].ref_,
//                 a.bits[i].pin_id,
//                 b.bits[i].ref_,
//                 b.bits[i].pin_id,
//                 carry.ref_,
//                 carry.pin_id,
//                 out_ref,
//             );
//             r.bits[i].ref_ = out_ref;
//             r.bits[i].pin_id = PinId::D;
//         }
//     }
// }
