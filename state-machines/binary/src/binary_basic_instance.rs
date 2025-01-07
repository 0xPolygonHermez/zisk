use crate::BinaryBasicSM;
use p3_field::PrimeField;
use proofman_common::{AirInstance, ProofCtx};
use sm_common::{CheckPoint, CollectInfoSkip, Instance, InstanceCtx, InstanceType};
use std::sync::Arc;
use zisk_common::{BusDevice, BusId, OperationBusData, OperationData};
use zisk_core::ZiskOperationType;
use zisk_pil::BinaryTrace;

pub struct BinaryBasicInstance {
    /// Binary Basic state machine
    binary_basic_sm: Arc<BinaryBasicSM>,

    /// Instance context
    ictx: InstanceCtx,

    /// Skip info
    skip_info: CollectInfoSkip,

    /// Collected inputs
    inputs: Vec<OperationData<u64>>,
}

impl BinaryBasicInstance {
    pub fn new(binary_basic_sm: Arc<BinaryBasicSM>, mut ictx: InstanceCtx) -> Self {
        let collect_info = ictx.plan.collect_info.take().expect("collect_info should be Some");
        let skip_info =
            *collect_info.downcast::<CollectInfoSkip>().expect("Expected CollectInfoSkip");

        Self { binary_basic_sm, ictx, skip_info, inputs: Vec::new() }
    }
}

impl<F: PrimeField> Instance<F> for BinaryBasicInstance {
    fn compute_witness(&mut self, _pctx: &ProofCtx<F>) -> Option<AirInstance<F>> {
        Some(self.binary_basic_sm.prove_instance::<F>(&self.inputs))
    }

    fn check_point(&self) -> CheckPoint {
        self.ictx.plan.check_point.clone()
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
}

impl BusDevice<u64> for BinaryBasicInstance {
    fn process_data(&mut self, _bus_id: &BusId, data: &[u64]) -> (bool, Vec<(BusId, Vec<u64>)>) {
        let data: OperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");
        let op_type = OperationBusData::get_op_type(&data);

        if op_type as u32 != ZiskOperationType::Binary as u32 {
            return (false, vec![]);
        }

        if self.skip_info.should_skip() {
            return (false, vec![]);
        }

        self.inputs.push(data);

        (self.inputs.len() == BinaryTrace::<usize>::NUM_ROWS, vec![])
    }
}
