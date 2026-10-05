//! Without CUDA there is no planner to register: every query says the RAM witness is unavailable.

pub fn clear_gpu_ram_witness() {}

pub fn gpu_ram_witness_available() -> bool {
    false
}

#[derive(Copy, Clone, Debug, Default)]
pub struct RamFillPrepared {
    pub status: i32,
    pub n_instances: u32,
    pub n_accesses: u64,
    pub n_lanes: u64,
    pub ms_sort: f32,
    pub ms_lanes: f32,
    pub ms_values: f32,
    pub ms_total: f32,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct RamFillResult {
    pub status: i32,
    pub n_lanes: u32,
    pub prev_addr_w: u32,
    pub last_addr_w: u32,
    pub prev_step: u64,
    pub prev_value: u64,
    pub last_step: u64,
    pub last_value: u64,
    pub ms_rows: f32,
    pub ms_d2h: f32,
}

pub fn gpu_ram_witness_prepare() -> Result<RamFillPrepared, String> {
    Err("built without CUDA".to_string())
}

pub fn gpu_ram_witness_fill(
    _inst: u32,
    _out_rows: &mut [u64],
    _n_rows: u32,
) -> Result<RamFillResult, String> {
    Err("built without CUDA".to_string())
}

pub fn gpu_ram_witness_fill_all() -> Result<RamFillPrepared, String> {
    Err("built without CUDA".to_string())
}

pub fn gpu_rom_witness_fill_all() -> Result<RamFillPrepared, String> {
    Err("built without CUDA".to_string())
}

pub fn gpu_rom_witness_fill(
    _inst: u32,
    _out_rows: &mut [u64],
    _n_rows: u32,
) -> Result<RamFillResult, String> {
    Err("built without CUDA".to_string())
}

pub fn gpu_input_witness_fill_all(_image: &[u8]) -> Result<RamFillPrepared, String> {
    Err("built without CUDA".to_string())
}

pub fn gpu_input_witness_fill(
    _inst: u32,
    _out_rows: &mut [u64],
    _n_rows: u32,
) -> Result<RamFillResult, String> {
    Err("built without CUDA".to_string())
}

pub fn gpu_align_witness_fill_all(_plans: &[zisk_common::Plan]) -> Result<RamFillPrepared, String> {
    Err("built without CUDA".to_string())
}

pub fn gpu_align_witness_fill(
    _air_id: usize,
    _segment: usize,
    _out_rows: &mut [u64],
) -> Result<usize, String> {
    Err("built without CUDA".to_string())
}
