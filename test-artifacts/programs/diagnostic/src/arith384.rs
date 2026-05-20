use ziskos::syscalls::*;

pub fn test_arith384() {
    //////////////
    // Arith384 Tests
    //////////////

    let a: [u64; 6] = [0, 0, 0, 0, 0, 0];
    let b: [u64; 6] = [0, 0, 0, 0, 0, 0];
    let c: [u64; 6] = [0, 0, 0, 0, 0, 0];
    let module: [u64; 6] = [0, 0, 0, 0, 0, 0];
    let mut d: [u64; 6] = [0, 0, 0, 0, 0, 0];
    let mut params = SyscallArith384ModParams { a: &a, b: &b, c: &c, module: &module, d: &mut d };

    // Test #0: arith384_mod
    params.a = &[0, 0, 0, 0, 0, 0];
    params.b = &[0, 0, 0, 0, 0, 0];
    params.c = &[0, 0, 0, 0, 0, 0];
    params.module = &[1, 0, 0, 0, 0, 0];
    syscall_arith384_mod(&mut params);
    let expected_d: [u64; 6] = [0, 0, 0, 0, 0, 0];
    assert_eq!(params.d, &expected_d);

    params.a = &[1, 0, 0, 0, 0, 0];
    params.b = &[0, 0, 0, 0, 0, 0];
    params.c = &[0, 0, 0, 0, 0, 0];
    params.module = &[1, 0, 0, 0, 0, 0];
    syscall_arith384_mod(&mut params);
    let expected_d: [u64; 6] = [0, 0, 0, 0, 0, 0];
    assert_eq!(params.d, &expected_d);

    params.a = &[
        4332616871279656262,
        10917124144477883021,
        13281191951274694749,
        3486998266802970665,
        0,
        0,
    ];
    params.b = &[
        4332616871279656262,
        10917124144477883021,
        13281191951274694749,
        3486998266802970665,
        0,
        0,
    ];
    params.c = &[
        4332616871279656262,
        10917124144477883021,
        13281191951274694749,
        3486998266802970665,
        0,
        0,
    ];
    params.module = &[
        4332616871279656262,
        10917124144477883021,
        13281191951274694749,
        3486998266802970665,
        0,
        0,
    ];
    syscall_arith384_mod(&mut params);
    let expected_d: [u64; 6] = [0, 0, 0, 0, 0, 0];
    assert_eq!(params.d, &expected_d);
}
