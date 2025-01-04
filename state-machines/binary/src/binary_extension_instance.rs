use crate::BinaryExtensionSM;
use p3_field::PrimeField;
use proofman_common::{AirInstance, ProofCtx};
use sm_common::{CheckPoint, CollectInfoSkip, Instance, InstanceCtx, InstanceType};
use std::sync::Arc;
use zisk_common::{BusDevice, BusId, OperationBusData, OperationData};
use zisk_core::ZiskOperationType;
use zisk_pil::BinaryExtensionTrace;

pub struct BinaryExtensionInstance<F: PrimeField> {
    /// Binary Extension state machine
    binary_extension_sm: Arc<BinaryExtensionSM<F>>,

    /// Instance context
    iectx: InstanceCtx,

    /// Collected inputs
    inputs: Vec<OperationData<u64>>,
}

impl<F: PrimeField> BinaryExtensionInstance<F> {
    pub fn new(binary_extension_sm: Arc<BinaryExtensionSM<F>>, iectx: InstanceCtx) -> Self {
        Self { binary_extension_sm, iectx, inputs: Vec::new() }
    }
}

impl<F: PrimeField> Instance<F> for BinaryExtensionInstance<F> {
    fn compute_witness(&mut self, _pctx: &ProofCtx<F>) -> Option<AirInstance<F>> {
        Some(self.binary_extension_sm.prove_instance(&self.inputs))
    }

    fn check_point(&self) -> CheckPoint {
        self.iectx.plan.check_point.clone()
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
}

impl<F: PrimeField> BusDevice<u64> for BinaryExtensionInstance<F> {
    fn process_data(&mut self, _bus_id: &BusId, data: &[u64]) -> (bool, Vec<(BusId, Vec<u64>)>) {
        let data: OperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");
        let op_type = OperationBusData::get_op_type(&data);

        if op_type as u32 != ZiskOperationType::BinaryE as u32 {
            return (false, vec![]);
        }

        let info_skip = self.iectx.plan.collect_info.as_mut().unwrap();
        let info_skip = info_skip.downcast_mut::<CollectInfoSkip>().unwrap();
        if info_skip.should_skip() {
            return (false, vec![]);
        }

        self.inputs.push(data);

        (self.inputs.len() == BinaryExtensionTrace::<usize>::NUM_ROWS, vec![])
    }
}
