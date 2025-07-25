//! The `Instance` module defines a framework for handling computation instances and state machines
//! in the context of proof systems. It includes traits and macros for defining instances
//! and integrating them with state machines and proofs.

use crate::{BusDevice, CheckPoint, ChunkId, PayloadType};
use fields::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, SetupCtx};

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
pub trait Instance<F: PrimeField64>: Send + Sync {
    /// Computes the witness for the instance based on the proof context.
    ///
    /// # Arguments
    /// * `_pctx` - The proof context, unused in this implementation.
    /// * `_sctx` - The setup context, unused in this implementation.
    /// * `_collectors` - A vector of input collectors to process and collect data for witness,
    ///   unused in this implementation
    ///
    /// # Returns
    /// An optional `AirInstance` object representing the computed witness.
    fn compute_witness(
        &self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        _collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        _trace_buffer: Vec<F>,
    ) -> Option<AirInstance<F>> {
        None
    }

    /// Retrieves the checkpoint associated with the instance.
    ///
    /// # Returns
    /// A `CheckPoint` object representing the state of the computation plan.
    fn check_point(&self) -> &CheckPoint;

    /// Retrieves the type of the instance.
    ///
    /// # Returns
    /// An `InstanceType` indicating whether the instance is standalone or table-based.
    fn instance_type(&self) -> InstanceType;

    /// Builds an input collector for the instance.
    ///
    /// # Arguments
    /// * `chunk_id` - The chunk ID associated with the input collector.
    ///
    /// # Returns
    /// An `Option` containing the input collector for the instance.
    fn build_inputs_collector(
        &self,
        _chunk_id: ChunkId,
    ) -> Option<Box<dyn BusDevice<PayloadType>>> {
        None
    }

    /// Debugs the instance.
    ///
    /// # Arguments
    /// * `_pctx` - The proof context, unused in this implementation.
    /// * `_sctx` - The setup context, unused in this implementation.
    fn debug(&self, _pctx: &ProofCtx<F>, _sctx: &SetupCtx<F>) {}

    fn reset(&self) {}
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
        use std::collections::VecDeque;
        use std::sync::Arc;

        use fields::PrimeField64;

        use proofman_common::{AirInstance, FromTrace, ProofCtx, SetupCtx};
        use zisk_common::{
            BusDevice, BusId, CheckPoint, Instance, InstanceCtx, InstanceType, PayloadType,
        };
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

        impl<F: PrimeField64> Instance<F> for $InstanceName {
            fn compute_witness(
                &self,
                pctx: &ProofCtx<F>,
                _sctx: &SetupCtx<F>,
                _collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
                _trace_buffer: Vec<F>,
            ) -> Option<AirInstance<F>> {
                let multiplicity = self.table_sm.detach_multiplicity();
                self.table_sm.set_calculated();

                pctx.dctx_distribute_multiplicity(multiplicity, self.ictx.global_id);

                if pctx.dctx_is_my_instance(self.ictx.global_id) {
                    let mut trace = $Trace::new();

                    trace.row_slice_mut().par_iter_mut().enumerate().for_each(|(i, input)| {
                        input.multiplicity = F::from_u64(
                            multiplicity[i].swap(0, std::sync::atomic::Ordering::Relaxed),
                        )
                    });

                    Some(AirInstance::new_from_trace(FromTrace::new(&mut trace)))
                } else {
                    multiplicity.par_iter().for_each(|m| {
                        m.swap(0, std::sync::atomic::Ordering::Relaxed);
                    });
                    None
                }
            }

            fn check_point(&self) -> &CheckPoint {
                &self.ictx.plan.check_point
            }

            fn instance_type(&self) -> InstanceType {
                InstanceType::Table
            }

            fn reset(&self) {
                self.table_sm.reset_calculated();
            }
        }

        impl BusDevice<u64> for $InstanceName {
            fn process_data(
                &mut self,
                bus_id: &BusId,
                data: &[u64],
                _pending: &mut VecDeque<(BusId, Vec<u64>)>,
            ) -> bool {
                true
            }
            fn bus_id(&self) -> Vec<BusId> {
                vec![self.bus_id]
            }

