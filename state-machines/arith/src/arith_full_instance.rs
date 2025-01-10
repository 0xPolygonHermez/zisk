use crate::ArithFullSM;
use p3_field::PrimeField;
use proofman_common::{AirInstance, ProofCtx};
use sm_common::{CheckPoint, CollectInfoSkip, Instance, InstanceCtx, InstanceType};
use std::sync::Arc;
use zisk_common::{BusDevice, BusId, OperationBusData, OperationData};
use zisk_core::ZiskOperationType;
use zisk_pil::ArithTrace;

pub struct ArithFullInstance {
    /// Arith state machine
    arith_full_sm: Arc<ArithFullSM>,

    /// Instance context
    ictx: InstanceCtx,

    /// Skip info
    skip_info: CollectInfoSkip,

    /// Collected inputs
    inputs: Vec<OperationData<u64>>,

    bus_id: BusId,
}

impl ArithFullInstance {
    pub fn new(arith_full_sm: Arc<ArithFullSM>, mut ictx: InstanceCtx, bus_id: BusId) -> Self {
        let collect_info = ictx.plan.collect_info.take().expect("collect_info should be Some");
        let skip_info =
            *collect_info.downcast::<CollectInfoSkip>().expect("Expected CollectInfoSkip");

        Self { arith_full_sm, ictx, skip_info, inputs: Vec::new(), bus_id }
    }
}

impl<F: PrimeField> Instance<F> for ArithFullInstance {
    fn compute_witness(&mut self, _pctx: &ProofCtx<F>) -> Option<AirInstance<F>> {
        Some(self.arith_full_sm.prove_instance::<F>(&self.inputs))
    }

    fn check_point(&self) -> CheckPoint {
        self.ictx.plan.check_point.clone()
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }

    fn bus_id(&self) -> Vec<BusId> {
        vec![self.bus_id]
    }
}

impl BusDevice<u64> for ArithFullInstance {
    fn process_data(&mut self, _bus_id: &BusId, data: &[u64]) -> (bool, Vec<(BusId, Vec<u64>)>) {
        let data: OperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");
        let op_type = OperationBusData::get_op_type(&data);

        if op_type as u32 != ZiskOperationType::Arith as u32 {
            return (false, vec![]);
        }

        if self.skip_info.should_skip() {
            return (false, vec![]);
        }

        self.inputs.push(data);

        (self.inputs.len() == ArithTrace::<usize>::NUM_ROWS, vec![])
    }
}
