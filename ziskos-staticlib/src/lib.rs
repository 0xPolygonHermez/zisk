#![cfg_attr(zisk_guest, no_std)]
#![cfg_attr(zisk_guest, feature(core_intrinsics))]
#![cfg_attr(zisk_guest, allow(internal_features))]

// This crate produces libziskos.a for linking by C (or Rust) host programs.
//
// Instead of re-exporting ziskos's public functions directly, we expose a thin
// wrapper per function that:
//   1. calls `reset_sys_alloc()` to rewind ziskos's private bump heap, so every
//      host-facing call starts with the full heap (ziskos allocations are scratch
//      and never persist across calls in the staticlib), and
//   2. forwards to the real implementation, preserving its exact signature.
//
// The wrapper OWNS the public `#[no_mangle]` symbol (e.g. `zkvm_keccak256`). For
// that to be possible without a duplicate-symbol clash, the underlying ziskos
// implementations drop their own `#[no_mangle]` when compiled as part of the
// staticlib (gated on the `zisk_staticlib` cfg in the ziskos crate); they remain
// reachable here only through their Rust path.
//
// The `#[panic_handler]` is required by the staticlib target but not by rlib.

#[cfg(all(feature = "panic-handler", zisk_guest))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    core::intrinsics::abort()
}

/// Defines a `#[no_mangle] extern "C"` wrapper that resets ziskos's bump heap and
/// then forwards to the real implementation given by its full path.
///
/// Usage:
/// ```ignore
/// wrap_export!(fn zkvm_keccak256(data: *const u8, len: usize, output: *mut zkvm_keccak256_hash) -> zkvm_status
///     => ziskos::zisklib::zkvm_accelerators::zkvm_keccak256);
/// ```
///
/// The exported symbol is `$name`; the implementation is imported under a private
/// alias so it does not shadow the wrapper.
#[cfg(zisk_guest)]
macro_rules! wrap_export {
    (
        fn $name:ident ( $( $arg:ident : $argty:ty ),* $(,)? ) $( -> $ret:ty )?
            => $( $seg:ident )::+
    ) => {
        #[no_mangle]
        pub unsafe extern "C" fn $name ( $( $arg : $argty ),* ) $( -> $ret )? {
            // Import the real implementation under a private alias so this
            // wrapper can own the public `$name` symbol.
            use $( $seg )::+ as __inner;

            // Rewind ziskos's private heap before every host-facing call.
            reset_sys_alloc();

            let __ret = __inner( $( $arg ),* );

            // Fold this call's peak heap usage into the running maximum, which
            // `zkvm_deinit` reports at the end of the program.
            #[cfg(feature = "alloc-stats")]
            update_max_used_sys_alloc();

            __ret
        }
    };
}

#[cfg(zisk_guest)]
mod exports {
    use zkvm_interface::{
        zkvm_blake2f_message, zkvm_blake2f_offset, zkvm_blake2f_state, zkvm_bls12_381_fp,
        zkvm_bls12_381_fp2, zkvm_bls12_381_g1_msm_pair, zkvm_bls12_381_g1_point,
        zkvm_bls12_381_g2_msm_pair, zkvm_bls12_381_g2_point, zkvm_bls12_381_pairing_pair,
        zkvm_bn254_g1_point, zkvm_bn254_pairing_pair, zkvm_bn254_scalar, zkvm_keccak256_hash,
        zkvm_kzg_commitment, zkvm_kzg_field_element, zkvm_kzg_proof, zkvm_ripemd160_hash,
        zkvm_secp256k1_hash, zkvm_secp256k1_pubkey, zkvm_secp256k1_signature, zkvm_secp256r1_hash,
        zkvm_secp256r1_pubkey, zkvm_secp256r1_signature, zkvm_sha256_hash, zkvm_status,
    };

    // ziskos's bump-heap reset, exported as a C symbol from the ziskos crate
    // (`#[no_mangle]`, defined only in the staticlib build). Resolved by name at
    // link time — `mod alloc` is private in ziskos so it is not reachable by path.
    extern "C" {
        fn reset_sys_alloc();
        #[cfg(feature = "alloc-stats")]
        fn update_max_used_sys_alloc();
    }

