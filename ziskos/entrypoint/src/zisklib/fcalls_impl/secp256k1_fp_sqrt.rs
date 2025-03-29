use lib_c::secp256k1_fp_parity_sqrt_c;

pub fn secp256k1_fp_sqrt(
    params: &[u64],
    results: &mut [u64],
    mem_read: impl Fn(u64) -> u64,
) -> i64 {
    // Extract the input value from the parameters
    let addr = params[0];
    let p_value = [mem_read(addr), mem_read(addr + 8), mem_read(addr + 16), mem_read(addr + 24)];
    let parity = params[1];

    // Perform the inversion
    let res_c_call = secp256k1_fp_parity_sqrt_c(&p_value, parity, results);
    println!("results: {:?}", results);
    if res_c_call == 0 {
        5
    } else {
        res_c_call as i64
    }
}
