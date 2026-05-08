use std::any::Any;
use std::sync::Arc;

use fields::PrimeField64;
use proofman_common::{
    trace::TraceRow, AirInstance, ProofCtx, ProofmanError, ProofmanResult, SetupCtx,
};
use serde::de::DeserializeOwned;

/// Mirror of `zisk_pil::ZISK_AIRGROUP_ID`. Inlined here to avoid a
/// dependency on `zisk-pil` from the leaf crate that hosts this trait.
const ZISK_AIRGROUP_ID: usize = 0;

/// Per-SM contract used by the unit-test executor.
pub trait UnitTestSm<F: PrimeField64>: Send + Sync + 'static {
    /// SM-specific input type. Round-trips through JSON via `serde` so the
    /// CLI path and the programmatic path share one wire format.
    type Input: serde::Serialize + DeserializeOwned + Send + Sync + 'static;

    /// The non-packed trace row type the SM writes. Hooks receive
    /// `&mut Self::Row` and call its inherent `set_<col>` setters.
    type Row: TraceRow + Default + Copy + Send + Sync + 'static;

    /// SM-specific runtime handle (typically `<Sm>Manager<F>` or `<Sm>SM<F>`).
    /// Stored erased in the executor's manager registry and downcast back
    /// to this concrete type at each call site.
    type Manager: Send + Sync + 'static;

    /// AIR id this SM corresponds to.
    fn air_id() -> usize;

    /// Human-readable identifier for this SM. Used as the top-level JSON
    /// key on the CLI path (e.g. `"Binary"`) and in error messages.
    fn name() -> &'static str;

    /// Maximum number of inputs that fit in one AIR-instance trace.
    fn chunk_size(mgr: &Self::Manager) -> usize;

    /// Number of trace rows per input. Default 1; multi-row SMs override.
    ///
    /// Variable-row SMs (MemAlign, DMA Dma64Aligned/Unaligned) cannot use
    /// a single constant — for those the default of 1 means hooks receive
    /// the absolute row index in `input_idx` and `clock` is always 0; the
    /// user's hook closure can recompute from `row_idx` if needed.
    fn rows_per_input() -> usize {
        1
    }

    /// Plan AIR instances. Default uniformly chunks by [`Self::chunk_size`].
    /// Variable-row SMs (e.g. MemAlign) override.
    fn plan(
        mgr: &Self::Manager,
        pctx: &ProofCtx<F>,
        inputs: Vec<Self::Input>,
    ) -> ProofmanResult<Vec<(usize, Vec<Self::Input>)>> {
        let chunk_size = Self::chunk_size(mgr);
        let air_id = Self::air_id();
        let mut chunks = Vec::new();
        let mut iter = inputs.into_iter();
        loop {
            let chunk: Vec<Self::Input> = iter.by_ref().take(chunk_size).collect();
            if chunk.is_empty() {
                break;
            }
            let global_id = pctx.add_instance(ZISK_AIRGROUP_ID, air_id)?;
            chunks.push((global_id, chunk));
        }
        Ok(chunks)
    }

    /// Compute the witness for one AIR-instance worth of inputs.
    fn compute_witness(
        mgr: &Self::Manager,
        sctx: &SetupCtx<F>,
        inputs: Vec<Self::Input>,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<AirInstance<F>>;
}

