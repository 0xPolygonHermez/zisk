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

/// The sub-operations, in the canonical order used by both the PIL selector list and the witness
/// (`SEL_OP_*` in `arith_eq_constants`): the 11 little-endian ones first, then their 11 big-endian
/// twins in the same order.
///
/// A big-endian op (`*Be`) is the same operation with its 256-bit operands stored in memory as
/// big-endian integers. It is a *different bus opcode* (`OP_*_BE`), proved only by the
/// `big_endian: 1` airs, so the counter/planner/collector keep it apart from its little-endian twin;
/// but inside an air it drives the *same* selector column (`sel_arith256`, ...) and the same
/// equations, so the witness fill only needs [`ArithEqOp::little_endian`].
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
    Arith256Be,
    Arith256ModBe,
    Secp256k1AddBe,
    Secp256k1DblBe,
    Bn254CurveAddBe,
    Bn254CurveDblBe,
    Bn254ComplexAddBe,
    Bn254ComplexSubBe,
    Bn254ComplexMulBe,
    Secp256r1AddBe,
    Secp256r1DblBe,
}

impl ArithEqOp {
    /// Index of this operation, matching its `SEL_OP_*` value and its slot in per-op arrays.
    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }

    /// Whether the operands of this op live in memory as big-endian integers.
    #[inline]
    pub fn is_big_endian(self) -> bool {
        self.index() >= crate::ARITH_EQ_LE_OP_NUM
    }

    /// The little-endian twin of this op (itself for a little-endian op): the one whose selector
    /// column and equations it uses inside an air.
    #[inline]
    pub fn little_endian(self) -> ArithEqOp {
        Self::ALL[self.index() % crate::ARITH_EQ_LE_OP_NUM]
    }

    /// The big-endian twin of this op (itself for a big-endian op).
    #[inline]
    pub fn big_endian(self) -> ArithEqOp {
        Self::ALL[self.index() % crate::ARITH_EQ_LE_OP_NUM + crate::ARITH_EQ_LE_OP_NUM]
    }

    /// Map a ZisK opcode (`data[OP]`) to its `ArithEqOp`, or `None` if it isn't an ArithEq sub-op.
    pub fn from_opcode(op: u8) -> Option<ArithEqOp> {
        use zisk_core::zisk_ops::ZiskOp;
        Some(match op {
            ZiskOp::ARITH256 => ArithEqOp::Arith256,
            ZiskOp::ARITH256_MOD => ArithEqOp::Arith256Mod,
            ZiskOp::SECP256K1_ADD => ArithEqOp::Secp256k1Add,
            ZiskOp::SECP256K1_DBL => ArithEqOp::Secp256k1Dbl,
            ZiskOp::BN254_CURVE_ADD => ArithEqOp::Bn254CurveAdd,
            ZiskOp::BN254_CURVE_DBL => ArithEqOp::Bn254CurveDbl,
            ZiskOp::BN254_COMPLEX_ADD => ArithEqOp::Bn254ComplexAdd,
            ZiskOp::BN254_COMPLEX_SUB => ArithEqOp::Bn254ComplexSub,
            ZiskOp::BN254_COMPLEX_MUL => ArithEqOp::Bn254ComplexMul,
            ZiskOp::SECP256R1_ADD => ArithEqOp::Secp256r1Add,
            ZiskOp::SECP256R1_DBL => ArithEqOp::Secp256r1Dbl,
            ZiskOp::ARITH256_BE => ArithEqOp::Arith256Be,
            ZiskOp::ARITH256_MOD_BE => ArithEqOp::Arith256ModBe,
            ZiskOp::SECP256K1_ADD_BE => ArithEqOp::Secp256k1AddBe,
            ZiskOp::SECP256K1_DBL_BE => ArithEqOp::Secp256k1DblBe,
            ZiskOp::BN254_CURVE_ADD_BE => ArithEqOp::Bn254CurveAddBe,
            ZiskOp::BN254_CURVE_DBL_BE => ArithEqOp::Bn254CurveDblBe,
            ZiskOp::BN254_COMPLEX_ADD_BE => ArithEqOp::Bn254ComplexAddBe,
            ZiskOp::BN254_COMPLEX_SUB_BE => ArithEqOp::Bn254ComplexSubBe,
            ZiskOp::BN254_COMPLEX_MUL_BE => ArithEqOp::Bn254ComplexMulBe,
            ZiskOp::SECP256R1_ADD_BE => ArithEqOp::Secp256r1AddBe,
            ZiskOp::SECP256R1_DBL_BE => ArithEqOp::Secp256r1DblBe,
            _ => return None,
        })
    }

    /// All operations, indexed by their `SEL_OP_*` value.
    pub const ALL: [ArithEqOp; 22] = [
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
        ArithEqOp::Arith256Be,
        ArithEqOp::Arith256ModBe,
        ArithEqOp::Secp256k1AddBe,
        ArithEqOp::Secp256k1DblBe,
        ArithEqOp::Bn254CurveAddBe,
        ArithEqOp::Bn254CurveDblBe,
        ArithEqOp::Bn254ComplexAddBe,
        ArithEqOp::Bn254ComplexSubBe,
        ArithEqOp::Bn254ComplexMulBe,
        ArithEqOp::Secp256r1AddBe,
        ArithEqOp::Secp256r1DblBe,
    ];

    /// The little-endian operations, indexed by their `SEL_OP_*` value.
    pub const ALL_LE: [ArithEqOp; 11] = [
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

    /// The big-endian operations, in the order of their little-endian twins.
    pub const ALL_BE: [ArithEqOp; 11] = [
        ArithEqOp::Arith256Be,
        ArithEqOp::Arith256ModBe,
        ArithEqOp::Secp256k1AddBe,
        ArithEqOp::Secp256k1DblBe,
        ArithEqOp::Bn254CurveAddBe,
        ArithEqOp::Bn254CurveDblBe,
        ArithEqOp::Bn254ComplexAddBe,
        ArithEqOp::Bn254ComplexSubBe,
        ArithEqOp::Bn254ComplexMulBe,
        ArithEqOp::Secp256r1AddBe,
        ArithEqOp::Secp256r1DblBe,
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
    /// Whether this is a `big_endian: 1` air: x1..y3 are committed as byte pairs (`x1_c[2]`, ...)
    /// and range-checked in the `DualByte` table instead of the 16-bit chunk range, and the air
    /// proves the big-endian opcodes. The setters below take the 16-bit chunk in both cases.
    const BIG_ENDIAN: bool;

    /// Create the trace over an existing field buffer.
    fn new_trace(buffer: Vec<F>) -> ProofmanResult<Self::Trace>;
    /// Number of rows in the trace.
    fn trace_num_rows(trace: &Self::Trace) -> usize;
    /// Mutable view of the trace rows.
    fn trace_rows(trace: &mut Self::Trace) -> &mut [Self];
    /// Wrap the filled trace into an `AirInstance` tagged with this config's air.
    fn into_air_instance(trace: &mut Self::Trace) -> AirInstance<F>;

    // 16-bit chunk columns — present in every config (as two byte columns in a big-endian air).
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
/// * `big_endian:` whether the air was instantiated with `big_endian: 1` (x1..y3 are `x1_c[2]`...
///   byte pairs; the chunk setters split the value). Optional, defaults to `false`.
/// * `opt:` the optional scalar columns the config has (delegated; the rest stay no-op).
/// * `sels:` for each operation the config instantiates, `Variant => selector setter, clk0 setter`
///   (for a big-endian air, the `*Be` variants, which drive the same selector columns).
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
        $( big_endian: $big_endian:tt, )?
        opt: [ $($opt:ident),* $(,)? ],
        sels: [ $( $variant:ident => $sel_set:ident, $clk0_set:ident ),* $(,)? ]
    ) => {
        $crate::impl_arith_eq_row!(@row $unpacked, $trace, $qs, $use_s, $ceqs,
            big_endian: [ $( $big_endian )? ],
            opt: [ $($opt),* ],
            sels: [ $( $variant => $sel_set, $clk0_set ),* ]);
        $crate::impl_arith_eq_row!(@row $packed, $trace, $qs, $use_s, $ceqs,
            big_endian: [ $( $big_endian )? ],
            opt: [ $($opt),* ],
            sels: [ $( $variant => $sel_set, $clk0_set ),* ]);
    };

    // Internal: implement for a single row type.
    (@row $row:ident, $trace:ident, $qs:literal, $use_s:literal, $ceqs:literal,
        big_endian: [ $( $big_endian:tt )? ],
        opt: [ $($opt:ident),* $(,)? ],
        sels: [ $( $variant:ident => $sel_set:ident, $clk0_set:ident ),* $(,)? ]
    ) => {
            impl<F: proofman_fields::PrimeField64> $crate::ArithEqRow<F> for ::zisk_pil::$row<F> {
                type Trace = ::zisk_pil::$trace<Self>;

                const AIR_NAME: &'static str = ::std::stringify!($trace);
                const QS: usize = $qs;
                const USE_S: bool = $use_s;
                const CEQS: usize = $ceqs;
                const BIG_ENDIAN: bool = false $( || $big_endian )?;

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

                // Always-present columns: the memory operands as 16-bit chunks, or as byte pairs
                // (low byte, high byte) in a big-endian air.
                $crate::impl_arith_eq_row!(@mem_cols [ $( $big_endian )? ]);
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

    // Internal: the six memory-operand setters. Little-endian air: one 16-bit column each.
    (@mem_cols [ ]) => {
        fn set_x1(&mut self, v: u16) { self.set_x1(v); }
        fn set_y1(&mut self, v: u16) { self.set_y1(v); }
        fn set_x2(&mut self, v: u16) { self.set_x2(v); }
        fn set_y2(&mut self, v: u16) { self.set_y2(v); }
        fn set_x3(&mut self, v: u16) { self.set_x3(v); }
        fn set_y3(&mut self, v: u16) { self.set_y3(v); }
    };
    (@mem_cols [ false ]) => { $crate::impl_arith_eq_row!(@mem_cols [ ]); };
    // Big-endian air: `x1_c[2]` byte pair, `x1 = x1_c[0] + 256 * x1_c[1]` in the PIL.
    (@mem_cols [ true ]) => {
        fn set_x1(&mut self, v: u16) { self.set_all_x1_c(&[v as u8, (v >> 8) as u8]); }
        fn set_y1(&mut self, v: u16) { self.set_all_y1_c(&[v as u8, (v >> 8) as u8]); }
        fn set_x2(&mut self, v: u16) { self.set_all_x2_c(&[v as u8, (v >> 8) as u8]); }
        fn set_y2(&mut self, v: u16) { self.set_all_y2_c(&[v as u8, (v >> 8) as u8]); }
        fn set_x3(&mut self, v: u16) { self.set_all_x3_c(&[v as u8, (v >> 8) as u8]); }
        fn set_y3(&mut self, v: u16) { self.set_all_y3_c(&[v as u8, (v >> 8) as u8]); }
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
