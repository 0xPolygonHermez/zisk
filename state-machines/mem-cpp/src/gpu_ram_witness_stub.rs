//! Without CUDA there is no planner: every query says the device memory witness is unavailable,
//! with the same surface as `gpu_ram_witness.rs`.

use crate::mem_witness_types::RamFillResult;

const UNAVAILABLE: &str = "built without CUDA";

pub fn clear_gpu_ram_witness() {}

pub fn gpu_ram_witness_fill(
    _inst: u32,
    _out_rows: &mut [u64],
    _n_rows: u32,
) -> Result<RamFillResult, String> {
    Err(UNAVAILABLE.to_string())
}

pub fn gpu_rom_witness_fill(
    _inst: u32,
    _out_rows: &mut [u64],
    _n_rows: u32,
) -> Result<RamFillResult, String> {
    Err(UNAVAILABLE.to_string())
}

pub fn gpu_input_witness_fill(
    _inst: u32,
    _out_rows: &mut [u64],
    _n_rows: u32,
) -> Result<RamFillResult, String> {
    Err(UNAVAILABLE.to_string())
}

pub fn gpu_align_witness_fill(
    _air_id: usize,
    _segment: usize,
    _out_rows: &mut [u64],
) -> Result<usize, String> {
    Err(UNAVAILABLE.to_string())
}

pub fn gpu_slot_witness_prepare_async(
    _image: Vec<u8>,
    _ram: Vec<u32>,
    _rom: Vec<u32>,
    _input: Vec<u32>,
    _align_plans: &[&zisk_common::Plan],
    _host_rows: bool,
) -> Result<(), String> {
    Err(UNAVAILABLE.to_string())
}

pub fn gpu_mem_witness_families() -> u32 {
    0
}

pub fn gpu_mem_witness_host_rows_mask() -> u32 {
    0
}

pub fn gpu_mem_witness_scalars(_family: u32, _inst: u32) -> Result<RamFillResult, String> {
    Err(UNAVAILABLE.to_string())
}

pub fn gpu_mem_witness_align_used(_air_id: usize, _segment: usize) -> Result<u32, String> {
    Err(UNAVAILABLE.to_string())
}

pub fn gpu_slot_witness_arm(_n_pending: usize, _release: Box<dyn FnOnce() + Send>) {}

pub fn gpu_slot_witness_release_now() {}

/// # Safety
/// Never called: the prover only takes this kernel from a GPU build.
pub unsafe extern "C" fn zisk_mem_witness_slot_kernel(
    _d_ops: *const core::ffi::c_void,
    _num_ops: u64,
    _d_dst: *mut u64,
    _device_id: i32,
    _stream: *mut core::ffi::c_void,
) -> i32 {
    -1
}