            /// Provides a dynamic reference for downcasting purposes.
            fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
                self
            }
        }
    };
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
macro_rules! table_instance_array {
    ($InstanceName:ident, $TableSM:ident, $Trace:ident) => {
        use std::collections::VecDeque;
        use std::sync::Arc;

        use fields::PrimeField64;

        use proofman_common::{AirInstance, ProofCtx, SetupCtx, TraceInfo};
        use zisk_common::{
            BusDevice, BusId, CheckPoint, Instance, InstanceCtx, InstanceType, PayloadType,
        };
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

        impl<F: PrimeField64> Instance<F> for $InstanceName {
            fn compute_witness(
                &self,
                pctx: &ProofCtx<F>,
                _sctx: &SetupCtx<F>,
                _collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
                _trace_buffer: Vec<F>,
            ) -> Option<AirInstance<F>> {
                let multiplicities = self.table_sm.detach_multiplicities();
                self.table_sm.set_calculated();
                pctx.dctx_distribute_multiplicities(multiplicities, self.ictx.global_id);

                if pctx.dctx_is_my_instance(self.ictx.global_id) {
                    let mut trace = $Trace::new();

                    let mut buffer = trace.get_buffer();

                    buffer.par_chunks_mut(trace.row_size).enumerate().for_each(|(row, chunk)| {
                        for (col, vec) in multiplicities.iter().enumerate() {
                            chunk[col] =
                                F::from_u64(vec[row].swap(0, std::sync::atomic::Ordering::Relaxed));
                        }
                    });

                    Some(AirInstance::new(TraceInfo::new(
                        trace.airgroup_id,
                        trace.air_id,
                        buffer,
                        false,
                    )))
                } else {
                    multiplicities.par_iter().for_each(|vec| {
                        for i in 0..vec.len() {
                            vec[i].swap(0, std::sync::atomic::Ordering::Relaxed);
                        }
                    });
                    None
                }
            }

            fn check_point(&self) -> &CheckPoint {
                &self.ictx.plan.check_point
            }

            fn instance_type(&self) -> InstanceType {
                InstanceType::Table
            }

            fn reset(&self) {
                self.table_sm.reset_calculated();
            }
        }

        impl BusDevice<u64> for $InstanceName {
            fn process_data(
                &mut self,
                bus_id: &BusId,
                data: &[u64],
                _pending: &mut VecDeque<(BusId, Vec<u64>)>,
            ) -> bool {
                true
            }

            fn bus_id(&self) -> Vec<BusId> {
                vec![self.bus_id]
            }

            /// Provides a dynamic reference for downcasting purposes.
            fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
                self
            }
        }
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
        use data_bus::BusId;
        use proofman_common::{AirInstance, ProofCtx};
        use sm_common::{CheckPointSkip, Instance, InstanceType};

        /// Represents a standalone computation instance.
        pub struct $name {
            /// The state machine.
            sm: Arc<$sm>,

            /// The instance context.
            ictx: InstanceCtx,

            /// Collected inputs for computation.
            inputs: Vec<zisk_core::ZiskRequiredOperation>,
        }

        impl<F: PrimeField64> $name<F> {
            /// Creates a new instance of the standalone computation instance.
            ///
            /// # Arguments
            /// * `sm` - An `Arc` reference to the state machine.
            /// * `ictx` - The instance context for the computation.
            pub fn new(sm: Arc<$sm>, ictx: InstanceCtx) -> Self {
                Self { sm, ictx, inputs: Vec::new() }
            }
        }

        impl<F: PrimeField64> Instance<F> for $name {
            fn compute_witness(
                &self,
                _pctx: &ProofCtx<F>,
                _sctx: &SetupCtx<F>,
            ) -> Option<AirInstance<F>> {
                Some(self.sm.compute_witness(&self.inputs))
            }

            fn check_point(&self) -> Option<CheckPointSkip> {
                self.ictx.plan.check_point
            }

            fn instance_type(&self) -> InstanceType {
                InstanceType::Instance
            }
        }

        impl<F: PrimeField64> data_bus::BusDevice<u64> for $name {}
    };
}