    // --- Standard accelerators (zkvm_accelerators.rs) ------------------------
    wrap_export!(fn zkvm_keccak256(data: *const u8, len: usize, output: *mut zkvm_keccak256_hash) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators::zkvm_keccak256);
    wrap_export!(fn zkvm_sha256(data: *const u8, len: usize, output: *mut zkvm_sha256_hash) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators::zkvm_sha256);
    wrap_export!(fn zkvm_ripemd160(data: *const u8, len: usize, output: *mut zkvm_ripemd160_hash) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators::zkvm_ripemd160);
    wrap_export!(fn zkvm_modexp(base: *const u8, base_len: usize, exp: *const u8, exp_len: usize, modulus: *const u8, mod_len: usize, output: *mut u8) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators::zkvm_modexp);
    wrap_export!(fn zkvm_bn254_g1_add(p1: *const zkvm_bn254_g1_point, p2: *const zkvm_bn254_g1_point, result: *mut zkvm_bn254_g1_point) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators::zkvm_bn254_g1_add);
    wrap_export!(fn zkvm_bn254_g1_mul(point: *const zkvm_bn254_g1_point, scalar: *const zkvm_bn254_scalar, result: *mut zkvm_bn254_g1_point) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators::zkvm_bn254_g1_mul);
    wrap_export!(fn zkvm_bn254_pairing(pairs: *const zkvm_bn254_pairing_pair, num_pairs: usize, verified: *mut bool) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators::zkvm_bn254_pairing);
    wrap_export!(fn zkvm_blake2f(rounds: u32, h: *mut zkvm_blake2f_state, m: *const zkvm_blake2f_message, t: *const zkvm_blake2f_offset, f: u8) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators::zkvm_blake2f);
    wrap_export!(fn zkvm_kzg_point_eval(commitment: *const zkvm_kzg_commitment, z: *const zkvm_kzg_field_element, y: *const zkvm_kzg_field_element, proof: *const zkvm_kzg_proof, verified: *mut bool) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators::zkvm_kzg_point_eval);
    wrap_export!(fn zkvm_bls12_g1_add(p1: *const zkvm_bls12_381_g1_point, p2: *const zkvm_bls12_381_g1_point, result: *mut zkvm_bls12_381_g1_point) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators::zkvm_bls12_g1_add);
    wrap_export!(fn zkvm_bls12_g1_msm(pairs: *const zkvm_bls12_381_g1_msm_pair, num_pairs: usize, result: *mut zkvm_bls12_381_g1_point) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators::zkvm_bls12_g1_msm);
    wrap_export!(fn zkvm_bls12_g2_add(p1: *const zkvm_bls12_381_g2_point, p2: *const zkvm_bls12_381_g2_point, result: *mut zkvm_bls12_381_g2_point) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators::zkvm_bls12_g2_add);
    wrap_export!(fn zkvm_bls12_g2_msm(pairs: *const zkvm_bls12_381_g2_msm_pair, num_pairs: usize, result: *mut zkvm_bls12_381_g2_point) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators::zkvm_bls12_g2_msm);
    wrap_export!(fn zkvm_bls12_pairing(pairs: *const zkvm_bls12_381_pairing_pair, num_pairs: usize, verified: *mut bool) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators::zkvm_bls12_pairing);
    wrap_export!(fn zkvm_bls12_map_fp_to_g1(field_element: *const zkvm_bls12_381_fp, result: *mut zkvm_bls12_381_g1_point) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators::zkvm_bls12_map_fp_to_g1);
    wrap_export!(fn zkvm_bls12_map_fp2_to_g2(field_element: *const zkvm_bls12_381_fp2, result: *mut zkvm_bls12_381_g2_point) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators::zkvm_bls12_map_fp2_to_g2);
    wrap_export!(fn zkvm_secp256r1_verify(msg: *const zkvm_secp256r1_hash, sig: *const zkvm_secp256r1_signature, pubkey: *const zkvm_secp256r1_pubkey, verified: *mut bool) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators::zkvm_secp256r1_verify);
    wrap_export!(fn zkvm_secp256k1_verify(msg: *const zkvm_secp256k1_hash, sig: *const zkvm_secp256k1_signature, pubkey: *const zkvm_secp256k1_pubkey, verified: *mut bool) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators::zkvm_secp256k1_verify);
    wrap_export!(fn zkvm_secp256k1_ecrecover(msg: *const zkvm_secp256k1_hash, sig: *const zkvm_secp256k1_signature, recid: u8, output: *mut zkvm_secp256k1_pubkey) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators::zkvm_secp256k1_ecrecover);

    // --- Standard IO (zkvm_io.rs) -------------------------------------------
    wrap_export!(fn read_input(buf_ptr: *mut *const u8, buf_size: *mut usize)
        => ziskos::zisklib::zkvm_io::read_input);
    wrap_export!(fn write_output(output: *const u8, size: usize)
        => ziskos::zisklib::zkvm_io::write_output);

    // --- Lifecycle (lib.rs) --------------------------------------------------
    wrap_export!(fn zkvm_init() => ziskos::zkvm_init);
    wrap_export!(fn zkvm_deinit() => ziskos::zkvm_deinit);
}