/// Generates a [`UnitTestSm`] impl for a marker struct.
///
/// Two forms:
///
/// **Shorthand** — for SMs whose witness call is just
/// `mgr.<accessor>().compute_witness::<Row>(&[inputs], buf)`. Provide
/// `row`, `row_packed`, and `accessor` instead of writing the
/// packed/non-packed branch by hand:
///
/// ```ignore
/// unit_test_sm! {
///     Blake2Sm => {
///         name: "Blake2",
///         air: BLAKE_2_BR_AIR_IDS[0],
///         input: Blake2Input,
///         manager: Blake2Manager<F>,
///         accessor: blake2_sm,
///         row: Blake2brTraceRow<F>,
///         row_packed: Blake2brTraceRowPacked<F>,
///         rows_per_input: CLOCKS,
///         chunk_size: |mgr| mgr.blake2_sm().num_available_blake2s,
///     }
/// }
/// ```
///
/// **Full form** — for SMs whose call has a different shape (extra
/// `sctx`, packed-as-argument, segment id, …). Provide an explicit
/// `compute` closure:
///
/// ```ignore
/// unit_test_sm! {
///     ArithEqSm => {
///         name: "ArithEq",
///         air: ARITH_EQ_AIR_IDS[0],
///         input: ArithEqInput,
///         row: ArithEqTraceRow<F>,
///         manager: ArithEqManager<F>,
///         rows_per_input: ARITH_EQ_ROWS_BY_OP,
///         chunk_size: |_| ArithEqTrace::<usize>::NUM_ROWS / ARITH_EQ_ROWS_BY_OP,
///         compute: |mgr, sctx, inputs, buf, packed| {
///             let inputs = vec![inputs];
///             if packed {
///                 mgr.arith_eq_sm()
///                     .compute_witness::<ArithEqTraceRowPacked<F>>(sctx, &inputs, buf)
///             } else {
///                 mgr.arith_eq_sm()
///                     .compute_witness::<ArithEqTraceRow<F>>(sctx, &inputs, buf)
///             }
///         },
///     }
/// }
/// ```
///
/// `rows_per_input: <expr>` is optional in either form (default 1).
#[macro_export]
macro_rules! unit_test_sm {
    // Shorthand: `compute` is auto-generated from `row` + `row_packed`.
    // Inner call shape: `sm.compute_witness::<R>(&inputs, buf)` — the
    // marker's `manager` is the inner SM itself (not an orchestrator), so
    // no separate accessor is needed.
    (
        $marker:ident => {
            name: $name:literal,
            air: $air:expr,
            input: $input_ty:ty,
            manager: $mgr_ty:ty,
            row: $row_ty:ty,
            row_packed: $row_packed_ty:ty,
            $( rows_per_input: $rpi:expr, )?
            chunk_size: |$cs_mgr:pat_param| $cs_body:expr $(,)?
        }
    ) => {
        $crate::unit_test_sm! {
            $marker => {
                name: $name,
                air: $air,
                input: $input_ty,
                row: $row_ty,
                manager: $mgr_ty,
                $( rows_per_input: $rpi, )?
                chunk_size: |$cs_mgr| $cs_body,
                compute: |sm, _sctx, inputs, buf, packed| {
                    let inputs = ::std::vec![inputs];
                    if packed {
                        sm.compute_witness::<$row_packed_ty>(&inputs, buf)
                    } else {
                        sm.compute_witness::<$row_ty>(&inputs, buf)
                    }
                },
            }
        }
    };

    // Full form: explicit `compute` closure for non-standard call shapes.
    (
        $marker:ident => {
            name: $name:literal,
            air: $air:expr,
            input: $input_ty:ty,
            row: $row_ty:ty,
            manager: $mgr_ty:ty,
            $( rows_per_input: $rpi:expr, )?
            chunk_size: |$cs_mgr:pat_param| $cs_body:expr,
            compute: |$cw_mgr:pat_param, $cw_sctx:pat_param, $cw_inputs:pat_param, $cw_buf:pat_param, $cw_packed:pat_param| $cw_body:expr
            $(,)?
        }
    ) => {
        pub struct $marker;

        impl<F: ::fields::PrimeField64> $crate::UnitTestSm<F> for $marker {
            type Input = $input_ty;
            type Row = $row_ty;
            type Manager = $mgr_ty;

            fn air_id() -> usize { $air }
            fn name() -> &'static str { $name }
            fn chunk_size($cs_mgr: &Self::Manager) -> usize { $cs_body }
            $( fn rows_per_input() -> usize { $rpi } )?

            fn compute_witness(
                $cw_mgr: &Self::Manager,
                $cw_sctx: &::proofman_common::SetupCtx<F>,
                $cw_inputs: ::std::vec::Vec<Self::Input>,
                $cw_buf: ::std::vec::Vec<F>,
                $cw_packed: bool,
            ) -> ::proofman_common::ProofmanResult<::proofman_common::AirInstance<F>> {
                $cw_body
            }
        }
    };
}

