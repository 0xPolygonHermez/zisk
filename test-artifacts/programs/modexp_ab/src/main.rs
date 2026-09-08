#![no_main]
ziskos::entrypoint!(main);

use ziskos::zisklib::U256;

// Backend selection — identical to test-artifacts/programs/bigint/src/modexp.rs.
// Default: the Rust zisklib `modexp`, compiled into the guest and run as RISC-V.
#[cfg(not(feature = "ziskasm"))]
use ziskos::zisklib::modexp;

// `ziskasm`: call the flat binding, whose stub is redirected to the hand-written
// `.zisk` routine during transpilation.
#[cfg(feature = "ziskasm")]
fn modexp(base: &[U256], exp: &[u64], modulus: &[U256]) -> Vec<U256> {
    let mut out = vec![0u64; modulus.len() * 4];
    let n = zisklib::modexp_u64(
        U256::slice_to_flat(base),
        exp,
        U256::slice_to_flat(modulus),
        &mut out,
    );
    U256::flat_to_slice(&out[..n]).to_vec()
}

// Single, fixed workload so the ONLY difference between the two builds is which
// modexp implementation serves the call: a 1024-bit modulus (4 U256, all-ones =
// 2^1024 - 1, odd) with base 2 and a 256-bit all-ones exponent — a full
// square-and-multiply over the long (multi-U256) modmul path.
fn main() {
    let base = [U256::TWO];
    let exp = [u64::MAX; 4]; // 256-bit exponent
    let modulus = [U256::MAX; 4]; // 1024-bit modulus

    let res = modexp(&base, &exp, &modulus);

    // Fold the result so the compiler cannot eliminate the computation.
    let flat = U256::slice_to_flat(&res);
    let mut acc = 0u64;
    for l in flat {
        acc ^= *l;
    }
    println!("modexp_ab acc={:016x} limbs={}", acc, flat.len());
}
