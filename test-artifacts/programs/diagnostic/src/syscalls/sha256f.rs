use ziskos::syscalls::*;

pub fn diagnostic_sha256f() {
    //////////////
    // Sha256f Tests
    //////////////

    // Test #0: sha256f
    let mut state: [u64; 4] =
        [13503953895726638695, 11912009169889063794, 11170449402626986623, 6620516960021240235];
    let input: [u64; 8] = [128, 0, 0, 0, 0, 0, 0, 0];
    syscall_sha256_f(&mut state, &input);
    let expected_out: [u64; 4] =
        [11023716863941067842, 11056259177088021704, 7249549980475474404, 8670194910614624539];
    assert_eq!(state, expected_out);
}
