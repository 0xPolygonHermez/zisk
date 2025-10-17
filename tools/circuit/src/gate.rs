use crate::{Pin, PinId, NUM_PINS};

/*
    a -----||-------\
           ||        |
           ||        |
    b -----||   OP    )----- d
           ||        |
           ||        |
    c -----||-------/
*/
#[derive(Clone, Debug)]
pub struct Gate {
    pub op: GateOperation,
    pub pins: [Pin; NUM_PINS],
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum GateOperation {
    Unknown = 0,
    Xor2 = 1,    // Xor(a,b) := a ^ b
    Nand = 2,    // Nand(a,b) := ¬a & b
    Xor3 = 3,    // Xor(a,b,c) := a ^ b ^ c
    XorNand = 4, // XorNand(a,b,c) := a ^ (¬b & c)
}

impl Gate {
    pub fn new() -> Self {
        // Default gate is XOR
        Self {
            op: GateOperation::Xor2,
            pins: [Pin::new(PinId::A), Pin::new(PinId::B), Pin::new(PinId::C), Pin::new(PinId::D)],
        }
    }

    pub fn reset(&mut self) {
        self.op = GateOperation::Xor2;
        for pin in self.pins.iter_mut() {
            pin.reset();
        }
    }
}
