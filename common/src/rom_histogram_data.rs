/// ROM instruction frequency data collected during execution.
#[repr(C)]
#[derive(Debug, Default)]
pub struct RomHistogramData {
    pub steps: u64,
    pub bios_inst_count: Vec<u64>,
    pub prog_inst_count: Vec<u64>,
}

impl RomHistogramData {
    pub fn new(steps: u64, bios_inst_count: Vec<u64>, prog_inst_count: Vec<u64>) -> Self {
        Self { steps, bios_inst_count, prog_inst_count }
    }
}
