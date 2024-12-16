use std::sync::Arc;

use p3_field::PrimeField;

use proofman_common::{AirInstance, FromTrace, ProofCtx};
use proofman_util::{timer_start_debug, timer_stop_and_log_debug};
use sm_common::{InputsCollector, Instance, InstanceExpanderCtx, InstanceType};
use zisk_core::{ZiskRequiredOperation, ZiskRom};
use zisk_pil::BinaryExtensionTrace;
use ziskemu::EmuTrace;

use crate::BinaryExtensionSM;

pub struct BinaryExtensionInstance<F: PrimeField> {
    /// Binary extension state machine
    binary_extension_sm: Arc<BinaryExtensionSM<F>>,

    /// Instance expander context
    iectx: InstanceExpanderCtx,

    /// Binary extension trace
    trace: BinaryExtensionTrace<F>,

    /// Inputs
    inputs: Vec<ZiskRequiredOperation>,
}

impl<F: PrimeField> BinaryExtensionInstance<F> {
    pub fn new(binary_extension_sm: Arc<BinaryExtensionSM<F>>, iectx: InstanceExpanderCtx) -> Self {
        Self { binary_extension_sm, iectx, inputs: Vec::new(), trace: BinaryExtensionTrace::new() }
    }
}

impl<F: PrimeField> Instance<F> for BinaryExtensionInstance<F> {
    fn collect_inputs(
        &mut self,
        zisk_rom: &ZiskRom,
        min_traces: &[EmuTrace],
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        self.inputs = InputsCollector::collect(
            self.iectx.plan.check_point.unwrap(),
            BinaryExtensionTrace::<F>::NUM_ROWS,
            zisk_rom,
            min_traces,
            zisk_core::ZiskOperationType::BinaryE,
        )?;

        Ok(())
    }

    fn compute_witness(&mut self, _pctx: &ProofCtx<F>) -> Option<AirInstance<F>> {
        timer_start_debug!(PROVE_BINARY);
        self.binary_extension_sm.prove_instance(&self.inputs, &mut self.trace);
        timer_stop_and_log_debug!(PROVE_BINARY);

        let instance = AirInstance::new_from_trace(FromTrace::new(&mut self.trace));

        Some(instance)
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
}

unsafe impl<F: PrimeField> Sync for BinaryExtensionInstance<F> {}
