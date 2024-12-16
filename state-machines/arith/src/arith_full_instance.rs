use std::sync::Arc;

use p3_field::PrimeField;

use proofman_common::{AirInstance, FromTrace};
use sm_common::{InputsCollector, Instance, InstanceExpanderCtx, InstanceType};
use zisk_core::{ZiskRequiredOperation, ZiskRom};
use zisk_pil::ArithTrace;
use ziskemu::EmuTrace;

use crate::ArithFullSM;

pub struct ArithFullInstance<F: PrimeField> {
    /// Arith full state machine
    arith_full_sm: Arc<ArithFullSM>,

    /// Instance expander context
    iectx: InstanceExpanderCtx,

    /// Arith trace
    trace: ArithTrace<F>,

    /// Inputs
    inputs: Vec<ZiskRequiredOperation>,
}

impl<F: PrimeField> ArithFullInstance<F> {
    pub fn new(arith_full_sm: Arc<ArithFullSM>, iectx: InstanceExpanderCtx) -> Self {
        Self { arith_full_sm, iectx, trace: ArithTrace::new(), inputs: Vec::new() }
    }
}

impl<F: PrimeField> Instance<F> for ArithFullInstance<F> {
    fn collect_inputs(
        &mut self,
        zisk_rom: &ZiskRom,
        min_traces: &[EmuTrace],
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        self.inputs = InputsCollector::collect(
            self.iectx.plan.check_point.unwrap(),
            ArithTrace::<F>::NUM_ROWS,
            zisk_rom,
            min_traces,
            zisk_core::ZiskOperationType::Arith,
        )?;

        Ok(())
    }

    fn compute_witness(&mut self) -> Option<AirInstance<F>> {
        self.arith_full_sm.prove_instance(&self.inputs, &mut self.trace);

        Some(AirInstance::new_from_trace(FromTrace::new(&mut self.trace)))
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
}

unsafe impl<F: PrimeField> Sync for ArithFullInstance<F> {}
