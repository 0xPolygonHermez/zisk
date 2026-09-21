//! The Rust half of zisk's GPU witness-kernel contract.
//!
//! A kernel crate writes its `.cu` and one [`declare_kernel!`] invocation, and
//! gets the FFI surface, a [`GpuWitnessKernel`] impl the prover dispatch can be
//! generic over, and — through [`assert_matches_cpu`] — a correctness test that
//! is two closures rather than a hundred and fifty lines. The C side of the same
//! contract is `zisk_gpu_witness.cuh`, shipped by `zisk-cuda-build`.
//!
//! Adding a kernel to a state machine should be: the `.cu`, a one-line
//! `build.rs`, a `declare_kernel!`, and a test that says how to make inputs and
//! how the CPU packs them. Everything else belongs here.

use std::ffi::c_void;

/// The C ABI that `ZISK_GPU_WITNESS_ENTRIES` generates, as Rust sees it.
///
/// Implemented by [`declare_kernel!`]; there is no reason to write one by hand.
pub trait GpuWitnessKernel {
    /// Per-operation input, laid out to match the kernel's struct. `#[repr(C)]`.
    type Op: Copy;

    /// Whether a CUDA device is present. Callers report "n/a" rather than
    /// failing when this is false — a CPU-only host is not a broken one.
    fn available() -> bool;

    /// u64 the packed trace of `num_ops` operations occupies. Comes from the
    /// kernel, so the two sides cannot disagree about the buffer size.
    fn out_words(num_ops: usize) -> usize;

    /// u64 per packed trace row. Used to report a mismatch as (row, word).
    fn row_words() -> usize;

    /// Enqueue the kernel against caller-owned device memory.
    ///
    /// Allocates nothing, synchronises with nothing, creates no stream: the
    /// only work added to `stream` is one launch, which is what makes it legal
    /// inside a CUDA graph capture region. `device_id` may be -1 to leave the
    /// current device alone.
    ///
    /// # Safety
    ///
    /// `d_ops` must point to `num_ops` valid `Op` on device `device_id`, and
    /// `d_dst` to at least [`out_words(num_ops)`](Self::out_words) writable u64
    /// on the same device. `stream` must be a `cudaStream_t` of that device and
    /// must outlive the launch. Execution errors surface at the caller's next
    /// synchronisation, not here.
    unsafe fn fill(
        d_ops: *const Self::Op,
        num_ops: usize,
        d_dst: *mut u64,
        device_id: i32,
        stream: *mut c_void,
    ) -> Result<(), i32>;

    /// Drive [`fill`](Self::fill) against device memory the kernel side owns.
    ///
    /// Test scaffolding — it creates a stream and buffers per call, which is
    /// exactly what the prover does not do. It exists so the correctness test
    /// exercises the entry the prover uses instead of a parallel benchmark path.
    fn fill_probe(ops: &[Self::Op], out: &mut [u64]) -> Result<(), i32>;
}

