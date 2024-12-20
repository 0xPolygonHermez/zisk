use p3_field::PrimeField;
use proofman_common::{AirInstance, ProofCtx};

use crate::CheckPoint;

#[derive(Debug, PartialEq)]
pub enum InstanceType {
    Instance,
    Table,
}

pub trait Instance<F: PrimeField>: Send + Sync {
    fn compute_witness(&mut self, pctx: &ProofCtx<F>) -> Option<AirInstance<F>>;

    fn check_point(&self) -> CheckPoint;

    fn instance_type(&self) -> InstanceType;
}

#[macro_export]
macro_rules! table_instance {
    ($InstanceName:ident, $TableSM:ident, $Trace:ident) => {
        use std::sync::Arc;

        use p3_field::PrimeField;

        use proofman_common::{AirInstance, FromTrace, ProofCtx};
        use sm_common::{CheckPoint, Instance, InstanceExpanderCtx, InstanceType};
        use zisk_common::BusId;
        use zisk_pil::$Trace;

        use rayon::prelude::*;

        pub struct $InstanceName {
            /// State machine
            table_sm: Arc<$TableSM>,

            /// Instance expander context
            iectx: InstanceExpanderCtx,
        }

        impl $InstanceName {
            pub fn new(table_sm: Arc<$TableSM>, iectx: InstanceExpanderCtx) -> Self {
                Self { table_sm, iectx }
            }
        }

        unsafe impl Sync for $InstanceName {}

        impl<F: PrimeField> Instance<F> for $InstanceName {
            fn compute_witness(&mut self, pctx: &ProofCtx<F>) -> Option<AirInstance<F>> {
                let mut multiplicity = self.table_sm.detach_multiplicity();

                pctx.dctx_distribute_multiplicity(&mut multiplicity, self.iectx.global_idx);

                let mut trace = $Trace::new();

                trace.buffer[0..trace.num_rows].par_iter_mut().enumerate().for_each(
                    |(i, input)| input.multiplicity = F::from_canonical_u64(multiplicity[i]),
                );

                Some(AirInstance::new_from_trace(FromTrace::new(&mut trace)))
            }

            fn check_point(&self) -> CheckPoint {
                self.iectx.plan.check_point.clone()
            }

            fn instance_type(&self) -> InstanceType {
                InstanceType::Table
            }
        }

        impl zisk_common::BusDevice<u64> for $InstanceName {
            fn process_data(
                &mut self,
                _bus_id: &zisk_common::BusId,
                _data: &[u64],
            ) -> (bool, Vec<(BusId, Vec<u64>)>) {
                (true, vec![])
            }
        }
    };
}

#[macro_export]
macro_rules! instance {
    ($name:ident, $sm:ty, $num_rows:path, $operation:path) => {
        use proofman_common::{AirInstance, ProofCtx};
        use sm_common::{CheckPointSkip, Instance, InstanceType};
        use zisk_common::BusId;

        pub struct $name<F: PrimeField> {
            /// State machine
            sm: Arc<$sm>,

            /// Instance expander context
            iectx: InstanceExpanderCtx,

            /// Inputs
            inputs: Vec<zisk_core::ZiskRequiredOperation>,

            _phantom: std::marker::PhantomData<F>,
        }

        impl<F: PrimeField> $name<F> {
            pub fn new(sm: Arc<$sm>, iectx: InstanceExpanderCtx) -> Self {
                Self { sm, iectx, inputs: Vec::new(), _phantom: std::marker::PhantomData }
            }
        }

        impl<F: PrimeField> Instance<F> for $name<F> {
            fn compute_witness(&mut self, _pctx: &ProofCtx<F>) -> Option<AirInstance<F>> {
                Some(self.sm.prove_instance(&self.inputs))
            }

            fn check_point(&self) -> Option<CheckPointSkip> {
                self.iectx.plan.check_point
            }

            fn instance_type(&self) -> InstanceType {
                InstanceType::Instance
            }
        }

        impl<F: PrimeField> zisk_common::BusDevice<u64> for $name<F> {
            fn process_data(
                &mut self,
                _bus_id: &zisk_common::BusId,
                _data: &[u64],
            ) -> (bool, Vec<(BusId, Vec<u64>)>) {
                (true, vec![])
            }
        }

        unsafe impl<F: PrimeField> Sync for $name<F> {}
    };
}