/// Object-safe counterpart to [`UnitTestSm`]. The dispatcher only touches
/// this trait. Each method takes the SM's manager erased as
/// `Arc<dyn Any + Send + Sync>` and downcasts internally.
pub trait DynUnitTestSm<F: PrimeField64>: Send + Sync {
    fn air_id(&self) -> usize;
    fn name(&self) -> &'static str;
    fn rows_per_input(&self) -> usize;
    fn row_size(&self) -> usize;

    fn deserialize_inputs(
        &self,
        arr: Vec<serde_json::Value>,
    ) -> ProofmanResult<Box<dyn Any + Send + Sync>>;

    fn plan_erased(
        &self,
        mgr: &Arc<dyn Any + Send + Sync>,
        pctx: &ProofCtx<F>,
        inputs: Box<dyn Any + Send + Sync>,
    ) -> ProofmanResult<Vec<(usize, Box<dyn Any + Send + Sync>)>>;

    fn compute_witness_erased(
        &self,
        mgr: &Arc<dyn Any + Send + Sync>,
        sctx: &SetupCtx<F>,
        inputs: Box<dyn Any + Send + Sync>,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<AirInstance<F>>;
}

fn downcast_mgr<M: Send + Sync + 'static>(
    mgr: &Arc<dyn Any + Send + Sync>,
    sm_name: &str,
) -> Arc<M> {
    Arc::clone(mgr)
        .downcast::<M>()
        .unwrap_or_else(|_| panic!("manager registry holds wrong type for {sm_name}"))
}

impl<F: PrimeField64, T: UnitTestSm<F>> DynUnitTestSm<F> for T {
    fn air_id(&self) -> usize {
        T::air_id()
    }

    fn name(&self) -> &'static str {
        T::name()
    }

    fn rows_per_input(&self) -> usize {
        T::rows_per_input()
    }

    fn row_size(&self) -> usize {
        <T::Row as TraceRow>::ROW_SIZE
    }

    fn deserialize_inputs(
        &self,
        arr: Vec<serde_json::Value>,
    ) -> ProofmanResult<Box<dyn Any + Send + Sync>> {
        let mut typed: Vec<T::Input> = Vec::with_capacity(arr.len());
        for v in arr {
            let inst: T::Input = serde_json::from_value(v).map_err(|e| {
                ProofmanError::InvalidSetup(format!(
                    "UnitTestSm({}): bad input entry: {}",
                    T::name(),
                    e
                ))
            })?;
            typed.push(inst);
        }
        Ok(Box::new(typed))
    }

    fn plan_erased(
        &self,
        mgr: &Arc<dyn Any + Send + Sync>,
        pctx: &ProofCtx<F>,
        inputs: Box<dyn Any + Send + Sync>,
    ) -> ProofmanResult<Vec<(usize, Box<dyn Any + Send + Sync>)>> {
        let typed: Vec<T::Input> = *inputs.downcast::<Vec<T::Input>>().map_err(|_| {
            ProofmanError::InvalidSetup(format!(
                "UnitTestSm({}): plan: input type mismatch",
                T::name()
            ))
        })?;
        let mgr = downcast_mgr::<T::Manager>(mgr, T::name());
        let chunks = T::plan(&mgr, pctx, typed)?;
        Ok(chunks
            .into_iter()
            .map(|(gid, chunk)| (gid, Box::new(chunk) as Box<dyn Any + Send + Sync>))
            .collect())
    }

    fn compute_witness_erased(
        &self,
        mgr: &Arc<dyn Any + Send + Sync>,
        sctx: &SetupCtx<F>,
        inputs: Box<dyn Any + Send + Sync>,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<AirInstance<F>> {
        let typed: Vec<T::Input> = *inputs.downcast::<Vec<T::Input>>().map_err(|_| {
            ProofmanError::InvalidSetup(format!(
                "UnitTestSm({}): compute_witness: input type mismatch",
                T::name()
            ))
        })?;
        let mgr = downcast_mgr::<T::Manager>(mgr, T::name());
        T::compute_witness(&mgr, sctx, typed, trace_buffer, packed)
    }
}
