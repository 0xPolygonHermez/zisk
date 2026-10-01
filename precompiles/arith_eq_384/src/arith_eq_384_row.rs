//! Parameterized witness-row abstraction for the `ArithEq384` family.
//!
//! One witness computation (`arith_eq_384.rs`, `ArithEq384SM`) serves every `ArithEq384` air: the
//! little-endian one and its `big_endian: 1` twin, each at two heights. The twins commit the same
//! columns except for the six memory operands, which the big-endian air keeps as byte pairs
//! (`x1_c[2]`, ...), so their generated row types differ; this trait is the common setter surface
//! the shared fill writes through, plus the trace lifecycle that ties a row to its concrete
//! `GenericTrace` alias (and therefore to the right `AIR_ID`). Same design as `ArithEqRow` in the
//! `arith_eq` crate.

use proofman_common::trace::TraceRow;
use proofman_common::{AirInstance, ProofmanResult};
use proofman_fields::PrimeField64;

use crate::ARITH_EQ_384_OP_NUM;

/// Setter surface + trace lifecycle for any `ArithEq384` row type.
pub trait ArithEq384Row<F: PrimeField64>: TraceRow {
    /// The concrete `GenericTrace` alias for this air (carries the right `AIR_ID`).
    type Trace;

    /// Human-readable air name (e.g. `"ArithEq384BeTrace"`), for logs.
    const AIR_NAME: &'static str;

    /// Whether this is a `big_endian: 1` air: x1..y3 are committed as byte pairs and range-checked
    /// in the `DualByte` table instead of the 16-bit chunk range, and the air proves the big-endian
    /// opcodes. The chunk setters below take the 16-bit chunk in both cases.
    const BIG_ENDIAN: bool;

    /// Create the trace over an existing field buffer, zeroed (the padding rows must be all zero).
    fn new_trace(buffer: Vec<F>) -> ProofmanResult<Self::Trace>;
    /// Number of rows in the trace.
    fn trace_num_rows(trace: &Self::Trace) -> usize;
    /// Mutable view of the trace rows.
    fn trace_rows(trace: &mut Self::Trace) -> &mut [Self];
    /// Wrap the filled trace into an `AirInstance` tagged with this air.
    fn into_air_instance(trace: &mut Self::Trace) -> AirInstance<F>;

    // The memory operands, as 16-bit chunks (split into bytes by a big-endian row).
    fn set_x1(&mut self, v: u16);
    fn set_y1(&mut self, v: u16);
    fn set_x2(&mut self, v: u16);
    fn set_y2(&mut self, v: u16);
    fn set_x3(&mut self, v: u16);
    fn set_y3(&mut self, v: u16);

    // Quotients and lambda.
    fn set_q0(&mut self, v: u32);
    fn set_q1(&mut self, v: u32);
    fn set_q2(&mut self, v: u32);
    fn set_s(&mut self, v: u32);

    // One-hot operation selector (its clk0 twin is a `<==` column, computed by the prover).
    fn set_all_sel_op(&mut self, sel: &[bool; ARITH_EQ_384_OP_NUM]);

    // Alias-free (less-than-prime) flags and the x1 != x2 helpers.
    fn set_x3_lt(&mut self, on: bool);
    fn set_y3_lt(&mut self, on: bool);
    fn set_x_are_different(&mut self, on: bool);
    fn set_x_delta_chunk_inv(&mut self, v: u64);

    // Concurrent-equation carries and the step/address packing.
    fn set_all_carry(&mut self, carry: &[[u64; 2]; 3]);
    fn set_step_addr(&mut self, v: u64);
}

