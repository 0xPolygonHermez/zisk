//! Plain data the device memory witness hands back; mirrored byte for byte in
//! `cu/count_and_plan.cuh` (the `static_assert`s there and the asserts here keep them aligned).

/// What the block's preparation established. Fields are read by the Rust side only through
/// `Copy`.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct RamFillPrepared {
    /// 0 when the preparation succeeded, else the planner's reason code.
    pub status: i32,
    /// Instances the preparation covered, every family together.
    pub n_instances: u32,
    /// Retained RAM accesses of the block.
    pub n_accesses: u64,
    /// Mem lanes after dual pairing.
    pub n_lanes: u64,
    /// One bit per family served: 1 Mem, 2 RomData, 4 InputData, 8 MemAlign. A family left
    /// out stays on the CPU witness for the block.
    pub families: u32,
    pub reserved: u32,
}
const _: () = assert!(core::mem::size_of::<RamFillPrepared>() == 32);

/// One filled instance: its scalars for the air values, and its status.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct RamFillResult {
    /// 0 ok; -5 resolved for another process; other negatives: a failure.
    pub status: i32,
    /// Lanes the instance's accesses occupy, before padding (MemAlign: rows used).
    pub n_lanes: u32,
    /// The lane before the instance's window (continuation).
    pub prev_addr_w: u32,
    /// The instance's last lane.
    pub last_addr_w: u32,
    pub prev_step: u64,
    pub prev_value: u64,
    pub last_step: u64,
    pub last_value: u64,
}
const _: () = assert!(core::mem::size_of::<RamFillResult>() == 48);

/// `families` bits of [`RamFillPrepared`] and the family numbering of the planner's slot ops.
pub const MEM_FAMILY_RAM: u32 = 0;
pub const MEM_FAMILY_ROM: u32 = 1;
pub const MEM_FAMILY_INPUT: u32 = 2;
pub const MEM_FAMILY_ALIGN: u32 = 3;
