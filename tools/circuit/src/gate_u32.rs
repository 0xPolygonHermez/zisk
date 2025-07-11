use core::panic;
use std::cell::RefCell;

use crate::{bits_to_u32_msb, u32_to_bits_msb};

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
        let bits = u32_to_bits_msb(value);

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

        bits_to_u32_msb(&bits)
    }

    pub fn rotate_right(&mut self, pos: usize) {
        let mut rotated = [GateBit::new(self.state.borrow().gate_config.zero_ref.unwrap()); 32];
        for (i, rotated_bit) in rotated.iter_mut().enumerate() {
            *rotated_bit = self.bits[(32 - pos + i) % 32];
        }
        self.bits = rotated;
    }

    pub fn shift_right(&mut self, pos: usize) {
        let mut shifted = [GateBit::new(self.state.borrow().gate_config.zero_ref.unwrap()); 32];

        // Zero out the first `pos` bits
        for s in shifted.iter_mut().take(pos) {
            *s = GateBit::new(self.state.borrow().gate_config.zero_ref.unwrap());
        }

        // Shift the remaining bits
        for (i, shifted_bit) in shifted.iter_mut().enumerate().skip(pos) {
            *shifted_bit = self.bits[i - pos];
        }

        self.bits = shifted;
    }
}

pub fn gate_u32_xor(
    gate_state: &mut GateState,
    a: &GateU32,
    b: &GateU32,
    c: &GateU32,
    r: &mut GateU32,
) {
    for i in 0..32 {
        let out_ref = gate_state.get_free_ref();
        gate_state.xor3(
            a.bits[i].ref_,
            a.bits[i].pin_id,
            b.bits[i].ref_,
            b.bits[i].pin_id,
            c.bits[i].ref_,
            c.bits[i].pin_id,
            out_ref,
        );
        r.bits[i].ref_ = out_ref;
        r.bits[i].pin_id = PinId::D;
    }
}

pub fn gate_u32_ch(
    gate_state: &mut GateState,
    a: &GateU32,
    b: &GateU32,
    c: &GateU32,
    r: &mut GateU32,
) {
    for i in 0..32 {
        let out_ref = gate_state.get_free_ref();
        gate_state.ch(
            a.bits[i].ref_,
            a.bits[i].pin_id,
            b.bits[i].ref_,
            b.bits[i].pin_id,
            c.bits[i].ref_,
            c.bits[i].pin_id,
            out_ref,
        );
        r.bits[i].ref_ = out_ref;
        r.bits[i].pin_id = PinId::D;
    }
}

pub fn gate_u32_maj(
    gate_state: &mut GateState,
    a: &GateU32,
    b: &GateU32,
    c: &GateU32,
    r: &mut GateU32,
) {
    for i in 0..32 {
        let out_ref = gate_state.get_free_ref();
        gate_state.maj(
            a.bits[i].ref_,
            a.bits[i].pin_id,
            b.bits[i].ref_,
            b.bits[i].pin_id,
            c.bits[i].ref_,
            c.bits[i].pin_id,
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
    let mut prev_ref = gate_state.get_free_ref();
    // First bit
    r.bits[31].ref_ = prev_ref;
    gate_state.add(
        a.bits[31].ref_,
        a.bits[31].pin_id,
        b.bits[31].ref_,
        b.bits[31].pin_id,
        gate_state.gate_config.zero_ref.unwrap(),
        PinId::A,
        r.bits[31].ref_,
    );
    r.bits[31].pin_id = PinId::D;

    for i in (0..31).rev() {
        // Calculate result bit
        if i == 0 {
            r.bits[i].ref_ = gate_state.get_free_ref();
            gate_state.xor3(
                a.bits[i].ref_,
                a.bits[i].pin_id,
                b.bits[i].ref_,
                b.bits[i].pin_id,
                prev_ref,
                PinId::E,
                r.bits[i].ref_,
            );
            r.bits[i].pin_id = PinId::D;
        } else {
            r.bits[i].ref_ = gate_state.get_free_ref();
            gate_state.add(
                a.bits[i].ref_,
                a.bits[i].pin_id,
                b.bits[i].ref_,
                b.bits[i].pin_id,
                prev_ref,
                PinId::E,
                r.bits[i].ref_,
            );
            r.bits[i].pin_id = PinId::D;

            // Update the ref
            prev_ref = r.bits[i].ref_;
        }
    }
}
