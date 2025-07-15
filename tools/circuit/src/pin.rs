use std::ops::{Index, IndexMut};

#[derive(Clone, Debug)]
pub struct Pin {
    id: PinId,
    pub source: PinSource,
    pub wired_ref: u64,
    pub wired_pin_id: PinId,
    pub fan_out: u64, // Number of pins connected to this pin as an output
    pub bit: u8,      // 0 or 1
    pub connections_to_input_a: Vec<u64>,
    pub connections_to_input_b: Vec<u64>,
    pub connections_to_input_c: Vec<u64>,
}

#[derive(Debug, Clone, Copy)]
pub enum PinId {
    A = 0, // Input a pin
    B = 1, // Input b pin
    C = 2, // Input c pin
    D = 3, // Output d pin representing the result of the gate
    E = 4, // Output e pin representing the result of the gate (used for e.g., carry)
}

impl Index<PinId> for [Pin; 5] {
    type Output = Pin;

    fn index(&self, pin_id: PinId) -> &Self::Output {
        &self[pin_id as usize]
    }
}

impl IndexMut<PinId> for [Pin; 5] {
    fn index_mut(&mut self, pin_id: PinId) -> &mut Self::Output {
        &mut self[pin_id as usize]
    }
}

#[derive(Clone, Debug)]
/// Describes how the bit value of that pin is established
pub enum PinSource {
    External = 0, // A fixed value externally provided; an external signal
    Wired = 1,    // Connected to another pin
    Gated = 2,    // This pin is the output of a gate; only used with pin_c pins
}

impl Pin {
    pub fn new(id: PinId) -> Self {
        // Default values for the pin
        Self {
            id,
            // If the pin is not connected, then source is external
            // If the pin is connected to another pin, then source is gated
            source: match id {
                PinId::A | PinId::B | PinId::C => PinSource::External,
                PinId::D | PinId::E => PinSource::Gated,
            },
            wired_ref: 0,
            wired_pin_id: PinId::D,
            fan_out: 0,
            bit: 0,
            connections_to_input_a: Vec::new(),
            connections_to_input_b: Vec::new(),
            connections_to_input_c: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.source = match self.id {
            PinId::A | PinId::B | PinId::C => PinSource::External,
            PinId::D | PinId::E => PinSource::Gated,
        };
        self.wired_ref = 0;
        self.wired_pin_id = PinId::D;
        self.fan_out = 0;
        self.bit = 0;
        self.connections_to_input_a.clear();
        self.connections_to_input_b.clear();
        self.connections_to_input_c.clear();
    }
}
