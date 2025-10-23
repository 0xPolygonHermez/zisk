#[derive(Debug, Clone)]
pub struct GateConfig {
    pub gate_number: u64,
    pub max_refs: u64,
    pub zero_ref: Option<u64>,
    pub first_usable_ref: u64,
    pub sin_first_ref: u64,
    pub sin_ref_group_by: u64,
    pub sin_ref_number: u64,
    pub sin_ref_distance: u64,
    pub sin_last_ref: u64,
    pub sout_first_ref: u64,
    pub sout_ref_group_by: u64,
    pub sout_ref_number: u64,
    pub sout_ref_distance: u64,
    pub sout_last_ref: u64,
}

impl GateConfig {
    /// Creates a new GateConfig with specified values
    #[allow(clippy::too_many_arguments)]
    pub const fn with_values(
        gate_number: u64,
        max_refs: u64,
        zero_ref: Option<u64>,
        sin_first_ref: u64,
        sin_ref_group_by: u64,
        sin_ref_number: u64,
        sin_ref_distance: u64,
        sout_first_ref: u64,
        sout_ref_group_by: u64,
        sout_ref_number: u64,
        sout_ref_distance: u64,
    ) -> Self {
        assert!(max_refs >= gate_number);

        let sin_last_ref = sin_first_ref
            + (sin_ref_number - sin_ref_group_by) * sin_ref_distance / sin_ref_group_by
            + (sin_ref_group_by - 1);
        let sout_last_ref = sout_first_ref
            + (sout_ref_number - sout_ref_group_by) * sout_ref_distance / sout_ref_group_by
            + (sout_ref_group_by - 1);

        let mut first_usable_ref = 0;
        while {
            let is_zero = match zero_ref {
                Some(z) => z == first_usable_ref,
                None => false,
            };

            // If it coincides with the zero_ref or any sin_ref or sout_ref, skip it
            is_zero
                || (first_usable_ref >= sin_first_ref
                    && first_usable_ref <= sin_last_ref
                    && (first_usable_ref - sin_first_ref) % sin_ref_distance < sin_ref_group_by)
                || (first_usable_ref >= sout_first_ref
                    && first_usable_ref <= sout_last_ref
                    && (first_usable_ref - sout_first_ref) % sout_ref_distance < sout_ref_group_by)
        } {
            first_usable_ref += 1;
        }
        assert!(first_usable_ref <= max_refs);

        Self {
            gate_number,
            max_refs,
            zero_ref,
            first_usable_ref,
            sin_first_ref,
            sin_ref_group_by,
            sin_ref_number,
            sin_ref_distance,
            sin_last_ref,
            sout_first_ref,
            sout_ref_group_by,
            sout_ref_number,
            sout_ref_distance,
            sout_last_ref,
        }
    }

    /// Converts a relative reference to an absolute reference based on the slot
    pub fn rel_ref_to_abs_ref(&self, ref_: u64, slot: u64) -> u64 {
        // References have an offset of one slot size per slot
        slot * self.gate_number + ref_
    }
}
