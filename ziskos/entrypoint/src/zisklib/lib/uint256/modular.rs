use crate::syscalls::{syscall_arith256_mod, SyscallArith256ModParams};
use crate::zisklib::fcall_bin_decomp;
use crate::zisklib::fcall_uint256_inv_mod;
use crate::zisklib::lib::{
    constants::{ONE_256 as ONE, ZERO_256 as ZERO},
    utils::{be_bytes_to_u64_4, is_one, is_zero, lt, u64_4_to_be_bytes},
};

/// Given 256-bit integers `a` and `modulus`, it computes `a (mod modulus)`.
pub fn reduce_mod256(
    a: &[u64; 4],
    modulus: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    if is_zero(modulus) {
        return ZERO;
    }

    // Only `a < modulus` is already reduced; `a == modulus` and `a > modulus` both need the
    // syscall (e.g. `m mod m = 0`, which the strict-`gt` short-circuit used to miss).
    if lt(a, modulus) {
        *a
    } else {
        let mut d = ZERO;
        let mut params =
            SyscallArith256ModParams { a, b: &ONE, c: &ZERO, module: modulus, d: &mut d };
        syscall_arith256_mod(
            &mut params,
            #[cfg(feature = "hints")]
            hints,
        );
        d
    }
}

/// Given 256-bit integers `a,b` and `modulus`, it computes `(a + b) (mod modulus)`.
pub fn add_mod256(
    a: &[u64; 4],
    b: &[u64; 4],
    modulus: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    if is_zero(modulus) {
        return ZERO;
    }

    let mut d = ZERO;
    let mut params = SyscallArith256ModParams { a, b: &ONE, c: b, module: modulus, d: &mut d };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    d
}

/// Given 256-bit integers `a,b` and `modulus`, it computes `(a * b) (mod modulus)`.
pub fn mul_mod256(
    a: &[u64; 4],
    b: &[u64; 4],
    modulus: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    if is_zero(modulus) {
        return ZERO;
    }

    let mut d = ZERO;
    let mut params = SyscallArith256ModParams { a, b, c: &ZERO, module: modulus, d: &mut d };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    d
}

