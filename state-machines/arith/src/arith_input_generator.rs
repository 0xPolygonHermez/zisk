use p3_field::PrimeField;
use proofman_common::{AirInstance, ProofCtx};
use sm_common::{CheckPoint, Instance};
use zisk_common::{BusDevice, BusId, OperationBusData, OperationData};
use zisk_core::ZiskOperationType;

use crate::ArithFullSM;

pub struct ArithInputGenerator {
    bus_id: BusId,
}

impl ArithInputGenerator {
    pub fn new(bus_id: BusId) -> Self {
        Self { bus_id }
    }
}

impl<F: PrimeField> Instance<F> for ArithInputGenerator {
    fn check_point(&self) -> CheckPoint {
        CheckPoint::None
    }

    fn instance_type(&self) -> sm_common::InstanceType {
        sm_common::InstanceType::Instance
    }

    fn compute_witness(&mut self, _: &ProofCtx<F>) -> Option<AirInstance<F>> {
        None
    }

    fn bus_id(&self) -> Vec<BusId> {
        vec![self.bus_id]
    }
}

impl BusDevice<u64> for ArithInputGenerator {
    fn process_data(&mut self, bus_id: &BusId, data: &[u64]) -> (bool, Vec<(BusId, Vec<u64>)>) {
        let input: OperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");
        let op_type = OperationBusData::get_op_type(&input);

        if op_type as u32 != ZiskOperationType::Arith as u32 {
            return (false, vec![]);
        }

        let inputs = ArithFullSM::generate_inputs(&input)
            .into_iter()
            .map(|x| (*bus_id, x))
            .collect::<Vec<_>>();

        (false, inputs)
    }
}
