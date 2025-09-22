use zisk_common::{ExecutorStats, Plan};

use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use anyhow::Result;

pub struct PreloadedMO {}

// This struct is used to run the assembly code in a separate process and generate minimal traces.
#[derive(Debug)]
pub struct AsmRunnerMO {
    pub plans: Vec<Plan>,
}

unsafe impl Send for AsmRunnerMO {}
unsafe impl Sync for AsmRunnerMO {}

impl AsmRunnerMO {
    pub fn run(
        _: &mut PreloadedMO,
        _: u64,
        _: u64,
        _: i32,
        _: i32,
        _: Option<u16>,
        _: Arc<Mutex<ExecutorStats>>,
    ) -> Result<Self> {
        Err(anyhow::anyhow!(
            "AsmRunnerMO::run() is not supported on this platform. Only Linux x86_64 is supported."
        ))
    }
}
