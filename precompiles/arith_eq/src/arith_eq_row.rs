//! Parameterized witness-row abstraction for the ArithEq family.
//!
//! Goal: one witness computation (`arith_eq.rs`, `ArithEqSM`) that serves every `equations`
//! configuration of the `ArithEq` airtemplate (ArithEq, Arith256X, ArithSecp256K1, ArithBn254, and
//! the `Large` sibling of each) without replicating the file, even though each config is a
//! *different* air with
//! a *different* trace row type, a *different* set of columns, and a *different* `AIR_ID`.
//!
//! Design (no proofman changes; only this crate + the generated `zisk_pil` trace rows):
//!   * `ArithEqRow<F>` is a hand-written trait implemented by every config's generated row
//!     (unpacked *and* packed). It has two parts:
//!       - trace lifecycle (`new_trace` / `trace_num_rows` / `trace_rows` / `into_air_instance`): the
//!         associated `Trace` type ties the row to its concrete `GenericTrace` alias, so the created
//!         trace — and therefore the resulting `AirInstance` — carries the *right* `AIR_ID` even
//!         though `compute_witness` only receives the row type `R`;
//!       - column setters, one per **primary** witness column. Absent columns / disabled operations
//!         are no-ops per config. It intentionally excludes `const expr` (eq_*_chunks, sel_list,
//!         use_*: compile-time only) and `<==` columns (delta_x3, delta_y3: auto-computed).
//!   * `ArithEqSM::compute_witness<R: ArithEqRow<F>>` builds the trace through the lifecycle
//!     methods and fills rows through the setters — a single generic body for all configs.
//!   * Each config's rows implement the trait via `impl_arith_eq_row!` (present columns delegate to
//!     the generated `set_<col>`, absent ones stay no-op, only instantiated ops get a selector arm).
//!   * A config becomes buildable by adding a `zisk_precompile_explicit!(sm = ArithEqSM, trace = …)`
//!     registration pointing every alias at the one shared `ArithEqSM`.

use proofman_common::trace::TraceRow;
use proofman_common::{AirInstance, ProofmanResult};
use proofman_fields::PrimeField64;

/// The 11 sub-operations, in the canonical order used by both the PIL selector list and the witness
/// (`SEL_OP_*` in `arith_eq_constants`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ArithEqOp {
    Arith256,
    Arith256Mod,
    Secp256k1Add,
    Secp256k1Dbl,
    Bn254CurveAdd,
    Bn254CurveDbl,
    Bn254ComplexAdd,
    Bn254ComplexSub,
    Bn254ComplexMul,
    Secp256r1Add,
    Secp256r1Dbl,
}

impl ArithEqOp {
    /// Index of this operation, matching its `SEL_OP_*` value and its slot in per-op arrays.
    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }

    /// Map a ZisK opcode (`data[OP]`) to its `ArithEqOp`, or `None` if it isn't an ArithEq sub-op.
    pub fn from_opcode(op: u8) -> Option<ArithEqOp> {
        use zisk_core::zisk_ops::ZiskOp;
        Some(match op {
            x if x == ZiskOp::Arith256.code() => ArithEqOp::Arith256,
            x if x == ZiskOp::Arith256Mod.code() => ArithEqOp::Arith256Mod,
            x if x == ZiskOp::Secp256k1Add.code() => ArithEqOp::Secp256k1Add,
            x if x == ZiskOp::Secp256k1Dbl.code() => ArithEqOp::Secp256k1Dbl,
            x if x == ZiskOp::Bn254CurveAdd.code() => ArithEqOp::Bn254CurveAdd,
            x if x == ZiskOp::Bn254CurveDbl.code() => ArithEqOp::Bn254CurveDbl,
            x if x == ZiskOp::Bn254ComplexAdd.code() => ArithEqOp::Bn254ComplexAdd,
            x if x == ZiskOp::Bn254ComplexSub.code() => ArithEqOp::Bn254ComplexSub,
            x if x == ZiskOp::Bn254ComplexMul.code() => ArithEqOp::Bn254ComplexMul,
            x if x == ZiskOp::Secp256r1Add.code() => ArithEqOp::Secp256r1Add,
            x if x == ZiskOp::Secp256r1Dbl.code() => ArithEqOp::Secp256r1Dbl,
            _ => return None,
        })
    }

    /// All operations, indexed by their `SEL_OP_*` value.
    pub const ALL: [ArithEqOp; 11] = [
        ArithEqOp::Arith256,
        ArithEqOp::Arith256Mod,
        ArithEqOp::Secp256k1Add,
        ArithEqOp::Secp256k1Dbl,
        ArithEqOp::Bn254CurveAdd,
        ArithEqOp::Bn254CurveDbl,
        ArithEqOp::Bn254ComplexAdd,
        ArithEqOp::Bn254ComplexSub,
        ArithEqOp::Bn254ComplexMul,
        ArithEqOp::Secp256r1Add,
        ArithEqOp::Secp256r1Dbl,
    ];
}

