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

    /// Collected inputs
    inputs: Vec<OperationData<u64>>,
}

impl ArithFullInstance {
    pub fn new(arith_full_sm: Arc<ArithFullSM>, ictx: InstanceCtx) -> Self {
        Self { arith_full_sm, ictx, inputs: Vec::new() }
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
}

impl BusDevice<u64> for ArithFullInstance {
    fn process_data(&mut self, _bus_id: &BusId, data: &[u64]) -> (bool, Vec<(BusId, Vec<u64>)>) {
        let data: OperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");
        let op_type = OperationBusData::get_op_type(&data);

        if op_type as u32 != ZiskOperationType::Arith as u32 {
            return (false, vec![]);
        }

        let info_skip = self.ictx.plan.collect_info.as_mut().unwrap();
        let info_skip = info_skip.downcast_mut::<CollectInfoSkip>().unwrap();

        if info_skip.should_skip() {
            return (false, vec![]);
        }

        self.inputs.push(data);

        (self.inputs.len() == ArithTrace::<usize>::NUM_ROWS, vec![])
    }
}
