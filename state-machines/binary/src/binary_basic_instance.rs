use crate::BinaryBasicSM;
use p3_field::PrimeField;
use proofman_common::{AirInstance, FromTrace, ProofCtx};
use sm_common::{CheckPoint, Instance, InstanceExpanderCtx, InstanceType};
use std::sync::Arc;
use zisk_common::{BusDevice, BusId, OperationBusData, OperationData};
use zisk_core::{ZiskOperationType, ZiskRom};
use zisk_pil::BinaryTrace;
use ziskemu::EmuTrace;
pub struct BinaryBasicInstance<F: PrimeField> {
    /// Binary basic state machine
    binary_basic_sm: Arc<BinaryBasicSM>,
    /// Instance expander context
    iectx: InstanceExpanderCtx,
    /// Binary trace
    trace: BinaryTrace<F>,
    /// Inputs
    inputs: Vec<OperationData<u64>>,

    skipping: bool,
    skipped: u64,
}
impl<F: PrimeField> BinaryBasicInstance<F> {
    pub fn new(binary_basic_sm: Arc<BinaryBasicSM>, iectx: InstanceExpanderCtx) -> Self {
        Self {
            binary_basic_sm,
            iectx,
            inputs: Vec::new(),
            trace: BinaryTrace::new(),
            skipping: true,
            skipped: 0,
        }
    }
}
impl<F: PrimeField> Instance<F> for BinaryBasicInstance<F> {
    fn collect_inputs(
        &mut self,
        _zisk_rom: &ZiskRom,
        _min_traces: &[EmuTrace],
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        Ok(())
    }

    fn compute_witness(&mut self, _pctx: &ProofCtx<F>) -> Option<AirInstance<F>> {
        self.binary_basic_sm.prove_instance::<F>(&self.inputs);
        Some(AirInstance::new_from_trace(FromTrace::new(&mut self.trace)))
    }

    fn check_point(&self) -> Option<CheckPoint> {
        self.iectx.plan.check_point
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
}

impl<F: PrimeField> BusDevice<u64> for BinaryBasicInstance<F> {
    fn process_data(&mut self, _bus_id: &BusId, data: &[u64]) -> (bool, Vec<(BusId, Vec<u64>)>) {
        let data: OperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");
        let op_type = OperationBusData::get_op_type(&data);

        if op_type as u32 != ZiskOperationType::Binary as u32 {
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

        (self.inputs.len() == self.trace.num_rows(), vec![])
    }
}

unsafe impl<F: PrimeField> Sync for BinaryBasicInstance<F> {}
