//! The `Instance` module defines a framework for handling computation instances and state machines
//! in the context of proof systems. It includes traits and macros for defining instances
//! and integrating them with state machines and proofs.

use crate::{BusDevice, CheckPoint, ChunkId, PayloadType, StatsType};
use proofman_common::{AirInstance, ProofCtx, ProofmanError, SetupCtx};
use proofman_fields::PrimeField64;
use std::any::Any;

/// Represents the type of an instance, either a standalone instance or a table.
#[derive(Debug, PartialEq)]
pub enum InstanceType {
    /// A standalone computation instance.
    Instance,

    /// A table-backed computation instance.
    Table,
}

/// A failed witness computation. `trace_buffer` is the pooled buffer the instance was given, when
/// it can still hand it back: the caller then reuses or returns it instead of losing it.
#[derive(Debug)]
pub struct WitnessError<F> {
    /// What failed.
    pub error: ProofmanError,
    /// The trace buffer, when the instance could hand it back.
    pub trace_buffer: Option<Vec<F>>,
}

impl<F> WitnessError<F> {
    /// A failure that hands `trace_buffer` back.
    pub fn with_buffer(error: ProofmanError, trace_buffer: Vec<F>) -> Self {
        Self { error, trace_buffer: Some(trace_buffer) }
    }
}

impl<F> From<ProofmanError> for WitnessError<F> {
    fn from(error: ProofmanError) -> Self {
        Self { error, trace_buffer: None }
    }
}

impl<F> From<std::io::Error> for WitnessError<F> {
    fn from(error: std::io::Error) -> Self {
        ProofmanError::from(error).into()
    }
}

/// The result of [`Instance::compute_witness`].
pub type WitnessResult<F> = Result<Option<AirInstance<F>>, WitnessError<F>>;

/// The `Instance` trait defines the interface for any computation instance used in proof systems.
///
/// It provides methods to compute witnesses, retrieve checkpoints, and specify instance types.
pub trait Instance<F: PrimeField64>: Any + Send + Sync {
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
    ///
    /// # Errors
    ///
    /// Returns an error if the witness computation fails, with `trace_buffer` when the
    /// implementation can still hand it back. Implementations that do no fallible work return `Ok`.
    fn compute_witness(
        &self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        _collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        _trace_buffer: Vec<F>,
        _packed: bool,
    ) -> WitnessResult<F> {
        Ok(None)
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

    /// Provides access to the underlying Any trait for downcasting.
    ///
    /// # Returns
    /// A reference to self as `&dyn Any`.
    fn as_any(&self) -> &dyn Any;

    /// Retrieves the statistics type associated with the instance.
    ///
    /// # Returns
    /// A `StatsType` indicating the type of statistics collected by the instance.
    fn stats_type(&self) -> StatsType {
        StatsType::Other
    }

    /// Resets the state of the instance, if applicable.
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

        use proofman_fields::PrimeField64;

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

            pub fn process_data(&mut self, _bus_id: &BusId, _data: &[u64]) -> bool {
                true
            }
        }

        impl<F: PrimeField64> Instance<F> for $InstanceName {
            fn compute_witness(
                &self,
                pctx: &ProofCtx<F>,
                _sctx: &SetupCtx<F>,
                _collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
                _trace_buffer: Vec<F>,
                _packed: bool,
            ) -> $crate::WitnessResult<F> {
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

                    Ok(Some(AirInstance::new_from_trace(FromTrace::new(&mut trace))))
                } else {
                    multiplicity.par_iter().for_each(|m| {
                        m.swap(0, std::sync::atomic::Ordering::Relaxed);
                    });
                    Ok(None)
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

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        }

        impl BusDevice<u64> for $InstanceName {
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

        use proofman_fields::PrimeField64;

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

            pub fn process_data(&mut self, bus_id: &BusId, data: &[u64]) -> bool {
                true
            }
        }

        impl<F: PrimeField64> Instance<F> for $InstanceName {
            fn compute_witness(
                &self,
                pctx: &ProofCtx<F>,
                _sctx: &SetupCtx<F>,
                _collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
                _trace_buffer: Vec<F>,
                _packed: bool,
            ) -> $crate::WitnessResult<F> {
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

                    Ok(Some(AirInstance::new(TraceInfo::new(
                        trace.airgroup_id,
                        trace.air_id,
                        buffer,
                        false,
                    ))))
                } else {
                    multiplicities.par_iter().for_each(|vec| {
                        for i in 0..vec.len() {
                            vec[i].swap(0, std::sync::atomic::Ordering::Relaxed);
                        }
                    });
                    Ok(None)
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

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        }

        impl BusDevice<u64> for $InstanceName {
            /// Provides a dynamic reference for downcasting purposes.
            fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
                self
            }
        }
    };
}
