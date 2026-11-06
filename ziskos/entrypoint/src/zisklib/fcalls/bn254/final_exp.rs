use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(zisk_guest)] {
        use core::arch::asm;
        use crate::{
            ziskos_fcall, ziskos_fcall_param,
            zisklib::FCALL_BN254_FINAL_EXP_WITNESS_ID,
        };
        #[cfg(not(feature = "inputcpy"))]
        use crate::ziskos_fcall_get;
        #[cfg(feature = "inputcpy")]
        use crate::ziskos_inputcpy;
    } else {
        use crate::zisklib::fcalls_impl::bn254::bn254_final_exp_witness;
    }
}

/// Residue witness of the BN254 final exponentiation for the batch Miller loop of `g1_points`
/// and `g2_points`, computed by the host from the points so that the guest can carry it through
/// its own Miller loop (`miller_loop_batch_with_witness_bn254`).
///
/// Returns `(c⁻¹, s)`; at most `BN254_FINAL_EXP_WITNESS_MAX_PAIRS` pairs.
///
/// Note that this is a *free-input call*, meaning the ZisK VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_bn254_final_exp_witness(
    g1_points: &[[u64; 8]],
    g2_points: &[[u64; 16]],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 48], [u64; 24]) {
    #[cfg(not(zisk_guest))]
    {
        let (c_inv, s) = bn254_final_exp_witness(g1_points, g2_points);
        #[cfg(feature = "hints")]
        {
            hints.push(72);
            hints.extend_from_slice(&c_inv);
            hints.extend_from_slice(&s);
        }
        (c_inv, s)
    }
    #[cfg(zisk_guest)]
    {
        let n = g1_points.len();
        ziskos_fcall_param!(n, 1);
        for (p, q) in g1_points.iter().zip(g2_points) {
            ziskos_fcall_param!(p, 8);
            ziskos_fcall_param!(q, 16);
        }
        ziskos_fcall!(FCALL_BN254_FINAL_EXP_WITNESS_ID);
        #[cfg(not(feature = "inputcpy"))]
        {
            (
                core::array::from_fn(|_| ziskos_fcall_get()),
                core::array::from_fn(|_| ziskos_fcall_get()),
            )
        }
        #[cfg(feature = "inputcpy")]
        {
            use core::mem::MaybeUninit;
            let mut c_inv: MaybeUninit<[u64; 48]> = MaybeUninit::uninit();
            ziskos_inputcpy!(c_inv, 48 * 8);
            let mut s: MaybeUninit<[u64; 24]> = MaybeUninit::uninit();
            ziskos_inputcpy!(s, 24 * 8);
            (unsafe { c_inv.assume_init() }, unsafe { s.assume_init() })
        }
    }
}
