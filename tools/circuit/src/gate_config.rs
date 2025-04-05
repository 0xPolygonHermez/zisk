#[derive(Default, Debug, Clone)]
pub struct GateConfig {
    pub zero_ref: u64,
    pub slot_size: u64,
    pub max_refs: u64,
    pub first_next_ref: u64,
    pub sin_ref0: u64,
    pub sin_ref_group_by: u64,
    pub sin_ref_number: u64,
    pub sin_ref_distance: u64,
    pub sout_ref0: u64,
    pub sout_ref_group_by: u64,
    pub sout_ref_number: u64,
    pub sout_ref_distance: u64,
    pub pol_length: u64,
}

impl GateConfig {
    /// Creates a new GateConfig with all fields initialized to zero
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new GateConfig with specified values
    pub fn with_values(
        zero_ref: u64,
        slot_size: u64,
        max_refs: u64,
        first_next_ref: u64,
        sin_ref0: u64,
        sin_ref_group_by: u64,
        sin_ref_number: u64,
        sin_ref_distance: u64,
        sout_ref0: u64,
        sout_ref_group_by: u64,
        sout_ref_number: u64,
        sout_ref_distance: u64,
        pol_length: u64,
    ) -> Self {
        Self {
            zero_ref,
            slot_size,
            max_refs,
            first_next_ref,
            sin_ref0,
            sin_ref_group_by,
            sin_ref_number,
            sin_ref_distance,
            sout_ref0,
            sout_ref_group_by,
            sout_ref_number,
            sout_ref_distance,
            pol_length,
        }
    }

    /// Converts a relative reference to an absolute reference based on the slot
    pub fn rel_ref_to_abs_ref(&self, ref_: u64, slot: u64) -> u64 {
        // References have an offset of one slot size per slot
        slot * self.slot_size + ref_
    }
}
