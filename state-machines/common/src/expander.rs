use std::sync::Arc;

use zisk_core::ZiskRom;
use ziskemu::EmuTrace;

pub trait InstanceXXXX: Send + Sync {
    fn expand(
        &mut self,
        zisk_rom: &ZiskRom,
        min_traces: Arc<Vec<EmuTrace>>,
    ) -> Result<(), Box<dyn std::error::Error + Send>>;

    fn prove(
        &mut self,
        min_traces: Arc<Vec<EmuTrace>>,
    ) -> Result<(), Box<dyn std::error::Error + Send>>;
}
