use crate::BinaryBasicSM;
use p3_field::PrimeField;
use proofman_common::{AirInstance, ProofCtx};
use sm_common::{CheckPoint, CollectInfoSkip, Instance, InstanceExpanderCtx, InstanceType};
use std::sync::Arc;
use zisk_common::{BusDevice, BusId, OperationBusData, OperationData};
use zisk_core::ZiskOperationType;
use zisk_pil::BinaryTrace;

pub struct BinaryBasicInstance {
    /// Binary basic state machine
    binary_basic_sm: Arc<BinaryBasicSM>,
    /// Instance expander context
    iectx: InstanceExpanderCtx,
    /// Inputs
    inputs: Vec<OperationData<u64>>,

    skipping: bool,
    skipped: u64,
}
impl BinaryBasicInstance {
    pub fn new(binary_basic_sm: Arc<BinaryBasicSM>, iectx: InstanceExpanderCtx) -> Self {
        Self { binary_basic_sm, iectx, inputs: Vec::new(), skipping: true, skipped: 0 }
    }
}
impl<F: PrimeField> Instance<F> for BinaryBasicInstance {
    fn compute_witness(&mut self, _pctx: &ProofCtx<F>) -> Option<AirInstance<F>> {
        Some(self.binary_basic_sm.prove_instance::<F>(&self.inputs))
    }

    fn check_point(&self) -> CheckPoint {
        self.iectx.plan.check_point.clone()
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
}

unsafe impl Sync for BinaryBasicInstance {}

impl BusDevice<u64> for BinaryBasicInstance {
    fn process_data(&mut self, _bus_id: &BusId, data: &[u64]) -> (bool, Vec<(BusId, Vec<u64>)>) {
        let data: OperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");
        let op_type = OperationBusData::get_op_type(&data);

        if op_type as u32 != ZiskOperationType::Binary as u32 {
            return (false, vec![]);
        }

        if self.skipping {
            let info_skip = self.iectx.plan.collect_info.as_ref().unwrap();
            let info_skip = info_skip.downcast_ref::<CollectInfoSkip>().unwrap();

            if info_skip.skip == 0 || self.skipped == info_skip.skip {
                self.skipping = false;
            } else {
                self.skipped += 1;
                return (false, vec![]);
            }
        }

        self.inputs.push(data);

        (self.inputs.len() == BinaryTrace::<usize>::NUM_ROWS, vec![])
    }
}