/// Setter surface + trace lifecycle for any ArithEq config. Absent columns / disabled operations are
/// implemented as no-ops per config, so the shared fill logic can call every setter unconditionally.
pub trait ArithEqRow<F: PrimeField64>: TraceRow {
    /// The concrete `GenericTrace` alias for this config (carries the right `AIR_ID`).
    type Trace;

    /// Human-readable air name (e.g. `"Arith256XTrace"`), for logs.
    const AIR_NAME: &'static str;

    // Range-check profile — must mirror exactly which columns this config's PIL range-checks, so the
    // shared witness registers the same std range-check contributions the PIL looks up (otherwise the
    // range-check bus is unbalanced). See `ArithEqSM::compute_witness` padding + `expand_data_on_trace`.
    /// Number of quotient columns present/range-checked: q0..q{QS-1}.
    const QS: usize;
    /// Whether the lambda column `s` is present (range-checked on the chunk range every row).
    const USE_S: bool;
    /// Number of concurrent-equation carry rows this config has (MAX_CEQS: 1, 2, or 3).
    const CEQS: usize;

    /// Create the trace over an existing field buffer.
    fn new_trace(buffer: Vec<F>) -> ProofmanResult<Self::Trace>;
    /// Number of rows in the trace.
    fn trace_num_rows(trace: &Self::Trace) -> usize;
    /// Mutable view of the trace rows.
    fn trace_rows(trace: &mut Self::Trace) -> &mut [Self];
    /// Wrap the filled trace into an `AirInstance` tagged with this config's air.
    fn into_air_instance(trace: &mut Self::Trace) -> AirInstance<F>;

    // 16-bit chunk columns — present in every config.
    fn set_x1(&mut self, v: u16);
    fn set_y1(&mut self, v: u16);
    fn set_x2(&mut self, v: u16);
    fn set_y2(&mut self, v: u16);
    fn set_x3(&mut self, v: u16);
    fn set_y3(&mut self, v: u16);

    // Step/address packing — present in every config.
    fn set_step_addr(&mut self, v: u64);

    // Concurrent-equation carries, full [MAX_CEQS=3][CBC=2]. Reduced configs (MAX_CEQS<3) delegate
    // only the rows they have; the shared logic always passes the full array.
    fn set_carry(&mut self, carry: &[[u64; 2]; 3]);

    // One-hot operation selector and its clk0 twin. Maps the op to the config's named column, or
    // no-op if that op isn't part of this config.
    fn set_sel(&mut self, op: ArithEqOp, on: bool);
    fn set_sel_clk0(&mut self, op: ArithEqOp, on: bool);

    // Quotients / lambda — present only when the config's equations use them (QS / use_s). No-op else.
    fn set_q0(&mut self, _v: u32) {}
    fn set_q1(&mut self, _v: u32) {}
    fn set_q2(&mut self, _v: u32) {}
    fn set_s(&mut self, _v: u32) {}

    // Alias-free (less-than-prime) flags — only when has_check_lt. No-op else.
    fn set_x3_lt(&mut self, _on: bool) {}
    fn set_y3_lt(&mut self, _on: bool) {}

    // EC point-add "x1 != x2" helpers — only when has_check_diff. No-op else.
    fn set_x_are_different(&mut self, _on: bool) {}
    fn set_x_delta_chunk_inv(&mut self, _v: u64) {}
}

