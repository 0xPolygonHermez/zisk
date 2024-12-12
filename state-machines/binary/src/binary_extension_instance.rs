use std::sync::Arc;

use p3_field::PrimeField;

use proofman_common::{AirInstance, FromTrace};
use proofman_util::{timer_start_debug, timer_stop_and_log_debug};
use sm_common::{Instance, InstanceExpanderCtx, InstanceType, RegularInstance};
use zisk_core::{ZiskRequiredOperation, ZiskRom};
use zisk_pil::BinaryExtensionTrace;
use ziskemu::EmuTrace;

use crate::BinaryExtensionSM;

pub struct BinaryExtensionInstance<F: PrimeField> {
    binary_extension_sm: Arc<BinaryExtensionSM<F>>,
    iectx: InstanceExpanderCtx,

    inputs: Vec<ZiskRequiredOperation>,
    binary_e_trace: BinaryExtensionTrace<F>,
}

impl<F: PrimeField> BinaryExtensionInstance<F> {
    pub fn new(binary_extension_sm: Arc<BinaryExtensionSM<F>>, iectx: InstanceExpanderCtx) -> Self {
        Self {
            binary_extension_sm,
            iectx,
            inputs: Vec::new(),
            binary_e_trace: BinaryExtensionTrace::new(),
        }
    }
}

impl<F: PrimeField> Instance<F> for BinaryExtensionInstance<F> {
    fn collect(
        &mut self,
        zisk_rom: &ZiskRom,
        min_traces: Arc<Vec<EmuTrace>>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        self.inputs = RegularInstance::collect(
            self.iectx.plan.check_point.unwrap(),
            BinaryExtensionTrace::<F>::NUM_ROWS,
            zisk_rom,
            min_traces,
            zisk_core::ZiskOperationType::BinaryE,
        )?;

        Ok(())
    }

    fn compute_witness(&mut self) -> Option<AirInstance<F>> {
        timer_start_debug!(PROVE_BINARY);
        self.binary_extension_sm.prove_instance(&self.inputs, &mut self.binary_e_trace);
        timer_stop_and_log_debug!(PROVE_BINARY);

        let instance = AirInstance::new_from_trace(FromTrace::new(&mut self.binary_e_trace));

        Some(instance)
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
}

unsafe impl<F: PrimeField> Sync for BinaryExtensionInstance<F> {}
