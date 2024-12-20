use crate::BinaryExtensionSM;
use p3_field::PrimeField;
use proofman_common::{AirInstance, ProofCtx};
use sm_common::{CheckPointType, Instance, InstanceExpanderCtx, InstanceType};
use std::sync::Arc;
use zisk_common::{BusDevice, BusId, OperationBusData, OperationData};
use zisk_core::{ZiskOperationType, ZiskRom};
use zisk_pil::BinaryExtensionTrace;
use ziskemu::EmuTrace;
pub struct BinaryExtensionInstance<F: PrimeField> {
    /// Binary extension state machine
    binary_extension_sm: Arc<BinaryExtensionSM<F>>,
    /// Instance expander context
    iectx: InstanceExpanderCtx,

    /// Inputs
    inputs: Vec<OperationData<u64>>,

    skipping: bool,
    skipped: u64,
}
impl<F: PrimeField> BinaryExtensionInstance<F> {
    pub fn new(binary_extension_sm: Arc<BinaryExtensionSM<F>>, iectx: InstanceExpanderCtx) -> Self {
        Self { binary_extension_sm, iectx, inputs: Vec::new(), skipping: true, skipped: 0 }
    }
}
impl<F: PrimeField> Instance<F> for BinaryExtensionInstance<F> {
    fn collect_inputs(
        &mut self,
        _zisk_rom: &ZiskRom,
        _min_traces: &[EmuTrace],
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        Ok(())
    }

    fn compute_witness(&mut self, _pctx: &ProofCtx<F>) -> Option<AirInstance<F>> {
        Some(self.binary_extension_sm.prove_instance(&self.inputs))
    }

    fn check_point(&self) -> CheckPointType {
        self.iectx.plan.check_point.clone()
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
}

unsafe impl<F: PrimeField> Sync for BinaryExtensionInstance<F> {}

impl<F: PrimeField> BusDevice<u64> for BinaryExtensionInstance<F> {
    fn process_data(&mut self, _bus_id: &BusId, data: &[u64]) -> (bool, Vec<(BusId, Vec<u64>)>) {
        let data: OperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");
        let op_type = OperationBusData::get_op_type(&data);

        if op_type as u32 != ZiskOperationType::BinaryE as u32 {
            return (false, vec![]);
        }

        if self.skipping {
            if let CheckPointType::Skip(check_point) = &self.iectx.plan.check_point {
                let skip = check_point.collect_info.skip;
                if skip != 0 && self.skipped != skip {
                    self.skipped += 1;
                    return (false, vec![]);
                }
            }
            self.skipping = false;
        }

        self.inputs.push(data);

        (self.inputs.len() == BinaryExtensionTrace::<usize>::NUM_ROWS, vec![])
    }
}
