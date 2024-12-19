use crate::ArithFullSM;
use p3_field::PrimeField;
use proofman_common::{AirInstance, ProofCtx};
use sm_common::{CheckPoint, Instance, InstanceExpanderCtx, InstanceType};
use std::sync::Arc;
use zisk_common::{BusDevice, BusId, OperationBusData, OperationData};
use zisk_core::{ZiskOperationType, ZiskRom};
use zisk_pil::ArithTrace;
use ziskemu::EmuTrace;
pub struct ArithFullInstance {
    /// Arith state machine
    arith_full_sm: Arc<ArithFullSM>,
    /// Instance expander context
    iectx: InstanceExpanderCtx,
    /// Inputs
    inputs: Vec<OperationData<u64>>,

    skipping: bool,
    skipped: u64,
}
impl ArithFullInstance {
    pub fn new(arith_full_sm: Arc<ArithFullSM>, iectx: InstanceExpanderCtx) -> Self {
        Self {
            arith_full_sm,
            iectx,
            inputs: Vec::new(),
            skipping: true,
            skipped: 0,
        }
    }
}
impl<F: PrimeField> Instance<F> for ArithFullInstance {
    fn collect_inputs(
        &mut self,
        _zisk_rom: &ZiskRom,
        _min_traces: &[EmuTrace],
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        Ok(())
    }

    fn compute_witness(&mut self, _pctx: &ProofCtx<F>) -> Option<AirInstance<F>> {
        Some(self.arith_full_sm.prove_instance::<F>(&self.inputs))
    }

    fn check_point(&self) -> Option<CheckPoint> {
        self.iectx.plan.check_point
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
}

unsafe impl Sync for ArithFullInstance {}

impl BusDevice<u64> for ArithFullInstance {
    fn process_data(&mut self, _bus_id: &BusId, data: &[u64]) -> (bool, Vec<(BusId, Vec<u64>)>) {
        let data: OperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");
        let op_type = OperationBusData::get_op_type(&data);

        if op_type as u32 != ZiskOperationType::Arith as u32 {
            return (false, vec![]);
        }

        if self.skipping {
            let checkpoint = self.iectx.plan.check_point.as_ref().unwrap();
            if checkpoint.skip == 0 || self.skipped == checkpoint.skip {
                self.skipping = false;
            } else {
                self.skipped += 1;
                return (false, vec![]);
            }
        }

        self.inputs.push(data);

        (self.inputs.len() == ArithTrace::<usize>::NUM_ROWS, vec![])
    }
}