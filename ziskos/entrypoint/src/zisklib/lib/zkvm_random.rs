//! Standard zkVM host randomness implementing the C interface from zkvm_random.h.
//!
//! This function implements the standard interface defined in:
//! https://github.com/eth-act/zkvm-standards/tree/main/standards/host-randomness
//!
//! The values are the SHAKE256 output stream of a 256-bit seed (its 32 bytes), read as
//! consecutive little-endian u64 words. On ZisK the host provides the seed with each
//! execution, and the stream is produced by the ZisK library routine
//! `ziskasm_zkvm_random_u64` (ziskasm/zisklib/zkvm/random.zisk), reached through its
//! zkvmcall. Natively, the same stream is produced here from a seed drawn from the OS.
//!
//! It is incompatible with hints: the native run that produces them would draw other values
//! than the prover's, so the precompile calls it records could differ from the ones the
//! prover executes. In a hints build it panics, and the prover's hints setup rejects a guest
//! that uses it.

/// Returns 64 random bits. Cannot fail.
///
/// # Safety
///
/// Always safe; `unsafe` only because it is the C ABI function of the standard.
#[cfg(zisk_guest)]
#[cfg_attr(not(zisk_staticlib), no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn zkvm_random_u64() -> u64 {
    // The transpiler turns the csrs into a jump to the library routine, which returns
    // straight to the caller; the `ret` is never reached
    core::arch::naked_asm!(
        "csrs {id}, x0",
        "ret",
        id = const zisk_definitions::zkvmcall_id("zkvm_random_u64"),
    )
}

/// The SHAKE256 output stream of a 256-bit seed (given as its little-endian words), as 64-bit
/// words: the same values `ziskasm_zkvm_random_u64` returns for that seed.
#[cfg(all(not(zisk_guest), not(any(feature = "hints", zisk_hints, zisk_hints_debug))))]
use crate::zisklib::RANDOM_SEED_WORDS;

#[cfg(all(not(zisk_guest), not(any(feature = "hints", zisk_hints, zisk_hints_debug))))]
struct Shake256Stream {
    state: [u64; 25],
    next: usize,
}

#[cfg(all(not(zisk_guest), not(any(feature = "hints", zisk_hints, zisk_hints_debug))))]
impl Shake256Stream {
    /// SHAKE256 rate in 64-bit words (136 bytes)
    const RATE: usize = 17;

    fn new(seed: [u64; RANDOM_SEED_WORDS]) -> Self {
        // Absorb the seed as SHAKE256 pads a 32-byte message: the 0x1F domain byte right
        // after it and the final 0x80 in the last byte of the rate
        let mut state = [0u64; 25];
        state[..RANDOM_SEED_WORDS].copy_from_slice(&seed);
        state[RANDOM_SEED_WORDS] = 0x1F;
        state[Self::RATE - 1] = 0x8000_0000_0000_0000;
        Self { state, next: Self::RATE }
    }

    fn next_u64(&mut self) -> u64 {
        if self.next == Self::RATE {
            tiny_keccak::keccakf(&mut self.state);
            self.next = 0;
        }
        let value = self.state[self.next];
        self.next += 1;
        value
    }
}

/// Returns 64 random bits. Cannot fail.
///
/// # Safety
///
/// Always safe; `unsafe` only because it is the C ABI function of the standard.
#[cfg(not(zisk_guest))]
#[cfg_attr(all(not(feature = "hints"), not(zisk_staticlib)), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_zkvm_random_u64")]
pub unsafe extern "C" fn zkvm_random_u64() -> u64 {
    #[cfg(any(feature = "hints", zisk_hints, zisk_hints_debug))]
    panic!(
        "zkvm_random_u64 is incompatible with hints: this native run would draw other random \
         values than the prover's, so the precompile calls it records could differ from the \
         prover's. Run the guest without hints"
    );

    #[cfg(not(any(feature = "hints", zisk_hints, zisk_hints_debug)))]
    {
        static STREAM: std::sync::Mutex<Option<Shake256Stream>> = std::sync::Mutex::new(None);

        let mut stream = STREAM.lock().unwrap();
        stream.get_or_insert_with(|| Shake256Stream::new(rand::random())).next_u64()
    }
}

#[cfg(all(test, not(zisk_guest), not(any(feature = "hints", zisk_hints, zisk_hints_debug))))]
mod tests {
    use super::Shake256Stream;

    #[test]
    fn stream_is_shake256_of_the_seed() {
        // SHAKE256 of the 32 seed bytes 00 01 .. 1f (Python hashlib.shake_256), read as
        // little-endian u64 words; word 17 is the first of the second squeezed block
        let seed = [0x0706050403020100, 0x0f0e0d0c0b0a0908, 0x1716151413121110, 0x1f1e1d1c1b1a1918];
        let mut stream = Shake256Stream::new(seed);
        let words: Vec<u64> = (0..20).map(|_| stream.next_u64()).collect();
        assert_eq!(words[0], 0x0280_ce40_887c_f069);
        assert_eq!(words[1], 0x5b3d_2c88_3909_b34d);
        assert_eq!(words[16], 0x3cba_2be4_176c_f0eb);
        assert_eq!(words[17], 0x4a1a_5f66_7155_f0dc);
        assert_eq!(words[19], 0xafe3_d608_d086_6665);
    }
}
