//! The `Instance` module defines a framework for handling computation instances and state machines
//! in the context of proof systems. It includes traits and macros for defining instances
//! and integrating them with state machines and proofs.

use p3_field::PrimeField;
use proofman_common::{AirInstance, ProofCtx};
use zisk_common::BusId;

use crate::CheckPoint;

/// Represents the type of an instance, either a standalone instance or a table.
#[derive(Debug, PartialEq)]
pub enum InstanceType {
    /// A standalone computation instance.
    Instance,

    /// A table-backed computation instance.
    Table,
}

/// The `Instance` trait defines the interface for any computation instance used in proof systems.
///
/// It provides methods to compute witnesses, retrieve checkpoints, and specify instance types.
pub trait Instance<F: PrimeField>: Send {
    /// Computes the witness for the instance based on the proof context.
    ///
    /// # Arguments
    /// * `pctx` - The proof context containing necessary information for computation.
    ///
    /// # Returns
    /// An optional `AirInstance` object representing the computed witness.
    fn compute_witness(&mut self, pctx: &ProofCtx<F>) -> Option<AirInstance<F>>;

    /// Retrieves the checkpoint associated with the instance.
    ///
    /// # Returns
    /// A `CheckPoint` object representing the state of the computation plan.
    fn check_point(&self) -> CheckPoint;

    /// Retrieves the type of the instance.
    ///
    /// # Returns
    /// An `InstanceType` indicating whether the instance is standalone or table-based.
    fn instance_type(&self) -> InstanceType;

    /// Returns the bus IDs associated with this instance.
    ///
    /// # Returns
    /// A vector containing the connected bus ID.
    fn bus_id(&self) -> Vec<BusId>;
}

/// Macro to define a table-backed instance.
///
/// This macro automates the creation of an instance that relies on a table state machine
/// and uses a trace structure for witness computation.
///
/// # Parameters
/// * `$InstanceName` - The name of the instance to define.
/// * `$TableSM` - The table state machine associated with the instance.
/// * `$Trace` - The trace structure used for witness computation.
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

        /// Represents an instance backed by a table state machine.
        pub struct $InstanceName {
            /// The table state machine.
            table_sm: Arc<$TableSM>,

            /// The instance context.
            ictx: InstanceCtx,

            /// The connected bus ID.
            bus_id: BusId,
        }

        impl $InstanceName {
            /// Creates a new instance of the table-backed computation instance.
            ///
            /// # Arguments
            /// * `table_sm` - An `Arc` reference to the table state machine.
            /// * `ictx` - The instance context for the computation.
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

/// Macro to define a standalone computation instance.
///
/// This macro automates the creation of a state-machine-based instance and integrates it
/// with a trace and operation structure.
///
/// # Parameters
/// * `$name` - The name of the instance to define.
/// * `$sm` - The state machine associated with the instance.
/// * `$num_rows` - The number of rows in the trace.
/// * `$operation` - The operation structure for computation.
#[macro_export]
macro_rules! instance {
    ($name:ident, $sm:ty, $num_rows:path, $operation:path) => {
        use proofman_common::{AirInstance, ProofCtx};
        use sm_common::{CheckPointSkip, Instance, InstanceType};
        use zisk_common::BusId;

        /// Represents a standalone computation instance.
        pub struct $name<F: PrimeField> {
            /// The state machine.
            sm: Arc<$sm>,

            /// The instance context.
            ictx: InstanceCtx,

            /// Collected inputs for computation.
            inputs: Vec<zisk_core::ZiskRequiredOperation>,

            /// Phantom marker for generic field type.
            _phantom: std::marker::PhantomData<F>,
        }

        impl<F: PrimeField> $name<F> {
            /// Creates a new instance of the standalone computation instance.
            ///
            /// # Arguments
            /// * `sm` - An `Arc` reference to the state machine.
            /// * `ictx` - The instance context for the computation.
            pub fn new(sm: Arc<$sm>, ictx: InstanceCtx) -> Self {
                Self { sm, ictx, inputs: Vec::new(), _phantom: std::marker::PhantomData }
            }
        }

        impl<F: PrimeField> Instance<F> for $name<F> {
            fn compute_witness(&mut self, _pctx: &ProofCtx<F>) -> Option<AirInstance<F>> {
                Some(self.sm.compute_witness(&self.inputs))
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
