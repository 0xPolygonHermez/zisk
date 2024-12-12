use std::sync::Arc;

use p3_field::PrimeField;

use proofman_common::{AirInstance, FromTrace};
use sm_common::{InputsCollector, Instance, InstanceExpanderCtx, InstanceType};
use zisk_core::{ZiskRequiredOperation, ZiskRom};
use zisk_pil::BinaryTrace;
use ziskemu::EmuTrace;

use crate::BinaryBasicSM;

pub struct BinaryBasicInstance<F: PrimeField> {
    binary_basic_sm: Arc<BinaryBasicSM>,
    iectx: InstanceExpanderCtx,

    inputs: Vec<ZiskRequiredOperation>,
    binary_trace: BinaryTrace<F>,
}

impl<F: PrimeField> BinaryBasicInstance<F> {
    pub fn new(binary_basic_sm: Arc<BinaryBasicSM>, iectx: InstanceExpanderCtx) -> Self {
        Self { binary_basic_sm, iectx, inputs: Vec::new(), binary_trace: BinaryTrace::new() }
    }
}

impl<F: PrimeField> Instance<F> for BinaryBasicInstance<F> {
    fn collect(
        &mut self,
        zisk_rom: &ZiskRom,
        min_traces: Arc<Vec<EmuTrace>>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        self.inputs = InputsCollector::collect(
            self.iectx.plan.check_point.unwrap(),
            BinaryTrace::<F>::NUM_ROWS,
            zisk_rom,
            min_traces,
            zisk_core::ZiskOperationType::Binary,
        )?;

        Ok(())
    }

    fn compute_witness(&mut self) -> Option<AirInstance<F>> {
        self.binary_basic_sm.prove_instance(&self.inputs, &mut self.binary_trace);

        let instance = AirInstance::new_from_trace(FromTrace::new(&mut self.binary_trace));
        Some(instance)
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
}

unsafe impl<F: PrimeField> Sync for BinaryBasicInstance<F> {}
