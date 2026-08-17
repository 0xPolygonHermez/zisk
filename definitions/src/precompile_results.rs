//! Per-precompile switches for the precompile-results stream.
//!
//! The stream is a single untagged cursor, so each switch gates three things that
//! must agree: the ASM read (`precompile_results_*` in `core/src/zisk_rom_2_asm.rs`),
//! the result recorded by `syscall_*`, and the hint that carries it (`hints/`).
//!
//! Off does not remove the operation — the guest still executes it; the ASM just
//! recomputes it instead of reading a precomputed result.

/// `arith256_mod` — `d = (a * b + c) mod module`.
///
/// Off: `arith256_mod` is a leaf routine used by every 256-bit field/scalar path
/// (`bn254`, `secp256k1`, `secp256r1`, `uint256`, `bigint`), so it is reached from
/// the replay of hints that are still enabled. Both sides must stay in step.
pub const ARITH256MOD_RESULTS: bool = false;

/// `keccakf` — the Keccak-f\[1600\] permutation.
pub const KECCAK_RESULTS: bool = false;

/// `sha256f` — the SHA-256 compression function.
pub const SHA256_RESULTS: bool = false;
