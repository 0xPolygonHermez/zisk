//! Development tooling: nothing here is part of the witness computation.
//!
//! * `arith_carry_range` — computes the exact range of the `carry` columns and checks that
//!   `ArithRangeTable` covers it.
//! * `arith_table_decode` / `arith_table_decode_gen` — decode `ArithTable` into the canonical text
//!   form committed at `docs/arith_table.txt`, so that the semantic delta of a change to the table
//!   is reviewable as a diff.
//!
//! `arith_carry_range` and `arith_table_decode_gen` are the `[[bin]]` targets; `arith_table_decode`
//! is a library module because its decoding is shared with the test that guards the committed file.

mod arith_table_decode;
pub use arith_table_decode::*;
