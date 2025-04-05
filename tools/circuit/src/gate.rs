use crate::{Pin, PinId, PinSource};

/*
    a -----||---\
           || OP )----- c
    b -----||---/
*/
#[derive(Clone, Debug)]
pub struct Gate {
    pub op: GateOperation,
    pub pins: [Pin; 3],
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum GateOperation {
    Unknown = 0,
    Xor = 1,  // Xor(a,b) := a ^ b
    Andp = 2, // Andp(a.b) := ¬a & b
    Or = 3,   // Or(a,b) := a | b
    And = 4,  // And(a,b) := a & b
    Ch = 5,   // Ch(a,b,c) := (a & b) ^ (¬a & c)
    Maj = 6,  // Maj(a,b,c) := (a & b) ^ (a & c) ^ (b & c)
    Add = 7,  // Add(a,b,c) := a + b + c
}

impl Gate {
    pub fn new() -> Self {
        // Default gate is XOR(0,0), where 0 is externally set
        Self {
            op: GateOperation::Xor,
            pins: [Pin::new(PinId::A), Pin::new(PinId::B), Pin::new(PinId::C)],
        }
    }

    pub fn reset(&mut self) {
        self.op = GateOperation::Xor;

        // Reset pins
        for pin in self.pins.iter_mut() {
            pin.reset();
        }

        // Set the default values for the pins
        self.pins[PinId::A].source = PinSource::External;
        self.pins[PinId::A].bit = 0;

        self.pins[PinId::B].source = PinSource::External;
        self.pins[PinId::B].bit = 0;

        self.pins[PinId::C].source = PinSource::Gated;
        self.pins[PinId::C].bit = 0; // XOR(0,0) = 0
    }
}
