use std::sync::Arc;

use p3_field::PrimeField;
use proofman_common::AirInstance;
use zisk_core::ZiskRom;
use ziskemu::EmuTrace;

#[derive(PartialEq)]
pub enum InstanceType {
    Instance,
    Table,
}

pub trait Instance<F: PrimeField>: Send + Sync {
    fn collect(
        &mut self,
        zisk_rom: &ZiskRom,
        min_traces: Arc<Vec<EmuTrace>>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        let _ = zisk_rom;
        let _ = min_traces;
        Ok(())
    }

    fn compute_witness(&mut self) -> Option<AirInstance<F>>;

    fn instance_type(&self) -> InstanceType;
}
