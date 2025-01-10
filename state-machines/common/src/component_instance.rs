use p3_field::PrimeField;
use proofman_common::{AirInstance, ProofCtx};
use zisk_common::BusId;

use crate::CheckPoint;

#[derive(Debug, PartialEq)]
pub enum InstanceType {
    Instance,
    Table,
}

pub trait Instance<F: PrimeField>: Send {
    fn compute_witness(&mut self, pctx: &ProofCtx<F>) -> Option<AirInstance<F>>;

    fn check_point(&self) -> CheckPoint;

    fn instance_type(&self) -> InstanceType;

    fn bus_id(&self) -> Vec<BusId>;
}

#[macro_export]
macro_rules! table_instance {
    ($InstanceName:ident, $TableSM:ident, $Trace:ident) => {
        use std::sync::Arc;

        use p3_field::PrimeField;

        use proofman_common::{AirInstance, FromTrace, ProofCtx};
        use sm_common::{CheckPoint, Instance, InstanceCtx, InstanceType};
        use zisk_common::BusId;
        use zisk_pil::$Trace;

        use rayon::prelude::*;

        pub struct $InstanceName {
            /// State machine
            table_sm: Arc<$TableSM>,

            /// Instance context
            ictx: InstanceCtx,

            bus_id: BusId,
        }

        impl $InstanceName {
            pub fn new(table_sm: Arc<$TableSM>, ictx: InstanceCtx, bus_id: BusId) -> Self {
                Self { table_sm, ictx, bus_id }
            }
        }

        impl<F: PrimeField> Instance<F> for $InstanceName {
            fn compute_witness(&mut self, pctx: &ProofCtx<F>) -> Option<AirInstance<F>> {
                let mut multiplicity = self.table_sm.detach_multiplicity();

                pctx.dctx_distribute_multiplicity(&mut multiplicity, self.ictx.global_idx);

                let mut trace = $Trace::new();

                trace.buffer[0..trace.num_rows].par_iter_mut().enumerate().for_each(
                    |(i, input)| input.multiplicity = F::from_canonical_u64(multiplicity[i]),
                );

                Some(AirInstance::new_from_trace(FromTrace::new(&mut trace)))
            }

            fn check_point(&self) -> CheckPoint {
                self.ictx.plan.check_point.clone()
            }

            fn instance_type(&self) -> InstanceType {
                InstanceType::Table
            }

            fn bus_id(&self) -> Vec<BusId> {
                vec![self.bus_id]
            }
        }

        impl zisk_common::BusDevice<u64> for $InstanceName {}
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

            /// Instance context
            ictx: InstanceCtx,

            /// Collected inputs
            inputs: Vec<zisk_core::ZiskRequiredOperation>,

            _phantom: std::marker::PhantomData<F>,
        }

        impl<F: PrimeField> $name<F> {
            pub fn new(sm: Arc<$sm>, ictx: InstanceCtx) -> Self {
                Self { sm, ictx, inputs: Vec::new(), _phantom: std::marker::PhantomData }
            }
        }

        impl<F: PrimeField> Instance<F> for $name<F> {
            fn compute_witness(&mut self, _pctx: &ProofCtx<F>) -> Option<AirInstance<F>> {
                Some(self.sm.prove_instance(&self.inputs))
            }

            fn check_point(&self) -> Option<CheckPointSkip> {
                self.ictx.plan.check_point
            }

            fn instance_type(&self) -> InstanceType {
                InstanceType::Instance
            }
        }

        impl<F: PrimeField> zisk_common::BusDevice<u64> for $name<F> {}
    };
}
