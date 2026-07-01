use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(zisk_guest)] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_param, ziskos_fcall_get, zisklib::FCALL_UINT256_INV_MOD_ID};
    } else {
        use crate::zisklib::fcalls_impl::uint256::uint256_inv_mod;
    }
}

/// Outcome of a modular-inversion fcall.
///
/// Either the inverse exists and is returned, or it does not — in which case the host returns a
/// witness that no inverse can exist.
pub enum ModInvResult {
    /// The inverse `x` such that `a * x ≡ 1 (mod b)`.
    Inverse([u64; 4]),
    /// No inverse exists; witness that `gcd(a, b) > 1`.
    NoInverse { gcd: [u64; 4], qa: [u64; 4], qm: [u64; 4] },
}

/// Given 256-bit unsigned integers `a` and `b`, it computes `x` such that `a * x ≡ 1 (mod b)` if
/// such an `x` exists ([`ModInvResult::Inverse`]); otherwise it returns a witness proving that no
/// inverse exists ([`ModInvResult::NoInverse`]).
///
/// ### Safety
///
/// The caller must ensure that the input pointer is valid and aligned to an 8-byte boundary.
///
/// Note that this is a *free-input call*, meaning the ZisK VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_uint256_inv_mod(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ModInvResult {
    #[cfg(not(zisk_guest))]
    {
        let res = uint256_inv_mod(a, b);
        #[cfg(feature = "hints")]
        {
            hints.push(13);
            match &res {
                ModInvResult::Inverse(inv) => {
                    hints.push(1);
                    hints.extend_from_slice(inv);
                    hints.extend_from_slice(&[0u64; 8]);
                }
                ModInvResult::NoInverse { gcd, qa, qm } => {
                    hints.push(0);
                    hints.extend_from_slice(gcd);
                    hints.extend_from_slice(qa);
                    hints.extend_from_slice(qm);
                }
            }
        }
        res
    }
    #[cfg(zisk_guest)]
    {
        ziskos_fcall_param!(a, 4);
        ziskos_fcall_param!(b, 4);
        ziskos_fcall!(FCALL_UINT256_INV_MOD_ID);

        let has_inv = ziskos_fcall_get();
        if has_inv == 0 {
            // Read the no-inverse witness in the same order the proxy writes it: gcd, qa, qm.
            ModInvResult::NoInverse {
                gcd: [
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                ],
                qa: [
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                ],
                qm: [
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                ],
            }
        } else {
            ModInvResult::Inverse([
                ziskos_fcall_get(),
                ziskos_fcall_get(),
                ziskos_fcall_get(),
                ziskos_fcall_get(),
            ])
        }
    }
}
