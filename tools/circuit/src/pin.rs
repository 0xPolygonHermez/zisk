use std::ops::{Index, IndexMut};

pub const NUM_PINS: usize = 4;

#[derive(Debug, Clone)]
pub struct Pin {
    id: PinId,
    pub source: PinSource,
    pub wired_ref: u64,
    pub wired_pin_id: PinId,
    pub fan_out: u64, // Number of pins connected to this pin as an output
    pub bit: u8,      // 0 or 1
    pub connections: [Vec<u64>; NUM_PINS],
}

#[derive(Debug, Clone, Copy)]
pub enum PinId {
    A = 0, // Input a pin
    B = 1, // Input b pin
    C = 2, // Input c pin
    D = 3, // Output d pin representing the result of the gate
}

impl Index<PinId> for [Pin; NUM_PINS] {
    type Output = Pin;

    fn index(&self, pin_id: PinId) -> &Self::Output {
        &self[pin_id as usize]
    }
}

impl IndexMut<PinId> for [Pin; NUM_PINS] {
    fn index_mut(&mut self, pin_id: PinId) -> &mut Self::Output {
        &mut self[pin_id as usize]
    }
}

impl PinId {
    // If the pin is not connected, then source is external
    // If the pin is connected to another pin, then source is gated
    pub const fn default_source(self) -> PinSource {
        match self {
            PinId::A | PinId::B | PinId::C => PinSource::External,
            PinId::D => PinSource::Gated,
        }
    }
}

#[derive(Debug, Clone)]
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
            source: id.default_source(),
            wired_ref: 0,
            wired_pin_id: PinId::D,
            fan_out: 0,
            bit: 0,
            connections: Default::default(),
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new(self.id);
    }

    pub fn add_connection_to(&mut self, target_pin_id: PinId, target_ref: u64) {
        self.connections[target_pin_id as usize].push(target_ref);
    }
}