/// Given 256-bit integers `a` and `modulus`, it computes `a^2 (mod modulus)`.
pub fn square_mod256(
    a: &[u64; 4],
    modulus: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    mul_mod256(
        a,
        a,
        modulus,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// Given 256-bit integers `base`, `exp` and `modulus`, it computes `base^exp (mod modulus)`.
pub fn pow_mod256(
    base: &[u64; 4],
    exp: &[u64; 4],
    modulus: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    if is_zero(modulus) {
        return ZERO;
    }

    // Modulo 1 every result is 0, including the base^0 / 1^exp fast paths below.
    if is_one(modulus) {
        return ZERO;
    }

    // Early returns (modulus > 1 from here on, so `1` and `0` are already reduced)
    if is_zero(exp) {
        // base^0 = 1 (includes 0^0)
        return ONE;
    } else if is_one(exp) {
        // base^1 = base (mod modulus); `base` may be >= modulus and must be reduced
        return reduce_mod256(
            base,
            modulus,
            #[cfg(feature = "hints")]
            hints,
        );
    }

    if is_zero(base) {
        // 0^exp = 0
        return ZERO;
    } else if is_one(base) {
        // 1^exp = 1
        return ONE;
    }
    // We can assume exp,base > 1 from now on

    // Hint the binary decomposition of the exponent (MSB first)
    let (len, bits) = fcall_bin_decomp(
        exp,
        #[cfg(feature = "hints")]
        hints,
    );

    // The leading bit must be 1 for a non-zero exponent
    assert!(len > 0 && bits[0] == 1, "Exponent must be non-zero");

    // Left-to-right square-and-multiply, starting from the second bit
    let mut result = reduce_mod256(
        base,
        modulus,
        #[cfg(feature = "hints")]
        hints,
    );
    let mut rec_exp = [0u64; 4];
    let bit_pos = len - 1;
    rec_exp[bit_pos / 64] = 1u64 << (bit_pos % 64);
    for (bit_idx, &bit) in bits.iter().enumerate().skip(1) {
        if is_zero(&result) {
            return ZERO;
        }

        // Compute result = result² (mod modulus)
        result = square_mod256(
            &result,
            modulus,
            #[cfg(feature = "hints")]
            hints,
        );

        if bit == 1 {
            // Compute result = (result * base) (mod modulus)
            result = mul_mod256(
                &result,
                base,
                modulus,
                #[cfg(feature = "hints")]
                hints,
            );

            // Recompose the exponent
            let bit_pos = len - 1 - bit_idx;
            rec_exp[bit_pos / 64] |= 1u64 << (bit_pos % 64);
        }
    }

    // Verify the hinted decomposition matches the original exponent
    assert_eq!(rec_exp, *exp, "Exponent decomposition mismatch");

    result
}

pub fn inv_mod256(
    a: &[u64; 4],
    modulus: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Option<[u64; 4]> {
    // Hint the inverse
    let inv = fcall_uint256_inv_mod(
        a,
        modulus,
        #[cfg(feature = "hints")]
        hints,
    );

    if let Some(inv) = inv {
        // Verify: a * inv ≡ 1 (mod modulus)
        let result = mul_mod256(
            a,
            &inv,
            modulus,
            #[cfg(feature = "hints")]
            hints,
        );
        assert_eq!(result, ONE, "a * inv must equal 1 mod modulus");

        Some(inv)
    } else {
        None
    }
}

// ==================== C FFI Functions ====================

/// 256-bit modular reduction`.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `modulus_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_reduce_mod256_c")]
pub unsafe extern "C" fn reduce_mod256_c(
    a_ptr: *const u64,
    modulus_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a = &*(a_ptr as *const [u64; 4]);
    let modulus = &*(modulus_ptr as *const [u64; 4]);

    let res = reduce_mod256(
        a,
        modulus,
        #[cfg(feature = "hints")]
        hints,
    );

    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = res;
}

/// 256-bit modular addition.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `b_ptr` must point to a valid `[u64; 4]` array
/// - `modulus_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_add_mod256_c")]
pub unsafe extern "C" fn add_mod256_c(
    a_ptr: *const u64,
    b_ptr: *const u64,
    modulus_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a = &*(a_ptr as *const [u64; 4]);
    let b = &*(b_ptr as *const [u64; 4]);
    let modulus = &*(modulus_ptr as *const [u64; 4]);

    let res = add_mod256(
        a,
        b,
        modulus,
        #[cfg(feature = "hints")]
        hints,
    );

    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = res;
}

/// 256-bit modular multiplication.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `b_ptr` must point to a valid `[u64; 4]` array
/// - `modulus_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_mul_mod256_c")]
pub unsafe extern "C" fn mul_mod256_c(
    a_ptr: *const u64,
    b_ptr: *const u64,
    modulus_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a = &*(a_ptr as *const [u64; 4]);
    let b = &*(b_ptr as *const [u64; 4]);
    let modulus = &*(modulus_ptr as *const [u64; 4]);

    let res = mul_mod256(
        a,
        b,
        modulus,
        #[cfg(feature = "hints")]
        hints,
    );

    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = res;
}

/// 256-bit modular multiplication.
///
/// # Safety
/// - `a` must point to a valid array of 32 bytes (big endian)
/// - `b` must point to a valid array of 32 bytes (big endian)
/// - `m` must point to a valid array of 32 bytes (big endian)
/// - `result` must point to a valid array of at least 32 bytes
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_mul_mod_bytes256_c")]
pub unsafe extern "C" fn mul_mod_bytes256_c(
    a_ptr: *const u8,
    b_ptr: *const u8,
    m_ptr: *const u8,
    result_ptr: *mut u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a_bytes = &*(a_ptr as *const [u8; 32]);
    let b_bytes = &*(b_ptr as *const [u8; 32]);
    let m_bytes = &*(m_ptr as *const [u8; 32]);

    // Convert from big-endian bytes to little-endian
    let a = be_bytes_to_u64_4(a_bytes);
    let b = be_bytes_to_u64_4(b_bytes);
    let m = be_bytes_to_u64_4(m_bytes);

    let result = mul_mod256(
        &a,
        &b,
        &m,
        #[cfg(feature = "hints")]
        hints,
    );

    // Convert result back to big-endian bytes
    let result_bytes = &mut *(result_ptr as *mut [u8; 32]);
    *result_bytes = u64_4_to_be_bytes(&result);
}

/// 256-bit modular reduction `a mod m`, big-endian byte operands.
///
/// # Safety
/// - `a_ptr`, `m_ptr` must each point to a valid array of 32 bytes (big endian)
/// - `result_ptr` must point to a valid array of at least 32 bytes
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_reduce_mod_bytes256_c")]
pub unsafe extern "C" fn reduce_mod_bytes256_c(
    a_ptr: *const u8,
    m_ptr: *const u8,
    result_ptr: *mut u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a = be_bytes_to_u64_4(&*(a_ptr as *const [u8; 32]));
    let m = be_bytes_to_u64_4(&*(m_ptr as *const [u8; 32]));
    let result = reduce_mod256(
        &a,
        &m,
        #[cfg(feature = "hints")]
        hints,
    );
    *(result_ptr as *mut [u8; 32]) = u64_4_to_be_bytes(&result);
}

/// 256-bit modular addition `(a + b) mod m`, big-endian byte operands.
///
/// # Safety
/// - `a_ptr`, `b_ptr`, `m_ptr` must each point to a valid array of 32 bytes (big endian)
/// - `result_ptr` must point to a valid array of at least 32 bytes
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_add_mod_bytes256_c")]
pub unsafe extern "C" fn add_mod_bytes256_c(
    a_ptr: *const u8,
    b_ptr: *const u8,
    m_ptr: *const u8,
    result_ptr: *mut u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a = be_bytes_to_u64_4(&*(a_ptr as *const [u8; 32]));
    let b = be_bytes_to_u64_4(&*(b_ptr as *const [u8; 32]));
    let m = be_bytes_to_u64_4(&*(m_ptr as *const [u8; 32]));
    let result = add_mod256(
        &a,
        &b,
        &m,
        #[cfg(feature = "hints")]
        hints,
    );
    *(result_ptr as *mut [u8; 32]) = u64_4_to_be_bytes(&result);
}

/// 256-bit modular squaring `a² mod m`, big-endian byte operands.
///
/// # Safety
/// - `a_ptr`, `m_ptr` must each point to a valid array of 32 bytes (big endian)
/// - `result_ptr` must point to a valid array of at least 32 bytes
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_square_mod_bytes256_c")]
pub unsafe extern "C" fn square_mod_bytes256_c(
    a_ptr: *const u8,
    m_ptr: *const u8,
    result_ptr: *mut u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a = be_bytes_to_u64_4(&*(a_ptr as *const [u8; 32]));
    let m = be_bytes_to_u64_4(&*(m_ptr as *const [u8; 32]));
    let result = square_mod256(
        &a,
        &m,
        #[cfg(feature = "hints")]
        hints,
    );
    *(result_ptr as *mut [u8; 32]) = u64_4_to_be_bytes(&result);
}

/// 256-bit modular exponentiation `base^exp mod m`, big-endian byte operands.
///
/// # Safety
/// - `base_ptr`, `exp_ptr`, `m_ptr` must each point to a valid array of 32 bytes (big endian)
/// - `result_ptr` must point to a valid array of at least 32 bytes
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_pow_mod_bytes256_c")]
pub unsafe extern "C" fn pow_mod_bytes256_c(
    base_ptr: *const u8,
    exp_ptr: *const u8,
    m_ptr: *const u8,
    result_ptr: *mut u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let base = be_bytes_to_u64_4(&*(base_ptr as *const [u8; 32]));
    let exp = be_bytes_to_u64_4(&*(exp_ptr as *const [u8; 32]));
    let m = be_bytes_to_u64_4(&*(m_ptr as *const [u8; 32]));
    let result = pow_mod256(
        &base,
        &exp,
        &m,
        #[cfg(feature = "hints")]
        hints,
    );
    *(result_ptr as *mut [u8; 32]) = u64_4_to_be_bytes(&result);
}

/// 256-bit modular inverse `a⁻¹ mod m`, big-endian byte operands.
/// Returns 1 if the inverse exists, 0 otherwise.
///
/// # Safety
/// - `a_ptr`, `m_ptr` must each point to a valid array of 32 bytes (big endian)
/// - `result_ptr` must point to a valid array of at least 32 bytes
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_inv_mod_bytes256_c")]
pub unsafe extern "C" fn inv_mod_bytes256_c(
    a_ptr: *const u8,
    m_ptr: *const u8,
    result_ptr: *mut u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let a = be_bytes_to_u64_4(&*(a_ptr as *const [u8; 32]));
    let m = be_bytes_to_u64_4(&*(m_ptr as *const [u8; 32]));
    match inv_mod256(
        &a,
        &m,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Some(res) => {
            *(result_ptr as *mut [u8; 32]) = u64_4_to_be_bytes(&res);
            1
        }
        None => 0,
    }
}

/// 256-bit modular squaring.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `modulus_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_square_mod256_c")]
pub unsafe extern "C" fn square_mod256_c(
    a_ptr: *const u64,
    modulus_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a = &*(a_ptr as *const [u64; 4]);
    let modulus = &*(modulus_ptr as *const [u64; 4]);

    let res = square_mod256(
        a,
        modulus,
        #[cfg(feature = "hints")]
        hints,
    );

    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = res;
}

/// 256-bit modular exponentiation.
///
/// # Safety
/// - `base_ptr` must point to a valid `[u64; 4]` array
/// - `exp_ptr` must point to a valid `[u64; 4]` array
/// - `modulus_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_pow_mod256_c")]
pub unsafe extern "C" fn pow_mod256_c(
    base_ptr: *const u64,
    exp_ptr: *const u64,
    modulus_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let base = &*(base_ptr as *const [u64; 4]);
    let exp = &*(exp_ptr as *const [u64; 4]);
    let modulus = &*(modulus_ptr as *const [u64; 4]);

    let res = pow_mod256(
        base,
        exp,
        modulus,
        #[cfg(feature = "hints")]
        hints,
    );

    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = res;
}

/// 256-bit modular inverse. Returns 1 if the inverse exists, 0 otherwise.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `modulus_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_inv_mod256_c")]
pub unsafe extern "C" fn inv_mod256_c(
    a_ptr: *const u64,
    modulus_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let a = &*(a_ptr as *const [u64; 4]);
    let modulus = &*(modulus_ptr as *const [u64; 4]);

    match inv_mod256(
        a,
        modulus,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Some(res) => {
            let result = &mut *(result_ptr as *mut [u64; 4]);
            *result = res;
            1
        }
        None => 0,
    }
}