/// Implements `ArithEqRow` for one or more generated trace rows sharing the same config
/// (typically the unpacked row and its `…Packed` twin).
///
/// * `rows:` the row types (without the `zisk_pil::` prefix).
/// * `trace:` the `GenericTrace` alias for this air (used as `Alias<Self>`, so the same alias works
///   for both packed and unpacked rows).
/// * `ceqs:` MAX_CEQS for this config — how many `carry` rows the trace actually has (1, 2, or 3).
/// * `opt:` the optional scalar columns the config has (delegated; the rest stay no-op).
/// * `sels:` for each operation the config instantiates, `Variant => selector setter, clk0 setter`.
///
/// ```ignore
/// impl_arith_eq_row!(
///     rows: [Arith256ModTraceRow, Arith256ModTraceRowPacked],
///     trace: Arith256ModTrace,
///     ceqs: 1,
///     opt: [q0, q1, x3_lt, y3_lt],
///     sels: [ Arith256Mod => set_sel_arith256_mod, set_arith256_mod_clk0 ]
/// );
/// ```
#[macro_export]
macro_rules! impl_arith_eq_row {
    // Public entry: implement for a config's unpacked row and its packed twin. Two explicit `@row`
    // calls (no repetition over rows) so `opt`/`sels` stay at their declared nesting depth.
    (
        unpacked: $unpacked:ident,
        packed: $packed:ident,
        trace: $trace:ident,
        qs: $qs:literal,
        use_s: $use_s:literal,
        ceqs: $ceqs:literal,
        opt: [ $($opt:ident),* $(,)? ],
        sels: [ $( $variant:ident => $sel_set:ident, $clk0_set:ident ),* $(,)? ]
    ) => {
        $crate::impl_arith_eq_row!(@row $unpacked, $trace, $qs, $use_s, $ceqs,
            opt: [ $($opt),* ],
            sels: [ $( $variant => $sel_set, $clk0_set ),* ]);
        $crate::impl_arith_eq_row!(@row $packed, $trace, $qs, $use_s, $ceqs,
            opt: [ $($opt),* ],
            sels: [ $( $variant => $sel_set, $clk0_set ),* ]);
    };

    // Internal: implement for a single row type.
    (@row $row:ident, $trace:ident, $qs:literal, $use_s:literal, $ceqs:literal,
        opt: [ $($opt:ident),* $(,)? ],
        sels: [ $( $variant:ident => $sel_set:ident, $clk0_set:ident ),* $(,)? ]
    ) => {
            impl<F: proofman_fields::PrimeField64> $crate::ArithEqRow<F> for ::zisk_pil::$row<F> {
                type Trace = ::zisk_pil::$trace<Self>;

                const AIR_NAME: &'static str = ::std::stringify!($trace);
                const QS: usize = $qs;
                const USE_S: bool = $use_s;
                const CEQS: usize = $ceqs;

                fn new_trace(buffer: ::std::vec::Vec<F>)
                    -> ::proofman_common::ProofmanResult<Self::Trace>
                {
                    ::zisk_pil::$trace::<Self>::new_from_vec(buffer)
                }
                fn trace_num_rows(trace: &Self::Trace) -> usize { trace.num_rows() }
                fn trace_rows(trace: &mut Self::Trace) -> &mut [Self] { &mut trace.buffer[..] }
                fn into_air_instance(trace: &mut Self::Trace) -> ::proofman_common::AirInstance<F> {
                    ::proofman_common::AirInstance::new_from_trace(
                        ::proofman_common::FromTrace::new(trace),
                    )
                }

                // Always-present columns.
                fn set_x1(&mut self, v: u16) { self.set_x1(v); }
                fn set_y1(&mut self, v: u16) { self.set_y1(v); }
                fn set_x2(&mut self, v: u16) { self.set_x2(v); }
                fn set_y2(&mut self, v: u16) { self.set_y2(v); }
                fn set_x3(&mut self, v: u16) { self.set_x3(v); }
                fn set_y3(&mut self, v: u16) { self.set_y3(v); }
                fn set_step_addr(&mut self, v: u64) { self.set_step_addr(v); }
                fn set_carry(&mut self, carry: &[[u64; 2]; 3]) {
                    // The trace's carry is [[u64;2];CEQS]; take the first CEQS equation rows.
                    self.set_all_carry(
                        <&[[u64; 2]; $ceqs]>::try_from(&carry[..$ceqs]).unwrap(),
                    );
                }

                // Optional scalar columns present in this config (others keep the default no-op).
                $( $crate::impl_arith_eq_row!(@opt $opt); )*

                // Selector dispatch: only the ops this config instantiates get an arm.
                fn set_sel(&mut self, op: $crate::ArithEqOp, on: bool) {
                    match op {
                        $( $crate::ArithEqOp::$variant => self.$sel_set(on), )*
                        #[allow(unreachable_patterns)]
                        _ => {}
                    }
                }
                fn set_sel_clk0(&mut self, op: $crate::ArithEqOp, on: bool) {
                    match op {
                        $( $crate::ArithEqOp::$variant => self.$clk0_set(on), )*
                        #[allow(unreachable_patterns)]
                        _ => {}
                    }
                }
            }
    };

    // Internal: one delegating override per optional column, keyed on its name so the signature is
    // fixed here (not at the call site).
    (@opt q0) => { fn set_q0(&mut self, v: u32) { self.set_q0(v); } };
    (@opt q1) => { fn set_q1(&mut self, v: u32) { self.set_q1(v); } };
    (@opt q2) => { fn set_q2(&mut self, v: u32) { self.set_q2(v); } };
    (@opt s)  => { fn set_s(&mut self, v: u32) { self.set_s(v); } };
    (@opt x3_lt) => { fn set_x3_lt(&mut self, on: bool) { self.set_x3_lt(on); } };
    (@opt y3_lt) => { fn set_y3_lt(&mut self, on: bool) { self.set_y3_lt(on); } };
    (@opt x_are_different) => {
        fn set_x_are_different(&mut self, on: bool) { self.set_x_are_different(on); }
    };
    (@opt x_delta_chunk_inv) => {
        fn set_x_delta_chunk_inv(&mut self, v: u64) { self.set_x_delta_chunk_inv(v); }
    };
}
