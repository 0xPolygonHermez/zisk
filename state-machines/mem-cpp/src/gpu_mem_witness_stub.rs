//! Without CUDA there is no planner: every query says the device memory witness is unavailable,
//! with the same surface as `gpu_mem_witness.rs`.

use crate::mem_witness_types::MemFillResult;

const UNAVAILABLE: &str = "built without CUDA";

pub fn clear_gpu_mem_witness() {}

pub fn gpu_ram_witness_fill(
    _inst: u32,
    _out_rows: &mut [u64],
    _n_rows: u32,
) -> Result<MemFillResult, String> {
    Err(UNAVAILABLE.to_string())
}

pub fn gpu_rom_witness_fill(
    _inst: u32,
    _out_rows: &mut [u64],
    _n_rows: u32,
) -> Result<MemFillResult, String> {
    Err(UNAVAILABLE.to_string())
}

pub fn gpu_input_witness_fill(
    _inst: u32,
    _out_rows: &mut [u64],
    _n_rows: u32,
) -> Result<MemFillResult, String> {
    Err(UNAVAILABLE.to_string())
}

pub fn gpu_align_witness_fill(
    _air_id: usize,
    _segment: usize,
    _out_rows: &mut [u64],
) -> Result<usize, String> {
    Err(UNAVAILABLE.to_string())
}

pub fn gpu_mem_witness_prepare_async(
    _image: Vec<u8>,
    _ram: Vec<u32>,
    _rom: Vec<u32>,
    _input: Vec<u32>,
    _align_plans: &[&zisk_common::Plan],
) -> Result<(), String> {
    Err(UNAVAILABLE.to_string())
}

pub fn gpu_mem_witness_arm(_n_pending: usize, _release: Box<dyn FnOnce() + Send>) {}

pub fn gpu_mem_witness_release_now() {}
