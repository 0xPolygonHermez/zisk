use ziskos::syscalls::*;

#[inline]
fn be6(a: [u64; 6]) -> [u64; 6] {
    [
        a[5].swap_bytes(),
        a[4].swap_bytes(),
        a[3].swap_bytes(),
        a[2].swap_bytes(),
        a[1].swap_bytes(),
        a[0].swap_bytes(),
    ]
}

pub fn diagnostic_arith384_be() {
    //////////////
    // Arith384ModBe Tests
    //////////////

    let a: [u64; 6] = [0, 0, 0, 0, 0, 0];
    let b: [u64; 6] = [0, 0, 0, 0, 0, 0];
    let c: [u64; 6] = [0, 0, 0, 0, 0, 0];
    let module: [u64; 6] = [0, 0, 0, 0, 0, 0];
    let mut d: [u64; 6] = [0, 0, 0, 0, 0, 0];
    let mut params = SyscallArith384ModParams { a: &a, b: &b, c: &c, module: &module, d: &mut d };

    // Test #0: arith384_mod_be
    let a_be = be6([0, 0, 0, 0, 0, 0]);
    let b_be = be6([0, 0, 0, 0, 0, 0]);
    let c_be = be6([0, 0, 0, 0, 0, 0]);
    let module_be = be6([1, 0, 0, 0, 0, 0]);
    params.a = &a_be;
    params.b = &b_be;
    params.c = &c_be;
    params.module = &module_be;
    syscall_arith384_mod_be(&mut params);
    let expected_d_be: [u64; 6] = be6([0, 0, 0, 0, 0, 0]);
    assert_eq!(params.d, &expected_d_be);

    let a_be = be6([1, 0, 0, 0, 0, 0]);
    let b_be = be6([0, 0, 0, 0, 0, 0]);
    let c_be = be6([0, 0, 0, 0, 0, 0]);
    let module_be = be6([1, 0, 0, 0, 0, 0]);
    params.a = &a_be;
    params.b = &b_be;
    params.c = &c_be;
    params.module = &module_be;
    syscall_arith384_mod_be(&mut params);
    let expected_d_be: [u64; 6] = be6([0, 0, 0, 0, 0, 0]);
    assert_eq!(params.d, &expected_d_be);

    let a_be = be6([
        4332616871279656262,
        10917124144477883021,
        13281191951274694749,
        3486998266802970665,
        0,
        0,
    ]);
    let b_be = be6([
        4332616871279656262,
        10917124144477883021,
        13281191951274694749,
        3486998266802970665,
        0,
        0,
    ]);
    let c_be = be6([
        4332616871279656262,
        10917124144477883021,
        13281191951274694749,
        3486998266802970665,
        0,
        0,
    ]);
    let module_be = be6([
        4332616871279656262,
        10917124144477883021,
        13281191951274694749,
        3486998266802970665,
        0,
        0,
    ]);
    params.a = &a_be;
    params.b = &b_be;
    params.c = &c_be;
    params.module = &module_be;
    syscall_arith384_mod_be(&mut params);
    let expected_d_be: [u64; 6] = be6([0, 0, 0, 0, 0, 0]);
    assert_eq!(params.d, &expected_d_be);
}
