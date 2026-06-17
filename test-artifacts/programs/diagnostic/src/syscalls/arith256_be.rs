use ziskos::syscalls::*;

#[inline]
fn be4(a: [u64; 4]) -> [u64; 4] {
    [a[3].swap_bytes(), a[2].swap_bytes(), a[1].swap_bytes(), a[0].swap_bytes()]
}

pub fn diagnostic_arith256_be() {
    //////////////
    // Add256Be Tests
    //////////////

    let a: [u64; 4] = [0, 0, 0, 0];
    let b: [u64; 4] = [0, 0, 0, 0];
    let mut c: [u64; 4] = [0, 0, 0, 0];
    let mut params = SyscallAdd256Params { a: &a, b: &b, cin: 0, c: &mut c };

    let a_be =
        be4([1229782938247303441, 2459565876494606882, 3689348814741910323, 4919131752989213764]);
    let b_be = be4([
        17216961135462248174,
        15987178197214944733,
        14757395258967641292,
        13527612320720337851,
    ]);
    params.a = &a_be;
    params.b = &b_be;
    params.cin = 1;
    let cout = syscall_add256_be(&mut params);
    let expected_c_be: [u64; 4] = be4([0, 0, 0, 0]);
    let expected_cout: u64 = 1;
    assert_eq!(params.c, &expected_c_be);
    assert_eq!(cout, expected_cout);

    let a_be = be4([42, 1337, 3735928559, 2343432205]);
    let b_be = be4([13, 42, 305419896, 2271560481]);
    params.a = &a_be;
    params.b = &b_be;
    params.cin = 0;
    let cout = syscall_add256_be(&mut params);
    let expected_c_be: [u64; 4] = be4([55, 1379, 4041348455, 4614992686]);
    let expected_cout: u64 = 0;
    assert_eq!(params.c, &expected_c_be);
    assert_eq!(cout, expected_cout);

    //////////////
    // Arith256Be Tests
    //////////////

    let a: [u64; 4] = [0, 0, 0, 0];
    let b: [u64; 4] = [0, 0, 0, 0];
    let c: [u64; 4] = [0, 0, 0, 0];
    let mut dl: [u64; 4] = [0, 0, 0, 0];
    let mut dh: [u64; 4] = [0, 0, 0, 0];
    let mut params = SyscallArith256Params { a: &a, b: &b, c: &c, dl: &mut dl, dh: &mut dh };

    let a_be = be4([3, 0, 0, 0]);
    let b_be = be4([2, 0, 0, 0]);
    let c_be = be4([5, 0, 0, 0]);
    params.a = &a_be;
    params.b = &b_be;
    params.c = &c_be;
    syscall_arith256_be(&mut params);
    let expected_dh_be: [u64; 4] = be4([0, 0, 0, 0]);
    let expected_dl_be: [u64; 4] = be4([11, 0, 0, 0]);
    assert_eq!(params.dh, &expected_dh_be);
    assert_eq!(params.dl, &expected_dl_be);

    let a_be =
        be4([13970229013151504741, 8476296752562947313, 11810450538887363942, 511990551865481398]);
    let b_be =
        be4([11990850244716481796, 14558188671963395327, 9424388055416098482, 1459171711273467932]);
    let c_be =
        be4([16528603495754341937, 8893271371239080203, 9406449307822347647, 250213327518958686]);
    params.a = &a_be;
    params.b = &b_be;
    params.c = &c_be;
    syscall_arith256_be(&mut params);
    let expected_dh_be: [u64; 4] =
        be4([4910774022637574197, 12870152955407492665, 17746475360205808972, 40499403403452059]);
    let expected_dl_be: [u64; 4] =
        be4([3242244678432810181, 2099669192879440901, 14496343886419199978, 10002311647969911313]);
    assert_eq!(params.dh, &expected_dh_be);
    assert_eq!(params.dl, &expected_dl_be);

    //////////////
    // Arith256ModBe Tests
    //////////////

    let a: [u64; 4] = [0, 0, 0, 0];
    let b: [u64; 4] = [0, 0, 0, 0];
    let c: [u64; 4] = [0, 0, 0, 0];
    let module: [u64; 4] = [0, 0, 0, 0];
    let mut d: [u64; 4] = [0, 0, 0, 0];
    let mut params = SyscallArith256ModParams { a: &a, b: &b, c: &c, module: &module, d: &mut d };

    let a_be = be4([0, 0, 0, 0]);
    let b_be = be4([0, 0, 0, 0]);
    let c_be = be4([0, 0, 0, 0]);
    let module_be = be4([1, 0, 0, 0]);
    params.a = &a_be;
    params.b = &b_be;
    params.c = &c_be;
    params.module = &module_be;
    syscall_arith256_mod_be(&mut params);
    let expected_d_be: [u64; 4] = be4([0, 0, 0, 0]);
    assert_eq!(params.d, &expected_d_be);

    let a_be =
        be4([4332616871279656262, 10917124144477883021, 13281191951274694749, 3486998266802970665]);
    let b_be =
        be4([4332616871279656262, 10917124144477883021, 13281191951274694749, 3486998266802970665]);
    let c_be =
        be4([4332616871279656262, 10917124144477883021, 13281191951274694749, 3486998266802970665]);
    let module_be =
        be4([4332616871279656262, 10917124144477883021, 13281191951274694749, 3486998266802970665]);
    params.a = &a_be;
    params.b = &b_be;
    params.c = &c_be;
    params.module = &module_be;
    syscall_arith256_mod_be(&mut params);
    let expected_d_be: [u64; 4] = be4([0, 0, 0, 0]);
    assert_eq!(params.d, &expected_d_be);
}
