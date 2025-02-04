//! The `StdInstance` performs the witness computation based on the provided ROM execution plan
//!
//! This instance is responsible for managing range check operations and processing
//! execution plans specific to the PIL2 standard library.

use std::sync::Arc;

use data_bus::{BusDevice, BusId, PayloadType};
use p3_field::PrimeField;
use pil_std_lib::{RangeCheckAir, Std};
use proofman_common::{AirInstance, ProofCtx};
use sm_common::{BusDeviceWrapper, CheckPoint, Instance, InstanceCtx, InstanceType};

/// The `StdInstance` struct represents an instance to perform the witness computations for PIL2
/// standard library plans.
///
/// It manages range check operations and interacts with the standard library to process
/// execution plans.
pub struct StdInstance<F: PrimeField> {
    /// Reference to the PIL2 standard library.
    std: Arc<Std<F>>,

    /// The instance context.
    ictx: InstanceCtx,
}

impl<F: PrimeField> StdInstance<F> {
    /// Creates a new `StdInstance`.
    ///
    /// # Arguments
    /// * `std` - An `Arc`-wrapped reference to the PIL2 standard library.
    /// * `ictx` - The `InstanceCtx` associated with this instance.
    ///
    /// # Returns
    /// A new `StdInstance` instance initialized with the provided standard library and context.
    pub fn new(std: Arc<Std<F>>, ictx: InstanceCtx) -> Self {
        Self { std, ictx }
    }
}

impl<F: PrimeField> Instance<F> for StdInstance<F> {
    /// Computes the witness for the execution plan using the standard library.
    ///
    /// This method processes the range check type from the execution plan's metadata
    /// and interacts with the standard library to handle input draining.
    ///
    /// # Arguments
    /// * `_pctx` - The proof context, unused in this implementation.
    /// * `_collectors` - A vector of input collectors to process and collect data for witness
    ///
    /// # Returns
    /// Always returns `None` as this instance does not generate an `AirInstance`.
    fn compute_witness(
        &mut self,
        _pctx: &ProofCtx<F>,
        _collectors: Vec<(usize, Box<BusDeviceWrapper<PayloadType>>)>,
    ) -> Option<AirInstance<F>> {
        let plan = &self.ictx.plan;
        let rc_type = plan.meta.as_ref().unwrap().downcast_ref::<RangeCheckAir>().unwrap();

        self.std.drain_inputs(rc_type);

        None
    }

    /// Retrieves the checkpoint associated with this instance.
    ///
    /// # Returns
    /// A `CheckPoint` object representing the checkpoint of the execution plan.
    fn check_point(&self) -> CheckPoint {
        self.ictx.plan.check_point.clone()
    }

    /// Retrieves the type of this instance.
    ///
    /// # Returns
    /// An `InstanceType` representing the type of this instance (`InstanceType::Instance`).
    fn instance_type(&self) -> InstanceType {
        InstanceType::Table
    }
}

impl<F: PrimeField> BusDevice<u64> for StdInstance<F> {
    /// Returns the bus IDs associated with this instance.
    ///
    /// # Returns
    /// A vector containing the connected bus ID.
    fn bus_id(&self) -> Vec<BusId> {
        vec![]
    }

    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