/// Implements `ArithEq384Row` for one air's unpacked row and its packed twin.
///
/// * `unpacked:` / `packed:` the generated row types (without the `zisk_pil::` prefix);
/// * `trace:` the `GenericTrace` alias of the air;
/// * `big_endian:` `true` for a `big_endian: 1` air (byte-pair operand columns).
#[macro_export]
macro_rules! impl_arith_eq_384_row {
    (
        unpacked: $unpacked:ident,
        packed: $packed:ident,
        trace: $trace:ident,
        big_endian: $big_endian:tt $(,)?
    ) => {
        $crate::impl_arith_eq_384_row!(@row $unpacked, $trace, $big_endian);
        $crate::impl_arith_eq_384_row!(@row $packed, $trace, $big_endian);
    };

    (@row $row:ident, $trace:ident, $big_endian:tt) => {
        impl<F: proofman_fields::PrimeField64> $crate::ArithEq384Row<F> for ::zisk_pil::$row<F> {
            type Trace = ::zisk_pil::$trace<Self>;

            const AIR_NAME: &'static str = ::std::stringify!($trace);
            const BIG_ENDIAN: bool = $big_endian;

            fn new_trace(buffer: ::std::vec::Vec<F>) -> ::proofman_common::ProofmanResult<Self::Trace> {
                ::zisk_pil::$trace::<Self>::new_from_vec_zeroes(buffer)
            }
            fn trace_num_rows(trace: &Self::Trace) -> usize { trace.num_rows() }
            fn trace_rows(trace: &mut Self::Trace) -> &mut [Self] { &mut trace.buffer[..] }
            fn into_air_instance(trace: &mut Self::Trace) -> ::proofman_common::AirInstance<F> {
                ::proofman_common::AirInstance::new_from_trace(::proofman_common::FromTrace::new(trace))
            }

            $crate::impl_arith_eq_384_row!(@mem_cols $big_endian);

            fn set_q0(&mut self, v: u32) { self.set_q0(v); }
            fn set_q1(&mut self, v: u32) { self.set_q1(v); }
            fn set_q2(&mut self, v: u32) { self.set_q2(v); }
            fn set_s(&mut self, v: u32) { self.set_s(v); }
            fn set_all_sel_op(&mut self, sel: &[bool; $crate::ARITH_EQ_384_OP_NUM]) {
                self.set_all_sel_op(sel);
            }
            fn set_x3_lt(&mut self, on: bool) { self.set_x3_lt(on); }
            fn set_y3_lt(&mut self, on: bool) { self.set_y3_lt(on); }
            fn set_x_are_different(&mut self, on: bool) { self.set_x_are_different(on); }
            fn set_x_delta_chunk_inv(&mut self, v: u64) { self.set_x_delta_chunk_inv(v); }
            fn set_all_carry(&mut self, carry: &[[u64; 2]; 3]) { self.set_all_carry(carry); }
            fn set_step_addr(&mut self, v: u64) { self.set_step_addr(v); }
        }
    };

    // Little-endian air: one 16-bit column per operand.
    (@mem_cols false) => {
        fn set_x1(&mut self, v: u16) { self.set_x1(v); }
        fn set_y1(&mut self, v: u16) { self.set_y1(v); }
        fn set_x2(&mut self, v: u16) { self.set_x2(v); }
        fn set_y2(&mut self, v: u16) { self.set_y2(v); }
        fn set_x3(&mut self, v: u16) { self.set_x3(v); }
        fn set_y3(&mut self, v: u16) { self.set_y3(v); }
    };
    // Big-endian air: `x1_c[2]` byte pair, `x1 = x1_c[0] + 256 * x1_c[1]` in the PIL.
    (@mem_cols true) => {
        fn set_x1(&mut self, v: u16) { self.set_all_x1_c(&[v as u8, (v >> 8) as u8]); }
        fn set_y1(&mut self, v: u16) { self.set_all_y1_c(&[v as u8, (v >> 8) as u8]); }
        fn set_x2(&mut self, v: u16) { self.set_all_x2_c(&[v as u8, (v >> 8) as u8]); }
        fn set_y2(&mut self, v: u16) { self.set_all_y2_c(&[v as u8, (v >> 8) as u8]); }
        fn set_x3(&mut self, v: u16) { self.set_all_x3_c(&[v as u8, (v >> 8) as u8]); }
        fn set_y3(&mut self, v: u16) { self.set_all_y3_c(&[v as u8, (v >> 8) as u8]); }
    };
}

impl_arith_eq_384_row!(
    unpacked: ArithEq384TraceRow,
    packed: ArithEq384TraceRowPacked,
    trace: ArithEq384Trace,
    big_endian: false,
);
impl_arith_eq_384_row!(
    unpacked: ArithEq384LargeTraceRow,
    packed: ArithEq384LargeTraceRowPacked,
    trace: ArithEq384LargeTrace,
    big_endian: false,
);
impl_arith_eq_384_row!(
    unpacked: ArithEq384BeTraceRow,
    packed: ArithEq384BeTraceRowPacked,
    trace: ArithEq384BeTrace,
    big_endian: true,
);
impl_arith_eq_384_row!(
    unpacked: ArithEq384BeLargeTraceRow,
    packed: ArithEq384BeLargeTraceRowPacked,
    trace: ArithEq384BeLargeTrace,
    big_endian: true,
);