/// Declare a kernel's FFI and implement [`GpuWitnessKernel`] for a marker type.
///
/// `prefix` must match the first argument of the kernel's
/// `ZISK_GPU_WITNESS_ENTRIES`, which is what names the symbols.
///
/// ```ignore
/// zisk_gpu_witness::declare_kernel! {
///     /// Keccakf packed-witness kernel.
///     pub Keccakf, prefix = keccakf, op = GpuOp,
/// }
/// ```
#[macro_export]
macro_rules! declare_kernel {
    (
        $(#[$meta:meta])*
        $vis:vis $marker:ident, prefix = $prefix:ident, op = $op:ty $(,)?
    ) => {
        $crate::__paste::paste! {
            extern "C" {
                fn [<$prefix _gpu_available>]() -> i32;
                fn [<$prefix _gpu_out_words>](num_ops: u32) -> u64;
                fn [<$prefix _gpu_row_words>]() -> i32;
                // Signature fixed by the prover's registry; see zisk_gpu_witness.cuh.
                fn [<$prefix _gpu_fill>](
                    d_ops: *const ::std::ffi::c_void,
                    num_ops: u64,
                    d_dst: *mut u64,
                    device_id: i32,
                    stream: *mut ::std::ffi::c_void,
                ) -> i32;
                fn [<$prefix _gpu_fill_probe>](
                    ops: *const $op,
                    num_ops: u32,
                    out_host: *mut u64,
                ) -> i32;
            }

            $(#[$meta])*
            $vis struct $marker;

            impl $marker {
                /// The raw entry point, typed exactly as the prover's registry stores
                /// it. This is what goes into a `GpuWitnessAir` declaration; fn-pointer
                /// types are structural, so it unifies with the prover's alias without
                /// this crate depending on it.
                pub const FILL: unsafe extern "C" fn(
                    *const ::std::ffi::c_void,
                    u64,
                    *mut u64,
                    i32,
                    *mut ::std::ffi::c_void,
                ) -> i32 = [<$prefix _gpu_fill>];

                /// Bytes one staged operation occupies, for the declaration.
                pub const BYTES_PER_OP: u64 = ::std::mem::size_of::<$op>() as u64;
            }

            impl $crate::GpuWitnessKernel for $marker {
                type Op = $op;

                fn available() -> bool {
                    unsafe { [<$prefix _gpu_available>]() == 1 }
                }

                fn out_words(num_ops: usize) -> usize {
                    unsafe { [<$prefix _gpu_out_words>](num_ops as u32) as usize }
                }

                fn row_words() -> usize {
                    unsafe { [<$prefix _gpu_row_words>]() as usize }
                }

                unsafe fn fill(
                    d_ops: *const $op,
                    num_ops: usize,
                    d_dst: *mut u64,
                    device_id: i32,
                    stream: *mut ::std::ffi::c_void,
                ) -> ::std::result::Result<(), i32> {
                    match [<$prefix _gpu_fill>](
                        d_ops as *const ::std::ffi::c_void, num_ops as u64, d_dst, device_id, stream,
                    ) {
                        0 => ::std::result::Result::Ok(()),
                        rc => ::std::result::Result::Err(rc),
                    }
                }

                fn fill_probe(ops: &[$op], out: &mut [u64]) -> ::std::result::Result<(), i32> {
                    assert_eq!(
                        out.len(),
                        <Self as $crate::GpuWitnessKernel>::out_words(ops.len()),
                        "output buffer has the wrong length",
                    );
                    match unsafe {
                        [<$prefix _gpu_fill_probe>](ops.as_ptr(), ops.len() as u32, out.as_mut_ptr())
                    } {
                        0 => ::std::result::Result::Ok(()),
                        rc => ::std::result::Result::Err(rc),
                    }
                }
            }
        }
    };
}

#[doc(hidden)]
pub use paste as __paste;

/// Operation counts every kernel should be compared at.
///
/// Odd counts exercise a trailing slot with its second operation absent; 31, 32
/// and 33 straddle a warp, which is where the cross-lane exchanges break if the
/// launch geometry is wrong. These are the shapes that have actually caught
/// bugs, so they are the default rather than a suggestion.
pub const DEFAULT_COMPARE_COUNTS: &[usize] = &[1, 2, 3, 4, 31, 32, 33, 64, 257];

/// Assert a kernel reproduces its CPU path's packed trace, word for word.
///
/// The caller supplies the two SM-specific things: how to make `n` inputs from
/// a seed, and how the CPU packs those same inputs. Everything else — running
/// the kernel through [`fill_probe`](GpuWitnessKernel::fill_probe), locating the
/// first differing word and reporting it as (row, word) — is here.
///
/// No-ops with a note when no CUDA device is present, so the test is safe to
/// run unconditionally in CI.
pub fn assert_matches_cpu<K, In>(
    counts: &[usize],
    gen_inputs: impl Fn(usize, u64) -> Vec<In>,
    cpu_packed: impl Fn(&[In]) -> Vec<u64>,
) where
    K: GpuWitnessKernel,
    for<'a> K::Op: From<&'a In>,
{
    if !K::available() {
        eprintln!("no CUDA device — skipping");
        return;
    }
    let row_words = K::row_words();
    for &count in counts {
        // Seed per count, so a failure at one size reproduces on its own.
        let inputs = gen_inputs(count, 0x9e37_79b9_7f4a_7c15 ^ count as u64);
        let cpu = cpu_packed(&inputs);
        let ops: Vec<K::Op> = inputs.iter().map(K::Op::from).collect();

        let mut gpu = vec![0u64; K::out_words(ops.len())];
        K::fill_probe(&ops, &mut gpu)
            .unwrap_or_else(|rc| panic!("count={count}: fill failed ({rc})"));

        assert_eq!(
            cpu.len(),
            gpu.len(),
            "count={count}: CPU produced {} words, kernel {}",
            cpu.len(),
            gpu.len()
        );
        if let Some(at) = cpu.iter().zip(&gpu).position(|(c, g)| c != g) {
            panic!(
                "count={count}: row {} word {}: cpu {:#018x} gpu {:#018x}",
                at / row_words,
                at % row_words,
                cpu[at],
                gpu[at],
            );
        }
    }
}
