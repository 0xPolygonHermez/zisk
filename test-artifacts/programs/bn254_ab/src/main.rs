#![no_main]
ziskos::entrypoint!(main);

// Matched BN254 pairing-check A/B. e(P,Q) for the generators != 1, so the check
// returns "reject" — but the full Miller loop + final exponentiation still runs,
// which is the work we want to measure. The ziskasm binding returns a status u64;
// the Rust entry returns Result<bool,u8>; both are folded to the same code.
#[cfg(not(feature = "ziskasm"))]
use ziskos::zisklib::pairing_check_safe_bn254;

#[cfg(feature = "ziskasm")]
fn pairing_status(g1: &[[u64; 8]], g2: &[[u64; 16]]) -> u64 {
    zisklib::bn254_pairing_check(g1, g2)
}
#[cfg(not(feature = "ziskasm"))]
fn pairing_status(g1: &[[u64; 8]], g2: &[[u64; 16]]) -> u64 {
    match pairing_check_safe_bn254(g1, g2) {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(e) => 2 + e as u64,
    }
}

fn main() {
    // P = G1 generator (1, 2); Q = G2 generator (test-artifacts bn254 constant).
    let p: [u64; 8] = [1, 0, 0, 0, 2, 0, 0, 0];
    let q: [u64; 16] = [
        0x46DEBD5CD992F6ED, 0x674322D4F75EDADD, 0x426A00665E5C4479, 0x1800DEEF121F1E76,
        0x97E485B7AEF312C2, 0xF1AA493335A9E712, 0x7260BFB731FB5D25, 0x198E9393920D483A,
        0x4CE6CC0166FA7DAA, 0xE3D1E7690C43D37B, 0x4AAB71808DCB408F, 0x12C85EA5DB8C6DEB,
        0x55ACDADCD122975B, 0xBC4B313370B38EF3, 0xEC9E99AD690C3395, 0x090689D0585FF075,
    ];

    let status = pairing_status(&[p], &[q]);
    println!("bn254_ab status={}", status);
}
