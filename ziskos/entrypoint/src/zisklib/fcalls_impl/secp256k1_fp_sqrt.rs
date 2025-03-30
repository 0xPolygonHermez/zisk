use lib_c::secp256k1_fp_parity_sqrt_c;

pub fn secp256k1_fp_sqrt(params: &[u64], results: &mut [u64]) -> i64 {
    let parity = params[4];

    // Perform the inversion
    let res_c_call = secp256k1_fp_parity_sqrt_c(&params, parity, results);
    if res_c_call == 0 {
        5
    } else {
        res_c_call as i64
    }
}
