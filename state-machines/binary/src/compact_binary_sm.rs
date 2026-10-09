//! Witness of the fused `CompactBinary` air: a slice of each of the four binary families, on the
//! same rows.

use std::sync::Arc;

use proofman_common::{trace::TraceRow, AirInstance, FromTrace, ProofmanResult};
use proofman_fields::PrimeField64;
use zisk_pil::{
    CompactBinaryAirValues, CompactBinaryTrace, CompactBinaryTraceRow, CompactBinaryTraceRowPacked,
};

use crate::{
    BinaryAddHiLaneRow, BinaryAddHiSM, BinaryAddLaneRow, BinaryAddSM, BinaryBasicLaneRow,
    BinaryBasicSM, BinaryExtensionLaneRow, BinaryExtensionSM, BinaryInput,
};

/// The operations of one `CompactBinary` instance, per block and per chunk, as the collectors
/// handed them over.
#[derive(Default)]
pub struct CompactBinaryInputs {
    pub basic: Vec<Vec<BinaryInput>>,
    pub add: Vec<Vec<BinaryInput>>,
    pub add_hi: Vec<Vec<BinaryInput>>,
    pub ext: Vec<Vec<BinaryInput>>,
}

/// Fills the `CompactBinary` trace by running the four fills over the same rows.
///
/// There is no proving logic of its own here: each block is filled by the state machine of the air
/// it comes from, through the row adapters of `compact_binary_rows.rs`, and each fill only ever
/// touches its own columns. What this type adds is the one thing the four cannot do separately --
/// putting their results in a single `AirInstance`.
pub struct CompactBinarySM<F: PrimeField64> {
    basic_sm: Arc<BinaryBasicSM<F>>,
    add_sm: Arc<BinaryAddSM<F>>,
    add_hi_sm: Arc<BinaryAddHiSM<F>>,
    ext_sm: Arc<BinaryExtensionSM<F>>,
}

impl<F: PrimeField64> CompactBinarySM<F> {
    pub fn new(
        basic_sm: Arc<BinaryBasicSM<F>>,
        add_sm: Arc<BinaryAddSM<F>>,
        add_hi_sm: Arc<BinaryAddHiSM<F>>,
        ext_sm: Arc<BinaryExtensionSM<F>>,
    ) -> Arc<Self> {
        Arc::new(Self { basic_sm, add_sm, add_hi_sm, ext_sm })
    }

    pub fn compute_witness(
        &self,
        inputs: &CompactBinaryInputs,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<AirInstance<F>> {
        if packed {
            self.compute_witness_inner::<CompactBinaryTraceRowPacked<F>>(inputs, trace_buffer)
        } else {
            self.compute_witness_inner::<CompactBinaryTraceRow<F>>(inputs, trace_buffer)
        }
    }

    fn compute_witness_inner<R>(
        &self,
        inputs: &CompactBinaryInputs,
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>>
    where
        R: TraceRow
            + BinaryBasicLaneRow<F>
            + BinaryAddLaneRow<F>
            + BinaryAddHiLaneRow<F>
            + BinaryExtensionLaneRow<F>
            + 'static,
    {
        // Taken as-is, not zeroed, exactly as the standalone airs take their buffers: every fill
        // writes every committed column of its block on every row, the padding included, and the
        // `<==` columns are derived by the prover before it commits.
        let mut trace = CompactBinaryTrace::<R>::new_from_vec(trace_buffer)?;

        // The four fills write disjoint columns of the same rows, so the order does not matter.
        let basic_padding = self.basic_sm.fill_rows(trace.buffer.as_mut_slice(), &inputs.basic);
        let add_padding = self.add_sm.fill_rows(trace.buffer.as_mut_slice(), &inputs.add);
        let add_hi_padding = self.add_hi_sm.fill_rows(trace.buffer.as_mut_slice(), &inputs.add_hi);
        let ext_padding = self.ext_sm.fill_rows(trace.buffer.as_mut_slice(), &inputs.ext);

        // Each block cancels its own padding on the bus, through the air value its template
        // declares under the block's prefix.
        let mut air_values = CompactBinaryAirValues::<F>::new();
        air_values.basic_padding_size = F::from_usize(basic_padding);
        air_values.add_padding_size = F::from_usize(add_padding);
        air_values.add_hi_padding_size = F::from_usize(add_hi_padding);
        air_values.ext_padding_size = F::from_usize(ext_padding);

        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace).with_air_values(&mut air_values)))
    }
}
