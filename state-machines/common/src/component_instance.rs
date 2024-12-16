use p3_field::PrimeField;
use proofman_common::{AirInstance, ProofCtx};
use zisk_core::ZiskRom;
use ziskemu::EmuTrace;

#[derive(PartialEq)]
pub enum InstanceType {
    Instance,
    Table,
}

pub trait Instance<F: PrimeField>: Send + Sync {
    fn collect_inputs(
        &mut self,
        zisk_rom: &ZiskRom,
        min_traces: &[EmuTrace],
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        let _ = zisk_rom;
        let _ = min_traces;
        Ok(())
    }

    fn compute_witness(&mut self, pctx: &ProofCtx<F>) -> Option<AirInstance<F>>;

    fn instance_type(&self) -> InstanceType;
}
